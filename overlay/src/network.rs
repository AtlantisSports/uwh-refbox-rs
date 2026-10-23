use log::{debug, error, info, warn};
use reqwest::{Client, ClientBuilder, IntoUrl};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::OnceLock, time::Duration};
use time::{OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};
use tokio::net::TcpStream;
use uwh_common::{
    color::Color,
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{EventId, GameNumber, TeamId},
};

use crate::{AppConfig, feed::FeedReader};

const START_TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]");

static CLIENT_CELL: OnceLock<Client> = OnceLock::new();

pub type Image = (u16, u16, Vec<u8>);

pub const BLACK_TEAM_NAME: &str = "BLACK";
pub const WHITE_TEAM_NAME: &str = "WHITE";

/// The HTTP client every portal request goes through, built the same way the direct
/// networking thread builds it.
///
/// The `expect` is the direct thread's existing one, moved here unchanged. Building only
/// fails if the TLS backend cannot start, a broken install with no portal access at all.
fn build_http_client() -> Client {
    ClientBuilder::new()
        .connect_timeout(Duration::from_secs(20))
        .build()
        .expect("Couldn't create HTTP client!")
}

/// Make sure the shared HTTP client exists. Safe to call more than once: later calls
/// leave the first client in place.
#[cfg(feature = "bridge")]
pub fn init_http_client() {
    CLIENT_CELL.get_or_init(build_http_client);
}

/// Why a one-attempt portal fetch produced nothing.
#[cfg(feature = "bridge")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalFetchError {
    /// A network error, or a reply that was not the expected JSON. Worth retrying later.
    Unreachable,
    /// The schedule loaded but holds no game with that number. Not worth retrying.
    NotInSchedule,
}

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
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct MemberRaw {
    pub name: String,
    pub role: Option<String>,
    pub number: Option<u8>,
    pub picture: Option<Image>,
    pub geared_picture: Option<Image>,
}
/// Contains information about team. `flag` here is a byte array for `Serialize`, which is
/// processed into `Texture2D` when struct is converted into `TeamInfo`
#[derive(Serialize, Deserialize, Clone, Default, PartialEq)]
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

        team_from_json(&data, team_color).await
    }

    /// One attempt at fetching a team. Never panics: a failed request or a reply that is
    /// not JSON comes back as `PortalFetchError::Unreachable`.
    #[cfg(feature = "bridge")]
    pub async fn try_new(
        uwhportal_url: &str,
        team_id: Option<&TeamId>,
        team_color: Color,
    ) -> Result<Self, PortalFetchError> {
        let team_id = match team_id {
            Some(id) => id,
            None => return Ok(Self::default()),
        };

        let client = CLIENT_CELL.get_or_init(build_http_client);
        info!("Requesting UWH API for team information for team: {team_id}");
        let text = client
            .get(format!("{uwhportal_url}/api/admin/get-event-team"))
            .query(&[("teamId", team_id.full())])
            .send()
            .await
            .map_err(|e| {
                warn!("Couldn't request team data for team {team_id}: {e}");
                PortalFetchError::Unreachable
            })?
            .text()
            .await
            .map_err(|e| {
                warn!("Couldn't get team data body for team {team_id}: {e}");
                PortalFetchError::Unreachable
            })?;
        let data: Value = serde_json::from_str(&text).map_err(|e| {
            warn!("Team data for team {team_id} was not valid JSON: {e}");
            PortalFetchError::Unreachable
        })?;

        Ok(team_from_json(&data, team_color).await)
    }
}

/// Build a team from a `get-event-team` reply, fetching its pictures. Shared by the
/// panicking and the fallible team fetchers.
async fn team_from_json(data: &Value, team_color: Color) -> TeamInfoRaw {
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

    TeamInfoRaw {
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

        logos_from_json(&data, event_id).await
    }

    /// One attempt at fetching an event's logos. Never panics: a failed request or a reply
    /// that is not JSON comes back as `PortalFetchError::Unreachable`.
    #[cfg(feature = "bridge")]
    pub async fn try_new(uwhportal_url: &str, event_id: EventId) -> Result<Self, PortalFetchError> {
        let client = CLIENT_CELL.get_or_init(build_http_client);
        info!("Requesting Portal API for overlay images for event: {event_id}");
        let text = client
            .get(format!(
                "{uwhportal_url}/api/admin/events/{}/overlay-attachments",
                event_id.partial()
            ))
            .send()
            .await
            .map_err(|e| {
                warn!("Couldn't request event images for event {event_id}: {e}");
                PortalFetchError::Unreachable
            })?
            .text()
            .await
            .map_err(|e| {
                warn!("Couldn't get event images body for event {event_id}: {e}");
                PortalFetchError::Unreachable
            })?;
        let data: Value = serde_json::from_str(&text).map_err(|e| {
            warn!("Event images for event {event_id} were not valid JSON: {e}");
            PortalFetchError::Unreachable
        })?;

        Ok(logos_from_json(&data, event_id).await)
    }
}

