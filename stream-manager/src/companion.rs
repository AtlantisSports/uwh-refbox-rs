//! Live status on the Stream Deck buttons (ADR 026 §4): pushes each court's Hold, rosters, now
//! live and up next into Bitfocus Companion's custom variables, so buttons can show them.
//!
//! The request, confirmed against Companion 5.0.7's own source
//! (<https://github.com/bitfocus/companion/blob/v5.0.7/companion/lib/Service/HttpApi.ts>, route
//! `/custom-variable/:name/value`, mounted under `/api` in
//! <https://github.com/bitfocus/companion/blob/v5.0.7/companion/lib/UI/Express.ts>):
//!
//! ```text
//! POST http://<companion address>/api/custom-variable/<name>/value?value=<text>
//! ```
//!
//! Companion answers `ok` on success, 404 "Not found" if no custom variable has that name (it
//! must be created in Companion first), and 403 while Companion's HTTP API is turned off. The
//! value goes in the query string, not the body, because Companion refuses an empty body and
//! empty values ("") are needed to blank a button.

use crate::BoxError;
use std::{collections::HashMap, time::Duration};

/// How long to wait for Companion. Short, so a missing Companion is noticed quickly.
const TIMEOUT: Duration = Duration::from_secs(2);

/// The four custom-variable names for one court.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourtVariables {
    pub hold: String,
    pub rosters: String,
    pub now: String,
    pub next: String,
}

/// Custom-variable names for one court, e.g. "sm_court_1_hold". Only `[a-z0-9_]` is used: the
/// name is lowercased, every run of other characters becomes one `_`, and a leading "Court" is
/// dropped, so court "1" and "Court 1" both give `sm_court_1_…`.
pub fn variable_names(court_name: &str) -> CourtVariables {
    let slug = slug(court_name);
    let court = match slug.strip_prefix("court") {
        Some(rest) if rest.is_empty() || rest.starts_with('_') => rest.trim_start_matches('_'),
        _ => slug.as_str(),
    };
    let base = if court.is_empty() {
        "sm_court".to_string()
    } else {
        format!("sm_court_{court}")
    };
    CourtVariables {
        hold: format!("{base}_hold"),
        rosters: format!("{base}_rosters"),
        now: format!("{base}_now"),
        next: format!("{base}_next"),
    }
}

/// Lowercase letters and digits, with every run of anything else turned into one `_`, and no
/// `_` at either end.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

/// What one court's buttons show, in the order hold, rosters, now, next.
pub struct CourtState<'a> {
    pub hold: bool,
    /// Seconds until the overlay starts showing rosters, while counting down to them.
    pub secs_until_rosters: Option<u32>,
    /// The overlay is showing rosters now.
    pub in_rosters: bool,
    /// The game whose video is live.
    pub live: Option<&'a str>,
    /// The game whose video goes live next.
    pub next: Option<&'a str>,
}

/// The four values for one court: "ON"/"OFF"; "Rosters in m:ss" / "Rosters on screen" / "";
/// "Now: Game 14" or ""; "Next: Game 15" or "".
pub fn court_values(state: &CourtState) -> [String; 4] {
    let hold = if state.hold { "ON" } else { "OFF" }.to_string();
    let rosters = match (state.secs_until_rosters, state.in_rosters) {
        (Some(secs), _) => format!("Rosters in {}:{:02}", secs / 60, secs % 60),
        (None, true) => "Rosters on screen".to_string(),
        (None, false) => String::new(),
    };
    let now = state
        .live
        .map(|game| format!("Now: Game {game}"))
        .unwrap_or_default();
    let next = state
        .next
        .map(|game| format!("Next: Game {game}"))
        .unwrap_or_default();
    [hold, rosters, now, next]
}

/// Pairs each of a court's variable names with its value.
pub fn court_pairs(court_name: &str, state: &CourtState) -> Vec<(String, String)> {
    let names = variable_names(court_name);
    [names.hold, names.rosters, names.now, names.next]
        .into_iter()
        .zip(court_values(state))
        .collect()
}

/// Remembers what Companion was last sent, so only changed values are sent again.
#[derive(Debug, Default)]
pub struct LastSent {
    address: String,
    sent: HashMap<String, String>,
}

impl LastSent {
    /// The values in `wanted` that Companion at `address` doesn't have yet. A different address
    /// than last time forgets everything, so all values are sent again.
    pub fn changes(&mut self, address: &str, wanted: &[(String, String)]) -> Vec<(String, String)> {
        if self.address != address {
            self.sent.clear();
            self.address = address.to_string();
        }
        wanted
            .iter()
            .filter(|(name, value)| self.sent.get(name) != Some(value))
            .cloned()
            .collect()
    }

    /// Companion accepted `value` for `name`.
    pub fn record(&mut self, name: &str, value: &str) {
        self.sent.insert(name.to_string(), value.to_string());
    }

    /// Forgets everything, e.g. when the feature is turned off.
    pub fn forget(&mut self) {
        self.sent.clear();
    }
}

