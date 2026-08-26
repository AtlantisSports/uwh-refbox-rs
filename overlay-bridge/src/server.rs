//! Serves the bridge's live picture of the game over HTTP, and owns the wiring that joins every
//! other module together: the feed's snapshots, the Portal directory, and `tables`' pure row
//! shapes.
//!
//! # Routes
//!
//! `GET /scorebug`, `/penalties`, `/fouls`, `/warnings`, `/nextgame` each serve exactly what
//! [`tables`] builds for them -- a JSON array of objects, every row carrying a `connected`
//! column, as documented on those functions. `GET /status.json` reports, in the same
//! "always a JSON array" shape (its columns are this module's own invention, not a `tables`
//! shape), whether the refbox is currently in contact ([`feed::Connection`]), the current game
//! number and period, how long the connection has been down (Task 7, only while it is down --
//! see [`status`]'s module doc for why), and whether the TCP keepalive check is actually active
//! (also Task 7). `GET /` is the human-facing counterpart of the same information, rendered as a
//! self-contained HTML operator status page by [`status::render_page`] -- see that module's doc
//! for the full rationale.
//!
//! `POST /refbox` and `POST /scan` are the two things that page can *do* (Task 8), and they are
//! the only routes in this crate that change anything. `/refbox` points the bridge at a different
//! refbox -- from the manual-entry field or from a scan result, which are two front ends onto one
//! mechanism ([`choose_refbox`]); `/scan` checks the local network for refboxes. Both answer with
//! a redirect back to `GET /` rather than a page of their own, so a browser reload after either
//! one simply re-reads the status page instead of silently re-running the action.
//!
//! # Choosing a refbox never breaks a working one
//!
//! [`choose_refbox`] proves a candidate is a refbox **before** anything about the running bridge
//! is touched: it connects to the candidate as a separate, throwaway connection and reads the
//! snapshot a refbox replays on connect (`discovery::probe`). A mistyped address, or an address
//! with something other than a refbox on it, therefore comes back as a sentence on the status page
//! with the existing connection still running and nothing changed at all. Only once the candidate
//! has answered as a refbox does anything change -- and then, in this order: the bridge is marked
//! out of contact, its picture of the old game is thrown away, and only then is the supervisor
//! pointed at the new address. That order is what stops the previous refbox's game being served,
//! or displayed, while the new one is still being reached.
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
    net::Ipv4Addr,
    path::PathBuf,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, Instant},
};

use axum::{
    Form, Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, Redirect},
    routing::{get, post},
};
use reqwest::Client;
use serde::Deserialize;
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
    config, discovery,
    feed::{Connection, ConnectionState, ConnectionStatus, FeedMessage, FeedTarget, RefboxAddress},
    portal::{Directory, TeamNames},
    state::{Display, LiveState},
    status::{self, Notice, ScanOutcome},
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
    /// The operator's side-of-pool setting (`--white-on-right`, persisted by `config` since
    /// Task 7), fixed for the life of the process.
    white_on_right: bool,
    /// The bridge's connection to the refbox right now, and (since Task 7) when it last dropped --
    /// see [`Connection`] and [`crate::feed::ConnectionStatus`]. The only writer is
    /// [`crate::feed::Supervisor::run`], via the handle [`AppState::connection_handle`] hands out;
    /// every route handler reads it (through [`is_connected`] for the flag alone, or
    /// [`ConnectionState::snapshot`] where the duration is needed too -- see
    /// [`crate::feed::ConnectionStatus`]'s doc for why those must never be read as two separate
    /// calls) to decide whether to serve real values or blank ones (see `tables`' module doc).
    connection: ConnectionState,
    /// Which refbox the bridge reads -- **the** address, not a copy of one: this is the same
    /// handle [`crate::feed::Supervisor::run`] takes its address from, so what the status page
    /// displays and what the supervisor connects to cannot drift apart. Task 7 kept a separate
    /// display-only copy here because the address could not change while the bridge ran; Task 8
    /// makes it changeable, and a second copy of a value that changes is exactly the kind of
    /// two-sources bug this crate has already had once (see [`ConnectionStatus`]'s doc).
    target: FeedTarget,
    /// The court label operator setting (design spec §5.2, persisted by `config` since Task 7) --
    /// display-only, unlike the address above.
    court: String,
    /// Where to write a refbox address the operator chooses while the bridge is running, so the
    /// next run comes back to the same refbox. `None` means "nowhere": the settings file's
    /// location could not be worked out (see [`config::settings_path`]), or -- in this crate's own
    /// tests -- the test deliberately does not want a real user's settings file touched. A choice
    /// still takes effect for the current run either way; the operator is told plainly when it
    /// could not be remembered.
    settings_path: Option<PathBuf>,
    /// What the last network scan the operator ran turned up, or `None` if they have not run one.
    /// Kept here rather than rendered straight into a response so the results survive the
    /// redirect that follows the scan, and any page reload after it.
    last_scan: RwLock<Option<ScanOutcome>>,
    /// The outcome of the operator's last action on the status page -- choosing a refbox, or
    /// scanning -- in plain English, shown until they do something else.
    notice: RwLock<Option<Notice>>,
    /// What `consume_snapshots` has last seen on the feed, used to notice when the event or game
    /// has changed. Held here rather than as locals inside that loop for one reason: it has to be
    /// **cleared by [`AppState::forget_game`]** along with everything else belonging to a refbox
    /// the operator has left. See that method's doc.
    last_seen: RwLock<LastSeen>,
    /// Serializes choosing a refbox. An async mutex, not a lock: it is deliberately held across
    /// the probe's `.await` (see [`choose_refbox`]), which a `std` lock could not be.
    switching: tokio::sync::Mutex<()>,
    /// Held for the duration of a network search, and only ever `try_lock`ed -- a second search
    /// starting while one is running is refused with a sentence rather than queued behind it (see
    /// [`run_scan`]). A double-clicked "Search the network" button must not launch two sweeps of
    /// 254 addresses.
    scanning: tokio::sync::Mutex<()>,
}

/// The last event and game `consume_snapshots` saw, so it can tell a change from a repeat. See
/// [`AppState::last_seen`].
#[derive(Debug, Default)]
struct LastSeen {
    event_id: Option<EventId>,
    game_number: Option<GameNumber>,
}

impl AppState {
    /// Builds a fresh, unconnected bridge state from the bridge's resolved settings: no event
    /// known yet, no connection made yet ([`Connection::NeverConnected`]), and the live picture
    /// seeded from [`GameSnapshot::default`] so every route is servable immediately.
    ///
    /// **Takes every setting as one value, and there is no other way in.** The optional builder
    /// methods this replaced (`with_operator_info`, `with_settings_path`) were a bug waiting to
    /// happen: forgetting one in `main.rs` left the bridge silently reading a default address
    /// instead of the configured one, with nothing failing to say so. See [`config::Resolved`].
    pub fn new(settings: config::Resolved) -> Self {
        Self {
            live: RwLock::new(LiveState::new(GameSnapshot::default(), Instant::now())),
            directory: RwLock::new(None),
            white_on_right: settings.white_on_right,
            connection: ConnectionState::new(),
            target: FeedTarget::new(settings.refbox),
            court: settings.court,
            settings_path: settings.settings_path,
            last_scan: RwLock::new(None),
            notice: RwLock::new(None),
            last_seen: RwLock::new(LastSeen::default()),
            switching: tokio::sync::Mutex::new(()),
            scanning: tokio::sync::Mutex::new(()),
        }
    }

    /// A cloned handle to this state's connection tracker, for handing to
    /// [`crate::feed::Supervisor::run`] -- the only thing that ever writes it. Cheap: clones the
    /// `Arc` inside [`ConnectionState`], not any of `AppState`'s own data.
    pub fn connection_handle(&self) -> ConnectionState {
        self.connection.clone()
    }

    /// A cloned handle to the refbox address in use, for handing to
    /// [`crate::feed::Supervisor::run`]. Cheap in the same way as [`AppState::connection_handle`],
    /// and for the same reason: the supervisor and the status page must be looking at one value,
    /// not two copies of one.
    pub fn target_handle(&self) -> FeedTarget {
        self.target.clone()
    }

    /// Throws away the bridge's live picture of the game, so nothing of the refbox just left can
    /// be served as though it belonged to the one just chosen.
    ///
    /// Blanking the tables is not enough on its own here. `tables` blanks every value while the
    /// connection is down, so a poll during the switch is already safe -- but `/status.json` and
    /// the operator status page show the event, game and period unconditionally, and after
    /// switching to a refbox that turns out to be slow to reach (or that goes away again), those
    /// would go on naming the *previous* refbox's game for as long as it took, right beside a red
    /// "Disconnected". That is the confidently-wrong display spec §4.6 exists to remove, and it is
    /// not a millisecond-scale window: it lasts as long as the new refbox is unreachable.
    ///
    /// **Everything belonging to that refbox goes, not only the scores.** The live picture is the
    /// obvious half; the Portal directory and the last-seen event and game are the other half, and
    /// leaving them behind reproduces exactly the same fault one step further along. The directory
    /// is built for whichever *event* the previous refbox reported, and team and player names are
    /// looked up in it by game number alone -- so a refbox on a different event whose snapshots
    /// carry no event id (or the same game numbers, as tournament game numbers routinely are)
    /// would have its games resolved against the event just left, and the overlay would show the
    /// wrong teams' names for the right game. And `last_seen` is what decides whether an arriving
    /// event id counts as "new": left set, an event the bridge has genuinely left and returned to
    /// would look unchanged, and the stale directory would never be rebuilt.
    ///
    /// Not a timing rule and not a liveness rule: nothing here consults how long it has been since
    /// a message arrived (see `state`'s module doc). It fires exactly once, on an explicit
    /// operator action.
    fn forget_game(&self) {
        *write_lock(&self.live) = LiveState::new(GameSnapshot::default(), Instant::now());
        *write_lock(&self.directory) = None;
        *write_lock(&self.last_seen) = LastSeen::default();
    }
}

