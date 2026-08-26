//! Serves the bridge's live picture of the game over HTTP, and owns the wiring that joins every
//! other module together: the feed's snapshots, the Portal directory, and `tables`' pure row
//! shapes.
//!
//! # Routes
//!
//! `GET /scorebug`, `/penalties`, `/fouls`, `/warnings`, `/nextgame` each serve exactly what
//! [`tables`] builds for them -- a JSON array of objects, every row carrying a `connected`
//! column, as documented on those functions. `GET /status.json` is new here, not part of
//! `tables`' published shapes: a single-row array reporting whether the refbox is currently in
//! contact ([`feed::Connection`]) and the current game number and period. It follows the same
//! "always an array" convention as every other route so a poller can treat all six routes
//! uniformly, even though its columns are this module's own invention rather than a `tables`
//! shape -- the fuller operator status page (an HTML `GET /`, plus richer content such as the
//! discovery list and the keepalive-availability signal) is Task 7's job, not this one's.
//!
//! **Every response reads the live state fresh at request time**, via
//! [`state::LiveState::current`] inside each handler -- never a value cached between requests.
//! This module used to recompute a projected clock value here as well, from a fresh
//! [`Instant::now`] on every call; that projection is gone (spec §4.6), so `current` now simply
//! returns the last real snapshot the refbox sent, verbatim, and there is nothing left to
//! recompute. What every handler still reads fresh on every request is **connection state**
//! ([`is_connected`], backed by [`feed::ConnectionState::get`]) -- the thing that now decides
//! whether a table shows real values or blanks them (see `tables`' module doc's "The `connected`
//! column" section), and which can change between one request and the next exactly as the old
//! clock projection used to.
//!
//! # Closing the roster gap
//!
//! [`tables::penalties`], [`tables::fouls`] and [`tables::warnings`] all take a [`tables::Rosters`]
//! (cap-number-to-name maps for both teams), but [`portal::Directory`] had no way to hand one out:
//! its roster cache is keyed by [`TeamId`], and its only public accessors before this task were
//! `names_for` (display strings, no team id) and `refresh_roster`/`player_name` (both *require* a
//! `TeamId` a caller had no way to obtain). [`portal::Directory::team_ids_for`] closes that gap --
//! a small additive accessor, described fully in its own doc. [`current_rosters`] below is what
//! uses it: for each colour, it resolves that colour's team id for the game currently on screen,
//! then looks up a name only for the cap numbers that actually appear in that game's penalties,
//! fouls and warnings (never all 256 cap numbers) via the existing `player_name`.
//!
//! # The Portal refresh loop
//!
//! [`refresh_portal_loop`] is the only thing in this crate that drives
//! [`portal::Directory::refresh_schedule`] and [`portal::Directory::refresh_roster`] on a timer.
//! `portal`'s module doc explains why `Directory` was built to make every failure non-fatal and
//! retriable; this loop is what actually exercises that contract; without it, a single Portal
//! outage would be permanent for the rest of the run. It fires on a fixed interval (default
//! [`PORTAL_REFRESH_INTERVAL`], a parameter of [`refresh_portal_loop`] itself) and also whenever
//! [`consume_snapshots`] notices the event id or game number has changed, so a new game's teams do
//! not wait out a full idle interval to appear. Both triggers matter and are tested separately:
//! the timer alone is what covers a Portal that is down for a whole half with no game change to
//! wake it early.
//!
//! # Which game is "next"
//!
//! [`tables::next_game`] deliberately does not decide which game counts as "next" -- it renders
//! whatever it is handed. [`GameSnapshot::next_game_number`] already resolves that question
//! correctly, including its `BetweenGames` special case (where the feed's own `next_game_number`
//! field is already standing in for the *current* game, via [`GameSnapshot::game_number`], and
//! there is genuinely no separate "next" to show). Using that existing method -- rather than
//! reading the raw `next_game_number` field directly -- is what "do not invent a rule" means in
//! practice here: the rule already exists in `uwh-common`, and inventing a second one by hand
//! would risk disagreeing with it exactly in that edge case.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, Instant},
};

use axum::{Json, Router, extract::State, routing::get};
use reqwest::Client;
use tokio::{
    sync::{Notify, mpsc},
    time::{MissedTickBehavior, interval},
};
use uwh_common::{
    color::Color,
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{EventId, GameNumber, TeamId},
};

use crate::{
    feed::{Connection, ConnectionState},
    portal::{Directory, TeamNames},
    state::{Display, LiveState},
    tables::{self, Rosters},
};

/// Shared state every route handler reads from, and that [`consume_snapshots`] and
/// [`refresh_portal_loop`] both write to. Cheap to share: handed to axum as `Arc<AppState>`,
/// which the framework clones (just the `Arc`, not the contents) per request via its `State`
/// extractor, and cloned the same way into the two background tasks spawned alongside the
/// server.
pub struct AppState {
    /// The bridge's live picture of the game. Seeded with an all-default snapshot at startup
    /// (see [`AppState::new`]) so every route already has something coherent -- blank names,
    /// zeroed clock -- to serve before a refbox has ever been reached, the same
    /// no-chicken-and-egg principle the design spec applies to the operator status page.
    live: RwLock<LiveState>,
    /// The Portal directory for whichever event the feed currently reports, or `None` before the
    /// first snapshot carrying an event id has arrived. Wrapped in an inner `Arc` (not just the
    /// outer `RwLock`) so [`refresh_once`] can clone a handle out and drop the lock before making
    /// any network call -- no lock guard in this module is ever held across an `.await`.
    directory: RwLock<Option<Arc<Directory>>>,
    /// The operator's side-of-pool setting (`--white-on-right`), fixed for the life of the
    /// process. Persisting it and changing it at runtime is Task 7's job, not this one's.
    white_on_right: bool,
    /// The bridge's connection to the refbox right now -- see [`Connection`]. The only writer is
    /// [`crate::feed::Supervisor::run`], via the handle [`AppState::connection_handle`] hands out;
    /// every route handler reads it (through [`is_connected`]) to decide whether to serve real
    /// values or blank ones (see `tables`' module doc).
    connection: ConnectionState,
}

