use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::net::TcpStream;
use uwh_common::game_snapshot::{Color, GamePeriod, GameSnapshot};

#[derive(Serialize, Deserialize, Clone)]
pub struct TeamInfo {
    pub team_name: String,
    pub players: Vec<(String, u8)>,
    pub flag: Option<Vec<u8>>,
}

impl TeamInfo {
    pub async fn new(url: &String, tournament_id: u32, team_id: u64, team_color: Color) -> Self {
        debug!(
            "Requesting UWH API for team information for team {}",
            team_id
        );
        let data: Value = serde_json::from_str(
            &reqwest::get(format!(
                "https://{}/api/v1/tournaments/{}/teams/{}",
                url, tournament_id, team_id
            ))
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        )
        .unwrap();

        //TODO filter out players with empty name strings?
        let players: Vec<Value> = data["team"]["roster"]
            .as_array()
            .map(|x| x.to_vec())
            .unwrap_or_default();
        let mut player_list: Vec<(String, u8)> = Vec::new();
        for player in players {
            player_list.push((
                player["name"]
                    .as_str()
                    .unwrap_or({
                        debug!("Unwrap failed on player name. Using default value.");
                        ""
                    })
                    .to_string(),
                player["number"].as_u64().unwrap_or({
                    debug!("Unwrap failed on player number. Using default value.");
                    0
                }) as u8,
            ));
        }

        let x = Self {
            team_name: data["team"]["name"]
                .as_str()
                .unwrap_or(match team_color {
                    Color::Black => "Black",
                    Color::White => "White",
                })
                .to_string(),
            players: player_list,
            flag: match data["team"]["flag_url"].as_str().map(|s| async move {
                reqwest::get(s)
                    .await
                    .expect("flag_url invalid")
                    .bytes()
                    .await
                    .unwrap_or({
                        debug!("Unwrap failed on getting flag data");
                        bytes::Bytes::new()
                    })
                    .to_vec()
            }) {
                Some(d) => Some(d.await),
                None => None,
            },
        };
        x
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StatePacket {
    pub snapshot: GameSnapshot,
    pub black: Option<TeamInfo>,
    pub white: Option<TeamInfo>,
    pub game_id: Option<u32>,
    pub pool: Option<String>,
    pub start_time: Option<String>,
}

async fn fetch_game_data(
    tr: crossbeam_channel::Sender<(Value, TeamInfo, TeamInfo)>,
    url: String,
    tournament_id: u32,
    game_id: u32,
) {
    debug!("Requesting game data from UWH API");
    let data = reqwest::get(format!(
        "https://{}/api/v1/tournaments/{}/games/{}",
        url, tournament_id, game_id
    ))
    .await
    .expect("No internet! Give some Internet pls");
    let text = data.text().await.unwrap();
    let data: Value = serde_json::from_str(text.as_str()).unwrap();
    let team_id_black = data["game"]["black_id"].as_u64().unwrap_or(0);
    let team_id_white = data["game"]["white_id"].as_u64().unwrap_or(0);
    tr.send((
        data,
        TeamInfo::new(&url, tournament_id, team_id_black, Color::Black).await,
        TeamInfo::new(&url, tournament_id, team_id_white, Color::White).await,
    ))
    .unwrap();
}

#[tokio::main]
pub async fn networking_thread(
    tx: crossbeam_channel::Sender<StatePacket>,
    config: crate::AppConfig,
) {
    debug!("Attempting refbox connection!");
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect((config.refbox_ip, config.refbox_port as u16)) {
            break stream;
        }
    };

    let (tr, rc) = crossbeam_channel::bounded::<(Value, TeamInfo, TeamInfo)>(3);
    let url = config.uwhscores_url.clone();
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    let mut game_id = None;
    let mut tournament_id = None;
    debug!("Networking thread initialized!");
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if read_bytes == 0 {
            error!("Connection to refbox lost! Attempting to reconnect!");
            stream = loop {
                if let Ok(stream) =
                    TcpStream::connect((config.refbox_ip, config.refbox_port as u16))
                {
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
                let url = url.clone();
                let tr = tr.clone();
                game_id = Some(gid);
                tournament_id = Some(tid);
                debug!(
                    "Fetching game data for tid: {}, gid: {}",
                    tournament_id.unwrap(),
                    game_id.unwrap()
                );
                tokio::spawn(async move {
                    fetch_game_data(tr, url, tournament_id.unwrap(), game_id.unwrap()).await
                });
            }
            if let Ok((data, black, white)) = rc.try_recv() {
                tx.send(StatePacket {
                    snapshot,
                    game_id,
                    black: Some(black),
                    white: Some(white),
                    pool: Some(
                        data["game"]["pool"]
                            .as_str()
                            .map(|s| format!("POOL: {}", s))
                            .unwrap_or_default(),
                    ),
                    start_time: Some(
                        data["game"]["start_time"]
                            .as_str()
                            .map(|s| String::from("START: ") + s.split_at(11).1.split_at(5).0)
                            .unwrap_or_default(),
                    ),
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
                })
                .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"))
            }
        } else {
            warn!("Corrupted snapshot discarded!")
        }
    }
}
