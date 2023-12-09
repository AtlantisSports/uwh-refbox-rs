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
        Some("") => {
            warn!("Image URL is empty. Skipping.");
            None
        }
        Some(url) => Some(
            client
                .get(url)
                .send()
                .await
                .map_err(|e| {
                    warn!("Couldn't get image \"{url}\" from network: {e}");
                    e
                })
                .ok()?
                .bytes()
                .await
                .map_err(|e| {
                    warn!("Couldn't get image \"{url}\"  body: {e}");
                    e
                })
                .ok()?
                .to_vec(),
        ),
        None => None,
    }
}

/// Contains data of each individual in the roster. pictures are raw unprocessed bytes that are
/// serialisable
#[derive(Serialize, Deserialize, Clone)]
pub struct MemberRaw {
    pub name: String,
    pub role: Option<String>,
    pub number: Option<u8>,
    pub picture: Option<Vec<u8>>,
    pub geared_picture: Option<Vec<u8>>,
}
/// Contains information about team. `flag` here is a byte array for `Serialize`, which is
/// processed into `Texture2D` when struct is converted into `TeamInfo`
#[derive(Serialize, Deserialize, Clone)]
pub struct TeamInfoRaw {
    pub team_name: String,
    pub members: Vec<MemberRaw>,
    pub flag: Option<Vec<u8>>,
}

impl TeamInfoRaw {
    pub async fn new(
        uwhportal_url: &str,
        tournament_id: u32,
        team_id: u64,
        team_color: Color,
    ) -> Self {
        let client = CLIENT_CELL.get().unwrap();
        info!(
            "Requesting UWH API for team information for team {}",
            team_id
        );
        let data: Value = serde_json::from_str(
            &client
                .get(format!(
                    "{uwhportal_url}/api/admin/get-event-team?legacyEventId={tournament_id}&legacyTeamId={team_id}"
                ))
                .send()
                .await
                .expect("Coudn't request team data")
                .text()
                .await
                .expect("Coudn't get team data body"),
        )
        .unwrap();

        let (members, flag) = tokio::join!(
            futures::future::join_all(
                data["roster"]
                    .as_array()
                    .map(|x| x.to_vec())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|member| async move {
                        let (picture, geared_picture) = tokio::join!(
                            get_image_from_opt_url(member["photos"]["uniform"].as_str()),
                            get_image_from_opt_url(
                                member["photos"][match team_color {
                                    Color::Black => "darkGear",
                                    Color::White => "lightGear",
                                }]
                                .as_str(),
                            )
                        );
                        MemberRaw {
                            name: member["rosterName"]
                                .as_str()
                                .unwrap_or("Player")
                                .trim()
                                .to_string(),
                            number: member["capNumber"].as_u64().map(|e| e as u8),
                            role: member["roles"].as_array().and_then(|a| {
                                a.iter()
                                    .map(|v| v.as_str().unwrap_or("").to_owned())
                                    .find(|v| *v != "Player")
                            }),
                            picture,
                            geared_picture,
                        }
                    })
            ),
            get_image_from_opt_url(data["logoUrl"].as_str())
        );
        let members = members.into_iter().collect();