/// The bridge, assembled and running: its shared state, plus the background tasks that keep it fed
/// -- the feed supervisor, the snapshot consumer and the Portal refresh loop.
///
/// Dropping this stops those tasks. `main` holds it for the life of the program (so nothing stops
/// until the program does); this crate's own tests hold it for the life of a test.
pub struct Bridge {
    pub state: Arc<AppState>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Assembles a running bridge from its resolved settings: builds the shared state, points the feed
/// supervisor at the refbox those settings name, and starts the snapshot consumer and the Portal
/// refresh loop. Everything except binding the HTTP listener, which `main` does with
/// `settings.port` and [`router`].
///
/// **This lives here, not in `main.rs`, so that it can be tested** (Task 8 review, Important 3).
/// The wiring it performs -- in particular "the supervisor reads the refbox the settings name" --
/// was previously a hand-written sequence in `main` that no test could reach, where deleting one
/// line silently substituted a built-in default for every configured address. With the assembly in
/// one testable function and [`config::Resolved`] leaving no setting optional, that shape of bug
/// has nowhere left to hide.
pub fn start(settings: config::Resolved, portal_url: String) -> Bridge {
    let state = Arc::new(AppState::new(settings));

    let (tx, rx) = mpsc::unbounded_channel();
    // The supervisor takes the shared handle rather than an address value: the operator can point
    // the bridge at a different refbox from the status page at any time, and the supervisor
    // follows that handle (see `feed::FeedTarget`).
    let supervisor = tokio::spawn(crate::feed::Supervisor::run(
        state.target_handle(),
        tx,
        state.connection_handle(),
    ));

    let refresh_notify = Arc::new(Notify::new());
    let consumer = tokio::spawn(consume_snapshots(
        Arc::clone(&state),
        rx,
        Client::new(),
        portal_url,
        Arc::clone(&refresh_notify),
    ));
    let refresher = tokio::spawn(refresh_portal_loop(
        Arc::clone(&state),
        refresh_notify,
        PORTAL_REFRESH_INTERVAL,
    ));

    Bridge {
        state,
        tasks: vec![supervisor, consumer, refresher],
    }
}

/// Builds the axum app: the routes described in this module's doc, all backed by `state`.
/// An unmatched path falls through to axum's own default 404 response -- nothing here needs to
/// special-case it.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/scorebug", get(get_scorebug))
        .route("/penalties", get(get_penalties))
        .route("/fouls", get(get_fouls))
        .route("/warnings", get(get_warnings))
        .route("/nextgame", get(get_next_game))
        .route("/status.json", get(get_status))
        .route("/", get(get_status_page))
        .route("/refbox", post(post_refbox))
        .route("/scan", post(post_scan))
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
    // ONE read of both the connection flag and its duration -- see
    // `feed::ConnectionStatus`'s doc for why this must never be two separate calls.
    let status = state.connection.snapshot();
    Json(status_row(
        &display,
        status,
        state.connection.keepalive_active(),
    ))
}

/// `GET /` -- the operator status page. See [`status`]'s module doc for the full rationale;
/// this handler's only job is extracting `state` (and the request's own `Host` header, for the
/// vMix addresses `status::render_page` lists) into a [`status::PageData`] and delegating the
/// actual rendering to that pure function.
async fn get_status_page(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Html<String> {
    let display = current_display(&state);

    let base_url = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(|host| format!("http://{host}"));

    let address = state.target.current();
    let data = status::PageData {
        // ONE read of both the connection flag and its duration -- see
        // `feed::ConnectionStatus`'s doc for why this must never be two separate calls.
        status: state.connection.snapshot(),
        keepalive_active: state.connection.keepalive_active(),
        event_id: display
            .snapshot
            .event_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        game_number: display.snapshot.game_number().clone(),
        period: display.snapshot.current_period.to_string(),
        white_on_right: state.white_on_right,
        court: state.court.clone(),
        base_url,
        settings_file: config::settings_location(),
        scan_network: discovery::suggested_scan_network(&address),
        refbox_address: address,
        notice: read_lock(&state.notice).clone(),
        scan: read_lock(&state.last_scan).clone(),
    };

    Html(status::render_page(&data))
}

/// What a state-changing route answers with, once it is allowed to run: a redirect back to
/// `GET /`, or a refusal (see [`refuse_cross_site`]).
type ActionResult = Result<Redirect, (StatusCode, &'static str)>;

/// The sentence a refused request gets back. It is written for the rare case where a real
/// operator sees it -- an unusual browser, or a bookmarklet -- rather than for the attacker it is
/// actually aimed at, who will never read it.
const CROSS_SITE_REFUSAL: &str = "This can only be done from the bridge's own status page. Open \
                                  the bridge's address in a browser and use the buttons there.\n";

/// Refuses a request that some *other* website's page made, on the routes that change what is on
/// air. Every other request is allowed through untouched.
///
/// # Why the state-changing routes need this and the read-only ones do not
///
/// The bridge binds every network interface, has no password by design (design spec §6: anyone on
/// the venue network can read it), and its two actions are plain HTML form posts. Reading a table
/// is harmless -- that is the whole point of the bridge. But `POST /refbox` points the bridge at a
/// different refbox, which takes the current game's graphic off air, and a form post is exactly
/// what any web page open in a browser on the streaming PC can make to any address it likes,
/// without ever being able to read the reply. So an ordinary advert or a mistyped URL, in a tab
/// nobody is even looking at, could switch courts in the middle of a broadcast.
///
/// # Why this check and not a login
///
/// `Sec-Fetch-Site` is set by the **browser**, not by the page making the request, and cannot be
/// forged from JavaScript -- a request originating from a page on another site arrives stamped
/// `cross-site`. Refusing exactly that value costs a volunteer nothing: the status page's own
/// forms are same-origin, so every real operator click is unaffected, and there is no password to
/// distribute, forget or lock someone out with five minutes before a final. A request with no
/// `Sec-Fetch-Site` header at all (a script, `curl`, an old browser) is deliberately still
/// allowed -- this closes the browser-driven path, which is the one an operator cannot see
/// happening, and does not pretend to be authentication for a network the design already treats
/// as trusted to read.
fn refuse_cross_site(headers: &HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    let cross_site = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("cross-site"));

    if cross_site {
        Err((StatusCode::FORBIDDEN, CROSS_SITE_REFUSAL))
    } else {
        Ok(())
    }
}

/// The manual-entry field, and the hidden field behind every "use this refbox" button in a scan
/// result -- deliberately the same field name posted to the same route, because they are the same
/// action (see the module doc). `serde(default)` rather than a required field so a form that
/// arrives without it produces the page's own "type an address" sentence, not a bare 400 an
/// operator can do nothing with.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AddressForm {
    address: String,
}

/// `POST /refbox` -- point the bridge at a different refbox. Refused if some other website's page
/// made the request; see [`refuse_cross_site`].
async fn post_refbox(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<AddressForm>,
) -> ActionResult {
    refuse_cross_site(&headers)?;
    let notice = choose_refbox(&state, &form.address).await;
    *write_lock(&state.notice) = Some(notice);
    Ok(Redirect::to("/"))
}

/// The scan form. Both fields are text, and both are `serde(default)`, for the same reason as
/// [`AddressForm`]: a missing or unreadable value must produce a sentence, never a bare 400.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ScanForm {
    network: String,
    port: String,
}

/// `POST /scan` -- check the local network for refboxes. Refused on the same terms as
/// [`post_refbox`]: a scan changes nothing about what is on air, but it does sweep 254 addresses,
/// and there is no reason for another site to be able to start one.
async fn post_scan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<ScanForm>,
) -> ActionResult {
    refuse_cross_site(&headers)?;
    let notice = run_scan(&state, &form.network, &form.port).await;
    *write_lock(&state.notice) = Some(notice);
    Ok(Redirect::to("/"))
}

