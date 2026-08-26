//! The operator status page (`GET /`, wired up in `server.rs`) and the piece of state it needs
//! that nothing else in the crate already tracks: how long the connection has been continuously
//! disconnected.
//!
//! # The trap, again, on a new surface
//!
//! **Amended after Task 10 -- read this before changing anything here.** Task 10 deleted
//! `state::Contact` and its `Live`/`Stale { since }` pair outright, because both were derived
//! from how long it had been since a message *arrived*. The refbox sends nothing at all whenever
//! the clock is stopped (25 seconds measured on real hardware, `feed`'s module doc), so a page
//! driven by message timing shows red at every stoppage. **Nothing in this module may read
//! `state::LiveState`'s arrival time, or anything shaped like it, for any purpose.** The only
//! liveness signal this module is allowed to consult is [`crate::feed::Connection`], via
//! [`DisconnectWatch`] below, which polls [`ConnectionState::get`] and nothing else. The only
//! duration this page may show is time since the *connection* dropped, and only while it is
//! dropped -- never-connected shows no duration at all, because there is nothing to measure from.
//!
//! [`DisconnectWatch`] polls in the background rather than computing the duration on each HTTP
//! request, so the figure is accurate even if nobody has looked at the page in a while: an
//! on-request calculation would only notice a drop the next time someone happened to ask,
//! understating how long it had actually been down between requests.
//!
//! # Keepalive availability
//!
//! Task 3 configures TCP keepalive so a refbox that silently disappears is noticed instead of
//! hanging a read forever; if the OS refuses, the supervisor logs to stderr and keeps reading
//! anyway (tearing the connection down would turn a degraded detector into a total outage). But
//! stderr is invisible to an operator running a compiled program with no terminal in view, and in
//! that state the bridge is silently back to the freeze behaviour keepalive exists to prevent.
//! This page surfaces [`ConnectionState::keepalive_active`] (the minimal signal `feed.rs` now
//! exposes for exactly this) so that state is not silent.
//!
//! # No chicken-and-egg
//!
//! [`render_page`] renders something coherent for every [`Connection`] variant, including
//! `NeverConnected` -- the page must be servable, and be useful, from the instant the bridge
//! starts, before any refbox has ever been reached (design spec §5.6).
//!
//! # Self-contained, deliberately
//!
//! The rendered HTML inlines its own CSS and links nothing external -- no CDN, no web font, no
//! external script. Venue internet being unreliable is the entire reason this bridge exists; a
//! status page that needed the internet to render correctly would fail exactly when it was most
//! needed.

use std::{
    fmt::Write as _,
    sync::{Arc, PoisonError, RwLock},
    time::{Duration, Instant},
};

use tokio::time::sleep;

use crate::feed::{Connection, ConnectionState};

/// How often [`DisconnectWatch::spawn`]'s background task re-checks the connection. This is a
/// coarse "how long has this been down" figure for a human, not a precise timer, so a small
/// polling lag is immaterial; picked short enough that it never reads as inaccurate to someone
/// watching the page.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Tracks how long [`ConnectionState`] has continuously reported [`Connection::Disconnected`],
/// and nothing else -- see the module doc's "The trap, again" section for why this may never be
/// driven by message timing.
#[derive(Clone)]
pub struct DisconnectWatch {
    since: Arc<RwLock<Option<Instant>>>,
}

impl DisconnectWatch {
    /// Spawns the background poll described in the module doc and returns a handle to read its
    /// result. The task runs forever, tied to nothing but the tokio runtime it is spawned on --
    /// the same lifetime as this crate's other background tasks (`feed::Supervisor::run`,
    /// `server::consume_snapshots`, `server::refresh_portal_loop`); it stops only when that
    /// runtime shuts down.
    pub fn spawn(connection: ConnectionState) -> Self {
        let since: Arc<RwLock<Option<Instant>>> = Arc::new(RwLock::new(None));
        let watch = Self {
            since: Arc::clone(&since),
        };

        tokio::spawn(async move {
            loop {
                let is_disconnected = connection.get() == Connection::Disconnected;
                // Split into its own non-async call, rather than locking inline here: a
                // `std::sync::RwLockWriteGuard` is deliberately `!Send` (unlocking must happen on
                // the owning thread on some platforms), so a guard created directly inside this
                // `async` block -- even one dropped well before the `.await` below -- can still
                // make the compiler consider the whole block's generator state non-`Send`. A plain
                // function call fully scopes the guard to its own stack frame, invisible to the
                // outer future's captured state.
                update_since(&since, is_disconnected);
                sleep(POLL_INTERVAL).await;
            }
        });

        watch
    }

