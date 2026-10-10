//! Puts each game's YouTube watch link on the portal's schedule: the saved access key for the
//! linked portal event, swapping the portal's code for that key, and sending the links.
//!
//! The key is a secret. It is kept only in its own file (never in the settings), and is never
//! put in a log line or an error message.

use crate::{
    BoxError,
    config::Config,
    portal::{self, EventPlan},
    prepare::{self, EventState},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Duration};

/// The file, next to the settings, holding the access key for the linked portal event.
pub const LINK_FILE: &str = "portal-watch-links.json";

/// How much of a portal reply goes into an error message.
const REPLY_CHARS: usize = 200;

/// How long Prepare's preview waits for the key check. It only decides whether to warn, so a
/// slow portal mustn't hold up the preview.
const KEY_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// This Stream Manager's link to one portal event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalLink {
    pub portal_url: String,
    pub event_slug: String,
    pub access_key: String,
}

/// The saved link, if there is one for this portal and event. A missing, unreadable or
/// unparsable file, or one for another portal or event, is `None`.
pub fn link_for(path: &Path, portal_url: &str, event_slug: &str) -> Option<PortalLink> {
    let text = std::fs::read_to_string(path).ok()?;
    let link: PortalLink = serde_json::from_str(&text).ok()?;
    (link.portal_url == portal_url && link.event_slug == event_slug).then_some(link)
}

/// Saves the link with `prepare::write_atomically`.
pub fn save(path: &Path, link: &PortalLink) -> Result<(), BoxError> {
    prepare::write_atomically(path, &serde_json::to_string_pretty(link)?)
}

/// Deletes the file; a file that isn't there is fine.
pub fn forget(path: &Path) -> Result<(), BoxError> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Why the portal didn't give a key for a code.
#[derive(Debug, PartialEq, Eq)]
pub enum LinkError {
    /// The event has no link waiting for this Stream Manager ID.
    NoPendingLink,
    /// The code isn't the one the portal shows.
    InvalidCode,
    /// Anything else, as text a person can read.
    Other(String),
}

/// Swaps the portal's code for a key.
pub async fn exchange_code(
    portal_url: &str,
    event_slug: &str,
    stream_manager_id: &str,
    code: &str,
) -> Result<String, LinkError> {
    let url = format!(
        "{}/api/events/{event_slug}/access-keys/stream-manager",
        portal_url.trim_end_matches('/')
    );
    let client = crate::http_client().map_err(|e| LinkError::Other(unreachable_text(e)))?;
    let response = client
        .post(&url)
        .json(&json!({ "streamManagerId": stream_manager_id, "code": code }))
        .send()
        .await
        .map_err(|e| LinkError::Other(unreachable_text(e)))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| LinkError::Other(unreachable_text(e)))?;
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status == StatusCode::OK {
        return body["accessKey"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| LinkError::Other("The portal's reply had no access key".to_string()));
    }
    if status == StatusCode::BAD_REQUEST {
        match body["reason"].as_str() {
            Some("NoPendingLink") => return Err(LinkError::NoPendingLink),
            Some("InvalidCode") => return Err(LinkError::InvalidCode),
            _ => {}
        }
    }
    Err(LinkError::Other(portal_said(status, &text)))
}

/// Why the portal didn't take the links.
#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    /// The portal no longer accepts the key (401 or 403).
    KeyRefused,
    /// Anything else, as text a person can read.
    Other(String),
}

/// One PUT of `links` (game number → link, `None` clears it).
pub async fn send_watch_urls(
    portal_url: &str,
    event_slug: &str,
    access_key: &str,
    links: &BTreeMap<String, Option<String>>,
) -> Result<(), SendError> {
    if links.is_empty() {
        return Ok(());
    }
    put_watch_urls(portal_url, event_slug, access_key, links).await
}

/// The PUT itself. An empty `links` changes nothing but still runs the portal's key check.
async fn put_watch_urls(
    portal_url: &str,
    event_slug: &str,
    access_key: &str,
    links: &BTreeMap<String, Option<String>>,
) -> Result<(), SendError> {
    let url = format!(
        "{}/api/events/{event_slug}/schedule/watch-urls",
        portal_url.trim_end_matches('/')
    );
    let client = crate::http_client().map_err(|e| SendError::Other(unreachable_text(e)))?;
    let response = client
        .put(&url)
        .bearer_auth(access_key)
        .json(&json!({ "watchUrlsByGameNumber": links }))
        .send()
        .await
        .map_err(|e| SendError::Other(unreachable_text(e)))?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(SendError::KeyRefused);
    }
    let text = response
        .text()
        .await
        .map_err(|e| SendError::Other(unreachable_text(e)))?;
    Err(SendError::Other(portal_said(status, &text)))
}