impl AppState {
    /// Builds a fresh, unconnected bridge state: no event known yet, no connection made yet
    /// ([`Connection::NeverConnected`]), and the live picture seeded from
    /// [`GameSnapshot::default`] so every route is servable immediately.
    pub fn new(white_on_right: bool) -> Self {
        Self {
            live: RwLock::new(LiveState::new(GameSnapshot::default(), Instant::now())),
            directory: RwLock::new(None),
            white_on_right,
            connection: ConnectionState::new(),
        }
    }

    /// A cloned handle to this state's connection tracker, for handing to
    /// [`crate::feed::Supervisor::run`] -- the only thing that ever writes it. Cheap: clones the
    /// `Arc` inside [`ConnectionState`], not any of `AppState`'s own data.
    pub fn connection_handle(&self) -> ConnectionState {
        self.connection.clone()
    }
}

/// Builds the axum app: the six routes described in this module's doc, all backed by `state`. An
/// unmatched path falls through to axum's own default 404 response -- nothing here needs to
/// special-case it.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/scorebug", get(get_scorebug))
        .route("/penalties", get(get_penalties))
        .route("/fouls", get(get_fouls))
        .route("/warnings", get(get_warnings))
        .route("/nextgame", get(get_next_game))
        .route("/status.json", get(get_status))
        .with_state(state)
}

async fn get_scorebug(State(state): State<Arc<AppState>>) -> Json<Vec<BTreeMap<String, String>>> {
    let display = current_display(&state);
    let names = names_for_game(&state, display.snapshot.game_number());
    Json(tables::scorebug(
        &display,
        names.as_ref(),
        state.white_on_right,
        is_connected(&state),
    ))
}

async fn get_penalties(State(state): State<Arc<AppState>>) -> Json<Vec<BTreeMap<String, String>>> {
    let display = current_display(&state);
    let rosters = current_rosters(&state, &display.snapshot);
    Json(tables::penalties(&display, &rosters, is_connected(&state)))
}

async fn get_fouls(State(state): State<Arc<AppState>>) -> Json<Vec<BTreeMap<String, String>>> {
    let display = current_display(&state);
    let rosters = current_rosters(&state, &display.snapshot);
    Json(tables::fouls(&display, &rosters, is_connected(&state)))
}

async fn get_warnings(State(state): State<Arc<AppState>>) -> Json<Vec<BTreeMap<String, String>>> {
    let display = current_display(&state);
    let rosters = current_rosters(&state, &display.snapshot);
    Json(tables::warnings(&display, &rosters, is_connected(&state)))
}

async fn get_next_game(State(state): State<Arc<AppState>>) -> Json<Vec<BTreeMap<String, String>>> {
    let display = current_display(&state);
    // See the module doc's "Which game is 'next'" section: this is the existing `uwh-common`
    // rule, not one invented here.
    let names = display
        .snapshot
        .next_game_number()
        .and_then(|number| names_for_game(&state, number));
    Json(tables::next_game(names.as_ref(), is_connected(&state)))
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<Vec<BTreeMap<String, String>>> {
    let display = current_display(&state);
    Json(status_row(&display, state.connection.get()))
}

/// The bridge's live picture right now -- the last real snapshot the refbox sent, relayed
/// verbatim (see `state`'s module doc). Read fresh on every call, never a value cached from a
/// previous request or from startup, even though (unlike before this task) nothing here is
/// actually time-dependent any more: a stale read would only be possible if a newer snapshot had
/// arrived and this simply failed to notice it, not because of any clock math.
fn current_display(state: &AppState) -> Display {
    read_lock(&state.live).current()
}

/// Whether a served table should show the refbox's real values right now, or blank them -- see
/// [`Connection::is_live`] and `tables`' module doc's "The `connected` column" section. Judged
/// entirely by the connection itself, never by `state.live`'s arrival timing.
fn is_connected(state: &AppState) -> bool {
    state.connection.get().is_live()
}

/// Looks up `game_number`'s team names, court and start time from the currently-known Portal
/// directory. `None` if no event has been learned from the feed yet, or if the directory itself
/// has nothing cached for that game -- in both cases the caller's fallback is the same as
/// `tables`' own: render blank name columns rather than an error.
fn names_for_game(state: &AppState, game_number: &str) -> Option<TeamNames> {
    read_lock(&state.directory).as_ref()?.names_for(game_number)
}

/// Builds a [`Rosters`] for `snapshot`'s current game, ready to hand to `tables::penalties`,
/// `tables::fouls` or `tables::warnings`. Empty (never a panic, never a partial team) if no
/// event is known yet, or if either side's team id or roster has not been resolved -- the cap
/// number alone still renders in that case, per `tables`' own contract.
fn current_rosters(state: &AppState, snapshot: &GameSnapshot) -> Rosters {
    let directory = read_lock(&state.directory);
    let Some(directory) = directory.as_ref() else {
        return Rosters::default();
    };
    build_rosters(directory, snapshot.game_number(), snapshot)
}

/// See [`current_rosters`]. Split out so it takes a plain `&Directory` -- easier to exercise from
/// a test that has already unwrapped the `Option`/`RwLock`/`Arc` layers `AppState` wraps it in.
fn build_rosters(directory: &Directory, game_number: &str, snapshot: &GameSnapshot) -> Rosters {
    let team_ids = directory.team_ids_for(game_number).unwrap_or_default();
    Rosters {
        black: team_ids
            .black
            .map(|id| roster_for(directory, &id, snapshot, Color::Black))
            .unwrap_or_default(),
        white: team_ids
            .white
            .map(|id| roster_for(directory, &id, snapshot, Color::White))
            .unwrap_or_default(),
    }
}

/// `team`'s roster, resolved only for the cap numbers that actually appear in `color`'s
/// penalties, fouls and warnings in `snapshot` -- never all 256 possible cap numbers. A cap
/// number with no cached name is simply omitted from the map (not inserted as an empty string);
/// `tables`' own row-building already treats a missing map entry as "no name" the same way it
/// treats an explicit empty one.
fn roster_for(
    directory: &Directory,
    team: &TeamId,
    snapshot: &GameSnapshot,
    color: Color,
) -> HashMap<u8, String> {
    cap_numbers_for(snapshot, color)
        .filter_map(|cap| directory.player_name(team, cap).map(|name| (cap, name)))
        .collect()
}

/// Every cap number that appears anywhere in `color`'s penalties, fouls or warnings in
/// `snapshot`.
fn cap_numbers_for(snapshot: &GameSnapshot, color: Color) -> impl Iterator<Item = u8> + '_ {
    snapshot.penalties[color]
        .iter()
        .map(|penalty| penalty.player_number)
        .chain(
            snapshot.fouls[Some(color)]
                .iter()
                .filter_map(|entry| entry.player_number),
        )
        .chain(
            snapshot.warnings[color]
                .iter()
                .filter_map(|entry| entry.player_number),
        )
}

