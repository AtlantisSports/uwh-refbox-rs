use macroquad::prelude::Texture2D;
use serde_json::Value;
use std::io::Read;
use std::net::TcpStream;
use std::sync::Arc;
use uwh_common::game_snapshot::GameSnapshot;

pub struct State {
    pub snapshot: GameSnapshot,
    pub black: String,
    pub white: String,
    pub w_flag: Arc<Texture2D>,
}

pub fn networking_thread(
    tx: std::sync::mpsc::Sender<State>,
    config: crate::AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((config.refbox_ip(), config.refbox_port() as u16))
        .expect("Is the refbox running? We error'd out on the connection");
    let mut buff = vec![0u8; 1024];
    let mut read_bytes = stream.read(&mut buff).unwrap();
    let snapshot: GameSnapshot = serde_json::de::from_slice(&buff[..read_bytes]).unwrap();
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get(format!(
            "https://{}/api/v1/tournaments/{}/games/{}",
            config.uwhscores_url(),
            snapshot.tournament_id,
            snapshot.game_number
        ))?
        .text()?,
    )?;
    let black = data["game"]["black"].as_str().unwrap().to_owned();
    let white = data["game"]["white"].as_str().unwrap().to_owned();
    let w_flag = Arc::new(Texture2D::from_file_with_format(
        include_bytes!(".././assets/flags/Seattle (Typical Ratio).png"),
        None,
    ));
    if tx
        .send(State {
            snapshot,
            black: black.clone(),
            white: white.clone(),
            w_flag: w_flag.clone(),
        })
        .is_err()
    {
        eprintln!("Frontend could not recieve game snapshot!")
    }
    loop {
        read_bytes = stream.read(&mut buff).unwrap();
        if let Ok(snapshot) = serde_json::de::from_slice(&buff[..read_bytes]) {
            if tx
                .send(State {
                    snapshot,
                    black: black.clone(),
                    white: white.clone(),
                    w_flag: w_flag.clone(),
                })
                .is_err()
            {
                eprintln!("Frontend could not recieve game snapshot!")
            }
        }
    }
}
