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
    pub fn new(url: &String, tournament_id: u32, team_id: u64, team_color: Color) -> Self {
        debug!(
            "Requesting UWH API for team information for team {}",
            team_id
        );
        let data: Value = serde_json::from_str(
            &reqwest::blocking::get(format!(
                "https://{}/api/v1/tournaments/{}/teams/{}",
                url, tournament_id, team_id
            ))
            .unwrap()
            .text()
            .unwrap(),
        )
        .unwrap();
        let players: Vec<Value> = data["team"]["roster"]
            .as_array()
            .map(|x| x.to_vec())
            .unwrap_or_default();
        let mut player_list: Vec<(String, u8)> = Vec::new();
        for player in players {
            player_list.push((
                player["name"].as_str().unwrap().to_string(),
                player["number"].as_u64().unwrap() as u8,
            ));
        }

        Self {
            team_name: data["team"]["name"]
                .as_str()
                .unwrap_or(match team_color {
                    Color::Black => "Black",
                    Color::White => "White",
                })
                .to_string(),
            players: player_list,
            flag: data["team"]["flag_url"]
                .as_str()
                .map(|s| reqwest::blocking::get(s).unwrap().bytes().unwrap().to_vec()),
        }
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

fn fetch_game_data(
    tr: std::sync::mpsc::Sender<(Value, TeamInfo, TeamInfo)>,
    url: String,
    tournament_id: u32,
    game_id: u32,
) {
    debug!("Requesting game data from UWH API");
    let data = reqwest::blocking::get(format!(
        "https://{}/api/v1/tournaments/{}/games/{}",
        url, tournament_id, game_id
    ))
    .unwrap();
    let text = data.text().unwrap();
    let data: Value = serde_json::from_str(text.as_str()).unwrap();
    let team_id_black = Some(data["game"]["black_id"].as_u64().unwrap_or(0));
    let team_id_white = Some(data["game"]["white_id"].as_u64().unwrap_or(0));
    tr.send((
        data,
        TeamInfo::new(&url, tournament_id, team_id_black.unwrap(), Color::Black),
        TeamInfo::new(&url, tournament_id, team_id_white.unwrap(), Color::White),
    ))
    .unwrap();
}

#[tokio::main]
pub async fn networking_thread(tx: std::sync::mpsc::Sender<StatePacket>, config: crate::AppConfig) {
    let mut stream = TcpStream::connect((config.refbox_ip, config.refbox_port as u16))
        .expect("Refbox should be running and accessible");

    let (tr, rc) = std::sync::mpsc::channel::<(Value, TeamInfo, TeamInfo)>();
    let url = config.uwhscores_url.clone();
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    let mut game_id = None;
    let mut tournament_id = None;
    debug!("Networking thread initialized!");
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if read_bytes == 0 {
            error!("Connection to refbox lost!");
            return;
        }
        if let Ok(snapshot) = serde_json::de::from_slice::<GameSnapshot>(&buff[..read_bytes]) {
            let tid = snapshot.tournament_id;
            let gid =
                if snapshot.current_period == GamePeriod::BetweenGames && !snapshot.is_old_game {
                    snapshot.next_game_number
                } else {
                    snapshot.game_number
                };
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
                std::thread::spawn(move || {
                    fetch_game_data(tr, url, tournament_id.unwrap(), game_id.unwrap())
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
                            .unwrap_or("")
                            .to_string(),
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
