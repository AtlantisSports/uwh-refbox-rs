//! Puts each game's YouTube watch link on the portal's schedule: the saved access key for the
//! linked portal event, swapping the portal's code for that key, and sending the links.
//!
//! The key is a secret. It is kept only in its own file (never in the settings), and is never
//! put in a log line or an error message.

use crate::{BoxError, prepare};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

/// The file, next to the settings, holding the access key for the linked portal event.
pub const LINK_FILE: &str = "portal-watch-links.json";

/// How much of a portal reply goes into an error message.
const REPLY_CHARS: usize = 200;

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
    let text = response.text().await.unwrap_or_default();
    Err(SendError::Other(portal_said(status, &text)))
}

/// The watch link for a YouTube video.
pub fn watch_url(broadcast_id: &str) -> String {
    format!("https://youtu.be/{broadcast_id}")
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
