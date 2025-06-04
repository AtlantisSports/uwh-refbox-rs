use log::{debug, error, info, warn};
use reqwest::{Client, ClientBuilder, IntoUrl};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::OnceLock, time::Duration};
use time::{OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};
use tokio::{io::AsyncReadExt, net::TcpStream};
use uwh_common::{
    color::Color,
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{EventId, GameNumber, TeamId},
};

use crate::AppConfig;

const START_TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]");

static CLIENT_CELL: OnceLock<Client> = OnceLock::new();

pub type Image = (u16, u16, Vec<u8>);

pub const BLACK_TEAM_NAME: &str = "BLACK";
pub const WHITE_TEAM_NAME: &str = "WHITE";

async fn get_image_from_opt_url<T: IntoUrl>(url: Option<T>) -> Option<Image> {
    let client = CLIENT_CELL.get().unwrap();
    let img_bytes = match url {
        None => None,
        Some(url) => Some(
            client
                .get(url)
                .send()
                .await
                .map_err(|e| {
                    warn!("Couldn't get image from network: {e}");
                    e
                })
                .ok()?
                .bytes()
                .await
                .map_err(|e| {
                    warn!("Couldn't get image body: {e}");
                    e
                })
                .ok()?
                .to_vec(),
        ),
    };

    img_bytes
        .and_then(|bytes| image::load_from_memory(&bytes).ok())
        .map(|img| {
            (
                img.width() as u16,
                img.height() as u16,
                img.into_rgba8().into_raw(),
            )
        })
}

/// Contains data of each individual in the roster. pictures are raw unprocessed bytes that are
/// serialisable
#[derive(Serialize, Deserialize, Clone)]
pub struct MemberRaw {
    pub name: String,
    pub role: Option<String>,
    pub number: Option<u8>,
    pub picture: Option<Image>,
    pub geared_picture: Option<Image>,
}
/// Contains information about team. `flag` here is a byte array for `Serialize`, which is
/// processed into `Texture2D` when struct is converted into `TeamInfo`
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TeamInfoRaw {
    pub team_name: String,
    pub members: Vec<MemberRaw>,
    pub flag: Option<Image>,
}

