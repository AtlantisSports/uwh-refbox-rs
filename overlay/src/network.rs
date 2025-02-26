use log::{error, info, warn};
use reqwest::{Client, ClientBuilder, IntoUrl};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Read, net::TcpStream, sync::OnceLock, time::Duration};
use time::{OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};
use uwh_common::{
    color::Color,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::schedule::{EventId, TeamId},
};

const START_TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]");

static CLIENT_CELL: OnceLock<Client> = OnceLock::new();

pub type Image = (u16, u16, Vec<u8>);

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
pub struct EventLogos {
    pub event_logo: Option<Image>,
    pub sponsors: Option<Image>,
}

impl EventLogos {
    pub async fn new(uwhportal_url: &str, event_id: &EventId) -> Self {
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
            event_logo,
            sponsors,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StatePacket {
    pub snapshot: GameSnapshot,
    pub game_number: Option<u32>,
    pub data: Option<GameData>,
    pub event_logos: Option<EventLogos>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameData {
    pub pool: String,
    pub start_time: String,
    pub referees: Vec<MemberRaw>,
    pub black: TeamInfoRaw,
    pub white: TeamInfoRaw,
    pub sponsor_logo: Option<Image>,
    pub game_number: u32,
    pub event_id: Option<EventId>,
}

impl GameData {
    pub fn default(game_id: u32, event_id: Option<EventId>) -> Self {
        Self {
            pool: String::new(),
            start_time: String::new(),
            referees: Vec::new(),
            black: TeamInfoRaw::default(),
            white: TeamInfoRaw::default(),
            sponsor_logo: None,
            game_number: game_id,
            event_id,
        }
    }
}

async fn fetch_game_referees(
    uwhportal_url: &str,
    event_id: &EventId,
    game_number: u32,
) -> Result<Vec<MemberRaw>, reqwest::Error> {
    let client = CLIENT_CELL.get().unwrap();
    info!("Requesting Portal API for referees for (event, game): ({event_id}, {game_number})");
    let data: Value = client
        .get(format!("{uwhportal_url}/api/admin/events/game-referees"))
        .query(&[
            ("eventId", event_id.full()),
            ("gameNumber", &game_number.to_string()),
        ])
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
    tr: crossbeam_channel::Sender<(GameData, bool)>,
    uwhportal_url: &str,
    event_id: &EventId,
    game_number: u32,
    is_current_game: bool,
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
                    if game["number"].as_u64() == Some(game_number as u64) {
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
                fetch_game_referees(uwhportal_url, event_id, game_number),
                TeamInfoRaw::new(
                    uwhportal_url,
                    event_id,
                    team_id_black.as_ref(),
                    Color::Black
                ),
                TeamInfoRaw::new(
                    uwhportal_url,
                    event_id,
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

            tr.send((
                GameData {
                    pool,
                    start_time,
                    referees,
                    black,
                    white,
                    sponsor_logo: None,
                    event_id: Some(event_id.clone()),
                    game_number,
                },
                is_current_game,
            ))
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

    let (tr, rc) = crossbeam_channel::unbounded::<(GameData, bool)>();
    let (tt, rt) = crossbeam_channel::unbounded::<EventLogos>();
    let mut buff = vec![0u8; 1024];
    let mut read_bytes;
    let mut game_number = None;
    let mut event_id = None;
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
            let event_id_new = snapshot.event_id.clone();
            let game_number_new =
                if snapshot.current_period == GamePeriod::BetweenGames && !snapshot.is_old_game {
                    snapshot.next_game_number
                } else {
                    snapshot.game_number
                };
            let next_game_number = snapshot.next_game_number;

            let tr_ = tr.clone();
            let uwhportal_url = config.uwhportal_url.clone();

            // initial case when no data is initialised
            if game_number.is_none() {
                let tr_ = tr.clone();
                let uwhportal_url = config.uwhportal_url.clone();
                game_number = Some(game_number_new);
                event_id = event_id_new.clone();
                if let Some(ref id) = event_id {
                    info!("Fetching intial game data for event: {id}, game: {game_number_new}");
                    let id_ = id.clone();
                    tokio::spawn(async move {
                        fetch_game_data(tr_, &uwhportal_url, &id_, game_number_new, true).await;
                    });

                    let tt_ = tt.clone();
                    let uwhportal_url = config.uwhportal_url.clone();
                    let id_ = id.clone();
                    tokio::spawn(async move {
                        tt_.send(EventLogos::new(&uwhportal_url, &id_).await)
                            .map_err(|e| error!("Couldn't send event logos: {e}"))
                            .unwrap();
                    });
                }
            }

            // every other case, when atleast one game has been requested
            if game_number.is_some() && event_id.is_some() {
                let game_id_old = game_number.as_mut().unwrap();
                let tr_ = tr.clone();
                let uwhportal_url = config.uwhportal_url.clone();
                if *game_id_old != game_number_new || event_id != event_id_new {
                    if event_id != event_id_new && event_id_new.is_some() {
                        let tt_ = tt.clone();
                        let uwhportal_url = config.uwhportal_url.clone();
                        let id = event_id_new.clone().unwrap();
                        tokio::spawn(async move {
                            tt_.send(EventLogos::new(&uwhportal_url, &id).await)
                                .map_err(|e| error!("Couldn't send event logos: {e}"))
                                .unwrap();
                        });
                    }

                    *game_id_old = game_number_new;
                    event_id = event_id_new.clone();
                    info!("Got new game ID {game_number_new} / event ID {event_id_new:?}");

                    if next_game_data.is_some()
                        && next_game_data.as_ref().unwrap().game_number == game_number_new
                        && next_game_data.as_ref().unwrap().event_id == event_id_new
                    {
                        let next_game_data = next_game_data.clone().unwrap();
                        info!("Sending cached game data for next game");
                        tx.send(StatePacket {
                            snapshot,
                            game_number,
                            data: Some(next_game_data),
                            event_logos: None,
                        })
                        .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
                    } else if let Some(ref id) = event_id {
                        // This must succeed because of the `event_id.is_some()` check above
                        info!(
                            "Fetching game data for event: {id}, game: {game_number_new}. Cache is empty or invalid!"
                        );
                        let uwhportal_url_ = uwhportal_url.clone();
                        let tr__ = tr_.clone();
                        let id_ = id.clone();
                        tokio::spawn(async move {
                            fetch_game_data(tr__, &uwhportal_url_, &id_, game_number_new, true)
                                .await;
                        });
                    }
                    continue;
                }
            }

            // request new game cache if empty or invalid
            if event_id_new.is_some()
                && (next_game_data.is_none()
                    || next_game_data.as_ref().unwrap().game_number != next_game_number
                    || next_game_data.as_ref().unwrap().event_id != event_id_new)
            {
                info!(
                    "Fetching game data to cache for event: {}, game: {next_game_number}",
                    event_id_new.as_ref().unwrap()
                );
                let id = event_id_new.clone().unwrap();
                tokio::spawn(async move {
                    fetch_game_data(tr_, &uwhportal_url, &id, next_game_number, false).await;
                });
                next_game_data = Some(GameData::default(next_game_number, event_id_new.clone()));
            }

            let event_logos = if let Ok(event_logos) = rt.try_recv() {
                info!("Got event logos!");
                Some(event_logos)
            } else {
                None
            };

            // recieve data for requested games
            let this_game_data = if let Ok((game_data, is_current_game)) = rc.try_recv() {
                if is_current_game {
                    info!("Got game state update from network!");
                    Some(game_data)
                } else {
                    info!("Got game state update from network for next_game!");
                    next_game_data = Some(game_data);
                    None
                }
            } else {
                None
            };
            tx.send(StatePacket {
                snapshot,
                game_number,
                data: this_game_data,
                event_logos,
            })
            .unwrap_or_else(|e| error!("Frontend could not recieve snapshot!: {e}"));
        } else {
            warn!("Corrupted snapshot discarded!");
        }
    }
}
