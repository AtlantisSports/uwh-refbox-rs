//! The operator status page (`GET /`, wired up in `server.rs`).
//!
//! # The trap, again, on a new surface
//!
//! **Amended after Task 10 -- read this before changing anything here.** Task 10 deleted
//! `state::Contact` and its `Live`/`Stale { since }` pair outright, because both were derived
//! from how long it had been since a message *arrived*. The refbox sends nothing at all whenever
//! the clock is stopped (25 seconds measured on real hardware, `feed`'s module doc), so a page
//! driven by message timing shows red at every stoppage. **Nothing in this module may read
//! `state::LiveState`'s arrival time, or anything shaped like it, for any purpose.** The only
//! liveness signal this module is allowed to render is [`crate::feed::ConnectionStatus`], which
//! comes from [`crate::feed::ConnectionState::snapshot`] and nothing else. The only duration this
//! page may show is time since the *connection* dropped, and only while it is dropped --
//! never-connected shows no duration at all, because there is nothing to measure from.
//!
//! # One source, not two (Task 7 review, Important 1)
//!
//! This module's first version tracked the drop duration itself, in a background-polled watcher
//! reading `ConnectionState::get()` on its own timer. That put the connection flag and its
//! duration on two independently-timed paths: `server.rs` read `state.connection.get()` and
//! `state.disconnect_watch.duration()` as two separate calls, so on reconnect there was a window
//! (up to the watcher's poll interval) where the flag already read `Connected` but the watcher
//! had not yet noticed and cleared its own stale duration -- a served page could show a green
//! "Connected" indicator above a red "Down for 42s" line, and `/status.json` could ship
//! `contact: "Live"` beside a nonzero `disconnectedForSeconds`. Review caught it.
//!
//! The fix is not a check here that suppresses the duration when the connection looks connected
//! -- that would only hide the symptom in the HTML while leaving `/status.json`'s raw field just
//! as capable of disagreeing with `contact`, because the two pieces would still be sourced
//! independently. The fix is that there is no longer a second source at all:
//! `ConnectionState` itself now records *when* a disconnect happened, in the same lock
//! acquisition that changes the state, and hands both back together from one read
//! ([`crate::feed::ConnectionState::snapshot`]) -- see that type's own doc for the full
//! reasoning. `render_page` below simply renders whatever single [`crate::feed::ConnectionStatus`]
//! it is given; there is nothing left for this module to poll, and nothing to reconcile.
//!
//! # Keepalive availability
//!
//! Task 3 configures TCP keepalive so a refbox that silently disappears is noticed instead of
//! hanging a read forever; if the OS refuses, the supervisor logs to stderr and keeps reading
//! anyway (tearing the connection down would turn a degraded detector into a total outage). But
//! stderr is invisible to an operator running a compiled program with no terminal in view, and in
//! that state the bridge is silently back to the freeze behaviour keepalive exists to prevent.
//! This page surfaces [`crate::feed::ConnectionState::keepalive_active`] (the minimal signal
//! `feed.rs` exposes for exactly this) so that state is not silent.
//!
//! # Choosing a refbox (Task 8)
//!
//! The page does not merely display the refbox address any more: it is where an operator chooses
//! which refbox the bridge reads, either by picking one the bridge found on the network or by
//! typing an address. Both are plain HTML forms posting to `server.rs` (`POST /scan`,
//! `POST /refbox`), which does the work and redirects back here -- see that module's doc for why
//! an address is proved to be a refbox *before* anything running is touched.
//!
//! Two consequences for this module, both deliberate. **Everything shown here is display data
//! already resolved by the caller** -- this stays a pure string transform with no lock, no clock
//! and no network, so the scan results and the outcome sentence are rendered exactly the way the
//! connection state already was. And **the forms are forms, not a front-end application**: no
//! script of any kind, so the page behaves identically on the streaming PC's browser and on a
//! phone at the poolside, and so nothing here can fail to load (see "Self-contained" below).
//!
//! Command-line flags still set every persisted value, and the page still names them and says
//! where the settings file lives, because the address is the only one of them the page itself can
//! change -- for the others, that hint is the difference between a mistyped setting an operator
//! can fix and a dead end (Task 7 review, floor of Important 3 / Minor 8).
//!
//! # No chicken-and-egg
//!
//! [`render_page`] renders something coherent for every [`crate::feed::Connection`] variant,
//! including `NeverConnected` -- the page must be servable, and be useful, from the instant the
//! bridge starts, before any refbox has ever been reached (design spec §5.6).
//!
//! # Self-contained, deliberately
//!
//! The rendered HTML inlines its own CSS and links nothing external -- no CDN, no web font, no
//! external script. Venue internet being unreliable is the entire reason this bridge exists; a
//! status page that needed the internet to render correctly would fail exactly when it was most
//! needed.