/// The watch link for a YouTube video.
pub fn watch_url(broadcast_id: &str) -> String {
    format!("https://youtu.be/{broadcast_id}")
}

/// What to send after Prepare: every recorded video whose game is on `plan`, as its link.
/// Games no longer on the schedule are left out (the portal would refuse the whole update).
pub fn links_after_prepare(
    plan: &EventPlan,
    state: &EventState,
) -> BTreeMap<String, Option<String>> {
    plan.games
        .iter()
        .filter_map(|game| {
            let video = state.videos.get(&game.number)?;
            Some((game.number.clone(), Some(watch_url(&video.broadcast_id))))
        })
        .collect()
}

/// What to send after deleting videos: `None` for each deleted game that is on `plan`.
pub fn links_after_cleanup(
    plan: &EventPlan,
    deleted: &[String],
) -> BTreeMap<String, Option<String>> {
    plan.games
        .iter()
        .filter(|game| deleted.contains(&game.number))
        .map(|game| (game.number.clone(), None))
        .collect()
}

/// Whether the links being sent set or clear them; only changes the log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Set,
    Clear,
}

/// Sends `links` with the saved key for `config`'s portal and event, and logs one line (none
/// when there is nothing to send). Never fails: a portal problem is only that log line. A key
/// the portal refuses is forgotten, so the next run doesn't try it again.
pub async fn publish(
    link_file: &Path,
    config: &Config,
    links: &BTreeMap<String, Option<String>>,
    purpose: Purpose,
    log: &mut (dyn FnMut(String) + Send),
) {
    if links.is_empty() {
        return;
    }
    let Some(link) = link_for(link_file, &config.portal_url, &config.event_slug) else {
        log(match purpose {
            Purpose::Set => "Portal watch links: this Stream Manager isn't linked to the event, so none were set (Settings → Portal watch links)",
            Purpose::Clear => "Portal watch links: this Stream Manager isn't linked to the event, so the deleted games' links weren't cleared (Settings → Portal watch links)",
        }.to_string());
        return;
    };
    let result = send_watch_urls(&link.portal_url, &link.event_slug, &link.access_key, links).await;
    match (result, purpose) {
        (Ok(()), Purpose::Set) => log(format!("Portal: watch links set for {} games", links.len())),
        (Ok(()), Purpose::Clear) => log(format!(
            "Portal: watch links cleared for {} games",
            links.len()
        )),
        (Err(SendError::KeyRefused), _) => {
            forget_refused(link_file, &link);
            log("Portal: the event refused this Stream Manager's key (it was removed on the portal, or the event is over). Link it again in Settings to set watch links.".to_string());
        }
        (Err(SendError::Other(error)), Purpose::Set) => log(format!(
            "Portal: watch links weren't set: {error}. Run Prepare again to send them."
        )),
        (Err(SendError::Other(error)), Purpose::Clear) => {
            log(format!("Portal: watch links weren't cleared: {error}"))
        }
    }
}

/// What Prepare's preview says about setting watch links, checked before anything is created
/// so a refused key is known before any YouTube units are spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCheck {
    /// The portal accepts the saved key.
    Accepted,
    /// No key is saved for this portal and event.
    NotLinked,
    /// The portal refused the saved key; it has been forgotten.
    Refused,
    /// The portal couldn't be asked, as text a person can read.
    Unknown(String),
}

impl KeyCheck {
    /// What to tell the operator, if anything, and whether it is a warning.
    pub fn notice(&self) -> Option<(bool, String)> {
        match self {
            KeyCheck::Accepted => None,
            KeyCheck::NotLinked => Some((
                false,
                "Portal watch links: this Stream Manager isn't linked to the event, so none will \
                 be set (Settings → Portal watch links)."
                    .to_string(),
            )),
            KeyCheck::Refused => Some((
                true,
                "Portal watch links: the event refused this Stream Manager's key (it was removed \
                 on the portal, or the event is over), so watch links won't be set. Link it again \
                 in Settings first, or go ahead without watch links."
                    .to_string(),
            )),
            KeyCheck::Unknown(error) => Some((
                false,
                format!(
                    "Portal watch links: couldn't check the link ({error}). Prepare will still \
                     try to set them."
                ),
            )),
        }
    }
}

/// Asks the portal whether it still accepts the saved key, by sending an update with no
/// links: the portal runs the same key check as for a real update and changes nothing. A
/// refused key is forgotten, as after a refused update.
pub async fn check_key(link_file: &Path, config: &Config) -> KeyCheck {
    check_key_within(link_file, config, KEY_CHECK_TIMEOUT).await
}