/// Builds `/status.json`'s single row. See the module doc's "Routes" section for why this shape
/// is this module's own invention rather than a `tables` shape.
///
/// `contact` is [`Connection`], not the old timing-derived `state::Contact` this replaced -- see
/// the report for this task. There is deliberately no `sinceSeconds` (or similar) column any
/// more: the old one measured silence since the last message, which is exactly the quantity this
/// task established must never be treated as meaningful (a stopped clock produces long silence
/// legitimately). A "how long has the connection been down" figure would need its own tracking
/// this task does not add -- the fuller operator status page, including its own-worded contact
/// display, is Task 7's job (see the module doc's "Routes" section).
fn status_row(display: &Display, contact: Connection) -> Vec<BTreeMap<String, String>> {
    let contact_text = match contact {
        Connection::NeverConnected => "NeverConnected",
        Connection::Connected => "Live",
        Connection::Disconnected => "Lost",
    };

    let mut row = BTreeMap::new();
    row.insert("contact".to_string(), contact_text.to_string());
    row.insert(
        "gameNumber".to_string(),
        display.snapshot.game_number().clone(),
    );
    row.insert(
        "period".to_string(),
        display.snapshot.current_period.to_string(),
    );
    vec![row]
}

/// Reads snapshots from the feed supervisor's channel forever, applying each to the shared
/// [`LiveState`] and, on an event or game-number change, (re)pointing [`AppState::directory`] at
/// the right event and waking [`refresh_portal_loop`] so the new game's teams do not wait out a
/// full idle interval to appear. Returns once `snapshots` closes, which only happens if the feed
/// supervisor itself has stopped (it does not stop on its own -- see `feed::Supervisor::run`'s
/// doc).
pub async fn consume_snapshots(
    state: Arc<AppState>,
    mut snapshots: mpsc::UnboundedReceiver<GameSnapshot>,
    client: Client,
    portal_url: String,
    refresh_notify: Arc<Notify>,
) {
    let mut last_event_id: Option<EventId> = None;
    let mut last_game_number: Option<GameNumber> = None;

    while let Some(snapshot) = snapshots.recv().await {
        let now = Instant::now();
        let event_id = snapshot.event_id.clone();
        let game_number = snapshot.game_number().clone();

        write_lock(&state.live).apply(snapshot, now);

        // `Some` only when the feed just reported a *different* known event id -- `None` both
        // when the event id is unknown (`None` on the wire) and when it is unchanged.
        let new_event = event_id
            .as_ref()
            .filter(|&id| last_event_id.as_ref() != Some(id));
        if let Some(id) = new_event {
            *write_lock(&state.directory) = Some(Arc::new(Directory::new(
                client.clone(),
                portal_url.clone(),
                id.clone(),
            )));
        }

        let game_changed = last_game_number.as_ref() != Some(&game_number);
        if new_event.is_some() || game_changed {
            refresh_notify.notify_one();
        }

        // Only remember a *real* event id. A momentary `None` on the wire (a snapshot arriving
        // before the refbox has attached an event, or some other transient gap) must not be
        // allowed to overwrite the last known real one -- if it did, the very next snapshot
        // carrying that same real id back would look "new" again (since `last_event_id` would
        // have been cleared to `None`), triggering a needless `Directory` rebuild that throws
        // away every team name and roster already cached for an event nothing has actually left.
        if event_id.is_some() {
            last_event_id = event_id;
        }
        last_game_number = Some(game_number);
    }
}

/// How often the Portal is polled for fresh team names and rosters, absent any more urgent
/// trigger. Chosen as a balance between staleness -- a late roster correction, or a bracket slot
/// getting a team assigned, should show up within a single tournament changeover rather than
/// require restarting the bridge -- and being a polite, infrequent caller of an unauthenticated
/// public API (see `portal`'s module doc). This is the default at `main.rs`'s single call site;
/// [`refresh_portal_loop`] itself takes the interval as a parameter so a test can drive the timer
/// path in real time without a slow multi-second sleep.
pub const PORTAL_REFRESH_INTERVAL: Duration = Duration::from_secs(15);

/// Owns the Portal refresh loop -- see the module doc's "The Portal refresh loop" section. Runs
/// forever, retrying on every tick of `refresh_interval` (or eager wake) regardless of whether the
/// previous attempt succeeded: a failed fetch is silently absorbed by [`Directory`]'s own
/// never-fatal contract, and this loop simply tries again next time, never giving up for the rest
/// of the run. This is what covers the field case nothing else does: the Portal is down when the
/// bridge starts, the game number does not change for a whole half, and no eager wake ever fires
/// -- the timer alone is what keeps retrying through that.
pub async fn refresh_portal_loop(
    state: Arc<AppState>,
    refresh_notify: Arc<Notify>,
    refresh_interval: Duration,
) {
    let mut ticker = interval(refresh_interval);
    // A refresh that's merely running a little behind should just run once when it catches up,
    // not fire a burst of back-to-back catch-up ticks -- there is no benefit to hammering the
    // Portal repeatedly for state that changes on the order of minutes.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            () = refresh_notify.notified() => {}
        }
        refresh_once(&state).await;
    }
}