/// Points the bridge at the refbox the operator submitted, and reports back in plain English.
///
/// The order of what follows is the whole safety argument, and it is why this is one function
/// rather than logic spread across a handler (see the module doc's "Choosing a refbox never breaks
/// a working one"):
///
/// 1. **Read the address.** Unreadable input is reported and nothing is touched.
/// 2. **Stop if it is the address already in use.** Re-submitting it (a double-click, a reloaded
///    form) must not drop a working connection to reconnect to the identical place.
/// 3. **Probe the candidate**, on a separate throwaway connection. This is the step that can take
///    a couple of seconds, and it happens while the existing connection is still running
///    untouched: if the candidate is unreachable, or is not a refbox, the answer is a sentence and
///    the bridge carries on exactly as it was. **Nothing that is working is ever torn down to try
///    something speculative.**
/// 4. **Only then switch**, in this order: mark the bridge out of contact (unless it never was in
///    contact), forget the old refbox's game, and last of all point the supervisor at the new
///    address. Marking first is what guarantees no request can see "connected" beside the
///    previous refbox's game once the switch has begun.
/// 5. **Remember it**, so a restart comes back to the same refbox -- and say so honestly if that
///    could not be written down.
///
/// The residual race is the one that cannot be removed from any ordering: between the probe
/// succeeding and the supervisor connecting, the candidate could itself vanish. That resolves
/// itself the ordinary way -- the supervisor retries, the connection stays down, and the tables
/// stay blank -- which is the safe outcome, not a wrong one.
///
/// # One switch at a time
///
/// The whole of steps 1-4 runs under [`AppState::switching`], so two switches cannot interleave
/// -- and the address in step 1 is read *inside* that lock, so the second one sees the result of
/// the first. Without it, a double-clicked "Use this refbox" (or two open tabs) sends two requests
/// that both read the old address, both pass the "already set to" check, both probe, and both then
/// mark the bridge out of contact and blank its picture. The second one's `FeedTarget::set` then
/// reports no change -- the address is already what it is setting -- so **no supervisor wake
/// follows its mark**, and the bridge sits disconnected with a blank graphic until the socket
/// happens to drop, which for a refbox with a stopped clock is minutes. A lost update, not a
/// timing bug, but the same blank graphic at the same cost, reached by a double-click that a
/// volunteer under pressure will make.
///
/// Holding an async mutex across the probe's `.await` is the point rather than an oversight: the
/// probe is exactly the slow step another switch must not slip past. Nothing else in the crate
/// takes this lock, so serving tables, rendering the page and running a scan are all unaffected.
pub async fn choose_refbox(state: &AppState, submitted: &str) -> Notice {
    let _switching = state.switching.lock().await;
    let current = state.target.current();

    let candidate = match RefboxAddress::parse(submitted, current.port) {
        Ok(address) => address,
        Err(e) => {
            return Notice::problem(format!(
                "Could not use \"{}\": {e}. Nothing was changed — the bridge is still set to \
                 {current}.",
                submitted.trim()
            ));
        }
    };

    if candidate == current {
        // Not a failure, so not reported as one -- just nothing to do.
        return Notice::done(format!("The bridge is already set to {current}."));
    }

    let found = match discovery::probe(&candidate, discovery::PROBE_TIMEOUT).await {
        Ok(found) => found,
        Err(e) => {
            return Notice::problem(format!(
                "Could not use {candidate}: {e}. Nothing was changed — the bridge is still set \
                 to {current}."
            ));
        }
    };

    // Confirmed a refbox. From here the switch happens, in the order the doc above sets out.
    state.connection.set_disconnected_if_ever_connected();
    state.forget_game();
    state.target.set(candidate.clone());

    let remembered = match &state.settings_path {
        Some(path) => config::remember_refbox_address(path, &candidate.host, candidate.port),
        None => false,
    };

    // "Switched to", not "now reading": the supervisor has been pointed at it and will connect
    // in a moment, and the indicator at the top of the page is what says whether it has. Claiming
    // a connection this function has not seen would be the same overstatement the whole design is
    // built to avoid.
    let mut text = format!(
        "Switched to the refbox at {candidate} — {}.",
        found.label.trim()
    );
    if !remembered {
        text.push_str(
            " This could not be saved, so the bridge will go back to the previous address when \
             it next starts.",
        );
    }
    Notice::done(text)
}

/// Checks the local network for refboxes and remembers what it found, for the status page to
/// list. Reports back in plain English, exactly like [`choose_refbox`], and changes nothing about
/// the running bridge whatever it finds -- a scan only ever looks.
pub async fn run_scan(state: &AppState, network: &str, port: &str) -> Notice {
    // `try_lock`, not `lock`: a second search is refused rather than queued. Queueing would mean a
    // double-clicked button silently sweeping 254 addresses twice, and the operator waiting twice
    // as long to be told the same thing.
    let Ok(_scanning) = state.scanning.try_lock() else {
        return Notice::problem(
            "A search is already running. Wait a few seconds for it to finish.".to_string(),
        );
    };
    let current = state.target.current();

    let network = network.trim();
    let Ok(subnet) = network.parse::<Ipv4Addr>() else {
        return Notice::problem(format!(
            "Could not look for refboxes on \"{network}\": that is not a network address. Type an \
             address on the network to look at, such as 192.168.1.5 — this computer's own \
             address is usually right."
        ));
    };

    let port = match port.trim() {
        "" => current.port,
        text => match text.parse::<u16>() {
            Ok(port) if port != 0 => port,
            _ => {
                return Notice::problem(format!(
                    "Could not look for refboxes: \"{text}\" is not a port number between 1 and \
                     65535 — most refboxes use {}.",
                    config::DEFAULT_REFBOX_PORT
                ));
            }
        },
    };

    let found = discovery::scan(subnet, port).await;
    let notice = match found.len() {
        0 => Notice::problem(format!(
            "No refboxes answered on {}.x port {port}. Some venue networks and firewalls block \
             this check — if that is happening here, type the refbox's address instead.",
            network_prefix(subnet)
        )),
        1 => Notice::done(format!("Found 1 refbox on {}.x.", network_prefix(subnet))),
        n => Notice::done(format!(
            "Found {n} refboxes on {}.x.",
            network_prefix(subnet)
        )),
    };

    *write_lock(&state.last_scan) = Some(ScanOutcome {
        network: format!("{}.x", network_prefix(subnet)),
        port,
        found,
    });
    notice
}