use std::fmt::Write as _;

use crate::{
    discovery::Found,
    feed::{Connection, ConnectionStatus, RefboxAddress},
};

/// The outcome of the last thing the operator did on this page -- chose a refbox, or ran a scan --
/// in a sentence written for them.
///
/// Held by `server::AppState` between the action and the page it redirects to, rather than being
/// rendered straight into the response, so that a reload after the action shows what happened
/// instead of silently repeating it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub text: String,
    /// Whether what the operator asked for happened. `false` is not an internal error -- it is
    /// "your refbox did not answer" or "that is not an address", which is ordinary and expected.
    pub done: bool,
}

impl Notice {
    pub fn done(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            done: true,
        }
    }

    pub fn problem(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            done: false,
        }
    }
}

/// What the operator's last network scan turned up. An empty `found` is a perfectly ordinary
/// result (spec §9.3: venue networks and firewalls can block a scan outright), and the page says
/// so in words rather than showing an empty table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOutcome {
    /// The network as the operator reads it, e.g. `192.168.1.x`.
    pub network: String,
    pub port: u16,
    pub found: Vec<Found>,
}

/// Everything [`render_page`] needs, already resolved from live state. Kept separate from
/// `server::AppState` (the wiring module) so `render_page` stays a pure, directly testable string
/// transform with no lock, no clock read, and no knowledge of axum.
pub struct PageData {
    /// The connection state and, when disconnected, how long it has been -- always read together
    /// as one [`ConnectionStatus`] from [`crate::feed::ConnectionState::snapshot`], never as two
    /// separately-sourced pieces. See the module doc's "One source, not two" section.
    pub status: ConnectionStatus,
    pub keepalive_active: bool,
    /// The current event id, or empty if none is known yet.
    pub event_id: String,
    pub game_number: String,
    pub period: String,
    /// The refbox currently chosen -- what the supervisor is connected to, or trying to reach.
    pub refbox_address: RefboxAddress,
    pub white_on_right: bool,
    /// The court label operator setting, or empty if not set.
    pub court: String,
    /// The scheme, host and port the viewer's own browser used to reach this page (from the
    /// request's `Host` header) -- used to build the vMix addresses so they are always exactly
    /// what worked to load the page itself, rather than a guessed interface address on a
    /// multi-homed machine.
    pub base_url: Option<String>,
    /// Where the settings file lives on disk (`config::settings_location`) -- see the module
    /// doc's last paragraph.
    pub settings_file: String,
    /// What to pre-fill the scan form's network field with (`discovery::suggested_scan_network`),
    /// or empty when the bridge has nothing sensible to suggest.
    pub scan_network: String,
    /// The outcome of the operator's last action, or `None` if they have not done anything yet.
    pub notice: Option<Notice>,
    /// The last scan's results, or `None` if no scan has been run this session.
    pub scan: Option<ScanOutcome>,
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
    let (indicator_class, indicator_label) = match data.status.connection {
        Connection::Connected => ("live", "Connected"),
        Connection::NeverConnected => ("down", "Never connected to a refbox yet"),
        Connection::Disconnected => ("down", "Disconnected"),
    };

    let duration_html = data
        .status
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