impl TeamInfoRaw {
    pub async fn new(
        uwhportal_url: &str,
        event_id: &EventId,
        team_id: Option<&TeamId>,
        team_color: Color,
    ) -> Self {
        let team_id = match team_id {
            Some(id) => id,
            None => return Self::default(),
        };

        let client = CLIENT_CELL.get().unwrap();
        info!("Requesting UWH API for team information for team: {team_id} of event: {event_id}");
        let data: Value = serde_json::from_str(
            &client
                .get(format!("{uwhportal_url}/api/admin/get-event-team"))
                .query(&[("teamId", team_id.full())])
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
                data["roster"] // json array of players
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
                    Color::Black => String::from(BLACK_TEAM_NAME),
                    Color::White => String::from(WHITE_TEAM_NAME),
                },
                |s| s.trim().to_uppercase(),
            ),
            members,
            flag,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct EventLogos {
    pub event_id: EventId,
    pub event_logo: Option<Image>,
    pub sponsors: Option<Image>,
}

impl EventLogos {
    pub async fn new(uwhportal_url: &str, event_id: EventId) -> Self {
        let client = CLIENT_CELL.get().unwrap();
        info!("Requesting Portal API for overlay images for event: {event_id}");
        let data: Value = serde_json::from_str(
            &client
                .get(format!(
                    "{uwhportal_url}/api/admin/events/{}/overlay-attachments",
                    event_id.partial()
                ))
                .send()
                .await
                .expect("Coudn't request event images")
                .text()
                .await
                .expect("Coudn't get event images body"),
        )
        .unwrap();

        let mut event_logo_url = None;
        let mut sponsors_url = None;

        for attachment in data["overlayAttachments"]
            .as_array()
            .map(|x| x.to_vec())
            .unwrap_or_default()
        {
            if attachment["type"].as_str() == Some("Overlay") {
                event_logo_url = attachment["url"].as_str().map(|s| s.to_owned());
            } else if attachment["type"].as_str() == Some("Sponsor") {
                sponsors_url = attachment["url"].as_str().to_owned().map(|s| s.to_owned());
            }
        }

        let (event_logo, sponsors) = tokio::join!(
            get_image_from_opt_url(event_logo_url),
            get_image_from_opt_url(sponsors_url)
        );

        Self {
            event_id,
            event_logo,
            sponsors,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StatePacket {
    pub snapshot: GameSnapshot,
    pub game_number: Option<GameNumber>,
    pub data: Option<GameData>,
    pub event_logos: Option<EventLogos>,
}

#[derive(Serialize, Deserialize)]
pub enum StateUpdate {
    Snapshot(GameSnapshot),
    GameData(GameData),
    EventLogos(EventId, EventLogos),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameData {
    pub pool: String,
    pub start_time: String,
    pub referees: Vec<MemberRaw>,
    pub black: TeamInfoRaw,
    pub white: TeamInfoRaw,
    pub game_number: GameNumber,
    pub event_id: EventId,
}

async fn fetch_game_referees(
    uwhportal_url: &str,
    event_id: &EventId,
    game_number: &GameNumber,
) -> Result<Vec<MemberRaw>, reqwest::Error> {
    let client = CLIENT_CELL.get().unwrap();
    info!("Requesting Portal API for referees for (event, game): ({event_id}, {game_number})");
    let data: Value = client
        .get(format!("{uwhportal_url}/api/admin/events/game-referees"))
        .query(&[("eventId", event_id.full()), ("gameNumber", game_number)])
        .send()
        .await?
        .json()
        .await?;

    Ok(futures::future::join_all(
        data.get("referees")
            .and_then(|r| r.as_array())
            .map(|x| x.to_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|referee| async move {
                let (uniform_pic, gear_pic) = if let Some(Value::Object(photos)) =
                    &referee.get("user").and_then(|u| u.get("photos"))
                {
                    tokio::join!(
                        get_image_from_opt_url(photos.get("uniform").and_then(|u| u.as_str())),
                        get_image_from_opt_url(photos.get("inGear").and_then(|u| u.as_str()))
                    )
                } else {
                    (None, None)
                };

                let role = referee["role"].as_str().map(|s| s.trim());
                let role = match role {
                    Some("Water1") | Some("Water2") | Some("Water3") => Some("Water"),
                    Some("Chief") => Some("Chief"),
                    Some("TimeOrScoreKeeper") => Some("Timekeeper"),
                    Some(other) => {
                        warn!("Referee has unexpected role: {other:?}");
                        None
                    }
                    None => {
                        warn!("Referee has no role");
                        None
                    }
                }
                .map(|r| r.to_string());

                (
                    referee["user"]["name"]
                        .as_str()
                        .map(|s| s.trim().to_string()),
                    role,
                    uniform_pic,
                    gear_pic,
                )
            }),
    )
    .await
    .into_iter()
    .filter_map(|data| {
        if let (Some(name), role, picture, geared_picture) = data {
            Some(MemberRaw {
                name,
                role,
                number: None,
                picture,
                geared_picture,
            })
        } else {
            None
        }
    })
    .collect())
}

async fn fetch_game_data(
    game_data_tx: tokio::sync::mpsc::UnboundedSender<GameData>,
    uwhportal_url: &str,
    event_id: EventId,
    game_number: &GameNumber,
) {
    let client = CLIENT_CELL.get().unwrap();
    // retry periodically if no connection
    loop {
        if let Ok(data) = client
            .get(format!(
                "{uwhportal_url}/api/events/{}/schedule",
                event_id.partial()
            ))
            .send()
            .await
        {
            let text = data
                .text()
                .await
                .expect("Response body could not be recieved!");
            let data: Value = match serde_json::from_str(text.as_str()) {
                Ok(d) => d,
                _ => {
                    error!(
                        "Aborting schedule fetch! Server did not return valid JSON for event {event_id}: {text}"
                    );
                    return;
                }
            };
            let data_game = if let Some(game) = data["games"].as_array().and_then(|games| {
                games.iter().find_map(|game| {
                    if game["number"].as_str() == Some(game_number.as_str()) {
                        Some(game.clone())
                    } else {
                        None
                    }
                })
            }) {
                game
            } else {
                error!(
                    "Game number {game_number} was not found in the schedule for event {event_id}"
                );
                return;
            };
            info!("Got game data for event: {event_id}, game number: {game_number} from uwhportal");
            let team_id_black = data_game["dark"]["assignment"]["teamId"]
                .as_str()
                .and_then(|s| TeamId::from_full(s).ok());
            let team_id_white = data_game["light"]["assignment"]["teamId"]
                .as_str()
                .and_then(|s| TeamId::from_full(s).ok());

            let pool = data["court"]
                .as_str()
                .map(|s| format!("COURT: {s}"))
                .unwrap_or_default();
            let start_time = data["startsOn"]
                .as_str()
                .and_then(|s| {
                    OffsetDateTime::parse(s, &uwh_common::uwhportal::schedule::FORMAT).ok()
                })
                .and_then(|dt| Some(String::from("START: ") + &dt.format(START_TIME_FORMAT).ok()?))
                .unwrap_or_default();
            let (referees, black, white) = tokio::join!(
                fetch_game_referees(uwhportal_url, &event_id, game_number),
                TeamInfoRaw::new(
                    uwhportal_url,
                    &event_id,
                    team_id_black.as_ref(),
                    Color::Black
                ),
                TeamInfoRaw::new(
                    uwhportal_url,
                    &event_id,
                    team_id_white.as_ref(),
                    Color::White
                )
            );

            let referees = match referees {
                Ok(r) => {
                    info!(
                        "Fetched referees for game {game_number}, there are {}",
                        r.len()
                    );
                    r
                }
                Err(e) => {
                    warn!(
                        "Couldn't fetch referees for event {event_id}, game number {game_number}: {e}"
                    );
                    Vec::new()
                }
            };

            game_data_tx
                .send(GameData {
                    pool,
                    start_time,
                    referees,
                    black,
                    white,
                    event_id,
                    game_number: game_number.clone(),
                })
                .map_err(|e| error!("Couldn't send data: {e}"))
                .unwrap();
            return;
        }
        warn!(
            "Game data request for event: {event_id}, game: {game_number} failed. Trying again in 5 seconds."
        );
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[tokio::main]
pub async fn networking_thread(
    state_tx: crossbeam_channel::Sender<StateUpdate>,
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

    let AppConfig {
        refbox_ip,
        refbox_port,
        uwhportal_url,
    } = config;

    let (snapshot_tx, mut snapshot_rx) = tokio::sync::mpsc::unbounded_channel::<GameSnapshot>();
    let (game_data_tx, mut game_data_rx) = tokio::sync::mpsc::unbounded_channel::<GameData>();
    let (logos_tx, mut logos_rx) = tokio::sync::mpsc::unbounded_channel::<EventLogos>();

    tokio::spawn(async move {
        let mut buff = vec![0u8; 1024];
        loop {
            info!("Connecting to refbox at {refbox_ip}:{refbox_port}");
            let mut stream = loop {
                match TcpStream::connect((refbox_ip, refbox_port)).await {
                    Ok(stream) => {
                        info!("Connected to refbox at {refbox_ip}:{refbox_port}");
                        break stream;
                    }
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            };
            info!("Connected to refbox at {refbox_ip}:{refbox_port}, waiting for snapshots...");

            loop {
                let read_bytes = match stream.read(&mut buff).await {
                    Ok(0) => {
                        error!("Connection to refbox lost! Attempting to reconnect!");
                        break;
                    }
                    Ok(bytes) => bytes,
                    Err(e) => {
                        error!("Error reading from refbox: {e}");
                        break;
                    }
                };
                match serde_json::de::from_slice::<GameSnapshot>(&buff[..read_bytes]) {
                    Ok(snapshot) => {
                        debug!("Got snapshot from refbox!");
                        snapshot_tx.send(snapshot.clone()).unwrap_or_else(|e| {
                            error!("Frontend could not recieve snapshot!: {e}")
                        });
                    }
                    Err(e) => {
                        warn!("Corrupted snapshot discarded! Error: {e}");
                    }
                }
            }
        }
    });

    let mut last_snapshot: Option<GameSnapshot> = None;
    let mut next_game_data: Option<GameData> = None;
    info!("Networking thread initialized!");
    loop {
        tokio::select! {
            snapshot = snapshot_rx.recv() => {
                let new_snapshot = if let Some(snapshot) = snapshot {
                    snapshot
                } else {
                    error!("Snapshot channel closed, exiting networking thread!");
                    return;
                };

                state_tx
                    .send(StateUpdate::Snapshot(new_snapshot.clone()))
                    .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));

                // initial case when no data is initialised
                if last_snapshot.is_none() {
                    if let Some(ref event_id) = new_snapshot.event_id {
                        info!("Fetching intial game data for event: {event_id}, game: {}", new_snapshot.game_number());
                        let event_id_ = event_id.clone();
                        let game_data_tx_ = game_data_tx.clone();
                        let uwhportal_url_ = uwhportal_url.clone();
                        let game_number = new_snapshot.game_number().clone();
                        tokio::spawn(async move {
                            fetch_game_data(game_data_tx_, &uwhportal_url_, event_id_, &game_number)
                                .await;
                        });

                        let logos_tx_ = logos_tx.clone();
                        let uwhportal_url_ = uwhportal_url.clone();
                        let event_id_ = event_id.clone();
                        tokio::spawn(async move {
                            logos_tx_.send(EventLogos::new(&uwhportal_url_, event_id_).await)
                                .map_err(|e| error!("Couldn't send event logos: {e}"))
                                .unwrap();
                        });
                    }
                }
                // every other case, when at least one game has been requested
                else if let Some(ref old_snapshot) = last_snapshot {
                    if old_snapshot.game_number() != new_snapshot.game_number() || old_snapshot.event_id != new_snapshot.event_id {
                        if old_snapshot.event_id != new_snapshot.event_id && new_snapshot.event_id.is_some() {
                            let logos_tx_ = logos_tx.clone();
                            let uwhportal_url_ = uwhportal_url.clone();
                            let event_id = new_snapshot.event_id.clone().unwrap();
                            tokio::spawn(async move {
                                logos_tx_.send(EventLogos::new(&uwhportal_url_, event_id).await)
                                    .map_err(|e| error!("Couldn't send event logos: {e}"))
                                    .unwrap();
                            });
                        }

                        info!("Got new game ID {} / event ID {:?}", new_snapshot.game_number(), new_snapshot.event_id);

                        if next_game_data.is_some()
                            && new_snapshot.event_id.is_some()
                            && next_game_data.as_ref().unwrap().game_number == *new_snapshot.game_number()
                            && next_game_data.as_ref().unwrap().event_id == *new_snapshot.event_id.as_ref().unwrap()
                        {
                            let next_game_data = next_game_data.take().unwrap();
                            info!("Sending cached game data for next game");
                            state_tx
                                .send(StateUpdate::GameData(next_game_data))
                                .unwrap_or_else(|e| {
                                    error!("Frontend could not recieve snapshot!: {e}")
                                });
                        } else if let Some(ref event_id) = new_snapshot.event_id {
                            let game_number = new_snapshot.game_number().clone();
                            info!(
                                "Fetching game data for event: {event_id}, game: {game_number}. Cache is empty or invalid!",
                            );
                            let game_data_tx_ = game_data_tx.clone();
                            let uwhportal_url_ = uwhportal_url.clone();
                            let event_id_ = event_id.clone();
                            tokio::spawn(async move {
                                fetch_game_data(
                                    game_data_tx_,
                                    &uwhportal_url_,
                                    event_id_,
                                    &game_number,
                                )
                                .await;
                            });
                        }
                    }
                }

                // request new game cache if empty or invalid
                if new_snapshot.event_id.is_some()
                    && new_snapshot.next_game_number().is_some()
                    && (next_game_data.is_none()
                        || next_game_data.as_ref().unwrap().game_number != *new_snapshot.next_game_number().unwrap()
                        || next_game_data.as_ref().unwrap().event_id != *new_snapshot.event_id.as_ref().unwrap())
                {
                    let game_data_tx_ = game_data_tx.clone();
                    let uwhportal_url_ = uwhportal_url.clone();
                    let event_id = new_snapshot.event_id.clone().unwrap();
                    let next_game_number = new_snapshot.next_game_number().unwrap().clone();
                    info!(
                        "Fetching game data to cache for event: {event_id}, game: {next_game_number}",
                    );
                    tokio::spawn(async move {
                        fetch_game_data(game_data_tx_, &uwhportal_url_, event_id, &next_game_number)
                            .await;
                    });
                }
                last_snapshot = Some(new_snapshot);
            }
            game_data = game_data_rx.recv() => {
                if let Some(game_data) = game_data {
                    info!("Got game data from network for game: {} / event: {:?}", game_data.game_number, game_data.event_id);
                    if let Some(ref snapshot) = last_snapshot {
                        if let Some(ref event_id) = snapshot.event_id {
                            if game_data.event_id == *event_id {
                                if game_data.game_number == *snapshot.game_number() {
                                    info!("Sending game data for event: {:?}, game: {}", event_id, game_data.game_number);
                                    state_tx
                                        .send(StateUpdate::GameData(game_data))
                                        .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
                                } else if let Some(next_game_number) = snapshot.next_game_number() {
                                    if game_data.game_number == *next_game_number {
                                        info!("Saving game data for next game: {} / event: {:?}", next_game_number, event_id);
                                        next_game_data = Some(game_data);
                                    } else {
                                        warn!("Received game data for event {:?}, but for unexpected game {:?}, discarding!", game_data.event_id, game_data.game_number);
                                    }
                                } else {
                                    warn!("Received game data for event {:?}, but for unexpected game {:?}, discarding!", game_data.event_id, game_data.game_number);
                                }
                            } else {
                                warn!("Received game data for event {:?}, but current snapshot is for event {:?}, discarding!", game_data.event_id, snapshot.event_id);
                            }
                        } else {
                            warn!("Received game data, but current snapshot has no event ID, discarding!");
                        }
                    } else {
                        warn!("Received game data before any snapshots, discarding!");
                    }
                } else {
                    error!("Game data channel closed, exiting networking thread!");
                    return;
                }
            }
            event_logos = logos_rx.recv() => {
                if let Some(event_logos) = event_logos {
                    info!("Got event logos from network!");
                    if let Some(ref snapshot) = last_snapshot {
                        if let Some(ref event_id) = snapshot.event_id {
                            if event_logos.event_id == *event_id {
                                state_tx
                                    .send(StateUpdate::EventLogos(event_id.clone(), event_logos))
                                    .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
                            } else {
                                warn!("Received event logos for event {:?}, but current snapshot is for event {:?}, discarding!", event_logos.event_id, snapshot.event_id);
                            }
                        } else {
                            warn!("Received event logos, but current snapshot has no event ID, discarding!");
                        }
                    } else {
                        warn!("Received event logos before any snapshots, discarding!");
                    }
                } else {
                    error!("Event logos channel closed, exiting networking thread!");
                    return;
                }
            }
        }
    }
}
