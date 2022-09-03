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
    pub flag_url: Option<String>,
}

pub fn get_flag(flag_url: &str) -> Vec<u8> {
    reqwest::blocking::get(flag_url)
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec()
}

impl TeamInfo {
    pub fn new(config: &crate::AppConfig, tournament_id: u32, team_id: u64) -> Self {
        info!("Requesting UWH API for team information");
        let data: Value = serde_json::from_str(
            &reqwest::blocking::get(format!(
                "https://{}/api/v1/tournaments/{}/teams/{}",
                config.uwhscores_url, tournament_id, team_id
            ))
            .unwrap()
            .text()
            .unwrap(),
        )
        .unwrap();
        info!("Recieved response");
        let players: Vec<Value> = data["team"]["roster"]
            .as_array()
            .map(|x| x.clone())
            .unwrap_or_else(|| {
                info!("Player data not recieved from API");
                let temp = Vec::new();
                temp
            });
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
            flag_url: data["team"]["flag_url"].as_str().map(|s| s.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StatePacket {
    pub snapshot: GameSnapshot,
    pub black: Option<TeamInfo>,
    pub white: Option<TeamInfo>,
}

pub fn networking_thread(
    tx: std::sync::mpsc::Sender<StatePacket>,
    config: crate::AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((config.refbox_ip, config.refbox_port as u16))
        .expect("Is the refbox running? We error'd out on the connection");
    let mut buff = vec![0u8; 1024];
    let mut read_bytes = stream.read(&mut buff).unwrap();
    let snapshot: GameSnapshot = serde_json::de::from_slice(&buff[..read_bytes]).unwrap();
    info!("Requesting game data from UWH API");
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get(format!(
            "https://{}/api/v1/tournaments/{}/games/{}",
            config.uwhscores_url, 5, 2
        ))?
        .text()?,
    )?;
    info!("Recieved response");
    let team_id_black = data["game"]["black_id"].as_u64().unwrap();
    let team_id_white = data["game"]["white_id"].as_u64().unwrap();
    let black = TeamInfo::new(&config, 5, team_id_black);
    let white = TeamInfo::new(&config, 5, team_id_white);
    if tx
        .send(StatePacket {
            snapshot,
            black: Some(TeamInfo { ..black.clone() }),
            white: Some(TeamInfo { ..white.clone() }),
        })
        .is_err()
    {
        eprintln!("Frontend could not recieve game snapshot!")
    }
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if let Ok(snapshot) = serde_json::de::from_slice(&buff[..read_bytes]) {
            if tx
                .send(StatePacket {
                    snapshot,
                    black: None,
                    white: None,
                })
                .is_err()
            {
                info!("Frontend could not recieve game snapshot!")
            }
        }
    }
}