async fn check_key_within(link_file: &Path, config: &Config, timeout: Duration) -> KeyCheck {
    let Some(link) = link_for(link_file, &config.portal_url, &config.event_slug) else {
        return KeyCheck::NotLinked;
    };
    let empty = BTreeMap::new();
    let request = put_watch_urls(&link.portal_url, &link.event_slug, &link.access_key, &empty);
    match tokio::time::timeout(timeout, request).await {
        Err(_) => KeyCheck::Unknown(format!(
            "the portal didn't answer within {} seconds",
            timeout.as_secs()
        )),
        Ok(Ok(())) => KeyCheck::Accepted,
        Ok(Err(SendError::KeyRefused)) => {
            forget_refused(link_file, &link);
            KeyCheck::Refused
        }
        Ok(Err(SendError::Other(error))) => KeyCheck::Unknown(error),
    }
}

/// Forgets `refused` after the portal refused it, but only if it is still the saved key: a key
/// saved by linking again meanwhile must survive.
fn forget_refused(link_file: &Path, refused: &PortalLink) {
    let still_saved = link_for(link_file, &refused.portal_url, &refused.event_slug)
        .is_some_and(|saved| saved.access_key == refused.access_key);
    if still_saved {
        // If the file can't be removed, the next run is refused again and says so again.
        let _ = forget(link_file);
    }
}

/// After Prepare (whether it worked or failed): reloads the event's record from `state_file`
/// and sends a link for each recorded game on `plan`. Never fails; see [`publish`].
pub async fn publish_after_prepare(
    link_file: &Path,
    config: &Config,
    plan: &EventPlan,
    state_file: &Path,
    log: &mut (dyn FnMut(String) + Send),
) {
    match prepare::load_state(state_file, &config.event_slug) {
        Ok(state) => {
            let links = links_after_prepare(plan, &state);
            publish(link_file, config, &links, Purpose::Set, log).await;
        }
        Err(e) => log(format!("Portal: watch links weren't set: {e}")),
    }
}

/// After deleting videos (whether it finished or stopped half-way): clears the links of the
/// `deleted` games. Uses `loaded_plan` (the schedule of `config`'s event, if it is loaded),
/// otherwise fetches the schedule from the portal. Does nothing when nothing was deleted.
/// Never fails; see [`publish`].
pub async fn publish_after_cleanup(
    link_file: &Path,
    config: &Config,
    loaded_plan: Option<EventPlan>,
    deleted: &[String],
    log: &mut (dyn FnMut(String) + Send),
) {
    if deleted.is_empty() {
        return;
    }
    let plan = match loaded_plan {
        Some(plan) => Ok(plan),
        None => portal::fetch_event_plan(&config.portal_url, &config.event_slug).await,
    };
    match plan {
        Ok(plan) => {
            let links = links_after_cleanup(&plan, deleted);
            publish(link_file, config, &links, Purpose::Clear, log).await;
        }
        Err(e) => log(format!(
            "Portal: watch links weren't cleared: the schedule couldn't be loaded ({e})"
        )),
    }
}

fn unreachable_text(error: impl std::fmt::Display) -> String {
    format!("Couldn't reach the portal: {error}")
}

fn portal_said(status: StatusCode, body: &str) -> String {
    let start: String = body.chars().take(REPLY_CHARS).collect();
    format!("The portal said {status}: {start}")
}