    let notice_html = data
        .notice
        .as_ref()
        .map(|notice| {
            let class = if notice.done { "done" } else { "problem" };
            format!(
                "<p class=\"notice {class}\">{}</p>\n",
                escape_html(&notice.text)
            )
        })
        .unwrap_or_default();

    let scan_html = data.scan.as_ref().map(render_scan).unwrap_or_default();

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
<tr><th>Address</th><td>{refbox_address}</td></tr>
</table>
{notice_html}<form class="chooser" method="post" action="/refbox">
<label for="address">Read a different refbox:</label>
<input id="address" name="address" size="22" placeholder="192.168.1.50" autocomplete="off">
<button type="submit">Use this refbox</button>
</form>
<form class="chooser" method="post" action="/scan">
<label for="network">Or look for refboxes on</label>
<input id="network" name="network" size="16" value="{scan_network}" placeholder="192.168.1.5"
 autocomplete="off">
<label for="scan-port">port</label>
<input id="scan-port" name="port" size="6" value="{refbox_port}" autocomplete="off">
<button type="submit">Search the network</button>
</form>
<p class="hint">Searching checks every address on that network and takes a few seconds. It only
reports something as a refbox if it answers with a game, so anything else listening on the same
port is ignored. The first search may raise a firewall prompt on Windows; if searching is blocked
here, type the refbox's address instead.</p>
{scan_html}<h2>Operator settings</h2>
<table>
<tr><th>Side of pool</th><td>{side_text}</td></tr>
<tr><th>Court</th><td>{court_text}</td></tr>
</table>
<p class="hint">Every value above can also be set with a command-line flag
(<code>--refbox-host</code>, <code>--refbox-port</code>, <code>--port</code>,
<code>--white-on-right</code>, <code>--court</code>) and is remembered automatically for next
time. The refbox address is the only one this page can change; to fix a mistyped one of the
others, edit or delete the settings file: <code>{settings_file}</code></p>
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
        refbox_address = escape_html(&data.refbox_address.to_string()),
        refbox_port = data.refbox_address.port,
        scan_network = escape_html(&data.scan_network),
        notice_html = notice_html,
        scan_html = scan_html,
        side_text = side_text,
        court_text = court_text,
        settings_file = escape_html(&data.settings_file),
        vmix_addresses = vmix_addresses,
    )
}

