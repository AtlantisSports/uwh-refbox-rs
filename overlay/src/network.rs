use log::{error, info, warn};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::TcpStream;
use std::{io::Read, time::Duration};
use uwh_common::game_snapshot::{Color, GamePeriod, GameSnapshot};

async fn get_image_from_opt_url(url: Option<&str>) -> Option<Vec<u8>> {
    match url {
        Some(url) => Some(
            reqwest::get(url)
                .await
                .map_err(|e| {
                    error!("Couldn't get flag from network: {}", e);
                    e
                })
                .ok()?
                .bytes()
                .await
                .map_err(|e| {
                    error!("Couldn't get flag body: {}", e);
                    e
                })
                .ok()?
                .to_vec(),
        ),
        None => None,
    }
}

/// Contains information about team. `flag` here is a byte array for `Serialize`, which is
/// processed into `Texture2D` when struct is converted into `TeamInfo`
#[derive(Serialize, Deserialize)]
pub struct TeamInfoRaw {
    pub team_name: String,
    /// `Vec` of (Name, Number, Picture, Geared Picture)
    pub players: Vec<(String, u8, Option<Vec<u8>>, Option<Vec<u8>>)>,
    /// `Vec` of (Name, Role, Picture)
    pub support_members: Vec<(String, String, Option<Vec<u8>>)>,
    pub flag: Option<Vec<u8>>,
}

impl TeamInfoRaw {
    pub async fn new(url: &String, tournament_id: u32, team_id: u64, team_color: Color) -> Self {
        info!(
            "Requesting UWH API for team information for team {}",
            team_id
        );
        let data: Value = serde_json::from_str(
            &reqwest::get(format!(
                "http://{}/api/v1/tournaments/{}/teams/{}",
                url, tournament_id, team_id
            ))
            .await
            .expect("Coudn't request team data")
            .text()
            .await
            .expect("Coudn't get team data body"),
        )
        .unwrap();

        //TODO check filter out players with empty name strings?
        let mut players: Vec<(String, u8, Option<Vec<u8>>, Option<Vec<u8>>)> = Vec::new();
        for player in data["team"]["roster"]
            .as_array()
            .map(|x| x.to_vec())
            .unwrap_or_default()
        {
            if let (Some(name), Some(number), picture, geared_picture) = (
                player["name"].as_str().map(String::from),
                player["number"].as_u64().map(|e| e as u8),
                get_image_from_opt_url(player["picture_url"].as_str()).await,
                get_image_from_opt_url(player["geared_picture_url"].as_str()).await,
            ) {
                players.push((name, number, picture, geared_picture));
            }
        }

        let mut support_members: Vec<(String, String, Option<Vec<u8>>)> = Vec::new();
        for member in data["team"]["support"]
            .as_array()
            .map(|x| x.to_vec())
            .unwrap_or_default()
        {
            if let (Some(name), Some(role), picture) = (
                member["name"].as_str().map(String::from),
                member["role"].as_str().map(String::from),
                get_image_from_opt_url(member["picture_url"].as_str()).await,
            ) {
                support_members.push((name, role, picture));
            }
        }

        let x = Self {
            team_name: "kjdkjendkjenkdnekjndkejnde".to_string(),
            players,
            support_members,
            flag: get_image_from_opt_url(data["team"]["flag_url"].as_str()).await,
        };
        x
    }
}

#[derive(Serialize, Deserialize)]
pub struct StatePacket {
    pub snapshot: GameSnapshot,
    pub black: Option<TeamInfoRaw>,
    pub white: Option<TeamInfoRaw>,
    pub game_id: Option<u32>,
    pub pool: Option<String>,
    pub start_time: Option<String>,
    pub referees: Option<Vec<(String, Option<Vec<u8>>)>>,
}

