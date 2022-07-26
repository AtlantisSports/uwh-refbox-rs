use std::io::Read;
use std::net::TcpStream;
use uwh_common::game_snapshot::GameSnapshot;

pub fn networking_thread(
    tx: std::sync::mpsc::Sender<GameSnapshot>,
) -> Result<(), Box<dyn std::error::Error>> {
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get("https://uwhscores.com/api/v1/tournaments")?.text()?,
    //)?;
    //let tournament = &data["tournaments"][0]["tid"];
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get(format!(
    //        "https://uwhscores.com/api/v1/tournaments/{}",
    //        tournament
    //    ))?
    //    .text()?,
    //)?;
    //let division = &data["tournament"]["divisions"][0].as_str().unwrap();
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get(format!(
    //        "https://uwhscores.com/api/v1/tournaments/{}/games?div={}",
    //        tournament, division
    //    ))?
    //    .text()?,
    //)?;
    //let game = &data["games"][0]["gid"];
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get(format!(
    //        "https://uwhscores.com/api/v1/tournaments/{}/games/{}",
    //        tournament, game
    //    ))?
    //    .text()?,
    //)?;
    let mut stream = TcpStream::connect(("localhost", 8000))
        .expect("Is the refbox running? We error'd out on the connection");
    let mut buff = vec![0u8; 1024];
    loop {
        let read_bytes = stream.read(&mut buff).unwrap();
        let snapshot: GameSnapshot = serde_json::de::from_slice(&buff[..read_bytes]).unwrap();

        tx.send(snapshot).unwrap();
    }
}