        Self {
            team_name: data["name"].as_str().map_or(
                match team_color {
                    Color::Black => String::from("Black"),
                    Color::White => String::from("White"),
                },
                |s| s.trim().to_uppercase(),
            ),
            members,
            flag,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StatePacket {
    pub snapshot: GameSnapshot,
    pub game_id: Option<u32>,
    pub data: Option<GameData>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameData {
    pub pool: String,
    pub start_time: String,
    pub referees: Vec<MemberRaw>,
    pub black: TeamInfoRaw,
    pub white: TeamInfoRaw,
    pub sponsor_logo: Option<Vec<u8>>,
    pub game_id: u32,
    pub tournament_id: u32,
}

async fn fetch_game_data(
    tr: crossbeam_channel::Sender<(GameData, bool)>,
    uwhscores_url: &str,
    uwhportal_url: &str,
    tournament_id: u32,
    game_id: u32,
    is_current_game: bool,
) {
    let client = CLIENT_CELL.get().unwrap();
    // retry periodically if no connection
    loop {
        if let Ok(data) = client
            .get(format!(
                "{uwhscores_url}/api/v1/tournaments/{tournament_id}/games/{game_id}"
            ))
            .send()
            .await
        {
            info!(
                "Got game data for tid:{}, gid:{} from UWH API",
                tournament_id, game_id
            );
            let text = data
                .text()
                .await
                .expect("Response body could not be recieved!");
            let data: Value = serde_json::from_str(text.as_str())
                .unwrap_or_else(|_| panic!("Server did not return valid json!: {}", text));
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
                        let (picture, geared_picture) = tokio::join!(
                            get_image_from_opt_url(referee["picture_url"].as_str()),
                            get_image_from_opt_url(referee["geared_picture_url"].as_str())
                        );
                        (
                            referee["name"].as_str().map(|s| s.trim().to_uppercase()),
                            referee["number"].as_u64().map(|e| e as u8),
                            referee["role"].as_str().map(|s| s.trim().to_uppercase()),
                            picture,
                            geared_picture,
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
            let (black, white) = tokio::join!(
                TeamInfoRaw::new(uwhportal_url, tournament_id, team_id_black, Color::Black,),
                TeamInfoRaw::new(uwhportal_url, tournament_id, team_id_white, Color::White,)
            );
            tr.send((
                GameData {
                    pool,
                    start_time,
                    referees,
                    black,
                    white,
                    sponsor_logo,
                    tournament_id,
                    game_id,
                },
                is_current_game,
            ))
            .map_err(|e| error!("Couldn't send data: {e}"))
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

    let (tr, rc) = crossbeam_channel::bounded::<(GameData, bool)>(3);
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    let mut game_id = None;
    let mut tournament_id = None;
    let mut next_game_data: Option<GameData> = None;
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
            let tournament_id_new = snapshot.tournament_id;
            let game_id_new =
                if snapshot.current_period == GamePeriod::BetweenGames && !snapshot.is_old_game {
                    snapshot.next_game_number
                } else {
                    snapshot.game_number
                };
            let next_gid = snapshot.next_game_number;

            let tr_ = tr.clone();
            let uwhscores_url = config.uwhscores_url.clone();
            let uwhportal_url = config.uwhportal_url.clone();

            // Request new game cache if empty or invalid
            if next_game_data.is_none()
                || next_game_data.as_ref().unwrap().game_id != next_gid
                || next_game_data.as_ref().unwrap().tournament_id != tournament_id_new
            {
                info!(
                    "Fetching game data to cache for tid: {}, gid: {}",
                    tournament_id_new, next_gid,
                );
                tokio::spawn(async move {
                    fetch_game_data(
                        tr_,
                        &uwhscores_url,
                        &uwhportal_url,
                        tournament_id_new,
                        next_gid,
                        false,
                    )
                    .await;
                });
            }

            // initial case when no data is initialised
            if game_id.is_none() {
                let tr_ = tr.clone();
                let uwhscores_url = config.uwhscores_url.clone();
                let uwhportal_url = config.uwhportal_url.clone();
                game_id = Some(game_id_new);
                tournament_id = Some(tournament_id_new);
                info!(
                    "Fetching intial game data for tid: {}, gid: {}",
                    tournament_id_new, game_id_new
                );
                tokio::spawn(async move {
                    fetch_game_data(
                        tr_,
                        &uwhscores_url,
                        &uwhportal_url,
                        tournament_id_new,
                        game_id_new,
                        true,
                    )
                    .await;
                });
            }

            if let (Some(game_id_old), Some(tournament_id_old)) =
                (game_id.as_mut(), tournament_id.as_mut())
            {
                let tr_ = tr.clone();
                let uwhscores_url = config.uwhscores_url.clone();
                let uwhportal_url = config.uwhportal_url.clone();
                if *game_id_old != game_id_new || *tournament_id_old != tournament_id_new {
                    *game_id_old = game_id_new;
                    *tournament_id_old = tournament_id_new;
                    info!(
                        "Got new game ID {} / tournament ID {}",
                        game_id_new, tournament_id_new
                    );
                    if next_game_data.is_some()
                        && next_game_data.as_ref().unwrap().game_id == game_id_new
                        && next_game_data.as_ref().unwrap().tournament_id == tournament_id_new
                    {
                        let next_game_data = next_game_data.clone().unwrap();
                        info!("Sending cached game data for next game");
                        tx.send(StatePacket {
                            snapshot,
                            game_id,
                            data: Some(next_game_data),
                        })
                        .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
                    } else {
                        info!(
                            "Fetching game data for tid: {}, gid: {}. Cache is empty or invalid!",
                            tournament_id_new, game_id_new,
                        );
                        let (uwhscores_url_, uwhportal_url_, tr__) =
                            (uwhscores_url.clone(), uwhportal_url.clone(), tr_.clone());
                        tokio::spawn(async move {
                            fetch_game_data(
                                tr__,
                                &uwhscores_url_,
                                &uwhportal_url_,
                                tournament_id_new,
                                game_id_new,
                                true,
                            )
                            .await;
                        });
                    }
                    continue;
                }
            }
            if let Ok((game_data, is_current_game)) = rc.try_recv() {
                if is_current_game {
                    info!("Got game state update from network!");
                    tx.send(StatePacket {
                        snapshot,
                        game_id,
                        data: Some(game_data),
                    })
                    .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
                } else {
                    info!("Got game state update from network for next_game!");
                    next_game_data = Some(game_data);
                }
            } else {
                tx.send(StatePacket {
                    snapshot,
                    game_id,
                    data: None,
                })
                .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
            }
        } else {
            warn!("Corrupted snapshot discarded!");
        }
    }
}