/// The results of the operator's last network search: one row per refbox found, each with a
/// button that reads that refbox -- the same `POST /refbox` the typed-address field uses, with the
/// address filled in for them, because picking from a list and typing are the same action (see
/// `server`'s module doc).
///
/// The label beside each address is what actually lets an operator choose: at a two-court venue
/// both refboxes are on the same network, and "Game 14 · Second Half · 3:47 · 2–1" tells them
/// which is theirs where two nearly identical IP addresses would not.
///
/// Finding nothing says so in a sentence rather than showing an empty table -- see [`ScanOutcome`].
fn render_scan(scan: &ScanOutcome) -> String {
    let network = escape_html(&scan.network);
    if scan.found.is_empty() {
        return format!(
            "<p class=\"hint\">Nothing on {network} answered on port {} the last time this \
             computer looked.</p>\n",
            scan.port
        );
    }

    let mut rows = String::new();
    for refbox in &scan.found {
        let address = escape_html(&refbox.address.to_string());
        let label = escape_html(&refbox.label);
        // `write!` into a `String` never fails -- see the same note in `render_page`.
        let _ = writeln!(
            rows,
            "<tr><td><code>{address}</code></td><td>{label}</td>\
             <td><form method=\"post\" action=\"/refbox\">\
             <input type=\"hidden\" name=\"address\" value=\"{address}\">\
             <button type=\"submit\">Use this refbox</button></form></td></tr>"
        );
    }

    format!("<h3>Refboxes found on {network}</h3>\n<table class=\"found\">\n{rows}</table>\n")
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
.hint { font-size: 0.9rem; opacity: 0.8; max-width: 36rem; }
.notice { padding: 0.5rem 0.75rem; border-radius: 0.25rem; max-width: 36rem; }
.notice.done { background: rgba(26, 127, 55, 0.15); border-left: 0.25rem solid #1a7f37; }
.notice.problem { background: rgba(207, 34, 46, 0.12); border-left: 0.25rem solid #cf222e; }
.chooser { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: baseline; margin: 0.5rem 0; }
.chooser input { font: inherit; padding: 0.2rem 0.35rem; }
.chooser button, .found button { font: inherit; padding: 0.2rem 0.75rem; cursor: pointer; }
table.found td { padding-right: 1rem; vertical-align: baseline; }
table { border-collapse: collapse; margin: 0.25rem 0 1.25rem; }
th, td { padding: 0.15rem 0.75rem 0.15rem 0; text-align: left; font-weight: normal; }
th { opacity: 0.7; }
code { background: rgba(127, 127, 127, 0.15); padding: 0.1em 0.35em; border-radius: 0.25em; }
ul { padding-left: 1.2rem; }
"#;

/// A whole-number-of-seconds `Duration`, rendered as `"Xm Ys"` (or just `"Ys"` under a minute) --
/// a human-facing figure, not a data column (see `server.rs`'s `/status.json`, which serves the
/// raw seconds instead).
fn format_duration(d: std::time::Duration) -> String {
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
    use std::time::Duration;

    use crate::feed::ConnectionState;

    use super::*;

    fn base_data() -> PageData {
        PageData {
            status: ConnectionStatus {
                connection: Connection::NeverConnected,
                disconnected_for: None,
            },
            keepalive_active: true,
            event_id: String::new(),
            game_number: String::new(),
            period: String::new(),
            refbox_address: RefboxAddress::new("127.0.0.1", 8000),
            white_on_right: false,
            court: String::new(),
            base_url: Some("http://192.168.1.5:8099".to_string()),
            settings_file: "/home/operator/.config/overlay-bridge/default-config.toml".to_string(),
            scan_network: "192.168.1.5".to_string(),
            notice: None,
            scan: None,
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
            status: ConnectionStatus {
                connection: Connection::Connected,
                disconnected_for: None,
            },
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("indicator live"));
        assert!(!html.contains("class=\"duration\""));
    }

    #[test]
    fn a_disconnected_page_shows_the_down_indicator_and_the_duration() {
        let data = PageData {
            status: ConnectionStatus {
                connection: Connection::Disconnected,
                disconnected_for: Some(Duration::from_secs(135)),
            },
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("indicator down"));
        assert!(html.contains("Down for 2m 15s"));
    }

    /// The direct regression guard for Task 7 review's Important 1: a real reconnect, through the
    /// real `ConnectionState`, must never leave a stale duration behind for `render_page` to show
    /// beside a "Connected" indicator. This is the "Connected-plus-duration case" the review asked
    /// this test to gain -- previously the sibling test above only ever constructed the *already
    /// consistent* `Connected` + `None` pairing by hand, which could not have caught the bug: the
    /// real defect was in how `Connection` and its duration were sourced (two independently-timed
    /// reads), not in how `render_page` renders a pairing it's handed. Driving the real
    /// `ConnectionState` transition sequence, with no artificial delay before reading `snapshot()`
    /// back, is what actually exercises that sourcing path.
    #[tokio::test]
    async fn reconnecting_through_the_real_connection_state_leaves_no_stale_duration() {
        let connection = ConnectionState::new();

        connection.set_disconnected();
        // A real, if short, wait so a genuine nonzero duration has actually accumulated before
        // the reconnect -- proving the clear-on-reconnect isn't just "there was never anything to
        // clear yet".
        tokio::time::sleep(Duration::from_millis(50)).await;
        let while_disconnected = connection.snapshot();
        assert!(
            while_disconnected.disconnected_for.unwrap_or_default() > Duration::from_millis(1),
            "test setup should have let a real duration accumulate first"
        );

        connection.set_connected();
        // No sleep here, deliberately: the whole point is that `snapshot()` must already be
        // consistent the instant after `set_connected` returns, not after some catch-up window.
        let status = connection.snapshot();

        assert_eq!(status.connection, Connection::Connected);
        assert_eq!(
            status.disconnected_for, None,
            "a reconnect must clear the duration in the same update as the state change, with \
             no window in which a caller could observe them disagreeing"
        );

        let data = PageData {
            status,
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("indicator live"));
        assert!(
            !html.contains("class=\"duration\""),
            "the rendered page must not show a stale down-duration immediately after a real \
             reconnect"
        );
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
            refbox_address: RefboxAddress::new("192.168.1.50", 8000),
            ..base_data()
        };
        let html = render_page(&data);
        assert!(html.contains("White on right"));
        assert!(html.contains("Pool A"));
        assert!(html.contains("192.168.1.50:8000"));
    }

    /// Review floor item (Important 3, reduced to its minimum): the page must not be a dead end
    /// for an address (or any other setting) the operator wants to change. Proves the flag names
    /// and the settings file path both actually appear -- not just that some hint text exists.
    #[test]
    fn the_page_names_the_flags_and_settings_file_so_it_is_not_a_dead_end() {
        let html = render_page(&base_data());
        for flag in [
            "--refbox-host",
            "--refbox-port",
            "--port",
            "--white-on-right",
            "--court",
        ] {
            assert!(html.contains(flag), "missing flag hint {flag} in:\n{html}");
        }
        assert!(
            html.contains("/home/operator/.config/overlay-bridge/default-config.toml"),
            "missing settings file path in:\n{html}"
        );
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

    // ------------------------------------------------------ choosing a refbox on the page (Task 8)

    fn found(address: &str, port: u16, label: &str) -> Found {
        Found {
            address: RefboxAddress::new(address, port),
            label: label.to_string(),
        }
    }

    #[test]
    fn the_page_offers_a_field_for_typing_a_refbox_address() {
        // Manual entry is not a garnish on discovery: a first search can raise a Windows firewall
        // prompt and some venue networks block searching entirely (spec §9.3), so this field is
        // the path that always works and it must always be on the page.
        let html = render_page(&base_data());
        assert!(html.contains("<form class=\"chooser\" method=\"post\" action=\"/refbox\">"));
        assert!(html.contains("name=\"address\""));
        assert!(html.contains("Use this refbox"));
    }

    #[test]
    fn the_page_offers_a_network_search_prefilled_with_the_suggested_network() {
        let html = render_page(&base_data());
        assert!(html.contains("<form class=\"chooser\" method=\"post\" action=\"/scan\">"));
        assert!(
            html.contains("value=\"192.168.1.5\""),
            "the network field should be pre-filled with the suggestion, got:\n{html}"
        );
        assert!(
            html.contains("value=\"8000\""),
            "and the port field with the port already in use, got:\n{html}"
        );
    }

    #[test]
    fn the_page_says_what_searching_will_and_will_not_do() {
        // A broadcast volunteer needs to know it takes a few seconds, that a firewall prompt is
        // normal, and that typing the address is the way out if searching is blocked.
        let html = render_page(&base_data());
        assert!(html.contains("takes a few seconds"));
        assert!(html.contains("firewall"));
        // Unescaped here, unlike the notice text in `server`'s own test of the same sentence:
        // this one is the page's own fixed wording, not something interpolated into it.
        assert!(html.contains("type the refbox's address instead"));
    }

    #[test]
    fn the_outcome_of_the_last_action_is_shown() {
        let html = render_page(&PageData {
            notice: Some(Notice::done("Switched to the refbox at 192.168.1.50:8000.")),
            ..base_data()
        });
        assert!(html.contains("notice done"));
        assert!(html.contains("Switched to the refbox at 192.168.1.50:8000."));

        let html = render_page(&PageData {
            notice: Some(Notice::problem("Could not use 192.168.1.9:8000.")),
            ..base_data()
        });
        assert!(html.contains("notice problem"));
        assert!(html.contains("Could not use 192.168.1.9:8000."));
    }

    #[test]
    fn no_notice_is_shown_before_the_operator_has_done_anything() {
        let html = render_page(&base_data());
        assert!(!html.contains("class=\"notice"));
    }

    #[test]
    fn a_notice_repeating_what_the_operator_typed_is_not_rendered_as_markup() {
        // Every failure notice quotes the submitted text back, so an address field is a direct
        // route from typed input into this page -- and the bridge deliberately serves with no
        // password (spec §6), so "only the operator can type there" is not an assumption to make.
        let html = render_page(&PageData {
            notice: Some(Notice::problem(
                "Could not use \"<script>evil</script>\": nothing answered there.",
            )),
            ..base_data()
        });
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn search_results_list_each_refbox_with_its_label_and_a_button_that_reads_it() {
        let html = render_page(&PageData {
            scan: Some(ScanOutcome {
                network: "192.168.1.x".to_string(),
                port: 8000,
                found: vec![
                    found("192.168.1.50", 8000, "Game 14 · Second Half · 3:47 · 2–1"),
                    found("192.168.1.51", 8000, "Game 15 · Between Games"),
                ],
            }),
            ..base_data()
        });

        assert!(html.contains("Refboxes found on 192.168.1.x"));
        for (address, label) in [
            ("192.168.1.50:8000", "Game 14 · Second Half · 3:47 · 2–1"),
            ("192.168.1.51:8000", "Game 15 · Between Games"),
        ] {
            assert!(html.contains(address), "missing {address} in:\n{html}");
            assert!(html.contains(label), "missing {label} in:\n{html}");
            assert!(
                html.contains(&format!(
                    "<input type=\"hidden\" name=\"address\" value=\"{address}\">"
                )),
                "missing a button for {address} in:\n{html}"
            );
        }
    }

    #[test]
    fn a_search_that_found_nothing_says_so_rather_than_showing_an_empty_table() {
        let html = render_page(&PageData {
            scan: Some(ScanOutcome {
                network: "192.168.1.x".to_string(),
                port: 8000,
                found: Vec::new(),
            }),
            ..base_data()
        });
        assert!(html.contains("Nothing on 192.168.1.x answered on port 8000"));
        assert!(
            !html.contains("Refboxes found"),
            "an empty result must not be headed as if something was found:\n{html}"
        );
    }

    #[test]
    fn a_label_a_refbox_supplied_is_not_rendered_as_markup() {
        // The label is built from the candidate's own snapshot -- its game number is whatever that
        // machine sent, which is not this program's to trust.
        let html = render_page(&PageData {
            scan: Some(ScanOutcome {
                network: "192.168.1.x".to_string(),
                port: 8000,
                found: vec![found(
                    "192.168.1.50",
                    8000,
                    "Game <script>evil</script> · First Half",
                )],
            }),
            ..base_data()
        });
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn the_refbox_address_is_shown_as_it_would_be_typed_back_in() {
        let html = render_page(&PageData {
            refbox_address: RefboxAddress::new("::1", 8000),
            ..base_data()
        });
        assert!(
            html.contains("[::1]:8000"),
            "an IPv6 address must be shown bracketed, so pasting it back into the field works, \
             got:\n{html}"
        );
    }

    #[test]
    fn the_page_still_loads_nothing_from_anywhere_else() {
        // The forms added in Task 8 must not have brought in a framework: venue internet being
        // unreliable is the whole reason this bridge exists, and a status page that needed the
        // network to render would fail exactly when it was most needed.
        let html = render_page(&PageData {
            notice: Some(Notice::done("Switched to the refbox at 192.168.1.50:8000.")),
            scan: Some(ScanOutcome {
                network: "192.168.1.x".to_string(),
                port: 8000,
                found: vec![found(
                    "192.168.1.50",
                    8000,
                    "Game 14 · First Half · 1:00 · 0–0",
                )],
            }),
            ..base_data()
        });
        for forbidden in [
            "<script",
            "src=",
            "@import",
            "<link",
            "cdn.",
            "fonts.googleapis",
        ] {
            assert!(
                !html.contains(forbidden),
                "the page must be entirely self-contained, found {forbidden:?} in:\n{html}"
            );
        }
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