    /// How long the connection has been continuously disconnected right now, or `None` if it is
    /// not currently disconnected -- this covers both `Connected` and `NeverConnected` alike,
    /// because `NeverConnected` has no drop instant to measure from (see the module doc).
    pub fn duration(&self) -> Option<Duration> {
        self.since
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .map(|since| since.elapsed())
    }
}

/// Applies one poll's worth of the state transition [`DisconnectWatch::spawn`]'s loop describes --
/// see that loop's own comment for why this is a separate, non-async function.
fn update_since(since: &RwLock<Option<Instant>>, is_disconnected: bool) {
    let mut guard = since.write().unwrap_or_else(PoisonError::into_inner);
    match (*guard, is_disconnected) {
        // Just became disconnected: record the instant, once.
        (None, true) => *guard = Some(Instant::now()),
        // No longer disconnected (reconnected, or was never-connected all along): nothing left
        // to measure from.
        (Some(_), false) => *guard = None,
        // Already recording, and still disconnected -- leave the original instant alone, or the
        // duration would never grow past one poll interval.
        _ => {}
    }
}

/// Everything [`render_page`] needs, already resolved from live state. Kept separate from
/// `server::AppState` (the wiring module) so `render_page` stays a pure, directly testable string
/// transform with no lock, no clock read, and no knowledge of axum.
pub struct PageData {
    pub connection: Connection,
    /// See [`DisconnectWatch::duration`] -- `Some` only while `connection` is
    /// `Connection::Disconnected`.
    pub disconnected_for: Option<Duration>,
    pub keepalive_active: bool,
    /// The current event id, or empty if none is known yet.
    pub event_id: String,
    pub game_number: String,
    pub period: String,
    pub refbox_host: String,
    pub refbox_port: u16,
    pub white_on_right: bool,
    /// The court label operator setting, or empty if not set.
    pub court: String,
    /// The scheme, host and port the viewer's own browser used to reach this page (from the
    /// request's `Host` header) -- used to build the vMix addresses so they are always exactly
    /// what worked to load the page itself, rather than a guessed interface address on a
    /// multi-homed machine.
    pub base_url: Option<String>,
}

/// The vMix-facing routes to list under "Addresses for vMix" -- kept as one list so the page and
/// any future change to it stay in sync with each other by construction.
const VMIX_ROUTES: [&str; 6] = [
    "/scorebug",
    "/penalties",
    "/fouls",
    "/warnings",
    "/nextgame",
    "/status.json",
];

/// Renders the operator status page described in the module doc. Always produces a complete,
/// valid HTML document -- see "No chicken-and-egg" above for why every field of `data` must be
/// renderable in its default/empty form.
pub fn render_page(data: &PageData) -> String {
    let (indicator_class, indicator_label) = match data.connection {
        Connection::Connected => ("live", "Connected"),
        Connection::NeverConnected => ("down", "Never connected to a refbox yet"),
        Connection::Disconnected => ("down", "Disconnected"),
    };

    let duration_html = data
        .disconnected_for
        .map(|d| {
            format!(
                "<p class=\"duration\">Down for {}</p>\n",
                format_duration(d)
            )
        })
        .unwrap_or_default();

    let keepalive_html = if data.keepalive_active {
        String::new()
    } else {
        "<p class=\"warning\">Connection check unavailable — a lost refbox may not be \
         detected.</p>\n"
            .to_string()
    };

    let event_text = if data.event_id.is_empty() {
        "(none known yet)".to_string()
    } else {
        escape_html(&data.event_id)
    };
    let game_text = if data.game_number.is_empty() {
        "(none known yet)".to_string()
    } else {
        escape_html(&data.game_number)
    };
    let court_text = if data.court.is_empty() {
        "(not set)".to_string()
    } else {
        escape_html(&data.court)
    };
    let side_text = if data.white_on_right {
        "White on right"
    } else {
        "White on left"
    };

    let base = data.base_url.as_deref().unwrap_or(
        "(open this page in the browser you'll copy addresses from, to see the exact address)",
    );
    let escaped_base = escape_html(base);
    let mut vmix_addresses = String::new();
    for route in VMIX_ROUTES {
        // `write!` into a `String` never fails (`fmt::Error` only comes from a `fmt::Write`
        // implementation genuinely refusing to write, which `String`'s never does).
        let _ = writeln!(
            vmix_addresses,
            "  <li><code>{escaped_base}{route}</code></li>"
        );
    }

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>overlay-bridge status</title>
<style>
{style}
</style>
</head>
<body>
<h1>overlay-bridge</h1>
<p class="status-line"><span class="indicator {indicator_class}"></span>{indicator_label}</p>
{duration_html}{keepalive_html}<h2>Current game</h2>
<table>
<tr><th>Event</th><td>{event_text}</td></tr>
<tr><th>Game</th><td>{game_text}</td></tr>
<tr><th>Period</th><td>{period}</td></tr>
</table>
<h2>Refbox connection</h2>
<table>
<tr><th>Address</th><td>{refbox_host}:{refbox_port}</td></tr>
</table>
<h2>Operator settings</h2>
<table>
<tr><th>Side of pool</th><td>{side_text}</td></tr>
<tr><th>Court</th><td>{court_text}</td></tr>
</table>
<h2>Addresses for vMix</h2>
<ul>
{vmix_addresses}</ul>
</body>
</html>
"#,
        style = STYLE,
        indicator_class = indicator_class,
        indicator_label = indicator_label,
        duration_html = duration_html,
        keepalive_html = keepalive_html,
        event_text = event_text,
        game_text = game_text,
        period = escape_html(&data.period),
        refbox_host = escape_html(&data.refbox_host),
        refbox_port = data.refbox_port,
        side_text = side_text,
        court_text = court_text,
        vmix_addresses = vmix_addresses,
    )
}