/// `192.168.1.37` -> `192.168.1`, so a scanned network can be named the way an operator reads it
/// (`192.168.1.x`) rather than as an address that was never scanned.
fn network_prefix(subnet: Ipv4Addr) -> String {
    let [a, b, c, _] = subnet.octets();
    format!("{a}.{b}.{c}")
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
/// `status` is [`ConnectionStatus`], not the old timing-derived `state::Contact` this replaced
/// (Task 10) -- see `feed`'s and `status`'s module docs. There is deliberately no `sinceSeconds`
/// (or similar) column keyed off message arrival: the old one measured silence since the last
/// message, which is exactly the quantity Task 10 established must never be treated as
/// meaningful (a stopped clock produces long silence legitimately). `disconnectedForSeconds`
/// below is different in kind, not just in name: it measures time since the *connection*
/// dropped, and it is taken from the SAME `ConnectionStatus` value as `contact` -- both fields of
/// this row always come from one [`ConnectionState::snapshot`] call in the caller, never two
/// separate reads, which is what makes it structurally impossible for this row to ship
/// `contact: "Live"` beside a nonzero `disconnectedForSeconds` (see [`ConnectionStatus`]'s doc for
/// the real bug this closes).
fn status_row(
    display: &Display,
    status: ConnectionStatus,
    keepalive_active: bool,
) -> Vec<BTreeMap<String, String>> {
    let contact_text = match status.connection {
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
    row.insert(
        "disconnectedForSeconds".to_string(),
        status
            .disconnected_for
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default(),
    );
    row.insert("keepaliveActive".to_string(), keepalive_active.to_string());
    vec![row]
}

/// Reads snapshots from the feed supervisor's channel forever, applying each to the shared
/// [`LiveState`] and, on an event or game-number change, (re)pointing [`AppState::directory`] at
/// the right event and waking [`refresh_portal_loop`] so the new game's teams do not wait out a
/// full idle interval to appear. Returns once `snapshots` closes, which only happens if the feed
/// supervisor itself has stopped (it does not stop on its own -- see `feed::Supervisor::run`'s
/// doc).
///
/// # A snapshot is only ever applied to the refbox it came from
///
/// Every message carries the address it was read from ([`FeedMessage`]), and anything not from the
/// refbox currently chosen is **discarded, not applied**. This is the receiving half of the
/// attribution rule: when an operator switches refboxes, a snapshot from the previous one can
/// already be sitting in this channel, or be read in the instant before the supervisor notices the
/// change. Applying it would put a value the newly-connected refbox never sent into the bridge's
/// live picture, to be served with `connected: true` the moment that refbox connects -- one
/// court's game on another court's overlay. The check is on receipt rather than on send because
/// only the receiver knows what "currently chosen" means at the moment of applying.
pub async fn consume_snapshots(
    state: Arc<AppState>,
    mut snapshots: mpsc::UnboundedReceiver<FeedMessage>,
    client: Client,
    portal_url: String,
    refresh_notify: Arc<Notify>,
) {
    while let Some(message) = snapshots.recv().await {
        let chosen = state.target.current();
        if message.from != chosen {
            // See this function's doc. Reported rather than silently dropped: in normal operation
            // this happens at most for the handful of messages in flight across a switch, so a
            // stream of these would mean the supervisor had failed to follow the chosen address --
            // worth being able to see.
            eprintln!(
                "ignoring a snapshot from {} -- the bridge is now reading {chosen}",
                message.from
            );
            continue;
        }
        let snapshot = message.snapshot;

        let now = Instant::now();
        let event_id = snapshot.event_id.clone();
        let game_number = snapshot.game_number().clone();

        write_lock(&state.live).apply(snapshot, now);

        // `Some` only when the feed just reported a *different* known event id -- `None` both
        // when the event id is unknown (`None` on the wire) and when it is unchanged. Read from
        // `state`, not from a local, so that choosing a different refbox can clear it -- see
        // `AppState::forget_game`.
        let new_event = {
            let last_seen = read_lock(&state.last_seen);
            event_id
                .clone()
                .filter(|id| last_seen.event_id.as_ref() != Some(id))
        };
        if let Some(id) = &new_event {
            *write_lock(&state.directory) = Some(Arc::new(Directory::new(
                client.clone(),
                portal_url.clone(),
                id.clone(),
            )));
        }

        {
            let mut last_seen = write_lock(&state.last_seen);
            let game_changed = last_seen.game_number.as_ref() != Some(&game_number);
            if new_event.is_some() || game_changed {
                refresh_notify.notify_one();
            }

            // Only remember a *real* event id. A momentary `None` on the wire (a snapshot arriving
            // before the refbox has attached an event, or some other transient gap) must not be
            // allowed to overwrite the last known real one -- if it did, the very next snapshot
            // carrying that same real id back would look "new" again (since `event_id` would
            // have been cleared to `None`), triggering a needless `Directory` rebuild that throws
            // away every team name and roster already cached for an event nothing has actually
            // left.
            if event_id.is_some() {
                last_seen.event_id = event_id;
            }
            last_seen.game_number = Some(game_number);
        }
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

    /// Tags `snapshot` as having come from the refbox `state` is currently pointed at -- what the
    /// real `feed::Supervisor` does for every snapshot it reads (see `feed::FeedMessage`).
    ///
    /// Every test that feeds the consumer directly goes through this, which makes them all the
    /// positive control for `consume_snapshots`' origin check: a check that discarded everything,
    /// rather than only what came from a refbox the operator has left, would fail all of them.
    fn from_chosen_refbox(state: &AppState, snapshot: GameSnapshot) -> FeedMessage {
        FeedMessage {
            from: state.target.current(),
            snapshot,
        }
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
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

        let state = Arc::new(AppState::new(config::Resolved::default()));
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
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

        let state = Arc::new(AppState::new(config::Resolved::default()));
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

        let state = Arc::new(AppState::new(config::Resolved::default()));
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

        let state = Arc::new(AppState::new(config::Resolved::default()));
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
        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                scores: BlackWhiteBundle { black: 5, white: 2 },
                game_number: "1".to_string(),
                event_id: Some(event),
                ..Default::default()
            },
        ))
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            refresh_notify,
        ));

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                event_id: Some(EventId::from_partial("event-a")),
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
        .expect("channel should accept the first snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let directory_a = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist after the first event id is learned");

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                event_id: Some(EventId::from_partial("event-b")),
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
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
        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                event_id: Some(event.clone()),
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
        .expect("channel should accept the first snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let directory_before = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist after the event id is learned");

        // A snapshot with no event id at all -- a transient gap on the wire, not a real change of
        // event -- followed by the same real event id coming back.
        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                event_id: None,
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
        .expect("channel should accept the blip");
        tokio::time::sleep(Duration::from_millis(50)).await;

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                event_id: Some(event),
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
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
        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                event_id: Some(event.clone()),
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
        .expect("channel should accept the first snapshot");
        // The first snapshot always wakes the loop too (a new event) -- drain that wake so the
        // assertion below is specifically about the game-number change that follows it.
        tokio::time::timeout(Duration::from_secs(1), refresh_notify.notified())
            .await
            .expect("the first snapshot (a new event) should wake the refresh loop");

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                event_id: Some(event),
                game_number: "2".to_string(),
                ..Default::default()
            },
        ))
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
        let state = Arc::new(AppState::new(config::Resolved::default()));
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
    /// carried -- and, since Task 7, that `/status.json` and `GET /` (the operator status page)
    /// agree: still `"Live"`/the live indicator, with no `disconnectedForSeconds`/down-duration
    /// at all. Shared by the two regression guards for "the trap" (spec §4.6, §5.4) below, which
    /// differ only in how long they wait -- see each test's own doc for why both exist. Extending
    /// this one shared helper, rather than adding a second 30-second test, reuses the exact same
    /// real sleep for both surfaces at zero extra wall-clock cost.
    async fn assert_scorebug_survives_silence(silent_for: Duration) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let refbox_addr = listener.local_addr().expect("local_addr");

        // The supervisor reads the address from the shared handle (Task 8), so pointing this
        // state at the test's own listener is what makes it connect there.
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new(refbox_addr.ip().to_string(), refbox_addr.port()),
            ..Default::default()
        }));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());

        let supervisor = tokio::spawn(crate::feed::Supervisor::run(
            state.target_handle(),
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

        // Task 7's new surfaces must agree -- see this helper's own doc. These are the direct
        // regression guards for the NEW risk this task adds (deriving `disconnectedForSeconds` /
        // the page's down-duration from something other than `feed::Connection`); the assertions
        // above already covered `/scorebug`'s pre-existing `connected` flag.
        let status_json = get_json(addr, "/status.json").await;
        let status_row = &status_json[0];
        assert_eq!(
            status_row["contact"].as_str(),
            Some("Live"),
            "/status.json must also report the connection as live after {silent_for:?} of \
             silence with the connection still alive"
        );
        assert_eq!(
            status_row["disconnectedForSeconds"].as_str(),
            Some(""),
            "a live connection must never carry a disconnected-for duration, no matter how long \
             it has been silent -- {silent_for:?} of silence alone must never populate this"
        );

        let page = get_response(addr, "/")
            .await
            .text()
            .await
            .unwrap_or_else(|e| panic!("GET / body should be readable: {e}"));
        assert!(
            page.contains("indicator live"),
            "the operator status page must also show the live indicator after {silent_for:?} \
             of silence with the connection still alive"
        );
        assert!(
            !page.contains("class=\"duration\""),
            "the operator status page must never show a down-duration while still connected, \
             no matter how long the silence"
        );

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

        // The supervisor reads the address from the shared handle (Task 8), so pointing this
        // state at the test's own listener is what makes it connect there.
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new(refbox_addr.ip().to_string(), refbox_addr.port()),
            ..Default::default()
        }));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());

        let supervisor = tokio::spawn(crate::feed::Supervisor::run(
            state.target_handle(),
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

    // ---------------------------------------------------------------- the operator status page
    //
    // `GET /` (status.rs's own module has the pure `render_page` unit tests); these cover the
    // route's wiring -- what `AppState`'s real fields and `feed::ConnectionState` actually
    // produce when driven through real HTTP requests.

    #[tokio::test]
    async fn the_status_page_returns_200_html_before_any_refbox_has_ever_connected() {
        // No `feed::Supervisor` involved at all -- `AppState::new` alone, exactly as the bridge
        // looks the instant it starts (design spec §5.6: "available ... before any refbox is
        // configured, so there is no chicken-and-egg").
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let addr = spawn_test_server(state).await;

        let response = get_response(addr, "/").await;
        assert_eq!(response.status(), 200);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(
            content_type.starts_with("text/html"),
            "GET / should be text/html, got {content_type:?}"
        );

        let body = response
            .text()
            .await
            .unwrap_or_else(|e| panic!("GET / body should be readable: {e}"));
        assert!(body.starts_with("<!doctype html>"));
        assert!(
            body.contains("indicator down"),
            "a bridge that has never connected should show the down indicator, not a live one"
        );
    }

    #[tokio::test]
    async fn status_json_distinguishes_all_three_connection_states_with_a_duration_only_when_disconnected()
     {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let refbox_addr = listener.local_addr().expect("local_addr");

        // The supervisor reads the address from the shared handle (Task 8), so pointing this
        // state at the test's own listener is what makes it connect there.
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new(refbox_addr.ip().to_string(), refbox_addr.port()),
            ..Default::default()
        }));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        // 1. Never connected: no attempt has been made yet.
        let never = get_json(addr, "/status.json").await;
        assert_eq!(never[0]["contact"].as_str(), Some("NeverConnected"));
        assert_eq!(
            never[0]["disconnectedForSeconds"].as_str(),
            Some(""),
            "never-connected has nothing to measure a duration from"
        );

        // 2. Connected: drive a real supervisor against a real loopback socket.
        let (tx, _rx) = mpsc::unbounded_channel();
        let supervisor = tokio::spawn(crate::feed::Supervisor::run(
            state.target_handle(),
            tx,
            state.connection_handle(),
        ));
        let (accepted, _) = listener.accept().await.expect("accept");
        wait_for_connection(&state, Connection::Connected).await;

        let connected = get_json(addr, "/status.json").await;
        assert_eq!(connected[0]["contact"].as_str(), Some("Live"));
        assert_eq!(
            connected[0]["disconnectedForSeconds"].as_str(),
            Some(""),
            "a live connection must never carry a disconnected-for duration"
        );

        // 3. Disconnected: drop the connection. No extra wait beyond the connection state
        // transition itself is needed for the duration to be measurable -- `ConnectionState`
        // records the drop instant in the same update as the state change (Task 7 review,
        // Important 1: this used to require a background poller's catch-up window, which is
        // exactly the two-source race that was fixed).
        drop(accepted);
        wait_for_connection(&state, Connection::Disconnected).await;

        let disconnected = get_json(addr, "/status.json").await;
        assert_eq!(disconnected[0]["contact"].as_str(), Some("Lost"));
        let seconds: u64 = disconnected[0]["disconnectedForSeconds"]
            .as_str()
            .expect("disconnectedForSeconds should be present")
            .parse()
            .unwrap_or_else(|e| {
                panic!(
                    "disconnectedForSeconds should be a plain integer, got {:?}: {e}",
                    disconnected[0]["disconnectedForSeconds"]
                )
            });
        assert!(
            seconds < 5,
            "duration should be small this soon after the transition, got {seconds}s"
        );

        supervisor.abort();
    }

    #[tokio::test]
    async fn the_status_page_and_json_report_keepalive_as_active_by_default() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let json = get_json(addr, "/status.json").await;
        assert_eq!(json[0]["keepaliveActive"].as_str(), Some("true"));

        let page = get_response(addr, "/")
            .await
            .text()
            .await
            .unwrap_or_else(|e| panic!("GET / body should be readable: {e}"));
        assert!(!page.contains("Connection check unavailable"));
    }

    #[tokio::test]
    async fn the_status_page_and_json_report_keepalive_as_unavailable_when_the_supervisor_could_not_enable_it()
     {
        // Drives `feed::ConnectionState`'s own `pub(crate)` setter directly, the same as
        // `mark_connected` above and `feed.rs`'s own keepalive tests -- a genuine OS-level
        // keepalive failure can't be produced portably in a unit test (see
        // `configure_keepalive`'s doc), so this proves the page/JSON correctly surface the flag
        // once `feed::Supervisor::run` (or, here, a test standing in for it) has set it.
        let state = Arc::new(AppState::new(config::Resolved::default()));
        state.connection_handle().set_keepalive_unavailable();
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let json = get_json(addr, "/status.json").await;
        assert_eq!(json[0]["keepaliveActive"].as_str(), Some("false"));

        let page = get_response(addr, "/")
            .await
            .text()
            .await
            .unwrap_or_else(|e| panic!("GET / body should be readable: {e}"));
        assert!(
            page.contains("Connection check unavailable"),
            "the status page should surface the wording an operator would need to see -- got:\n\
             {page}"
        );
        assert!(page.contains("a lost refbox may not be detected"));
    }

    // ------------------------------------------------------ choosing a refbox at runtime (Task 8)
    //
    // The whole point of these is the ORDER of what `choose_refbox` does (see its own doc): prove
    // the candidate first, and only then touch anything that is working. Each test below pins one
    // link of that chain, and the "left alone" ones would go red the moment the order was
    // reversed -- which is the mistake that would take a broadcast off air.

    /// A listener that behaves like a refbox: it replays `snapshot` the instant anything connects
    /// (the real refbox behaviour discovery depends on, `refbox/src/app/update_sender.rs:606-630`)
    /// and then holds the connection open, saying nothing further, exactly as a refbox with a
    /// stopped clock does.
    async fn fake_refbox(snapshot: GameSnapshot) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let line = format!(
            "{}\n",
            serde_json::to_string(&snapshot).expect("GameSnapshot should serialize")
        );

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let line = line.clone();
                tokio::spawn(async move {
                    let _ = socket.write_all(line.as_bytes()).await;
                    let mut sink = Vec::new();
                    let _ = socket.read_to_end(&mut sink).await;
                });
            }
        });

        (addr, handle)
    }

    /// A refbox that answers exactly one connection -- enough for a probe to confirm it is a
    /// refbox -- and then stops listening altogether. Stands in for the refbox that is switched
    /// off, or falls off the Wi-Fi, in the moment between being chosen and being connected to:
    /// the bridge must end up out of contact, never back on the previous refbox's game.
    async fn vanishing_refbox(snapshot: GameSnapshot) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let line = format!(
            "{}\n",
            serde_json::to_string(&snapshot).expect("GameSnapshot should serialize")
        );

        let handle = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(line.as_bytes()).await;
            }
            // Dropping `listener` here is the point: nothing is listening on that port any more.
        });

        (addr, handle)
    }

    fn game(period: GamePeriod, secs: u32, black: u8, white: u8, number: &str) -> GameSnapshot {
        GameSnapshot {
            current_period: period,
            secs_in_period: secs,
            scores: BlackWhiteBundle { black, white },
            game_number: number.to_string(),
            ..Default::default()
        }
    }

    /// A bridge wired up exactly as `main.rs` wires it -- supervisor, snapshot consumer and HTTP
    /// server -- pointed at `refbox_addr`. Returns the state, the bridge's own HTTP address, and
    /// the background tasks to abort at the end of the test.
    async fn bridge_reading(
        refbox_addr: SocketAddr,
    ) -> (Arc<AppState>, SocketAddr, Vec<tokio::task::JoinHandle<()>>) {
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new(refbox_addr.ip().to_string(), refbox_addr.port()),
            ..Default::default()
        }));
        let (tx, rx) = mpsc::unbounded_channel();
        let supervisor = tokio::spawn(crate::feed::Supervisor::run(
            state.target_handle(),
            tx,
            state.connection_handle(),
        ));
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            Arc::new(Notify::new()),
        ));
        let addr = spawn_test_server(Arc::clone(&state)).await;
        (state, addr, vec![supervisor, consumer])
    }

    /// Submits the status page's own manual-entry form, following the redirect back to `GET /` the
    /// way a browser does, and returns the page the operator ends up looking at.
    async fn post_address(addr: SocketAddr, address: &str) -> String {
        reqwest::Client::new()
            .post(format!("http://{addr}/refbox"))
            .form(&[("address", address)])
            .send()
            .await
            .expect("POST /refbox should succeed")
            .text()
            .await
            .expect("the page after POST /refbox should be readable")
    }

    /// The same, for the scan form.
    async fn post_scan_form(addr: SocketAddr, network: &str, port: &str) -> String {
        reqwest::Client::new()
            .post(format!("http://{addr}/scan"))
            .form(&[("network", network), ("port", port)])
            .send()
            .await
            .expect("POST /scan should succeed")
            .text()
            .await
            .expect("the page after POST /scan should be readable")
    }

    #[tokio::test]
    async fn choosing_a_different_refbox_makes_the_tables_serve_that_refbox_s_game() {
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (second_addr, second) = fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;

        wait_for_scores(&state, 2, 1).await;
        let before = get_json(addr, "/scorebug").await;
        assert_eq!(before[0]["blackScore"].as_str(), Some("2"));

        let page = post_address(addr, &second_addr.to_string()).await;
        assert!(
            page.contains("Switched to the refbox at"),
            "the operator should be told plainly what happened, got:\n{page}"
        );

        // The tables now serve the newly chosen refbox's game, not the previous one's.
        wait_for_scores(&state, 9, 3).await;
        let after = get_json(addr, "/scorebug").await;
        assert_eq!(after[0]["connected"].as_str(), Some("true"));
        assert_eq!(after[0]["blackScore"].as_str(), Some("9"));
        assert_eq!(after[0]["whiteScore"].as_str(), Some("3"));
        assert_eq!(after[0]["clockSeconds"].as_str(), Some("44"));

        // And the page reports the address it is actually reading, from the same handle the
        // supervisor connects through -- not a display copy that could disagree with it.
        let page = get_response(addr, "/")
            .await
            .text()
            .await
            .expect("GET / body should be readable");
        assert!(
            page.contains(&second_addr.to_string()),
            "the page should show the newly chosen address, got:\n{page}"
        );

        first.abort();
        second.abort();
        for task in tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn an_unreachable_address_is_reported_and_the_working_connection_is_left_alone() {
        // The single most important test in this task: an operator mistypes an address while a
        // game is on air. The bridge must say so and carry on serving, NOT tear down what works
        // to go looking for what does not.
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let reserved = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let nothing_there = reserved.local_addr().expect("local_addr");
        drop(reserved); // every connect there is refused

        let page = post_address(addr, &nothing_there.to_string()).await;
        assert!(
            page.contains("Could not use") && page.contains("Nothing was changed"),
            "the operator should be told what went wrong and that nothing changed, got:\n{page}"
        );

        let still = get_json(addr, "/scorebug").await;
        assert_eq!(
            still[0]["connected"].as_str(),
            Some("true"),
            "a working connection must survive an address that turned out to be nothing"
        );
        assert_eq!(still[0]["blackScore"].as_str(), Some("2"));
        assert_eq!(
            state.target.current(),
            RefboxAddress::from(first_addr),
            "and the bridge must still be pointed at the refbox it was reading"
        );

        first.abort();
        for task in tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn an_address_that_is_not_a_refbox_is_reported_and_changes_nothing() {
        // Something IS listening on this port -- it just is not a refbox (it never sends a game).
        // Judging by "did the connection succeed" would switch the bridge to it and take the
        // graphic off air.
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let not_a_refbox = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let silent_addr = not_a_refbox.local_addr().expect("local_addr");
        let silent = tokio::spawn(async move {
            let held = not_a_refbox.accept().await;
            std::future::pending::<()>().await;
            drop(held);
        });

        let page = post_address(addr, &silent_addr.to_string()).await;
        assert!(
            page.contains("it did not send a game"),
            "the operator should be told that something is there but is not a refbox, got:\n{page}"
        );
        assert_eq!(
            state.target.current(),
            RefboxAddress::from(first_addr),
            "nothing should have changed"
        );
        let still = get_json(addr, "/scorebug").await;
        assert_eq!(still[0]["connected"].as_str(), Some("true"));
        assert_eq!(still[0]["blackScore"].as_str(), Some("2"));

        silent.abort();
        first.abort();
        for task in tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn an_address_that_cannot_be_read_is_reported_and_changes_nothing() {
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let page = post_address(addr, "192.168.1.50:eight thousand").await;
        assert!(
            page.contains("is not a port number"),
            "a mistyped port should come back as a sentence, got:\n{page}"
        );
        assert_eq!(state.target.current(), RefboxAddress::from(first_addr));

        // An empty submission is the other half of the same case -- a button pressed with nothing
        // typed must not be read as "connect to nowhere".
        let page = post_address(addr, "   ").await;
        assert!(
            page.contains("type the refbox&#39;s address"),
            "an empty submission should say what to type, got:\n{page}"
        );
        assert_eq!(state.target.current(), RefboxAddress::from(first_addr));

        let still = get_json(addr, "/scorebug").await;
        assert_eq!(still[0]["connected"].as_str(), Some("true"));
        assert_eq!(still[0]["blackScore"].as_str(), Some("2"));

        first.abort();
        for task in tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn the_previous_refbox_s_game_is_never_served_while_the_new_one_is_being_reached() {
        // The candidate answers the probe (so the switch is allowed to happen) and then goes away
        // before the supervisor can connect. The bridge must be out of contact and showing
        // nothing -- above all it must not go on serving, or displaying, the game belonging to the
        // refbox the operator just left.
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let (gone_addr, gone) =
            vanishing_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;
        let page = post_address(addr, &gone_addr.to_string()).await;
        assert!(
            page.contains("Switched to the refbox at"),
            "the candidate answered the probe, so the switch should have been allowed:\n{page}"
        );

        let scorebug = get_json(addr, "/scorebug").await;
        assert_ne!(
            scorebug[0]["blackScore"].as_str(),
            Some("2"),
            "the previous refbox's score must not still be served after switching away from it"
        );
        assert_ne!(scorebug[0]["clockSeconds"].as_str(), Some("613"));

        // `/status.json` and the status page name the game unconditionally -- they are not blanked
        // by the connection being down the way the tables are -- so this is where a leftover would
        // sit visibly for as long as the new refbox stayed unreachable.
        let status = get_json(addr, "/status.json").await;
        assert_ne!(
            status[0]["gameNumber"].as_str(),
            Some("14"),
            "the previous refbox's game number must not still be reported"
        );
        let page = get_response(addr, "/")
            .await
            .text()
            .await
            .expect("GET / body should be readable");
        assert!(
            page.contains("<tr><th>Game</th><td>(none known yet)</td></tr>"),
            "the status page must not still be naming the previous refbox's game, got:\n{page}"
        );

        gone.abort();
        first.abort();
        for task in tasks {
            task.abort();
        }
    }

    /// A refbox that waits `before_replying` after accepting, then replays `snapshot`. Used to
    /// hold two probes in flight at the same time on purpose: a probe of a loopback refbox is
    /// otherwise over in well under a millisecond, and a race that only sometimes overlaps is a
    /// test that only sometimes tests anything.
    async fn slow_refbox(
        snapshot: GameSnapshot,
        before_replying: Duration,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let line = format!(
            "{}\n",
            serde_json::to_string(&snapshot).expect("GameSnapshot should serialize")
        );

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let line = line.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(before_replying).await;
                    let _ = socket.write_all(line.as_bytes()).await;
                    let mut sink = Vec::new();
                    let _ = socket.read_to_end(&mut sink).await;
                });
            }
        });

        (addr, handle)
    }

    #[tokio::test]
    async fn two_switches_to_the_same_refbox_at_once_produce_exactly_one_switch() {
        // A double-clicked "Use this refbox", or the same page open in two tabs. Both requests
        // arrive while the bridge is still reading the previous refbox, so without serialisation
        // both read the old address, both pass the "already set to" check, both probe, and both
        // then mark the bridge out of contact and blank its picture. The second one's
        // `FeedTarget::set` reports no change -- the address is already what it is setting -- so
        // NO supervisor wake follows its mark, and the bridge sits disconnected behind a blank
        // graphic until the socket happens to drop. For a refbox with a stopped clock that is
        // minutes of dead air, caused by a double-click.
        //
        // The candidate deliberately takes 150ms to answer its probe, so both requests are
        // certainly in flight together: without the lock this fails every time, not sometimes.
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let settings_file = std::env::temp_dir().join(format!(
            "overlay-bridge-test-race-{}-{:?}.toml",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&settings_file);
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::from(first_addr),
            settings_path: Some(settings_file.clone()),
            ..Default::default()
        }));
        mark_connected(&state);

        let (candidate, slow) = slow_refbox(
            game(GamePeriod::SecondHalf, 44, 9, 3, "15"),
            Duration::from_millis(150),
        )
        .await;
        let submitted = candidate.to_string();

        let (one, two) = tokio::join!(
            choose_refbox(&state, &submitted),
            choose_refbox(&state, &submitted)
        );

        let switched = [&one, &two]
            .iter()
            .filter(|notice| notice.text.contains("Switched to the refbox at"))
            .count();
        let already = [&one, &two]
            .iter()
            .filter(|notice| notice.text.contains("already set to"))
            .count();
        assert_eq!(
            (switched, already),
            (1, 1),
            "two requests for the same refbox must result in exactly one switch and one \
             \"already set to\" -- two switches means both ran the mark-and-blank sequence, and \
             the second one's mark is never followed by a reconnect.\nfirst: {}\nsecond: {}",
            one.text,
            two.text
        );

        assert_eq!(state.target.current(), RefboxAddress::from(candidate));

        // Minor 4 falls out with it: with the sequence serialised, what was written down cannot
        // disagree with what is running.
        let stored: config::Settings =
            confy::load_path(&settings_file).expect("the settings file should be readable");
        let _ = std::fs::remove_file(&settings_file);
        assert_eq!(stored.refbox_port, Some(candidate.port()));
        assert_eq!(
            RefboxAddress::new(
                stored.refbox_host.unwrap_or_default(),
                stored.refbox_port.unwrap_or_default()
            ),
            state.target.current(),
            "the remembered address must be the one actually in use"
        );

        slow.abort();
        first.abort();
    }

    #[tokio::test]
    async fn a_snapshot_from_the_refbox_just_left_is_never_applied_to_the_one_now_connected() {
        // The invariant, stated as a test: a value served while the bridge is connected to refbox
        // B can never have originated from refbox A.
        //
        // Marking out of contact and forgetting the game (see `choose_refbox`) close the long
        // window -- the seconds or minutes a newly chosen refbox may take to answer. They cannot
        // close this one: a snapshot from A can already be sitting in the channel, or be read in
        // the instant before the supervisor notices the change, and would then be applied *after*
        // the switch. It would be served, with `connected: true`, as though B had sent it. The
        // duration is milliseconds; the wrongness is not the duration but the attribution -- at a
        // two-court event that is one court's game on the other court's overlay.
        //
        // Nothing here is timed. The channel is closed after the leftover is queued, and the
        // consumer is awaited to completion, so "the leftover has been handled" is a fact rather
        // than a wait.
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new(first_addr.ip().to_string(), first_addr.port()),
            ..Default::default()
        }));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        // No supervisor: this test drives the channel itself, which is the only way to place a
        // message from the previous refbox *after* the switch with certainty.
        let (tx, rx) = mpsc::unbounded_channel();
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            Arc::new(Notify::new()),
        ));

        tx.send(from_chosen_refbox(
            &state,
            game(GamePeriod::FirstHalf, 613, 2, 1, "14"),
        ))
        .expect("channel should accept the first refbox's snapshot");
        wait_for_scores(&state, 2, 1).await;

        // Switch to a second, genuine refbox through the real path.
        let (second_addr, second) = fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;
        let notice = choose_refbox(&state, &second_addr.to_string()).await;
        assert!(notice.done, "{}", notice.text);

        // The leftover: read from the FIRST refbox, delivered after the switch.
        tx.send(FeedMessage {
            from: RefboxAddress::from(first_addr),
            snapshot: game(GamePeriod::FirstHalf, 600, 5, 5, "14"),
        })
        .expect("channel should accept the leftover");
        drop(tx);
        consumer
            .await
            .expect("the consumer should finish once the channel closes");

        // And now the second refbox connects, which is the moment a leftover would become
        // visible: real values are served again, so anything left in the live picture is served
        // as though this refbox had sent it.
        mark_connected(&state);

        let held = current_display(&state).snapshot;
        assert_eq!(
            held,
            GameSnapshot::default(),
            "nothing of the refbox just left may survive the switch, and a snapshot from it \
             arriving afterwards must be discarded rather than applied"
        );

        let scorebug = get_json(addr, "/scorebug").await;
        assert_eq!(scorebug[0]["connected"].as_str(), Some("true"));
        assert_ne!(
            scorebug[0]["blackScore"].as_str(),
            Some("5"),
            "a score the connected refbox never sent must never be served as its own"
        );
        let status = get_json(addr, "/status.json").await;
        assert_ne!(status[0]["gameNumber"].as_str(), Some("14"));

        first.abort();
        second.abort();
    }

    #[tokio::test]
    async fn changing_address_while_already_disconnected_keeps_the_down_for_time_counting() {
        // The coverage Task 7's re-review deferred to here: `set_disconnected`'s guard against
        // restarting the drop clock, reached through the real address-change path rather than by
        // calling the setter twice by hand. No supervisor runs in this test, deliberately -- it
        // would reconnect and legitimately clear the duration, and what is under test is the
        // moment before that.
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new("127.0.0.1".to_string(), 9),
            ..Default::default()
        }));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        state.connection_handle().set_connected();
        state.connection_handle().set_disconnected();
        tokio::time::sleep(Duration::from_millis(1300)).await;

        let (candidate, refbox) = fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;
        let notice = choose_refbox(&state, &candidate.to_string()).await;
        assert!(
            notice.done,
            "the candidate is a real refbox: {}",
            notice.text
        );

        let status = get_json(addr, "/status.json").await;
        assert_eq!(status[0]["contact"].as_str(), Some("Lost"));
        let down_for: u64 = status[0]["disconnectedForSeconds"]
            .as_str()
            .unwrap_or_default()
            .parse()
            .expect("disconnectedForSeconds should be a whole number of seconds");
        assert!(
            down_for >= 1,
            "the down-for time must keep counting from the original drop, not restart at zero \
             because the operator chose another refbox -- got {down_for}s"
        );

        refbox.abort();
    }

    #[tokio::test]
    async fn the_chosen_refbox_is_remembered_so_the_next_run_comes_back_to_it() {
        let settings = std::env::temp_dir().join(format!(
            "overlay-bridge-test-chosen-{}-{:?}.toml",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&settings);

        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new("127.0.0.1", 9),
            settings_path: Some(settings.clone()),
            ..Default::default()
        }));
        let (candidate, refbox) = fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;

        let notice = choose_refbox(&state, &candidate.to_string()).await;
        assert!(notice.done, "{}", notice.text);
        assert!(
            !notice.text.contains("could not be saved"),
            "a writable settings file should not report a save failure: {}",
            notice.text
        );

        let stored: config::Settings =
            confy::load_path(&settings).expect("the settings file should be readable");
        let _ = std::fs::remove_file(&settings);
        assert_eq!(stored.refbox_host.as_deref(), Some("127.0.0.1"));
        assert_eq!(stored.refbox_port, Some(candidate.port()));

        refbox.abort();
    }

    #[tokio::test]
    async fn choosing_the_refbox_already_in_use_says_so_and_does_nothing() {
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let page = post_address(addr, &first_addr.to_string()).await;
        assert!(
            page.contains("already set to"),
            "re-submitting the current address should say so, got:\n{page}"
        );
        let still = get_json(addr, "/scorebug").await;
        assert_eq!(
            still[0]["connected"].as_str(),
            Some("true"),
            "and must not have dropped the connection to reconnect to the same place"
        );
        assert_eq!(still[0]["blackScore"].as_str(), Some("2"));

        first.abort();
        for task in tasks {
            task.abort();
        }
    }

    // ------------------------------------------------------------- from settings to the feed

    #[tokio::test]
    async fn the_bridge_reads_the_refbox_its_settings_name() {
        // The whole chain, in one test: resolved settings -> `AppState`'s feed target -> the
        // supervisor -> a served table. This is the wiring that used to live in `main.rs`, where
        // nothing could reach it and deleting one line silently pointed the bridge at
        // 127.0.0.1:8000 no matter what the operator configured (Task 8 review, Important 3).
        //
        // Deliberately end-to-end rather than a field-by-field check of `AppState`: the point is
        // not that each setting is copied somewhere, it is that the refbox actually read is the
        // one the settings name. `config::Resolved` having no optional fields is what makes the
        // copying itself impossible to forget.
        let (refbox_addr, refbox) = fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;

        let bridge = start(
            config::Resolved {
                refbox: RefboxAddress::from(refbox_addr),
                ..Default::default()
            },
            "http://portal.invalid".to_string(),
        );

        wait_for_scores(&bridge.state, 9, 3).await;
        let addr = spawn_test_server(Arc::clone(&bridge.state)).await;
        let body = get_json(addr, "/scorebug").await;
        assert_eq!(body[0]["connected"].as_str(), Some("true"));
        assert_eq!(body[0]["blackScore"].as_str(), Some("9"));
        assert_eq!(body[0]["clockSeconds"].as_str(), Some("44"));

        refbox.abort();
    }

    // ------------------------------------------------- one refbox's names on another's game

    /// A Portal that answers every request with the same body, for as long as the test needs it.
    /// [`serve_once`] answers exactly one connection, which is not enough when a schedule is
    /// fetched more than once.
    async fn serve_always(listener: TcpListener, body: String) {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            drain_request(&mut socket).await;
            write_response(&mut socket, "200 OK", &body).await;
        }
    }

    #[tokio::test]
    async fn team_names_from_the_event_just_left_are_not_shown_on_the_new_refbox_s_game() {
        // The other half of the attribution rule. Scores and the clock were closed by tagging
        // each snapshot with its origin; names come from somewhere else entirely -- the Portal
        // directory, built for whichever *event* the previous refbox reported, and consulted by
        // game number alone. Tournament game numbers repeat across events, and a refbox that
        // reports no event id resolves against whatever directory is lying around. So without
        // clearing it, switching courts shows the right game number with the wrong event's teams.
        let schedule = serde_json::json!({
            "games": [{
                "number": "10",
                "startsOn": "2026-08-01T09:00:00+10:00",
                "court": "1",
                "dark": {"assignment": {"teamId": "teams/1-A"}},
                "light": {"assignment": {"teamId": "teams/2-A"}},
            }],
            "teams": {
                "teams/1-A": {"name": "event a dark"},
                "teams/2-A": {"name": "event a light"},
            },
        })
        .to_string();
        let portal = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let portal_addr = portal.local_addr().expect("local_addr");
        let portal_task = tokio::spawn(serve_always(portal, schedule));

        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "10")).await;
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::from(first_addr),
            ..Default::default()
        }));
        mark_connected(&state);

        let (tx, rx) = mpsc::unbounded_channel();
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            format!("http://{portal_addr}"),
            Arc::new(Notify::new()),
        ));

        // The first refbox: event A, game 10. Its teams are resolved from the Portal.
        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                secs_in_period: 613,
                scores: BlackWhiteBundle { black: 2, white: 1 },
                game_number: "10".to_string(),
                event_id: Some(EventId::from_partial("event-a")),
                ..Default::default()
            },
        ))
        .expect("channel should accept the first refbox's snapshot");
        wait_for_scores(&state, 2, 1).await;
        refresh_once(&state).await;

        let addr = spawn_test_server(Arc::clone(&state)).await;
        let before = get_json(addr, "/scorebug").await;
        assert_eq!(
            before[0]["blackTeam"].as_str(),
            Some("EVENT A DARK"),
            "test setup: the first refbox's game should resolve to its own event's teams"
        );

        // Now switch to a refbox on another court -- which happens to number its games the same
        // way, and reports no event id of its own.
        let (second_addr, second) = fake_refbox(game(GamePeriod::FirstHalf, 500, 0, 0, "10")).await;
        let notice = choose_refbox(&state, &second_addr.to_string()).await;
        assert!(notice.done, "{}", notice.text);

        tx.send(FeedMessage {
            from: RefboxAddress::from(second_addr),
            snapshot: GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                secs_in_period: 500,
                scores: BlackWhiteBundle { black: 4, white: 4 },
                game_number: "10".to_string(),
                event_id: None,
                ..Default::default()
            },
        })
        .expect("channel should accept the second refbox's snapshot");
        wait_for_scores(&state, 4, 4).await;
        mark_connected(&state);

        let after = get_json(addr, "/scorebug").await;
        assert_eq!(
            after[0]["blackTeam"].as_str(),
            Some(""),
            "the event just left must not supply team names for a game on the refbox just \
             chosen -- better no name at all than the wrong team's"
        );
        assert_eq!(after[0]["whiteTeam"].as_str(), Some(""));
        assert_eq!(after[0]["blackScore"].as_str(), Some("4"));

        consumer.abort();
        portal_task.abort();
        second.abort();
        first.abort();
    }

    #[tokio::test]
    async fn returning_to_an_event_after_a_switch_rebuilds_its_directory_rather_than_reusing_it() {
        // The same clearing, from the other side: `last_seen` is what decides whether an arriving
        // event id counts as new. Left set across a switch, an event the bridge has genuinely left
        // and come back to would look unchanged, and the directory would never be rebuilt -- so
        // the clearing above would be undone by the very next snapshot.
        let (first_addr, first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "10")).await;
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::from(first_addr),
            ..Default::default()
        }));
        let (tx, rx) = mpsc::unbounded_channel();
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            "http://portal.invalid".to_string(),
            Arc::new(Notify::new()),
        ));

        let on_event = |scores: (u8, u8)| GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            scores: BlackWhiteBundle {
                black: scores.0,
                white: scores.1,
            },
            game_number: "10".to_string(),
            event_id: Some(EventId::from_partial("event-a")),
            ..Default::default()
        };

        tx.send(from_chosen_refbox(&state, on_event((2, 1))))
            .expect("channel should accept the first snapshot");
        wait_for_scores(&state, 2, 1).await;
        let first_directory = read_lock(&state.directory)
            .clone()
            .expect("a snapshot carrying an event id should have built a directory");

        let (second_addr, second) = fake_refbox(game(GamePeriod::FirstHalf, 500, 0, 0, "10")).await;
        let notice = choose_refbox(&state, &second_addr.to_string()).await;
        assert!(notice.done, "{}", notice.text);
        assert!(
            read_lock(&state.directory).is_none(),
            "the previous refbox's directory must be gone the moment the switch happens"
        );

        tx.send(FeedMessage {
            from: RefboxAddress::from(second_addr),
            snapshot: on_event((7, 7)),
        })
        .expect("channel should accept the second refbox's snapshot");
        wait_for_scores(&state, 7, 7).await;

        let second_directory = read_lock(&state.directory)
            .clone()
            .expect("the same event arriving again should have built a directory");
        assert!(
            !Arc::ptr_eq(&first_directory, &second_directory),
            "the same event id arriving after a switch must rebuild the directory, not be \
             treated as unchanged"
        );

        consumer.abort();
        second.abort();
        first.abort();
    }

    #[tokio::test]
    async fn a_second_network_search_while_one_is_running_is_refused_rather_than_queued() {
        // Same human behaviour as the double-clicked "Use this refbox": a volunteer who does not
        // see anything happen presses the button again. Two sweeps of 254 addresses would then
        // run, and the second answer would arrive twice as late for no benefit.
        let reserved = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let port = reserved.local_addr().expect("local_addr").port();
        drop(reserved);

        let state = Arc::new(AppState::new(config::Resolved::default()));
        let port = port.to_string();
        let (one, two) = tokio::join!(
            run_scan(&state, "127.0.0.1", &port),
            run_scan(&state, "127.0.0.1", &port)
        );

        let refused = [&one, &two]
            .iter()
            .filter(|notice| notice.text.contains("A search is already running"))
            .count();
        assert_eq!(
            refused, 1,
            "exactly one of two overlapping searches should be refused.\nfirst: {}\nsecond: {}",
            one.text, two.text
        );
    }

    // -------------------------------------------------------- searching the network (Task 8)

    #[tokio::test]
    async fn a_search_lists_what_it_found_with_a_button_that_reads_it() {
        let (refbox_addr, refbox) =
            fake_refbox(game(GamePeriod::SecondHalf, 227, 2, 1, "14")).await;
        let state = Arc::new(AppState::new(config::Resolved {
            refbox: RefboxAddress::new("127.0.0.1".to_string(), 9),
            ..Default::default()
        }));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let page = post_scan_form(addr, "127.0.0.1", &refbox_addr.port().to_string()).await;

        assert!(
            page.contains("Found 1 refbox"),
            "the search should report what it found, got:\n{page}"
        );
        assert!(
            page.contains(&refbox_addr.to_string()),
            "the refbox's address should be listed, got:\n{page}"
        );
        assert!(
            page.contains("Game 14 · Second Half · 3:47 · 2–1"),
            "the label is what the operator actually picks by, got:\n{page}"
        );
        assert!(
            page.contains(&format!(
                "<input type=\"hidden\" name=\"address\" value=\"{refbox_addr}\">"
            )),
            "each result needs a button that reads that refbox, got:\n{page}"
        );

        // And picking it is the same action as typing it: the same route, the same outcome.
        let page = post_address(addr, &refbox_addr.to_string()).await;
        assert!(page.contains("Switched to the refbox at"), "{page}");
        assert_eq!(state.target.current(), RefboxAddress::from(refbox_addr));

        refbox.abort();
    }

    #[tokio::test]
    async fn a_search_that_finds_nothing_says_so_and_suggests_typing_the_address() {
        let reserved = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let empty_port = reserved.local_addr().expect("local_addr").port();
        drop(reserved);

        let state = Arc::new(AppState::new(config::Resolved::default()));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let page = post_scan_form(addr, "127.0.0.1", &empty_port.to_string()).await;

        assert!(
            page.contains("No refboxes answered"),
            "an empty result must be said in words, got:\n{page}"
        );
        assert!(
            page.contains("type the refbox&#39;s address instead"),
            "and it must point at the way that always works, got:\n{page}"
        );
    }

    #[tokio::test]
    async fn a_search_of_something_that_is_not_a_network_is_reported() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let page = post_scan_form(addr, "my network", "8000").await;

        assert!(
            page.contains("that is not a network address"),
            "got:\n{page}"
        );
        assert!(
            !page.contains("Found"),
            "nothing was searched, so nothing should be reported as found:\n{page}"
        );
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

    // ------------------------------------------------- who is allowed to change what is on air

    /// Submits a form to `path` the way a page belonging to *some other website* would. A browser
    /// stamps `Sec-Fetch-Site: cross-site` on that request itself; the value cannot be set by the
    /// page making it.
    async fn post_from_another_site(
        addr: SocketAddr,
        path: &str,
        form: &[(&str, &str)],
    ) -> reqwest::Response {
        post_with_site(addr, path, form, "cross-site").await
    }

    /// The same, for the bridge's own status page submitting its own form -- what every real
    /// operator action looks like.
    async fn post_from_the_status_page(
        addr: SocketAddr,
        path: &str,
        form: &[(&str, &str)],
    ) -> reqwest::Response {
        post_with_site(addr, path, form, "same-origin").await
    }

    async fn post_with_site(
        addr: SocketAddr,
        path: &str,
        form: &[(&str, &str)],
        site: &str,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("http://{addr}{path}"))
            .header("Sec-Fetch-Site", site)
            .form(form)
            .send()
            .await
            .unwrap_or_else(|e| panic!("POST {path} should get a reply: {e}"))
    }

    #[tokio::test]
    async fn a_page_on_another_site_cannot_switch_which_refbox_is_on_air() {
        // The bridge has no password and binds every interface, so this is the whole defence: any
        // web page the operator happens to open on the streaming PC could otherwise post this
        // form and take the graphic off air mid-game.
        let (first_addr, _first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (second_addr, _second) =
            fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let response =
            post_from_another_site(addr, "/refbox", &[("address", &second_addr.to_string())]).await;

        assert_eq!(
            response.status().as_u16(),
            403,
            "a cross-site form post must be refused outright"
        );
        assert_eq!(
            state.target.current().port,
            first_addr.port(),
            "the bridge must still be reading the refbox it was reading before"
        );
        let still = get_json(addr, "/scorebug").await;
        assert_eq!(
            still[0]["blackScore"].as_str(),
            Some("2"),
            "and still serving that refbox's game"
        );

        for task in tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn the_status_page_s_own_refbox_form_still_switches_refboxes() {
        // The other half of the guard: the operator's own click must be completely unaffected.
        let (first_addr, _first) = fake_refbox(game(GamePeriod::FirstHalf, 613, 2, 1, "14")).await;
        let (second_addr, _second) =
            fake_refbox(game(GamePeriod::SecondHalf, 44, 9, 3, "15")).await;
        let (state, addr, tasks) = bridge_reading(first_addr).await;
        wait_for_scores(&state, 2, 1).await;

        let response =
            post_from_the_status_page(addr, "/refbox", &[("address", &second_addr.to_string())])
                .await;

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            state.target.current().port,
            second_addr.port(),
            "the operator's own submission must switch the refbox as it always did"
        );

        for task in tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn a_page_on_another_site_cannot_start_a_network_scan() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let response = post_from_another_site(
            addr,
            "/scan",
            &[("network", "not an address"), ("port", "")],
        )
        .await;

        assert_eq!(response.status().as_u16(), 403);
        assert!(
            read_lock(&state.notice).is_none(),
            "a refused request must not even reach the handler that writes the page's notice"
        );
    }

    #[tokio::test]
    async fn the_status_page_s_own_scan_form_still_runs() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let addr = spawn_test_server(Arc::clone(&state)).await;

        let response = post_from_the_status_page(
            addr,
            "/scan",
            &[("network", "not an address"), ("port", "")],
        )
        .await;

        assert_eq!(response.status().as_u16(), 200);
        let page = response.text().await.expect("the page should be readable");
        assert!(
            page.contains("that is not a network address"),
            "the operator's own submission must reach the handler and be answered by it, got:\n\
             {page}"
        );
    }
}