/// Build an event's logos from an `overlay-attachments` reply, fetching the images. Shared
/// by the panicking and the fallible logo fetchers.
async fn logos_from_json(data: &Value, event_id: EventId) -> EventLogos {
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

    EventLogos {
        event_id,
        event_logo,
        sponsors,
    }
}

#[derive(Serialize, Deserialize)]
pub enum StateUpdate {
    Snapshot(GameSnapshot),
    GameData(GameData),
    EventLogos(EventId, EventLogos),
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct GameData {
    pub pool: String,
    pub start_time: String,
    pub referees: Vec<MemberRaw>,
    pub black: TeamInfoRaw,
    pub white: TeamInfoRaw,
    pub game_number: GameNumber,
    /// `None` when there is no portal id at all -- the no-portal/local-CSV case
    /// (`bridge_network.rs`), where matching falls back to game number alone (see
    /// `State::update_state`'s `GameData` handling in `main.rs`). Always `Some` on this
    /// (portal-connected) path: fetching game data at all requires a real event id.
    pub event_id: Option<EventId>,
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

/// The parts of a public schedule response the overlay puts on screen for one game.
struct GameDisplayInfo {
    team_id_black: Option<TeamId>,
    team_id_white: Option<TeamId>,
    pool: String,
    start_time: String,
}

/// Pick `game_number` out of a public schedule response and build its display values.
///
/// Returns `None` if the schedule holds no game with that number.
fn parse_game_display_info(data: &Value, game_number: &GameNumber) -> Option<GameDisplayInfo> {
    let data_game = data["games"]
        .as_array()?
        .iter()
        .find(|game| game["number"].as_str() == Some(game_number.as_str()))?;

    let team_id_black = data_game["dark"]["assignment"]["teamId"]
        .as_str()
        .and_then(|s| TeamId::from_full(s).ok());
    let team_id_white = data_game["light"]["assignment"]["teamId"]
        .as_str()
        .and_then(|s| TeamId::from_full(s).ok());

    let pool = data_game["court"]
        .as_str()
        .map(|s| format!("COURT: {s}"))
        .unwrap_or_default();
    let start_time = data_game["startsOn"]
        .as_str()
        .and_then(|s| OffsetDateTime::parse(s, &uwh_common::uwhportal::schedule::FORMAT).ok())
        .and_then(|dt| Some(String::from("START: ") + &dt.format(START_TIME_FORMAT).ok()?))
        .unwrap_or_default();

    Some(GameDisplayInfo {
        team_id_black,
        team_id_white,
        pool,
        start_time,
    })
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
                .expect("Response body could not be received!");
            let data: Value = match serde_json::from_str(text.as_str()) {
                Ok(d) => d,
                _ => {
                    error!(
                        "Aborting schedule fetch! Server did not return valid JSON for event {event_id}: {text}"
                    );
                    return;
                }
            };
            let Some(GameDisplayInfo {
                team_id_black,
                team_id_white,
                pool,
                start_time,
            }) = parse_game_display_info(&data, game_number)
            else {
                error!(
                    "Game number {game_number} was not found in the schedule for event {event_id}"
                );
                return;
            };
            info!("Got game data for event: {event_id}, game number: {game_number} from uwhportal");
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
                    event_id: Some(event_id),
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

/// One attempt at fetching a game's display data: the same requests and the same result
/// as `fetch_game_data`, but it never panics, never retries and never sends. A failed
/// referee fetch gives an empty referee list, as on the direct path.
#[cfg(feature = "bridge")]
pub async fn try_fetch_game_data(
    uwhportal_url: &str,
    event_id: &EventId,
    game_number: &GameNumber,
) -> Result<GameData, PortalFetchError> {
    let client = CLIENT_CELL.get_or_init(build_http_client);
    let text = client
        .get(format!(
            "{uwhportal_url}/api/events/{}/schedule",
            event_id.partial()
        ))
        .send()
        .await
        .map_err(|e| {
            warn!("Game data request for event: {event_id}, game: {game_number} failed: {e}");
            PortalFetchError::Unreachable
        })?
        .text()
        .await
        .map_err(|e| {
            warn!("Schedule body for event {event_id} could not be received: {e}");
            PortalFetchError::Unreachable
        })?;
    let data: Value = serde_json::from_str(text.as_str()).map_err(|_| {
        error!("Server did not return valid JSON for event {event_id}: {text}");
        PortalFetchError::Unreachable
    })?;
    let Some(GameDisplayInfo {
        team_id_black,
        team_id_white,
        pool,
        start_time,
    }) = parse_game_display_info(&data, game_number)
    else {
        error!("Game number {game_number} was not found in the schedule for event {event_id}");
        return Err(PortalFetchError::NotInSchedule);
    };
    info!("Got game data for event: {event_id}, game number: {game_number} from uwhportal");
    let (referees, black, white) = tokio::join!(
        fetch_game_referees(uwhportal_url, event_id, game_number),
        TeamInfoRaw::try_new(uwhportal_url, team_id_black.as_ref(), Color::Black),
        TeamInfoRaw::try_new(uwhportal_url, team_id_white.as_ref(), Color::White)
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
            warn!("Couldn't fetch referees for event {event_id}, game number {game_number}: {e}");
            Vec::new()
        }
    };

    Ok(GameData {
        pool,
        start_time,
        referees,
        black: black?,
        white: white?,
        event_id: Some(event_id.clone()),
        game_number: game_number.clone(),
    })
}

#[tokio::main]
pub async fn networking_thread(
    state_tx: crossbeam_channel::Sender<StateUpdate>,
    config: crate::AppConfig,
) {
    CLIENT_CELL.set(build_http_client()).unwrap();

    let AppConfig {
        refbox_ip,
        refbox_port,
        uwhportal_url,
        // Only used by `bridge_network.rs`'s alternate networking path.
        bridge_url: _,
        bridge_port_moved: _,
        refbox_port_moved: _,
    } = config;

    let (snapshot_tx, mut snapshot_rx) = tokio::sync::mpsc::unbounded_channel::<GameSnapshot>();
    let (game_data_tx, mut game_data_rx) = tokio::sync::mpsc::unbounded_channel::<GameData>();
    let (logos_tx, mut logos_rx) = tokio::sync::mpsc::unbounded_channel::<EventLogos>();

    tokio::spawn(async move {
        loop {
            info!("Connecting to refbox at {refbox_ip}:{refbox_port}");
            let stream = loop {
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

            // A new reader per connection: bytes left buffered part-way through a line when a
            // connection dies must never be prepended to the next connection's first line.
            let mut reader = FeedReader::new(stream);

            loop {
                match reader.next_line().await {
                    Ok(Some(line)) => match serde_json::de::from_slice::<GameSnapshot>(&line) {
                        Ok(snapshot) => {
                            debug!("Got snapshot from refbox!");
                            snapshot_tx.send(snapshot).unwrap_or_else(|e| {
                                error!("Frontend could not receive snapshot!: {e}")
                            });
                        }
                        Err(e) => {
                            warn!("Corrupted snapshot discarded! Error: {e}");
                        }
                    },
                    Ok(None) => {
                        error!("Connection to refbox lost! Attempting to reconnect!");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from refbox: {e}");
                        break;
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
                    .unwrap_or_else(|e| error!("Frontend could not receive snapshot!: {e}"));

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
                            && next_game_data.as_ref().unwrap().event_id.as_ref() == new_snapshot.event_id.as_ref()
                        {
                            let next_game_data = next_game_data.take().unwrap();
                            info!("Sending cached game data for next game");
                            state_tx
                                .send(StateUpdate::GameData(next_game_data))
                                .unwrap_or_else(|e| {
                                    error!("Frontend could not receive snapshot!: {e}")
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
                        || next_game_data.as_ref().unwrap().event_id.as_ref() != new_snapshot.event_id.as_ref())
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
                            if game_data.event_id.as_ref() == Some(event_id) {
                                if game_data.game_number == *snapshot.game_number() {
                                    info!("Sending game data for event: {:?}, game: {}", event_id, game_data.game_number);
                                    state_tx
                                        .send(StateUpdate::GameData(game_data))
                                        .unwrap_or_else(|e| error!("Frontend could not receive snapshot!: {e}"));
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
                                    .unwrap_or_else(|e| error!("Frontend could not receive snapshot!: {e}"));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from a real `GET /api/events/{eventId}/schedule` response
    /// (dev portal, event `1889-B`, fetched 2026-08-15). The top level carries
    /// no `court` and no `startsOn` — both only exist per game.
    const PUBLIC_SCHEDULE: &str = r#"{
      "courtNames": ["1", "2"],
      "games": [
        {
          "number": "1",
          "court": "1",
          "startsOn": "2026-08-01T09:30:00+10:00",
          "dark": { "assignment": { "teamId": "teams/2529-B" } },
          "light": { "assignment": { "teamId": "teams/2530-B" } }
        },
        {
          "number": "4",
          "court": "2",
          "startsOn": "2026-08-01T09:55:00+10:00",
          "dark": { "assignment": { "teamId": "teams/2531-B" } },
          "light": { "assignment": { "teamId": "teams/2532-B" } }
        }
      ]
    }"#;

    #[test]
    fn court_and_start_come_from_the_matched_game() {
        let data: Value = serde_json::from_str(PUBLIC_SCHEDULE).unwrap();

        let info = parse_game_display_info(&data, &"4".to_string()).unwrap();

        assert_eq!(info.pool, "COURT: 2");
        assert_eq!(info.start_time, "START: 09:55");
    }

    #[test]
    fn an_unknown_game_number_yields_nothing() {
        let data: Value = serde_json::from_str(PUBLIC_SCHEDULE).unwrap();

        assert!(parse_game_display_info(&data, &"99".to_string()).is_none());
    }

    /// Nothing listens on port 1, so every request to it fails at once.
    #[cfg(feature = "bridge")]
    const UNREACHABLE_PORTAL: &str = "http://127.0.0.1:1";

    #[cfg(feature = "bridge")]
    #[tokio::test]
    async fn team_from_json_reads_names_numbers_roles_and_uppercases_the_team_name() {
        init_http_client();
        let data: Value = serde_json::from_str(
            r#"{"name":" Team A ","roster":[{"rosterName":" Ann ","capNumber":3,"roles":["Player","Captain"]},{"rosterName":"Coach Bob","roles":["Coach"]}]}"#,
        )
        .unwrap();

        let team = team_from_json(&data, Color::Black).await;

        assert_eq!(team.team_name, "TEAM A");
        let members: Vec<_> = team
            .members
            .iter()
            .map(|m| (m.name.as_str(), m.number, m.role.as_deref()))
            .collect();
        assert_eq!(
            members,
            vec![
                ("Ann", Some(3), Some("Captain")),
                ("Coach Bob", None, Some("Coach")),
            ]
        );
        assert!(team.flag.is_none());
        assert!(
            team.members
                .iter()
                .all(|m| m.picture.is_none() && m.geared_picture.is_none())
        );
    }

    #[cfg(feature = "bridge")]
    #[tokio::test]
    async fn try_fetch_game_data_reports_unreachable_instead_of_panicking() {
        init_http_client();
        let event_id = EventId::from_full("events/1889-B").unwrap();

        let result = try_fetch_game_data(UNREACHABLE_PORTAL, &event_id, &"1".to_string()).await;

        assert!(result == Err(PortalFetchError::Unreachable));
    }

    #[cfg(feature = "bridge")]
    #[tokio::test]
    async fn team_try_new_reports_unreachable_instead_of_panicking() {
        init_http_client();
        let team_id = TeamId::from_full("teams/2529-B").unwrap();

        let result = TeamInfoRaw::try_new(UNREACHABLE_PORTAL, Some(&team_id), Color::Black).await;

        assert!(result == Err(PortalFetchError::Unreachable));
    }

    #[cfg(feature = "bridge")]
    #[tokio::test]
    async fn team_try_new_with_no_team_id_is_the_default_team() {
        init_http_client();

        let result = TeamInfoRaw::try_new(UNREACHABLE_PORTAL, None, Color::White).await;

        assert!(result == Ok(TeamInfoRaw::default()));
    }

    #[cfg(feature = "bridge")]
    #[tokio::test]
    async fn event_logos_try_new_reports_unreachable_instead_of_panicking() {
        init_http_client();
        let event_id = EventId::from_full("events/1889-B").unwrap();

        let result = EventLogos::try_new(UNREACHABLE_PORTAL, event_id).await;

        assert!(matches!(result, Err(PortalFetchError::Unreachable)));
    }
}
