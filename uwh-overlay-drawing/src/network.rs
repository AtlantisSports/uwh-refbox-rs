use serde_json::Value;
use std::io::Read;
use std::net::TcpStream;
use uwh_common::game_snapshot::GameSnapshot;

pub struct StatePacket<'a> {
    pub snapshot: GameSnapshot,
    pub black: Option<String>,
    pub white: Option<String>,
    pub w_flag: Option<&'a [u8]>,
    pub b_flag: Option<&'a [u8]>,
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
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get(format!(
            "https://{}/api/v1/tournaments/{}/games/{}",
            config.uwhscores_url, 2, 1
        ))?
        .text()?,
    )?;
    let black = data["game"]["black"].as_str().unwrap().to_owned();
    let white = data["game"]["white"].as_str().unwrap().to_owned();
    let w_flag = include_bytes!(".././assets/flags/Seattle (Typical Ratio).png");
    let b_flag = include_bytes!(".././assets/flags/LA Kraken Stretched (1 to 2 Ratio).png");
    if tx
        .send(StatePacket {
            snapshot,
            black: Some(black.clone()),
            white: Some(white.clone()),
            w_flag: Some(w_flag),
            b_flag: Some(b_flag),
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
                    w_flag: None,
                    b_flag: None,
                })
                .is_err()
            {
                eprintln!("Frontend could not recieve game snapshot!")
            }
        }
    }
}
