use log::error;
use macroquad::prelude::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::net::TcpStream;
use uwh_common::game_snapshot::GameSnapshot;

#[derive(Serialize, Deserialize, Clone)]
pub struct TeamInfo {
    pub team_name: String,
    pub players: Vec<(String, u8)>,
    pub flag: Option<Vec<u8>>,
}

impl TeamInfo {
    pub fn new(url: &String, tournament_id: u32, team_id: u64) -> Self {
        info!(
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
        info!("Recieved response");
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
                .unwrap_or("Game not found")
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

fn fetch_data(
    tr: std::sync::mpsc::Sender<(Value, TeamInfo, TeamInfo)>,
    url: String,
    tournament_id: u32,
    game_id: u32,
) {
    info!("Requesting game data from UWH API");
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
        TeamInfo::new(&url, tournament_id, team_id_black.unwrap()),
        TeamInfo::new(&url, tournament_id, team_id_white.unwrap()),
    ))
    .unwrap();
}

#[tokio::main]
pub async fn networking_thread(
    tx: std::sync::mpsc::Sender<StatePacket>,
    config: crate::AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((config.refbox_ip, config.refbox_port as u16))
        .expect("Is the refbox running? We error'd out on the connection");
    let (tr, rc) = std::sync::mpsc::channel::<(Value, TeamInfo, TeamInfo)>();
    let url = config.uwhscores_url.clone();
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    let mut game_id = std::u32::MAX;
    let mut tournament_id = std::u32::MAX;
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if let Ok(snapshot) = serde_json::de::from_slice::<GameSnapshot>(&buff[..read_bytes]) {
            let tid = snapshot.tournament_id;
            let gid = snapshot.game_number;
            if tid != tournament_id || gid != game_id {
                let url = url.clone();
                let tr = tr.clone();
                game_id = gid;
                tournament_id = tid;
                info!(
                    "Refreshing game data for tid: {}, gid: {}",
                    tournament_id, game_id
                );
                std::thread::spawn(move || fetch_data(tr, url, tournament_id, game_id));
            }
            if let Ok((data, black, white)) = rc.try_recv() {
                if tx
                    .send(StatePacket {
                        snapshot,
                        game_id: Some(game_id),
                        black: Some(black),
                        white: Some(white),
                        pool: Some(data["game"]["pool"].as_str().unwrap_or("").to_string()),
                        start_time: Some(
                            data["game"]["start_time"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                        ),
                    })
                    .is_err()
                {
                    error!("Frontend could not recieve snapshot!")
                }
            } else {
                if tx
                    .send(StatePacket {
                        snapshot,
                        game_id: None,
                        black: None,
                        white: None,
                        pool: None,
                        start_time: None,
                    })
                    .is_err()
                {
                    error!("Frontend could not recieve snapshot!")
                }
            }
        }
    }
}