/// One refresh cycle: re-fetches the event schedule, then -- using the team ids it just resolved
/// for the game currently on screen -- refreshes both teams' rosters. A no-op if no event id has
/// been learned from the feed yet ([`AppState::directory`] is still empty). Every step can fail
/// independently and silently, by [`Directory`]'s own contract: a failure leaves the previous
/// cache exactly as it was, this function simply moves on, and the loop above tries again on its
/// next tick.
async fn refresh_once(state: &Arc<AppState>) {
    let directory = read_lock(&state.directory).clone();
    let Some(directory) = directory else {
        return;
    };

    directory.refresh_schedule().await;

    let game_number = current_display(state).snapshot.game_number().clone();
    if let Some(team_ids) = directory.team_ids_for(&game_number) {
        if let Some(id) = team_ids.black {
            directory.refresh_roster(&id).await;
        }
        if let Some(id) = team_ids.white {
            directory.refresh_roster(&id).await;
        }
    }
}

/// Reads `lock`, recovering the guard even if a previous holder panicked while holding it, rather
/// than propagating the poison as a panic of its own -- mirrors `portal`'s own helper of the same
/// name, and the same reasoning applies: nothing in this module panics while holding either lock,
/// so this exists purely as insurance.
fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// The write-side counterpart of [`read_lock`].
fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::{
        net::SocketAddr,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use serde_json::Value;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use uwh_common::{bundles::BlackWhiteBundle, game_snapshot::GamePeriod};

    use super::*;

    /// Spawns a real axum server backed by `state` on a loopback port and returns its address --
    /// tests drive it with real HTTP requests via `reqwest`, the same as vMix or any other third
    /// party would.
    async fn spawn_test_server(state: Arc<AppState>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener for the test server");
        let addr = listener.local_addr().expect("local_addr");
        let app = router(state);
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        addr
    }

    /// Marks `state` as connected without a real socket, for tests that want to see a table's
    /// real values and have nothing to do with connection lifecycle itself (that is what the
    /// dedicated tests further down, which drive a real `feed::Supervisor`, are for). `AppState`
    /// starts `NeverConnected` (see `AppState::new`), so any test asserting on a table's actual
    /// values -- rather than just its shape -- needs this first, or every value would already be
    /// blanked by `tables::finish_table`.
    fn mark_connected(state: &AppState) {
        state.connection_handle().set_connected();
    }

    async fn get_response(addr: SocketAddr, path: &str) -> reqwest::Response {
        reqwest::get(format!("http://{addr}{path}"))
            .await
            .unwrap_or_else(|e| panic!("GET {path} should succeed: {e}"))
    }

    async fn get_json(addr: SocketAddr, path: &str) -> Value {
        let text = get_response(addr, path)
            .await
            .text()
            .await
            .unwrap_or_else(|e| panic!("GET {path} body should be readable: {e}"));
        serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("GET {path} body should be valid JSON: {e}"))
    }

    /// Accepts exactly one connection and writes back `status_line` with `body`, mirroring
    /// `portal.rs`'s own `serve_one` test helper -- duplicated here rather than shared, since it
    /// is private to that module.
    async fn serve_once(listener: TcpListener, status_line: &str, body: String) {
        let (mut socket, _) = listener.accept().await.expect("accept a connection");
        drain_request(&mut socket).await;
        write_response(&mut socket, status_line, &body).await;
    }

    /// Like [`serve_once`], but loops forever, counting every connection it accepts into
    /// `attempts` and always responding with a server error -- simulates a Portal that is down
    /// for the whole test.
    async fn serve_failing_portal(listener: TcpListener, attempts: Arc<AtomicUsize>) {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            attempts.fetch_add(1, Ordering::SeqCst);
            drain_request(&mut socket).await;
            write_response(&mut socket, "500 Internal Server Error", "").await;
        }
    }

    async fn drain_request(socket: &mut tokio::net::TcpStream) {
        let mut chunk = [0u8; 4096];
        loop {
            let read = socket.read(&mut chunk).await.unwrap_or(0);
            if read == 0 || chunk[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
    }

    async fn write_response(socket: &mut tokio::net::TcpStream, status_line: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
    }

    // ---------------------------------------------------------------- routes, generally

    #[tokio::test]
    async fn every_route_returns_200_json_content_type_and_a_json_array() {
        let state = Arc::new(AppState::new(false));
        let addr = spawn_test_server(state).await;

        for path in [
            "/scorebug",
            "/penalties",
            "/fouls",
            "/warnings",
            "/nextgame",
            "/status.json",
        ] {
            let response = get_response(addr, path).await;
            assert_eq!(response.status(), 200, "GET {path} should return 200");
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            assert!(
                content_type.starts_with("application/json"),
                "GET {path} should be application/json, got {content_type:?}"
            );
            let body: Value = serde_json::from_str(
                &response
                    .text()
                    .await
                    .unwrap_or_else(|e| panic!("GET {path} body should be readable: {e}")),
            )
            .unwrap_or_else(|e| panic!("GET {path} body should be valid JSON: {e}"));
            assert!(
                body.is_array(),
                "GET {path} should return a JSON array, got {body:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_unknown_path_returns_404_rather_than_panicking() {
        let state = Arc::new(AppState::new(false));
        let addr = spawn_test_server(state).await;

        let response = get_response(addr, "/does-not-exist").await;
        assert_eq!(response.status(), 404);

        // The server must still be alive and answering real routes after that -- proves the
        // 404 didn't take the process down with it.
        let body = get_json(addr, "/scorebug").await;
        assert!(body.is_array());
    }

    // ---------------------------------------------------------------- /nextgame picks the right game

    #[tokio::test]
    async fn next_game_route_renders_the_feed_named_next_game_not_the_current_one() {
        let schedule_json = serde_json::json!({
            "games": [
                {
                    "number": "10",
                    "startsOn": "2026-08-01T09:00:00+10:00",
                    "court": "1",
                    "dark": {"assignment": {"teamId": "teams/1-A"}},
                    "light": {"assignment": {"teamId": "teams/2-A"}},
                },
                {
                    "number": "20",
                    "startsOn": "2026-08-01T10:00:00+10:00",
                    "court": "2",
                    "dark": {"assignment": {"teamId": "teams/3-A"}},
                    "light": {"assignment": {"teamId": "teams/4-A"}},
                },
            ],
            "teams": {
                "teams/1-A": {"name": "current dark"},
                "teams/2-A": {"name": "current light"},
                "teams/3-A": {"name": "next dark"},
                "teams/4-A": {"name": "next light"},
            },
        })
        .to_string();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let portal_addr = listener.local_addr().expect("local_addr");
        tokio::spawn(serve_once(listener, "200 OK", schedule_json));

        let directory = Directory::new(
            Client::new(),
            format!("http://{portal_addr}"),
            EventId::from_partial("evt"),
        );
        assert!(
            directory.refresh_schedule().await,
            "schedule fetch against the fixture body should succeed"
        );

        let state = Arc::new(AppState::new(false));
        mark_connected(&state);
        *write_lock(&state.directory) = Some(Arc::new(directory));
        *write_lock(&state.live) = LiveState::new(
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                game_number: "10".to_string(),
                next_game_number: "20".to_string(),
                event_id: Some(EventId::from_partial("evt")),
                ..Default::default()
            },
            Instant::now(),
        );

        let addr = spawn_test_server(state).await;
        let body = get_json(addr, "/nextgame").await;
        let row = &body[0];

        assert_eq!(row["blackTeam"].as_str(), Some("NEXT DARK"));
        assert_eq!(row["whiteTeam"].as_str(), Some("NEXT LIGHT"));
        assert_ne!(row["blackTeam"].as_str(), Some("CURRENT DARK"));
    }

    // ---------------------------------------------------------------- the clock is relayed verbatim

    #[tokio::test]
    async fn the_clock_is_never_projected_forward_between_requests_even_after_a_real_gap() {
        // This replaces a pre-Task-10 test of the opposite claim: it used to assert the served
        // clock *changed* between two requests spaced a real second apart, proving the old local
        // clock projection was recomputing on every request. That behaviour is deleted (spec
        // §4.6) -- the bridge now relays exactly what the refbox last sent, nothing else, so the
        // correct assertion is now the reverse: however much real time passes, and regardless of
        // how many real ticks arrived before the wait, the served clock is always exactly the
        // last real value, never a locally-continued one.
        let base = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 200,
            ..Default::default()
        };
        let state = Arc::new(AppState::new(false));
        mark_connected(&state);
        let t0 = Instant::now();
        *write_lock(&state.live) = LiveState::new(base.clone(), t0);
        // A second real tick, matching the steady-state arrival pattern the old projection logic
        // used to key off of -- kept here specifically so this test would have failed against
        // that old logic (which would have started projecting forward from this point) rather
        // than passing by accident because no second tick ever arrived.
        let t1 = Instant::now();
        write_lock(&state.live).apply(
            GameSnapshot {
                secs_in_period: 199,
                ..base
            },
            t1,
        );

        let addr = spawn_test_server(Arc::clone(&state)).await;

        let first = get_json(addr, "/scorebug").await;
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let second = get_json(addr, "/scorebug").await;

        assert_eq!(first[0]["clockSeconds"].as_str(), Some("199"));
        assert_eq!(
            second[0]["clockSeconds"].as_str(),
            Some("199"),
            "the clock must still read exactly the last real value after a real second has \
             passed with no further message -- never locally continued"
        );
    }

    // ---------------------------------------------------------------- Portal refresh loop
    //
    // Three separate tests, deliberately not one combined test, because the loop has two
    // independent triggers (its own timer, and an eager wake on event/game-number change) and a
    // test that drives both at once cannot tell them apart: a loop with no timer at all, only
    // `loop { notified().await; refresh_once().await }`, would still pass a test that only ever
    // sends notifications. The timer alone is what covers the field case nothing else does: the
    // Portal is down when the bridge starts, the game number does not change for a whole half,
    // and no eager wake ever fires.
    //
    // All three are deliberately real time, not `start_paused`: each refresh cycle here is a
    // genuine two-task TCP round trip (this test's own fake portal server has to actually accept,
    // read and respond on a separate spawned task), and tokio's paused-clock auto-advance is
    // unreliable across that kind of real socket I/O -- it can fast-forward the virtual clock past
    // a timer before the real OS-level I/O it was racing against has actually completed, which was
    // observed to make even a single request spuriously time out. `feed.rs`'s own paused-clock
    // tests avoid this because a refused connection fails immediately at the OS level, with no
    // real request/response exchange to race against.

    #[tokio::test]
    async fn portal_refresh_loop_retries_on_its_own_timer_with_no_notifications_sent_at_all() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let portal_addr = listener.local_addr().expect("local_addr");
        let attempts = Arc::new(AtomicUsize::new(0));
        tokio::spawn(serve_failing_portal(listener, Arc::clone(&attempts)));

        let state = Arc::new(AppState::new(false));
        *write_lock(&state.directory) = Some(Arc::new(Directory::new(
            Client::new(),
            format!("http://{portal_addr}"),
            EventId::from_partial("evt"),
        )));

        // `refresh_notify` is created but never notified -- if the loop only retried on eager
        // wake, this would hang at 1 attempt (the interval's unconditional first tick) forever.
        let refresh_notify = Arc::new(Notify::new());
        let loop_handle = tokio::spawn(refresh_portal_loop(
            Arc::clone(&state),
            refresh_notify,
            Duration::from_millis(50),
        ));

        tokio::time::sleep(Duration::from_millis(220)).await;

        let seen = attempts.load(Ordering::SeqCst);
        assert!(
            seen >= 3,
            "the loop's own timer should have retried repeatedly with no notifications sent at \
             all, got {seen} attempt(s)"
        );

        loop_handle.abort();
    }

    #[tokio::test]
    async fn portal_refresh_loop_retries_on_eager_wake_when_notified_repeatedly() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let portal_addr = listener.local_addr().expect("local_addr");
        let attempts = Arc::new(AtomicUsize::new(0));
        tokio::spawn(serve_failing_portal(listener, Arc::clone(&attempts)));

        let state = Arc::new(AppState::new(false));
        *write_lock(&state.directory) = Some(Arc::new(Directory::new(
            Client::new(),
            format!("http://{portal_addr}"),
            EventId::from_partial("evt"),
        )));

        let refresh_notify = Arc::new(Notify::new());
        // An interval far longer than this test can run for -- isolates the eager-wake path: the
        // only way this loop can retry within the test's lifetime is via `notify_one` below (plus
        // the interval's own unconditional first tick, which alone cannot reach the `>= 3`
        // threshold asserted below).
        let loop_handle = tokio::spawn(refresh_portal_loop(
            Arc::clone(&state),
            Arc::clone(&refresh_notify),
            Duration::from_secs(3600),
        ));

        // A short real sleep after each notification gives that cycle's real network round trip
        // time to finish before the next one is requested.
        for _ in 0..4 {
            refresh_notify.notify_one();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let seen = attempts.load(Ordering::SeqCst);
        assert!(
            seen >= 3,
            "the loop should have retried repeatedly on eager wake rather than giving up after \
             the first one, got {seen} attempt(s)"
        );

        loop_handle.abort();
    }

    #[tokio::test]
    async fn a_failing_portal_refresh_never_disturbs_game_data_arriving_through_the_real_feed_path()
    {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let portal_addr = listener.local_addr().expect("local_addr");
        let attempts = Arc::new(AtomicUsize::new(0));
        tokio::spawn(serve_failing_portal(listener, Arc::clone(&attempts)));

        let state = Arc::new(AppState::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());

        // Both background tasks run against the same shared `state`, exactly as `main.rs` wires
        // them: the feed consumer is what actually writes `state.live`, not this test poking it
        // directly -- so the assertion below has something real to be true about, rather than
        // checking a value nothing ever touched.
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            format!("http://{portal_addr}"),
            Arc::clone(&refresh_notify),
        ));
        let loop_handle = tokio::spawn(refresh_portal_loop(
            Arc::clone(&state),
            refresh_notify,
            Duration::from_millis(50),
        ));

        let event = EventId::from_partial("evt");
        tx.send(GameSnapshot {
            scores: BlackWhiteBundle { black: 5, white: 2 },
            game_number: "1".to_string(),
            event_id: Some(event),
            ..Default::default()
        })
        .expect("channel should accept the first snapshot");

        // Let several real refresh cycles run against the always-failing Portal while the feed
        // consumer is live and able to react.
        tokio::time::sleep(Duration::from_millis(300)).await;

        let seen = attempts.load(Ordering::SeqCst);
        assert!(
            seen >= 3,
            "the refresh loop should have retried multiple times during this window, got {seen} \
             attempt(s) -- if it stopped retrying, the rest of this test wouldn't be exercising \
             the failure case it's meant to"
        );

        let display = current_display(&state);
        assert_eq!(
            display.snapshot.scores.black, 5,
            "a Portal refresh failure must never disturb game data delivered through the real \
             feed path"
        );
        assert_eq!(display.snapshot.scores.white, 2);

        consumer.abort();
        loop_handle.abort();
    }

    // ---------------------------------------------------------------- consume_snapshots

    #[tokio::test]
    async fn a_new_event_id_creates_a_fresh_directory_replacing_the_previous_one() {
        let state = Arc::new(AppState::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            refresh_notify,
        ));

        tx.send(GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            event_id: Some(EventId::from_partial("event-a")),
            game_number: "1".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the first snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let directory_a = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist after the first event id is learned");

        tx.send(GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            event_id: Some(EventId::from_partial("event-b")),
            game_number: "1".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the second snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let directory_b = read_lock(&state.directory)
            .clone()
            .expect("a directory should still exist after the second event id is learned");

        assert!(
            !Arc::ptr_eq(&directory_a, &directory_b),
            "a genuinely different event id must replace the directory with a fresh one"
        );

        consumer.abort();
    }

    #[tokio::test]
    async fn a_momentary_missing_event_id_does_not_rebuild_the_directory_for_the_same_event() {
        let state = Arc::new(AppState::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            refresh_notify,
        ));

        let event = EventId::from_partial("evt");
        tx.send(GameSnapshot {
            event_id: Some(event.clone()),
            game_number: "1".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the first snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let directory_before = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist after the event id is learned");

        // A snapshot with no event id at all -- a transient gap on the wire, not a real change of
        // event -- followed by the same real event id coming back.
        tx.send(GameSnapshot {
            event_id: None,
            game_number: "1".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the blip");
        tokio::time::sleep(Duration::from_millis(50)).await;

        tx.send(GameSnapshot {
            event_id: Some(event),
            game_number: "1".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the recovered snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;

        let directory_after = read_lock(&state.directory)
            .clone()
            .expect("a directory should still exist after the blip");

        assert!(
            Arc::ptr_eq(&directory_before, &directory_after),
            "a momentary missing event id must not rebuild the directory (and so discard every \
             cached team name and roster) for what is still the same event"
        );

        consumer.abort();
    }

    #[tokio::test]
    async fn a_game_number_change_wakes_the_refresh_loop() {
        let state = Arc::new(AppState::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            Arc::clone(&refresh_notify),
        ));

        let event = EventId::from_partial("evt");
        // `current_period: FirstHalf` on both snapshots is load-bearing, not incidental: with
        // `..Default::default()` alone, `current_period` defaults to `BetweenGames`, and
        // `GameSnapshot::game_number()` (what `consume_snapshots` actually reads) returns
        // `next_game_number` -- not the `game_number` field -- whenever `current_period` is
        // `BetweenGames`. Both snapshots below would then read as game number `""` regardless of
        // the `game_number` field set here, `game_changed` would never go true, and this test
        // would hang waiting for a wake that correctly never fires. See the sibling
        // `a_new_event_id_creates_a_fresh_directory_replacing_the_previous_one` test above, which
        // sets the same field for the same reason.
        tx.send(GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            event_id: Some(event.clone()),
            game_number: "1".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the first snapshot");
        // The first snapshot always wakes the loop too (a new event) -- drain that wake so the
        // assertion below is specifically about the game-number change that follows it.
        tokio::time::timeout(Duration::from_secs(1), refresh_notify.notified())
            .await
            .expect("the first snapshot (a new event) should wake the refresh loop");

        tx.send(GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            event_id: Some(event),
            game_number: "2".to_string(),
            ..Default::default()
        })
        .expect("channel should accept the second snapshot");
        tokio::time::timeout(Duration::from_secs(1), refresh_notify.notified())
            .await
            .expect("a game-number change within the same event should also wake the refresh loop");

        consumer.abort();
    }

    // ---------------------------------------------------------------- connection state end-to-end
    //
    // These are the regression guards for "the trap" (spec §4.6, §5.4): the refbox goes
    // completely silent for ~25s whenever its clock is stopped, so nothing here may ever treat
    // silence itself as evidence of disconnection. Unlike the rest of this file's tests, these
    // drive a real `feed::Supervisor` against a real loopback socket rather than poking
    // `AppState` directly, because the property under test -- that connection state comes from
    // the connection and nothing else -- is a property of the wiring between `feed` and `server`,
    // not of either module in isolation.

    #[tokio::test]
    async fn a_never_connected_bridge_serves_connected_false_and_blank_tables() {
        // No `feed::Supervisor` involved at all here -- `AppState::new` alone, exactly as the
        // bridge looks the instant it starts, before any connection attempt has even begun. This
        // is the case the old timing-based `Contact` handled badly (see the report for this
        // task): it would report `Live` for the first few seconds after startup, purely because
        // the seeded default snapshot's arrival time was recent, with nothing behind it.
        let state = Arc::new(AppState::new(false));
        let addr = spawn_test_server(state).await;

        let body = get_json(addr, "/scorebug").await;
        let row = &body[0];

        assert_eq!(row["connected"].as_str(), Some("false"));
        assert_eq!(row["blackScore"].as_str(), Some(""));
        assert_eq!(row["clockSeconds"].as_str(), Some(""));
    }

    /// Runs a real `feed::Supervisor` against a real loopback socket, sends exactly one real
    /// snapshot, then leaves the connection open and silent for `silent_for` before asserting
    /// `/scorebug` still reports `connected: "true"` and exactly the values that one snapshot
    /// carried. Shared by the two regression guards for "the trap" (spec §4.6, §5.4) below, which
    /// differ only in how long they wait -- see each test's own doc for why both exist.
    async fn assert_scorebug_survives_silence(silent_for: Duration) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let refbox_addr = listener.local_addr().expect("local_addr");

        let state = Arc::new(AppState::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());

        let supervisor = tokio::spawn(crate::feed::Supervisor::run(
            refbox_addr,
            tx,
            state.connection_handle(),
        ));
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            refresh_notify,
        ));

        let (mut refbox_side, _) = listener
            .accept()
            .await
            .expect("accept the supervisor's connection");
        let one_real_message = format!(
            "{}\n",
            serde_json::to_string(&GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                secs_in_period: 613,
                scores: BlackWhiteBundle { black: 2, white: 1 },
                ..Default::default()
            })
            .expect("GameSnapshot should serialize")
        );
        refbox_side
            .write_all(one_real_message.as_bytes())
            .await
            .expect("write the one real snapshot");

        // Let the consumer apply it, then leave the connection open -- `refbox_side` stays in
        // scope, so the socket stays alive -- and send nothing further for `silent_for`. This is
        // exactly what the refbox itself does for ~25s every time the clock is stopped.
        tokio::time::sleep(silent_for).await;

        let addr = spawn_test_server(Arc::clone(&state)).await;
        let body = get_json(addr, "/scorebug").await;
        let row = &body[0];

        assert_eq!(
            row["connected"].as_str(),
            Some("true"),
            "the TCP connection is still alive and nothing about it has gone wrong -- \
             {silent_for:?} of silence alone must never flip this to false"
        );
        assert_eq!(
            row["clockSeconds"].as_str(),
            Some("613"),
            "the clock is stopped (nothing sent during the wait), so the last real value must be \
             served completely unchanged -- never blanked, never projected forward"
        );
        assert_eq!(row["blackScore"].as_str(), Some("2"));

        supervisor.abort();
        consumer.abort();
    }

    #[tokio::test]
    async fn a_six_second_silence_with_the_connection_alive_keeps_serving_real_values_and_connected_true()
     {
        // A fast sanity check, not the authoritative guard for the trap -- see
        // `a_very_long_silence_...` below for that one. On its own this only rules out a rule
        // whose threshold sits under six seconds (the deleted `state::Contact`'s old 3-second
        // `CONTACT_THRESHOLD` among them). A regression with any threshold >= 6s -- including the
        // 10-15s keepalive detection window spec §4.6 itself cites, or anything approaching the
        // 25s field-measured stopped-clock silence -- would still pass this test alone and go
        // undetected; that gap is exactly what the longer guard below closes.
        assert_scorebug_survives_silence(Duration::from_secs(6)).await;
    }

    #[tokio::test]
    async fn a_very_long_silence_with_the_connection_alive_keeps_serving_real_values_and_connected_true()
     {
        // THE regression guard for the trap (spec §4.6, §5.4). 30 real seconds, deliberately
        // comfortably above every timing threshold named anywhere in the design: the deleted
        // 3-second `CONTACT_THRESHOLD`, the 10-15s keepalive detection window §4.6 itself cites,
        // and the 25s field-measured stopped-clock silence -- "well over any plausible timeout"
        // has to mean this, not the 6-second test above alone. This spends real wall-clock time
        // on purpose rather than a paused/virtual one: the serving path this exercises is built
        // on `std::time::Instant`, which ignores tokio's paused clock entirely, so faking the
        // elapsed time here would mean not actually testing the real code path.
        //
        // Verified directly while fixing this test (see the report for this task for the full
        // transcript): temporarily reintroducing a silence-based `is_connected` with a 15-second
        // threshold -- comfortably inside the gap the 6-second test above cannot see -- left that
        // shorter test green (false confidence) while this one correctly went red; reverting to
        // the real `state.connection.get().is_live()` makes both pass again.
        assert_scorebug_survives_silence(Duration::from_secs(30)).await;
    }

    #[tokio::test]
    async fn disconnection_blanks_every_table_and_reconnection_restores_real_values() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let refbox_addr = listener.local_addr().expect("local_addr");

        let state = Arc::new(AppState::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());

        let supervisor = tokio::spawn(crate::feed::Supervisor::run(
            refbox_addr,
            tx,
            state.connection_handle(),
        ));
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            refresh_notify,
        ));

        let (first_connection, _) = listener
            .accept()
            .await
            .expect("accept the first connection");
        wait_for_connection(&state, Connection::Connected).await;

        let addr = spawn_test_server(Arc::clone(&state)).await;

        // Still connected but nothing sent yet -- `/scorebug` should already read `connected` and
        // blank values (the seeded startup default), same as the never-connected case above.
        // This isn't the main point of the test, just establishing the starting condition.
        drop(first_connection); // a clean close -- EOF, not a hang
        wait_for_connection(&state, Connection::Disconnected).await;

        let while_disconnected = get_json(addr, "/scorebug").await;
        let row = &while_disconnected[0];
        assert_eq!(row["connected"].as_str(), Some("false"));
        assert_eq!(row["blackScore"].as_str(), Some(""));
        assert_eq!(row["clockSeconds"].as_str(), Some(""));

        // The supervisor keeps retrying (`feed::RECONNECT_DELAY`) -- accept its next attempt and
        // send a real snapshot over it.
        let (mut second_connection, _) =
            tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .expect("the supervisor should have reconnected")
                .expect("second accept should succeed");
        let payload = format!(
            "{}\n",
            serde_json::to_string(&GameSnapshot {
                current_period: GamePeriod::SecondHalf,
                secs_in_period: 44,
                scores: BlackWhiteBundle { black: 9, white: 3 },
                ..Default::default()
            })
            .expect("GameSnapshot should serialize")
        );
        second_connection
            .write_all(payload.as_bytes())
            .await
            .expect("write the reconnection snapshot");

        wait_for_connection(&state, Connection::Connected).await;
        wait_for_scores(&state, 9, 3).await;

        let after_reconnect = get_json(addr, "/scorebug").await;
        let row = &after_reconnect[0];
        assert_eq!(row["connected"].as_str(), Some("true"));
        assert_eq!(row["blackScore"].as_str(), Some("9"));
        assert_eq!(row["whiteScore"].as_str(), Some("3"));
        assert_eq!(row["clockSeconds"].as_str(), Some("44"));

        supervisor.abort();
        consumer.abort();
    }

    /// Polls `state`'s connection handle until it reports `target`, failing the test (rather than
    /// hanging forever) if it never does within a generous real-time budget. Used instead of a
    /// fixed sleep-then-assert because exactly how long the supervisor takes to notice and
    /// publish a transition depends on async scheduling, not a fixed duration.
    async fn wait_for_connection(state: &AppState, target: Connection) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while state.connection_handle().get() != target {
            assert!(
                tokio::time::Instant::now() < deadline,
                "connection state did not reach {target:?} in time, currently {:?}",
                state.connection_handle().get()
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// Polls `state`'s live snapshot until both scores match, for the same reason as
    /// [`wait_for_connection`]: `consume_snapshots` applies an incoming snapshot asynchronously,
    /// so there is no single instant after which it is guaranteed to have happened.
    async fn wait_for_scores(state: &AppState, black: u8, white: u8) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = current_display(state).snapshot;
            if snapshot.scores.black == black && snapshot.scores.white == white {
                return;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "scores did not reach {black}-{white} in time"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    // ---------------------------------------------------------------- roster building

    #[tokio::test]
    async fn build_rosters_only_resolves_cap_numbers_that_actually_appear_in_the_snapshot() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let portal_addr = listener.local_addr().expect("local_addr");
        let roster_json = serde_json::json!({
            "roster": [
                {"capNumber": 7, "rosterName": "Smith"},
                {"capNumber": 9, "rosterName": "Ng"},
            ]
        })
        .to_string();
        tokio::spawn(serve_once(listener, "200 OK", roster_json));

        let directory = Directory::new(
            Client::new(),
            format!("http://{portal_addr}"),
            EventId::from_partial("evt"),
        );
        let team = TeamId::from_partial("1-A");
        assert!(directory.refresh_roster(&team).await);

        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![uwh_common::game_snapshot::PenaltySnapshot {
                    player_number: 7,
                    time: uwh_common::game_snapshot::PenaltyTime::Seconds(30),
                    infraction: uwh_common::game_snapshot::Infraction::Unknown,
                }],
                white: Vec::new(),
            },
            ..Default::default()
        };

        let roster = roster_for(&directory, &team, &snapshot, Color::Black);
        assert_eq!(roster.get(&7).map(String::as_str), Some("Smith"));
        // Cap 9 has a name on the roster, but never appears in this snapshot's black penalties,
        // fouls or warnings -- it must not be resolved (and, more importantly, a real game's
        // roster of ~10-15 players is bounded, so this also documents that `build_rosters` never
        // walks all 256 possible cap numbers).
        assert_eq!(roster.get(&9), None);
    }
}