/// Inline CSS only -- see the module doc's "Self-contained, deliberately" section. No external
/// stylesheet, font or script anywhere on this page.
const STYLE: &str = r#"
:root { color-scheme: light dark; }
body { font-family: system-ui, sans-serif; margin: 2rem; max-width: 40rem; line-height: 1.4; }
h1 { font-size: 1.5rem; margin-bottom: 0.25rem; }
h2 { font-size: 1.05rem; margin-bottom: 0.25rem; }
.status-line { font-size: 1.25rem; }
.indicator {
  display: inline-block; width: 1em; height: 1em; border-radius: 50%;
  vertical-align: middle; margin-right: 0.5em;
}
.indicator.live { background: #1a7f37; }
.indicator.down { background: #cf222e; }
.duration { color: #cf222e; margin: 0 0 1rem; }
.warning { color: #9a6700; font-weight: bold; }
table { border-collapse: collapse; margin: 0.25rem 0 1.25rem; }
th, td { padding: 0.15rem 0.75rem 0.15rem 0; text-align: left; font-weight: normal; }
th { opacity: 0.7; }
code { background: rgba(127, 127, 127, 0.15); padding: 0.1em 0.35em; border-radius: 0.25em; }
ul { padding-left: 1.2rem; }
"#;

/// A whole-number-of-seconds `Duration`, rendered as `"Xm Ys"` (or just `"Ys"` under a minute) --
/// a human-facing figure, not a data column (see `server.rs`'s `/status.json`, which serves the
/// raw seconds instead).
fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// Minimal HTML escaping for the handful of strings this page interpolates that are not
/// guaranteed to be free of `< > & " '` -- an operator-typed court label, a refbox-reported event
/// id or game number, and (most importantly) the incoming request's own `Host` header, which is
/// attacker-controlled input on a bridge this project deliberately serves with no password
/// (design spec §6, "anyone on the network can read it"). Not a general-purpose HTML sanitiser --
/// just enough to keep every interpolated value inert as markup.
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_data() -> PageData {
        PageData {
            connection: Connection::NeverConnected,
            disconnected_for: None,
            keepalive_active: true,
            event_id: String::new(),
            game_number: String::new(),
            period: String::new(),
            refbox_host: "127.0.0.1".to_string(),
            refbox_port: 8000,
            white_on_right: false,
            court: String::new(),
            base_url: Some("http://192.168.1.5:8099".to_string()),
        }
    }

    // ---------------------------------------------------------------------------- render_page

    #[test]
    fn a_never_connected_page_shows_the_down_indicator_and_no_duration() {
        let html = render_page(&base_data());
        assert!(html.contains("indicator down"));
        assert!(!html.contains("class=\"duration\""));
        assert!(html.to_lowercase().contains("never connected"));
    }

    #[test]
    fn a_connected_page_shows_the_live_indicator_and_no_duration() {
        let data = PageData {
            connection: Connection::Connected,
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("indicator live"));
        assert!(
            !html.contains("class=\"duration\""),
            "a connected page must never show a duration, even if disconnected_for were \
             mistakenly set -- this only checks the common path (None), the guard against the \
             other path is a server.rs-level test"
        );
    }

    #[test]
    fn a_disconnected_page_shows_the_down_indicator_and_the_duration() {
        let data = PageData {
            connection: Connection::Disconnected,
            disconnected_for: Some(Duration::from_secs(135)),
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("indicator down"));
        assert!(html.contains("Down for 2m 15s"));
    }

    #[test]
    fn keepalive_unavailable_shows_the_warning_wording() {
        let data = PageData {
            keepalive_active: false,
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("Connection check unavailable"));
        assert!(html.contains("a lost refbox may not be detected"));
    }

    #[test]
    fn keepalive_active_shows_no_warning() {
        let html = render_page(&base_data());
        assert!(!html.contains("Connection check unavailable"));
    }

    #[test]
    fn the_page_lists_every_vmix_route_built_from_the_request_host() {
        let html = render_page(&base_data());
        for route in VMIX_ROUTES {
            assert!(
                html.contains(&format!("http://192.168.1.5:8099{route}")),
                "missing vMix address for {route} in:\n{html}"
            );
        }
    }

    #[test]
    fn an_untrusted_host_header_is_not_rendered_as_markup() {
        // The `Host` header is attacker-controlled input on a bridge this project deliberately
        // serves with no password (see the module doc) -- this proves a crafted value can't
        // inject an element into the page.
        let data = PageData {
            base_url: Some("http://<script>evil</script>".to_string()),
            ..base_data()
        };
        let html = render_page(&data);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn operator_settings_and_refbox_address_are_shown() {
        let data = PageData {
            white_on_right: true,
            court: "Pool A".to_string(),
            refbox_host: "192.168.1.50".to_string(),
            refbox_port: 8000,
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("White on right"));
        assert!(html.contains("Pool A"));
        assert!(html.contains("192.168.1.50:8000"));
    }

    #[test]
    fn a_missing_host_header_still_renders_a_complete_page() {
        // No chicken-and-egg applies here too: an unusual client that omits `Host` must not
        // break the page, just fall back to explanatory text instead of a real address.
        let data = PageData {
            base_url: None,
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("</html>"));
    }

    // ---------------------------------------------------------------------------- DisconnectWatch

    #[tokio::test]
    async fn a_fresh_watch_reports_no_duration() {
        let watch = DisconnectWatch::spawn(ConnectionState::new());
        assert_eq!(watch.duration(), None);
    }

    #[tokio::test]
    async fn the_watch_starts_measuring_once_the_connection_reports_disconnected() {
        let connection = ConnectionState::new();
        let watch = DisconnectWatch::spawn(connection.clone());

        // NeverConnected -> Connected -> Disconnected, driven directly through the same
        // `pub(crate)` setters `feed::Supervisor::run` itself calls (see `feed.rs`'s tests for
        // the same pattern) -- this test is about the watch's own polling logic, not about
        // exercising a real socket.
        connection.set_connected();
        sleep(POLL_INTERVAL * 2).await;
        assert_eq!(watch.duration(), None, "still connected: no duration yet");

        connection.set_disconnected();
        sleep(POLL_INTERVAL * 3).await;
        let duration = watch
            .duration()
            .expect("the watch should report a duration once disconnected");
        assert!(
            duration < Duration::from_secs(2),
            "duration should be small this soon after the transition, got {duration:?}"
        );
    }

    #[tokio::test]
    async fn reconnecting_clears_the_duration() {
        let connection = ConnectionState::new();
        let watch = DisconnectWatch::spawn(connection.clone());

        connection.set_disconnected();
        sleep(POLL_INTERVAL * 3).await;
        assert!(watch.duration().is_some());

        connection.set_connected();
        sleep(POLL_INTERVAL * 3).await;
        assert_eq!(
            watch.duration(),
            None,
            "reconnecting must clear the duration -- there is nothing left to measure"
        );
    }

    #[tokio::test]
    async fn the_duration_keeps_growing_across_multiple_polls_not_resetting_each_time() {
        let connection = ConnectionState::new();
        let watch = DisconnectWatch::spawn(connection.clone());

        connection.set_disconnected();
        sleep(POLL_INTERVAL * 2).await;
        let first = watch.duration().expect("should be disconnected by now");

        sleep(POLL_INTERVAL * 3).await;
        let second = watch.duration().expect("should still be disconnected");

        assert!(
            second > first,
            "the duration must keep growing from the original drop instant, not reset on every \
             poll: first={first:?}, second={second:?}"
        );
    }

    // ---------------------------------------------------------------------------- format_duration

    #[test]
    fn format_duration_under_a_minute_shows_only_seconds() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
    }

    #[test]
    fn format_duration_over_a_minute_shows_minutes_and_seconds() {
        assert_eq!(format_duration(Duration::from_secs(135)), "2m 15s");
    }
}