async fn fetch_game_data(
    client: Client,
    tr: crossbeam_channel::Sender<(
        String,
        String,
        Vec<(String, Option<Vec<u8>>)>,
        TeamInfoRaw,
        TeamInfoRaw,
    )>,
    url: String,
    tournament_id: u32,
    game_id: u32,
) {
    // retry periodically if no connection
    loop {
        info!("Trying to request game data from UWH API");
        if let Ok(data) = client
            .get(format!(
                "http://{}/api/v1/tournaments/{}/games/{}",
                url, tournament_id, game_id
            ))
            .send()
            .await
        {
            let text = data.text().await.unwrap();
            let data: Value = serde_json::from_str(text.as_str()).unwrap();
            let team_id_black = data["game"]["black_id"].as_u64().unwrap_or(0);
            let team_id_white = data["game"]["white_id"].as_u64().unwrap_or(0);

            let pool = data["game"]["pool"]
                .as_str()
                .map(|s| format!("POOL: {}", s))
                .unwrap_or_default();
            let start_time = data["game"]["start_time"]
                .as_str()
                .map(|s| String::from("START: ") + s.split_at(11).1.split_at(5).0)
                .unwrap_or_default();
            let mut referees: Vec<(String, Option<Vec<u8>>)> = Vec::new();
            for referee in data["game"]["referees"]
                .as_array()
                .map(|x| x.to_vec())
                .unwrap_or_default()
            {
                if let (Some(name), picture) = (
                    referee["name"].as_str().map(String::from),
                    get_image_from_opt_url(referee["picture_url"].as_str()).await,
                ) {
                    referees.push((name, picture));
                }
            }
            tr.send((
                pool,
                start_time,
                referees,
                TeamInfoRaw::new(&url, tournament_id, team_id_black, Color::Black).await,
                TeamInfoRaw::new(&url, tournament_id, team_id_white, Color::White).await,
            ))
            .unwrap();
            return;
        } else {
            warn!("Game data request failed. Trying again in 5 seconds.");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

#[tokio::main]
pub async fn networking_thread(
    tx: crossbeam_channel::Sender<StatePacket>,
    config: crate::AppConfig,
) {
    info!("Attempting refbox connection!");
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect((config.refbox_ip, config.refbox_port as u16)) {
            break stream;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    };
    info!("Connected to refbox!");

    let client = ClientBuilder::new()
        .connect_timeout(Duration::from_secs(20))
        .build()
        .expect("Couldn't create HTTP client!");

    let (tr, rc) = crossbeam_channel::bounded::<(
        String,
        String,
        Vec<(String, Option<Vec<u8>>)>,
        TeamInfoRaw,
        TeamInfoRaw,
    )>(3);
    let url = config.uwhscores_url.clone();
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    let mut game_id = None;
    let mut tournament_id = None;
    info!("Networking thread initialized!");
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if read_bytes == 0 {
            error!("Connection to refbox lost! Attempting to reconnect!");
            stream = loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if let Ok(stream) =
                    TcpStream::connect((config.refbox_ip, config.refbox_port as u16))
                {
                    info!("Found refbox!");
                    break stream;
                }
            };
        }
        if let Ok(snapshot) = serde_json::de::from_slice::<GameSnapshot>(&buff[..read_bytes]) {
            let tid = snapshot.tournament_id;
            let gid =
                if snapshot.current_period == GamePeriod::BetweenGames && !snapshot.is_old_game {
                    snapshot.next_game_number
                } else {
                    snapshot.game_number
                };
            let (tid, gid) = (35, 1);
            if (tournament_id.is_some() && tournament_id.unwrap() != tid
                || game_id.is_some() && game_id.unwrap() != gid)
                || tournament_id.is_none() && game_id.is_none()
            {
                let tr = tr.clone();
                let url = url.clone();
                game_id = Some(gid);
                tournament_id = Some(tid);
                info!(
                    "Fetching game data for tid: {}, gid: {}",
                    tournament_id.unwrap(),
                    game_id.unwrap()
                );
                let client = client.clone();
                tokio::spawn(async move {
                    fetch_game_data(client, tr, url, tournament_id.unwrap(), game_id.unwrap()).await
                });
            }
            if let Ok((pool, start_time, referees, black, white)) = rc.try_recv() {
                info!("Got game state update from network!");
                tx.send(StatePacket {
                    snapshot,
                    game_id,
                    black: Some(black),
                    white: Some(white),
                    pool: Some(pool),
                    start_time: Some(start_time),
                    referees: Some(referees),
                })
                .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"))
            } else {
                tx.send(StatePacket {
                    snapshot,
                    game_id,
                    black: None,
                    white: None,
                    pool: None,
                    start_time: None,
                    referees: None,
                })
                .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"))
            }
        } else {
            warn!("Corrupted snapshot discarded!")
        }
    }
}