/// Sets Companion's custom variable `name` to `value`. `address` is Companion's address, e.g.
/// "127.0.0.1:8000"; empty means the feature is off and callers don't call this.
pub async fn set_variable(address: &str, name: &str, value: &str) -> Result<(), BoxError> {
    let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;
    let response = client
        .post(format!("http://{address}/api/custom-variable/{name}/value"))
        .query(&[("value", value)])
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Companion at {address}: {e}"))?;
    match response.status() {
        status if status.is_success() => Ok(()),
        reqwest::StatusCode::NOT_FOUND => Err(format!(
            "Companion has no custom variable \"{name}\"; create it in Companion's Variables tab"
        )
        .into()),
        reqwest::StatusCode::FORBIDDEN => {
            Err("Companion's HTTP API is turned off; turn it on in Companion's settings".into())
        }
        status => Err(format!("Companion at {address} refused {name} ({status})").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn state() -> CourtState<'static> {
        CourtState {
            hold: false,
            secs_until_rosters: None,
            in_rosters: false,
            live: None,
            next: None,
        }
    }

    #[test]
    fn court_names_become_lowercase_variable_names() {
        let names = variable_names("Court 1");
        assert_eq!(names.hold, "sm_court_1_hold");
        assert_eq!(names.rosters, "sm_court_1_rosters");
        assert_eq!(names.now, "sm_court_1_now");
        assert_eq!(names.next, "sm_court_1_next");
        assert_eq!(variable_names("1").hold, "sm_court_1_hold");
        assert_eq!(variable_names("Main Pool").now, "sm_court_main_pool_now");
        assert_eq!(variable_names("  A -- B!").next, "sm_court_a_b_next");
        assert_eq!(
            variable_names("Pool #2 (deep)").hold,
            "sm_court_pool_2_deep_hold"
        );
    }

    #[test]
    fn hold_shows_on_or_off() {
        assert_eq!(court_values(&state())[0], "OFF");
        let held = CourtState {
            hold: true,
            ..state()
        };
        assert_eq!(court_values(&held)[0], "ON");
    }

    #[test]
    fn rosters_count_down_then_show_on_screen_then_blank() {
        let counting = CourtState {
            secs_until_rosters: Some(45),
            ..state()
        };
        assert_eq!(court_values(&counting)[1], "Rosters in 0:45");
        let long = CourtState {
            secs_until_rosters: Some(125),
            ..state()
        };
        assert_eq!(court_values(&long)[1], "Rosters in 2:05");
        let showing = CourtState {
            in_rosters: true,
            ..state()
        };
        assert_eq!(court_values(&showing)[1], "Rosters on screen");
        assert_eq!(court_values(&state())[1], "");
    }

    #[test]
    fn now_and_next_name_the_games_or_are_blank() {
        assert_eq!(court_values(&state())[2], "");
        assert_eq!(court_values(&state())[3], "");
        let both = CourtState {
            live: Some("14"),
            next: Some("15"),
            ..state()
        };
        let values = court_values(&both);
        assert_eq!(values[2], "Now: Game 14");
        assert_eq!(values[3], "Next: Game 15");
    }

    #[test]
    fn unchanged_values_are_not_sent_twice() {
        let mut last = LastSent::default();
        let wanted = court_pairs("1", &state());
        let first = last.changes("127.0.0.1:8000", &wanted);
        assert_eq!(first, wanted);
        for (name, value) in &first {
            last.record(name, value);
        }
        assert!(last.changes("127.0.0.1:8000", &wanted).is_empty());

        let held = court_pairs(
            "1",
            &CourtState {
                hold: true,
                ..state()
            },
        );
        assert_eq!(
            last.changes("127.0.0.1:8000", &held),
            vec![("sm_court_1_hold".to_string(), "ON".to_string())]
        );
    }

    #[test]
    fn a_failed_value_is_tried_again() {
        let mut last = LastSent::default();
        let wanted = court_pairs("1", &state());
        assert_eq!(last.changes("127.0.0.1:8000", &wanted).len(), 4);
        // Nothing recorded: none of them reached Companion.
        assert_eq!(last.changes("127.0.0.1:8000", &wanted).len(), 4);
    }

    #[test]
    fn a_new_address_or_forget_sends_everything_again() {
        let mut last = LastSent::default();
        let wanted = court_pairs("1", &state());
        for (name, value) in last.changes("127.0.0.1:8000", &wanted) {
            last.record(&name, &value);
        }
        assert_eq!(last.changes("10.0.0.5:8000", &wanted).len(), 4);
        for (name, value) in last.changes("10.0.0.5:8000", &wanted) {
            last.record(&name, &value);
        }
        last.forget();
        assert_eq!(last.changes("10.0.0.5:8000", &wanted).len(), 4);
    }

    /// Review Focus 5: a Companion that isn't there fails fast instead of holding anything up.
    #[tokio::test]
    async fn closed_port_fails_in_under_two_seconds() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let started = Instant::now();
        let result = set_variable(&address, "sm_court_1_hold", "ON").await;
        assert!(result.is_err());
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
    }

    /// Answers one request with `reply` and returns the request line it received.
    async fn fake_companion(reply: &'static str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 4096];
            let read = socket.read(&mut buffer).await.unwrap();
            socket.write_all(reply.as_bytes()).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            request.lines().next().unwrap_or_default().to_string()
        });
        (address, handle)
    }

    #[tokio::test]
    async fn sends_a_post_with_the_value_in_the_query() {
        let (address, request) =
            fake_companion("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .await;
        set_variable(&address, "sm_court_1_now", "Now: Game 14")
            .await
            .unwrap();
        assert_eq!(
            request.await.unwrap(),
            "POST /api/custom-variable/sm_court_1_now/value?value=Now%3A+Game+14 HTTP/1.1"
        );
    }

    #[tokio::test]
    async fn missing_variable_says_to_create_it() {
        let (address, _request) = fake_companion(
            "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot found",
        )
        .await;
        let error = set_variable(&address, "sm_court_1_hold", "ON")
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("sm_court_1_hold"), "{error}");
        assert!(error.contains("create"), "{error}");
    }
}
