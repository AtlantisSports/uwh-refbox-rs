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
            team_name: data["team"]["name"].as_str().unwrap().to_string(),
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

fn fetch_data(tr: std::sync::mpsc::Sender<Value>, url: String) {
    info!("Requesting game data from UWH API");
    let data = reqwest::blocking::get(format!(
        "https://{}/api/v1/tournaments/{}/games/{}",
        url, 28, 2
    ))
    .unwrap();
    let t = data.text().unwrap();
    tr.send(serde_json::from_str(t.as_str()).unwrap()).unwrap();
}

#[tokio::main]
pub async fn networking_thread(
    tx: std::sync::mpsc::Sender<StatePacket>,
    config: crate::AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((config.refbox_ip, config.refbox_port as u16))
        .expect("Is the refbox running? We error'd out on the connection");
    let (tr, rc) = std::sync::mpsc::channel::<Value>();
    let url = config.uwhscores_url.clone();
    std::thread::spawn(move || fetch_data(tr, url));
    let mut team_id_black;
    let mut team_id_white;
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if let Ok(snapshot) = serde_json::de::from_slice(&buff[..read_bytes]) {
            if let Ok(data) = rc.try_recv() {
                team_id_black = Some(data["game"]["black_id"].as_u64().unwrap());
                team_id_white = Some(data["game"]["white_id"].as_u64().unwrap());
                if tx
                    .send(StatePacket {
                        snapshot,
                        game_id: Some(2),
                        black: Some(TeamInfo::new(
                            &config.uwhscores_url,
                            28,
                            team_id_black.unwrap(),
                        )),
                        white: Some(TeamInfo::new(
                            &config.uwhscores_url,
                            28,
                            team_id_white.unwrap(),
                        )),
                        pool: Some(data["game"]["pool"].as_str().unwrap().to_string()),
                        start_time: Some(data["game"]["start_time"].as_str().unwrap().to_string()),
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
