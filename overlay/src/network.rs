use log::{error, info, warn};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::TcpStream;
use std::sync::OnceLock;
use std::{io::Read, time::Duration};
use uwh_common::game_snapshot::{Color, GamePeriod, GameSnapshot};

static CLIENT_CELL: OnceLock<Client> = OnceLock::new();

async fn get_image_from_opt_url(url: Option<&str>) -> Option<Vec<u8>> {
    let client = CLIENT_CELL.get().unwrap();
    match url {
        Some(url) => Some(
            client
                .get(url)
                .send()
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

/// Contains data of each induvidual in the roster. pictures are raw unprocessed bytes that are
/// serialisable
#[derive(Serialize, Deserialize)]
pub struct MemberRaw {
    pub name: String,
    pub role: Option<String>,
    pub number: Option<u8>,
    pub picture: Option<Vec<u8>>,
    pub geared_picture: Option<Vec<u8>>,
}
/// Contains information about team. `flag` here is a byte array for `Serialize`, which is
/// processed into `Texture2D` when struct is converted into `TeamInfo`
#[derive(Serialize, Deserialize)]
pub struct TeamInfoRaw {
    pub team_name: String,
    pub members: Vec<MemberRaw>,
    pub flag: Option<Vec<u8>>,
}

impl TeamInfoRaw {
    pub async fn new(url: &String, tournament_id: u32, team_id: u64, team_color: Color) -> Self {
        let client = CLIENT_CELL.get().unwrap();
        info!(
            "Requesting UWH API for team information for team {}",
            team_id
        );
        let data: Value = serde_json::from_str(
            &client
                .get(format!(
                    "http://{url}/api/v1/tournaments/{tournament_id}/teams/{team_id}"
                ))
                .send()
                .await
                .expect("Coudn't request team data")
                .text()
                .await
                .expect("Coudn't get team data body"),
        )
        .unwrap();

        let mut members = Vec::new();
        futures::future::join_all(
                data["team"]["roster"]
                    .as_array()
                    .map(|x| x.to_vec())
                    .unwrap_or_default()
                    .iter()
                    .map(|member| async {
                        (
                            member["name"].as_str().map(|s| s.trim().to_string()),
                            member["number"].as_u64().map(|e| e as u8),
                            member["role"].as_str().map(|s| s.trim().to_uppercase()),
                            get_image_from_opt_url(member["picture_url"].as_str()).await,
                            get_image_from_opt_url(member["geared_picture_url"].as_str()).await,
                        )
                    }),
            )
            .await
            .into_iter()
            .filter_map(|data| {
                if let (Some(name), number, role, picture, geared_picture) = data {
                    Some(MemberRaw {
                        name,
                        role,
                        number,
                        picture,
                        geared_picture,
                    })
                } else {
                    None
                }
            })
            .for_each(|member|
                // don't push if name field is blank or if both number and role are missing
                // (roster data point has to be in the player or support category or both)
                if member.name != String::new() && (member.number.is_some() || member.role.is_some()) {
                    members.push(member);
                }
            );

        let x = Self {
            team_name: data["team"]["name"].as_str().map_or(
                match team_color {
                    Color::Black => String::from("Black"),
                    Color::White => String::from("White"),
                },
                |s| s.trim().to_uppercase(),
            ),
            members,
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
    pub sponsor_logo: Option<Vec<u8>>,
    pub game_id: Option<u32>,
    pub pool: Option<String>,
    pub start_time: Option<String>,
    pub referees: Option<Vec<MemberRaw>>,
}

async fn fetch_game_data(
    tr: crossbeam_channel::Sender<(
        String,
        String,
        Vec<MemberRaw>,
        TeamInfoRaw,
        TeamInfoRaw,
        Option<Vec<u8>>,
    )>,
    url: String,
    tournament_id: u32,
    game_id: u32,
) {
    let client = CLIENT_CELL.get().unwrap();
    // retry periodically if no connection
    loop {
        info!("Trying to request game data from UWH API");
        if let Ok(data) = client
            .get(format!(
                "http://{url}/api/v1/tournaments/{tournament_id}/games/{game_id}"
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
                .map(|s| format!("POOL: {s}"))
                .unwrap_or_default();
            let start_time = data["game"]["start_time"]
                .as_str()
                .map(|s| String::from("START: ") + s.split_at(11).1.split_at(5).0)
                .unwrap_or_default();
            let sponsor_logo = get_image_from_opt_url(data["game"]["sponsor_logo"].as_str()).await;
            let mut referees = Vec::new();
            futures::future::join_all(
                data["game"]["referees"]
                    .as_array()
                    .map(|x| x.to_vec())
                    .unwrap_or_default()
                    .iter()
                    .map(|referee| async {
                        (
                            referee["name"].as_str().map(|s| s.trim().to_uppercase()),
                            referee["number"].as_u64().map(|e| e as u8),
                            referee["role"].as_str().map(|s| s.trim().to_uppercase()),
                            get_image_from_opt_url(referee["picture_url"].as_str()).await,
                            get_image_from_opt_url(referee["geared_picture_url"].as_str()).await,
                        )
                    }),
            )
            .await
            .into_iter()
            .filter_map(|data| {
                if let (Some(name), number, role, picture, geared_picture) = data {
                    Some(MemberRaw {
                        name,
                        role,
                        number,
                        picture,
                        geared_picture,
                    })
                } else {
                    None
                }
            })
            .for_each(|referee| referees.push(referee));
            tr.send((
                pool,
                start_time,
                referees,
                TeamInfoRaw::new(&url, tournament_id, team_id_black, Color::Black).await,
                TeamInfoRaw::new(&url, tournament_id, team_id_white, Color::White).await,
                sponsor_logo,
            ))
            .unwrap();
            return;
        }
        warn!("Game data request failed. Trying again in 5 seconds.");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[tokio::main]
pub async fn networking_thread(
    tx: crossbeam_channel::Sender<StatePacket>,
    config: crate::AppConfig,
) {
    CLIENT_CELL
        .set(
            ClientBuilder::new()
                .connect_timeout(Duration::from_secs(20))
                .build()
                .expect("Couldn't create HTTP client!"),
        )
        .unwrap();

    info!("Attempting refbox connection!");
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect((config.refbox_ip, config.refbox_port as u16)) {
            break stream;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    };
    info!("Connected to refbox!");

    let (tr, rc) = crossbeam_channel::bounded::<(
        String,
        String,
        Vec<MemberRaw>,
        TeamInfoRaw,
        TeamInfoRaw,
        Option<Vec<u8>>,
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
                tokio::spawn(async move {
                    fetch_game_data(tr, url, tournament_id.unwrap(), game_id.unwrap()).await;
                });
            }
            if let Ok((pool, start_time, referees, black, white, sponsor_logo)) = rc.try_recv() {
                info!("Got game state update from network!");
                tx.send(StatePacket {
                    snapshot,
                    game_id,
                    black: Some(black),
                    white: Some(white),
                    pool: Some(pool),
                    start_time: Some(start_time),
                    referees: Some(referees),
                    sponsor_logo,
                })
                .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
            } else {
                tx.send(StatePacket {
                    snapshot,
                    game_id,
                    black: None,
                    white: None,
                    pool: None,
                    start_time: None,
                    referees: None,
                    sponsor_logo: None,
                })
                .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
            }
        } else {
            warn!("Corrupted snapshot discarded!")
        }
    }
}