/// A stand-in portal for tests: answers every request with a status and body the test chooses,
/// and records what it was sent.
#[cfg(test)]
pub(crate) mod test_portal {
    use axum::{
        Router,
        body::Bytes,
        extract::State,
        http::{HeaderMap, StatusCode, Uri, header},
        response::{IntoResponse, Response},
    };
    use serde_json::Value;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        time::Duration,
    };

    /// One request the stand-in portal was sent.
    #[derive(Debug, Clone)]
    pub(crate) struct Request {
        pub(crate) method: String,
        pub(crate) path: String,
        /// The `Authorization` header, if there was one.
        pub(crate) authorization: Option<String>,
        /// The JSON body, or `Null` when there was none.
        pub(crate) body: Value,
    }

    #[derive(Default)]
    struct Shared {
        default_reply: (u16, String),
        replies_by_path: HashMap<String, (u16, String)>,
        requests: Vec<Request>,
        /// How long to wait before each reply.
        delay: Duration,
    }

    pub(crate) struct MockPortal {
        base_url: String,
        shared: Arc<Mutex<Shared>>,
    }

    impl MockPortal {
        /// Starts a portal on 127.0.0.1 that answers every path with `status` and `body`.
        pub(crate) async fn start(status: u16, body: &str) -> Self {
            let shared = Arc::new(Mutex::new(Shared {
                default_reply: (status, body.to_string()),
                ..Shared::default()
            }));
            let router = Router::new()
                .fallback(answer)
                .with_state(Arc::clone(&shared));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(listener, router).await });
            Self { base_url, shared }
        }

        /// The address to use as the portal URL.
        pub(crate) fn base_url(&self) -> &str {
            &self.base_url
        }

        /// From now on, answers `path` (e.g. `/api/events/cup/schedule/watch-urls`) with
        /// `status` and `body`; other paths keep their reply.
        pub(crate) fn reply_on(&self, path: &str, status: u16, body: &str) {
            self.shared
                .lock()
                .unwrap()
                .replies_by_path
                .insert(path.to_string(), (status, body.to_string()));
        }

        /// From now on, waits `delay` before each reply (the request is recorded at once).
        pub(crate) fn delay_replies(&self, delay: Duration) {
            self.shared.lock().unwrap().delay = delay;
        }

        /// Every request so far, oldest first.
        pub(crate) fn requests(&self) -> Vec<Request> {
            self.shared.lock().unwrap().requests.clone()
        }
    }

    async fn answer(
        State(shared): State<Arc<Mutex<Shared>>>,
        method: axum::http::Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> Response {
        let (status, reply, delay) = {
            let mut shared = shared.lock().unwrap();
            shared.requests.push(Request {
                method: method.to_string(),
                path: uri.path().to_string(),
                authorization: headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string),
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });
            let (status, reply) = shared
                .replies_by_path
                .get(uri.path())
                .unwrap_or(&shared.default_reply)
                .clone();
            (status, reply, shared.delay)
        };
        tokio::time::sleep(delay).await;
        let status = StatusCode::from_u16(status).unwrap();
        (status, [(header::CONTENT_TYPE, "application/json")], reply).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::test_portal::MockPortal;
    use super::*;

    const EXCHANGE_PATH: &str = "/api/events/cup-2026/access-keys/stream-manager";
    const SEND_PATH: &str = "/api/events/cup-2026/schedule/watch-urls";
    const KEY: &str = "secret-portal-key";

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "stream-manager-watch-links-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn links() -> BTreeMap<String, Option<String>> {
        BTreeMap::from([
            ("14".to_string(), Some(watch_url("abc123"))),
            ("15".to_string(), None),
        ])
    }

    #[test]
    fn the_watch_url_is_the_short_youtube_link() {
        assert_eq!(watch_url("abc123"), "https://youtu.be/abc123");
    }

    #[tokio::test]
    async fn exchanging_a_code_returns_the_key_and_sends_the_id_and_code() {
        let portal = MockPortal::start(200, r#"{"accessKey":"the-key"}"#).await;
        let key = exchange_code(
            &format!("{}/", portal.base_url()),
            "cup-2026",
            "123456",
            "9876",
        )
        .await;
        assert_eq!(key, Ok("the-key".to_string()));
        let requests = portal.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].path, EXCHANGE_PATH);
        assert_eq!(requests[0].authorization, None);
        assert_eq!(
            requests[0].body,
            json!({ "streamManagerId": "123456", "code": "9876" })
        );
    }

    #[tokio::test]
    async fn exchanging_a_code_maps_the_portals_refusals() {
        let cases = [
            (
                400,
                r#"{"reason":"NoPendingLink"}"#,
                Some(LinkError::NoPendingLink),
            ),
            (
                400,
                r#"{"reason":"InvalidCode"}"#,
                Some(LinkError::InvalidCode),
            ),
            (400, "", None),
            (500, "boom", None),
        ];
        for (status, body, expected) in cases {
            let portal = MockPortal::start(status, body).await;
            let result = exchange_code(portal.base_url(), "cup-2026", "123456", "9876").await;
            match expected {
                Some(error) => assert_eq!(result, Err(error)),
                None => match result {
                    Err(LinkError::Other(text)) => {
                        assert!(text.starts_with("The portal said"), "{text}");
                        assert!(text.contains(&status.to_string()), "{text}");
                    }
                    other => panic!("{status}: {other:?}"),
                },
            }
        }
    }

    #[tokio::test]
    async fn an_unreachable_portal_says_so() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let result = exchange_code(&format!("http://{address}"), "cup-2026", "1", "2").await;
        match result {
            Err(LinkError::Other(text)) => {
                assert!(text.starts_with("Couldn't reach the portal: "), "{text}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn sending_links_uses_the_key_and_the_exact_body() {
        let portal = MockPortal::start(500, "not this path").await;
        portal.reply_on(SEND_PATH, 204, "");
        let result = send_watch_urls(portal.base_url(), "cup-2026", KEY, &links()).await;
        assert_eq!(result, Ok(()));
        let requests = portal.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "PUT");
        assert_eq!(requests[0].path, SEND_PATH);
        assert_eq!(
            requests[0].authorization.as_deref(),
            Some(format!("Bearer {KEY}").as_str())
        );
        assert_eq!(
            requests[0].body,
            json!({ "watchUrlsByGameNumber": { "14": "https://youtu.be/abc123", "15": null } })
        );
    }

    #[tokio::test]
    async fn sending_links_maps_the_portals_refusals_without_showing_the_key() {
        for status in [401, 403] {
            let portal = MockPortal::start(status, "").await;
            let result = send_watch_urls(portal.base_url(), "cup-2026", KEY, &links()).await;
            assert_eq!(result, Err(SendError::KeyRefused), "{status}");
        }
        for status in [400, 404, 500] {
            let portal = MockPortal::start(status, r#"{"reason":"UnknownGame"}"#).await;
            match send_watch_urls(portal.base_url(), "cup-2026", KEY, &links()).await {
                Err(SendError::Other(text)) => {
                    assert!(text.starts_with("The portal said"), "{text}");
                    assert!(text.contains(&status.to_string()), "{text}");
                    assert!(text.contains("UnknownGame"), "{text}");
                    assert!(!text.contains(KEY), "{text}");
                }
                other => panic!("{status}: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn a_long_portal_reply_is_cut_short_in_the_error() {
        let long = "x".repeat(500);
        let portal = MockPortal::start(500, &long).await;
        match send_watch_urls(portal.base_url(), "cup-2026", KEY, &links()).await {
            Err(SendError::Other(text)) => {
                assert!(text.ends_with(&"x".repeat(REPLY_CHARS)), "{text}");
                assert!(!text.contains(&"x".repeat(REPLY_CHARS + 1)), "{text}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn sending_no_links_sends_nothing() {
        let portal = MockPortal::start(204, "").await;
        let result = send_watch_urls(portal.base_url(), "cup-2026", KEY, &BTreeMap::new()).await;
        assert_eq!(result, Ok(()));
        assert!(portal.requests().is_empty());
    }

    #[test]
    fn a_saved_link_reads_back_only_for_its_own_portal_and_event() {
        let dir = temp_dir("saved");
        let path = dir.join(LINK_FILE);
        let link = PortalLink {
            portal_url: "https://portal.example".into(),
            event_slug: "cup-2026".into(),
            access_key: KEY.into(),
        };
        save(&path, &link).unwrap();
        assert_eq!(
            link_for(&path, "https://portal.example", "cup-2026"),
            Some(link)
        );
        assert_eq!(link_for(&path, "https://portal.example", "other-cup"), None);
        assert_eq!(link_for(&path, "https://other.example", "cup-2026"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_broken_link_file_is_no_link() {
        let dir = temp_dir("broken");
        let path = dir.join(LINK_FILE);
        assert_eq!(link_for(&path, "https://portal.example", "cup-2026"), None);
        for contents in ["not json", "{}"] {
            std::fs::write(&path, contents).unwrap();
            assert_eq!(
                link_for(&path, "https://portal.example", "cup-2026"),
                None,
                "{contents}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn plan(numbers: &[&str]) -> EventPlan {
        EventPlan {
            event_name: "Cup".into(),
            banner_url: None,
            games: numbers
                .iter()
                .map(|number| crate::portal::PlannedGame {
                    number: number.to_string(),
                    court: "Court 1".into(),
                    day: 1,
                    start: time::OffsetDateTime::UNIX_EPOCH,
                    dark: "A".into(),
                    light: "B".into(),
                    dark_team: None,
                    light_team: None,
                    description: None,
                })
                .collect(),
        }
    }

    fn recorded(games: &[(&str, &str)]) -> EventState {
        let mut state = EventState {
            event_slug: "cup-2026".into(),
            ..Default::default()
        };
        for (number, broadcast_id) in games {
            state.videos.insert(
                number.to_string(),
                prepare::VideoState {
                    broadcast_id: broadcast_id.to_string(),
                    title: String::new(),
                    description: String::new(),
                    scheduled_start: String::new(),
                    bound_stream: None,
                    in_playlist: true,
                    next_game_link: None,
                    portal_start: None,
                    court: None,
                    day: None,
                    thumbnail: None,
                },
            );
        }
        state
    }

    #[test]
    fn after_prepare_each_recorded_game_on_the_schedule_gets_its_link() {
        let links = links_after_prepare(
            &plan(&["14", "15", "16"]),
            &recorded(&[("14", "aaa"), ("15", "bbb")]),
        );
        assert_eq!(
            links,
            BTreeMap::from([
                ("14".to_string(), Some("https://youtu.be/aaa".to_string())),
                ("15".to_string(), Some("https://youtu.be/bbb".to_string())),
            ])
        );
    }

    #[test]
    fn after_prepare_a_recorded_game_no_longer_on_the_schedule_is_left_out() {
        let links = links_after_prepare(&plan(&["14"]), &recorded(&[("14", "aaa"), ("99", "zzz")]));
        assert_eq!(
            links,
            BTreeMap::from([("14".to_string(), Some("https://youtu.be/aaa".to_string()))])
        );
    }

    #[test]
    fn after_cleanup_only_deleted_games_on_the_schedule_are_cleared() {
        let deleted = ["14".to_string(), "99".to_string()];
        let links = links_after_cleanup(&plan(&["14", "15"]), &deleted);
        assert_eq!(links, BTreeMap::from([("14".to_string(), None)]));
    }

    /// A config for `portal`, the link file holding a key for `linked_event` (none if `None`),
    /// and the log lines `publish` wrote.
    struct Publishing {
        dir: std::path::PathBuf,
        link_file: std::path::PathBuf,
        config: Config,
        lines: Vec<String>,
    }

    impl Publishing {
        fn new(name: &str, portal: &MockPortal, linked_event: Option<&str>) -> Self {
            let dir = temp_dir(name);
            let link_file = dir.join(LINK_FILE);
            if let Some(event_slug) = linked_event {
                save(
                    &link_file,
                    &PortalLink {
                        portal_url: portal.base_url().to_string(),
                        event_slug: event_slug.to_string(),
                        access_key: KEY.into(),
                    },
                )
                .unwrap();
            }
            let config = Config {
                portal_url: portal.base_url().to_string(),
                event_slug: "cup-2026".into(),
                ..Config::default()
            };
            Self {
                dir,
                link_file,
                config,
                lines: Vec::new(),
            }
        }

        async fn publish(&mut self, links: &BTreeMap<String, Option<String>>, purpose: Purpose) {
            let mut lines = Vec::new();
            publish(&self.link_file, &self.config, links, purpose, &mut |line| {
                lines.push(line)
            })
            .await;
            self.lines.extend(lines);
        }
    }

    impl Drop for Publishing {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    const NOT_LINKED: &str = "Portal watch links: this Stream Manager isn't linked to the event, so none were set (Settings → Portal watch links)";

    #[tokio::test]
    async fn publishing_when_linked_sends_the_links_and_says_how_many() {
        let portal = MockPortal::start(204, "").await;
        let mut publishing = Publishing::new("publish-ok", &portal, Some("cup-2026"));
        let two = BTreeMap::from([
            ("14".to_string(), Some(watch_url("aaa"))),
            ("15".to_string(), Some(watch_url("bbb"))),
        ]);
        publishing.publish(&two, Purpose::Set).await;
        let requests = portal.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, SEND_PATH);
        assert_eq!(
            requests[0].body,
            json!({ "watchUrlsByGameNumber": {
                "14": "https://youtu.be/aaa",
                "15": "https://youtu.be/bbb",
            } })
        );
        assert_eq!(publishing.lines, ["Portal: watch links set for 2 games"]);

        publishing.publish(&links(), Purpose::Clear).await;
        assert_eq!(
            publishing.lines[1],
            "Portal: watch links cleared for 2 games"
        );
    }

    #[tokio::test]
    async fn publishing_when_not_linked_sends_nothing_and_says_so() {
        let portal = MockPortal::start(204, "").await;
        let mut publishing = Publishing::new("publish-unlinked", &portal, None);
        publishing.publish(&links(), Purpose::Set).await;
        assert!(portal.requests().is_empty());
        assert_eq!(publishing.lines, [NOT_LINKED]);
    }

    #[tokio::test]
    async fn a_key_for_another_event_is_not_sent_and_is_kept() {
        let portal = MockPortal::start(204, "").await;
        let mut publishing = Publishing::new("publish-other-event", &portal, Some("other-cup"));
        publishing.publish(&links(), Purpose::Set).await;
        assert!(portal.requests().is_empty());
        assert_eq!(publishing.lines, [NOT_LINKED]);
        assert!(publishing.link_file.exists());
    }

    #[tokio::test]
    async fn the_key_check_sends_an_empty_update_with_the_key() {
        let portal = MockPortal::start(204, "").await;
        let publishing = Publishing::new("check-accepted", &portal, Some("cup-2026"));
        let check = check_key(&publishing.link_file, &publishing.config).await;
        assert_eq!(check, KeyCheck::Accepted);
        assert_eq!(check.notice(), None);
        let requests = portal.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "PUT");
        assert_eq!(requests[0].path, SEND_PATH);
        assert_eq!(requests[0].authorization, Some(format!("Bearer {KEY}")));
        assert_eq!(requests[0].body, json!({ "watchUrlsByGameNumber": {} }));
        assert!(publishing.link_file.exists());
    }

    #[tokio::test]
    async fn the_key_check_warns_about_and_forgets_a_refused_key() {
        for status in [401, 403] {
            let portal = MockPortal::start(status, "").await;
            let publishing = Publishing::new("check-refused", &portal, Some("cup-2026"));
            let check = check_key(&publishing.link_file, &publishing.config).await;
            assert_eq!(check, KeyCheck::Refused, "{status}");
            let (warning, text) = check.notice().unwrap();
            assert!(warning);
            assert!(text.contains("Link it again in Settings first"), "{text}");
            assert!(!publishing.link_file.exists(), "{status}");
        }
    }

    #[tokio::test]
    async fn the_key_check_without_a_link_asks_nothing() {
        let portal = MockPortal::start(204, "").await;
        let publishing = Publishing::new("check-not-linked", &portal, None);
        let check = check_key(&publishing.link_file, &publishing.config).await;
        assert_eq!(check, KeyCheck::NotLinked);
        assert!(!check.notice().unwrap().0);
        assert!(portal.requests().is_empty());
    }

    #[tokio::test]
    async fn the_key_check_gives_up_on_a_slow_portal_and_keeps_the_key() {
        let portal = MockPortal::start(204, "").await;
        portal.delay_replies(Duration::from_secs(2));
        let publishing = Publishing::new("check-slow", &portal, Some("cup-2026"));
        let check = check_key_within(
            &publishing.link_file,
            &publishing.config,
            Duration::from_millis(200),
        )
        .await;
        match &check {
            KeyCheck::Unknown(text) => assert!(text.contains("didn't answer"), "{text}"),
            other => panic!("{other:?}"),
        }
        assert!(publishing.link_file.exists());
    }

    #[tokio::test]
    async fn the_key_check_keeps_the_key_when_the_portal_fails_otherwise() {
        let portal = MockPortal::start(500, "boom").await;
        let publishing = Publishing::new("check-unknown", &portal, Some("cup-2026"));
        let check = check_key(&publishing.link_file, &publishing.config).await;
        match &check {
            KeyCheck::Unknown(text) => assert!(text.starts_with("The portal said"), "{text}"),
            other => panic!("{other:?}"),
        }
        assert!(!check.notice().unwrap().0);
        assert!(publishing.link_file.exists());
    }

    #[tokio::test]
    async fn a_refused_key_is_forgotten_and_not_tried_again() {
        let portal = MockPortal::start(401, "").await;
        let mut publishing = Publishing::new("publish-refused", &portal, Some("cup-2026"));
        publishing.publish(&links(), Purpose::Set).await;
        assert!(!publishing.link_file.exists());
        assert_eq!(
            publishing.lines,
            [
                "Portal: the event refused this Stream Manager's key (it was removed on the portal, or the event is over). Link it again in Settings to set watch links."
            ]
        );
        publishing.publish(&links(), Purpose::Set).await;
        assert_eq!(portal.requests().len(), 1);
        assert_eq!(publishing.lines[1], NOT_LINKED);
    }

    #[tokio::test]
    async fn another_portal_error_keeps_the_key_and_says_to_run_prepare_again() {
        let portal = MockPortal::start(500, "boom").await;
        let mut publishing = Publishing::new("publish-error", &portal, Some("cup-2026"));
        publishing.publish(&links(), Purpose::Set).await;
        assert!(publishing.link_file.exists());
        assert_eq!(publishing.lines.len(), 1);
        let line = &publishing.lines[0];
        assert!(
            line.starts_with("Portal: watch links weren't set: The portal said 500"),
            "{line}"
        );
        assert!(
            line.ends_with(". Run Prepare again to send them."),
            "{line}"
        );
        assert!(!line.contains(KEY), "{line}");

        publishing.publish(&links(), Purpose::Clear).await;
        let line = &publishing.lines[1];
        assert!(
            line.starts_with("Portal: watch links weren't cleared: The portal said 500"),
            "{line}"
        );
    }

    #[tokio::test]
    async fn publishing_no_links_sends_nothing_and_logs_nothing() {
        let portal = MockPortal::start(204, "").await;
        let mut publishing = Publishing::new("publish-empty", &portal, Some("cup-2026"));
        publishing.publish(&BTreeMap::new(), Purpose::Set).await;
        assert!(portal.requests().is_empty());
        assert!(publishing.lines.is_empty());
    }

    const NOT_LINKED_CLEAR: &str = "Portal watch links: this Stream Manager isn't linked to the event, so the deleted games' links weren't cleared (Settings → Portal watch links)";

    #[tokio::test]
    async fn clearing_when_not_linked_says_the_deleted_games_links_werent_cleared() {
        let portal = MockPortal::start(204, "").await;
        let mut publishing = Publishing::new("publish-unlinked-clear", &portal, None);
        publishing.publish(&links(), Purpose::Clear).await;
        assert!(portal.requests().is_empty());
        assert_eq!(publishing.lines, [NOT_LINKED_CLEAR]);
    }

    #[test]
    fn a_refused_key_is_kept_when_another_key_was_saved_meanwhile() {
        let dir = temp_dir("refused-relinked");
        let path = dir.join(LINK_FILE);
        let refused = PortalLink {
            portal_url: "https://portal.example".into(),
            event_slug: "cup-2026".into(),
            access_key: KEY.into(),
        };
        let relinked = PortalLink {
            access_key: "a-newer-key".into(),
            ..refused.clone()
        };
        save(&path, &relinked).unwrap();
        forget_refused(&path, &refused);
        assert_eq!(
            link_for(&path, "https://portal.example", "cup-2026"),
            Some(relinked)
        );

        save(&path, &refused).unwrap();
        forget_refused(&path, &refused);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_reply_that_cant_be_read_says_the_portal_couldnt_be_reached() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        // A portal that promises a longer reply than it sends, then hangs up.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request).await;
            let _ = socket
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 100\r\n\r\nshort",
                )
                .await;
        });
        match send_watch_urls(&format!("http://{address}"), "cup-2026", KEY, &links()).await {
            Err(SendError::Other(text)) => {
                assert!(text.starts_with("Couldn't reach the portal: "), "{text}");
                assert!(!text.contains(KEY), "{text}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn after_prepare_the_record_is_reloaded_and_its_links_sent() {
        let portal = MockPortal::start(204, "").await;
        let publishing = Publishing::new("after-prepare", &portal, Some("cup-2026"));
        let state_file = publishing.dir.join("state-cup-2026.json");
        prepare::update_state(&state_file, "cup-2026", |state| {
            *state = recorded(&[("14", "aaa"), ("99", "zzz")]);
        })
        .unwrap();
        let mut lines = Vec::new();
        publish_after_prepare(
            &publishing.link_file,
            &publishing.config,
            &plan(&["14", "15"]),
            &state_file,
            &mut |line| lines.push(line),
        )
        .await;
        let requests = portal.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].body,
            json!({ "watchUrlsByGameNumber": { "14": "https://youtu.be/aaa" } })
        );
        assert_eq!(lines, ["Portal: watch links set for 1 games"]);
    }

    #[tokio::test]
    async fn after_prepare_a_record_that_cant_be_read_is_one_log_line() {
        let portal = MockPortal::start(204, "").await;
        let publishing = Publishing::new("after-prepare-broken", &portal, Some("cup-2026"));
        let state_file = publishing.dir.join("state-cup-2026.json");
        std::fs::write(&state_file, "not json").unwrap();
        let mut lines = Vec::new();
        publish_after_prepare(
            &publishing.link_file,
            &publishing.config,
            &plan(&["14"]),
            &state_file,
            &mut |line| lines.push(line),
        )
        .await;
        assert!(portal.requests().is_empty());
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].starts_with("Portal: watch links weren't set: "),
            "{}",
            lines[0]
        );
    }

    #[tokio::test]
    async fn after_cleanup_the_loaded_schedule_is_used_and_nothing_deleted_sends_nothing() {
        let portal = MockPortal::start(500, "not the schedule").await;
        portal.reply_on(SEND_PATH, 204, "");
        let publishing = Publishing::new("after-cleanup", &portal, Some("cup-2026"));
        let mut lines = Vec::new();
        let mut log = |line: String| lines.push(line);
        publish_after_cleanup(
            &publishing.link_file,
            &publishing.config,
            Some(plan(&["14", "15"])),
            &[],
            &mut log,
        )
        .await;
        assert!(portal.requests().is_empty());

        let deleted = ["14".to_string(), "99".to_string()];
        publish_after_cleanup(
            &publishing.link_file,
            &publishing.config,
            Some(plan(&["14", "15"])),
            &deleted,
            &mut log,
        )
        .await;
        let requests = portal.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, SEND_PATH);
        assert_eq!(
            requests[0].body,
            json!({ "watchUrlsByGameNumber": { "14": null } })
        );
        assert_eq!(lines, ["Portal: watch links cleared for 1 games"]);
    }

    #[tokio::test]
    async fn after_cleanup_with_no_schedule_loaded_it_is_fetched_and_a_failure_is_one_line() {
        let portal = MockPortal::start(500, "boom").await;
        let publishing = Publishing::new("after-cleanup-no-plan", &portal, Some("cup-2026"));
        let mut lines = Vec::new();
        publish_after_cleanup(
            &publishing.link_file,
            &publishing.config,
            None,
            &["14".to_string()],
            &mut |line| lines.push(line),
        )
        .await;
        let requests = portal.requests();
        assert!(!requests.is_empty());
        assert!(requests.iter().all(|request| request.method == "GET"));
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].starts_with(
                "Portal: watch links weren't cleared: the schedule couldn't be loaded ("
            ),
            "{}",
            lines[0]
        );
    }

    #[test]
    fn forgetting_removes_the_file_and_a_missing_file_is_fine() {
        let dir = temp_dir("forget");
        let path = dir.join(LINK_FILE);
        assert!(forget(&path).is_ok());
        std::fs::write(&path, "{}").unwrap();
        forget(&path).unwrap();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
