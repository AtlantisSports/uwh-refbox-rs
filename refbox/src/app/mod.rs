use self::infraction::InfractionDetails;
use super::{APP_NAME, fl};
use crate::panic_text::panic_reason;
use crate::{
    beep_test::{cadence::TournamentManager as BeepTestManager, snapshot::BeepTestSnapshot},
    config::{Config, CustomSite, GameSource, Mode, RemoteSource},
    penalty_editor::*,
    portal_manager::{
        ItemId, PortalEvent, PortalManager, SelectedEventId, SharedAccessKeys, UwhPortalIo,
    },
    sound_controller::*,
    tournament_manager::{penalty::*, *},
};
use futures_lite::Stream;
use iced::{
    Element, Length, Subscription, Task, Theme,
    application::Appearance,
    event,
    keyboard::{self, Key, key::Named},
    mouse,
    widget::column,
    window,
};
use log::*;
use std::{
    borrow::Cow,
    cmp::min,
    collections::{BTreeMap, BTreeSet},
    panic::{AssertUnwindSafe, catch_unwind},
    process::Child,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant, sleep, sleep_until, timeout_at},
};
use tokio_serial::SerialPortBuilder;
use uwh_common::{
    bundles::*,
    color::Color,
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{GamePeriod, GameSnapshot, Infraction, TimeoutSnapshot},
    uwhportal::{
        PortalTokenResponse, RosterPlayer, UwhPortalClient, check_access_key,
        schedule::{DateRange, Event, EventId, GameNumber, Schedule, TeamId},
    },
};

mod view_data;
use view_data::ViewData;

mod view_builders;
use view_builders::{
    beep_test::build_beep_test_page,
    shared_elements::{crosses_portal, portal_name_for_mode},
    *,
};

mod message;
use message::*;

pub mod theme;
use theme::*;

pub mod update_sender;
use update_sender::*;

pub(crate) mod languages;
use languages::*;

mod power_control;

mod custom_site;
use custom_site::SiteAddress;

mod event_store;
use event_store::EventStore;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the operator must hold a used-up team timeout button to revive
/// (give back) one team timeout. Long enough to confirm the hold was intentional.
const TIMEOUT_REVIVE_HOLD_DURATION: Duration = Duration::from_secs(3);

/// Which phase an in-progress timeout-revive long-press is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevivePhase {
    /// Finger down on a used-up button, counting down to the 3s revive.
    Reviving,
    /// Revived; finger still down. Stays here until release, which confirms the restore.
    Restored,
}

/// An in-progress timeout-revive long-press.
struct ReviveHold {
    color: Color,
    phase: RevivePhase,
    /// Token of the async timer this hold is currently waiting on; a timer whose
    /// token no longer matches the live hold is stale and ignored.
    token: u64,
}

/// Set to `true` by the in-app restart paths (`RestartAndApply` confirmation
/// for a Mode change, `LanguageSelectComplete` with a font-family change).
/// After the iced runtime exits gracefully — closing all windows — `main()`
/// checks this flag and spawns a fresh copy of the executable. This pattern
/// avoids the brief overlap of old + new windows that `std::process::exit(0)`
/// would otherwise produce.
pub static RESTART_PENDING: AtomicBool = AtomicBool::new(false);

/// Parse the version out of a `refbox-v<X.Y.Z>.bak` backup filename.
fn backup_version_from_filename(p: &std::path::Path) -> Option<crate::updater::version::Version> {
    let name = p.file_name()?.to_str()?;
    let v = name.strip_prefix("refbox-v")?.strip_suffix(".bak")?;
    crate::updater::version::Version::parse(v)
}

/// Map a low-level updater error to the coarse, operator-facing UI error.
fn updater_err_to_ui(e: crate::updater::UpdateError) -> crate::app::message::UpdateUiError {
    use crate::app::message::UpdateUiError as E;
    use crate::updater::UpdateError as U;
    match e {
        U::Network | U::NotJson => E::NoInternet,
        U::RateLimited => E::RateLimited,
        U::NoSpace => E::NoSpace,
        U::NotWritable => E::NotWritable,
        U::AssetMissing | U::BadVersion | U::Checksum | U::Io(_) => E::BadDownload,
    }
}

/// Map a filesystem error from the swap/revert step to the UI error.
fn updater_io_to_ui(e: &std::io::Error) -> crate::app::message::UpdateUiError {
    use crate::app::message::UpdateUiError as E;
    if e.raw_os_error() == Some(28) {
        E::NoSpace
    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
        E::NotWritable
    } else {
        E::BadDownload
    }
}

pub struct RefBoxApp {
    tm: SharedGame,
    /// Cadence engine for BeepTest mode. `Some(_)` only when
    /// `config.mode == Mode::BeepTest`; `None` in Hockey/Rugby modes.
    /// Driven by the `BeepTestTick` subscription, not by the game-clock
    /// `time_updater` stream.
    beep_test_tm: Option<BeepTestManager>,
    /// Most-recent BeepTest snapshot. Held alongside `snapshot` so the
    /// tick handler can compare before/after for sound-trigger decisions
    /// (mirrors `maybe_play_sound`'s use of `self.snapshot`).
    beep_test_snapshot: BeepTestSnapshot,
    config: Config,
    edited_settings: Option<EditableSettings>,
    page_entry_snapshot: Option<PageEntrySnapshot>,
    snapshot: GameSnapshot,
    pen_edit: ListEditor<Penalty, Color>,
    warn_edit: ListEditor<InfractionDetails, Color>,
    foul_edit: ListEditor<InfractionDetails, Option<Color>>,
    app_state: AppState,
    last_app_state: AppState,
    // The game/timeout clock times captured when the time-edit screen was opened,
    // used to gray out the Apply button when no change has been made.
    time_edit_old: (Duration, Option<Duration>),
    last_message: Message,
    update_sender: UpdateSender,
    uwhportal_client: Option<Arc<Mutex<UwhPortalClient>>>,
    /// Shared handle the background portal task reads to learn the
    /// currently-selected event for its periodic `verify_token` probe.
    /// Kept in lockstep with `current_event_id` via
    /// `set_current_event_id` — every write to `current_event_id` must
    /// go through that helper so the background task sees the latest
    /// selection.
    portal_event_id: SelectedEventId,
    /// Published to the background uploader so it can resolve a key per queued item.
    portal_access_keys: SharedAccessKeys,
    /// Where `uwhportal_client` actually points, kept in step with every
    /// rebuild. Read to decide whether an Apply would repoint the refbox at a
    /// different site (and so needs the clock and queue guards).
    current_site: SiteTarget,
    /// Bumped every time the client is actually pointed at a different site.
    /// Site-scoped requests carry the value current when they were issued, and
    /// their handlers drop a reply whose stamp no longer matches — see
    /// [`reply_is_current`].
    site_generation: u64,
    /// `--allow-http` inverted, as passed at launch. Governs the built-in
    /// portal only — a custom site derives TLS from the scheme that was typed.
    require_https: bool,
    source: GameSource,
    /// Event data for both remote sources, held apart so neither can be read
    /// as the other. See `event_store::EventStore`.
    events: EventStore,
    schedule: Option<Schedule>,
    /// The running game's copy of both teams' cap numbers, taken at kickoff so
    /// a mid-game REFRESH cannot move the grid under the operator's hand. Empty
    /// vectors mean "no usable roster" and the number pad is shown.
    game_rosters: BlackWhiteBundle<Vec<u8>>,
    /// Cap numbers for every team the portal has told us about, keyed by portal
    /// team id. Session-only: never written to disk, so a restart with no
    /// network falls back to the number pad until a fetch succeeds.
    team_rosters: BTreeMap<TeamId, Vec<u8>>,
    current_event_id: Option<EventId>,
    current_court: Option<String>,
    /// One-shot: the game number to re-select once the schedule arrives during
    /// a startup link restore; cleared on first use. `None` in normal operation.
    pending_restore_game: Option<GameNumber>,
    /// The game most recently played to a recorded result on this court: the
    /// anchor the schedule search starts from. Advanced only in `handle_game_end`,
    /// and only when the result that was recorded belongs to the game that just
    /// ended — an abandoned or interrupted game leaves it alone. Cleared whenever
    /// it stops being valid for the live event/court (portal switched off, or the
    /// Apply paths that repoint `current_event_id`/`current_court`): a carried-over
    /// anchor points at a real but wrong game and looks entirely plausible.
    last_played: Option<GameNumber>,
    /// The anchor's scheduled start, persisted alongside `last_played` so the
    /// search still works when the anchor game itself has been removed from the
    /// schedule.
    last_played_start: Option<time::OffsetDateTime>,
    /// One-shot: the event whose schedule to fetch once the event list lands
    /// during a startup link restore. Deferred (rather than fetched at startup)
    /// so the schedule arrives after the portal's event list is populated in
    /// `self.events` — `RecvSchedule` requires the event to be present there.
    /// Cleared on first use.
    pending_restore_schedule: Option<EventId>,
    sound: SoundController,
    sim_children: Vec<Child>,
    sim_spawn_config: crate::SimSpawnConfig,
    /// `true` when the refbox was started with `--serial-port`, meaning a
    /// real LED panel is connected. The "Open New Display" button is
    /// disabled in this state so the operator can't fork the panel feed
    /// into a window that competes with the physical display.
    has_led_panel: bool,
    /// Set to `true` the first time the operator presses Start in a
    /// BeepTest session. Gates the Reset button: Reset renders disabled
    /// until this flag is set. The flag is never cleared — Stop and Reset
    /// do not unset it — so it persists for the lifetime of the process.
    beep_test_has_run: bool,
    /// In-memory display layout for BeepTest mode, shown on the player-facing
    /// display. Starts at `Default` every boot and is never persisted (the
    /// game-mode `config.front_display_layout` is untouched). Changed only by
    /// the BeepTest Settings "DISPLAY LAYOUT" picker via
    /// `Message::BeepTestCycleDisplayLayout`.
    beep_test_display_layout: crate::sim_frame::FrontDisplayLayout,
    list_all_events: bool,
    /// `true` when running on a Raspberry Pi (device-tree model check). Gates the
    /// power button's visibility together with `force_power_controls`, and
    /// whether the Pi power actions actually execute.
    is_pi: bool,
    /// `true` when started with `--force-power-controls`: shows the power controls
    /// off-Pi for testing (the Pi actions stay safe no-ops off-Pi).
    force_power_controls: bool,
    mouse_alarm_held: bool,
    spacebar_held: bool,
    alarm_delay_token: u64,
    /// The in-progress timeout-revive long-press, if any (`None` = no hold active).
    timeout_revive: Option<ReviveHold>,
    /// Monotonic source of revive-timer tokens (never reset; guards stale timers).
    timeout_revive_token: u64,
    portal_manager: PortalManager,
    /// Receiver half of the portal-manager background task's event
    /// channel. Wrapped in `Arc<Mutex<Option<_>>>` so an iced
    /// Subscription factory can clone the Arc, `.take()` the Receiver
    /// out once on its first activation, and drive the channel from the
    /// stream task without needing a `&mut` on `self` (which iced's
    /// `subscription(&self)` entry point cannot provide).
    portal_event_rx: Arc<Mutex<Option<mpsc::Receiver<PortalEvent>>>>,
    /// Directory holding the persisted config + portal retry queue. Also
    /// where the self-update trial marker is written (next to the config).
    config_dir: std::path::PathBuf,
    /// Canonical path of the running binary, captured at startup before any
    /// self-update swap. `None` if it could not be resolved (the Updates page
    /// then refuses to install/revert rather than guessing a path).
    install_path: Option<std::path::PathBuf>,
    /// argv this process was launched with, replayed by `main()` after an
    /// in-app restart. Passed to the new binary's smoke test (`--self-check`
    /// short-circuits before any of these take effect).
    restart_argv: Vec<String>,
    /// The release the operator confirmed to install, captured when the check
    /// found a newer version. Drives the download/verify/install pipeline.
    pending_update: Option<crate::updater::release::ReleaseInfo>,
    /// Version of the on-disk backup (`refbox-v*.bak`), if one exists. Shown on
    /// the Revert button so the operator sees which version they'd roll back to.
    update_backup_version: Option<crate::updater::version::Version>,
    /// Debug-only one-shot: when `UWH_PORTAL_SCRAMBLE_TOKEN` is set in a
    /// debug build, this starts `true` and is cleared the first time
    /// `set_current_event_id` is called with `Some(_)`. At that point
    /// the in-memory portal token is replaced with garbage so the next
    /// `verify_token` tick fails and the token-expired flow can be
    /// exercised end-to-end. The on-disk token is never touched.
    ///
    /// Compile-time gated to debug builds so the field, the env-var
    /// name string, and the `std::env::var` call are absent from
    /// release binaries.
    #[cfg(debug_assertions)]
    scramble_token_pending: bool,
}

#[derive(Debug)]
pub struct RefBoxAppFlags {
    pub config: Config,
    pub config_dir: std::path::PathBuf,
    pub serial_ports: Vec<SerialPortBuilder>,
    pub binary_port: u16,
    pub json_port: u16,
    pub sim_children: Vec<Child>,
    pub sim_spawn_config: crate::SimSpawnConfig,
    pub require_https: bool,
    pub fullscreen: bool,
    pub list_all_events: bool,
    pub force_power_controls: bool,
    pub install_path: Option<std::path::PathBuf>,
    pub restart_argv: Vec<String>,
    /// Set by `main()` when the startup safety net auto-reverted a failed update
    /// on this boot. Makes `new()` land on the Updates page with a one-time
    /// rollback notice instead of the normal main screen.
    pub show_rolled_back: bool,
}

#[derive(Debug, Clone)]
enum AppState {
    MainPage,
    TimeEdit(bool, Duration, Option<Duration>),
    ScoreEdit {
        scores: BlackWhiteBundle<u8>,
        is_confirmation: bool,
    },
    PenaltyOverview(BlackWhiteBundle<usize>),
    WarningOverview(BlackWhiteBundle<usize>),
    FoulOverview(OptColorBundle<usize>),
    KeypadPage(KeypadPage, u32),
    GameDetailsPage(bool),
    WarningsSummaryPage,
    /// The Raspberry-Pi power page (Shut Down / Restart Pi / Restart Refbox).
    PowerPage,
    EditGameConfig(ConfigPage),
    // 3rd field: staged `single_half` choice, only meaningful for
    // LengthParameter::Half (the 2 Halves / 1 Period selector). Carried here so
    // it commits on Done and is discarded on Cancel, like the edited Duration.
    ParameterEditor(LengthParameter, Duration, bool),
    ParameterEditorHelp(LengthParameter, Duration, bool),
    ParameterList(ListableParameter, usize),
    ConfirmationPage(ConfirmationKind),
    ConfirmScores(BlackWhiteBundle<u8>),
    /// `scroll_index` is the current scroll offset into the detail-row
    /// list (see `make_scroll_list` in `shared_elements.rs`).
    PortalDetailPage {
        scroll_index: usize,
    },
    /// Shown when the operator taps a red stuck row on the detail page.
    /// `discard_armed` is the two-tap confirmation state for the
    /// DISCARD button; it starts false and flips to true on the first
    /// DISCARD tap (the second tap, for the same item, confirms).
    PortalAttentionAction {
        item_id: ItemId,
        discard_armed: bool,
    },
    /// Shown when `config.mode == Mode::BeepTest`. The cadence engine and
    /// its snapshot live on the running app (`beep_test_tm` / `beep_test_snapshot`).
    BeepTestPage,
    /// BeepTest Settings hierarchy. Reached via the Settings button on the
    /// BeepTest main view. Mirrors the `EditGameConfig(ConfigPage)` pattern.
    BeepTestSettings(BeepTestConfigPage),
    Updates {
        state: crate::app::message::UpdateUiState,
        backup_available: bool,
    },
}

/// Sub-pages inside `AppState::BeepTestSettings`.
///
/// `Main` is the 2x2 landing page (Sound, Edit Levels, App Mode, Language).
/// App Mode is cycled directly on the landing (not its own sub-page).
/// `Sound`, `EditLevels`, `Language`, and `Buzzer` are dedicated BeepTest sub-pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeepTestConfigPage {
    Main,
    Sound,
    EditLevels,
    Language,
    Buzzer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfirmationKind {
    Error(String),
    UwhPortalLinkFailed(PortalTokenResponse),
    // Raised when the site's reply carried an access key this refbox cannot
    // put in a header. Not a `PortalTokenResponse` variant: the server never
    // says this, the refbox concludes it, and that type should keep meaning
    // "what the server replied".
    UwhPortalKeyUnusable,
    // The *FromApply variants are raised by per-page Apply on Game Options. They
    // commit only the Game slice and navigate back to settings (not out to MainPage).
    GameNumberChangedFromApply,
    GameConfigChangedFromApply(GameConfig),
    UwhPortalIncompleteFromApply,
    // Raised by per-page Apply on Game Options when the operator turns the portal
    // OFF mid-game. Switching to manual clears the loaded schedule and resets the
    // before-game clock to the nominal break; this confirms whether to end the
    // current game first or keep it running.
    SwitchToManualFromApply,
    // Raised when the operator changes Mode across the portal boundary (Hockey ↔
    // Rugby). Carries the current and proposed modes so the confirmation page can
    // describe what will change. Raised in apply_app_options (Task 9); rendered
    // in Task 7 view builder; committed via RestartAndApply handler (Task 8).
    //
    // `source` decides which of two messages is shown. A custom site keeps both
    // its address and its token across the restart, so telling that operator the
    // link will be disabled is simply untrue. Manual is grouped with Portal
    // deliberately: a dormant portal link note is still invalidated by the tenant
    // change, so the portal wording remains correct there.
    PortalTenantSwitch {
        from_mode: Mode,
        to_mode: Mode,
        source: GameSource,
    },
    // Raised when an Apply would point the refbox at a different site while it
    // is unsafe to do so. Each carries the page to return to, so the operator
    // lands back where they were with their edit still staged rather than being
    // thrown out of settings.
    SiteLockedByGame(ConfigPage),
    SiteLockedByQueue(ConfigPage),
    /// Linking was attempted while a game is in progress. Carries no page: the
    /// ACCESS TOKEN row lives only on the Game config page, so there is one
    /// place to return to.
    LinkLockedByGame,
    /// Raised by a source-button tap that would clear a fully linked game.
    /// Carries the destination source so the message can name it — the same
    /// sentence serves both directions — and so the affirmative knows where to
    /// switch to. Cancel changes nothing at all: the source buttons never leave
    /// a staged choice showing, so there is nothing to snap back.
    SourceSwitchClearsSelection(GameSource),
}

/// Which of the two kinds of site an address belongs to. Decides which saved
/// credential is used: the operator's Portal login must never be sent to a
/// third-party site, which would hand that site a working Portal credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SiteKind {
    Portal,
    Custom,
}

/// Where the portal client points, resolved from the config.
///
/// One type, produced by one function ([`site_target`]), used both to build the
/// client and to decide whether an Apply repoints it. The SITE row shows the
/// committed address that this is derived from, so what the operator reads and
/// what the refbox talks to cannot drift apart — the drift this whole feature
/// exists to remove.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SiteTarget {
    kind: SiteKind,
    /// Base URL with no trailing slash — every request formats its path onto this.
    base_url: SiteAddress,
    /// `https_only` on the HTTP client. Fixed when the client is built, so a
    /// change here means building a new client rather than editing the old one.
    require_https: bool,
    /// The whole address this came from, which for a custom site includes the
    /// event. Carried so that comparing two targets asks "is this a different
    /// address?" rather than only "is this a different host?" — editing just
    /// the event in the URL must meet the same guards as editing the host.
    address: SiteAddress,
}

/// What the game feed publishes as the site the refbox is using, or `None` when there is nothing
/// safe to publish.
///
/// Extracted for the same reason as the log lines, and with more at stake: this feed is
/// unauthenticated and bound to every interface, so a credential published here reaches every
/// device on the pool LAN rather than a file on one Pi. Reaching for `expose()` at the call site
/// used to leave every test green.
fn published_site_address(site: &SiteTarget) -> Option<String> {
    custom_site::strip_credentials(site.base_url.expose())
}

/// The line logged when the HTTP client cannot be built for a site.
fn client_start_failure_log_line(site: &SiteTarget, error: &impl std::fmt::Display) -> String {
    format!("Failed to start the client for {}: {error}", site.base_url)
}

/// The line logged when there is no client to point at a new site.
fn no_client_log_line(site: &SiteTarget) -> String {
    format!(
        "Cannot point the refbox at {}: no client was started",
        site.base_url
    )
}

/// The line logged when the refbox is pointed at a site.
///
/// Built here rather than inline at the log site so that the test which pins it against
/// credentials exercises the real thing. Written inline, that test could only assert about its own
/// copy of the message, and someone reaching for `expose()` here would leave it green.
///
/// Logs the whole address, not just the host: a custom site and the developer override can share a
/// host, and "which site am I on?" is the first question asked of this line.
fn repoint_log_line(target: &SiteTarget) -> String {
    let kind = match target.kind {
        SiteKind::Portal => "portal",
        SiteKind::Custom => "custom site",
    };
    format!("Pointing the refbox at the {kind}: {}", target.address)
}

/// How a login attempt names the site it went to.
///
/// `portal_name_for_mode` answers a different question — which sport's portal this build is
/// configured for — so on a custom site it labels the exchange "UWH Portal", and in BeepTest mode
/// it labels it nothing at all. On this path that is wrong twice over: what is being logged is a
/// credential exchange, and attributing one site's credential to another is the exact fault the
/// site stamp exists to prevent.
///
/// Deliberately carries no address. `repoint_log_line` can afford one because it is pinned by a
/// test against credential leakage; here the kind is enough to tell a Portal login from a
/// custom-site one, and the address is already logged when the refbox is pointed at the site.
fn login_site_name(kind: SiteKind, mode: Mode) -> String {
    match kind {
        SiteKind::Custom => "the custom site".to_string(),
        SiteKind::Portal => match portal_name_for_mode(mode) {
            "" => "the portal".to_string(),
            name => format!("the {name} Portal"),
        },
    }
}

/// File a login answer's access key against the site and event it was issued to.
///
/// Returns `false` when the answer is stale — the refbox has moved to another site since the login
/// went out — in which case nothing is written.
///
/// The `RecvPortalToken` handler already drops a stale answer whole, before reading it, so in
/// normal flow this check never fires. It is here anyway, on purpose: this is the single line that
/// puts one site's credential into the config, and re-checking at the point of the write means a
/// later edit that moves or loses the handler's guard cannot silently reopen the leak. Extracted
/// rather than written inline for the same reason `repoint_log_line` is a function — so the test
/// exercises the real decision and not a copy of it.
fn file_login_key(
    config: &mut Config,
    site: &str,
    event: &EventId,
    issued_at: u64,
    now: u64,
    token: String,
) -> bool {
    if !reply_is_current(issued_at, now) {
        return false;
    }
    config.store_access_key(site, event, token);
    true
}

/// A client for `target` carrying the key filed for `event`, or carrying none when no key is held.
///
/// Deliberately not the shared client. The background health probe and the result-upload queue
/// hold a clone of that one (`UwhPortalIo::new(Arc::clone(client), ..)`) and both assume its key
/// belongs to the *linked* event. A foreground fetch for a different event -- the one being
/// drafted in the settings editor -- must not borrow theirs and put it back: review on 2026-09-04
/// showed that clearing the shared key from the event picker made the background probe report a
/// false "login expired", and sent queued uploads out uncredentialed for as long as the picker was
/// open.
///
/// Builds a fresh client per call, which is affordable because the callers are operator actions --
/// picking an event, logging in, refreshing -- and not a polling loop.
fn client_for_event(
    target: &SiteTarget,
    config: &Config,
    event: &EventId,
) -> Option<UwhPortalClient> {
    let Some(key) = config.access_key_for(target.base_url.expose(), event) else {
        // No key for this event, so no request. A token-less client would send the privileged
        // fetch with no Authorization header: refused by a strict site, and *accepted* by a
        // permissive one, which is worse -- it would answer as though the refbox were entitled
        // to data it has never authenticated for.
        // Info, not warn: no key yet is the ordinary state of every event before its first
        // login, and a warning per event picked would dilute the level in a support log.
        info!(
            "No access key held for {}; not fetching against it",
            event.full()
        );
        return None;
    };
    let mut client = build_site_client(target)?;
    {
        if let Err(why) = client.set_token(key) {
            // Only reachable from a hand-edited settings file. No client at all, rather than one
            // carrying nothing: the callers treat `None` as "cannot ask", while a token-less
            // client would send the privileged fetch and the token check unauthenticated -- and a
            // permissive site answering `200` to that check paints the row green over no
            // credential whatsoever.
            warn!("A saved access key cannot be sent, so no request will be made: {why}");
            return None;
        }
    }
    Some(client)
}

/// The key filed for one specific event, or `None` when none is held for it.
fn key_for_event<'a>(config: &'a Config, site: &str, event: &EventId) -> Option<&'a str> {
    config.access_key_for(site, event)
}

/// The line logged when a saved custom-site address cannot be used, and what follows from it.
///
/// Names the reason, never the address. The address is the one value that cannot be shown safely
/// here -- it is exactly the string that failed to parse, and an unparsed string cannot be shown
/// to be free of a password; see `custom_site`. The reason is also the more useful half: it says
/// what to fix, rather than handing back a string that already looked right to whoever typed it.
///
/// Both callers share it so neither can drift, and so the test that pins the message exercises the
/// real thing. Written inline, a test could only assert about its own copy, and someone reaching
/// for the address at either site would leave it green.
fn unusable_saved_address_log_line(
    reason: custom_site::CustomSiteError,
    consequence: &str,
) -> String {
    format!("Saved custom site address is not usable ({reason:?}); {consequence}")
}

/// The site `source` calls for, or `None` when it asks for no change.
///
/// `Manual` returns `None` deliberately: with manual games nothing is fetched,
/// but results already queued must keep going to the site they were queued for
/// rather than following the operator back to the built-in portal.
///
/// `Custom` returns `None` when the saved address is empty or unusable, which
/// leaves the client where it is instead of pointing it at nothing.
fn site_target(
    source: GameSource,
    mode: Mode,
    custom_site: &CustomSite,
    portal_require_https: bool,
) -> Option<SiteTarget> {
    match source {
        GameSource::Manual => None,
        GameSource::Portal => Some(portal_target(mode, portal_require_https)),
        GameSource::Custom => {
            custom_site::parse_custom_site(&custom_site.url)
                .ok()
                .map(|parsed| SiteTarget {
                    kind: SiteKind::Custom,
                    // TLS follows the scheme the operator typed, which is what
                    // lets a plain-http site work without the `--allow-http`
                    // launch flag. Mirrors schedule-processor/src/main.rs:63.
                    require_https: parsed.base_url.starts_with("https://"),
                    base_url: parsed.base_url.into(),
                    address: custom_site.url.trim().to_string().into(),
                })
        }
    }
}

/// Whether a client pointed at `kind` can answer for `source`.
///
/// The live client follows the *committed* source, while the editor previews the
/// *staged* one, so the two disagree for as long as a source change sits
/// unapplied. Fetching during that window sends one site's event id to the
/// other — a portal event asked of the operator's own server, or the reverse.
/// The APPLY that repoints the client is what closes the gap; until then the
/// fetch simply does not happen.
fn site_serves(kind: SiteKind, source: GameSource) -> bool {
    match (kind, source) {
        (SiteKind::Portal, GameSource::Portal) => true,
        (SiteKind::Custom, GameSource::Custom) => true,
        // Manual fetches nothing, so no site serves it.
        (_, GameSource::Manual)
        | (SiteKind::Portal, GameSource::Custom)
        | (SiteKind::Custom, GameSource::Portal) => false,
    }
}

/// What tapping one of the two source buttons on Game Options does.
///
/// Pure, and separate from the handler that acts on it, because the settings
/// screens and the message loop have no test harness in this crate: this is the
/// only part of the tap that can be pinned by a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceTapOutcome {
    /// A game is in progress. Refuse, and say which reason applies.
    RefusedByGame,
    /// Results are queued and unsent. Refuse, and say so.
    RefusedByQueue,
    /// A fully linked game would be cleared. Ask before doing it.
    Confirm,
    /// Nothing is at stake — move now, with no screen in between.
    SwitchNow,
}

/// Decide what a source-button tap does.
///
/// The refusals outrank everything else: they exist to stop the refbox being
/// pointed away from a site that a running game or unsent results still depend
/// on, and nothing makes that safe. They are the same two conditions
/// `refuse_repoint` applies to an Apply that would repoint the client; the tap
/// is now a third way to reach the same move, so it asks the same questions.
///
/// `fully_linked` is judged on what is *displayed* — the staged values in the
/// settings editor — not on what is committed. The two are identical unless the
/// operator has picked something and not yet applied it, and in that case the
/// displayed selection is what they would lose.
/// The source a site-scoped reply (`RecvSchedule`, `RecvTeamsList`)
/// should resolve against.
///
/// The COMMITTED source, with one exception: when that is `Manual`.
/// `site_serves` answers false for Manual against every site, so a request is
/// never issued FOR Manual — which means a reply arriving while Manual is
/// committed must belong to the remote the operator has staged and is picking
/// against. Resolving it against `Manual` instead discards it, and because
/// `EventStore::get_mut(Manual, _)` is unconditionally `None` that discard is
/// silent: COURT and GAME simply never fill and APPLY never lights.
///
/// Deliberately NOT the staged source in general. On a remote-to-remote stage
/// the reply still belongs to the committed source, and resolving it against
/// the staged one would file the departed site's data under the new one — the
/// leak the per-source store exists to prevent.
///
/// A different question from the site-generation guard: `reply_is_current` decides whether a
/// reply is accepted; this decides which bucket it lands in. `switch_to_source` commits the
/// source first, and the client may then not move at all — either `target_after_apply` answers
/// `None` so `repoint_client` is never called (a custom site with no usable address saved), or
/// `repoint_client` returns early itself (no client at all in degraded mode, or
/// `build_site_client` fails). Any of those leaves the source moved while the generation is
/// unchanged, so an in-flight reply is filed under the new source. Needs a colliding stale
/// entry there to do harm. Narrow, and deliberately not closed here.
fn reply_source(committed: GameSource, staged: Option<GameSource>) -> GameSource {
    if committed == GameSource::Manual {
        staged.unwrap_or(committed)
    } else {
        committed
    }
}

/// Whether a site-scoped reply still belongs to the site the refbox is on.
///
/// Every request that goes to a *site* reads `site_generation` as it is issued
/// and carries that value on its reply; `repoint_client` bumps the counter when
/// it moves the client. Equal means the refbox has not moved since; anything
/// else means the answer came from a site it has left, and applying it would
/// attribute one site's data to another.
///
/// Why a counter and not the site address: an address would let
/// `portal -> custom -> portal` accept a reply issued on the first portal
/// visit, because the address matches again. That reply is *correct* data, so
/// an address is strictly more precise — but it costs a heap `String` on every
/// one of the dozens of messages an event fires, and the cost of rejecting it
/// is one wasted fetch (a fresh fetch always follows a switch), not a wrong
/// answer. Erring toward dropping is the right default for a guard whose whole
/// purpose is to refuse data of uncertain origin. This is a deliberate
/// trade-off, not an oversight.
fn reply_is_current(issued_at: u64, now: u64) -> bool {
    issued_at == now
}

/// What an App-page APPLY should do with the committed event / court / schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkCommit {
    /// Commit the staged selection: it belongs to the source being applied.
    Staged,
    /// Clear the committed link — going manual means there is no link.
    Clear,
    /// Leave the committed link exactly as it is.
    Leave,
}

/// Decide it. The `Leave` arm is the one that matters: it used to be `Clear`,
/// which destroyed a restored link whenever the portal event list had not
/// loaded, because `owns` is a store lookup and answers false for a perfectly
/// good saved event. Declining to commit keeps an unowned selection out of the
/// committed state just as effectively, without destroying a correct one.
fn link_commit(selection_owned: bool, applied_source: GameSource) -> LinkCommit {
    if selection_owned {
        LinkCommit::Staged
    } else if applied_source == GameSource::Manual {
        LinkCommit::Clear
    } else {
        LinkCommit::Leave
    }
}

fn source_tap_outcome(
    game_in_progress: bool,
    results_queued: bool,
    fully_linked: bool,
) -> SourceTapOutcome {
    if game_in_progress {
        SourceTapOutcome::RefusedByGame
    } else if results_queued {
        SourceTapOutcome::RefusedByQueue
    } else if fully_linked {
        SourceTapOutcome::Confirm
    } else {
        SourceTapOutcome::SwitchNow
    }
}

/// The built-in portal's address for `mode`, honouring the developer override
/// environment variable.
///
/// The override is deliberately consulted only here, never for a custom site:
/// there the operator has typed an address, and quietly sending the requests
/// somewhere else is exactly the failure the source picker removes.
fn portal_target(mode: Mode, require_https: bool) -> SiteTarget {
    // BeepTest reuses the UWH defaults: the client is built during startup in
    // every mode, but the BeepTest UI never makes a portal call, so it sits idle.
    let (default_url, override_var) = match mode {
        Mode::Rugby => ("https://api.uwrportal.com", "UWR_PORTAL_URL_OVERRIDE"),
        Mode::Hockey6V6 | Mode::Hockey3V3 | Mode::BeepTest => {
            ("https://api.uwhportal.com", "UWH_PORTAL_URL_OVERRIDE")
        }
    };
    let url_override = std::env::var(override_var).ok();
    let base_url = url_override
        .as_deref()
        .unwrap_or(default_url)
        .trim_end_matches('/')
        .to_string();
    let address: SiteAddress = base_url.clone().into();
    if url_override.is_some() {
        // Formatted through the address rather than a free redaction helper, so that printing one
        // has exactly one route and a future log line cannot quietly take a plainer one.
        info!(
            "{override_var} active for {} Portal: using {address}",
            portal_name_for_mode(mode)
        );
    }
    SiteTarget {
        kind: SiteKind::Portal,
        address,
        base_url: base_url.into(),
        require_https,
    }
}

/// The message to log when `target` names an address that `https_only` will
/// refuse, or `None` when nothing will be refused.
///
/// `https_only` does not fail when the client is built — it rejects each
/// request inside `reqwest`, which reports it as `builder error for url (...)`.
/// That wording names neither the cause nor the remedy, and it reappears on
/// every call: the periodic health check, the event-list fetch, every upload.
/// Saying it once here, where the address is chosen, explains all of them —
/// and this is the only place that knows both halves of the conflict, the
/// address and the policy.
///
/// Deliberately only reports: the refusal itself is the intended behaviour of
/// the `--allow-http` flag, and the client is still built so that pointing the
/// refbox at a workable site later (a plain-http custom site, say) keeps
/// working without a restart.
fn https_policy_conflict(target: &SiteTarget) -> Option<String> {
    // Compared without case because a URL scheme is case-insensitive and
    // `reqwest` lowercases it before its own check: `HTTPS://…` works fine, so
    // matching it case-sensitively here would log "will all fail" over an
    // address that in fact succeeds.
    let is_https = target
        .base_url
        .expose()
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
    (target.require_https && !is_https).then(|| {
        format!(
            "Portal requests to {} will all fail: the address is not https and this refbox \
             requires https. Restart with --allow-http to use a plain-http address.",
            target.base_url
        )
    })
}

/// Build a client for `target`. Carries no credential -- see `client_for_event`.
///
/// `None` when the client cannot be built at all, which leaves the refbox in
/// its existing degraded mode (red indicator, nothing sent) rather than holding
/// a client pointed somewhere unintended.
fn build_site_client(target: &SiteTarget) -> Option<UwhPortalClient> {
    // No key here on purpose. Keys are filed per (site, event) and no event is named at this
    // point, so whoever needs a credential attaches it: `client_for_event` for a foreground
    // fetch, and the background task per call from the published store.
    let token = None;
    if let Some(msg) = https_policy_conflict(target) {
        error!("{msg}");
    }
    match UwhPortalClient::new(
        target.base_url.expose(),
        token,
        target.require_https,
        REQUEST_TIMEOUT,
    ) {
        Ok(c) => Some(c),
        Err(e) => {
            error!("{}", client_start_failure_log_line(target, &e));
            None
        }
    }
}

// PageEntrySnapshot is a singleton — `RefBoxApp.page_entry_snapshot` holds at most
// one variant at a time. The variant-size disparity from inline `Schedule` doesn't
// compound, so boxing fields purely to satisfy `large_enum_variant` is not worth the
// cascading churn through capture/revert/page_has_changes/apply.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PageEntrySnapshot {
    Game {
        config: GameConfig,
        game_number: GameNumber,
        source: GameSource,
        current_event_id: Option<EventId>,
        current_court: Option<String>,
        schedule: Option<Schedule>,
    },
    App {
        source: GameSource,
        current_event_id: Option<EventId>,
        current_court: Option<String>,
        schedule: Option<Schedule>,
        mode: Mode,
        collect_scorer_cap_num: bool,
        track_fouls_and_warnings: bool,
        force_keypad_numbers: bool,
        show_behind_schedule_time: bool,
        confirm_score: bool,
        hide_time: bool,
        audible_countdown: bool,
    },
    Display {
        white_on_right: bool,
        brightness: matrix_drawing::transmitted_data::Brightness,
        front_display_layout: crate::sim_frame::FrontDisplayLayout,
    },
    Sound {
        sound: SoundSettings,
    },
    Remotes {
        remotes: Vec<RemoteInfo>,
    },
    Language {
        original_language: Option<Language>,
        pending_language: Option<Language>,
    },
    Buzzer {
        buzzer_sound: BuzzerSound,
    },
    CustomSite {
        custom_site: CustomSite,
    },
}

impl PageEntrySnapshot {
    /// Restore the snapshotted fields back onto `edited`. Touches only the
    /// fields owned by this snapshot's page; other slices are left as the user
    /// last edited them.
    pub(in crate::app) fn revert_into(self, edited: &mut EditableSettings) {
        match self {
            PageEntrySnapshot::Game {
                config,
                game_number,
                source,
                current_event_id,
                current_court,
                schedule,
            } => {
                edited.config = config;
                edited.game_number = game_number;
                edited.source = source;
                edited.current_event_id = current_event_id;
                edited.current_court = current_court;
                edited.schedule = schedule;
            }
            PageEntrySnapshot::App {
                source,
                current_event_id,
                current_court,
                schedule,
                mode,
                collect_scorer_cap_num,
                track_fouls_and_warnings,
                force_keypad_numbers,
                show_behind_schedule_time,
                confirm_score,
                hide_time,
                audible_countdown,
            } => {
                edited.source = source;
                edited.current_event_id = current_event_id;
                edited.current_court = current_court;
                edited.schedule = schedule;
                edited.mode = mode;
                edited.collect_scorer_cap_num = collect_scorer_cap_num;
                edited.track_fouls_and_warnings = track_fouls_and_warnings;
                edited.force_keypad_numbers = force_keypad_numbers;
                edited.show_behind_schedule_time = show_behind_schedule_time;
                edited.confirm_score = confirm_score;
                edited.hide_time = hide_time;
                edited.audible_countdown = audible_countdown;
            }
            PageEntrySnapshot::Display {
                white_on_right,
                brightness,
                front_display_layout,
            } => {
                edited.white_on_right = white_on_right;
                edited.brightness = brightness;
                edited.front_display_layout = front_display_layout;
            }
            PageEntrySnapshot::Sound { sound } => {
                edited.sound = sound;
            }
            PageEntrySnapshot::Remotes { remotes } => {
                edited.sound.remotes = remotes;
            }
            PageEntrySnapshot::Language {
                original_language,
                pending_language,
            } => {
                edited.original_language = original_language;
                edited.pending_language = pending_language;
            }
            PageEntrySnapshot::Buzzer { buzzer_sound } => {
                edited.sound.buzzer_sound = buzzer_sound;
            }
            PageEntrySnapshot::CustomSite { custom_site } => {
                edited.custom_site = custom_site;
            }
        }
    }
}

/// One countdown beep this tick when: the audible-countdown setting is on, we are
/// in a break that precedes a playing period, the whole-second value just changed,
/// and the new value is in the final 10..=1 window. Reads the RAW snapshot, so it
/// is independent of the visual "show countdown" (`hide_time`) setting.
fn should_play_countdown_beep(
    period: GamePeriod,
    new_secs: u32,
    old_secs: u32,
    audible_countdown: bool,
) -> bool {
    let is_break_before_play = matches!(
        period,
        GamePeriod::BetweenGames
            | GamePeriod::HalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreSuddenDeath
    );
    audible_countdown
        && is_break_before_play
        && new_secs != old_secs
        && (1..=10).contains(&new_secs)
}

/// Whether the result the refbox has on file may be submitted for the game the clock just
/// left. A recorded result belongs to exactly one game and carries that game's number
/// (`LastGameInfo::game_number`).
///
/// Ending a game normally records a result for it, so the two agree and the result is sent.
/// Abandoning a game — END CURRENT GAME AND APPLY, which resets the game instead of ending
/// it — records nothing, so the newest result on file belongs to an EARLIER game. Sending
/// that under the abandoned game's number would post the previous game's score against a
/// game that was never played.
fn recorded_result_matches_ended_game(
    recorded_game: Option<&GameNumber>,
    ended_game: &GameNumber,
) -> bool {
    recorded_game == Some(ended_game)
}

/// Both teams' cap numbers for a scheduled game **on this court**, read from the
/// session cache. Empty vectors where the slot has no portal team assigned (a
/// placeholder such as "winner of A"), where no roster has arrived, or where the
/// game is not this court's -- those teams get the number pad.
///
/// The court check is load-bearing. Game numbers are unique across a whole event,
/// not per court, and when no next game is scheduled the engine synthesises one by
/// incrementing the current number (`next_game_number`). That invented number can
/// land on a real game being played on another court, whose two teams are not in
/// this pool at all. The number pad claims nothing; a grid of the wrong team's cap
/// numbers claims something false, so the pad is the better answer.
///
/// Note this is one reader being made honest, not the invented number being fixed.
/// `RecvSchedule` still adopts it as the engine's next game, and the Game Info page
/// still names that game and its teams without a court check -- so the wrong game
/// *is* already visible elsewhere on screen. Correcting that belongs where the
/// number is produced or adopted, not here.
///
/// A `current_court` of `None` is treated as "no court to disagree with" rather
/// than as a mismatch. That is safe because a portal setup cannot be applied
/// without a court in the first place: `EditableSettings::uwhportal_incomplete`
/// requires `current_court` to be set *and* the selected game to be on it, so
/// there is no court-less state with a game selection for a roster to resolve
/// against. Keeping it permissive also means no existing caller changes behaviour.
fn rosters_for_scheduled_game(
    schedule: Option<&Schedule>,
    team_rosters: &BTreeMap<TeamId, Vec<u8>>,
    current_court: Option<&str>,
    game_num: &GameNumber,
) -> BlackWhiteBundle<Vec<u8>> {
    let mut out = BlackWhiteBundle {
        black: Vec::new(),
        white: Vec::new(),
    };

    if let Some(schedule) = schedule {
        if let Some(game) = schedule.games.get(game_num) {
            if current_court.is_none_or(|court| game.court == court) {
                for (color, team) in [(Color::Black, &game.dark), (Color::White, &game.light)] {
                    if let Some(numbers) = team.assigned().and_then(|id| team_rosters.get(id)) {
                        out[color] = numbers.clone();
                    }
                }
            }
        }
    }

    out
}

/// The game whose roster the player picker must offer, or `None` to use the copy
/// pinned at kickoff.
///
/// The picker must always offer the roster of the game an entry made *now* would
/// land on. Between games -- including before the first kickoff of a session,
/// which is where the app sits from launch until the first game begins -- that is
/// the game about to start, so the picker follows `next_game_number`. It is read
/// live rather than pinned, so a roster arriving mid-break appears instead of
/// being locked out until the next kickoff.
///
/// During play it is the running game, and the copy pinned at kickoff is used
/// instead of a fresh lookup. That is what stops a mid-game REFRESH moving numbers
/// under the operator's hand, and with it keeps the grid design's guarantee that a
/// number recorded during a game is present on that game's grid.
///
/// Deliberately **not** `GameSnapshot::game_number()`, which looks like the right
/// answer and is not. That helper returns `next_game_number` only when
/// `BetweenGames && !is_old_game`; the post-game window is the *other* half,
/// `BetweenGames && is_old_game` (`is_old_game` is `!has_reset`, and `reset()`
/// has not yet run). So for the first two minutes of every break the helper names
/// the **finished** game, and using it here would put the previous game's players
/// on offer -- the exact bug this change exists to fix.
///
/// Note the two halves are easy to invert. `is_old_game` is *also* true throughout
/// normal play, so it is never a standalone test for "the game has ended".
fn picker_roster_game(snapshot: &GameSnapshot) -> Option<&GameNumber> {
    if snapshot.current_period == GamePeriod::BetweenGames {
        Some(&snapshot.next_game_number)
    } else {
        None
    }
}

/// The anchor after a game leaves the clock: advanced only when the result that
/// was recorded belongs to the game that just ended.
///
/// `recorded` is the game number the newest recorded result belongs to, or `None`
/// when no result was recorded at all. Both non-matching cases mean the game was
/// abandoned or interrupted, and the anchor must not move — the same game is
/// offered again, which is recoverable, where skipping it is not.
fn anchor_after_game_end(
    recorded: Option<&GameNumber>,
    ended: &GameNumber,
    scheduled_start: Option<time::OffsetDateTime>,
    current: (Option<GameNumber>, Option<time::OffsetDateTime>),
) -> (Option<GameNumber>, Option<time::OffsetDateTime>) {
    if recorded_result_matches_ended_game(recorded, ended) {
        (Some(ended.clone()), scheduled_start)
    } else {
        current
    }
}

/// `true` when the break now on screen will start nothing when it expires: we are
/// between games and the next-game number is blank, which is how the refbox reports
/// that the selected court has no further scheduled games.
///
/// All three break sounds are gated on this: the 30-second whistle, the start-of-play
/// buzzer at 0:00, and the audible countdown on the way there. Nothing is coming, so
/// counting the poolside down or sounding them would announce a game that never begins.
///
/// Only `BetweenGames` can be silenced. Every other break has a game in progress and
/// will start play again, whatever the next-game number says — hence the period test.
fn break_starts_nothing(period: GamePeriod, next_game_number: &str) -> bool {
    period == GamePeriod::BetweenGames && next_game_number.is_empty()
}

/// What the restart note should record about the game.
///
/// The distinction that matters is **knowledge versus ignorance**. Treating the
/// two alike is what made this note dangerous twice over: writing a guess into
/// it brought a finished court back as game 1, which replayed and re-posted the
/// day; writing a blank instead erased a mid-event operator's resume point,
/// sending the next launch back to the earliest game on the court.
#[derive(Debug, PartialEq, Eq)]
enum LinkNoteGame {
    /// Knowledge. `Some(number)` is the game the operator is on; `None` means
    /// this court's schedule is finished — remember the court, but no game, so
    /// a restart comes back to the same state.
    Write(Option<GameNumber>),
    /// Ignorance: no schedule has been read yet, so there is nothing to record.
    /// Leave any existing note exactly as it is.
    Unknown,
}

/// Decide what the restart note should say, from the **live engine** — never
/// from the cached `self.snapshot`. The finished state deliberately holds the
/// break clock stopped, and a stopped clock is what stops snapshots being
/// regenerated, so the cached copy goes stale exactly when this matters.
fn link_note_game(tm: &TournamentManager) -> LinkNoteGame {
    if tm.current_period() != GamePeriod::BetweenGames {
        // A game is in progress: that game is the operator's place.
        return LinkNoteGame::Write(Some(tm.game_number()));
    }

    if let Some(info) = tm.next_game_info() {
        // A schedule supplied this game — the only source we trust.
        return LinkNoteGame::Write(Some(info.number.clone()));
    }

    // With no next-game info, a blank number is the engine reporting that it was
    // *told* this court is finished. Anything else is the arithmetic fallback
    // `game_number + 1` — ignorance wearing the shape of an answer.
    if tm.next_game_number().is_empty() {
        LinkNoteGame::Write(None)
    } else {
        LinkNoteGame::Unknown
    }
}

impl RefBoxApp {
    fn apply_snapshot(&mut self, mut new_snapshot: GameSnapshot) -> Task<Message> {
        let mut task = Task::none();
        if new_snapshot.current_period != self.snapshot.current_period {
            if new_snapshot.current_period == GamePeriod::BetweenGames {
                task = self.handle_game_end(&new_snapshot.game_number);
            } else if self.snapshot.current_period == GamePeriod::BetweenGames {
                task = self.handle_game_start(&new_snapshot.game_number);
            }
        }

        new_snapshot.event_id = self.current_event_id.clone();
        // The address this refbox's own portal client is pointed at, so a consumer resolving names
        // from a portal looks them up where refbox looks them up. `current_site` is the value the
        // client is built from -- already accounting for the override env var, Rugby mode's
        // separate portal, and a custom site -- so this reports rather than re-derives, instead of
        // computing a second answer that could drift from the client's.
        //
        // It does NOT follow that the reported address always has a live client behind it.
        // `repoint_client` returns early without assigning `current_site` when there is no client
        // (degraded start) or when one cannot be built, so after such an APPLY this keeps naming
        // the startup site. The field's own doc is deliberate about that: it says where lookups
        // do or would go, not that any are happening.
        //
        // Credentials are stripped first: this feed is unauthenticated and bound to every
        // interface, and a custom site is stored exactly as the operator typed it, so a URL
        // entered as `https://user:password@host/...` would otherwise broadcast the password to
        // everyone on the pool LAN. An address that will not parse reports `None` rather than a
        // guess -- see `strip_credentials`.
        new_snapshot.portal_base_url = published_site_address(&self.current_site);

        self.maybe_play_sound(&new_snapshot);
        if let Err(e) = self.update_sender.send_snapshot(
            new_snapshot.clone(),
            self.config.hardware.white_on_right,
            self.config.hardware.brightness,
        ) {
            // Channel-full or closed: the next snapshot re-sends fresh state,
            // so dropping one is acceptable -- never crash the refbox over a
            // slow/stalled display consumer.
            warn!("Failed to send snapshot to displays: {e:?}");
        }
        self.snapshot = new_snapshot;
        task
    }

    fn maybe_play_sound(&self, new_snapshot: &GameSnapshot) {
        // A break that will start nothing must not be announced: no 30-second whistle,
        // no start-of-play buzzer at 0:00 and no countdown beeps on the way there.
        // Once the court's last game has ended the clock stops dead, so the common path
        // never reaches 30 seconds; this matters for the other ordering, where a break
        // is already counting down when a schedule refresh reports the court finished.
        let starts_nothing =
            break_starts_nothing(new_snapshot.current_period, &new_snapshot.next_game_number);

        let (play_whistle, play_buzzer) = match new_snapshot.timeout {
            Some(TimeoutSnapshot::Black(time)) | Some(TimeoutSnapshot::White(time)) => {
                match self.snapshot.timeout {
                    Some(TimeoutSnapshot::Black(old_time))
                    | Some(TimeoutSnapshot::White(old_time)) => (
                        time != old_time && time == 15,
                        time != old_time && time == 0,
                    ),
                    _ => (false, false),
                }
            }
            Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => (false, false),
            None => {
                let prereqs = new_snapshot.current_period != GamePeriod::SuddenDeath
                    && new_snapshot.secs_in_period != self.snapshot.secs_in_period;

                let is_whistle_period = match new_snapshot.current_period {
                    GamePeriod::BetweenGames
                    | GamePeriod::HalfTime
                    | GamePeriod::PreOvertime
                    | GamePeriod::OvertimeHalfTime
                    | GamePeriod::PreSuddenDeath => true,
                    GamePeriod::FirstHalf
                    | GamePeriod::SecondHalf
                    | GamePeriod::OvertimeFirstHalf
                    | GamePeriod::OvertimeSecondHalf
                    | GamePeriod::SuddenDeath => false,
                };

                let (end_starts_play, end_stops_play) = match new_snapshot.current_period {
                    GamePeriod::FirstHalf
                    | GamePeriod::SecondHalf
                    | GamePeriod::OvertimeFirstHalf
                    | GamePeriod::OvertimeSecondHalf => (false, true),
                    GamePeriod::BetweenGames
                    | GamePeriod::HalfTime
                    | GamePeriod::PreOvertime
                    | GamePeriod::OvertimeHalfTime
                    | GamePeriod::PreSuddenDeath => (true, false),
                    GamePeriod::SuddenDeath => (false, false),
                };

                let is_buzz_period = end_starts_play && self.config.sound.auto_sound_start_play
                    || end_stops_play && self.config.sound.auto_sound_stop_play;

                (
                    prereqs
                        && is_whistle_period
                        && !starts_nothing
                        && new_snapshot.secs_in_period == 30,
                    prereqs
                        && is_buzz_period
                        && !starts_nothing
                        && new_snapshot.secs_in_period == 0,
                )
            }
        };

        let play_countdown = new_snapshot.timeout.is_none()
            && !starts_nothing
            && should_play_countdown_beep(
                new_snapshot.current_period,
                new_snapshot.secs_in_period,
                self.snapshot.secs_in_period,
                self.config.audible_countdown,
            );

        if play_whistle {
            info!("Triggering whistle");
            self.sound.trigger_whistle();
        } else if play_buzzer {
            info!("Triggering buzzer");
            self.sound.trigger_buzzer();
        }

        if play_countdown {
            info!("Triggering countdown beep");
            self.sound.trigger_countdown_beep();
        }
    }

    /// Beep-test variant of `maybe_play_sound`. Compares a freshly-generated
    /// `BeepTestSnapshot` against `self.beep_test_snapshot` and fires the
    /// whistle (5 s before lap end) or the buzzer (at lap end, gated by
    /// the operator's auto-start/stop sound settings). Ported verbatim
    /// from `beep-test/src/app/mod.rs::maybe_play_sound`.
    fn maybe_play_beep_test_sound(&self, new_snapshot: &BeepTestSnapshot) {
        use crate::beep_test::snapshot::BeepTestPeriod;

        let (play_whistle, play_buzzer) = {
            let prereqs = new_snapshot.secs_in_period != self.beep_test_snapshot.secs_in_period;

            // Pre is gone — the only period variant is Level(_), which always
            // warrants whistle/buzzer triggers. Keeping the explicit match makes
            // the intent clear and stays correct if a new variant is added later.
            let is_whistle_period = matches!(new_snapshot.current_period, BeepTestPeriod::Level(_));

            let (end_starts_play, end_stops_play) = (true, false);

            let is_buzz_period = end_starts_play && self.config.sound.auto_sound_start_play
                || end_stops_play && self.config.sound.auto_sound_stop_play;

            (
                prereqs && is_whistle_period && new_snapshot.secs_in_period == 5,
                prereqs && is_buzz_period && new_snapshot.secs_in_period == 0,
            )
        };

        if play_whistle {
            info!("Triggering whistle");
            self.sound.trigger_whistle();
        } else if play_buzzer {
            info!("Triggering buzzer");
            self.sound.trigger_buzzer();
        }
    }

    /// Return the BeepTest page and engine to the idle (pre-START) state:
    /// green START, greyed RESET, LEVEL and LAP at 0, table cleared. Shared
    /// by the operator's RESET button and the automatic reset that fires
    /// when the schedule completes.
    fn reset_beep_test_state(&mut self, now: Instant) {
        if let Some(ref mut bt_tm) = self.beep_test_tm {
            bt_tm.reset_beep_test_now(now);
        }
        self.beep_test_snapshot = BeepTestSnapshot::default();
        self.beep_test_has_run = false;
    }

    /// Fetch the UWH Portal's event list — always from the portal itself.
    ///
    /// Deliberately not through `uwhportal_client`: that client follows the
    /// *committed* source, so on a custom site it points at the operator's own
    /// server, which has no portal event list and no reason to be asked for
    /// one. A client built here for the portal keeps the list loading whatever
    /// the refbox is committed to, and — unlike repointing — does not move
    /// where results are sent, which must keep going to the site they were
    /// queued for until an APPLY says otherwise.
    ///
    /// Carries no credential at all. `build_site_client` files no key -- keys belong to a
    /// (site, event) pair and no event is named here -- and the event list is not a privileged
    /// endpoint, so none is needed. This paragraph previously described where its token came
    /// from and how `UWH_PORTAL_SCRAMBLE_TOKEN` interacted with it; neither has been true since
    /// the key store landed.
    ///
    /// Deliberately NOT stamped with `site_generation`, unlike every other
    /// reply in this group. This fetch does not use the live client at all — it
    /// builds its own against the portal, so its answer is portal data whatever
    /// source the refbox is committed to, and `set_portal_list` files it in the
    /// portal bucket either way. Stamping it would drop a perfectly good event
    /// list whenever a source switch happened while it was in flight, emptying
    /// the event picker in a way that looks like a network fault — and would
    /// undo `297ff166`, which made the list load regardless of source on
    /// purpose. The guard belongs on replies whose meaning depends on which
    /// site answered; this one's does not.
    fn request_event_list(&self) -> Task<Message> {
        let target = portal_target(self.config.mode, self.require_https);
        let Some(client) = build_site_client(&target) else {
            // build_site_client has already logged why.
            return Task::none();
        };
        // The future captures nothing (`impl Future + use<>`), so the client can
        // be dropped here rather than held across the await.
        let request = client.get_event_list(self.list_all_events, true);
        Task::future(async move {
            match request.await {
                Ok(events) => {
                    info!("Got event list");
                    Message::RecvEventList(events)
                }
                Err(e) => {
                    // Expected whenever there is no route to the portal — a
                    // custom site on a closed poolside network, or manual games
                    // offline. Nothing else changes and nothing is shown.
                    error!("Failed to get event list: {e}");
                    Message::NoAction
                }
            }
        })
    }

    /// Directory the running binary lives in (where the new binary is swapped
    /// in and where the `refbox-v*.bak` backup is kept). `None` if the install
    /// path couldn't be resolved at startup.
    fn updater_install_dir(&self) -> Option<std::path::PathBuf> {
        self.install_path
            .as_ref()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
    }

    /// Scratch path the downloaded binary is written to before verification and
    /// the swap. Lives next to the running binary so the final swap is a rename
    /// within one filesystem.
    fn updater_temp_path(&self) -> Option<std::path::PathBuf> {
        self.updater_install_dir().map(|d| d.join("refbox.new"))
    }

    /// The single `refbox-v*.bak` backup next to the binary, if present.
    fn find_update_backup(&self) -> Option<std::path::PathBuf> {
        let dir = self.updater_install_dir()?;
        std::fs::read_dir(&dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("refbox-v") && n.ends_with(".bak"))
                    .unwrap_or(false)
            })
    }

    fn request_teams_list(&self, event_id: EventId) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // The site this request goes out against, carried on the reply.
            let issued_at = self.site_generation;
            // why this cannot panic: the guard is held only long enough to build
            // the request and is dropped before the await, and no writer panics
            // while holding it. `request_event_list` used to carry this note and
            // the sites below pointed at it, but it builds its own client now and
            // takes no lock at all — so the justification lives here instead.
            let request = client.lock().unwrap().get_event_teams(&event_id);
            Task::future(async move {
                match request.await {
                    Ok(teams) => {
                        info!("Got teams list");
                        Message::RecvTeamsList(event_id, teams, issued_at)
                    }
                    Err(e) => {
                        error!("Failed to get teams list: {e}");
                        Message::NoAction
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    fn request_team_roster(&self, team_id: TeamId) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // The site this request goes out against, carried on the reply so
            // the handler can tell a roster from the site the refbox is on now
            // from one still arriving from the site it has left. Read here, at
            // the moment the request is issued, because that is the only point
            // at which the answer is known. This was a `GameSource` until the
            // reply-origin work: a source tag cannot distinguish one custom
            // site from another, and rosters are exactly where that bites,
            // because the cache is keyed by team id alone.
            let issued_at = self.site_generation;
            // why this cannot panic: see `request_teams_list` above.
            let request = client.lock().unwrap().get_team_roster(&team_id);
            Task::future(async move {
                match request.await {
                    Ok(players) => {
                        let numbers = usable_cap_numbers(&players);
                        info!(
                            "Got roster for team {}: {} numbered players",
                            team_id.full(),
                            numbers.len()
                        );
                        Message::RecvTeamRoster(team_id, numbers, issued_at)
                    }
                    Err(e) => {
                        // A failure must leave whatever is cached untouched, so
                        // this reports nothing rather than an empty roster.
                        error!("Failed to get roster for team {}: {e}", team_id.full());
                        Message::NoAction
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    fn request_schedule(&self, event_id: EventId) -> Task<Message> {
        // The privileged schedule endpoint is the only per-event authenticated fetch, and it is
        // where a wrong key shows up as a court list that never loads. It carries the key filed
        // for the event it asks about, on a client of its own, so asking about an event other than
        // the linked one cannot disturb what the background tasks are holding.
        if let Some(client) = client_for_event(&self.current_site, &self.config, &event_id) {
            // The site this request goes out against, carried on the reply.
            // Read at the moment of issue — the only point at which it is known.
            let issued_at = self.site_generation;
            let schedule_req = client.get_event_schedule_privileged(&event_id);
            let names_req = client.get_event_referee_name_map_from_referees(&event_id);
            Task::future(async move {
                let mut schedule = match schedule_req.await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to get schedule: {e}");
                        return Message::NoAction;
                    }
                };
                // Fetch referee display names from the public /referees endpoint.
                // If the call fails (e.g. no network), log at warn level and proceed
                // without names — the schedule still loads and refs show "-".
                let name_map = match names_req.await {
                    Ok(map) => map,
                    Err(e) => {
                        warn!("Failed to fetch referee names: {e}");
                        Default::default()
                    }
                };
                for game in schedule.games.values_mut() {
                    if let Some(assignments) = &mut game.referee_assignments {
                        for assignment in assignments.iter_mut() {
                            if let Some(uid) = &assignment.user_id {
                                if let Some(name) = name_map.get(uid) {
                                    assignment.display_name = Some(name.clone());
                                }
                            }
                        }
                    }
                }
                info!("Got schedule");
                Message::RecvSchedule(event_id, schedule, issued_at)
            })
        } else {
            // Not `Task::none()`. `RequestPortalRefresh` arms the spinner on a condition this
            // function no longer depends on -- it builds its own client now -- and disarms it by
            // translating `NoAction`. Falling silent here leaves REFRESH spinning forever.
            Task::done(Message::NoAction)
        }
    }

    fn request_uwhportal_token(&self, event_id: &EventId, code: u32) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // The site this login goes out against, carried on the reply. Read
            // here, at the moment of issue, because that is the only point at
            // which the answer is known. It matters more here than anywhere
            // else in this group: what comes back is an access key, so a reply
            // resolved against the wrong site hands one site's credential to
            // another rather than merely showing stale data.
            let issued_at = self.site_generation;
            // The event this login was issued for, captured here rather than
            // read back off `self` when the reply lands -- see the field
            // comment on `Message::RecvPortalToken`.
            let event = event_id.clone();
            // why this cannot panic: see `request_teams_list` above.
            let request = client.lock().unwrap().login_to_portal(event_id, code);
            let site = login_site_name(self.current_site.kind, self.config.mode);
            Task::future(async move {
                match request.await {
                    Ok(token) => {
                        info!("Got a response from a login request to {site}");
                        Message::RecvPortalToken(token, event, issued_at)
                    }
                    Err(e) => {
                        error!("Failed to get an access key from {site}: {e}");
                        Message::NoAction
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    /// Whether games come from a remote site at all, as opposed to being
    /// entered by hand. Most callers only need this question; the few that must
    /// tell the official Portal from a third-party site match on `source`.
    fn uses_remote(&self) -> bool {
        !matches!(self.source, GameSource::Manual)
    }

    /// The source reads should resolve against: the one staged in the editor
    /// while it is open, the committed one otherwise.
    ///
    /// The editor previews the staged source, which is the same rule `view()`
    /// already applies to the staged *event* id — the pickers have to show what
    /// the operator is choosing, not what is committed. Writes do not follow
    /// this: `commit_source` is the only thing that moves the committed source.
    fn active_source(&self) -> GameSource {
        self.edited_settings
            .as_ref()
            .map_or(self.source, |edited| edited.source)
    }

    /// The source a site-scoped reply should resolve against. See
    /// `reply_source` for the rule; this only supplies the two inputs.
    fn reply_source(&self) -> GameSource {
        reply_source(
            self.source,
            self.edited_settings.as_ref().map(|edited| edited.source),
        )
    }

    /// Commit an applied source: the live field, the saved field so a relaunch
    /// comes back on the same source, and — for a real remote — the one to
    /// return to when MANUAL is switched off again.
    ///
    /// Only a real remote is remembered: applying MANUAL leaves the previous
    /// choice standing, which is the whole point of keeping it separately.
    fn commit_source(&mut self, source: GameSource) {
        self.source = source;
        self.config.source = source;
        match source {
            GameSource::Portal => self.config.remembered_remote = RemoteSource::Portal,
            GameSource::Custom => self.config.remembered_remote = RemoteSource::Custom,
            GameSource::Manual => {}
        }
        // Keep the engine's view of linkage in step with the app's, at the one
        // choke point where the live source is committed, so the two cannot
        // drift. Linked, the engine refuses to name a next game it was never
        // given; unlinked (manual), it resumes its own numbering.
        self.tm.lock().set_schedule_linked(self.uses_remote());
    }

    /// Commit a whole [`LinkSelection`].
    ///
    /// Every APPLY path that carries the operator's selections forward goes
    /// through here — including the ones that detour via a confirmation page —
    /// so that no such path can commit part of the set and silently drop the
    /// rest.
    ///
    /// The one deliberate exception is the `RestartAndApply` arm of a portal
    /// tenant switch, which commits `source` inline and forces `event_id` to
    /// `None` rather than carrying the selections over: it is unpinning the old
    /// tenant, so the court and schedule it would copy belong to an event that
    /// is being abandoned. That arm always ends in a restart, and `decide_restore`
    /// refuses a saved note from the other portal, so the dropped fields cannot
    /// be re-adopted on the way back up. A fifth field added to `LinkSelection`
    /// would need a decision there rather than an automatic copy.
    fn commit_link_selection(&mut self, selection: LinkSelection) {
        let LinkSelection {
            source,
            event_id,
            court,
            schedule,
        } = selection;
        // The anchor describes the last game played on one particular event and
        // court (decision 25), so it cannot outlive a change to either. Cleared
        // here rather than at each caller because this is the one funnel every
        // APPLY path goes through, and it must read the outgoing event/court
        // before the writes below replace them. `LinkSelection::manual()` carries
        // `None`/`None` — the portal going off entirely — which compares unequal
        // whenever there was a live event or court for the anchor to describe.
        self.clear_anchor_if_event_or_court_changing(&event_id, &court);
        self.commit_source(source);
        // Route through set_current_event_id so portal_event_id stays in sync
        // (ADR 011 amendment 2026-04-23 dormant-until-linked).
        self.set_current_event_id(event_id);
        self.current_court = court;
        self.schedule = schedule;
    }

    /// The site an Apply would leave the refbox pointed at, given the source
    /// and saved address it is about to commit. `None` means "no change".
    fn target_after_apply(
        &self,
        source: GameSource,
        mode: Mode,
        custom_site: &CustomSite,
    ) -> Option<SiteTarget> {
        site_target(source, mode, custom_site, self.require_https)
            .filter(|target| *target != self.current_site)
    }

    /// The different site this page's APPLY would move to, if it moves at all.
    ///
    /// Both controls that can repoint the refbox are covered: the source
    /// buttons, and the SITE address itself. The SITE editor is only reachable
    /// with CUSTOM chosen, so the address it commits is the one about to be
    /// used whether or not CUSTOM has been applied on the Game page yet —
    /// which is what lets the schedule and teams be fetched from it.
    fn pending_site_change(&self, page: ConfigPage) -> Option<SiteTarget> {
        let edited = self.edited_settings.as_ref()?;
        match page {
            ConfigPage::CustomSite(_) => {
                self.target_after_apply(GameSource::Custom, edited.mode, &edited.custom_site)
            }
            ConfigPage::Game | ConfigPage::App => {
                self.target_after_apply(edited.source, edited.mode, &edited.custom_site)
            }
            _ => None,
        }
    }

    /// Refuse a repoint that would strand the operator or their results, saying
    /// which of the two reasons applies. A silent refusal is indistinguishable
    /// from a broken button, so both cases produce a message rather than a
    /// greyed-out control.
    ///
    /// `page` is where to return to, so the refused edit stays staged and the
    /// operator can stop the clock (or send the results) and press APPLY again.
    fn refuse_repoint(&self, page: ConfigPage) -> Option<ConfirmationKind> {
        // A game in progress, not merely a running clock: between games the
        // clock counts down to the next game and is running by default, which
        // is exactly when an operator sets the source up. This matches the rule
        // the game-config and switch-to-manual gates already use.
        //
        // This refusal carries more than its own message. `apply_game_options`
        // returns early — before the repoint block in `ApplyConfigPage` ever
        // runs — on three confirmations that each require exactly this
        // condition: `GameConfigChangedFromApply`, `GameNumberChangedFromApply`
        // and `SwitchToManualFromApply`. Refusing here is the only thing
        // keeping a pending repoint from reaching them. Allowing a site change
        // during a game, even behind a confirmation, means repointing on those
        // paths too, or the new source is committed and the client never moves.
        if self.tm.lock().current_period() != GamePeriod::BetweenGames {
            return Some(ConfirmationKind::SiteLockedByGame(page));
        }
        if self.portal_manager.has_queued_items() {
            return Some(ConfirmationKind::SiteLockedByQueue(page));
        }
        None
    }

    /// Point the live client at `target`, with no restart.
    ///
    /// Building a fresh client rather than editing the old one is required, not
    /// stylistic: whether TLS is demanded is fixed on the HTTP client when it is
    /// created. The new client is assigned *through* the existing shared handle
    /// rather than replacing it, so the background retry task — which holds a
    /// clone of that handle — sends to the new site too. A request already in
    /// flight is unaffected: every call locks, builds its request, releases the
    /// lock and only then awaits, so it completes against the site it was
    /// addressed to.
    fn repoint_client(&mut self, target: SiteTarget) {
        let Some(shared) = self.uwhportal_client.as_ref() else {
            // No client at all means the refbox started in degraded mode (only
            // reachable from a bad https-only config). There is no background
            // task to hand a new client to, so this needs a restart, not a swap.
            error!("{}", no_client_log_line(&target));
            return;
        };
        let Some(new_client) = build_site_client(&target) else {
            return;
        };
        info!("{}", repoint_log_line(&target));
        self.current_site = target;
        // The new client and the key view that belongs to it, installed together under both
        // locks in the reader's order (keys, then client). `UwhPortalIo::request_for` takes the
        // same two the same way round: published separately, it could read the departed site's
        // key between the two acquisitions and put it on the client already pointing at the new
        // site, which is precisely the cross-site leak the store exists to prevent.
        {
            // why this cannot panic: `unwrap_or_else(into_inner)` keeps a poisoned lock usable,
            // and neither the assignment nor `publish` can panic while they are held.
            let mut keys = self
                .portal_access_keys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut client = shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *client = new_client;
            keys.publish(
                self.current_site.base_url.expose().to_string(),
                self.config.access_keys.clone(),
            );
        }
        // Only here. The two early returns above leave the client exactly where
        // it was, so replies in flight are still from the site the refbox is on
        // and must NOT be invalidated. (Same asymmetry `1f4bdc62` fixed for the
        // portal fetch; keep the two consistent.)
        self.site_generation = self.site_generation.wrapping_add(1);
    }

    /// Publish the site and the key store to the background uploader.
    ///
    /// The background task resolves a key per call -- the linked event's for its health probe, the
    /// item's own for each queued result -- so it needs the store, not a pre-loaded credential.
    /// This is the whole of the foreground's involvement with what the background sends.
    ///
    /// Called wherever the site or the store changes. It is deliberately *not* called when the
    /// operator merely links or unlinks an event: a result queued for an event is still that
    /// event's to deliver, so dropping to manual games when the network dies must not strand it.
    fn publish_access_keys(&self) {
        // why this cannot panic: `unwrap_or_else(into_inner)` keeps a poisoned lock usable, and
        // nothing here can panic while holding it.
        self.portal_access_keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .publish(
                self.current_site.base_url.expose().to_string(),
                self.config.access_keys.clone(),
            );
    }

    /// Re-seed the ACCESS TOKEN indicator after the refbox has been pointed at
    /// a different site.
    ///
    /// Without this the row keeps the verdict it reached about the *previous*
    /// site, which is worse than saying nothing: the credential is per-site, so
    /// a green verified state earned from the portal would sit above a third-party site the
    /// refbox has never authenticated to. Mirrors the seeding done when the
    /// settings editor is opened.
    fn refresh_token_indicator(&mut self) -> Task<Message> {
        // Asked of the store, for the site the refbox is now on. The shared client no longer
        // carries a credential of its own -- the background task loads one per call -- so there
        // is nothing there to ask.
        let (valid, task) = match self.current_event_id.clone() {
            Some(id)
                if self.uwhportal_client.is_some()
                    && key_for_event(&self.config, self.current_site.base_url.expose(), &id)
                        .is_some() =>
            {
                (None, self.check_uwhportal_auth(&id))
            }
            // No key for this site and event, or no event to check one against:
            // the resting state, same as opening the editor in that condition.
            _ => (Some(false), Task::none()),
        };
        if let Some(ref mut settings) = self.edited_settings {
            settings.uwhportal_token_valid = valid;
        }
        task
    }

    /// Adopt the event named inside the custom site's URL and pull its teams
    /// and schedule.
    ///
    /// A custom site never calls the event list — its event is named in the URL
    /// — so its entry in the per-source `EventStore` has to be made here. That
    /// entry is not bookkeeping: `RecvTeamsList` and `RecvSchedule` both store
    /// *into* it and log an error when it is missing, and the court picker
    /// reads its court list from it, so without one the court list stays
    /// permanently empty. It is unreachable from the portal's own picker by
    /// construction — `EventStore::selectable` answers `None` for a custom site.
    ///
    /// Safe to call more than once: `EventStore::adopt_custom` keeps the data
    /// already stored when re-adopting the same id, and replaces it outright
    /// when the id differs, so this refreshes rather than discards what has
    /// arrived.
    fn adopt_custom_event(&mut self) -> Task<Message> {
        let parsed = match custom_site::parse_custom_site(&self.config.custom_site.url) {
            Ok(parsed) => parsed,
            Err(e) => {
                // Only reachable from a hand-edited config file: every path that
                // commits a URL validates it first.
                error!("{}", unusable_saved_address_log_line(e, "no event adopted"));
                return Task::none();
            }
        };
        let event_id = parsed.event_id;
        let event_changed = self.current_event_id.as_ref() != Some(&event_id);

        self.events.adopt_custom(Event {
            id: event_id.clone(),
            // Never shown in a picker: a custom site offers no event list, so
            // `EventStore::selectable` answers `None` for it. The id keeps the
            // entry recognisable in a log rather than blank.
            name: event_id.partial().to_string(),
            slug: String::new(),
            // A custom site serves no event-level date range, and the only
            // reader sorts the portal's picker — unreachable for this event.
            // Both ends are set to now rather than invented.
            date_range: DateRange {
                start: time::OffsetDateTime::now_utc(),
                end: time::OffsetDateTime::now_utc(),
            },
            teams: None,
            schedule: None,
            courts: None,
        });

        // A different event invalidates the schedule anchor too: custom sites
        // deliberately reuse portal-style game numbering, so a `last_played`
        // carried over from the previous site would still name a real game —
        // just the wrong one (decision 25). This must run before
        // `set_current_event_id` below commits the new id, or the comparison
        // inside finds old == new and never fires. Mirrors the court value
        // this function is about to commit: cleared with the schedule when
        // the event changes, otherwise left as it is.
        let court_after_adopt = if event_changed {
            None
        } else {
            self.current_court.clone()
        };
        self.clear_anchor_if_event_or_court_changing(&Some(event_id.clone()), &court_after_adopt);

        // Route through set_current_event_id so portal_event_id stays in sync
        // for the background health check (ADR 011 amendment 2026-04-23).
        self.set_current_event_id(Some(event_id.clone()));
        // A different event invalidates the court and game chosen against the
        // previous one — the portal's event picker clears them for exactly this
        // reason (`EditableSettings::select_event`). Left in place, a court from
        // the old event filters the new event's game list down to nothing, and
        // the operator is left staring at an empty SELECT GAME with no
        // explanation. Clearing also lets the single-court auto-adopt fire,
        // which needs `current_court` to be `None`.
        if event_changed {
            self.current_court = None;
            self.schedule = None;
        }
        if let Some(ref mut edits) = self.edited_settings {
            if event_changed {
                edits.select_event(event_id.clone());
            } else {
                edits.current_event_id = Some(event_id.clone());
            }
        }
        info!("Adopted custom site event {}", event_id.full());
        Task::batch(vec![
            self.request_teams_list(event_id.clone()),
            self.request_schedule(event_id),
        ])
    }

    /// Move the refbox onto `target`, clearing the link that belonged to the
    /// site it is leaving.
    ///
    /// The one act behind both paths a source-button tap can take — straight
    /// through when there is nothing to lose, and through the confirmation when
    /// a game is fully linked. Keeping it in one place is what stops the two
    /// from drifting apart.
    ///
    /// This applies ADR 017's 2026-06-23 rule — switching the portal off is a
    /// clean wipe — to a switch between the two remote sources, for the same
    /// reason: a leftover start time from the site the refbox has left, silently
    /// driving the countdown, is confusing.
    ///
    /// The caller's obligation: `source_tap_outcome` has already cleared this.
    /// The two refusals are not re-checked here, and the period is therefore
    /// `BetweenGames`.
    fn switch_to_source(&mut self, target: GameSource) -> Task<Message> {
        // 1. The source itself: live, saved so a relaunch comes back here, and
        //    remembered as the remote to return to from MANUAL.
        self.commit_source(target);

        // 2. The live client follows. `target_after_apply` answers `None` both
        //    when the client is already there and when a custom site has no
        //    usable address saved — the second leaves the client where it is
        //    rather than pointing it at nothing, and the operator then types an
        //    address on the SITE page whose APPLY moves it.
        let mode = self.config.mode;
        // Cloned so the immutable borrow of `self.config` ends before
        // `repoint_client` takes `&mut self`. Mirrors the same clone in the
        // `ConfigPage::CustomSite` arm of `ApplyConfigPage`.
        let custom_site = self.config.custom_site.clone();
        if let Some(site) = self.target_after_apply(target, mode, &custom_site) {
            self.repoint_client(site);
        }

        // 3. Clear the link on both sides — what is committed and what the
        //    editor is showing. The selection belonged to the site being left;
        //    carrying it across is the leak this whole change exists to close.
        //    Route through set_current_event_id so portal_event_id stays in sync
        //    for the background health check (ADR 011 amendment 2026-04-23).
        self.set_current_event_id(None);
        self.current_court = None;
        self.schedule = None;
        // The player-number grid is otherwise only rewritten at kickoff, so
        // without this it would keep showing the previous site's cap numbers
        // until the next game starts. `clear_portal_selections_to_manual`
        // clears it for exactly this reason when the portal is switched off.
        self.game_rosters = BlackWhiteBundle {
            black: Vec::new(),
            white: Vec::new(),
        };
        // The roster cache behind that grid has to go with it. It is keyed by
        // team id, and a team id is whatever text the server chose to send, so
        // ids collide across sites exactly as event ids do. Two things go wrong
        // if it survives: `rosters_for_game` can seed the grid at the next
        // kickoff from the departed site's entry, and `RecvSchedule` skips
        // fetching any roster it already holds — so the stale entry would
        // shadow the real one for the rest of the session, surviving a REFRESH.
        self.team_rosters.clear();
        // A startup link note that has not been consumed yet named the site the
        // refbox is leaving. Left set, `RecvEventList` would take
        // `pending_restore_schedule` and ask for that event through the live
        // client — the one step 2 has just repointed — sending the old site's
        // event id to the new site; and `pending_restore_game` would later force
        // the old site's game number onto whatever the operator picks here, on
        // the first schedule that arrives once they leave the editor.
        self.pending_restore_game = None;
        self.pending_restore_schedule = None;
        // `commit_source` has already recorded the new remote in `config`; the
        // editor's copy has to follow or the MANUAL GAMES button still offers to
        // return to the source the refbox has left, which leaves one source
        // button highlighted wrongly and the other inert.
        let remembered = self.config.remembered_remote;
        if let Some(ref mut edited) = self.edited_settings {
            edited.source = target;
            edited.remembered_remote = remembered;
            // Clears the staged event, court, schedule and game number, and
            // resets the token indicator to its rejected state. With no event
            // staged the row renders blank and inert either way
            // (`token_row_actionable`), but the flag still matters: it keeps
            // `token_rejected` true, which is what holds the COURT picker
            // greyed until a credential has actually been checked.
            edited.clear_for_remote_switch();
        }

        // 4. The clock and the game number — and NOT the game configuration.
        //    The portal-off path pairs `reset_to_manual_break` with
        //    `set_config`, because it is going manual. Here the refbox stays on
        //    a remote source, so installing a manual game config would silently
        //    overwrite the operator's settings. Calling only this half is the
        //    whole point.
        //
        //    why this cannot panic: `reset_to_manual_break` takes no fallible
        //    path — it clears the next-game info, sets the game number and clock
        //    directly, and starts the clock.
        self.tm.lock().reset_to_manual_break(Instant::now());

        // 5. What the new site owes us.
        let mut task = match target {
            // The portal's event list is loaded whatever the source (ADR 017
            // amendment 2026-08-27), so the picker is usable the moment the
            // operator lands on it. Refreshed anyway, and deliberately not
            // conditioned on the previous source: an operator arriving here
            // wants the current list, and a stale one is what sends them to a
            // game that no longer exists.
            //
            // Deliberately NOT guarded on the live client's site, unlike the
            // Custom arm below: `request_event_list` builds its own portal
            // client from `portal_target` rather than using the live one, so it
            // always reaches the portal and cannot be misrouted. Guarding it
            // would only suppress a refresh that would have worked — and it
            // would defeat the ADR 017 amendment above, whose whole point is
            // that this list loads whatever the source.
            GameSource::Portal => self.request_event_list(),
            // A custom site names its event in the URL, so there is nothing to
            // pick — adopt it and pull its teams and schedule. Guarded because
            // step 2 leaves the client where it was when there is no usable
            // address: asking the portal for a custom site's event is the exact
            // mismatch `site_serves` exists to prevent.
            GameSource::Custom => {
                if site_serves(self.current_site.kind, GameSource::Custom) {
                    self.adopt_custom_event()
                } else {
                    Task::none()
                }
            }
            // Not reachable: the two source buttons are the only senders and
            // neither offers Manual. MANUAL GAMES keeps its own staged path.
            GameSource::Manual => Task::none(),
        };

        // 6. The saved credential is per-site, so the verdict the ACCESS TOKEN
        //    row reached about the old site says nothing about this one. After
        //    the adoption above, not before: for a custom site the adoption is
        //    what puts an event there to check the credential against, and
        //    without one this can only answer rejected.
        task = Task::batch(vec![task, self.refresh_token_indicator()]);

        self.persist_config();
        // The note follows whatever the switch left linked. A switch to the
        // portal adopts no event, so this takes its delete branch and a
        // relaunch starts dormant rather than silently re-linking the event
        // that belonged to the site the refbox has left. A switch to a custom
        // site whose address names an event has just adopted it above, so this
        // rewrites the note for that event and a relaunch returns there.
        self.persist_link_session();
        // The switch is committed and cannot be taken back, so the Game page's
        // CANCEL must not offer to put the old source and selection back into
        // the editor. Re-capturing makes the new state that page's baseline.
        self.capture_snapshot_for(ConfigPage::Game);
        task
    }

    fn check_uwhportal_auth(&self, event_id: &EventId) -> Task<Message> {
        if self.uwhportal_client.is_some() {
            // The site this check goes out against, carried on the reply. Read
            // here, at the moment of issue, because that is the only point at
            // which the answer is known.
            let issued_at = self.site_generation;
            // Asked of the store, about *this* event. The shared client's key belongs to the
            // linked event and answers a different question.
            let has_token =
                key_for_event(&self.config, self.current_site.base_url.expose(), event_id)
                    .is_some();
            if !has_token {
                // Never ask a site to vouch for a credential we do not hold.
                // Only the site can enforce a token, and a permissive one
                // answers an unauthenticated probe with `200` — which arrives
                // as a green "Connected" painted over nothing. Report the
                // rejected state here instead, without sending the request.
                return Task::done(Message::RecvTokenValid(event_id.clone(), false, issued_at));
            }
            let Some(client) = client_for_event(&self.current_site, &self.config, event_id) else {
                return Task::done(Message::RecvTokenValid(event_id.clone(), false, issued_at));
            };
            let request = client.verify_token(event_id);
            // Tag the result with the event it was checked for so the handler
            // can drop a late reply for a previously-selected event.
            let event_id = event_id.clone();
            Task::future(async move {
                match request.await {
                    Ok(()) => {
                        info!("Portal token validated");
                        Message::RecvTokenValid(event_id, true, issued_at)
                    }
                    Err(e) => {
                        error!("Portal token validity check failed: {e}");
                        Message::RecvTokenValid(event_id, false, issued_at)
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    /// Both teams' cap numbers for a scheduled game on this court, read from the
    /// session cache. See `rosters_for_scheduled_game`.
    fn rosters_for_game(&self, game_num: &GameNumber) -> BlackWhiteBundle<Vec<u8>> {
        rosters_for_scheduled_game(
            self.schedule.as_ref(),
            &self.team_rosters,
            self.current_court.as_deref(),
            game_num,
        )
    }

    fn handle_game_start(&mut self, new_game_num: &GameNumber) -> Task<Message> {
        // A remembered game exists only to put the operator back where they were at
        // startup. Once a game has actually started, it is stale: leaving it in place
        // lets it fire on the end-of-game schedule refresh hours later and re-adopt an
        // old game, which would silently undo the finished-court state.
        self.pending_restore_game = None;

        // Fix this game's rosters now. From here until the next kickoff they do
        // not change: a REFRESH mid-game re-pulls the event, but must not move
        // the grid under the operator's hand.
        self.game_rosters = self.rosters_for_game(new_game_num);

        // An empty pin means the grid never appears for this whole game, and the
        // three causes -- no roster fetched, a placeholder team slot, or a game
        // that is not this court's -- look identical on screen. Say which, once,
        // at kickoff. Deliberately here and not in the lookup itself: that runs
        // on every frame while the picker is open, and a warning there would
        // flood the rolling log and roll the useful history away.
        if self.schedule.is_some()
            && self.game_rosters.black.is_empty()
            && self.game_rosters.white.is_empty()
        {
            warn!(
                "No cap numbers pinned for game {new_game_num} (court {:?}); the player grid will \
                 show the number pad for this game",
                self.current_court,
            );
        }

        let mut tasks: Vec<Task<Message>> = Vec::new();

        if self.uses_remote() {
            debug!("Searching for next game info after game {new_game_num}");
            if let (Some(schedule), Some(pool)) = (&self.schedule, &self.current_court) {
                let this_game_start = match schedule.games.get(new_game_num) {
                    Some(g) => g.start_time,
                    None => {
                        error!("Could not find new game's start time (game {new_game_num}");
                        return Task::batch(tasks);
                    }
                };

                let next_game = schedule.next_game_on_court(pool, this_game_start);

                let mut tm = self.tm.lock();
                let next_game_number = if let Some(next_game) = next_game {
                    let timing = schedule.get_game_timing(&next_game.number).cloned();
                    let info = NextGameInfo {
                        number: next_game.number.clone(),
                        timing,
                        start_time: Some(next_game.start_time),
                    };
                    tm.set_next_game(info);
                    Some(next_game.number.clone())
                } else {
                    // Definite answer, not a failure: this court's schedule is finished.
                    // Recording it stops the engine guessing another court's game.
                    info!("No games scheduled on court {pool} after game {new_game_num}");
                    tm.set_no_next_game();
                    None
                };
                self.config.game = tm.config().clone();
                // `roster_refresh_tasks` borrows `self`, so the `tm` lock (a
                // borrow of `self.tm`) must be released first.
                drop(tm);

                if let Some(next_game_number) = next_game_number {
                    tasks.extend(self.roster_refresh_tasks(&next_game_number));
                }
            }
        } else {
            debug!("Skipped next game info search after game {new_game_num}");
        }

        Task::batch(tasks)
    }

    /// Re-pull both teams' rosters for an upcoming game. Fired at the kickoff of
    /// the previous game so the fetch has the whole break to land rather than
    /// the instant of the next start. A failure changes nothing: the fetch
    /// reports `NoAction` and the cached copy stands.
    fn roster_refresh_tasks(&self, game_num: &GameNumber) -> Vec<Task<Message>> {
        let mut tasks = Vec::new();

        if let Some(schedule) = &self.schedule {
            if let Some(game) = schedule.games.get(game_num) {
                for team in [&game.dark, &game.light] {
                    if let Some(id) = team.assigned() {
                        tasks.push(self.request_team_roster(id.clone()));
                    }
                }
            }
        }

        tasks
    }

    fn handle_game_end(&mut self, game_number: &GameNumber) -> Task<Message> {
        let mut tasks = vec![];
        if self.uses_remote() {
            // Copy everything needed out from under the lock: the recorded result's own
            // game number, its scores, and its stats JSON.
            let recorded = {
                let tm = self.tm.lock();
                tm.last_game_info()
                    .map(|info| (info.game_number.clone(), info.scores, info.stats.as_json()))
            };

            match recorded {
                Some((recorded_game, scores, stats))
                    if recorded_result_matches_ended_game(Some(&recorded_game), game_number) =>
                {
                    info!("Game ended, scores: {scores:?} stats were: {stats:?}");

                    let scheduled_start = self
                        .schedule
                        .as_ref()
                        .and_then(|s| s.games.get(game_number))
                        .map(|g| g.start_time);
                    let (anchor, anchor_start) = anchor_after_game_end(
                        Some(&recorded_game),
                        game_number,
                        scheduled_start,
                        (self.last_played.clone(), self.last_played_start),
                    );
                    self.last_played = anchor;
                    self.last_played_start = anchor_start;

                    if let Some(ref event_id) = self.current_event_id {
                        let event_id_str = event_id.full().to_string();
                        tasks.push(self.request_schedule(event_id.clone()));
                        if game_number.is_empty() {
                            warn!("Game ended with no game number; not posting to the portal");
                        } else if let Err(e) = self.portal_manager.enqueue_game_end(
                            event_id_str,
                            game_number.to_string(),
                            scores.black,
                            scores.white,
                            stats,
                        ) {
                            error!("portal_manager.enqueue_game_end failed: {e}");
                        }
                    } else {
                        error!("Missing current event id to handle game end");
                    }
                }
                Some((recorded_game, _, _)) => {
                    warn!(
                        "Clock left game {game_number} without a result being recorded for it \
                         (the newest recorded result belongs to game {recorded_game}); nothing \
                         sent to the portal"
                    );
                }
                None => {
                    warn!(
                        "Clock left game {game_number} with no recorded result available; \
                         nothing sent to the portal"
                    );
                }
            }

            // Write the anchor down now. Acceptance criterion 2 closes and reopens
            // the app seconds after the last game ends; the health-tick heartbeat
            // that normally refreshes the note is ~5 minutes away. Kept inside
            // `uses_remote()`: outside it, manual mode would reach
            // `persist_link_session`, whose not-linked branch deletes the note —
            // a delete reachable from the game-clock path for no benefit.
            self.persist_link_session();
        }

        Task::batch(tasks)
    }

    /// Update `current_event_id` and mirror the new value into the
    /// `portal_event_id` shared handle so the background portal-health
    /// task sees it on its next tick. Every per-page apply that writes
    /// `current_event_id` should route through this so the tile's
    /// `verify_token` leg reflects the operator's actual event selection
    /// (ADR 011 amendment 2026-04-23, dormant-until-linked).
    fn set_current_event_id(&mut self, new: Option<EventId>) {
        #[cfg(debug_assertions)]
        let new_is_some = new.is_some();
        self.current_event_id = new.clone();
        // why this cannot panic: the guarded data is a plain `Option`
        // and no writer panics while holding the guard; a poisoned
        // mutex just returns the previous value, which we then
        // overwrite.
        *self
            .portal_event_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = new;
        // Must run after the event is published: the scramble exists to exercise
        // token *rejection*, and its own log line says the replacement
        // happens "after event linked" -- so the scramble has to have the
        // last word, or the debug flag would silently stop doing its job.
        // The `is_empty` guard matters: after an upgrade the store is empty until the first
        // login, and spending the one-shot flag there would replace nothing, then exercise the
        // *missing key* path rather than the rejection path the flag exists for.
        #[cfg(debug_assertions)]
        if self.scramble_token_pending && new_is_some && !self.config.access_keys.is_empty() {
            // Scrambles the *published* store only, not `config.access_keys`. The background
            // task loads a key per call from the published view, so this is what makes its probe
            // exercise rejection; replacing the client's token instead would be overwritten
            // before the next probe.
            //
            // Known limitation, deliberate: the settings ACCESS TOKEN row reads the config, so it
            // stays green while the background probe goes red, and the next repoint or login
            // republishes the real keys and undoes this. Scrambling the config itself would risk
            // `persist_config` writing a scrambled key to the operator's settings file, which is
            // not a trade worth making for a debug flag.
            let scrambled: Vec<_> = self
                .config
                .access_keys
                .iter()
                .map(|k| crate::config::AccessKey {
                    site: k.site.clone(),
                    event: k.event.clone(),
                    // A literal printable-ASCII key, so the check cannot refuse it; this is the
                    // debug scramble path, not an operator's key.
                    key: "invalid-debug-token".to_string(),
                })
                .collect();
            // why this cannot panic: see `publish_access_keys`.
            self.portal_access_keys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .publish(self.current_site.base_url.expose().to_string(), scrambled);
            warn!(
                "UWH_PORTAL_SCRAMBLE_TOKEN: background key view scrambled after event linked \
                 (settings row still reads the real config; republished on the next repoint)"
            );
            self.scramble_token_pending = false;
        }
    }

    /// Write or delete `portal_link.json` to reflect the current live link.
    /// Linked (portal on + an event selected) → write a note stamped now so a
    /// relaunch or short shutdown can re-establish the link. Not linked →
    /// delete any existing note. Errors are logged, never fatal: a failed note
    /// write only means a future restart won't auto-relink.
    fn persist_link_session(&self) {
        use crate::portal_manager::link_session::{self, LinkSessionFile};
        if self.uses_remote() {
            if let Some(event_id) = self.current_event_id.clone() {
                // The game the operator is on, taken from the live engine — see
                // `link_note_game` for why the cached snapshot cannot be trusted
                // here, and why "I don't know yet" must not be written down.
                let game_number = match link_note_game(&self.tm.lock()) {
                    LinkNoteGame::Write(game_number) => game_number,
                    // Nothing is known yet, so there is nothing worth saying.
                    // Returning leaves the existing note untouched — including a
                    // mid-event resume point we would otherwise wipe seconds
                    // before the schedule arrives to confirm it.
                    LinkNoteGame::Unknown => return,
                };
                let note = LinkSessionFile {
                    version: LinkSessionFile::CURRENT_VERSION,
                    event_id,
                    court: self.current_court.clone(),
                    current_game: game_number,
                    last_played: self.last_played.clone(),
                    last_played_start: self.last_played_start,
                    mode: self.config.mode,
                    last_active: time::OffsetDateTime::now_utc(),
                };
                if let Err(e) = link_session::save(&self.config_dir, &note) {
                    error!("Failed to write portal_link.json: {e}");
                }
                return;
            }
        }
        if let Err(e) = link_session::delete(&self.config_dir) {
            error!("Failed to delete portal_link.json: {e}");
        }
    }

    /// Clear the last-played anchor when an Apply is about to repoint the live
    /// event or court. Must be called against the live values, before the new
    /// ones are committed: the anchor is per-event and per-court, and a
    /// carried-over anchor points at a real but wrong game and looks entirely
    /// plausible (decision 25).
    fn clear_anchor_if_event_or_court_changing(
        &mut self,
        new_event_id: &Option<EventId>,
        new_court: &Option<String>,
    ) {
        if self.current_event_id != *new_event_id || self.current_court != *new_court {
            self.last_played = None;
            self.last_played_start = None;
        }
    }

    fn apply_app_options(&mut self) -> Option<ConfirmationKind> {
        // The operator has made their own choice; the remembered game is spent.
        // First statement deliberately: reaching this function at all means APPLY
        // was pressed, so the note is spent whichever branch runs below —
        // including the ones that return early to raise a confirmation page.
        self.pending_restore_game = None;

        let edited = self.edited_settings.as_ref()?;
        // Snapshot the fields we need so the immutable borrow on
        // `edited_settings` ends before we call `set_current_event_id`
        // (which takes `&mut self`).
        let source = edited.source;
        let mode = edited.mode;

        // Cross-portal Mode change requires explicit confirmation and an app
        // restart. Raise the confirmation before committing any fields so that
        // cancelling rolls back the entire Apply (Option A semantics, matching
        // GameConfigChangedFromApply). The commit happens in the RestartAndApply
        // branch of the confirmation handler, which calls the same
        // `commit_app_toggles` used below so the two cannot drift.
        if crosses_portal(self.config.mode, mode) {
            return Some(ConfirmationKind::PortalTenantSwitch {
                from_mode: self.config.mode,
                to_mode: mode,
                source,
            });
        }

        // This is the second commit site for `current_event_id` / `current_court`
        // / `schedule` — `apply_game_options` is the first. The ownership check
        // is NOT here because a switch leaves those fields staged: it no longer
        // does, `switch_to_source` clears them. It is here because more than one
        // site commits these fields and each must vouch for them independently,
        // and because `owns` alone cannot prove provenance — custom sites are
        // required to reuse portal event numbering, so the same id resolves in
        // either store, which is why it is ANDed with `site_serves`. Without this guard: commit Custom, stage
        // Portal on the Game page, APPLY there (which correctly nulls the commit
        // because the selection isn't owned), then reopen App Options and APPLY
        // again — that second Apply would re-commit the still-staged custom event
        // id under Portal and write it into portal_link.json as a Portal link,
        // sending the real UWH Portal a request for an event only the operator's
        // own server has. Mirrors `selection_owned` in `apply_game_options`.
        let client_serves_staged = site_serves(self.current_site.kind, source);
        let selection_owned =
            self.events.owns(source, edited.current_event_id.as_ref()) && client_serves_staged;
        // Three cases, and the last is the fix for a link-losing bug.
        //
        //   owned         -> commit the staged selection.
        //   staged Manual -> clear it; going manual means there is no link.
        //   otherwise     -> LEAVE THE COMMITTED LINK ALONE.
        //
        // That last arm used to null all three fields. `owns` is a store lookup,
        // so it answers false for a perfectly good saved event whenever the
        // portal event list has not loaded — a Pi whose wifi comes up after
        // refbox starts, with a link note restored at startup. An APPLY for any
        // unrelated App toggle then wiped the committed link, and
        // `persist_link_session` took its delete branch and removed
        // portal_link.json with it. Declining to commit keeps an unowned
        // selection out of the committed state just as effectively, without
        // destroying one that is already there and correct.
        //
        // Built while `edited` is still borrowed, because `commit_link_selection`
        // takes `&mut self`. `LinkSelection::manual()` fits the Clear arm exactly:
        // that arm only fires when the source being applied IS Manual.
        let commit_link = match link_commit(selection_owned, source) {
            LinkCommit::Staged => Some(LinkSelection::from_edited(edited)),
            LinkCommit::Clear => Some(LinkSelection::manual()),
            LinkCommit::Leave => None,
        };

        // Committed here, while `edited` is still borrowed, so the seven toggles
        // need no per-field locals. `config` and `edited_settings` are disjoint
        // fields, so this mutable borrow and the immutable one above coexist.
        let hide_time_changed = commit_app_toggles(&mut self.config, edited);

        // Switching to manual from this page must not leave the engine holding remote
        // next-game state: it would keep reporting a blank next-game number, which greys
        // START NOW and would strand the operator in manual mode. The Game page does this
        // via reset_to_manual_break; here the minimal clear is enough, since manual mode
        // resumes its own numbering from "unknown". Read the outgoing source before
        // `commit_source` overwrites it.
        //
        // Detection has to happen here, ahead of `commit_source`: "no next game" is
        // exactly what `commit_source` is about to stop being true, so read the state
        // the operator is escaping from before it is unwound. The clock work that acts
        // on it waits until after, for the same reason in reverse — see below.
        let escaping_finished_state = if self.uses_remote() && matches!(source, GameSource::Manual)
        {
            let mut tm = self.tm.lock();
            // In the finished state the break clock is held stopped at 0:00, so no
            // snapshot is generated again on its own. Clearing the engine alone would
            // leave the main page reading the stale blank number, with START NOW still
            // greyed — the very trap the operator switched the portal off to escape.
            let escaping = tm.current_period() == GamePeriod::BetweenGames
                && tm.next_game_number().is_empty()
                && !tm.clock_is_running();
            tm.clear_portal_next_game();
            escaping
        } else {
            false
        };

        match commit_link {
            Some(link) => self.commit_link_selection(link),
            // Leave: commit the source, but do NOT touch the committed link.
            // Nulling it here destroyed a restored link whenever the portal
            // event list had not loaded.
            None => self.commit_source(source),
        }

        // Now, and not before: `commit_source` is what tells the engine it is no longer
        // tied to a site's schedule, and a still-linked engine refuses to start a clock
        // toward a game no schedule has named — so a `start_clock` run ahead of it would
        // leave the break frozen at the nominal time, the snapshot still carrying a blank
        // next-game number, and the update loop spinning on a stopped clock it believes
        // is running.
        if escaping_finished_state {
            let now = Instant::now();
            let snapshot = {
                let mut tm = self.tm.lock();
                let nominal_break = tm.config().nominal_break;
                // why this cannot panic: `escaping_finished_state` requires a stopped
                // clock, nothing between there and here starts one, and
                // `set_game_clock_time` only errors while the clock is running.
                tm.set_game_clock_time(nominal_break).unwrap();
                tm.start_clock(now);
                tm.generate_snapshot(now)
            };
            if let Some(snapshot) = snapshot {
                // Between games before and after, so `apply_snapshot` returns an
                // empty task; this page's Apply has no task to carry in any case.
                let _ = self.apply_snapshot(snapshot);
            }
        }
        self.config.mode = mode;
        // `hide_time` is mirrored to the update server, which the pure helper
        // cannot do — so notify here, and only when it actually changed.
        // Bounded channel with one message per Apply from the GUI loop, so it
        // cannot be full; Closed means the update-server task is gone
        // (application-fatal), matching the other unwrap sites.
        if hide_time_changed {
            self.update_sender
                .set_hide_time(self.config.hide_time)
                .unwrap();
        }
        None
    }

    fn apply_display_options(&mut self) {
        let Some(edited) = self.edited_settings.as_ref() else {
            return;
        };
        self.config.hardware.white_on_right = edited.white_on_right;
        self.config.hardware.brightness = edited.brightness;
        self.config.front_display_layout = edited.front_display_layout;
        // A real LED panel always renders Default; mirror the button's gating so
        // Apply never pushes a full-screen layout to the hardware path.
        let effective = if self.has_led_panel {
            crate::sim_frame::FrontDisplayLayout::Default
        } else {
            edited.front_display_layout
        };
        // Same bounded channel as set_hide_time: one message per Apply from the
        // GUI loop, so it cannot be full; Closed means the update-server task is
        // gone (application-fatal), matching the existing unwrap at that site.
        self.update_sender.set_layout(effective).unwrap();
    }

    fn apply_sound_options(&mut self) {
        let Some(edited) = self.edited_settings.as_ref() else {
            return;
        };
        self.config.sound = edited.sound.clone();
        self.sound.update_settings(self.config.sound.clone());
    }

    fn apply_remote_options(&mut self) {
        let Some(edited) = self.edited_settings.as_ref() else {
            return;
        };
        self.config.sound.remotes = edited.sound.remotes.clone();
        self.sound.update_settings(self.config.sound.clone());
    }

    /// Clear the on-screen portal selections back to a fresh manual slate.
    ///
    /// The TM-side reset (clock + next-game) is done separately by the caller via the
    /// engine routines (`reset_to_manual_break` / `clear_portal_next_game`).
    /// Does NOT clear `config.uwhportal.token` (no logout).
    /// The `manual_config` parameter is the `GameConfig` that should be persisted as the
    /// active game config; pass it in as a snapshot captured before the `edited` borrow
    /// ends (mirrors the borrow choreography of the config-change branch).
    fn clear_portal_selections_to_manual(&mut self, manual_config: GameConfig) {
        self.commit_link_selection(LinkSelection::manual());
        // The Using-UWH-Portal setting is now off, which the player-grid design
        // spec lists as an unconditional number-pad condition. `game_rosters` is
        // otherwise only written at kickoff, so without this the grid would keep
        // showing the previous event's numbers until the next kickoff.
        self.game_rosters = BlackWhiteBundle {
            black: Vec::new(),
            white: Vec::new(),
        };
        self.config.game = manual_config;
        self.persist_config();
        // Delete the saved portal-link note now that the portal is off, so a
        // later restart starts dormant instead of silently re-linking the old
        // event. Mirrors the Apply path's `persist_config(); persist_link_session();`
        // pairing: the mid-game switch-to-manual confirmation returns early into
        // a ConfirmationPage and never reaches that Apply-path call, so without
        // this the note would survive and the restore on next launch would turn
        // the portal back on (finding H3). With the portal now off and no event
        // selected, `persist_link_session` takes its delete branch.
        self.persist_link_session();
    }

    /// Commit the Game-Options slice (game config + game number) to the live state.
    ///
    /// Returns `Some(ConfirmationKind)` when a safety gate fires (uwhportal-incomplete,
    /// game-config change mid-game, or game-number change mid-game) — the caller must
    /// route into a ConfirmationPage. Returns `None` when the commit happened directly
    /// (or there was nothing to commit).
    ///
    /// Unlike `apply_settings_change`, this does NOT clear `edited_settings` and does
    /// NOT touch other slices — the user is still inside settings and may have unrelated
    /// edits to commit on other pages.
    fn apply_game_options(&mut self) -> Option<ConfirmationKind> {
        // The operator has made their own choice; the remembered game is spent.
        // First statement deliberately: reaching this function at all means APPLY
        // was pressed, so the note is spent whichever branch runs below —
        // including the ones that return early to raise a confirmation page.
        self.pending_restore_game = None;

        let edited = self.edited_settings.as_ref()?;

        // A store hit alone does not prove the selection came from the site the
        // client is actually pointed at — event ids collide across sites — so
        // `selection_owned` also requires the client to currently serve the
        // staged source.
        let client_serves_staged = site_serves(self.current_site.kind, edited.source);
        let selection_owned = self
            .events
            .owns(edited.source, edited.current_event_id.as_ref())
            && client_serves_staged;
        if game_apply_blocked(edited, selection_owned) {
            return Some(ConfirmationKind::UwhPortalIncompleteFromApply);
        }

        let mut tm = self.tm.lock();

        // Detect the ON→OFF portal transition.  At function entry `self.uses_remote()`
        // still holds the prior committed value, so a true→false change means the
        // operator just switched the portal toggle off.
        let switching_to_manual = self.uses_remote() && !edited.uses_remote();
        if switching_to_manual {
            if tm.current_period() != GamePeriod::BetweenGames {
                // Mid-game: surface the confirmation page; Task 4 will handle the actions.
                return Some(ConfirmationKind::SwitchToManualFromApply);
            }
            // Between games: commit the clean manual slate directly.
            // Safety: BetweenGames checked above; set_config only errors when a game is
            // in progress, so this path is unreachable.
            tm.set_config(edited.config.clone()).unwrap();
            let now = Instant::now();
            tm.reset_to_manual_break(now);
            // Snapshot the config we need before dropping the `edited` borrow so we can
            // call `&mut self` methods below (mirrors the existing config-change branch at
            // ~line 1049).
            let manual_config = edited.config.clone();
            std::mem::drop(tm);
            self.clear_portal_selections_to_manual(manual_config);
            return None;
        }

        // The `!selection_owned` arm just below is unreachable today:
        // `game_apply_blocked` now returns true whenever `uses_remote() &&
        // !selection_owned`, and this function already returned early on
        // that a few lines up, so a bare remote-to-remote switch never
        // reaches here. It stays anyway as a defensive fallback, not dead
        // code to delete — if that gate is ever loosened, `edited.schedule`
        // on such a switch still belongs to the site the refbox has not
        // moved to yet, and reading game timing out of it would commit the
        // other source's period lengths, breaks, and overtime/sudden-death
        // settings as this game's live config — the same leak the ownership
        // check exists to stop, just reached through `new_config` instead of
        // `current_event_id`. Falling back to the unchanged `tm.config()`
        // keeps a bare switch from changing the config at all; the operator's
        // next APPLY, after picking the new source's own event/court/game,
        // sets it properly.
        let new_config = if edited.uses_remote() {
            if selection_owned {
                edited
                    .schedule
                    .as_ref()
                    .and_then(|schedule| schedule.get_game_timing(&edited.game_number))
                    .cloned()
                    .map(|tr| tr.into())
                    .unwrap_or_else(|| tm.config().clone())
            } else {
                tm.config().clone()
            }
        } else {
            edited.config.clone()
        };

        if new_config != *tm.config() {
            if tm.current_period() != GamePeriod::BetweenGames {
                return Some(ConfirmationKind::GameConfigChangedFromApply(new_config));
            }
            // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
            tm.set_config(new_config.clone()).unwrap();

            let (game, timing) = edited
                .schedule
                .as_ref()
                .map(|schedule| schedule.get_game_and_timing(&edited.game_number))
                .unwrap_or((None, None));
            let start_time = game.map(|g| g.start_time);

            tm.set_next_game(NextGameInfo {
                number: edited.game_number.clone(),
                timing: timing.cloned(),
                start_time,
            });

            if edited.uses_remote() {
                // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
                tm.apply_next_game_start(Instant::now()).unwrap();
            } else {
                tm.clear_scheduled_game_start();
            }

            std::mem::drop(tm);
            // Snapshot the selection so the immutable borrow on `edited` ends
            // before `commit_link_selection` takes `&mut self`.
            //
            // Guarded on ownership, which `commit_link_selection` deliberately does
            // not do for itself: a selection belonging to the OTHER source must not
            // be committed under this one. `owns` is an id lookup, and custom sites
            // are required to reuse portal event numbering, so the same id resolves
            // in either store — which is why `selection_owned` pairs it with
            // `site_serves`. Committing regardless would put a custom site's event
            // under PORTAL, which is what sent the portal a request for an event
            // only the operator's own server has. The game config and next-game info
            // drawn from that same unowned schedule are guarded separately, by the
            // `new_config` and `game_number` checks above — together, nothing derived
            // from an unowned selection reaches the committed state.
            let link = if selection_owned {
                LinkSelection::from_edited(edited)
            } else {
                LinkSelection {
                    source: edited.source,
                    event_id: None,
                    court: None,
                    schedule: None,
                }
            };

            self.config.game = new_config;
            self.commit_link_selection(link);
            return None;
        }

        // Same leak, same fix, as the `new_config` guard above: on a bare
        // remote-to-remote switch the staged game number resolves against the
        // other source's schedule, not the one just committed. Skip this branch
        // entirely rather than build a NextGameInfo out of it.
        if edited.game_number != self.snapshot.game_number
            && (!edited.uses_remote() || selection_owned)
        {
            if tm.current_period() != GamePeriod::BetweenGames {
                return Some(ConfirmationKind::GameNumberChangedFromApply);
            }
            let next_game_info = if edited.uses_remote() {
                let (game, timing) = edited
                    .schedule
                    .as_ref()
                    .map(|schedule| schedule.get_game_and_timing(&edited.game_number))
                    .unwrap_or((None, None));
                NextGameInfo {
                    number: edited.game_number.clone(),
                    timing: timing.cloned(),
                    start_time: game.map(|g| g.start_time),
                }
            } else {
                NextGameInfo {
                    number: edited.game_number.clone(),
                    timing: None,
                    start_time: None,
                }
            };

            tm.set_next_game(next_game_info);

            if edited.uses_remote() {
                // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
                tm.apply_next_game_start(Instant::now()).unwrap();
            }
        }

        std::mem::drop(tm);
        // Snapshot the selection so the immutable borrow on `edited` ends
        // before `commit_link_selection` takes `&mut self`.
        //
        // Guarded on ownership, which `commit_link_selection` deliberately does
        // not do for itself: a selection belonging to the OTHER source must not
        // be committed under this one. `owns` is an id lookup, and custom sites
        // are required to reuse portal event numbering, so the same id resolves
        // in either store — which is why `selection_owned` pairs it with
        // `site_serves`. Committing regardless would put a custom site's event
        // under PORTAL, which is what sent the portal a request for an event
        // only the operator's own server has. The game config and next-game info
        // drawn from that same unowned schedule are guarded separately, by the
        // `new_config` and `game_number` checks above — together, nothing derived
        // from an unowned selection reaches the committed state.
        let link = if selection_owned {
            LinkSelection::from_edited(edited)
        } else {
            LinkSelection {
                source: edited.source,
                event_id: None,
                court: None,
                schedule: None,
            }
        };

        self.commit_link_selection(link);

        None
    }

    /// Handle the user's selection on a `*FromApply` ConfirmationPage. Mirrors the
    /// logic of `Message::ConfirmationSelected` for the global-Done variants, but
    /// commits only the Game slice and routes back into settings (not out to MainPage).
    fn apply_game_confirmation(&mut self, selection: ConfirmationOption) -> Task<Message> {
        // Dispatch the switch-to-manual confirmation to its own handler so its logic
        // (which does NOT involve a new_config carried in the kind) stays separate from
        // the GameConfigChanged / GameNumberChanged option arms below.
        if matches!(
            self.app_state,
            AppState::ConfirmationPage(ConfirmationKind::SwitchToManualFromApply)
        ) {
            return self.apply_switch_to_manual_confirmation(selection);
        }

        let new_config = if let AppState::ConfirmationPage(
            ConfirmationKind::GameConfigChangedFromApply(ref config),
        ) = self.app_state
        {
            Some(config.clone())
        } else {
            None
        };

        let mut task = Task::none();
        let app_state = match selection {
            ConfirmationOption::DiscardChanges => {
                // When cancelling a PortalTenantSwitch confirmation the operator was
                // on the App Options sub-page — return them there, not to Main.
                let landing = if matches!(
                    self.app_state,
                    AppState::ConfirmationPage(ConfirmationKind::PortalTenantSwitch { .. })
                ) {
                    ConfigPage::App
                } else {
                    ConfigPage::Main
                };
                self.revert_from_snapshot();
                // revert_from_snapshot() consumes the page-entry snapshot, so
                // re-capture one for the landing page. Without this the App
                // Options APPLY button compares against a missing snapshot,
                // reads "nothing changed", and stays greyed for every later
                // edit until the operator leaves and re-enters the page
                // (finding H1). Mirrors navigate_to_parent's re-capture; a
                // no-op for the Main landing, where capture_snapshot_for
                // early-returns for navigation-only pages.
                self.capture_snapshot_for(landing);
                AppState::EditGameConfig(landing)
            }
            ConfirmationOption::GoBack => AppState::EditGameConfig(ConfigPage::Game),
            ConfirmationOption::EndGameAndApply => {
                // Safety: *FromApply confirmations are only raised while edited_settings is Some; the invariant is enforced by apply_game_options.
                let edited = self.edited_settings.as_ref().unwrap();
                let link = LinkSelection::from_edited(edited);
                let mut tm = self.tm.lock();
                let now = Instant::now();
                tm.reset_game(now);
                if let Some(ref config) = new_config {
                    // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
                    tm.set_config(config.clone()).unwrap();
                }

                let (game, timing) = edited
                    .schedule
                    .as_ref()
                    .map(|schedule| schedule.get_game_and_timing(&edited.game_number))
                    .unwrap_or((None, None));
                let start_time = game.map(|g| g.start_time);

                tm.set_next_game(NextGameInfo {
                    number: edited.game_number.clone(),
                    timing: timing.cloned(),
                    start_time,
                });

                if edited.uses_remote() {
                    // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
                    tm.apply_next_game_start(now).unwrap();
                } else {
                    tm.clear_scheduled_game_start();
                }

                std::mem::drop(tm);
                if let Some(config) = new_config {
                    self.config.game = config;
                }
                self.page_entry_snapshot = None;
                // Safety: snapshot generation only fails before the tournament manager is initialised, which happens in RefBoxApp::new().
                let new_snapshot = self.tm.lock().generate_snapshot(now).unwrap();
                // `apply_snapshot` FIRST, before the incoming selections are
                // committed. It processes the OUTGOING game's snapshot — which
                // includes filing that game's result when the clock has just run
                // it out — and it files against whatever event is current at the
                // time. Committing first would file a finished game against the
                // event the operator is switching TO.
                //
                // Both apply arms of THIS handler are ordered this way. The two
                // in `apply_switch_to_manual_confirmation` still commit first,
                // inside `clear_portal_selections_to_manual` — there the commit
                // sets the source to Manual, which makes `handle_game_end` return
                // early, so nothing is filed either way. That is pre-existing and
                // unchanged here; this ordering is not a guarantee extending to
                // them.
                //
                // The stale schedule fetch this can leave behind is harmless, but
                // not because the response is dropped: `RecvSchedule` warns on a
                // mismatch and still caches the payload under the event it was
                // fetched for. It is the live `self.schedule` that is guarded by
                // an event-id check, so the outgoing event's schedule cannot
                // displace the incoming one.
                task = self.apply_snapshot(new_snapshot);
                // Before `persist_config`, which writes the source this sets.
                self.commit_link_selection(link);
                self.persist_config();
                // Paired with `persist_config` exactly as the live Apply path pairs
                // them. The note is what a relaunch restores from, so committing the
                // court in memory without writing it here would leave the next
                // restart re-adopting the old court.
                //
                // AFTER `apply_snapshot`, not before: the note takes its game number
                // from `self.snapshot`, which `apply_snapshot` is what refreshes.
                // Writing first pairs the incoming court with the OUTGOING game — a
                // note naming a game that does not exist on the court beside it.
                self.persist_link_session();
                AppState::EditGameConfig(ConfigPage::Main)
            }
            ConfirmationOption::KeepGameAndApply => {
                // Safety: *FromApply confirmations are only raised while edited_settings is Some; the invariant is enforced by apply_game_options.
                let edited = self.edited_settings.as_ref().unwrap();
                let link = LinkSelection::from_edited(edited);
                let mut tm = self.tm.lock();
                tm.set_game_number(&edited.game_number);
                // Safety: snapshot generation only fails before the tournament manager is initialised, which happens in RefBoxApp::new().
                let new_snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                std::mem::drop(tm);
                self.page_entry_snapshot = None;
                // `apply_snapshot` FIRST, before the incoming selections are
                // committed. It processes the OUTGOING game's snapshot — which
                // includes filing that game's result when the clock has just run
                // it out — and it files against whatever event is current at the
                // time. Committing first would file a finished game against the
                // event the operator is switching TO.
                //
                // Both apply arms of THIS handler are ordered this way. The two
                // in `apply_switch_to_manual_confirmation` still commit first,
                // inside `clear_portal_selections_to_manual` — there the commit
                // sets the source to Manual, which makes `handle_game_end` return
                // early, so nothing is filed either way. That is pre-existing and
                // unchanged here; this ordering is not a guarantee extending to
                // them.
                //
                // The stale schedule fetch this can leave behind is harmless, but
                // not because the response is dropped: `RecvSchedule` warns on a
                // mismatch and still caches the payload under the event it was
                // fetched for. It is the live `self.schedule` that is guarded by
                // an event-id check, so the outgoing event's schedule cannot
                // displace the incoming one.
                task = self.apply_snapshot(new_snapshot);
                // Before `persist_config`, which writes the source this sets.
                self.commit_link_selection(link);
                self.persist_config();
                // Paired with `persist_config` exactly as the live Apply path pairs
                // them. The note is what a relaunch restores from, so committing the
                // court in memory without writing it here would leave the next
                // restart re-adopting the old court.
                //
                // AFTER `apply_snapshot`, not before: the note takes its game number
                // from `self.snapshot`, which `apply_snapshot` is what refreshes.
                // Writing first pairs the incoming court with the OUTGOING game — a
                // note naming a game that does not exist on the court beside it.
                self.persist_link_session();
                AppState::EditGameConfig(ConfigPage::Main)
            }
            ConfirmationOption::RestartAndApply => {
                // Extract the proposed mode and source from the in-flight
                // PortalTenantSwitch state. This arm is only reachable when the
                // app_state is PortalTenantSwitch (the view builder only offers
                // RestartAndApply for that kind).
                let (to_mode, new_source) =
                    if let AppState::ConfirmationPage(ConfirmationKind::PortalTenantSwitch {
                        to_mode,
                        source,
                        ..
                    }) = self.app_state
                    {
                        (to_mode, source)
                    } else {
                        unreachable!("RestartAndApply is only offered by PortalTenantSwitch pages")
                    };

                // Capture the committed source BEFORE anything below overwrites
                // it. The queue flush must be decided on the source the queued
                // items were created under, not the one being applied — see the
                // flush comment below.
                let old_source = self.source;

                // apply_app_options raised this confirmation *before* committing
                // any field, so that Cancel rolls back the whole Apply. That
                // makes this arm the only place the operator's App-page edits can
                // be written, and the restart re-reads config from disk — so
                // anything skipped here is silently lost. The shared helper is
                // what keeps this list from drifting from the live Apply path.
                //
                // Safety: PortalTenantSwitch is only raised by apply_app_options,
                // which returns early unless edited_settings is Some, and the
                // confirmation page offers no action that clears it. Same
                // invariant as the KeepGameAndApply arm above.
                let edited = self.edited_settings.as_ref().unwrap();
                // The "hide_time changed" flag is deliberately dropped: the
                // update server is told at startup, and this process is about to
                // be replaced by a fresh one.
                let _ = commit_app_toggles(&mut self.config, edited);

                // Commit the new mode. The proposed mode was held only in the
                // ConfirmationKind variant and was never written to self.config, so
                // this is the first and only write.
                self.config.mode = to_mode;

                // Commit the source too, mirroring apply_app_options exactly.
                //
                // This is a no-op in every state reachable today: the source can
                // only be changed on the Game page, `edited.source` is seeded
                // from the committed source on entry, and every exit from a page
                // either commits it (the Apply path) or reverts it
                // (CancelConfigPage -> revert_from_snapshot, and the Game
                // snapshot carries `source`). So it always already equals
                // `self.source` here. Kept anyway, because dropping it restores
                // the very asymmetry between the two commit paths that this
                // helper pairing exists to remove — and it becomes a real loss
                // the day a source control appears on the App page.
                //
                // Safe without repointing the live client for the same reason
                // the mode is: this arm always ends in a restart, and the restart
                // rebuilds the client from the committed config. See the repoint
                // invariant in the ApplyConfigPage handler.
                //
                // Read from the confirmation variant rather than from `edited`
                // above, because that is the value the operator was shown and
                // confirmed. The two are equal in any case: the confirmation page
                // offers only Cancel and Restart, so nothing can edit
                // `edited_settings` while it is up.
                self.commit_source(new_source);

                // Clear the current event id. This unpins the portal-health background
                // task from the old tenant's event so it stops probing after restart.
                self.set_current_event_id(None);

                // Flush the portal retry queue. Items queued under the old portal
                // tenant cannot be delivered to the new tenant — discard them so the
                // restarted app starts with a clean queue.
                //
                // Only for the built-in portal, and only for the source the items
                // were queued under, which is why `old_source` is captured above.
                // A custom site is one address the operator typed and it does not
                // change with the mode, so results queued for it are still
                // deliverable to exactly that site after the restart; flushing
                // them would destroy real game results.
                if old_source == GameSource::Portal {
                    if let Err(e) = self.portal_manager.flush_queue_for_tenant_switch() {
                        error!("Failed to flush portal queue before restart: {e}");
                        // Continue with restart — the operator pressed Restart and we
                        // must not block. The queue will be treated as stale items for
                        // the new tenant, which the retry logic will eventually discard.
                    }
                }

                // Persist the new mode to disk so the restarted exe reads it.
                if let Err(e) = confy::store(APP_NAME, None, &self.config) {
                    error!("Failed to persist config before restart: {e}");
                    // Continue with restart anyway — the operator pressed Restart.
                }

                // Kill every simulator child so they do not linger as orphans
                // after the iced runtime closes its windows.
                for mut child in self.sim_children.drain(..) {
                    let _ = child.kill();
                }
                // Mark the restart and let iced gracefully close its windows.
                // `main()` will spawn a fresh copy of the exe after the iced
                // runtime returns — this avoids the brief overlap of old and
                // new windows that a synchronous `std::process::exit(0)` would
                // otherwise produce. Mirrored in `Message::LanguageSelectComplete`.
                RESTART_PENDING.store(true, Ordering::Relaxed);
                task = iced::exit();
                AppState::MainPage
            }
            ConfirmationOption::SwitchSource => {
                unreachable!(
                    "SwitchSource is only offered by the SourceSwitchClearsSelection page, \
                     which is dispatched before this function is reached."
                )
            }
        };
        self.app_state = app_state;
        trace!("AppState changed to {:?}", self.app_state);
        task
    }

    /// Handles operator responses to `ConfirmationKind::SwitchToManualFromApply`.
    ///
    /// This confirmation is raised when the operator turns the portal toggle OFF while
    /// a game is in progress.  The four options mean:
    ///
    /// - `EndGameAndApply` — end the current game, reset to a clean manual slate.
    /// - `KeepGameAndApply` — leave the running game untouched; only clear the portal
    ///   next-game/grid so the break after this game uses the nominal break duration
    ///   from the manual config.
    /// - `DiscardChanges` — revert the edited settings and return to the main settings
    ///   page (mirrors the shared DiscardChanges arm).
    /// - `GoBack` — return to the Game sub-page without committing anything (mirrors
    ///   the shared GoBack arm).
    fn apply_switch_to_manual_confirmation(
        &mut self,
        selection: ConfirmationOption,
    ) -> Task<Message> {
        let mut task = Task::none();
        let app_state = match selection {
            ConfirmationOption::EndGameAndApply => {
                // Snapshot the manual config before acquiring the TM lock so the
                // borrow on `edited_settings` is released before the `&mut self`
                // call to `clear_portal_selections_to_manual`.
                //
                // Safety: *FromApply confirmations are only raised while
                // `edited_settings` is Some; the invariant is enforced by
                // `apply_game_options`.
                let manual_config = self.edited_settings.as_ref().unwrap().config.clone();
                let now = Instant::now();
                {
                    let mut tm = self.tm.lock();
                    tm.reset_game(now);
                    // `set_config` must run BEFORE `reset_to_manual_break` because
                    // `reset_to_manual_break` reads `self.config.nominal_break` to
                    // set the break clock.  After `reset_game` the period is
                    // `BetweenGames`, so `set_config` cannot error.
                    tm.set_config(manual_config.clone()).unwrap();
                    // Overrides reset_game's minimum-break clock with the nominal
                    // break; also resets the game number to "0", clears the
                    // next-game / grid, drops the schedule link, and starts the
                    // break counting down. `clear_portal_selections_to_manual`
                    // below commits the same manual source to the app, so the
                    // engine's view of linkage and the app's end up agreeing.
                    tm.reset_to_manual_break(now);
                }
                self.clear_portal_selections_to_manual(manual_config);
                // Clear the page-entry snapshot like the sibling apply arms so a
                // later path can't read a stale "has changes" state.
                self.page_entry_snapshot = None;
                // Safety: snapshot generation only fails before the tournament
                // manager is initialised, which happens in `RefBoxApp::new()`.
                let new_snapshot = self.tm.lock().generate_snapshot(now).unwrap();
                task = self.apply_snapshot(new_snapshot);
                AppState::EditGameConfig(ConfigPage::Main)
            }
            ConfirmationOption::KeepGameAndApply => {
                // The game keeps running.  Do NOT call set_config or
                // reset_to_manual_break — that would error (game in progress) or
                // disrupt the live clock.  Only clear the portal next-game/grid so
                // the break after this game uses the nominal break duration from the
                // manual config.
                //
                // Safety: *FromApply confirmations are only raised while
                // `edited_settings` is Some; the invariant is enforced by
                // `apply_game_options`.
                let manual_config = self.edited_settings.as_ref().unwrap().config.clone();
                {
                    let mut tm = self.tm.lock();
                    tm.clear_portal_next_game();
                }
                self.clear_portal_selections_to_manual(manual_config);
                // Clear the page-entry snapshot like the sibling apply arms so a
                // later path can't read a stale "has changes" state.
                self.page_entry_snapshot = None;
                // Safety: snapshot generation only fails before the tournament
                // manager is initialised, which happens in `RefBoxApp::new()`.
                let new_snapshot = self.tm.lock().generate_snapshot(Instant::now()).unwrap();
                task = self.apply_snapshot(new_snapshot);
                AppState::EditGameConfig(ConfigPage::Main)
            }
            ConfirmationOption::DiscardChanges => {
                self.revert_from_snapshot();
                AppState::EditGameConfig(ConfigPage::Main)
            }
            ConfirmationOption::GoBack => AppState::EditGameConfig(ConfigPage::Game),
            ConfirmationOption::RestartAndApply => {
                unreachable!("RestartAndApply is only offered by PortalTenantSwitch pages")
            }
            ConfirmationOption::SwitchSource => {
                unreachable!(
                    "SwitchSource is only offered by the SourceSwitchClearsSelection page, \
                     which is dispatched before this function is reached."
                )
            }
        };
        self.app_state = app_state;
        trace!("AppState changed to {:?}", self.app_state);
        task
    }

    fn capture_snapshot_for(&mut self, page: ConfigPage) {
        let Some(edited) = self.edited_settings.as_ref() else {
            return;
        };
        let snapshot = match page {
            ConfigPage::CustomSite(_) => PageEntrySnapshot::CustomSite {
                custom_site: edited.custom_site.clone(),
            },
            ConfigPage::Game => PageEntrySnapshot::Game {
                config: edited.config.clone(),
                game_number: edited.game_number.clone(),
                source: edited.source,
                current_event_id: edited.current_event_id.clone(),
                current_court: edited.current_court.clone(),
                schedule: edited.schedule.clone(),
            },
            ConfigPage::App => PageEntrySnapshot::App {
                source: edited.source,
                current_event_id: edited.current_event_id.clone(),
                current_court: edited.current_court.clone(),
                schedule: edited.schedule.clone(),
                mode: edited.mode,
                collect_scorer_cap_num: edited.collect_scorer_cap_num,
                track_fouls_and_warnings: edited.track_fouls_and_warnings,
                force_keypad_numbers: edited.force_keypad_numbers,
                show_behind_schedule_time: edited.show_behind_schedule_time,
                confirm_score: edited.confirm_score,
                hide_time: edited.hide_time,
                audible_countdown: edited.audible_countdown,
            },
            ConfigPage::Display => PageEntrySnapshot::Display {
                white_on_right: edited.white_on_right,
                brightness: edited.brightness,
                front_display_layout: edited.front_display_layout,
            },
            ConfigPage::Sound => PageEntrySnapshot::Sound {
                sound: edited.sound.clone(),
            },
            ConfigPage::Remotes(_, _) => PageEntrySnapshot::Remotes {
                remotes: edited.sound.remotes.clone(),
            },
            ConfigPage::Language => PageEntrySnapshot::Language {
                original_language: edited.original_language,
                pending_language: edited.pending_language,
            },
            ConfigPage::Buzzer => PageEntrySnapshot::Buzzer {
                buzzer_sound: edited.sound.buzzer_sound,
            },
            ConfigPage::Main | ConfigPage::User => return,
        };
        self.page_entry_snapshot = Some(snapshot);
    }

    fn revert_from_snapshot(&mut self) {
        let (Some(edited), Some(snapshot)) = (
            self.edited_settings.as_mut(),
            self.page_entry_snapshot.take(),
        ) else {
            return;
        };
        snapshot.revert_into(edited);
    }

    fn persist_config(&self) {
        if let Err(e) = confy::store(APP_NAME, None, &self.config) {
            error!("Failed to persist config: {e}");
        }
    }

    /// Initialize `edited_settings` from the current app/TM state and
    /// navigate to the Game Options editor, landing on `landing`.
    ///
    /// Used by both `Message::EditGameConfig` (landing = `ConfigPage::Main`)
    /// and `Message::EditGameConfigPage` (arbitrary landing page, e.g. for
    /// the game-info table tap-through added in Task 7).
    fn enter_game_config(&mut self, landing: ConfigPage) -> Task<Message> {
        let mut task = Task::none();

        // Whether a key is on file for the event this editor is about to show, asked of the
        // store. Reading the shared client instead made the row report an event the refbox holds
        // a good key for as disconnected, because that client's key had been cleared by an
        // earlier visit to the picker.
        let uwhportal_token_valid = if self.uwhportal_client.is_some() {
            match self.current_event_id.as_ref() {
                Some(event_id)
                    if key_for_event(
                        &self.config,
                        self.current_site.base_url.expose(),
                        event_id,
                    )
                    .is_some() =>
                {
                    task = self.check_uwhportal_auth(event_id);
                    None
                }
                _ => Some(false),
            }
        } else {
            Some(false)
        };

        let edited_settings = EditableSettings {
            config: self.tm.lock().config().clone(),
            game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                self.snapshot.next_game_number.clone()
            } else {
                self.snapshot.game_number.clone()
            },
            white_on_right: self.config.hardware.white_on_right,
            brightness: self.config.hardware.brightness,
            front_display_layout: self.config.front_display_layout,
            source: self.source,
            remembered_remote: self.config.remembered_remote,
            custom_site: self.config.custom_site.clone(),
            uwhportal_token_valid,
            current_event_id: self.current_event_id.clone(),
            current_court: self.current_court.clone(),
            schedule: self.schedule.clone(),
            sound: self.config.sound.clone(),
            mode: self.config.mode,
            hide_time: self.config.hide_time,
            collect_scorer_cap_num: self.config.collect_scorer_cap_num,
            track_fouls_and_warnings: self.config.track_fouls_and_warnings,
            force_keypad_numbers: self.config.force_keypad_numbers,
            show_behind_schedule_time: self.config.show_behind_schedule_time,
            confirm_score: self.config.confirm_score,
            audible_countdown: self.config.audible_countdown,
            pending_language: None,
            original_language: None,
            beep_test_levels: None,
            selected_level: 0,
        };

        self.edited_settings = Some(edited_settings);

        self.app_state = AppState::EditGameConfig(landing);
        // Start change-watching for the landing page right away. Without this,
        // entering the editor directly on a real page (e.g. the game-info
        // shortcut into Game Options) leaves no page-entry snapshot, so
        // page_has_changes always reports "no changes" and Apply never enables.
        // No-op for Main/User landings (capture_snapshot_for early-returns).
        self.capture_snapshot_for(landing);
        trace!("AppState changed to {:?}", self.app_state);
        task
    }

    fn navigate_to_parent(&mut self, page: ConfigPage) {
        let parent = match page {
            ConfigPage::Game | ConfigPage::App | ConfigPage::User | ConfigPage::Language => {
                ConfigPage::Main
            }
            ConfigPage::Display | ConfigPage::Sound => ConfigPage::User,
            ConfigPage::Remotes(_, _) => ConfigPage::Sound,
            ConfigPage::Buzzer => ConfigPage::Sound,
            ConfigPage::CustomSite(_) => ConfigPage::Game,
            ConfigPage::Main => ConfigPage::Main,
        };
        self.app_state = AppState::EditGameConfig(parent);
        // Re-capture the parent's snapshot so its Apply gate works after returning
        // from a sub-page (Cancel/Apply on a sub-page consumes or clears the snapshot).
        // capture_snapshot_for early-returns for Main and User, so this is a no-op
        // for navigation-only parents.
        self.capture_snapshot_for(parent);
    }
}

impl Drop for RefBoxApp {
    fn drop(&mut self) {
        for mut child in self.sim_children.drain(..) {
            info!("Waiting for sim child");
            // Best-effort: a wait() failure here (e.g. the child was
            // already reaped) must not panic inside Drop, which would
            // mask the real shutdown reason.
            let _ = child.wait();
        }
    }
}

impl RefBoxApp {
    /// Whether the game-info power button and the power page are shown: only on
    /// the Pi, or when forced on for testing.
    fn power_controls_visible(&self) -> bool {
        self.is_pi || self.force_power_controls
    }

    pub(super) fn new(flags: RefBoxAppFlags) -> (Self, Task<Message>) {
        let RefBoxAppFlags {
            config,
            config_dir,
            serial_ports,
            binary_port,
            json_port,
            sim_children,
            sim_spawn_config,
            require_https,
            fullscreen,
            list_all_events,
            force_power_controls,
            install_path,
            restart_argv,
            show_rolled_back,
        } = flags;

        // Paint in the saved display mode from the first frame.
        crate::app::theme::set_display_mode(config.display_mode);

        // The clock is deliberately NOT started here. Whether there is anything to
        // count down to depends on the startup source and the restored link note,
        // neither of which has been read yet, and a break started now cannot be
        // taken back: it would count fifteen minutes down toward a game that is
        // never coming. Started (or parked) once, below, after both are known.
        let tm = TournamentManager::new(config.game.clone());

        // In BeepTest mode, also build a cadence engine. `None` for the
        // ordinary Hockey/Rugby modes — the game `tm` above remains the
        // single source of truth there.
        let beep_test_tm = if config.mode == Mode::BeepTest {
            Some(BeepTestManager::new(config.beep_test.clone()))
        } else {
            None
        };

        // A custom site is the one source that comes back on its own after a
        // relaunch: the event it uses is written inside the saved URL, so there
        // is nothing to restore from the portal link note. Portal mode is still
        // decided by that note further down, exactly as before.
        let startup_source = if config.source == GameSource::Custom {
            // Asked of the parser directly rather than through `site_target`: that function builds
            // a Custom target from this very call and returns `None` for no other reason, so the
            // two cannot disagree -- and only the parser hands back *why* it refused.
            match custom_site::parse_custom_site(&config.custom_site.url) {
                Ok(_) => GameSource::Custom,
                Err(reason) => {
                    error!(
                        "{}",
                        unusable_saved_address_log_line(reason, "starting with manual games")
                    );
                    GameSource::Manual
                }
            }
        } else {
            GameSource::Manual
        };

        // Falls back to the built-in portal for Manual and Portal alike: the
        // client always exists, and which games the operator gets is decided by
        // `source`, not by which site the idle client happens to hold.
        let current_site = site_target(
            startup_source,
            config.mode,
            &config.custom_site,
            require_https,
        )
        .unwrap_or_else(|| portal_target(config.mode, require_https));

        #[cfg(debug_assertions)]
        let scramble_token_pending = std::env::var("UWH_PORTAL_SCRAMBLE_TOKEN").is_ok();
        #[cfg(debug_assertions)]
        if scramble_token_pending {
            warn!(
                "UWH_PORTAL_SCRAMBLE_TOKEN armed: in-memory token will be invalidated after first event link"
            );
        }
        let uwhportal_client = build_site_client(&current_site).map(|c| Arc::new(Mutex::new(c)));

        // Shared event id the background portal task consults for its
        // periodic `verify_token` check. Mirrors `current_event_id` on
        // `RefBoxApp`; both start `None` here and are kept in sync via
        // `set_current_event_id` on every subsequent write.
        let portal_event_id: SelectedEventId = Arc::new(Mutex::new(None));
        let portal_access_keys: SharedAccessKeys = Arc::new(Mutex::new(Default::default()));

        let tm = SharedGame::new(tm);

        let has_led_panel = !serial_ports.is_empty();

        let update_sender = UpdateSender::new(
            serial_ports,
            binary_port,
            json_port,
            config.hide_time,
            config.mode == Mode::BeepTest,
            if has_led_panel || config.mode == Mode::BeepTest {
                crate::sim_frame::FrontDisplayLayout::Default
            } else {
                config.front_display_layout
            },
        );

        let sound =
            SoundController::new(config.sound.clone(), update_sender.get_trigger_flash_fn());

        let snapshot = Default::default();

        // If the queue file exists but is unreadable (rare — permission
        // error on the refbox's own config dir), we log and fall back to
        // a fresh in-memory queue under the system temp dir so the UI can
        // still start. If even the temp dir refuses I/O (e.g. a locked-
        // down loaner laptop), we fall back to a degraded mode with no
        // persistence and the portal indicator pinned Red so the operator
        // sees the problem — but the core game clock and scoring still
        // work, which is what matters at the pool.
        //
        // The production `UwhPortalIo` is built from a clone of the shared
        // `UwhPortalClient` handle so that token mutations on the UI thread are
        // immediately visible to the background retry task. If the client
        // failed to construct (only possible on a bad https-only config), we go
        // straight to degraded mode (a visible red indicator, no background
        // task) rather than a `NullIo` uploader — a `NullIo`-backed manager
        // would report success for every call, keeping the dot green while it
        // fake-resolved (deleted) every queued game and nothing reached the
        // portal (finding M6).
        std::fs::create_dir_all(&config_dir).ok();

        let (portal_manager, portal_event_rx) = match &uwhportal_client {
            // No portal client: degraded mode (red, not-sending). See above.
            None => {
                warn!(
                    "portal client unavailable; portal subsystem starting in degraded \
                     (red, not-sending) mode — no results will be uploaded this session"
                );
                PortalManager::new_degraded(&config_dir)
            }
            // Have a client: build the real uploader, trying `config_dir` first,
            // then `std::env::temp_dir()`, then falling back to degraded mode if
            // even the temp dir refuses I/O. Each attempt gets its own freshly
            // built `UwhPortalIo`, so the retry helper is a closure.
            Some(client) => {
                let try_new_manager = |dir: &std::path::Path| {
                    PortalManager::new(
                        dir,
                        UwhPortalIo::new(
                            Arc::clone(client),
                            Arc::clone(&portal_event_id),
                            Arc::clone(&portal_access_keys),
                        ),
                    )
                };
                match try_new_manager(&config_dir) {
                    Ok(pair) => pair,
                    Err(primary_err) => {
                        error!(
                            "portal manager startup failed on config dir ({}); falling back to temp dir",
                            primary_err
                        );
                        match try_new_manager(&std::env::temp_dir()) {
                            Ok(pair) => pair,
                            Err(secondary_err) => {
                                error!(
                                    "portal manager also failed on temp dir ({}); \
                                     continuing in degraded mode — retry queue will not persist, \
                                     portal indicator will show red",
                                    secondary_err
                                );
                                PortalManager::new_degraded(&config_dir)
                            }
                        }
                    }
                }
            }
        };
        let portal_event_rx = Arc::new(Mutex::new(Some(portal_event_rx)));

        // BeepTest mode boots straight into the beep-test screen. Hockey and
        // Rugby modes keep the historic MainPage landing.
        let default_app_state = if config.mode == Mode::BeepTest {
            AppState::BeepTestPage
        } else {
            AppState::MainPage
        };

        // After a startup auto-revert, land on the Updates page showing the
        // one-time rollback notice instead of the normal landing screen. The
        // backup was consumed (renamed away) by the revert, so no revert button
        // is offered. `last_app_state` stays the normal default so Back is sane.
        let initial_app_state = if show_rolled_back {
            AppState::Updates {
                state: UpdateUiState::RolledBack,
                backup_available: false,
            }
        } else {
            default_app_state.clone()
        };

        let mut new = Self {
            pen_edit: ListEditor::new(tm.clone()),
            warn_edit: ListEditor::new(tm.clone()),
            foul_edit: ListEditor::new(tm.clone()),
            tm,
            beep_test_tm,
            beep_test_snapshot: BeepTestSnapshot::default(),
            config,
            edited_settings: Default::default(),
            page_entry_snapshot: None,
            snapshot,
            app_state: initial_app_state,
            last_app_state: default_app_state,
            time_edit_old: (Duration::ZERO, None),
            last_message: Message::NoAction,
            update_sender,
            uwhportal_client,
            portal_event_id,
            portal_access_keys,
            current_site,
            site_generation: 0,
            require_https,
            source: startup_source,
            events: EventStore::default(),
            schedule: None,
            game_rosters: BlackWhiteBundle {
                black: Vec::new(),
                white: Vec::new(),
            },
            team_rosters: BTreeMap::new(),
            current_event_id: None,
            current_court: None,
            pending_restore_game: None,
            pending_restore_schedule: None,
            last_played: None,
            last_played_start: None,
            sound,
            sim_children,
            sim_spawn_config,
            has_led_panel,
            beep_test_has_run: false,
            beep_test_display_layout: crate::sim_frame::FrontDisplayLayout::Default,
            list_all_events,
            is_pi: crate::app::power_control::detect_raspberry_pi(),
            force_power_controls,
            mouse_alarm_held: false,
            spacebar_held: false,
            alarm_delay_token: 0,
            timeout_revive: None,
            timeout_revive_token: 0,
            portal_manager,
            portal_event_rx,
            config_dir,
            install_path,
            restart_argv,
            pending_update: None,
            update_backup_version: None,
            #[cfg(debug_assertions)]
            scramble_token_pending,
        };

        // Before anything else touches it: the background uploader resolves a key per call from
        // this, and nothing else publishes until the operator switches source or logs in again.
        // Left unpublished, a restart left every queued result undeliverable while the foreground
        // read the same keys from `config` and reported the login healthy.
        new.publish_access_keys();

        // Restore a recent portal link so a relaunch (language change, self-update)
        // or a short shutdown comes back recognized instead of dormant. A stale or
        // cross-portal note is simply not restored (the app starts dormant) but the
        // note is NOT deleted here: a Raspberry Pi can boot with an uncorrected clock,
        // and deleting on a momentarily-wrong clock would permanently lose a recent
        // link and force the operator to re-supply the token on every restart. The
        // note's real lifecycle lives in `persist_link_session` (rewritten on Apply /
        // heartbeat, deleted when the portal is switched off or no event is linked).
        // See ADR 011/017 amendment and
        // docs/superpowers/specs/2026-06-25-portal-link-restore-resilience-design.md.
        //
        // Skipped entirely when a custom site was restored above: the note
        // records a *portal* link, and applying it would silently move the
        // operator off the site they configured.
        match crate::portal_manager::link_session::load_or_none(&new.config_dir) {
            Ok(Some(_)) if new.source == GameSource::Custom => {
                info!("Custom site restored from config; portal link note ignored");
            }
            Ok(Some(note)) => {
                if decide_restore(&note, time::OffsetDateTime::now_utc(), new.config.mode) {
                    info!(
                        "Restoring portal link to {} (court {:?}, game {:?})",
                        note.event_id.full(),
                        note.court,
                        note.current_game
                    );
                    new.source = GameSource::Portal;
                    new.current_court = note.court.clone();
                    new.pending_restore_game = note.current_game.clone();
                    // Push the remembered game into the engine now rather than
                    // waiting for a schedule. With the network off no schedule ever
                    // arrives, and an engine with nothing next used to answer
                    // `game_number + 1` — the phantom that played a game unattended
                    // and queued a 0-0. Timing and start time stay unknown until a
                    // schedule confirms them.
                    if let Some(ref number) = note.current_game {
                        new.tm.lock().set_next_game(NextGameInfo {
                            number: number.clone(),
                            timing: None,
                            start_time: None,
                        });
                    }
                    new.last_played = note.last_played.clone();
                    new.last_played_start = note.last_played_start;
                    new.set_current_event_id(Some(note.event_id.clone()));
                    // Defer the schedule fetch to RecvEventList (after the event
                    // list populates self.events) so it can't race ahead of the
                    // event list and silently skip the game re-selection.
                    new.pending_restore_schedule = Some(note.event_id);
                } else {
                    info!(
                        "Portal link note present but stale/cross-portal; starting dormant (note kept)"
                    );
                }
            }
            Ok(None) => {}
            Err(e) => error!("Failed to read portal_link.json: {e}"),
        }

        // Every decision that can settle the startup source has now been made: the
        // saved custom site above, and the portal-link note just restored. Tell the
        // engine once, from the app's own answer, so no startup path can miss it —
        // neither of those two routes through `commit_source`, and an engine left
        // unlinked while games come from a site goes on naming them by arithmetic,
        // which is the invented game this whole change exists to remove. Manual
        // leaves the flag false: manual mode numbers its games sequentially.
        //
        // The break clock is started here, not at construction, because only now is
        // it known whether a game is coming. Nothing next means nothing to count
        // down to, so the clock is parked at 0:00 — the same state a court played to
        // the end of its schedule is left in, so a restart comes back to the state it
        // left rather than to a fifteen-minute countdown toward nothing.
        let linked = new.uses_remote();
        {
            let mut tm = new.tm.lock();
            tm.set_schedule_linked(linked);
            if tm.next_game_number().is_empty() {
                // why this cannot panic: `TournamentManager::new` leaves the clock
                // `Stopped` and nothing above starts it, and `set_game_clock_time`
                // only errors while the clock is running.
                tm.set_game_clock_time(Duration::ZERO).unwrap();
            } else {
                tm.start_clock(Instant::now());
            }
        }

        // The event-list fetch pushed below no longer waits for the operator to
        // turn Using-UWH-Portal ON — that dormancy was relaxed 2026-08-27; see
        // the comment at the fetch itself for why.
        let mut startup_tasks = vec![if fullscreen {
            window::get_latest().and_then(|w| window::change_mode(w, window::Mode::Fullscreen))
        } else {
            Task::none()
        }];
        // The portal's event list is fetched whatever the source, so that
        // choosing UWH PORTAL in the editor finds the list already there rather
        // than an empty picker — and, critically, never the custom site's own
        // event standing in for it. Offline this fails and is logged; see
        // `request_event_list`. This relaxes ADR 017's dormancy contract for
        // the event list alone; see the 2026-08-27 amendment.
        startup_tasks.push(new.request_event_list());
        // A custom site's event is named in its URL rather than picked from a
        // list, so it is adopted directly. This is also what brings its
        // schedule and teams back after a restart.
        if new.source == GameSource::Custom {
            startup_tasks.push(new.adopt_custom_event());
        }
        // Arm a one-shot ~20s timer. If the app is still running when it fires,
        // startup was healthy and the update trial marker can be cleared so a
        // later boot is not mistaken for a failed update trial.
        startup_tasks.push(Task::future(async {
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            Message::UpdaterHealthyCheck
        }));
        let task = Task::batch(startup_tasks);

        (new, task)
    }

    fn manual_alarm_hold_duration(&self) -> Duration {
        match (self.snapshot.current_period, self.snapshot.timeout) {
            (
                GamePeriod::FirstHalf
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::SuddenDeath,
                None,
            ) => Duration::from_millis(150),
            _ => Duration::from_secs(1),
        }
    }

    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        trace!("Handling message: {message:?}");

        if !message.is_repeatable() && (message == self.last_message) {
            warn!("Ignoring a repeated message: {message:?}");
            self.last_message = message.clone();
            return Task::none();
        } else {
            self.last_message = message.clone();
        }

        match message {
            Message::NewSnapshot(snapshot) => self.apply_snapshot(snapshot),
            Message::EditTime => {
                let now = Instant::now();
                let mut tm = self.tm.lock();
                let was_running = tm.clock_is_running();
                tm.stop_clock(now).unwrap();
                let game_time = tm.game_clock_time(now).unwrap();
                let timeout_time = tm.timeout_clock_time(now);
                self.time_edit_old = (game_time, timeout_time);
                self.last_app_state = self.app_state.clone();
                self.app_state = AppState::TimeEdit(was_running, game_time, timeout_time);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ChangeTime {
                increase,
                secs,
                timeout,
            } => {
                let (dur, large_max) = match self.app_state {
                    AppState::TimeEdit(_, ref mut game_dur, ref mut timeout_dur) => {
                        if timeout {
                            (timeout_dur.as_mut().unwrap(), false)
                        } else {
                            (game_dur, true)
                        }
                    }
                    AppState::ParameterEditor(_, ref mut dur, _) => (dur, false),
                    AppState::KeypadPage(KeypadPage::TeamTimeouts(ref mut dur, _), _) => {
                        (dur, false)
                    }
                    _ => unreachable!(),
                };
                if increase {
                    *dur = min(
                        Duration::from_secs(if large_max {
                            MAX_LONG_STRINGABLE_SECS as u64
                        } else {
                            MAX_STRINGABLE_SECS as u64
                        }),
                        dur.saturating_add(Duration::from_secs(secs)),
                    );
                } else {
                    *dur = std::cmp::max(
                        dur.saturating_sub(Duration::from_secs(secs)),
                        Duration::from_micros(1),
                    );
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::TimeEditComplete { canceled } => {
                if let AppState::TimeEdit(was_running, game_time, timeout_time) = self.app_state {
                    let mut tm = self.tm.lock();
                    let now = Instant::now();
                    if !canceled {
                        tm.set_game_clock_time(game_time).unwrap();
                        if let Some(time) = timeout_time {
                            tm.set_timeout_clock_time(time).unwrap();
                        }
                    }
                    if was_running {
                        tm.start_clock(now);
                        tm.update(now).unwrap();
                    }
                    let snapshot = tm.generate_snapshot(now).unwrap();
                    drop(tm);
                    let task = self.apply_snapshot(snapshot);
                    self.app_state = self.last_app_state.clone();
                    trace!("AppState changed to {:?}", self.app_state);
                    task
                } else {
                    unreachable!();
                }
            }
            Message::StartPlayNow => {
                let mut tm = self.tm.lock();
                let now = Instant::now();
                if let Err(e) = tm.start_play_now(now) {
                    // The only reachable error is a court whose schedule is finished,
                    // where START NOW is greyed out anyway. Log and do nothing rather
                    // than crash the refbox mid-tournament.
                    warn!("Could not start play now: {e}");
                    return Task::none();
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
            Message::EditScores => {
                let tm = self.tm.lock();
                self.app_state = AppState::ScoreEdit {
                    scores: tm.get_scores(),
                    is_confirmation: false,
                };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::AddNewScore(color) => {
                let task = if self.config.collect_scorer_cap_num {
                    // Opens with `team_score` off: naming the scorer is the
                    // normal case, so the grid stays live and the operator taps
                    // TEAM SCORE only when there is no individual to credit.
                    self.app_state = AppState::KeypadPage(
                        KeypadPage::AddScore {
                            color,
                            team_score: false,
                        },
                        0,
                    );
                    Task::none()
                } else {
                    let mut tm = self.tm.lock();
                    let now = Instant::now();
                    if tm.current_period() == GamePeriod::SuddenDeath {
                        let mut scores = tm.get_scores();
                        scores[color] = scores[color].saturating_add(1);

                        tm.pause_for_confirm(now).unwrap();
                        self.app_state = AppState::ConfirmScores(scores);
                        Task::none()
                    } else {
                        tm.add_score(color, 0, now);
                        let snapshot = tm.generate_snapshot(now).unwrap(); // TODO: Remove this unwrap
                        std::mem::drop(tm);
                        let task = self.apply_snapshot(snapshot);
                        self.app_state = AppState::MainPage;
                        task
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::ChangeScore { color, increase } => {
                if let AppState::ScoreEdit { ref mut scores, .. } = self.app_state {
                    if increase {
                        scores[color] = scores[color].saturating_add(1);
                    } else {
                        scores[color] = scores[color].saturating_sub(1);
                    }
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ScoreEditComplete { canceled } => {
                let mut tasks = vec![];
                let mut tm = self.tm.lock();
                let mut now = Instant::now();

                self.app_state = if let AppState::ScoreEdit {
                    scores,
                    is_confirmation,
                } = self.app_state
                {
                    if is_confirmation {
                        tm.set_scores(scores, now);
                        if let Err(e) = tm.end_confirm_pause(now) {
                            // The background updater can end this pause first. If its tick
                            // ended the pause but then failed to build a snapshot, the
                            // message that would have closed this page was lost with the
                            // error, so the page is still up while the engine has moved on.
                            // Acting on a pause that is already over gives the operator the
                            // outcome they asked for, so carry on rather than crash.
                            debug!("Confirm pause had already ended: {e}");
                        }
                        tm.start_clock(now);

                        // Update `tm` after game ends to get into Between Games
                        now += Duration::from_millis(2);
                        tm.update(now).unwrap();
                        AppState::MainPage
                    } else if !canceled {
                        if tm.current_period() == GamePeriod::SuddenDeath
                            && (scores.black != scores.white)
                        {
                            tm.pause_for_confirm(now).unwrap();
                            AppState::ConfirmScores(scores)
                        } else {
                            tm.set_scores(scores, now);
                            AppState::MainPage
                        }
                    } else {
                        AppState::MainPage
                    }
                } else {
                    unreachable!()
                };

                let snapshot = tm.generate_snapshot(now).unwrap(); // `now` is in the past!
                std::mem::drop(tm);
                tasks.push(self.apply_snapshot(snapshot));

                trace!("AppState changed to {:?}", self.app_state);
                Task::batch(tasks)
            }
            Message::PenaltyOverview => {
                if let Err(e) = self.pen_edit.start_session() {
                    warn!("Failed to start penalty edit session: {e}");
                    self.pen_edit.abort_session();
                    self.pen_edit.start_session().unwrap();
                }
                self.app_state = AppState::PenaltyOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::WarningOverview => {
                if let Err(e) = self.warn_edit.start_session() {
                    warn!("Failed to start warning edit session: {e}");
                    self.warn_edit.abort_session();
                    self.warn_edit.start_session().unwrap();
                }
                self.app_state = AppState::WarningOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::FoulOverview => {
                if let Err(e) = self.foul_edit.start_session() {
                    warn!("Failed to start foul edit session: {e}");
                    self.foul_edit.abort_session();
                    self.foul_edit.start_session().unwrap();
                }
                self.app_state = AppState::FoulOverview(OptColorBundle {
                    black: 0,
                    equal: 0,
                    white: 0,
                });
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::Scroll { which, up } => {
                match self.app_state {
                    AppState::PenaltyOverview(ref mut indices)
                    | AppState::WarningOverview(ref mut indices) => {
                        let idx = match which {
                            ScrollOption::Black => &mut indices.black,
                            ScrollOption::White => &mut indices.white,
                            ScrollOption::GameParameter
                            | ScrollOption::Equal
                            | ScrollOption::PortalDetail => unreachable!(),
                        };
                        if up {
                            *idx = idx.saturating_sub(1);
                        } else {
                            *idx = idx.saturating_add(1);
                        }
                    }
                    AppState::FoulOverview(ref mut indices) => {
                        let idx = match which {
                            ScrollOption::Black => &mut indices.black,
                            ScrollOption::Equal => &mut indices.equal,
                            ScrollOption::White => &mut indices.white,
                            ScrollOption::GameParameter | ScrollOption::PortalDetail => {
                                unreachable!()
                            }
                        };
                        if up {
                            *idx = idx.saturating_sub(1);
                        } else {
                            *idx = idx.saturating_add(1);
                        }
                    }
                    AppState::ParameterList(_, ref mut idx) => {
                        debug_assert_eq!(which, ScrollOption::GameParameter);
                        if up {
                            *idx = idx.saturating_sub(1);
                        } else {
                            *idx = idx.saturating_add(1);
                        }
                    }
                    AppState::PortalDetailPage {
                        ref mut scroll_index,
                    } => {
                        debug_assert_eq!(which, ScrollOption::PortalDetail);
                        if up {
                            *scroll_index = scroll_index.saturating_sub(1);
                        } else {
                            *scroll_index = scroll_index.saturating_add(1);
                        }
                    }
                    _ => {
                        unreachable!();
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::PenaltyOverviewComplete { canceled } => {
                if canceled {
                    self.pen_edit.abort_session();
                    self.app_state = AppState::MainPage;
                } else if let Err(e) = self.pen_edit.apply_changes(Instant::now()) {
                    let err_string = match e {
                        PenaltyEditorError::ListTooLong(colors) => format!(
                            "The {colors} penalty list(s) \
                            is/are too long. Some penalties will not be visible on the main page."
                        ),
                        e => format!(
                            "An error occurred while applying the changes to the penalties. \
                            Some of the changes may have been applied. Please retry any remaining changes.\n\n\
                            Error Message:\n{e}"
                        ),
                    };
                    error!("{err_string}");
                    self.pen_edit.abort_session();
                    self.app_state =
                        AppState::ConfirmationPage(ConfirmationKind::Error(err_string));
                } else {
                    self.app_state = AppState::MainPage;
                }
                let snapshot = self.tm.lock().generate_snapshot(Instant::now()).unwrap();
                let task = self.apply_snapshot(snapshot);
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::WarningOverviewComplete { canceled } => {
                if canceled {
                    self.warn_edit.abort_session();
                    self.app_state = AppState::WarningsSummaryPage;
                } else if let Err(e) = self.warn_edit.apply_changes(Instant::now()) {
                    let err_string = format!(
                        "An error occurred while applying the changes to the warnings. \
                    Some of the changes may have been applied. Please retry any remaining changes.\n\n\
                    Error Message:\n{e}"
                    );
                    error!("{err_string}");
                    self.warn_edit.abort_session();
                    self.app_state =
                        AppState::ConfirmationPage(ConfirmationKind::Error(err_string));
                } else {
                    self.app_state = AppState::WarningsSummaryPage;
                }
                let snapshot = self.tm.lock().generate_snapshot(Instant::now()).unwrap();
                let task = self.apply_snapshot(snapshot);
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::FoulOverviewComplete { canceled } => {
                if canceled {
                    self.foul_edit.abort_session();
                    self.app_state = AppState::WarningsSummaryPage;
                } else if let Err(e) = self.foul_edit.apply_changes(Instant::now()) {
                    let err_string = format!(
                        "An error occurred while applying the changes to the fouls. \
                    Some of the changes may have been applied. Please retry any remaining changes.\n\n\
                    Error Message:\n{e}"
                    );
                    error!("{err_string}");
                    self.foul_edit.abort_session();
                    self.app_state =
                        AppState::ConfirmationPage(ConfirmationKind::Error(err_string));
                } else {
                    self.app_state = AppState::WarningsSummaryPage;
                }
                let snapshot = self.tm.lock().generate_snapshot(Instant::now()).unwrap();
                let task = self.apply_snapshot(snapshot);
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::ChangeKind(new_kind) => {
                if let AppState::KeypadPage(KeypadPage::Penalty(_, _, ref mut kind, _), _) =
                    self.app_state
                {
                    *kind = new_kind;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ChangeInfraction(new_infraction) => {
                match self.app_state {
                    AppState::KeypadPage(KeypadPage::Penalty(_, _, _, ref mut infraction), _)
                    | AppState::KeypadPage(
                        KeypadPage::FoulAdd {
                            ref mut infraction, ..
                        },
                        _,
                    )
                    | AppState::KeypadPage(
                        KeypadPage::WarningAdd {
                            ref mut infraction, ..
                        },
                        _,
                    ) => {
                        *infraction = new_infraction;
                    }
                    _ => unreachable!(),
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::PenaltyEditComplete { canceled, deleted } => {
                if !canceled {
                    if let AppState::KeypadPage(
                        KeypadPage::Penalty(origin, color, kind, infraction),
                        player_num,
                    ) = self.app_state
                    {
                        if deleted {
                            if let Some((old_color, index)) = origin {
                                self.pen_edit.delete_item(old_color, index).unwrap();
                            } else {
                                unreachable!();
                            }
                        } else {
                            let player_num = player_num.try_into().unwrap();
                            if let Some((old_color, index)) = origin {
                                self.pen_edit
                                    .edit_item(
                                        old_color, index, color, player_num, kind, infraction,
                                    )
                                    .unwrap();
                            } else {
                                self.pen_edit
                                    .add_item(color, player_num, kind, infraction)
                                    .unwrap();
                            }
                        }
                    } else {
                        unreachable!();
                    }
                }
                self.app_state = AppState::PenaltyOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::WarningEditComplete {
                canceled,
                deleted,
                ret_to_overview,
            } => {
                if !canceled {
                    if let AppState::KeypadPage(
                        KeypadPage::WarningAdd {
                            origin,
                            color,
                            infraction,
                            team_warning,
                            ..
                        },
                        player_num,
                    ) = self.app_state
                    {
                        let player_num = if team_warning {
                            None
                        } else {
                            Some(player_num.try_into().unwrap())
                        };

                        if deleted {
                            if let Some((old_color, index)) = origin {
                                self.warn_edit.delete_item(old_color, index).unwrap();
                            } else {
                                unreachable!();
                            }
                        } else if !ret_to_overview {
                            self.tm
                                .lock()
                                .add_warning(color, player_num, infraction, Instant::now())
                                .unwrap();
                        } else if let Some((old_color, index)) = origin {
                            self.warn_edit
                                .edit_item(old_color, index, color, player_num, (), infraction)
                                .unwrap();
                        } else {
                            self.warn_edit
                                .add_item(color, player_num, (), infraction)
                                .unwrap();
                        }
                    } else {
                        unreachable!();
                    }
                }
                self.app_state = if !ret_to_overview {
                    AppState::MainPage
                } else {
                    AppState::WarningOverview(BlackWhiteBundle { black: 0, white: 0 })
                };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::FoulEditComplete {
                canceled,
                deleted,
                ret_to_overview,
            } => {
                if !canceled {
                    if let AppState::KeypadPage(
                        KeypadPage::FoulAdd {
                            origin,
                            color,
                            infraction,
                            ..
                        },
                        player_num,
                    ) = self.app_state
                    {
                        let player_num = if color.is_none() {
                            None
                        } else {
                            Some(player_num.try_into().unwrap())
                        };

                        if deleted {
                            if let Some((old_color, index)) = origin {
                                self.foul_edit.delete_item(old_color, index).unwrap();
                            } else {
                                unreachable!();
                            }
                        } else if !ret_to_overview {
                            self.tm
                                .lock()
                                .add_foul(color, player_num, infraction, Instant::now())
                                .unwrap();
                        } else if let Some((old_color, index)) = origin {
                            self.foul_edit
                                .edit_item(old_color, index, color, player_num, (), infraction)
                                .unwrap();
                        } else {
                            self.foul_edit
                                .add_item(color, player_num, (), infraction)
                                .unwrap();
                        }
                    } else {
                        unreachable!();
                    }
                }
                self.app_state = if !ret_to_overview {
                    AppState::MainPage
                } else {
                    AppState::FoulOverview(OptColorBundle {
                        black: 0,
                        equal: 0,
                        white: 0,
                    })
                };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::KeypadPage(mut page) => {
                let init_val = match page {
                    KeypadPage::AddScore { .. }
                    | KeypadPage::Penalty(None, _, _, _)
                    | KeypadPage::FoulAdd { origin: None, .. }
                    | KeypadPage::WarningAdd { origin: None, .. } => 0,
                    KeypadPage::Penalty(Some((color, index)), _, _, _) => {
                        self.pen_edit.get_item(color, index).unwrap().player_number as u32
                    }
                    KeypadPage::WarningAdd {
                        origin: Some((color, index)),
                        ..
                    } => self
                        .warn_edit
                        .get_item(color, index)
                        .unwrap()
                        .player_number
                        .map(|n| n.into())
                        .unwrap_or(0),
                    KeypadPage::FoulAdd {
                        origin: Some((color, index)),
                        ..
                    } => self
                        .foul_edit
                        .get_item(color, index)
                        .unwrap()
                        .player_number
                        .map(|n| n.into())
                        .unwrap_or(0),
                    KeypadPage::TeamTimeouts(_, _) => self
                        .edited_settings
                        .as_ref()
                        .map(|s| s.config.num_team_timeouts_allowed as u32)
                        .unwrap_or(self.config.game.num_team_timeouts_allowed as u32),
                    KeypadPage::GameNumber => self
                        .edited_settings
                        .as_ref()
                        .unwrap()
                        .game_number
                        .parse()
                        .unwrap_or(0),
                    KeypadPage::PortalLogin(ref mut id, _) => {
                        // Linking is refused while a game is in progress. A
                        // successful link immediately re-requests the schedule,
                        // which would replace the loaded schedule under a
                        // running game — and the operator has no reason to be
                        // linking then. Same rule as the site-change guard:
                        // a game in progress, not merely a running clock,
                        // because the between-games clock always runs.
                        if self.tm.lock().current_period() != GamePeriod::BetweenGames {
                            self.app_state =
                                AppState::ConfirmationPage(ConfirmationKind::LinkLockedByGame);
                            trace!("AppState changed to {:?}", self.app_state);
                            return Task::none();
                        }
                        // why this cannot panic: this branch only runs when the
                        // portal client was successfully constructed at startup,
                        // and the guard is held only for a synchronous `id()` call.
                        *id = self.uwhportal_client.as_ref().unwrap().lock().unwrap().id();
                        0
                    }
                };
                self.app_state = AppState::KeypadPage(page, init_val);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::KeypadButtonPress(key) => {
                if let AppState::KeypadPage(ref page, ref mut val) = self.app_state {
                    let new_val = match key {
                        KeypadButton::Zero => val.saturating_mul(10),
                        KeypadButton::One => val.saturating_mul(10).saturating_add(1),
                        KeypadButton::Two => val.saturating_mul(10).saturating_add(2),
                        KeypadButton::Three => val.saturating_mul(10).saturating_add(3),
                        KeypadButton::Four => val.saturating_mul(10).saturating_add(4),
                        KeypadButton::Five => val.saturating_mul(10).saturating_add(5),
                        KeypadButton::Six => val.saturating_mul(10).saturating_add(6),
                        KeypadButton::Seven => val.saturating_mul(10).saturating_add(7),
                        KeypadButton::Eight => val.saturating_mul(10).saturating_add(8),
                        KeypadButton::Nine => val.saturating_mul(10).saturating_add(9),
                        KeypadButton::Delete => val.saturating_div(10),
                    };
                    if new_val <= page.max_val() {
                        *val = new_val;
                    }
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::SelectPlayerNumber(number) => {
                if let AppState::KeypadPage(ref page, ref mut val) = self.app_state {
                    if number <= page.max_val() {
                        *val = number;
                    }
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::SetTeamTimeoutCount(count) => {
                if let AppState::KeypadPage(KeypadPage::TeamTimeouts(_, _), ref mut val) =
                    self.app_state
                {
                    *val = count;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::SetTeamTimeoutLength(new_len) => {
                if let AppState::KeypadPage(KeypadPage::TeamTimeouts(ref mut dur, _), _) =
                    self.app_state
                {
                    *dur = new_len;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ChangeColor(new_color) => {
                match self.app_state {
                    AppState::KeypadPage(
                        KeypadPage::AddScore { ref mut color, .. },
                        ref mut player_num,
                    )
                    | AppState::KeypadPage(
                        KeypadPage::Penalty(_, ref mut color, _, _),
                        ref mut player_num,
                    )
                    | AppState::KeypadPage(
                        KeypadPage::WarningAdd { ref mut color, .. },
                        ref mut player_num,
                    ) => {
                        *color = new_color.expect("Invalid color value");
                        // A number chosen for one team means nothing on the
                        // other — #7 is a different person on each roster.
                        *player_num = 0;
                    }
                    AppState::KeypadPage(
                        KeypadPage::FoulAdd { ref mut color, .. },
                        ref mut player_num,
                    ) => {
                        *color = new_color;
                        *player_num = 0;
                    }
                    _ => {
                        unreachable!()
                    }
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::AddScoreComplete { canceled } => {
                let mut task = Task::none();
                self.app_state = if !canceled {
                    if let AppState::KeypadPage(KeypadPage::AddScore { color, .. }, player) =
                        self.app_state
                    {
                        let mut tm = self.tm.lock();
                        let now = Instant::now();

                        let app_state = if tm.current_period() == GamePeriod::SuddenDeath {
                            let mut scores = tm.get_scores();
                            scores[color] = scores[color].saturating_add(1);

                            tm.pause_for_confirm(now).unwrap();
                            AppState::ConfirmScores(scores)
                        } else {
                            tm.add_score(color, player.try_into().unwrap(), now);
                            AppState::MainPage
                        };
                        let snapshot = tm.generate_snapshot(now).unwrap();

                        std::mem::drop(tm);
                        task = self.apply_snapshot(snapshot);

                        app_state
                    } else {
                        unreachable!()
                    }
                } else {
                    AppState::MainPage
                };
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::ShowGameDetails => {
                self.app_state = AppState::GameDetailsPage(false);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::OpenPowerPage => {
                self.app_state = AppState::PowerPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::PowerAction(action) => match action {
                PowerAction::RestartRefbox => {
                    // Reuse the existing graceful-restart path: kill sim children,
                    // mark the restart, and let iced close its windows; `main()`
                    // respawns the exe (see RESTART_PENDING).
                    for mut child in self.sim_children.drain(..) {
                        let _ = child.kill();
                    }
                    RESTART_PENDING.store(true, Ordering::Relaxed);
                    iced::exit()
                }
                // The real OS commands run only on an actual Pi; off-Pi (forced
                // on for testing) they log so a walkthrough can't power off the
                // test machine.
                PowerAction::ShutDownPi => {
                    if self.is_pi {
                        if let Err(e) = power_control::shut_down_pi() {
                            error!("Failed to power off the Pi: {e}");
                        }
                    } else {
                        info!(
                            "--force-power-controls: would power off the Pi (systemctl poweroff)"
                        );
                    }
                    Task::none()
                }
                PowerAction::RestartPi => {
                    if self.is_pi {
                        if let Err(e) = power_control::reboot_pi() {
                            error!("Failed to reboot the Pi: {e}");
                        }
                    } else {
                        info!("--force-power-controls: would reboot the Pi (systemctl reboot)");
                    }
                    Task::none()
                }
            },
            Message::UpdateAudioOutput => {
                self.sound.reload_audio_output();
                Task::none()
            }
            Message::OpenNewDisplay => {
                if self.has_led_panel {
                    // Defense in depth — the UI grays the button out when an
                    // LED panel is connected, so this branch should be
                    // unreachable via normal interaction. Log if it ever fires.
                    warn!(
                        "Ignoring OpenNewDisplay: a serial LED panel is connected; \
                         simulator windows are disabled in this configuration"
                    );
                    return Task::none();
                }
                match crate::spawn_sim_child(&self.sim_spawn_config) {
                    Ok(child) => {
                        info!(
                            "Opened new sim window; total now {}",
                            self.sim_children.len() + 1
                        );
                        self.sim_children.push(child);
                    }
                    Err(e) => {
                        error!("Failed to spawn new sim window: {e:?}");
                    }
                }
                Task::none()
            }
            Message::OpenPortalDetailPage => {
                self.app_state = AppState::PortalDetailPage { scroll_index: 0 };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ClosePortalDetailPage => {
                self.app_state = AppState::MainPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ClosePortalAttentionAction => {
                self.app_state = AppState::PortalDetailPage { scroll_index: 0 };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::PortalEvent(ev) => {
                // The background portal task woke us with a state-change
                // signal. Most variants are notifications whose effect on
                // indicator state is already covered by `ui_tick`'s
                // recompute — the main-thread `PortalManager` is the
                // source of truth. `ItemResolved` is the one variant
                // where the main-thread queue needs a mutation: we
                // remove the item and record it in the recent-success
                // ring so it appears on the detail page.
                match ev {
                    PortalEvent::ItemResolved(id) => {
                        self.portal_manager.on_item_resolved(id);
                    }
                    PortalEvent::ScoreSentStatsPending(id) => {
                        self.portal_manager.on_score_sent_stats_pending(id);
                    }
                    PortalEvent::ItemAttempted { id, attempts, at } => {
                        self.portal_manager.on_item_attempted(id, attempts, at);
                    }
                    PortalEvent::HealthChanged | PortalEvent::ItemUpdated => {
                        self.portal_manager.ui_tick();
                    }
                    PortalEvent::TokenStatus(valid) => {
                        self.portal_manager.on_token_status(valid);
                    }
                    PortalEvent::TokenUnreachable => {
                        self.portal_manager.on_token_unreachable();
                    }
                }
                // The background task fires verify_token on its cadence
                // (~5 min when healthy). Use that heartbeat to refresh the link
                // note's last-active timestamp (and current game number) so the
                // 48h restore window tracks real usage, not just first link.
                if self.uses_remote() && self.current_event_id.is_some() {
                    self.persist_link_session();
                }
                Task::none()
            }
            Message::PortalUiTick => {
                // Pure UI-layer tick that lets the 30-minute stuck-item
                // escalation reach the screen without waiting for an
                // unrelated re-render.
                self.portal_manager.ui_tick();
                Task::none()
            }
            Message::PortalRowTapped(id) => {
                if self.portal_manager.has_startup_problem() {
                    // The portal subsystem never started, so no row here is
                    // actionable and every route out of a tap is misleading:
                    // there is no background task to retry with (a retry would
                    // silently reset the timer and look like it worked), and
                    // Discard would permanently delete — unarchived — results
                    // the operator was never given a chance to send. Consistent
                    // with the rest of this state: the startup-failure row is
                    // not tappable and REFRESH declines to spin.
                    return Task::none();
                }
                if self.portal_manager.is_stuck(&id) {
                    self.app_state = AppState::PortalAttentionAction {
                        item_id: id,
                        discard_armed: false,
                    };
                    trace!("AppState changed to {:?}", self.app_state);
                } else if self.portal_manager.is_stats_pending(&id) {
                    // Stats-pending row: fire one stats attempt. No
                    // background loop, no escalation.
                    self.portal_manager.request_stats_retry(&id);
                } else {
                    // Young score-pending row tapped — force an immediate retry.
                    if let Err(e) = self.portal_manager.force_immediate_retry(&id) {
                        error!("force_immediate_retry failed: {e}");
                    }
                }
                Task::none()
            }
            Message::PortalForceSubmit(id) => {
                if let Err(e) = self.portal_manager.force_submit(&id) {
                    error!("force_submit failed: {e}");
                }
                self.app_state = AppState::PortalDetailPage { scroll_index: 0 };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::PortalRetryAll => {
                if let Err(e) = self.portal_manager.retry_all() {
                    error!("retry_all failed: {e}");
                }
                Task::none()
            }
            Message::PortalDiscardTapped(id) => {
                // Snapshot the current attention-page state before any
                // mutation so we don't fight the borrow checker when
                // reassigning `self.app_state` below.
                let current = if let AppState::PortalAttentionAction {
                    item_id,
                    discard_armed,
                } = &self.app_state
                {
                    Some((item_id.clone(), *discard_armed))
                } else {
                    None
                };
                if let Some((item_id, discard_armed)) = current {
                    if item_id == id {
                        if discard_armed {
                            if let Err(e) = self.portal_manager.discard(&id) {
                                error!("discard failed: {e}");
                            }
                            self.app_state = AppState::PortalDetailPage { scroll_index: 0 };
                        } else {
                            self.app_state = AppState::PortalAttentionAction {
                                item_id: id,
                                discard_armed: true,
                            };
                        }
                        trace!("AppState changed to {:?}", self.app_state);
                    }
                }
                Task::none()
            }
            Message::RequestPortalRefresh => {
                // Only spin the REFRESH button when there is actually an event
                // to refresh AND a site the refbox could reach; otherwise
                // nothing would arrive to clear the flag. The no-client case is
                // real: a degraded startup (see PortalManager::new_degraded) can
                // still have an event linked from a restored link note, and it
                // means the site could not be built at all -- `request_schedule`
                // builds against that same target and fails the same way, so
                // without this the button sticks on "Refreshing..." forever.
                match (
                    self.current_event_id.clone(),
                    self.uwhportal_client.is_some(),
                ) {
                    (Some(event_id), true) => {
                        if let AppState::GameDetailsPage(ref mut is_refreshing) = self.app_state {
                            *is_refreshing = true;
                        }
                        // request_schedule yields NoAction when the fetch fails;
                        // translate that into a refresh-finished signal so the
                        // "Refreshing..." button cannot stick on a network error.
                        self.request_schedule(event_id).map(|msg| match msg {
                            Message::NoAction => Message::PortalRefreshFinished,
                            other => other,
                        })
                    }
                    (Some(_), false) => {
                        // Degraded mode: the button is live (the login is not
                        // the problem) but there is nothing to fetch with. Log
                        // it — a press that does nothing is otherwise invisible
                        // in a support log.
                        warn!("REFRESH pressed but no portal client was started; nothing to fetch");
                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
            Message::PortalRefreshFinished => {
                // The refresh ended without a schedule (failure path). Stop the
                // REFRESH spinner; the success path clears it in RecvSchedule.
                if let AppState::GameDetailsPage(ref mut is_refreshing) = self.app_state {
                    *is_refreshing = false;
                }
                Task::none()
            }
            Message::ShowWarnings => {
                self.app_state = AppState::WarningsSummaryPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::EditGameConfig => self.enter_game_config(ConfigPage::Main),
            Message::EditGameConfigPage(page) => self.enter_game_config(page),
            Message::CycleDisplayMode => {
                let next = self.config.display_mode.next();
                self.config.display_mode = next;
                crate::app::theme::set_display_mode(next);
                self.persist_config();
                // Deliberately no window work here: a display-mode change is a
                // palette change and nothing else. Forcing a repaint by leaving
                // and re-entering fullscreen used to live here, but that is a
                // real surface resize — it re-laid-out the whole UI and left the
                // window at the wrong height on the Pi, moving every element
                // sized as a share of that height. The window background is now
                // a drawn element (see `window_background_container`), so the
                // repaint happens on its own.
                Task::none()
            }
            Message::ChangeConfigPage(new_page) => {
                if let AppState::EditGameConfig(ref mut page) = self.app_state {
                    if new_page == ConfigPage::Language {
                        let current =
                            Language::from_lang_id(&crate::LANGUAGE_LOADER.current_languages()[0]);
                        let settings = self.edited_settings.as_mut().unwrap();
                        settings.original_language = Some(current);
                        settings.pending_language = Some(current);
                    }
                    *page = new_page;
                } else {
                    unreachable!();
                }
                self.capture_snapshot_for(new_page);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ApplyConfigPage(page) => {
                // Decide up front whether this APPLY moves the refbox to a
                // different site, and refuse before anything is committed, so a
                // refusal leaves every edit staged exactly as it was left.
                let new_site = self.pending_site_change(page);
                // The SITE editor's APPLY is offered whether or not the address
                // was edited, so an operator whose event was cleared — the path
                // back from MANUAL — can get onto their site again. That means
                // this APPLY can commit a source change with no repoint attached,
                // which the `new_site` test alone would wave through, so ask for
                // the guard explicitly.
                let commits_custom_source =
                    matches!(page, ConfigPage::CustomSite(_)) && self.source != GameSource::Custom;
                if new_site.is_some() || commits_custom_source {
                    if let Some(refusal) = self.refuse_repoint(page) {
                        self.app_state = AppState::ConfirmationPage(refusal);
                        trace!("AppState changed to {:?}", self.app_state);
                        return Task::none();
                    }
                }
                match page {
                    ConfigPage::App => {
                        if let Some(kind) = self.apply_app_options() {
                            self.app_state = AppState::ConfirmationPage(kind);
                            trace!("AppState changed to {:?}", self.app_state);
                            return Task::none();
                        }
                    }
                    ConfigPage::Display => self.apply_display_options(),
                    ConfigPage::Sound => self.apply_sound_options(),
                    ConfigPage::Remotes(_, _) => self.apply_remote_options(),
                    ConfigPage::Game => {
                        if let Some(kind) = self.apply_game_options() {
                            self.app_state = AppState::ConfirmationPage(kind);
                            trace!("AppState changed to {:?}", self.app_state);
                            return Task::none();
                        }
                    }
                    ConfigPage::Language | ConfigPage::Main | ConfigPage::User => {
                        // Language uses its own LanguageSelectComplete path. Main
                        // and User are navigation-only and should never receive
                        // Apply.
                        return Task::none();
                    }
                    ConfigPage::Buzzer => {
                        let bs = self.edited_settings.as_ref().map(|e| e.sound.buzzer_sound);
                        if let Some(bs) = bs {
                            self.config.sound.buzzer_sound = bs;
                        }
                        self.sound.update_settings(self.config.sound.clone());
                    }
                    ConfigPage::CustomSite(_) => {
                        // why this cannot panic: the SITE editor is only
                        // reachable from the Game Options editor, which
                        // populates `edited_settings` before it can be drawn.
                        // This is the idiom the rest of this file uses for the
                        // same value; defaulting an absent one instead would
                        // mean saving a blank site over the operator's address.
                        let site = self.edited_settings.as_ref().unwrap().custom_site.clone();
                        // Reject a bad address here rather than let it fail
                        // later: an operator on the pool deck cannot debug a
                        // URL mid-game. The page stays open with the message.
                        if custom_site::parse_custom_site(&site.url).is_err() {
                            self.app_state = AppState::EditGameConfig(ConfigPage::CustomSite(true));
                            trace!("AppState changed to {:?}", self.app_state);
                            return Task::none();
                        }
                        self.config.custom_site = site;
                        // This page is only reachable with CUSTOM chosen, so
                        // applying an address here is an unambiguous "use this
                        // site" — `pending_site_change` already says as much. The
                        // source has to be committed with it, or the adoption
                        // below cannot fire: coming back from MANUAL it is the
                        // committed source that is still Manual, which left the
                        // operator with a valid address on screen, a greyed APPLY
                        // on the Game page, and no way back to their own site.
                        self.commit_source(GameSource::Custom);
                    }
                }
                // Committed without a refusal, so the site the operator just
                // chose becomes the one the refbox talks to — no restart.
                //
                // Three things about the order below are load-bearing, and the
                // compiler enforces none of them:
                //
                // 1. The repoint must come before the adoption.
                //    `adopt_custom_event` fetches teams and the schedule
                //    through the live client, so adopting first would pull the
                //    new event's data from the *previous* site — the silent
                //    mismatch this feature exists to remove — or 401 against it.
                // 2. `commit_source` must come before the adoption test, which
                //    reads the committed `self.source`. The other way round it
                //    sees the pre-apply value and skips adoption, which is the
                //    MANUAL -> CUSTOM dead end that left the operator with a
                //    valid address on screen and no usable control.
                // 3. Every early return in the match above skips this block
                //    entirely, and three unrelated mechanisms are all that keep
                //    a pending repoint from reaching one:
                //    - The Game arm's four confirmations. Three require a game
                //      in progress, which `refuse_repoint` has already turned
                //      back; the fourth requires the incomplete remote state
                //      that leaves the Game page's APPLY with no press action.
                //    - The App arm's `PortalTenantSwitch`, whose
                //      `RestartAndApply` handler commits the new source without
                //      repointing the live client, and is safe only because it
                //      always ends in a restart, and the restart rebuilds the
                //      client from the committed config.
                //    - Language/Main/User and the unusable-address exit, where
                //      no repoint can be pending at all: `pending_site_change`
                //      answers `None` for those pages, and `site_target`
                //      answers `None` for an address that will not parse.
                //    Weaken any of the three and an APPLY can commit a new
                //    source while the refbox goes on talking to the old site.
                //    See `refuse_repoint`.
                let mut task = Task::none();
                let moved_to_custom = new_site
                    .as_ref()
                    .is_some_and(|t| t.kind == SiteKind::Custom);
                let moved_to_portal = new_site
                    .as_ref()
                    .is_some_and(|t| t.kind == SiteKind::Portal);
                if let Some(target) = new_site {
                    self.repoint_client(target);
                    task = self.refresh_token_indicator();
                }
                // Adopt the custom site's event either when the site itself just
                // changed, or when CUSTOM is in use with no event yet — the
                // second covers coming back from MANUAL, which clears the event
                // while leaving the site address untouched.
                if moved_to_custom
                    || (self.source == GameSource::Custom && self.current_event_id.is_none())
                {
                    task = Task::batch(vec![task, self.adopt_custom_event()]);
                    // Re-seed the token indicator now that an event exists to
                    // check against. Changing source resets it to the rejected
                    // state, and until this line there was nothing to verify it
                    // with, so it would have sat there with a good saved token
                    // — keeping the court and game pickers greyed and the Game
                    // page's APPLY blocked for a reason that was an artifact.
                    task = Task::batch(vec![task, self.refresh_token_indicator()]);
                }
                // The refbox has just moved onto the portal, so the event the
                // operator staged before the move can finally be fetched. This
                // is the other half of the guard in `ParameterSelected::Event`:
                // that skipped the fetch because the client was elsewhere, and
                // without this line the operator is left with an event selected,
                // COURT stuck on "loading" and no control that would fill it.
                //
                // Gated on `moved_to_portal`, not `self.source`, so this does
                // not re-run on every later APPLY once the refbox is already on
                // the portal — only the APPLY that actually repoints the client
                // fires it. The `site_serves` conjunct still earns its place:
                // `repoint_client` returns early without updating
                // `self.current_site` when `build_site_client` fails, so a
                // Portal-committed source can still be talking to a stale
                // client, and the fetch must stay suppressed in that case too.
                if moved_to_portal && site_serves(self.current_site.kind, GameSource::Portal) {
                    if let Some(event_id) = self.current_event_id.clone() {
                        task = Task::batch(vec![
                            task,
                            self.request_teams_list(event_id.clone()),
                            self.request_schedule(event_id),
                        ]);
                    }
                }
                self.page_entry_snapshot = None;
                self.persist_config();
                // Keep the link note in step with the committed portal fields
                // (link/unlink, court, game) so a relaunch restores the right
                // state — or deletes the note when the portal was switched off.
                self.persist_link_session();
                self.navigate_to_parent(page);
                task
            }
            Message::CancelConfigPage(page) => {
                self.revert_from_snapshot();
                self.navigate_to_parent(page);
                Task::none()
            }
            Message::OpenUpdatesPage => {
                let backup = self.find_update_backup();
                self.update_backup_version = backup
                    .as_ref()
                    .and_then(|p| backup_version_from_filename(p));
                self.pending_update = None;
                self.app_state = AppState::Updates {
                    state: UpdateUiState::Unknown,
                    backup_available: backup.is_some(),
                };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::UpdatesCheck => {
                if let AppState::Updates { ref mut state, .. } = self.app_state {
                    *state = UpdateUiState::Checking;
                }
                Task::future(async move {
                    match crate::updater::net::check_latest().await {
                        Ok(info) => Message::UpdatesCheckDone(Ok(info)),
                        Err(e) => Message::UpdatesCheckDone(Err(updater_err_to_ui(e))),
                    }
                })
            }
            Message::UpdatesCheckDone(result) => {
                // Drop a stale result if the operator left the page or cancelled.
                if !matches!(
                    self.app_state,
                    AppState::Updates {
                        state: UpdateUiState::Checking,
                        ..
                    }
                ) {
                    return Task::none();
                }
                if let AppState::Updates { ref mut state, .. } = self.app_state {
                    match result {
                        Ok(info) => {
                            // The current crate version is a compile-time-constant valid
                            // semver, so parsing it is expected to succeed; a parse failure
                            // is treated as "not newer" rather than panicking.
                            let newer = match crate::updater::version::Version::parse(env!(
                                "CARGO_PKG_VERSION"
                            )) {
                                Some(current) => {
                                    info.version.cmp_to(&current) == std::cmp::Ordering::Greater
                                }
                                None => false,
                            };
                            if newer {
                                self.pending_update = Some(info);
                                *state = UpdateUiState::UpdateAvailable;
                            } else {
                                self.pending_update = None;
                                *state = UpdateUiState::UpToDate;
                            }
                        }
                        Err(e) => *state = UpdateUiState::Error(e),
                    }
                }
                Task::none()
            }
            Message::UpdatesConfirmInstall => {
                // Triggered by the Install button in the bottom-right of the
                // footer when an update is available. Defensive re-check: never
                // start an update if a game began.
                if self.snapshot.current_period != GamePeriod::BetweenGames {
                    self.app_state = AppState::EditGameConfig(ConfigPage::App);
                    trace!("AppState changed to {:?}", self.app_state);
                    return Task::none();
                }
                let info = self.pending_update.clone();
                let temp = self.updater_temp_path();
                match (info, temp) {
                    (Some(info), Some(temp)) => {
                        if let AppState::Updates { ref mut state, .. } = self.app_state {
                            *state = UpdateUiState::Downloading;
                        }
                        let binary_url = info.binary_url.clone();
                        let checksum_url = info.checksum_url.clone();
                        Task::future(async move {
                            if let Err(e) =
                                crate::updater::net::download_to(&binary_url, &temp).await
                            {
                                return Message::UpdatesDownloaded(Err(updater_err_to_ui(e)));
                            }
                            match crate::updater::net::fetch_text(&checksum_url).await {
                                Ok(text) => {
                                    // The checksum file is "<hex>" (possibly "<hex>  name").
                                    let sum =
                                        text.split_whitespace().next().unwrap_or("").to_string();
                                    Message::UpdatesDownloaded(Ok(sum))
                                }
                                Err(e) => Message::UpdatesDownloaded(Err(updater_err_to_ui(e))),
                            }
                        })
                    }
                    _ => {
                        if let AppState::Updates { ref mut state, .. } = self.app_state {
                            *state = UpdateUiState::Error(UpdateUiError::BadDownload);
                        }
                        Task::none()
                    }
                }
            }
            Message::UpdatesDownloaded(result) => {
                // Drop a stale result if the operator left the page or cancelled
                // (Cancel during download must not proceed to verify/swap).
                if !matches!(
                    self.app_state,
                    AppState::Updates {
                        state: UpdateUiState::Downloading,
                        ..
                    }
                ) {
                    return Task::none();
                }
                match result {
                    Ok(checksum) => {
                        if let AppState::Updates { ref mut state, .. } = self.app_state {
                            *state = UpdateUiState::Verifying;
                        }
                        match self.updater_temp_path() {
                            Some(temp) => Task::future(async move {
                                let ok = tokio::task::spawn_blocking(move || {
                                    crate::updater::verify::verify_sha256(&temp, &checksum)
                                        .unwrap_or(false)
                                })
                                .await
                                .unwrap_or(false);
                                if ok {
                                    Message::UpdatesVerified(Ok(()))
                                } else {
                                    Message::UpdatesVerified(Err(UpdateUiError::BadDownload))
                                }
                            }),
                            None => {
                                if let AppState::Updates { ref mut state, .. } = self.app_state {
                                    *state = UpdateUiState::Error(UpdateUiError::BadDownload);
                                }
                                Task::none()
                            }
                        }
                    }
                    Err(e) => {
                        if let AppState::Updates { ref mut state, .. } = self.app_state {
                            *state = UpdateUiState::Error(e);
                        }
                        Task::none()
                    }
                }
            }
            Message::UpdatesVerified(result) => {
                // Drop a stale result if the operator left the page or cancelled
                // (Cancel during verify must not proceed to the binary swap).
                if !matches!(
                    self.app_state,
                    AppState::Updates {
                        state: UpdateUiState::Verifying,
                        ..
                    }
                ) {
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        let temp = self.updater_temp_path();
                        let install = self.install_path.clone();
                        let trying = self.pending_update.as_ref().map(|r| r.version);
                        let prev =
                            crate::updater::version::Version::parse(env!("CARGO_PKG_VERSION"));
                        let config_dir = self.config_dir.clone();
                        let argv = self.restart_argv.clone();
                        match (temp, install, trying, prev) {
                            (Some(temp), Some(install), Some(trying), Some(prev)) => {
                                if let AppState::Updates { ref mut state, .. } = self.app_state {
                                    *state = UpdateUiState::Installing;
                                }
                                Task::future(async move {
                                    let result = tokio::task::spawn_blocking(
                                        move || -> std::result::Result<(), UpdateUiError> {
                                            // Smoke test: the downloaded binary must start on this
                                            // machine (--self-check exits 0 before opening a window
                                            // or binding ports). Pass the replay argv it would be
                                            // restarted with; --self-check short-circuits before
                                            // those take effect.
                                            let ok = std::process::Command::new(&temp)
                                                .arg("--self-check")
                                                .args(&argv)
                                                .stdin(std::process::Stdio::null())
                                                .status()
                                                .map(|s| s.success())
                                                .unwrap_or(false);
                                            if !ok {
                                                return Err(UpdateUiError::BadDownload);
                                            }
                                            crate::updater::swap::swap_in_place(
                                                &install, &temp, &prev,
                                            )
                                            .map_err(|e| updater_io_to_ui(&e))?;
                                            // Best-effort: a failed marker write means the new
                                            // binary won't auto-revert, but it passed the smoke
                                            // test, so proceed with the restart.
                                            if let Err(e) = crate::updater::marker::write_trial(
                                                &config_dir,
                                                &trying,
                                                &prev,
                                            ) {
                                                warn!("Failed to write update trial marker: {e}");
                                            }
                                            Ok(())
                                        },
                                    )
                                    .await
                                    .unwrap_or(Err(UpdateUiError::BadDownload));
                                    Message::UpdatesInstalled(result)
                                })
                            }
                            _ => {
                                if let AppState::Updates { ref mut state, .. } = self.app_state {
                                    *state = UpdateUiState::Error(UpdateUiError::BadDownload);
                                }
                                Task::none()
                            }
                        }
                    }
                    Err(e) => {
                        if let AppState::Updates { ref mut state, .. } = self.app_state {
                            *state = UpdateUiState::Error(e);
                        }
                        Task::none()
                    }
                }
            }
            Message::UpdatesInstalled(result) => match result {
                Ok(()) => {
                    if let AppState::Updates {
                        ref mut state,
                        ref mut backup_available,
                    } = self.app_state
                    {
                        *state = UpdateUiState::Restarting;
                        *backup_available = true;
                    }
                    self.update_backup_version =
                        crate::updater::version::Version::parse(env!("CARGO_PKG_VERSION"));
                    for mut child in self.sim_children.drain(..) {
                        let _ = child.kill();
                    }
                    RESTART_PENDING.store(true, Ordering::Relaxed);
                    iced::exit()
                }
                Err(e) => {
                    if let AppState::Updates { ref mut state, .. } = self.app_state {
                        *state = UpdateUiState::Error(e);
                    }
                    Task::none()
                }
            },
            Message::UpdatesRevert => {
                if let AppState::Updates { ref mut state, .. } = self.app_state {
                    *state = UpdateUiState::RevertConfirm;
                }
                Task::none()
            }
            Message::UpdatesConfirmRevert => {
                let backup = self.find_update_backup();
                let install = self.install_path.clone();
                match (backup, install) {
                    (Some(backup), Some(install)) => {
                        // Clear the trial marker first so reverting can't itself be
                        // mistaken for a failed trial and auto-reverted on next boot.
                        let _ = crate::updater::marker::clear_trial(&self.config_dir);
                        match crate::updater::swap::revert(&install, &backup) {
                            Ok(()) => {
                                // Record the rollback before the tracked backup
                                // version is cleared. The running binary is the
                                // version being reverted *from*; `update_backup_version`
                                // is the one being restored *to*.
                                if let (Some(from), Some(to)) = (
                                    crate::updater::version::Version::parse(env!(
                                        "CARGO_PKG_VERSION"
                                    )),
                                    self.update_backup_version,
                                ) {
                                    info!("Reverted update: v{from} -> v{to}; restarting to apply");
                                }
                                if let AppState::Updates {
                                    ref mut state,
                                    ref mut backup_available,
                                } = self.app_state
                                {
                                    *state = UpdateUiState::Restarting;
                                    *backup_available = false;
                                }
                                self.update_backup_version = None;
                                for mut child in self.sim_children.drain(..) {
                                    let _ = child.kill();
                                }
                                RESTART_PENDING.store(true, Ordering::Relaxed);
                                iced::exit()
                            }
                            Err(e) => {
                                if let AppState::Updates { ref mut state, .. } = self.app_state {
                                    *state = UpdateUiState::Error(updater_io_to_ui(&e));
                                }
                                Task::none()
                            }
                        }
                    }
                    _ => {
                        if let AppState::Updates { ref mut state, .. } = self.app_state {
                            *state = UpdateUiState::Error(UpdateUiError::BadDownload);
                        }
                        Task::none()
                    }
                }
            }
            Message::UpdatesBack => {
                if let AppState::Updates { ref mut state, .. } = self.app_state {
                    match state {
                        UpdateUiState::Checking => *state = UpdateUiState::Unknown,
                        UpdateUiState::Downloading
                        | UpdateUiState::Verifying
                        | UpdateUiState::Installing => *state = UpdateUiState::UpdateAvailable,
                        UpdateUiState::RevertConfirm => *state = UpdateUiState::Unknown,
                        UpdateUiState::Restarting => {} // disabled — no-op
                        UpdateUiState::RolledBack => {
                            // The rollback notice was shown at startup, not reached
                            // from settings, so Back returns to the main screen.
                            self.app_state = AppState::MainPage;
                            trace!("AppState changed to {:?}", self.app_state);
                        }
                        UpdateUiState::Unknown
                        | UpdateUiState::UpToDate
                        | UpdateUiState::UpdateAvailable
                        | UpdateUiState::Error(_) => {
                            self.app_state = AppState::EditGameConfig(ConfigPage::App);
                            trace!("AppState changed to {:?}", self.app_state);
                        }
                    }
                }
                Task::none()
            }
            Message::UpdaterHealthyCheck => {
                // The app processed this message ~20s after launch, so it started
                // healthily. A trial marker still present here means this is the
                // first healthy run of a freshly self-installed binary, so record
                // the version change before clearing the marker below. (`trying` is
                // the version now running; `backup` is the one it replaced.)
                if let Some((trying, backup)) =
                    crate::updater::marker::trial_versions(&self.config_dir)
                {
                    info!("Self-update succeeded: now running v{trying} (updated from v{backup})");
                }
                // Clear any update trial marker so a later boot is not mistaken for
                // a failed trial. Idempotent / no-op if absent.
                if let Err(e) = crate::updater::marker::clear_trial(&self.config_dir) {
                    warn!("Failed to clear update trial marker: {e}");
                }
                Task::none()
            }
            Message::ConfigEditComplete => {
                // Per-page Apply/Cancel chrome is the only commit path after ADR 009
                // Tasks 8-13. ConfigEditComplete only fires `canceled: true` now (from
                // the Settings Main back button and other escape paths); it just exits
                // settings and drops the in-flight edit buffer.
                //
                // BeepTest mode no longer routes through EditGameConfig (it has its
                // own Settings hierarchy), so this exit path always returns to MainPage.
                self.edited_settings = None;
                self.app_state = AppState::MainPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::EditParameter(param) => {
                // Seed the editor from the in-progress edits when present (mirroring
                // the single_half handling below), so reopening a parameter shows the
                // value the operator just entered rather than the last-saved one.
                let config = self
                    .edited_settings
                    .as_ref()
                    .map(|s| &s.config)
                    .unwrap_or(&self.config.game);
                let single_half = config.single_half;
                let dur = match param {
                    LengthParameter::Half => config.half_play_duration,
                    LengthParameter::HalfTime => config.half_time_duration,
                    LengthParameter::GameBlock => config.game_block,
                    LengthParameter::MinimumBetweenGame => config.minimum_break,
                    LengthParameter::PreOvertime => config.pre_overtime_break,
                    LengthParameter::OvertimeHalf => config.ot_half_play_duration,
                    LengthParameter::OvertimeHalfTime => config.ot_half_time_duration,
                    LengthParameter::PreSuddenDeath => config.pre_sudden_death_duration,
                };
                self.app_state = AppState::ParameterEditor(param, dur, single_half);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ShowParameterHelp => {
                if let AppState::ParameterEditor(param, dur, single_half) = self.app_state {
                    self.app_state = AppState::ParameterEditorHelp(param, dur, single_half);
                    trace!("AppState changed to {:?}", self.app_state);
                }
                Task::none()
            }
            Message::CloseParameterHelp => {
                if let AppState::ParameterEditorHelp(param, dur, single_half) = self.app_state {
                    self.app_state = AppState::ParameterEditor(param, dur, single_half);
                    trace!("AppState changed to {:?}", self.app_state);
                }
                Task::none()
            }
            Message::SelectParameter(param) => {
                let index = match param {
                    ListableParameter::Event => Some(0),
                    ListableParameter::Court => Some(0),
                    ListableParameter::Game => self.schedule.as_ref().and_then(|schedule| {
                        let court = self
                            .edited_settings
                            .as_ref()
                            .and_then(|edit| edit.current_court.clone())?;

                        schedule
                            .games
                            .iter()
                            .filter(|(_, game)| game.court == court)
                            .enumerate()
                            .find(|(_, (game_num, _))| {
                                **game_num == self.edited_settings.as_ref().unwrap().game_number
                            })
                            .map(|(i, _)| i)
                    }),
                }
                .unwrap_or(0);
                self.app_state = AppState::ParameterList(param, index);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ParameterEditComplete { canceled } => {
                let mut task = Task::none();
                if !canceled {
                    match self.app_state {
                        AppState::ParameterEditor(param, dur, single_half) => {
                            let edited_settings = self.edited_settings.as_mut().unwrap();
                            match param {
                                LengthParameter::Half => {
                                    edited_settings.config.half_play_duration = dur;
                                    edited_settings.config.single_half = single_half;
                                }
                                LengthParameter::HalfTime => {
                                    edited_settings.config.half_time_duration = dur
                                }
                                LengthParameter::GameBlock => {
                                    edited_settings.config.game_block = dur
                                }
                                LengthParameter::MinimumBetweenGame => {
                                    edited_settings.config.minimum_break = dur
                                }
                                LengthParameter::PreOvertime => {
                                    edited_settings.config.pre_overtime_break = dur
                                }
                                LengthParameter::OvertimeHalf => {
                                    edited_settings.config.ot_half_play_duration = dur
                                }
                                LengthParameter::OvertimeHalfTime => {
                                    edited_settings.config.ot_half_time_duration = dur
                                }
                                LengthParameter::PreSuddenDeath => {
                                    edited_settings.config.pre_sudden_death_duration = dur
                                }
                            }
                        }
                        AppState::KeypadPage(KeypadPage::GameNumber, num) => {
                            let edited_settings = self.edited_settings.as_mut().unwrap();
                            edited_settings.game_number = num.to_string();
                        }
                        AppState::KeypadPage(KeypadPage::TeamTimeouts(len, per_half), num) => {
                            let edited_settings = self.edited_settings.as_mut().unwrap();
                            edited_settings.config.team_timeout_duration = len;
                            edited_settings.config.num_team_timeouts_allowed = num as u16;
                            edited_settings.config.timeouts_counted_per_half = per_half;
                        }
                        AppState::KeypadPage(
                            KeypadPage::PortalLogin(_, ref mut requested),
                            code,
                        ) => {
                            // Reachable two ways: the legacy edit-config flow
                            // (edited_settings is Some, with the just-picked
                            // event held there) and the portal-detail GO TO
                            // LOGIN flow (edited_settings is None, the
                            // previously-linked event lives on the running
                            // app). Update the form state if it exists; read
                            // the event id from edited settings first and
                            // fall back to the running app — mirrors the
                            // RecvPortalToken path below.
                            *requested = true;
                            if let Some(ref mut settings) = self.edited_settings {
                                settings.uwhportal_token_valid = None;
                            }
                            let event_id = self
                                .edited_settings
                                .as_ref()
                                .and_then(|s| s.current_event_id.clone())
                                .or_else(|| self.current_event_id.clone())
                                .expect("PortalLogin keypad reachable only with a linked event");
                            task = self.request_uwhportal_token(&event_id, code);
                        }
                        _ => unreachable!(),
                    }
                }

                // Where to land after Done depends on which path the operator
                // took to the keypad. The PortalLogin keypad reached from the
                // portal-detail flow has no edit-config session to return to,
                // so we route back to the detail page directly (Unit 7's
                // new branch). The RecvPortalToken handler will replace
                // this once the network request completes. All in-settings
                // routes return to Game Options per ADR 009 (Unit 3's
                // redesign of the post-keypad landing).
                let next_state = match self.app_state {
                    AppState::ParameterEditor(_, _, _) => {
                        AppState::EditGameConfig(ConfigPage::Game)
                    }
                    AppState::KeypadPage(KeypadPage::GameNumber, _) => {
                        AppState::EditGameConfig(ConfigPage::Game)
                    }
                    AppState::KeypadPage(KeypadPage::TeamTimeouts(_, _), _) => {
                        AppState::EditGameConfig(ConfigPage::Game)
                    }
                    AppState::KeypadPage(KeypadPage::PortalLogin(_, _), _) => {
                        if self.edited_settings.is_some() {
                            AppState::EditGameConfig(ConfigPage::Game)
                        } else {
                            AppState::PortalDetailPage { scroll_index: 0 }
                        }
                    }
                    AppState::ParameterList(param, _) => match param {
                        ListableParameter::Game => AppState::EditGameConfig(ConfigPage::Game),
                        ListableParameter::Event | ListableParameter::Court => {
                            AppState::EditGameConfig(ConfigPage::Game)
                        }
                    },
                    _ => unreachable!(),
                };

                self.app_state = next_state;
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::ParameterSelected(param, val) => {
                let task = match param {
                    ListableParameter::Event => {
                        let id = EventId::from_full(val).unwrap();
                        // why this cannot panic: the same precondition as the single
                        // borrow this replaces -- `ParameterSelected` is only reachable
                        // from a parameter list, which only exists while the editor does.
                        let edited_source = self.edited_settings.as_ref().unwrap().source;
                        // Whether the live client will actually be asked about
                        // this event — see `site_serves`. Shared by the token
                        // indicator below and the fetch batch at the end of
                        // this arm; both need the same answer.
                        let will_fetch = site_serves(self.current_site.kind, edited_source);
                        // Set the new event id and clear court / game number / schedule
                        // that were filtered by the previous event so the user re-picks
                        // against the new event's data.
                        // why this cannot panic: `ParameterSelected` is only reachable from a
                        // parameter list, which only exists while the editor does -- the same
                        // precondition as the single borrow these four replaced.
                        self.edited_settings
                            .as_mut()
                            .unwrap()
                            .select_event(id.clone());

                        // Only resolve (or reset) the token indicator when a
                        // fetch will actually follow to settle it. Setting it to
                        // the checking state when the fetch is suppressed would
                        // promise a check that is not coming — the same untruth
                        // `EditableSettings::clear_for_remote_switch` avoids by
                        // resetting to the rejected state rather than `None`.
                        // Here, when suppressed, the previous verdict is left
                        // standing.
                        //
                        // A source button now moves the client at the tap, so
                        // the suppressed case is no longer "before the APPLY
                        // that repoints": what remains is a custom site with no
                        // usable address saved, where the SITE page's APPLY is
                        // what moves the client, or a repoint that failed.
                        if will_fetch {
                            // Whether a key is on file for the event just staged, read from the
                            // store. The shared client's key belongs to the linked event and says
                            // nothing about this one.
                            let have_key = self.uwhportal_client.is_some()
                                && key_for_event(
                                    &self.config,
                                    self.current_site.base_url.expose(),
                                    &id,
                                )
                                .is_some();
                            // why this cannot panic: see `select_event` above.
                            self.edited_settings.as_mut().unwrap().uwhportal_token_valid =
                                if have_key { None } else { Some(false) };
                        }

                        if let Some(pools) = self
                            .events
                            .get(self.active_source(), &id)
                            .and_then(|e| e.courts.as_ref())
                        {
                            if pools.len() == 1 {
                                if let Some(ref mut edits) = self.edited_settings {
                                    edits.current_court = Some(pools[0].clone());
                                }
                            }
                        }
                        // Only fetch when the refbox is actually on this
                        // source's site. The source buttons move the client
                        // themselves now, so this is no longer the ordinary
                        // "chosen before the APPLY that moves it there" case —
                        // it is left for the two states where the client did
                        // not move: a custom site with no usable address (the
                        // SITE page's APPLY fetches instead) and a failed
                        // repoint.
                        if will_fetch {
                            Task::batch(vec![
                                self.check_uwhportal_auth(&id),
                                // Teams for this event only. They used to arrive in
                                // the batch fired from `RecvEventList` for every
                                // event at once; that burst is gone, so the game
                                // picker's team names now depend on this line.
                                self.request_teams_list(id.clone()),
                                self.request_schedule(id),
                            ])
                        } else {
                            Task::none()
                        }
                    }
                    ListableParameter::Court => {
                        // Set the new court and clear the game number that was filtered
                        // by the previous court so the user re-picks from the new
                        // court's filtered list.
                        // why this cannot panic: see the Event arm above.
                        self.edited_settings.as_mut().unwrap().select_court(val);
                        Task::none()
                    }
                    ListableParameter::Game => {
                        // why this cannot panic: see the Event arm above.
                        self.edited_settings.as_mut().unwrap().game_number = val;
                        Task::none()
                    }
                };

                let next_page = match param {
                    ListableParameter::Event
                    | ListableParameter::Court
                    | ListableParameter::Game => ConfigPage::Game,
                };

                self.app_state = AppState::EditGameConfig(next_page);
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::ToggleBoolParameter(param) => {
                match param {
                    BoolGameParameter::TeamWarning => {
                        if let AppState::KeypadPage(
                            KeypadPage::WarningAdd {
                                ref mut team_warning,
                                ..
                            },
                            _,
                        ) = self.app_state
                        {
                            *team_warning ^= true
                        } else {
                            unreachable!()
                        }
                        trace!("AppState changed to {:?}", self.app_state)
                    }

                    BoolGameParameter::TeamScore => {
                        if let AppState::KeypadPage(
                            KeypadPage::AddScore {
                                ref mut team_score, ..
                            },
                            ref mut player_num,
                        ) = self.app_state
                        {
                            *team_score ^= true;
                            if *team_score {
                                // The goal is the team's, so no player number
                                // may linger: the readout, the greyed panel and
                                // the score that gets recorded must agree.
                                *player_num = 0;
                            }
                        } else {
                            unreachable!()
                        }
                        trace!("AppState changed to {:?}", self.app_state)
                    }

                    BoolGameParameter::TimeoutsCountedPerHalf => {
                        if let AppState::KeypadPage(
                            KeypadPage::TeamTimeouts(_, ref mut per_half),
                            _,
                        ) = self.app_state
                        {
                            *per_half ^= true
                        } else {
                            unreachable!()
                        }
                        trace!("AppState changed to {:?}", self.app_state)
                    }

                    BoolGameParameter::SingleHalf => {
                        // Staged inside the Half Length parameter editor (the
                        // 2 Halves / 1 Period selector), so flip the editor's
                        // staged bool in place — it commits on Done / discards
                        // on Cancel, like the edited Duration.
                        if let AppState::ParameterEditor(_, _, ref mut single_half) = self.app_state
                        {
                            *single_half ^= true
                        } else {
                            unreachable!()
                        }
                        trace!("AppState changed to {:?}", self.app_state)
                    }

                    _ => {
                        let edited_settings = self.edited_settings.as_mut().unwrap();
                        match param {
                            BoolGameParameter::OvertimeAllowed => {
                                edited_settings.config.overtime_allowed ^= true
                            }
                            BoolGameParameter::SuddenDeathAllowed => {
                                edited_settings.config.sudden_death_allowed ^= true
                            }
                            BoolGameParameter::WhiteOnRight => {
                                edited_settings.white_on_right ^= true
                            }
                            BoolGameParameter::SoundEnabled => {
                                edited_settings.sound.sound_enabled ^= true
                            }
                            BoolGameParameter::RefAlertEnabled => {
                                edited_settings.sound.whistle_enabled ^= true
                            }
                            BoolGameParameter::AutoSoundStartPlay => {
                                edited_settings.sound.auto_sound_start_play ^= true
                            }
                            BoolGameParameter::AutoSoundStopPlay => {
                                edited_settings.sound.auto_sound_stop_play ^= true
                            }
                            BoolGameParameter::HideTime => edited_settings.hide_time ^= true,
                            BoolGameParameter::ScorerCapNum => {
                                edited_settings.collect_scorer_cap_num ^= true
                            }
                            BoolGameParameter::FoulsAndWarnings => {
                                edited_settings.track_fouls_and_warnings ^= true
                            }
                            BoolGameParameter::ForceKeypadNumbers => {
                                edited_settings.force_keypad_numbers ^= true
                            }
                            BoolGameParameter::ShowBehindScheduleTime => {
                                edited_settings.show_behind_schedule_time ^= true
                            }
                            BoolGameParameter::TeamWarning
                            | BoolGameParameter::TeamScore
                            | BoolGameParameter::TimeoutsCountedPerHalf
                            | BoolGameParameter::SingleHalf => {
                                unreachable!()
                            }
                            BoolGameParameter::ConfirmScore => {
                                edited_settings.confirm_score ^= true
                            }
                            BoolGameParameter::AudibleCountdown => {
                                edited_settings.audible_countdown ^= true
                            }
                            BoolGameParameter::ManualAlarmEnabled => {
                                edited_settings.sound.manual_alarm_enabled ^= true
                            }
                        }
                    }
                };
                // No boolean parameter triggers a fetch any more: the only one
                // that did was the portal toggle, now Message::SelectGameSource.
                Task::none()
            }
            Message::CustomSiteUrlChanged(typed) => {
                let url = typed.into_inner();
                // why this cannot panic: the URL editor is only reachable from
                // the Game Options editor, which populates `edited_settings`
                // before it can be drawn.
                self.edited_settings.as_mut().unwrap().custom_site.url = url;
                // Typing clears a previous rejection: the operator is already
                // acting on it, so leaving the message up would be nagging.
                if let AppState::EditGameConfig(ConfigPage::CustomSite(true)) = self.app_state {
                    self.app_state = AppState::EditGameConfig(ConfigPage::CustomSite(false));
                }
                Task::none()
            }
            Message::SelectGameSource(new_source) => {
                // why this cannot panic: the source control is only reachable
                // from the Game Options editor, which populates
                // `edited_settings` before it can be drawn.
                let edited_settings = self.edited_settings.as_mut().unwrap();
                let was_using = edited_settings.uses_remote();
                edited_settings.source = new_source;

                // `remembered_remote` is deliberately NOT updated here. It is
                // recorded on Apply instead, from the source actually applied,
                // so a choice the operator cancels cannot survive as a hidden
                // preference. See the plan's deviation note.

                // Per ADR 017: on manual -> remote, start the pickers from a
                // blank slate.
                if !was_using && edited_settings.uses_remote() {
                    edited_settings.clear_for_remote_switch();
                }
                if was_using && !edited_settings.uses_remote() {
                    // remote -> manual is a clean slate.
                    edited_settings.current_event_id = None;
                    edited_settings.current_court = None;
                    edited_settings.schedule = None;
                    edited_settings.game_number = String::new();
                }

                // Refresh the list on every switch into PORTAL, including from
                // CUSTOM — which used to fetch nothing at all, because both
                // sources counted as "remote" and the transition fell between
                // the two branches above. That is what left the custom site's
                // event standing in the portal's picker. Deliberately NOT
                // conditioned on the previous source: an operator returning to
                // PORTAL wants the current list, and a stale one is what sent
                // them to a game that no longer exists.
                if new_source == GameSource::Portal {
                    self.request_event_list()
                } else {
                    Task::none()
                }
            }
            Message::SwitchGameSource(target) => {
                // Tapping the source already in use is not a switch: there is
                // nothing to move, nothing to clear and nothing to confirm.
                if target == self.source {
                    return Task::none();
                }
                // A game in progress, not merely a running clock: between
                // games the clock counts down to the next game and is running
                // by default, which is exactly when an operator sets the source
                // up. Same test `refuse_repoint` applies.
                //
                // Bound to a local rather than inlined into the `match` below:
                // the lock guard is a temporary that would otherwise live to the
                // end of the match block, and the switch arm needs `&mut self`
                // before then.
                let game_in_progress = self.tm.lock().current_period() != GamePeriod::BetweenGames;
                let results_queued = self.portal_manager.has_queued_items();
                // Completeness is judged on what is DISPLAYED — the staged
                // values in the editor. `selection_owned` is the same test the
                // GAME tile uses to decide whether to show a number at all, so
                // the prompt cannot fire over a game the operator cannot see.
                let fully_linked = self.edited_settings.as_ref().is_some_and(|edited| {
                    let client_serves_staged = site_serves(self.current_site.kind, edited.source);
                    let selection_owned = self
                        .events
                        .owns(edited.source, edited.current_event_id.as_ref())
                        && client_serves_staged;
                    selection_owned && !edited.uwhportal_incomplete()
                });
                match source_tap_outcome(game_in_progress, results_queued, fully_linked) {
                    // Both refusals reuse the existing pages, and carry
                    // `ConfigPage::Game` so their one button returns the
                    // operator to the page the buttons live on.
                    SourceTapOutcome::RefusedByGame => {
                        self.app_state = AppState::ConfirmationPage(
                            ConfirmationKind::SiteLockedByGame(ConfigPage::Game),
                        );
                        trace!("AppState changed to {:?}", self.app_state);
                        Task::none()
                    }
                    SourceTapOutcome::RefusedByQueue => {
                        self.app_state = AppState::ConfirmationPage(
                            ConfirmationKind::SiteLockedByQueue(ConfigPage::Game),
                        );
                        trace!("AppState changed to {:?}", self.app_state);
                        Task::none()
                    }
                    SourceTapOutcome::Confirm => {
                        self.app_state = AppState::ConfirmationPage(
                            ConfirmationKind::SourceSwitchClearsSelection(target),
                        );
                        trace!("AppState changed to {:?}", self.app_state);
                        Task::none()
                    }
                    SourceTapOutcome::SwitchNow => self.switch_to_source(target),
                }
            }
            Message::CycleParameter(param) => {
                let settings = &mut self.edited_settings.as_mut().unwrap();
                match param {
                    CyclingParameter::RemoteBuzzerSound(idx) => {
                        settings.sound.remotes[idx].sound.cycle()
                    }
                    CyclingParameter::AlertVolume => settings.sound.whistle_vol.cycle(),
                    CyclingParameter::AboveWaterVol => settings.sound.above_water_vol.cycle(),
                    CyclingParameter::UnderWaterVol => settings.sound.under_water_vol.cycle(),
                    CyclingParameter::Mode => settings.mode.cycle(),
                    CyclingParameter::Brightness => settings.brightness.cycle(),
                    CyclingParameter::FrontDisplayLayout => settings.front_display_layout.cycle(),
                }
                Task::none()
            }
            Message::SelectLanguage(lang) => {
                self.edited_settings.as_mut().unwrap().pending_language = Some(lang);
                Task::none()
            }
            Message::LanguageSelectComplete { canceled } => {
                let settings = self.edited_settings.as_mut().unwrap();
                if !canceled {
                    if let Some(lang) = settings.pending_language {
                        let original = settings.original_language.unwrap_or(Language::English);
                        let needs_restart = original.ui_font() != lang.ui_font();
                        self.config.language = Some(lang);
                        if let Err(e) = confy::store(crate::APP_NAME, None, &self.config) {
                            error!("Failed to persist config: {e}");
                        }
                        if needs_restart {
                            // Kill every simulator child so they do not linger as
                            // orphans after the iced runtime closes its windows.
                            for mut child in self.sim_children.drain(..) {
                                let _ = child.kill();
                            }
                            // Mark the restart and let iced gracefully close its
                            // windows. `main()` will spawn a fresh copy of the
                            // exe after the iced runtime returns — this avoids
                            // the brief overlap of old and new windows that a
                            // synchronous `std::process::exit(0)` would produce.
                            RESTART_PENDING.store(true, Ordering::Relaxed);
                            return iced::exit();
                        }
                        // Apply the new language to the running UI (same font family, no restart needed).
                        crate::request_language(&crate::LANGUAGE_LOADER, &[lang.as_lang_id()]);
                    }
                }
                settings.pending_language = None;
                settings.original_language = None;
                // This path is only reachable from the hockey/rugby Language
                // sub-page inside EditGameConfig. BeepTest has its own
                // language picker (`BeepTestLanguageApply` /
                // `BeepTestLanguageCancel`) so it never lands here.
                if let AppState::EditGameConfig(ref mut page) = self.app_state {
                    *page = ConfigPage::Main;
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::SelectBuzzer(sound) => {
                if let Some(edited) = self.edited_settings.as_mut() {
                    edited.sound.buzzer_sound = sound;
                }
                Task::none()
            }
            Message::TestBuzzer => {
                if let Some(edited) = self.edited_settings.as_ref() {
                    self.sound.test_buzzer(edited.sound.buzzer_sound);
                }
                Task::none()
            }
            Message::RequestRemoteId => {
                let task =
                    if let AppState::EditGameConfig(ConfigPage::Remotes(_, ref mut listening)) =
                        self.app_state
                    {
                        *listening = true;
                        Task::future(self.sound.request_next_remote_id()).map(|maybe_id| {
                            if let Some(id) = maybe_id {
                                Message::GotRemoteId(id)
                            } else {
                                Message::NoAction
                            }
                        })
                    } else {
                        unreachable!()
                    };
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::GotRemoteId(id) => {
                if let AppState::EditGameConfig(ConfigPage::Remotes(_, ref mut listening)) =
                    self.app_state
                {
                    self.edited_settings
                        .as_mut()
                        .unwrap()
                        .sound
                        .remotes
                        .push(RemoteInfo { id, sound: None });
                    *listening = false;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::DeleteRemote(index) => {
                if let Some(ref mut settings) = self.edited_settings {
                    settings.sound.remotes.remove(index);
                } else {
                    unreachable!()
                }
                Task::none()
            }
            Message::ConfirmationSelected(selection) => {
                // The site-locked refusals carry the page they were raised
                // from and offer one button, which returns the operator there
                // with their edit still staged — nothing was committed and
                // nothing is discarded.
                if let AppState::ConfirmationPage(
                    ConfirmationKind::SiteLockedByGame(page)
                    | ConfirmationKind::SiteLockedByQueue(page),
                ) = self.app_state
                {
                    self.app_state = AppState::EditGameConfig(page);
                    trace!("AppState changed to {:?}", self.app_state);
                    return Task::none();
                }

                // The link refusal carries no page: the ACCESS TOKEN row exists
                // only on the Game config page, so that is where it returns.
                if matches!(
                    self.app_state,
                    AppState::ConfirmationPage(ConfirmationKind::LinkLockedByGame)
                ) {
                    self.app_state = AppState::EditGameConfig(ConfigPage::Game);
                    trace!("AppState changed to {:?}", self.app_state);
                    return Task::none();
                }

                // The source-switch confirmation is its own two-option page and
                // is deliberately not routed through `apply_game_confirmation`,
                // whose options all mean something about a game config this page
                // never touches. Both buttons land back on the Game page, where
                // the source buttons are.
                if let AppState::ConfirmationPage(ConfirmationKind::SourceSwitchClearsSelection(
                    target,
                )) = self.app_state
                {
                    return match selection {
                        ConfirmationOption::SwitchSource => {
                            // The clock kept running while this page was up, and
                            // the between-games clock starts the next game by
                            // itself when it expires — so the check made when the
                            // button was tapped may now be stale. Re-take it here
                            // or a switch confirmed seconds earlier repoints the
                            // results client and resets the clock underneath a
                            // game that has since started. `switch_to_source`
                            // documents BetweenGames as a precondition, and
                            // `reset_to_manual_break` does not set the period, so
                            // it would leave the running period with a
                            // break-length clock.
                            match self.refuse_repoint(ConfigPage::Game) {
                                Some(kind) => {
                                    self.app_state = AppState::ConfirmationPage(kind);
                                    trace!("AppState changed to {:?}", self.app_state);
                                    Task::none()
                                }
                                None => {
                                    self.app_state = AppState::EditGameConfig(ConfigPage::Game);
                                    trace!("AppState changed to {:?}", self.app_state);
                                    self.switch_to_source(target)
                                }
                            }
                        }
                        // Cancel. Nothing was committed, so there is nothing to
                        // put back — source, link, clock and client are all
                        // untouched.
                        _ => {
                            self.app_state = AppState::EditGameConfig(ConfigPage::Game);
                            trace!("AppState changed to {:?}", self.app_state);
                            Task::none()
                        }
                    };
                }

                if matches!(
                    self.app_state,
                    AppState::ConfirmationPage(
                        ConfirmationKind::GameConfigChangedFromApply(_)
                            | ConfirmationKind::GameNumberChangedFromApply
                            | ConfirmationKind::UwhPortalIncompleteFromApply
                            | ConfirmationKind::PortalTenantSwitch { .. }
                            | ConfirmationKind::SwitchToManualFromApply
                    )
                ) {
                    return self.apply_game_confirmation(selection);
                }

                // After ADR 009 Task 13 retired the global apply path, only
                // `ConfirmationKind::Error` (which offers DiscardChanges) and
                // `ConfirmationKind::UwhPortalLinkFailed` /
                // `ConfirmationKind::UwhPortalKeyUnusable` (which offer GoBack)
                // reach this match. The Game-related and PortalTenantSwitch
                // confirmations are dispatched to apply_game_confirmation above.
                self.app_state = match selection {
                    ConfirmationOption::DiscardChanges => AppState::MainPage,
                    ConfirmationOption::GoBack => AppState::KeypadPage(
                        KeypadPage::PortalLogin(
                            // why this cannot panic: this branch only runs after a
                            // portal link attempt, which requires a successfully
                            // constructed client; the guard is held only for `id()`.
                            self.uwhportal_client.as_ref().unwrap().lock().unwrap().id(),
                            false,
                        ),
                        0,
                    ),
                    ConfirmationOption::EndGameAndApply | ConfirmationOption::KeepGameAndApply => {
                        unreachable!(
                            "EndGameAndApply / KeepGameAndApply are only offered by *FromApply \
                             ConfirmationKind variants, which are dispatched above to \
                             apply_game_confirmation."
                        )
                    }
                    ConfirmationOption::RestartAndApply => {
                        unreachable!(
                            "RestartAndApply is only offered by PortalTenantSwitch pages, \
                             which are dispatched above to apply_game_confirmation."
                        )
                    }
                    ConfirmationOption::SwitchSource => {
                        unreachable!(
                            "SwitchSource is only offered by the SourceSwitchClearsSelection \
                             page, which is dispatched above."
                        )
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ConfirmScores(snapshot) => {
                let mut task = Task::none();
                if self.config.confirm_score {
                    task = self.apply_snapshot(snapshot);
                    self.app_state = AppState::ConfirmScores(self.snapshot.scores);
                    trace!("AppState changed to {:?}", self.app_state);
                } else {
                    let mut tm = self.tm.lock();
                    let now = Instant::now();
                    if let Err(e) = tm.end_confirm_pause(now) {
                        // See `Message::ScoreConfirmation`: the updater can have ended this
                        // pause already, leaving the page up. Not a crash.
                        debug!("Confirm pause had already ended: {e}");
                    }
                    tm.start_clock(now);
                    tm.update(now + Duration::from_millis(2)).unwrap(); // Need to update after game ends
                    self.app_state = AppState::MainPage;
                    trace!("AppState changed to {:?}", self.app_state);
                }
                task
            }
            Message::ScoreConfirmation { correct } => {
                info!("Manual Score confirmation");
                self.app_state = if let AppState::ConfirmScores(scores) = self.app_state {
                    if correct {
                        let now = Instant::now();
                        let mut tm = self.tm.lock();

                        tm.set_scores(scores, now);
                        if let Err(e) = tm.end_confirm_pause(now) {
                            // The background updater can end this pause first. If its tick
                            // ended the pause but then failed to build a snapshot, the
                            // message that would have closed this page was lost with the
                            // error, so the page is still up while the engine has moved on.
                            // Acting on a pause that is already over gives the operator the
                            // outcome they asked for, so carry on rather than crash.
                            debug!("Confirm pause had already ended: {e}");
                        }
                        AppState::MainPage
                    } else {
                        AppState::ScoreEdit {
                            scores,
                            is_confirmation: true,
                        }
                    }
                } else {
                    unreachable!()
                };

                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::AutoConfirmScores(snapshot) => {
                info!("Autoconfirming");

                let task = self.apply_snapshot(snapshot);

                self.app_state = AppState::MainPage;

                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::TeamTimeout(color, switch) => {
                let mut tm = self.tm.lock();
                let now = Instant::now();
                if switch {
                    tm.switch_to_team_timeout(color).unwrap();
                } else {
                    tm.start_team_timeout(color, now).unwrap();
                }
                if let AppState::TimeEdit(_, _, ref mut time) = self.app_state {
                    *time = Some(tm.timeout_clock_time(now).unwrap());
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
            Message::RefTimeout(switch) => {
                let mut tm = self.tm.lock();
                let now = Instant::now();
                if switch {
                    tm.switch_to_ref_timeout(now).unwrap();
                } else {
                    tm.start_ref_timeout(now).unwrap();
                }
                if let AppState::TimeEdit(_, _, ref mut time) = self.app_state {
                    *time = Some(tm.timeout_clock_time(now).unwrap());
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
            Message::PenaltyShot(switch) => {
                let mut tm = self.tm.lock();
                let now = Instant::now();
                if switch {
                    if self.config.mode == Mode::Rugby {
                        tm.switch_to_rugby_penalty_shot(now).unwrap();
                    } else {
                        tm.switch_to_penalty_shot().unwrap();
                    }
                } else if self.config.mode == Mode::Rugby {
                    tm.start_rugby_penalty_shot(now).unwrap();
                } else {
                    tm.start_penalty_shot(now).unwrap();
                }
                if let AppState::TimeEdit(_, _, ref mut time) = self.app_state {
                    *time = Some(tm.timeout_clock_time(now).unwrap());
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
            Message::EndTimeout => {
                let mut tm = self.tm.lock();
                let now = Instant::now();
                let would_end = tm.timeout_end_would_end_game(now).unwrap();
                if would_end {
                    // Arm the end-of-game confirm pause so the confirm screen (below) can finish
                    // the game cleanly; a bare halt would leave `end_confirm_pause` with no pause
                    // to end, panicking on confirm (R6).
                    tm.end_game_ending_timeout(now).unwrap();
                } else {
                    tm.end_timeout(now).unwrap();
                    tm.update(now).unwrap();
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                let task = self.apply_snapshot(snapshot);

                if would_end {
                    self.app_state = AppState::ConfirmScores(self.snapshot.scores);
                    trace!("AppState changed to {:?}", self.app_state);
                }

                if let AppState::TimeEdit(_, _, ref mut timeout) = self.app_state {
                    *timeout = None;
                }
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::CancelTimeout => {
                let mut tm = self.tm.lock();
                let now = Instant::now();
                // The Cancel button is only rendered for an active team timeout
                // inside its grace window, so this call is valid by construction.
                // Unlike EndTimeout, no `timeout_end_would_end_game` check is needed:
                // a team timeout only happens mid-half with the game clock stopped, so
                // cancelling it can never end the game — it just resumes play.
                tm.cancel_team_timeout(now).unwrap();
                tm.update(now).unwrap();
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
            Message::RecvEventList(e_list) => {
                let mut tasks = vec![];
                let e_map: BTreeMap<_, _> = e_list.into_iter().map(|e| (e.id.clone(), e)).collect();
                // Teams are NOT fetched here any more. This handler used to fire
                // one teams request per event, which was affordable only while
                // the list itself was fetched solely on opting in; now that the
                // list loads whatever the source, it would mean dozens of
                // requests for events nobody opens. They are fetched for the
                // event the operator picks instead — see
                // `ParameterSelected::Event` — which is the deferral ADR 017
                // parked in its Q2.
                self.events.set_portal_list(e_map);
                // Startup link restore: now that the event list is populated,
                // fetch the schedule for the restored event so RecvSchedule can
                // re-select the remembered game and start its scheduled countdown.
                if let Some(event_id) = self.pending_restore_schedule.take() {
                    let in_list = self.events.owns(GameSource::Portal, Some(&event_id));
                    if in_list {
                        tasks.push(self.request_teams_list(event_id.clone()));
                        tasks.push(self.request_schedule(event_id));
                    } else {
                        warn!(
                            "Restore event {} not in fetched event list; not re-linking schedule",
                            event_id.full()
                        );
                    }
                }
                Task::batch(tasks)
            }
            Message::RecvTeamsList(event_id, teams, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    warn!(
                        "Discarding the teams list for event {}: it was fetched from \
                         site generation {}, and the refbox is now on {}",
                        event_id.full(),
                        issued_at,
                        self.site_generation
                    );
                    return Task::none();
                }
                // Resolves against the COMMITTED source, not the staged one:
                // staging alone never moves the client, so a reply arriving
                // after a merely staged source change still belongs to the
                // committed one.
                let source = self.reply_source();
                if let Some(event) = self.events.get_mut(source, &event_id) {
                    event.teams = Some(teams);
                } else if source == GameSource::Portal && !self.events.portal_list_loaded() {
                    error!(
                        "Received teams for event_id {}, but there is no event list yet",
                        event_id.full()
                    );
                } else {
                    error!(
                        "Received teams for event_id {}, it is not in the event list",
                        event_id.full()
                    );
                }
                Task::none()
            }
            Message::RecvTeamRoster(team_id, numbers, issued_at) => {
                // Roster fetches go out in a batch, one per team in the
                // schedule, so a switch made while they are in flight would
                // otherwise let the departed site's replies refill the cache
                // `switch_to_source` has just cleared. `RecvSchedule` skips
                // re-fetching any team it already holds, so such an entry would
                // then shadow the new site's numbers for the rest of the
                // session and survive a REFRESH.
                if reply_is_current(issued_at, self.site_generation) {
                    self.team_rosters.insert(team_id, numbers);
                } else {
                    warn!(
                        "Discarding the roster for team {}: it was fetched from site \
                         generation {}, and the refbox is now on {}",
                        team_id.full(),
                        issued_at,
                        self.site_generation
                    );
                }
                Task::none()
            }
            Message::RecvSchedule(event_id, mut schedule, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    warn!(
                        "Discarding the schedule for event {}: it was fetched from \
                         site generation {}, and the refbox is now on {}",
                        event_id.full(),
                        issued_at,
                        self.site_generation
                    );
                    return Task::none();
                }
                // A manual REFRESH (RequestPortalRefresh) spins the Game Info
                // button until a schedule arrives. Clear it for every success
                // path here, not just the between-games branch below.
                if let AppState::GameDetailsPage(ref mut is_refreshing) = self.app_state {
                    *is_refreshing = false;
                }
                if let Some(id) = self.current_event_id.as_ref().or_else(|| {
                    self.edited_settings
                        .as_ref()
                        .and_then(|edits| edits.current_event_id.as_ref())
                }) {
                    if id.full() != event_id.full() {
                        warn!(
                            "Received event data, but for the wrong event_id: {}",
                            event_id.full()
                        )
                    }
                } else {
                    warn!("Received event data, but there is no current event_id");
                }

                schedule
                    .games
                    .sort_by(|_, v1, _, v2| v1.start_time.cmp(&v2.start_time));

                // Pre-load every team in the schedule while there is time and
                // network, so the grid is available from the first game of the
                // day and no game start depends on a live fetch. `RecvSchedule`
                // is not once-per-link: handle_game_end requests a fresh
                // schedule at the end of every portal-linked game, and REFRESH,
                // event selection and startup restore all trigger it too. Skip
                // teams already in the roster cache so a 40-team event does not
                // re-fire ~40 concurrent GETs at the end of every game, right
                // when that game's score/stats POST is being enqueued.
                let mut roster_tasks: Vec<Task<Message>> = Vec::new();
                let mut seen_teams = BTreeSet::new();
                let mut courts = BTreeSet::new();
                for game in schedule.games.values() {
                    for team in [&game.dark, &game.light] {
                        if let Some(id) = team.assigned() {
                            if seen_teams.insert(id.clone()) && !self.team_rosters.contains_key(id)
                            {
                                roster_tasks.push(self.request_team_roster(id.clone()));
                            }
                        }
                    }
                    if !courts.contains(&game.court) {
                        courts.insert(game.court.clone());
                    }
                }
                let courts: Vec<_> = courts.into_iter().collect();

                if let Some(ref mut edits) = self.edited_settings {
                    if edits.should_adopt_auto_court(&event_id, courts.len()) {
                        edits.current_court = Some(courts[0].clone());
                    }
                }

                // Resolves against the COMMITTED source, not the staged one:
                // staging alone never moves the client, so a reply arriving
                // after a merely staged source change still belongs to the
                // committed one.
                let source = self.reply_source();
                if let Some(event) = self.events.get_mut(source, &event_id) {
                    event.courts = Some(courts);
                    event.schedule = Some(schedule.clone());
                    if let Some(ref mut edits) = self.edited_settings {
                        if let Some(ref id) = edits.current_event_id {
                            if *id == event_id {
                                edits.schedule = Some(schedule.clone());
                            }
                        }
                    }
                    if let Some(ref id) = self.current_event_id {
                        if *id == event_id {
                            self.schedule = Some(schedule);
                            if self.edited_settings.is_none() {
                                let mut tm = self.tm.lock();
                                if tm.current_period() == GamePeriod::BetweenGames {
                                    // The one-shot is consumed here, whatever the
                                    // outcome, so a restore applies exactly once.
                                    let restore_num = self.pending_restore_game.take();
                                    // Safety: `self.schedule` was assigned from `schedule` two lines above.
                                    let schedule = self.schedule.as_ref().unwrap();
                                    // A blank next-game number is the engine's
                                    // "this court is finished" state, where the
                                    // break clock is held stopped at 0:00. Noted
                                    // before anything changes it, so that leaving
                                    // that state can restart the clock below.
                                    let was_court_finished = tm.next_game_number().is_empty();
                                    let decision = next_game_from_schedule(
                                        schedule,
                                        restore_num.as_ref(),
                                        tm.next_game_info().as_ref().map(|info| &info.number),
                                        self.last_played.as_ref(),
                                        self.last_played_start,
                                        self.current_court.as_deref(),
                                    );
                                    let found = match decision {
                                        NextGameFromSchedule::Game(ref number) => {
                                            schedule.get_game_and_timing(number)
                                        }
                                        NextGameFromSchedule::CourtFinished
                                        | NextGameFromSchedule::NothingScheduled
                                        | NextGameFromSchedule::NeedsPick
                                        | NextGameFromSchedule::Unknown => (None, None),
                                    };

                                    if let (Some(game), Some(timing)) = found {
                                        info!(
                                            "Setting upcoming game info from received schedule: {game:?}"
                                        );
                                        tm.set_next_game(NextGameInfo {
                                            number: game.number.clone(),
                                            timing: Some(timing.clone()),
                                            start_time: Some(game.start_time),
                                        });
                                        let leaving_finished_state =
                                            was_court_finished && !tm.clock_is_running();
                                        if restore_num.is_some() || leaving_finished_state {
                                            // Start the live countdown to the
                                            // scheduled start so a restored session
                                            // is ready to go (same path the normal
                                            // between-games transition uses). A
                                            // court coming back to life needs the
                                            // same treatment: its break clock is
                                            // held stopped at 0:00, which idles the
                                            // time updater, so without this the
                                            // table would stay dashed, START NOW
                                            // greyed and the clock frozen.
                                            let now = Instant::now();
                                            // why this cannot panic: BetweenGames was
                                            // just checked and next_game was just set.
                                            tm.apply_next_game_start(now).unwrap();
                                            let new_game_config = tm.config().clone();
                                            let snapshot = tm.generate_snapshot(now).unwrap();
                                            std::mem::drop(tm);
                                            self.config.game = new_game_config;
                                            roster_tasks.push(self.apply_snapshot(snapshot));
                                            return Task::batch(roster_tasks);
                                        }
                                    } else {
                                        match decision {
                                            // Both are definite "nothing is next
                                            // here" answers, and both park the
                                            // clock. Kept apart above so an empty
                                            // court is never recorded as a
                                            // completed one.
                                            NextGameFromSchedule::CourtFinished
                                            | NextGameFromSchedule::NothingScheduled => {
                                                tm.set_no_next_game();
                                            }
                                            // Nothing is known well enough to act
                                            // on. Leave the engine as it is and let
                                            // the operator pick; with the portal
                                            // linked, `next_game_number` already
                                            // refuses to invent one.
                                            NextGameFromSchedule::NeedsPick
                                            | NextGameFromSchedule::Unknown => {}
                                            NextGameFromSchedule::Game(_) => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if source == GameSource::Portal && !self.events.portal_list_loaded() {
                    error!(
                        "Received schedule for event_id {}, but there is no event list yet",
                        event_id.full()
                    );
                } else {
                    error!(
                        "Received schedule for event_id {}, it is not in the event list",
                        event_id.full()
                    );
                }
                Task::batch(roster_tasks)
            }
            Message::RecvPortalToken(token_response, issued_for_event, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    // The answer came from a site the refbox has left, so it is
                    // dropped whole, before anything reads it. Two things below
                    // depend on that: `set_token` would install the departed
                    // site's key on the client now pointing elsewhere, and the
                    // config slot is chosen from `current_site`, which is
                    // assigned in the same two lines of `repoint_client` that
                    // bump the generation — so a current generation is exactly
                    // what makes that choice correct.
                    //
                    // The failure arms are dropped too. A "login failed" page
                    // for a site the operator has walked away from is the same
                    // wrong-site attribution, thrown over the page they are on
                    // now. Returning here also leaves `app_state` untouched:
                    // the handler otherwise routes to the Game page, which
                    // would pull the operator out of wherever the switch took
                    // them.
                    //
                    // Silent by design — a log line and nothing on screen. The
                    // operator is already looking at the new site's true token
                    // state (`refresh_token_indicator` runs on the switch), and
                    // the cost of the drop is one re-login, not a wrong answer.
                    warn!(
                        "Discarding a login answer: it was issued against site \
                         generation {}, and the refbox is now on {}",
                        issued_at, self.site_generation
                    );
                    return Task::none();
                }
                let mut task = Task::none();
                self.app_state = match token_response {
                    PortalTokenResponse::Success(token) => {
                        // The site's reply is not trusted text. A custom site is
                        // somebody else's server, and a key carrying a character a
                        // header cannot hold used to take the refbox down on the
                        // next call. One decision point, not two: the client itself
                        // refuses the key, so there is no way for the check and the
                        // store to disagree and let a request go out with no
                        // credential attached. Refusing here also keeps the key out
                        // of the config file, so it cannot come back at the next
                        // launch.
                        let token = token.trim().to_string();
                        // Judged without touching the shared client: installing the key to test
                        // it would put one event's credential on the handle the background
                        // uploader uses for another's.
                        //
                        // Empty is refused separately because `check_access_key` accepts it -- it
                        // only rejects characters a header cannot carry. Filed, `""` would be
                        // reported as a successful login while every reader treated it as no key
                        // at all: fetches uncredentialed, uploads refused, and nothing said so.
                        let refused = if token.is_empty() {
                            warn!("The site returned an empty access key");
                            true
                        } else if let Some(why) = check_access_key(&token).err() {
                            warn!("The site returned an access key that cannot be sent: {why}");
                            true
                        } else {
                            false
                        };
                        if refused {
                            AppState::ConfirmationPage(ConfirmationKind::UwhPortalKeyUnusable)
                        } else {
                            info!("Portal token request succeeded");
                            // Save it against the exact site and event that
                            // issued it, never against whatever event happens
                            // to be selected now -- the operator may have
                            // switched events while the login was in flight,
                            // and a key filed under the wrong event is a key
                            // that cannot work.
                            if !file_login_key(
                                &mut self.config,
                                self.current_site.base_url.expose(),
                                &issued_for_event,
                                issued_at,
                                self.site_generation,
                                token,
                            ) {
                                // Unreachable in normal flow — the guard at the top of this
                                // handler has already returned. Logged rather than ignored so
                                // that if it ever does fire, the reason a login silently failed
                                // to stick is in the log rather than nowhere.
                                warn!("A login answer went stale before its key could be filed");
                            } else {
                                // Into the store the background uploader reads. It resolves a
                                // key per call, so a newly filed one has to reach it before the
                                // next queued result goes out.
                                self.publish_access_keys();
                                // Save immediately. The handler routes to the Game page, so a
                                // key filed only in memory is lost if the operator backs out
                                // without applying, or the machine loses power — and returning
                                // to that event would need a fresh code from the portal
                                // website, which is the whole thing this store exists to avoid.
                                self.persist_config();
                            }
                            if let Some(ref mut settings) = self.edited_settings {
                                // Only for the event this key was issued for. If the operator
                                // staged a different event while the login was in flight, nothing
                                // here has checked *that* event, and painting the row green over
                                // it is the same untruth the suppressed-fetch case avoids. Its
                                // own verdict, set when it was staged, is left standing.
                                if settings.current_event_id.as_ref() == Some(&issued_for_event) {
                                    settings.uwhportal_token_valid = Some(true);
                                }
                            }

                            // Tell the portal manager the token is healthy
                            // again: clears the token-known-problem flag and
                            // resets queue-item attempt counters so pending
                            // items resume retrying on the next background
                            // tick. Errors here are logged but not
                            // propagated — the in-memory state is already
                            // correct and the operator has no actionable
                            // recovery from an I/O failure at this point.
                            if let Err(e) = self.portal_manager.token_refreshed() {
                                error!("portal_manager.token_refreshed failed: {e}");
                            }

                            if let Some(event_id) = self
                                .edited_settings
                                .as_ref()
                                .and_then(|settings| settings.current_event_id.as_ref())
                                .or(self.current_event_id.as_ref())
                            {
                                info!("Requesting schedule for event_id: {}", event_id.full());
                                task = self.request_schedule(event_id.clone())
                            }

                            // A successful re-login lands on the edit-config
                            // Game page (the portal parameters page).
                            AppState::EditGameConfig(ConfigPage::Game)
                        }
                    }
                    r @ PortalTokenResponse::NoPendingLink
                    | r @ PortalTokenResponse::InvalidCode => {
                        warn!("Portal token request failed: {:?}", r);
                        AppState::ConfirmationPage(ConfirmationKind::UwhPortalLinkFailed(r))
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::RecvTokenValid(event_id, valid, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    warn!(
                        "Discarding the token verdict for event {}: it was checked \
                         against site generation {}, and the refbox is now on {}",
                        event_id.full(),
                        issued_at,
                        self.site_generation
                    );
                    return Task::none();
                }
                if let Some(ref mut settings) = self.edited_settings {
                    // Drop a stale reply for an event the operator has since
                    // switched away from, so a late "valid" for a previous
                    // event can't paint a false OK for the current one. The
                    // schedule and auto-court paths already guard on event id.
                    if settings.current_event_id.as_ref() == Some(&event_id) {
                        settings.uwhportal_token_valid = Some(valid);
                    }
                }
                Task::none()
            }
            Message::StartClock => {
                self.tm.lock().start_clock(Instant::now());
                Task::none()
            }
            Message::StopClock => {
                self.tm.lock().stop_clock(Instant::now()).unwrap();
                Task::none()
            }
            Message::TimeUpdaterStarted(tx) => {
                // The updater's end of this handshake was hardened; this end is the same
                // handshake and runs on the UI thread, where a crash kills the process
                // outright and no lock recovery can help. If the updater has already gone
                // away there is nothing to hand the game state to, and saying so beats
                // taking the app down.
                if tx.blocking_send(self.tm.clone()).is_err() {
                    error!(
                        "Clock updater went away before it could be given the game state; \
                         the game clock will not advance"
                    );
                }
                Task::none()
            }
            Message::AlarmPressed => {
                // Mouse press on the alarm button.
                // Uniform hold model: always schedule a delay; duration depends on game state.
                if !(self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled) {
                    return Task::none();
                }
                if self.mouse_alarm_held {
                    return Task::none();
                }
                let was_active = self.spacebar_held;
                self.mouse_alarm_held = true;
                if was_active {
                    return Task::none();
                }
                let hold_duration = self.manual_alarm_hold_duration();
                self.alarm_delay_token += 1;
                let token = self.alarm_delay_token;
                info!(
                    "Manual alarm delay started (mouse), duration={hold_duration:?}, token={token}"
                );
                Task::future(async move {
                    sleep(hold_duration).await;
                    Message::AlarmDelayElapsed(token)
                })
            }
            Message::AlarmReleased => {
                // Mouse release — stop alarm only when spacebar is also not held.
                if self.mouse_alarm_held {
                    self.mouse_alarm_held = false;
                    if !self.spacebar_held {
                        info!("Manual alarm released (mouse)");
                        self.sound.stop_manual_buzzer();
                    }
                }
                Task::none()
            }
            Message::SpacebarPressed => {
                // Keyboard press — spacebar_held guards against OS key-repeat.
                if !(self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled) {
                    return Task::none();
                }
                // Spec: spacebar has no effect on screens other than the main game screen.
                // The subscription captures spacebar globally; this gate enforces the spec
                // restriction in the handler so text inputs and other screens are unaffected.
                if !matches!(self.app_state, AppState::MainPage) {
                    return Task::none();
                }
                if self.spacebar_held {
                    return Task::none();
                }
                let was_active = self.mouse_alarm_held;
                self.spacebar_held = true;
                if was_active {
                    return Task::none();
                }
                let hold_duration = self.manual_alarm_hold_duration();
                self.alarm_delay_token += 1;
                let token = self.alarm_delay_token;
                info!(
                    "Manual alarm delay started (spacebar), duration={hold_duration:?}, token={token}"
                );
                Task::future(async move {
                    sleep(hold_duration).await;
                    Message::AlarmDelayElapsed(token)
                })
            }
            Message::SpacebarReleased => {
                // Keyboard release — stop alarm only when mouse is also not held.
                if self.spacebar_held {
                    self.spacebar_held = false;
                    if !self.mouse_alarm_held {
                        info!("Manual alarm released (spacebar)");
                        self.sound.stop_manual_buzzer();
                    }
                }
                Task::none()
            }
            Message::AlarmDelayElapsed(token) => {
                // Fires after the per-state hold delay (150ms in active play, 1s otherwise).
                // Only start the sound if the token still matches (no newer press has
                // superseded this one) and at least one input is still held.
                if token == self.alarm_delay_token && (self.mouse_alarm_held || self.spacebar_held)
                {
                    info!("Manual alarm started after delay, token={token}");
                    self.sound.start_manual_buzzer();
                }
                Task::none()
            }
            Message::TimeoutRevivePressed(color) => {
                // Press-down on a used-up (greyed) team timeout button: begin the
                // 3-second revive hold. The view only attaches this on an eligible button.
                if matches!(&self.timeout_revive, Some(h) if h.color == color) {
                    return Task::none();
                }
                self.timeout_revive_token += 1;
                let token = self.timeout_revive_token;
                self.timeout_revive = Some(ReviveHold {
                    color,
                    phase: RevivePhase::Reviving,
                    token,
                });
                info!("Timeout-revive hold started for {color}, token={token}");
                Task::future(async move {
                    sleep(TIMEOUT_REVIVE_HOLD_DURATION).await;
                    Message::TimeoutReviveHoldElapsed(token, color)
                })
            }
            Message::TimeoutReviveReleased(color) => {
                // Finger up, or pointer left the button. In Reviving this cancels
                // (nothing given back); in Restored it confirms the already-revived timeout.
                if matches!(&self.timeout_revive, Some(h) if h.color == color) {
                    self.timeout_revive = None;
                    info!("Timeout-revive hold released for {color}");
                }
                Task::none()
            }
            Message::TimeoutReviveHoldElapsed(token, color) => {
                // The 3-second revive hold elapsed. Only proceed if this is still the
                // current Reviving hold for this team.
                if !matches!(
                    &self.timeout_revive,
                    Some(h) if h.color == color
                        && h.token == token
                        && h.phase == RevivePhase::Reviving
                ) {
                    return Task::none();
                }
                let mut tm = self.tm.lock();
                let now = Instant::now();
                if tm.revive_team_timeout(color).is_err() {
                    // State moved on during the hold (e.g. half ended); nothing to do.
                    std::mem::drop(tm);
                    self.timeout_revive = None;
                    return Task::none();
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                let apply_task = self.apply_snapshot(snapshot);
                // Enter the "restored, hold to keep showing" state. It has no timer:
                // it persists until the finger is lifted, and release confirms the restore.
                // The token is still bumped so any stray in-flight hold timer is ignored;
                // Restored itself never waits on it.
                self.timeout_revive_token += 1;
                let token = self.timeout_revive_token;
                self.timeout_revive = Some(ReviveHold {
                    color,
                    phase: RevivePhase::Restored,
                    token,
                });
                info!("Timeout revived for {color}; awaiting release to confirm, token={token}");
                apply_task
            }
            Message::BeepTestStart => {
                // Distinguish "fresh start" from "resume from pause" using
                // beep_test_has_run, not current_period. During the warm-up
                // (current_period == Level(0)), both the fresh state AND the
                // paused state have the same period, so keying off the period
                // would clobber the paused remaining time. The has_run flag
                // is the right signal: false → operator pressing START for
                // the first time (fresh-start the warmup countdown), true →
                // operator pressing RESUME (preserve clock_state).
                let was_run_already = self.beep_test_has_run;
                self.beep_test_has_run = true;
                if let Some(ref mut bt_tm) = self.beep_test_tm {
                    let now = Instant::now();
                    if was_run_already {
                        // Resume — start_clock preserves the Stopped
                        // clock_time (the paused remaining time).
                        bt_tm.start_clock(now);
                    } else if let Err(e) = bt_tm.start_beep_test_now(now) {
                        error!("Failed to start beep test: {e}");
                    }
                }
                Task::none()
            }
            Message::BeepTestStop => {
                if let Some(ref mut bt_tm) = self.beep_test_tm {
                    if let Err(e) = bt_tm.stop_clock(Instant::now()) {
                        error!("Failed to stop beep-test clock: {e}");
                    }
                }
                Task::none()
            }
            Message::BeepTestReset => {
                self.reset_beep_test_state(Instant::now());
                Task::none()
            }
            Message::BeepTestTick => {
                // Drives the cadence engine forward, ships the snapshot to the
                // LED panel, and triggers any whistles/buzzers at the same
                // boundaries the standalone beep-test would.
                let now = Instant::now();
                let (completed, new_snapshot) = match self.beep_test_tm {
                    Some(ref mut bt_tm) => {
                        if let Err(e) = bt_tm.update(now) {
                            error!("Beep-test engine update failed: {e}");
                        }
                        (bt_tm.take_completed(), bt_tm.generate_snapshot(now))
                    }
                    None => return Task::none(),
                };

                // The schedule ran off its end: return the page to idle so the
                // next group can start without the operator pressing RESET.
                if completed {
                    info!("Beep test complete — resetting to idle");
                    self.reset_beep_test_state(now);
                    return Task::none();
                }

                // generate_snapshot returns None when the clock time would be
                // negative; nothing to ship this tick. The next tick recovers.
                let Some(new_snapshot) = new_snapshot else {
                    return Task::none();
                };
                self.maybe_play_beep_test_sound(&new_snapshot);
                // The LED panel pipeline accepts the full GameSnapshot;
                // synthesize one from the beep-test snapshot the same way the
                // existing `BeepTestSnapshot -> GameSnapshotNoHeap` conversion
                // does (BetweenGames + lap_count as white score).
                let game_snap = GameSnapshot {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: new_snapshot.secs_in_period,
                    scores: BlackWhiteBundle {
                        black: 0,
                        white: new_snapshot.lap_count,
                    },
                    ..Default::default()
                };
                if let Err(e) = self.update_sender.send_snapshot(
                    game_snap,
                    // Beep test has no sides control: lap count always on the left.
                    false,
                    self.config.hardware.brightness,
                ) {
                    // Channel-full or closed: the next tick re-sends a fresh
                    // snapshot, so dropping one is acceptable.
                    warn!("Failed to send beep-test snapshot to LED panel: {e:?}");
                }
                self.beep_test_snapshot = new_snapshot;
                Task::none()
            }
            Message::BeepTestCycleDisplayLayout => {
                // Session-only: advance the in-memory beep-test layout and push
                // it to the display. Never written to config (resets to Default
                // on the next boot).
                self.beep_test_display_layout = self.beep_test_display_layout.next();
                let effective = crate::sim_frame::effective_beep_layout(
                    self.has_led_panel,
                    self.beep_test_display_layout,
                );
                if let Err(e) = self.update_sender.set_layout(effective) {
                    warn!("Failed to push beep-test display layout: {e:?}");
                }
                Task::none()
            }
            Message::BeepTestOpenSettings => {
                // Seed `edited_settings.mode` with the current mode so the
                // App Mode cycle button on the Settings landing has a value
                // to read/mutate. Other sub-pages (Sound, Edit Levels,
                // Language) overwrite `edited_settings` on their own entry,
                // so this seeding is safe.
                let current_language =
                    Language::from_lang_id(&crate::LANGUAGE_LOADER.current_languages()[0]);
                let edited_settings = EditableSettings {
                    config: self.tm.lock().config().clone(),
                    game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                        self.snapshot.next_game_number.clone()
                    } else {
                        self.snapshot.game_number.clone()
                    },
                    white_on_right: self.config.hardware.white_on_right,
                    brightness: self.config.hardware.brightness,
                    front_display_layout: self.config.front_display_layout,
                    source: self.source,
                    remembered_remote: self.config.remembered_remote,
                    custom_site: self.config.custom_site.clone(),
                    uwhportal_token_valid: None,
                    current_event_id: self.current_event_id.clone(),
                    current_court: self.current_court.clone(),
                    schedule: self.schedule.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
                    force_keypad_numbers: self.config.force_keypad_numbers,
                    show_behind_schedule_time: self.config.show_behind_schedule_time,
                    confirm_score: self.config.confirm_score,
                    audible_countdown: self.config.audible_countdown,
                    pending_language: Some(current_language),
                    original_language: Some(current_language),
                    beep_test_levels: None,
                    selected_level: 0,
                };
                self.edited_settings = Some(edited_settings);
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestCloseSettings => {
                // Discard any staged edits (including the seeded mode) and
                // return to the BeepTest main view.
                self.edited_settings = None;
                self.app_state = AppState::BeepTestPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestEditOpenLanguage => {
                // Seed `edited_settings` with current language fields so the
                // BeepTest Language picker can stage a selection. Other
                // fields are filled with defaults / current-state mirrors;
                // the sub-page only reads `pending_language` and
                // `original_language`, so the rest is inert.
                let current_language =
                    Language::from_lang_id(&crate::LANGUAGE_LOADER.current_languages()[0]);
                let edited_settings = EditableSettings {
                    config: self.tm.lock().config().clone(),
                    game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                        self.snapshot.next_game_number.clone()
                    } else {
                        self.snapshot.game_number.clone()
                    },
                    white_on_right: self.config.hardware.white_on_right,
                    brightness: self.config.hardware.brightness,
                    front_display_layout: self.config.front_display_layout,
                    source: self.source,
                    remembered_remote: self.config.remembered_remote,
                    custom_site: self.config.custom_site.clone(),
                    uwhportal_token_valid: None,
                    current_event_id: self.current_event_id.clone(),
                    current_court: self.current_court.clone(),
                    schedule: self.schedule.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
                    force_keypad_numbers: self.config.force_keypad_numbers,
                    show_behind_schedule_time: self.config.show_behind_schedule_time,
                    confirm_score: self.config.confirm_score,
                    audible_countdown: self.config.audible_countdown,
                    pending_language: Some(current_language),
                    original_language: Some(current_language),
                    beep_test_levels: None,
                    selected_level: 0,
                };
                self.edited_settings = Some(edited_settings);
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Language);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestLanguageCancel => {
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestLanguageApply => {
                // Commit the staged language. Mirrors
                // `LanguageSelectComplete { canceled: false }`: when the
                // font family changes, the app restarts; otherwise we
                // apply the new language to the running UI in place.
                let lang_opt = self
                    .edited_settings
                    .as_ref()
                    .and_then(|e| e.pending_language);
                let original = self
                    .edited_settings
                    .as_ref()
                    .and_then(|e| e.original_language)
                    .unwrap_or(Language::English);
                if let Some(lang) = lang_opt {
                    let needs_restart = original.ui_font() != lang.ui_font();
                    self.config.language = Some(lang);
                    if let Err(e) = confy::store(crate::APP_NAME, None, &self.config) {
                        error!("Failed to persist config: {e}");
                    }
                    if needs_restart {
                        // Kill every simulator child so they do not linger as
                        // orphans after the iced runtime closes its windows.
                        for mut child in self.sim_children.drain(..) {
                            let _ = child.kill();
                        }
                        // Mark the restart and let iced gracefully close its
                        // windows. `main()` will spawn a fresh copy of the
                        // exe after the iced runtime returns.
                        RESTART_PENDING.store(true, Ordering::Relaxed);
                        self.app_state = AppState::MainPage;
                        trace!("AppState changed to {:?}", self.app_state);
                        return iced::exit();
                    }
                    // Apply the new language to the running UI (same font family).
                    crate::request_language(&crate::LANGUAGE_LOADER, &[lang.as_lang_id()]);
                }
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestEditOpenSound => {
                // Seed `edited_settings` with a clone of the current sound
                // settings so the existing `ToggleBoolParameter` /
                // `CycleParameter` handlers (which mutate
                // `edited_settings.sound`) can be reused unchanged. Other
                // fields are filled with defaults / current-state mirrors;
                // the sub-page only reads `sound`, so the rest is inert.
                let current_language =
                    Language::from_lang_id(&crate::LANGUAGE_LOADER.current_languages()[0]);
                let edited_settings = EditableSettings {
                    config: self.tm.lock().config().clone(),
                    game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                        self.snapshot.next_game_number.clone()
                    } else {
                        self.snapshot.game_number.clone()
                    },
                    white_on_right: self.config.hardware.white_on_right,
                    brightness: self.config.hardware.brightness,
                    front_display_layout: self.config.front_display_layout,
                    source: self.source,
                    remembered_remote: self.config.remembered_remote,
                    custom_site: self.config.custom_site.clone(),
                    uwhportal_token_valid: None,
                    current_event_id: self.current_event_id.clone(),
                    current_court: self.current_court.clone(),
                    schedule: self.schedule.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
                    force_keypad_numbers: self.config.force_keypad_numbers,
                    show_behind_schedule_time: self.config.show_behind_schedule_time,
                    confirm_score: self.config.confirm_score,
                    audible_countdown: self.config.audible_countdown,
                    pending_language: Some(current_language),
                    original_language: Some(current_language),
                    beep_test_levels: None,
                    selected_level: 0,
                };
                self.edited_settings = Some(edited_settings);
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Sound);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestSoundSettingsSave => {
                // Commit staged sound edits to live config, push them to the
                // sound controller, and persist to disk — mirroring the
                // ConfigPage::Sound Apply path used by the hockey-mode Sound
                // sub-page.
                self.apply_sound_options();
                self.persist_config();
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestSoundSettingsCancel => {
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestEditOpenBuzzer => {
                // Navigate from the BeepTest Sound sub-page to the Buzzer
                // picker. `edited_settings` is already seeded by
                // `BeepTestEditOpenSound` — do NOT re-seed it here.
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Buzzer);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestSelectBuzzer(sound) => {
                // Stage the tapped sound into edited_settings.
                if let Some(edited) = self.edited_settings.as_mut() {
                    edited.sound.buzzer_sound = sound;
                }
                Task::none()
            }
            Message::BeepTestTestBuzzer => {
                // Play the currently-staged buzzer sound without committing.
                if let Some(edited) = self.edited_settings.as_ref() {
                    self.sound.test_buzzer(edited.sound.buzzer_sound);
                }
                Task::none()
            }
            Message::BeepTestBuzzerSave => {
                // Return to Sound sub-page keeping the staged selection;
                // `BeepTestSoundSettingsSave` will persist it.
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Sound);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestBuzzerCancel => {
                // Revert the staged sound to the live value, then return.
                if let Some(edited) = self.edited_settings.as_mut() {
                    edited.sound.buzzer_sound = self.config.sound.buzzer_sound;
                }
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Sound);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestEditOpenLevels => {
                // Seed `edited_settings` with a clone of the live level list
                // and `selected_level = 0` so the Edit Levels page can mutate
                // the staged copy in isolation. Other fields are filled with
                // defaults / current-state mirrors mirroring
                // `BeepTestEditOpenSound`; the sub-page only reads
                // `beep_test_levels` and `selected_level`, so the rest is inert.
                let current_language =
                    Language::from_lang_id(&crate::LANGUAGE_LOADER.current_languages()[0]);
                let edited_settings = EditableSettings {
                    config: self.tm.lock().config().clone(),
                    game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                        self.snapshot.next_game_number.clone()
                    } else {
                        self.snapshot.game_number.clone()
                    },
                    white_on_right: self.config.hardware.white_on_right,
                    brightness: self.config.hardware.brightness,
                    front_display_layout: self.config.front_display_layout,
                    source: self.source,
                    remembered_remote: self.config.remembered_remote,
                    custom_site: self.config.custom_site.clone(),
                    uwhportal_token_valid: None,
                    current_event_id: self.current_event_id.clone(),
                    current_court: self.current_court.clone(),
                    schedule: self.schedule.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
                    force_keypad_numbers: self.config.force_keypad_numbers,
                    show_behind_schedule_time: self.config.show_behind_schedule_time,
                    confirm_score: self.config.confirm_score,
                    audible_countdown: self.config.audible_countdown,
                    pending_language: Some(current_language),
                    original_language: Some(current_language),
                    beep_test_levels: Some(self.config.beep_test.levels.clone()),
                    selected_level: 0,
                };
                self.edited_settings = Some(edited_settings);
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::EditLevels);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestEditSelectLevel(idx) => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref levels) = edited.beep_test_levels {
                        if idx < levels.len() {
                            edited.selected_level = idx;
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditCountInc => {
                if let Some(ref mut edited) = self.edited_settings {
                    let sel = edited.selected_level;
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if let Some(level) = levels.get_mut(sel) {
                            if level.count < MAX_LAPS_PER_LEVEL {
                                level.count = level.count.saturating_add(1);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditCountDec => {
                if let Some(ref mut edited) = self.edited_settings {
                    let sel = edited.selected_level;
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if let Some(level) = levels.get_mut(sel) {
                            if level.count > 1 {
                                level.count -= 1;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditDurationInc => {
                if let Some(ref mut edited) = self.edited_settings {
                    let sel = edited.selected_level;
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if let Some(level) = levels.get_mut(sel) {
                            level.duration = level
                                .duration
                                .saturating_add(std::time::Duration::from_secs(1));
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditDurationDec => {
                if let Some(ref mut edited) = self.edited_settings {
                    let sel = edited.selected_level;
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if let Some(level) = levels.get_mut(sel) {
                            if level.duration > std::time::Duration::from_secs(1) {
                                level.duration -= std::time::Duration::from_secs(1);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditAddLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if levels.len() < MAX_LEVELS {
                            let new_level = levels
                                .get(edited.selected_level)
                                .cloned()
                                .unwrap_or_else(|| crate::config::Level {
                                    count: 4,
                                    duration: std::time::Duration::from_secs(20),
                                });
                            let insert_at = (edited.selected_level + 1).min(levels.len());
                            levels.insert(insert_at, new_level);
                            edited.selected_level = insert_at;
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditRemoveLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    let sel = edited.selected_level;
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if levels.len() > 1 && sel < levels.len() {
                            levels.remove(sel);
                            edited.selected_level = sel.saturating_sub(1);
                        }
                    }
                }
                Task::none()
            }
            Message::BeepTestEditSelectPreset(preset) => {
                // Replace the staged levels wholesale. This is a staged edit
                // like any other on this page — the Apply footer commits it and
                // Cancel discards it. Selection returns to the first level
                // because the previously-selected index may no longer exist.
                // preset.config() also returns `pre` (the warm-up), which is
                // discarded here: `pre` isn't operator-editable anywhere and is
                // 10 seconds for every preset, so only `levels` is staged.
                if let Some(ref mut edited) = self.edited_settings {
                    if edited.beep_test_levels.is_some() {
                        edited.beep_test_levels = Some(preset.config().levels);
                        edited.selected_level = 0;
                    }
                }
                Task::none()
            }
            Message::BeepTestEditLevelsSave => {
                if let Some(ref edited) = self.edited_settings {
                    if let Some(ref levels) = edited.beep_test_levels {
                        self.config.beep_test.levels = levels.clone();
                    }
                }
                self.persist_config();
                // The engine holds its own copy of the schedule — push the
                // edited one in, or the next run counts down the old times.
                let beep_test_config = self.config.beep_test.clone();
                let now = Instant::now();
                if let Some(ref mut bt_tm) = self.beep_test_tm {
                    bt_tm.set_config(beep_test_config, now);
                }
                // set_config() only resets the engine — bring the app-side
                // has_run/snapshot state back to idle too, so this stays
                // consistent if a future entry point (e.g. a court-length
                // preset) reaches set_config() while has_run is true.
                self.reset_beep_test_state(now);
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestEditLevelsCancel => {
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::BeepTestRestartToApply => {
                // Commit the staged mode (cycled directly on the Settings
                // landing). When it differs from the current mode, follow
                // the same exec-restart sequence used by the hockey-mode
                // PortalTenantSwitch RestartAndApply arm: clear the linked
                // event (so the portal-health task stops probing the old
                // tenant), flush the portal retry queue (items queued
                // under the old tenant cannot be delivered to the new
                // one), persist the config to disk, kill simulator
                // children, mark the restart, and exit iced. `main()`
                // spawns a fresh copy of the exe after the runtime
                // returns, avoiding the brief overlap of old + new windows
                // that a synchronous `std::process::exit(0)` would produce.
                let new_mode = self
                    .edited_settings
                    .as_ref()
                    .map(|e| e.mode)
                    .unwrap_or(self.config.mode);
                self.edited_settings = None;
                if new_mode != self.config.mode {
                    self.config.mode = new_mode;
                    self.set_current_event_id(None);
                    // Portal only, for the reason given in the RestartAndApply
                    // arm: a custom site is the same address either side of a
                    // mode change, so its queued results stay deliverable.
                    if self.source == GameSource::Portal {
                        if let Err(e) = self.portal_manager.flush_queue_for_tenant_switch() {
                            error!("Failed to flush portal queue before restart: {e}");
                        }
                    }
                    if let Err(e) = confy::store(APP_NAME, None, &self.config) {
                        error!("Failed to persist config before restart: {e}");
                    }
                    for mut child in self.sim_children.drain(..) {
                        let _ = child.kill();
                    }
                    RESTART_PENDING.store(true, Ordering::Relaxed);
                    self.app_state = AppState::MainPage;
                    trace!("AppState changed to {:?}", self.app_state);
                    return iced::exit();
                }
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::NoAction => Task::none(),
        }
    }

    pub(super) fn view(&self) -> Element<'_, Message> {
        // During Game Config edit, the operator may have picked a new event whose
        // teams have already loaded into the EventStore but whose commit is still
        // pending Apply. Resolve teams against the in-edit event id AND the staged
        // source (when present) so the picker shows real team names during edit;
        // fall back to the committed event id and source outside of edit.
        let active_event_id = self
            .edited_settings
            .as_ref()
            .and_then(|edits| edits.current_event_id.as_ref())
            .or(self.current_event_id.as_ref());
        let data = ViewData {
            snapshot: &self.snapshot,
            mode: self.config.mode,
            source: self.source,
            clock_running: self.tm.lock().clock_is_running(),
            teams: active_event_id.and_then(|id| {
                self.events
                    .get(self.active_source(), id)
                    .and_then(|event| event.teams.as_ref())
            }),
            // The portal health indicator is dormant whenever Using-UWH-Portal
            // is off OR no event is linked. `Some` only when the feature is on
            // AND an event is linked; otherwise `None` and the time banner
            // falls back to the pre-feature layout. See ADR 011 amendments
            // 2026-04-23 (event-linked gate) and 2026-05-16 (using-uwh-portal gate).
            portal_indicator: if self.uses_remote() {
                self.current_event_id.as_ref().map(|_| {
                    let mut state = self.portal_manager.indicator_state();
                    // The committed source, not the one staged in the editor:
                    // the tile reports the live connection, so choosing CUSTOM
                    // must not change the emblem until APPLY.
                    state.site_is_custom = self.source == GameSource::Custom;
                    state
                })
            } else {
                None
            },
            has_led_panel: self.has_led_panel,
            committed_site_url: &self.config.custom_site.url,
        };

        let mut main_view = column![match self.app_state {
            AppState::MainPage => {
                let new_config = if self.snapshot.current_period == GamePeriod::BetweenGames {
                    self.tm
                        .lock()
                        .next_game_info()
                        .as_ref()
                        .and_then(|info| Some(info.timing.as_ref()?.clone().into()))
                } else {
                    None
                };

                let game_config = if let Some(ref c) = new_config {
                    c
                } else {
                    &self.config.game
                };
                let behind_schedule = if self.config.show_behind_schedule_time {
                    self.tm.lock().behind_schedule_shown(Instant::now())
                } else {
                    std::time::Duration::ZERO
                };
                build_main_view(
                    data,
                    game_config,
                    self.uses_remote(),
                    self.schedule.as_ref(),
                    self.config.fouls_tracked(),
                    self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled,
                    self.mouse_alarm_held || self.spacebar_held,
                    behind_schedule,
                    self.tm
                        .lock()
                        .last_game_info()
                        .map(|i| (i.game_number.clone(), i.scores)),
                )
            }
            AppState::TimeEdit(_, time, timeout_time) => build_time_edit_view(
                data,
                time,
                timeout_time,
                self.time_edit_old.0,
                self.time_edit_old.1,
            ),
            AppState::ScoreEdit {
                scores,
                is_confirmation,
            } => build_score_edit_view(
                data,
                scores,
                is_confirmation,
                self.snapshot.conf_pause_time,
                self.snapshot.scores,
            ),
            AppState::PenaltyOverview(indices) => build_penalty_overview_page(
                data,
                self.pen_edit.get_printable_lists(Instant::now()).unwrap(),
                indices
            ),
            AppState::WarningOverview(indices) => build_warning_overview_page(
                data,
                self.warn_edit.get_printable_lists(Instant::now()).unwrap(),
                indices
            ),
            AppState::FoulOverview(indices) => build_foul_overview_page(
                data,
                self.foul_edit.get_printable_lists(Instant::now()).unwrap(),
                indices
            ),
            AppState::KeypadPage(page, player_num) => {
                // Between games the roster is read live so a roster arriving
                // mid-break appears; during play the kickoff copy is used as-is,
                // which is also why this is a Cow rather than an owned clone --
                // the mid-game path allocates nothing per frame.
                let rosters = match picker_roster_game(&self.snapshot) {
                    Some(game_num) => Cow::Owned(self.rosters_for_game(game_num)),
                    None => Cow::Borrowed(&self.game_rosters),
                };
                build_keypad_page(
                    data,
                    page,
                    player_num,
                    self.config.fouls_tracked(),
                    self.edited_settings.as_ref().map(|e| e.game_number.clone()),
                    &rosters,
                    self.config.keypad_numbers_forced(),
                )
            }
            AppState::GameDetailsPage(is_refreshing) => build_game_info_page(
                data,
                &self.config.game,
                self.uses_remote(),
                is_refreshing,
                self.schedule.as_ref(),
                self.tm
                    .lock()
                    .last_game_info()
                    .map(|i| (i.game_number.clone(), i.scores)),
            ),
            AppState::WarningsSummaryPage => build_warnings_summary_page(data),
            AppState::PowerPage => build_power_page(data),
            AppState::EditGameConfig(page) => build_game_config_edit_page(
                data,
                self.edited_settings.as_ref().unwrap(),
                &self.events,
                page,
                self.page_entry_snapshot.as_ref(),
                self.power_controls_visible(),
                site_serves(
                    self.current_site.kind,
                    self.edited_settings.as_ref().unwrap().source,
                ),
            ),
            AppState::ParameterEditor(param, dur, single_half) => build_game_parameter_editor(
                data,
                param,
                dur,
                single_half,
                self.edited_settings
                    .as_ref()
                    .map_or(&self.config.game, |s| &s.config),
            ),
            AppState::ParameterEditorHelp(param, dur, single_half) => {
                build_parameter_help_page(data, param, dur, single_half)
            }
            AppState::ParameterList(param, index) => build_list_selector_page(
                data,
                param,
                index,
                self.edited_settings.as_ref().unwrap(),
                &self.events,
            ),
            AppState::ConfirmationPage(ref kind) => {
                build_confirmation_page(data, kind)
            }
            AppState::ConfirmScores(scores) =>
                build_score_confirmation_page(data, scores, self.snapshot.conf_pause_time),
            AppState::PortalDetailPage { scroll_index } =>
                build_portal_detail_page(
                    data,
                    self.portal_manager.detail_rows(),
                    scroll_index,
                    !self.portal_manager.has_startup_problem(),
                ),
            AppState::PortalAttentionAction {
                ref item_id,
                discard_armed,
            } => {
                if let Some(item) = self.portal_manager.find(item_id) {
                    build_portal_attention_action(
                        data,
                        item_id.clone(),
                        item.id.game_number.clone(),
                        item.black_score,
                        item.white_score,
                        discard_armed,
                    )
                } else {
                    // Item was resolved or discarded while the operator
                    // was on this page. Fall back to the detail page so
                    // the operator sees the actual queue state.
                    build_portal_detail_page(
                        data,
                        self.portal_manager.detail_rows(),
                        0,
                        !self.portal_manager.has_startup_problem(),
                    )
                }
            }
            AppState::BeepTestPage => {
                // Invariant (Task 6 of beep-test absorption): `beep_test_tm`
                // is `Some` exactly when `config.mode == Mode::BeepTest`, and
                // `AppState::BeepTestPage` is only ever reached in that mode.
                // A panic here would indicate that invariant was violated —
                // a programming error, not a runtime condition.
                let bt_tm = self
                    .beep_test_tm
                    .as_ref()
                    .expect("beep_test_tm must be Some when AppState is BeepTestPage");
                build_beep_test_page(
                    &self.beep_test_snapshot,
                    &self.config.beep_test,
                    bt_tm.clock_is_running(),
                    self.beep_test_has_run,
                )
            }
            AppState::Updates {
                ref state,
                backup_available,
            } => make_updates_page(
                data,
                state,
                backup_available,
                self.pending_update.as_ref().map(|r| r.version),
                self.update_backup_version,
            ),
            AppState::BeepTestSettings(page) => match page {
                BeepTestConfigPage::Main => {
                    // App Mode is cycled directly on the landing. The
                    // staged mode lives in `edited_settings.mode` (seeded
                    // by `BeepTestOpenSettings`); a missing `edited_settings`
                    // here falls back to the current mode, which renders
                    // identically.
                    let staged_mode = self
                        .edited_settings
                        .as_ref()
                        .map(|e| e.mode)
                        .unwrap_or(self.config.mode);
                    build_beep_test_settings_landing(
                        &self.config,
                        staged_mode,
                        self.beep_test_has_run,
                        self.beep_test_display_layout,
                        self.has_led_panel,
                    )
                }
                BeepTestConfigPage::Sound => {
                    // Invariant (Task 5 of beep-test redesign):
                    // `BeepTestEditOpenSound` seeds `edited_settings` before
                    // navigating to the Sound sub-page, and every exit path
                    // (Save / Cancel) clears it on its way back to the
                    // landing. Reaching this arm with `edited_settings ==
                    // None` would indicate that invariant was violated —
                    // a programming error, not a runtime condition.
                    let edited = self.edited_settings.as_ref().expect(
                        "edited_settings must be Some when AppState is BeepTestSettings(Sound)",
                    );
                    build_beep_test_sound_settings_page(
                        &self.config,
                        &edited.sound,
                    )
                }
                BeepTestConfigPage::EditLevels => {
                    // Invariant (Task 6 of beep-test redesign):
                    // `BeepTestEditOpenLevels` seeds `edited_settings` with a
                    // staged `beep_test_levels` before navigating to this
                    // sub-page, and every exit path (Save / Cancel) clears
                    // `edited_settings` on its way back to the landing.
                    // Reaching this arm with `edited_settings == None` or
                    // `beep_test_levels == None` would indicate that invariant
                    // was violated — a programming error, not a runtime
                    // condition.
                    let edited = self.edited_settings.as_ref().expect(
                        "edited_settings must be Some when AppState is BeepTestSettings(EditLevels)",
                    );
                    let levels = edited.beep_test_levels.as_ref().expect(
                        "beep_test_levels must be Some when AppState is BeepTestSettings(EditLevels)",
                    );
                    build_beep_test_edit_levels_page(
                        &self.config,
                        levels,
                        edited.selected_level,
                    )
                }
                BeepTestConfigPage::Language => {
                    // Invariant: `BeepTestEditOpenLanguage` seeds
                    // `edited_settings` before navigating to the Language
                    // sub-page, and every exit path (Apply / Cancel) clears
                    // `edited_settings` on its way back to the landing.
                    // Reaching this arm with `edited_settings == None` would
                    // indicate that invariant was violated — a programming
                    // error, not a runtime condition.
                    let edited = self.edited_settings.as_ref().expect(
                        "edited_settings must be Some when AppState is BeepTestSettings(Language)",
                    );
                    build_beep_test_language_picker(
                        edited,
                    )
                }
                BeepTestConfigPage::Buzzer => {
                    // Invariant: `BeepTestEditOpenSound` seeds
                    // `edited_settings` before navigating to the Sound
                    // sub-page, and `BeepTestEditOpenBuzzer` navigates
                    // from Sound to here without re-seeding. Every exit path
                    // (BeepTestBuzzerSave / BeepTestBuzzerCancel) returns to
                    // the Sound sub-page. Reaching this arm with
                    // `edited_settings == None` indicates a programming error.
                    let edited = self.edited_settings.as_ref().expect(
                        "edited_settings must be Some when AppState is BeepTestSettings(Buzzer)",
                    );
                    build_beep_test_buzzer_picker(&self.config, &edited.sound)
                }
            },
        }]
        .spacing(SPACING)
        .padding(PADDING);

        match self.app_state {
            AppState::ScoreEdit {
                is_confirmation, ..
            } if is_confirmation => {}
            AppState::ConfirmScores(_) => {}
            // The power page has its own Back button and no game controls.
            AppState::PowerPage => {}
            // BeepTest mode has its own bottom action row; the timeout ribbon
            // is a hockey/rugby concept and does not belong here.
            AppState::BeepTestPage => {}
            // BeepTest Settings pages also live inside the BeepTest hierarchy
            // and have no concept of timeouts. (Each sub-page renders its own
            // bottom filler that mimics the ribbon's vertical footprint, to
            // keep Fill-share proportions consistent with Hockey/Rugby.)
            AppState::BeepTestSettings(_) => {}
            _ => {
                main_view = main_view.push(build_timeout_ribbon(
                    &self.snapshot,
                    &self.tm,
                    self.config.mode,
                    self.timeout_revive.as_ref().map(|h| (h.color, h.phase)),
                ));
            }
        }

        // Paint the window background as a real element rather than relying on
        // iced's background clear, whose colour its damage tracking remembers
        // only once for all buffers (see `window_background_container`). Fill
        // in both directions with no padding, border or text colour, so it
        // cannot move anything.
        iced::widget::container(main_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(window_background_container)
            .into()
    }

    pub(super) fn subscription(&self) -> Subscription<Message> {
        // Portal event pump: forwards `PortalEvent`s from the
        // background portal task into the iced message loop. Registered
        // with a stable ID so iced 0.13 deduplicates it and we never
        // end up with two consumers racing on the same `Receiver`.
        let portal_rx_handle = self.portal_event_rx.clone();
        let portal_events =
            Subscription::run_with_id("portal-events", portal_event_stream(portal_rx_handle));

        // Pure UI-layer tick (1 Hz) so the 30-minute stuck-item
        // escalation reaches the screen without waiting for an
        // unrelated re-render. This is deliberately NOT derived from
        // the game clock, the penalty clocks, or the background task's
        // poll interval.
        let portal_tick =
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::PortalUiTick);

        let mut subs = vec![portal_events, portal_tick];

        // Game-clock stream is only relevant in game modes. In BeepTest
        // mode it would race with the BeepTest tick and overwrite our
        // cadence snapshot on the LED panel.
        if self.config.mode != Mode::BeepTest {
            subs.push(Subscription::run(time_updater));
        } else {
            // BeepTest tick (10 Hz) — drives the cadence engine forward
            // and ships snapshots to the LED panel.
            subs.push(
                iced::time::every(std::time::Duration::from_millis(100))
                    .map(|_| Message::BeepTestTick),
            );
        }

        if self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled {
            let key_press = keyboard::on_key_press(|key, _modifiers| {
                if matches!(key, Key::Named(Named::Space)) {
                    Some(Message::SpacebarPressed)
                } else {
                    None
                }
            });
            let key_release = keyboard::on_key_release(|key, _modifiers| {
                if matches!(key, Key::Named(Named::Space)) {
                    Some(Message::SpacebarReleased)
                } else {
                    None
                }
            });
            // mouse_area.on_release only fires when the cursor is still over the widget.
            // This global subscription catches the release anywhere in the window, so
            // alarm_held never gets stuck true if the user moves the mouse away first.
            let mouse_release = event::listen_with(|ev, _status, _window| match ev {
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::AlarmReleased)
                }
                _ => None,
            });
            subs.push(key_press);
            subs.push(key_release);
            subs.push(mouse_release);
        }

        Subscription::batch(subs)
    }

    pub fn application_style(&self, _theme: &Theme) -> Appearance {
        let text_color = if display_mode() == DisplayMode::HighContrast {
            white()
        } else {
            black()
        };
        Appearance {
            background_color: window_background(),
            text_color,
        }
    }
}

/// Cap numbers from a roster that are usable on the player grid.
///
/// Entries with no cap number are skipped — there is nothing to tap. Entries
/// outside `1..=99` are skipped too: every player-attribution page gates
/// `SelectPlayerNumber` on `page.max_val()`, which is 99, so a cap of 100+
/// would render a tappable cell that silently does nothing, and a cap of 0
/// would render a cell that can never highlight (0 already means "no player
/// selected" everywhere in the app — a team goal, on the goal page).
fn usable_cap_numbers(players: &[RosterPlayer]) -> Vec<u8> {
    players
        .iter()
        .filter_map(|p| p.number)
        .filter(|n| (1..=99).contains(n))
        .collect()
}

#[cfg(test)]
mod usable_cap_numbers_tests {
    use super::*;

    fn player(number: Option<u8>) -> RosterPlayer {
        RosterPlayer {
            number,
            name: String::new(),
            is_captain: false,
            is_vice_captain: false,
        }
    }

    #[test]
    fn unnumbered_players_are_skipped() {
        assert_eq!(usable_cap_numbers(&[player(None)]), Vec::<u8>::new());
    }

    #[test]
    fn in_range_numbers_pass_through() {
        assert_eq!(
            usable_cap_numbers(&[player(Some(1)), player(Some(7)), player(Some(99))]),
            vec![1, 7, 99]
        );
    }

    #[test]
    fn zero_is_filtered_out() {
        // 0 means "no player selected" (a team goal on the goal page); a
        // roster entry of 0 must not become a tappable, never-highlighting cell.
        assert_eq!(
            usable_cap_numbers(&[player(Some(0)), player(Some(5))]),
            vec![5]
        );
    }

    #[test]
    fn caps_above_ninety_nine_are_filtered_out() {
        // Every player page gates SelectPlayerNumber on max_val() == 99; a
        // cap of 100+ would be a dead cell.
        assert_eq!(
            usable_cap_numbers(&[player(Some(100)), player(Some(255)), player(Some(50))]),
            vec![50]
        );
    }
}

/// The portal-link selections an APPLY carries from the edit buffer to the live
/// app: the game source, the linked event, the chosen court, and the schedule
/// fetched for that event.
///
/// Grouped into one value, committed by one routine, because committing a
/// *subset* of the set is a silent failure rather than a visible one. The Game
/// and App pages commit these directly, but a change made while a game is in
/// progress defers to a confirmation arm — and those arms committed the game
/// number while dropping the court. Switching court mid-game therefore left the
/// refbox still believing it was on the old court, and it went on to offer that
/// court's games once the operator's own court had run out.
#[derive(Debug, Clone, PartialEq)]
struct LinkSelection {
    source: GameSource,
    event_id: Option<EventId>,
    court: Option<String>,
    schedule: Option<Schedule>,
}

impl LinkSelection {
    /// What the operator chose on the Game / App pages.
    fn from_edited(edited: &EditableSettings) -> Self {
        Self {
            source: edited.source,
            event_id: edited.current_event_id.clone(),
            court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
        }
    }

    /// A fresh manual slate: no event, no court, no schedule.
    fn manual() -> Self {
        Self {
            source: GameSource::Manual,
            event_id: None,
            court: None,
            schedule: None,
        }
    }
}

#[cfg(test)]
mod link_selection_tests {
    use super::*;

    fn event(id: &str) -> EventId {
        EventId::from_full(id).unwrap()
    }

    fn schedule_for(id: &str) -> Schedule {
        Schedule {
            event_id: event(id),
            games: Default::default(),
            non_game_entries: Vec::new(),
            groups: Vec::new(),
            timing_rules: Vec::new(),
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }

    /// Every selection driven away from the `EditableSettings` default, and the
    /// event and the schedule deliberately given *different* ids, so a field
    /// taken from the wrong place cannot pass by coincidence.
    fn edited() -> EditableSettings {
        EditableSettings {
            source: GameSource::Custom,
            current_event_id: Some(event("events/11-A")),
            current_court: Some("Court 7".to_string()),
            schedule: Some(schedule_for("events/22-B")),
            ..Default::default()
        }
    }

    #[test]
    fn from_edited_carries_all_four_selections() {
        let link = LinkSelection::from_edited(&edited());

        assert_eq!(link.source, GameSource::Custom);
        assert_eq!(link.event_id, Some(event("events/11-A")));
        assert_eq!(link.court.as_deref(), Some("Court 7"));
        // The differing id is the point: this proves the schedule is copied from
        // `edited.schedule` rather than rebuilt from `current_event_id`.
        assert_eq!(link.schedule, Some(schedule_for("events/22-B")));
    }

    #[test]
    fn manual_clears_every_selection() {
        let link = LinkSelection::manual();

        assert_eq!(link.source, GameSource::Manual);
        assert_eq!(link.event_id, None);
        assert_eq!(link.court, None);
        assert_eq!(link.schedule, None);
    }
}

/// Copy the seven plain `Config` toggles owned by the App Options page out of the
/// staged edits. Returns `true` when `hide_time` changed, which the live Apply
/// path has to report to the update server.
///
/// A free function shared by both commit paths, rather than a method or two
/// copies of the field list, because the two paths drifting apart *is* the
/// defect this exists to prevent: the App page's Apply commits directly, while a
/// Hockey ↔ Rugby change defers its commit to the `RestartAndApply` confirmation
/// arm, which for a long time committed only the mode and silently dropped the
/// rest.
///
/// `mode` is deliberately NOT handled here: its side effects differ between the
/// two paths (the portal-queue flush) and it needs `&mut self`. The source,
/// event, court and schedule are handled by the `&mut self` sibling
/// [`RefBoxApp::commit_link_selection`], for exactly the same anti-drift reason.
fn commit_app_toggles(config: &mut Config, edited: &EditableSettings) -> bool {
    config.collect_scorer_cap_num = edited.collect_scorer_cap_num;
    config.track_fouls_and_warnings = edited.track_fouls_and_warnings;
    config.force_keypad_numbers = edited.force_keypad_numbers;
    config.show_behind_schedule_time = edited.show_behind_schedule_time;
    config.confirm_score = edited.confirm_score;
    config.audible_countdown = edited.audible_countdown;
    let hide_time_changed = config.hide_time != edited.hide_time;
    config.hide_time = edited.hide_time;
    hide_time_changed
}

#[cfg(test)]
mod commit_app_toggles_tests {
    use super::*;

    /// All seven toggles set to the opposite of their `Config` default, so a
    /// missing assignment cannot pass by coincidence.
    fn all_flipped() -> EditableSettings {
        EditableSettings {
            collect_scorer_cap_num: false,
            track_fouls_and_warnings: true,
            show_behind_schedule_time: false,
            confirm_score: false,
            audible_countdown: true,
            hide_time: true,
            force_keypad_numbers: true,
            ..Default::default()
        }
    }

    #[test]
    fn all_seven_toggles_are_copied() {
        let mut config = Config::default();

        commit_app_toggles(&mut config, &all_flipped());

        assert!(!config.collect_scorer_cap_num);
        assert!(config.track_fouls_and_warnings);
        assert!(!config.show_behind_schedule_time);
        assert!(!config.confirm_score);
        assert!(config.audible_countdown);
        assert!(config.hide_time);
        assert!(config.force_keypad_numbers);
    }

    #[test]
    fn all_seven_toggles_are_copied_the_other_way() {
        // Guards against a hardcoded assignment rather than a copy: the same
        // seven fields driven back in the opposite direction. All seven start
        // `true` so that committing an all-default (all-false) EditableSettings
        // has to change every one of them — starting from `all_flipped()` would
        // leave three already false, and their assertions would then pass even
        // with the assignment deleted.
        let mut config = Config::default();
        config.collect_scorer_cap_num = true;
        config.track_fouls_and_warnings = true;
        config.show_behind_schedule_time = true;
        config.confirm_score = true;
        config.audible_countdown = true;
        config.hide_time = true;
        config.force_keypad_numbers = true;
        let edited = EditableSettings::default();

        commit_app_toggles(&mut config, &edited);

        assert_eq!(config.collect_scorer_cap_num, edited.collect_scorer_cap_num);
        assert_eq!(
            config.track_fouls_and_warnings,
            edited.track_fouls_and_warnings
        );
        assert_eq!(
            config.show_behind_schedule_time,
            edited.show_behind_schedule_time
        );
        assert_eq!(config.confirm_score, edited.confirm_score);
        assert_eq!(config.audible_countdown, edited.audible_countdown);
        assert_eq!(config.hide_time, edited.hide_time);
        assert_eq!(config.force_keypad_numbers, edited.force_keypad_numbers);
    }

    #[test]
    fn hide_time_change_is_reported() {
        let mut config = Config::default();
        assert!(!config.hide_time, "test assumes the default is off");
        let edited = EditableSettings {
            hide_time: true,
            ..Default::default()
        };

        assert!(commit_app_toggles(&mut config, &edited));
    }

    #[test]
    fn unchanged_hide_time_is_not_reported() {
        // The live Apply path messages the update server only on a real change.
        let mut config = Config::default();
        let edited = EditableSettings {
            hide_time: config.hide_time,
            ..Default::default()
        };
        assert!(!commit_app_toggles(&mut config, &edited));

        config.hide_time = true;
        let edited = EditableSettings {
            hide_time: true,
            ..Default::default()
        };
        assert!(!commit_app_toggles(&mut config, &edited));
    }

    #[test]
    fn only_the_seven_toggles_are_written() {
        // `mode` and `source` must NOT be committed by this helper. Both have
        // side effects that differ between the two commit paths — the mode
        // confirmation and the portal-queue flush — so they stay with the
        // callers. Adding them here would bypass both.
        //
        // Compared whole-struct rather than field by field, so this also fails
        // if a field is dropped from the helper, or if any other Config field
        // (game, sound, custom_site, display_mode, ...) is written. Drift
        // between the two commit paths is the defect the helper exists to
        // prevent, so the guard is deliberately exhaustive rather than a list
        // of the fields someone thought to name.
        let mut config = Config::default();
        let edited = EditableSettings {
            mode: Mode::Rugby,
            source: GameSource::Portal,
            ..all_flipped()
        };
        assert_ne!(
            config.mode, edited.mode,
            "test needs an edited mode that differs from the default"
        );
        assert_ne!(
            config.source, edited.source,
            "test needs an edited source that differs from the default"
        );

        let mut expected = config.clone();
        expected.collect_scorer_cap_num = edited.collect_scorer_cap_num;
        expected.track_fouls_and_warnings = edited.track_fouls_and_warnings;
        expected.show_behind_schedule_time = edited.show_behind_schedule_time;
        expected.confirm_score = edited.confirm_score;
        expected.audible_countdown = edited.audible_countdown;
        expected.hide_time = edited.hide_time;
        expected.force_keypad_numbers = edited.force_keypad_numbers;

        commit_app_toggles(&mut config, &edited);

        assert_eq!(config, expected);
    }
}

/// Decide whether a remembered link note should be restored at startup:
/// it must be fresh (within the freshness window) and belong to the same
/// portal as the current mode (a UWH note is never restored into UWR).
fn decide_restore(
    note: &crate::portal_manager::link_session::LinkSessionFile,
    now: time::OffsetDateTime,
    current_mode: Mode,
) -> bool {
    use crate::portal_manager::link_session::{FRESHNESS_WINDOW, is_fresh};
    // A link for a different portal (UWR vs UWH) is never restored, regardless
    // of timing — that is a correctness guard, not a freshness question.
    if crosses_portal(note.mode, current_mode) {
        return false;
    }
    // A Raspberry Pi has no battery-backed clock, so it can boot with a time
    // that has not yet been corrected by the network. If `now` reads *earlier*
    // than when the link was last saved, the clock is plainly not trustworthy
    // yet — and the link is therefore genuinely recent. Restore it rather than
    // discard a fresh link on the strength of a bad clock. The token is
    // re-verified against the portal after restore, so this is safe.
    if now < note.last_active {
        return true;
    }
    // Trustworthy clock: restore only while the link is within the window.
    is_fresh(note.last_active, now, FRESHNESS_WINDOW)
}

#[cfg(test)]
mod site_target_tests {
    use super::*;

    fn custom(url: &str) -> CustomSite {
        CustomSite {
            url: url.to_string(),
            token: String::new(),
        }
    }

    /// The typed scheme decides whether TLS is demanded — this is what removes
    /// the `--allow-http` launch flag for a third-party site. The launch flag
    /// passed here is `true` (https demanded) precisely to prove it is ignored
    /// for a custom address.
    #[test]
    fn custom_http_site_does_not_demand_tls() {
        let target = site_target(
            GameSource::Custom,
            Mode::Hockey6V6,
            &custom("http://scoreboard.local:8099/api/events/1234-A"),
            true,
        )
        .unwrap();
        assert_eq!(target.kind, SiteKind::Custom);
        assert_eq!(target.base_url.expose(), "http://scoreboard.local:8099");
        assert!(!target.require_https);
    }

    #[test]
    fn custom_https_site_demands_tls() {
        let target = site_target(
            GameSource::Custom,
            Mode::Hockey6V6,
            &custom("https://scoreboard.example/api/events/1234-A"),
            false,
        )
        .unwrap();
        assert_eq!(target.base_url.expose(), "https://scoreboard.example");
        assert!(target.require_https);
    }

    /// An address that cannot be used asks for no change, leaving the client
    /// where it is rather than pointing it at nothing.
    #[test]
    fn unusable_custom_address_asks_for_no_change() {
        for url in [
            "",
            "   ",
            "scoreboard.local/api/events/1234-A",
            "http://x/api",
        ] {
            assert!(
                site_target(GameSource::Custom, Mode::Hockey6V6, &custom(url), false).is_none(),
                "{url:?} should not produce a target"
            );
        }
    }

    /// Changing only the event inside the address still counts as a different
    /// target, so editing it meets the same guards as changing the host. The
    /// client itself is unaffected — both share a base URL.
    #[test]
    fn a_different_event_is_a_different_target() {
        let a = site_target(
            GameSource::Custom,
            Mode::Hockey6V6,
            &custom("http://scoreboard.local:8099/api/events/1234-A"),
            false,
        )
        .unwrap();
        let b = site_target(
            GameSource::Custom,
            Mode::Hockey6V6,
            &custom("http://scoreboard.local:8099/api/events/1234-B"),
            false,
        )
        .unwrap();
        assert_eq!(a.base_url, b.base_url);
        assert_ne!(a, b);
    }

    // ---- Credentials must never reach a log ----
    //
    // A custom site address is stored exactly as the operator typed it, so
    // `https://user:password@host/...` used to reach the log file verbatim --
    // and on a Pi those logs rotate to disk and get shared when something is
    // being diagnosed. The redaction itself lives in `custom_site`, and is
    // tested there; what these pin are the log lines built here.

    const LEAKY: &str = "https://scorekeeper:hunter2@scoreboard.local:8099/api/1234-A";

    /// SiteTarget derives Debug, so dumping a whole target must be safe too.
    #[test]
    fn a_whole_site_target_never_prints_credentials() {
        assert!(!format!("{:?}", custom_target(LEAKY)).contains("hunter2"));
        assert!(!format!("{:?}", target(LEAKY, true)).contains("hunter2"));
    }

    /// The exact line Eric saw leak during the PR #2744 walkthrough. It calls the real builder
    /// rather than re-typing the message, so reaching for `expose()` at the log site turns this
    /// red -- which is the only reason it is worth having on top of the redaction's own tests.
    ///
    /// Both kinds are checked: a custom site is the one an operator types a password into, and it
    /// is the arm that would otherwise go unexercised.

    #[test]
    fn a_stale_login_answer_files_no_key_anywhere() {
        // Before the site stamp there was no `issued_at` to compare and the key was written
        // unconditionally, so this assertion had nothing to hold on to.
        let mut config = Config::default();
        config.store_access_key(
            "https://api.uwhportal.com",
            &ev("current"),
            "ORIGINAL-KEY".into(),
        );
        assert!(!file_login_key(
            &mut config,
            "https://api.uwhportal.com",
            &ev("current"),
            3,
            4,
            "FROM-A-SITE-WE-LEFT".to_string()
        ));
        assert_eq!(
            config.access_key_for("https://api.uwhportal.com", &ev("current")),
            Some("ORIGINAL-KEY")
        );
    }

    #[test]
    fn a_current_login_answer_is_filed_against_its_own_site_and_event_only() {
        let mut config = Config::default();
        config.store_access_key(
            "https://api.uwhportal.com",
            &ev("first"),
            "FIRST-KEY".into(),
        );
        assert!(file_login_key(
            &mut config,
            "https://api.uwhportal.com",
            &ev("second"),
            4,
            4,
            "SECOND-KEY".to_string()
        ));
        assert_eq!(
            config.access_key_for("https://api.uwhportal.com", &ev("second")),
            Some("SECOND-KEY")
        );
        assert_eq!(
            config.access_key_for("https://api.uwhportal.com", &ev("first")),
            Some("FIRST-KEY"),
            "a login for one event must never overwrite the key filed for another"
        );
    }

    /// A fetch carries the key filed for the event it is asking about. This is the half that
    /// broke on 2026-09-04: the key was chosen from the *linked* event, so the first fetch after a
    /// login went out with no credential at all and the court list hung.
    #[test]
    fn a_request_carries_the_key_filed_for_the_event_it_asks_for() {
        let mut config = Config::default();
        // Keyed off the target's own address, not a literal: `portal_target` reads
        // UWH_PORTAL_URL_OVERRIDE at runtime, and with that exported -- the documented dev-portal
        // workflow -- a hard-coded site made this test fail and its two negative siblings pass
        // for the wrong reason.
        let target = portal_target(Mode::Hockey6V6, false);
        config.store_access_key(target.base_url.expose(), &ev("asked-about"), "KEY-B".into());
        let client = client_for_event(&target, &config, &ev("asked-about")).unwrap();
        assert!(client.has_token(), "a key is on file for this event");
    }

    /// ...and one filed for a different event is never substituted. A client that fell back to
    /// "any key for this site" would pass the test above and still send one event's credential to
    /// a request about another.
    #[test]
    fn a_request_for_an_event_with_no_key_is_not_made_at_all() {
        let mut config = Config::default();
        let target = portal_target(Mode::Hockey6V6, false);
        config.store_access_key(target.base_url.expose(), &ev("other"), "KEY-A".into());
        assert!(
            client_for_event(&target, &config, &ev("asked-about")).is_none(),
            "a key filed for another event must never be substituted -- and a client carrying no \
             credential would send the privileged fetch unauthenticated, which a permissive site \
             would answer as though the refbox were entitled to the data"
        );
    }

    /// The pre-upgrade `[uwhportal] token` is kept in the file and never used as a credential:
    /// adoption was dropped on 2026-09-01, so an upgrading operator logs in exactly once.
    ///
    /// Replaces a test that asserted `build_site_client` carries no key. That was true but could
    /// not fail -- the function no longer receives the config it was set up with, so the
    /// assertion held no matter what the config said.
    #[test]
    fn a_legacy_token_is_never_sent_as_an_access_key() {
        let mut config = Config::default();
        config.uwhportal.token = "LEGACY-KEY".into();
        let target = portal_target(Mode::Hockey6V6, false);
        assert!(
            client_for_event(&target, &config, &ev("abc")).is_none(),
            "the legacy slot is retained for rollback, never sent"
        );
    }

    #[test]
    fn a_login_log_line_never_calls_a_custom_site_the_portal() {
        assert_eq!(
            login_site_name(SiteKind::Portal, Mode::Hockey6V6),
            "the UWH Portal"
        );
        assert_eq!(
            login_site_name(SiteKind::Portal, Mode::Rugby),
            "the UWR Portal"
        );
        // BeepTest has no portal name; the line must not degrade to "the  Portal".
        assert_eq!(
            login_site_name(SiteKind::Portal, Mode::BeepTest),
            "the portal"
        );
        let custom = login_site_name(SiteKind::Custom, Mode::Hockey6V6);
        assert_eq!(custom, "the custom site");
        assert!(
            !custom.contains("Portal"),
            "a third-party login must not be reported as a Portal login"
        );
    }

    #[test]
    fn the_repoint_log_line_cannot_carry_credentials() {
        for target in [custom_target(LEAKY), target(LEAKY, true)] {
            let line = repoint_log_line(&target);
            assert!(!line.contains("hunter2"), "leaked: {line}");
            assert!(!line.contains("scorekeeper"), "leaked: {line}");
            // Still says which site, or the line is not worth logging.
            assert!(line.contains("scoreboard.local"), "unusable: {line}");
        }
    }

    /// The https-policy explanation formats an address too, and is a plain function, so a
    /// credentialed target is one line to check and would otherwise be pinned by nothing.
    #[test]
    fn the_https_policy_explanation_cannot_carry_credentials() {
        let mut target = custom_target("http://scorekeeper:hunter2@scoreboard.local:8099");
        target.require_https = true;
        let msg = https_policy_conflict(&target).expect("plain http under the https rule");
        assert!(!msg.contains("hunter2"), "leaked: {msg}");
    }

    /// The game feed reaches every device on the pool LAN, so it matters more than any log file.
    #[test]
    fn the_game_feed_never_publishes_credentials() {
        let published = published_site_address(&custom_target(LEAKY)).expect("a usable site");
        assert!(!published.contains("hunter2"), "leaked: {published}");
        assert!(!published.contains("scorekeeper"), "leaked: {published}");
        assert_eq!(published, "https://scoreboard.local:8099/api/1234-A");
    }

    /// ...and publishes nothing at all, rather than a guess, for an address it cannot vouch for.
    #[test]
    fn the_game_feed_publishes_nothing_it_cannot_vouch_for() {
        let target = custom_target(r"https://scoreboard.local\scorekeeper:hunter2@pool");
        assert_eq!(published_site_address(&target), None);
    }

    /// The two error lines that also name an address.
    #[test]
    fn the_client_failure_lines_cannot_carry_credentials() {
        let target = custom_target(LEAKY);
        let started = client_start_failure_log_line(&target, &"connection refused");
        assert!(!started.contains("hunter2"), "leaked: {started}");
        let none = no_client_log_line(&target);
        assert!(!none.contains("hunter2"), "leaked: {none}");
    }

    /// The same trace line carries the Portal access key. This is the live path: a successful
    /// link arrives as `RecvPortalToken(Success(<key>))`, and `trace!("Handling message:
    /// {message:?}")` runs on it like any other.
    ///
    /// Checked through the message rather than through `PortalTokenResponse` alone, because it is
    /// the message that reaches the log -- the type having a safe `Debug` is only useful if
    /// nothing between it and the log undoes that.
    #[test]
    fn a_traced_message_never_carries_a_portal_token() {
        const SECRET: &str = "s3cret-access-key";
        let message = Message::RecvPortalToken(
            PortalTokenResponse::Success(SECRET.to_string()),
            ev("abc"),
            0,
        );
        assert!(
            !format!("{message:?}").contains(SECRET),
            "leaked: {message:?}"
        );
    }

    /// The app state is traced too (`trace!("AppState changed to {:?}", ...)`), and one of its
    /// variants nests a `PortalTokenResponse` inside a `ConfirmationKind`. Nothing can put a token
    /// there today -- the view has an `unreachable!()` for exactly that case, because a *failed*
    /// link is the only thing that builds it -- so this is not a leak that exists. It is here so
    /// that if the type ever does travel that way, the redaction still holds through two layers of
    /// derived `Debug`, which is the whole reason the guard lives on the type and not at the log.
    #[test]
    fn a_nested_token_is_still_redacted_through_derived_debug() {
        const SECRET: &str = "s3cret-access-key";
        let state = AppState::ConfirmationPage(ConfirmationKind::UwhPortalLinkFailed(
            PortalTokenResponse::Success(SECRET.to_string()),
        ));
        assert!(!format!("{state:?}").contains(SECRET), "leaked: {state:?}");
    }

    /// Every message is Debug-logged at trace level, which runs on every keystroke, so what the
    /// operator is part-way through typing must not be in it.
    #[test]
    fn a_half_typed_address_is_not_traced() {
        let msg = Message::CustomSiteUrlChanged(LEAKY.to_string().into());
        assert!(!format!("{msg:?}").contains("hunter2"), "leaked: {msg:?}");
    }

    /// The startup line names why the address is unusable and never the address itself: the
    /// address is precisely the string that would not parse, and an unparsed string cannot be
    /// shown to be free of a password.
    #[test]
    fn the_startup_unusable_address_line_cannot_carry_credentials() {
        for leaky in [
            "scorekeeper:hunter2@scoreboard.local:8099/api/1234-A",
            "https://scorekeeper:hunter2@scoreboard.local:8099",
            r"https://scoreboard.local\scorekeeper:hunter2@pool",
            "foo:/scorekeeper:hunter2@host",
        ] {
            let reason = custom_site::parse_custom_site(leaky).expect_err("unusable");
            let line = unusable_saved_address_log_line(reason, "starting with manual games");
            assert!(!line.contains("hunter2"), "leaked: {line}");
            assert!(!line.contains("scorekeeper"), "leaked: {line}");
        }
    }

    /// ...and still says something worth reading. An empty address is much the commonest reason
    /// this line fires, and naming that beats echoing a blank string back.
    #[test]
    fn the_startup_unusable_address_line_says_what_is_wrong() {
        let reason = custom_site::parse_custom_site("").expect_err("empty is unusable");
        assert!(
            unusable_saved_address_log_line(reason, "starting with manual games").contains("Empty"),
            "should name the reason"
        );
        let reason =
            custom_site::parse_custom_site("https://scoreboard.local:8099").expect_err("no /api/");
        assert!(
            unusable_saved_address_log_line(reason, "starting with manual games")
                .contains("MissingApiSegment"),
            "should name the reason"
        );
    }

    /// Manual never repoints: results already queued must keep going to the
    /// site they were queued for, not follow the operator to the portal.
    #[test]
    fn manual_asks_for_no_change() {
        assert!(
            site_target(
                GameSource::Manual,
                Mode::Hockey6V6,
                &custom("http://scoreboard.local:8099/api/events/1234-A"),
                false,
            )
            .is_none()
        );
    }

    /// A custom target -- the kind an operator actually types a password into.
    fn custom_target(address: &str) -> SiteTarget {
        SiteTarget {
            kind: SiteKind::Custom,
            base_url: address.to_string().into(),
            require_https: address.starts_with("https://"),
            address: address.to_string().into(),
        }
    }

    fn target(base_url: &str, require_https: bool) -> SiteTarget {
        SiteTarget {
            kind: SiteKind::Portal,
            base_url: base_url.to_string().into(),
            require_https,
            address: base_url.to_string().into(),
        }
    }

    /// `https_only` refuses a plain-http address inside `reqwest`, which reports
    /// it as "builder error for url" — wording that names neither the cause nor
    /// the remedy, and that repeats on every call. This is the one line that
    /// says both, so both must be in it: the address that will be refused, and
    /// the flag that would allow it.
    #[test]
    fn a_plain_http_address_under_the_https_rule_is_explained() {
        let msg = https_policy_conflict(&target("http://127.0.0.1:9099", true))
            .expect("a plain-http address under the https rule must be explained");
        assert!(msg.contains("http://127.0.0.1:9099"), "{msg}");
        assert!(msg.contains("--allow-http"), "{msg}");
    }

    /// Nothing to explain when nothing will be refused. An https address always
    /// works, and a plain-http address is fine once the rule is off — which is
    /// how every plain-http custom site already runs, with no flag.
    #[test]
    fn an_address_that_will_work_is_not_explained() {
        assert!(https_policy_conflict(&target("https://api.uwhportal.com", true)).is_none());
        assert!(https_policy_conflict(&target("http://scoreboard.local:8099", false)).is_none());
    }

    /// A URL scheme is case-insensitive and `reqwest` lowercases it before
    /// deciding, so `HTTPS://…` is sent normally. Announcing that every request
    /// "will all fail" over an address that in fact works would be a worse
    /// error than the opaque one this replaced.
    #[test]
    fn an_uppercase_scheme_that_will_work_is_not_explained() {
        assert!(https_policy_conflict(&target("HTTPS://api.uwhportal.com", true)).is_none());
    }

    /// The portal keeps taking its TLS requirement from the launch flag, and
    /// never from a custom address. (The address itself is not asserted: it can
    /// legitimately come from the developer override environment variable.)
    #[test]
    fn portal_takes_tls_from_the_launch_flag() {
        let site = custom("http://scoreboard.local:8099/api/events/1234-A");
        for require_https in [true, false] {
            let target =
                site_target(GameSource::Portal, Mode::Hockey6V6, &site, require_https).unwrap();
            assert_eq!(target.kind, SiteKind::Portal);
            assert_eq!(target.require_https, require_https);
            assert!(!target.base_url.expose().contains("scoreboard.local"));
        }
    }

    /// Whether the live client can answer for a source. A staged source the
    /// client has not moved to yet cannot be fetched from, and asking anyway is
    /// how a portal event id ended up being sent to somebody's own server.
    #[test]
    fn site_serves_only_its_own_source() {
        assert!(site_serves(SiteKind::Portal, GameSource::Portal));
        assert!(!site_serves(SiteKind::Portal, GameSource::Custom));
        assert!(site_serves(SiteKind::Custom, GameSource::Custom));
        assert!(!site_serves(SiteKind::Custom, GameSource::Portal));
        // Manual fetches nothing, so no site serves it.
        assert!(!site_serves(SiteKind::Portal, GameSource::Manual));
        assert!(!site_serves(SiteKind::Custom, GameSource::Manual));
    }

    fn ev(id: &str) -> EventId {
        EventId::from_full(format!("events/{id}")).unwrap()
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;
    use crate::portal_manager::link_session::LinkSessionFile;

    fn note(last_active: time::OffsetDateTime, mode: Mode) -> LinkSessionFile {
        LinkSessionFile {
            version: LinkSessionFile::CURRENT_VERSION,
            event_id: EventId::from_full("events/2113-A").unwrap(),
            court: Some("1".into()),
            current_game: Some("G1".into()),
            last_played: None,
            last_played_start: None,
            mode,
            last_active,
        }
    }

    #[test]
    fn fresh_same_portal_restores() {
        let now = time::OffsetDateTime::now_utc();
        let n = note(now - time::Duration::hours(20), Mode::Hockey6V6);
        assert!(decide_restore(&n, now, Mode::Hockey6V6));
        // 3v3 shares the UWH portal with 6v6 → still restore
        assert!(decide_restore(&n, now, Mode::Hockey3V3));
    }

    #[test]
    fn stale_does_not_restore() {
        // Trustworthy clock, link older than the 120h window → dormant.
        let now = time::OffsetDateTime::now_utc();
        let n = note(now - time::Duration::hours(121), Mode::Hockey6V6);
        assert!(!decide_restore(&n, now, Mode::Hockey6V6));
    }

    #[test]
    fn clock_behind_save_restores() {
        // Pi booted with an uncorrected clock: `now` reads earlier than the
        // link was saved. The clock is untrustworthy and the link is genuinely
        // recent, so it must restore (this is the bad-boot-clock fix).
        let now = time::OffsetDateTime::now_utc();
        let n = note(now + time::Duration::hours(3), Mode::Hockey6V6);
        assert!(decide_restore(&n, now, Mode::Hockey6V6));
    }

    #[test]
    fn clock_behind_save_does_not_override_cross_portal() {
        // The cross-portal guard wins even when the clock looks untrustworthy.
        let now = time::OffsetDateTime::now_utc();
        let n = note(now + time::Duration::hours(3), Mode::Hockey6V6);
        assert!(!decide_restore(&n, now, Mode::Rugby));
    }

    #[test]
    fn cross_portal_does_not_restore() {
        let now = time::OffsetDateTime::now_utc();
        let n = note(now, Mode::Hockey6V6);
        // Rugby uses the UWR portal → must NOT restore a UWH note
        assert!(!decide_restore(&n, now, Mode::Rugby));
    }
}

/// What a schedule says is next on this court.
///
/// The last three are all "nothing is next" and are **displayed identically**.
/// They are kept apart so an empty court or an unreadable schedule is never
/// mistaken for a completed one — that conflation is what caused the original
/// defects.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NextGameFromSchedule {
    Game(GameNumber),
    /// The schedule was read and holds nothing after the anchor.
    CourtFinished,
    /// The schedule was read and this court has no games at all.
    NothingScheduled,
    /// The schedule was read but the refbox holds no history for this court, so
    /// it cannot know which games are already played. Requires an operator pick.
    NeedsPick,
    /// Nothing can be judged: no court selected, or no usable anchor.
    Unknown,
}

/// Decide what a freshly-received schedule means for the upcoming game.
///
/// The refbox never invents a game: every answer here comes from the schedule or
/// from the operator. In priority order:
///
/// 1. A startup restore re-selects the game the operator was on. A fact about
///    where they were, applied once, to bootstrap the display.
/// 2. The game the engine already holds wins over any search: the operator may
///    have picked a game out of order, and a refresh must not silently replace
///    that choice with the next game in schedule order.
/// 3. Otherwise search this court for the first game after the anchor — the game
///    last played to a recorded result. Nothing after it means the court is
///    finished, and that answer is the same however many times it is asked,
///    because nothing is consumed to produce it.
/// 4. With no anchor there is no safe automatic answer: a court the refbox holds
///    no record for is a fresh morning or a replacement box mid-day, and it
///    cannot tell them apart. Ask.
fn next_game_from_schedule(
    schedule: &Schedule,
    restore_num: Option<&GameNumber>,
    engine_next: Option<&GameNumber>,
    last_played: Option<&GameNumber>,
    last_played_start: Option<time::OffsetDateTime>,
    court: Option<&str>,
) -> NextGameFromSchedule {
    if let Some(num) = restore_num {
        return NextGameFromSchedule::Game(num.clone());
    }

    if let Some(num) = engine_next {
        return NextGameFromSchedule::Game(num.clone());
    }

    let Some(court) = court else {
        return NextGameFromSchedule::Unknown;
    };

    if !schedule.games.values().any(|game| game.court == court) {
        return NextGameFromSchedule::NothingScheduled;
    }

    let Some(anchor) = last_played else {
        return NextGameFromSchedule::NeedsPick;
    };

    // Prefer the schedule's own copy of the anchor's start time; fall back to the
    // one written down when it was played, so a game moved off this court since
    // does not blind the search.
    let anchor_start = schedule
        .games
        .get(anchor)
        .map(|game| game.start_time)
        .or(last_played_start);

    match anchor_start {
        Some(start) => match schedule.next_game_on_court(court, start) {
            Some(game) => NextGameFromSchedule::Game(game.number.clone()),
            None => NextGameFromSchedule::CourtFinished,
        },
        // The anchor is not in this schedule and no start time was remembered for
        // it, so nothing can be judged. Not a guess, and not "finished".
        None => NextGameFromSchedule::Unknown,
    }
}

#[cfg(test)]
mod refresh_next_game_tests {
    use super::*;
    use uwh_common::uwhportal::schedule::{Game, GameList, ScheduledTeam};

    fn game_at(number: &str, court: &str, start: time::OffsetDateTime) -> Game {
        Game {
            number: number.to_string(),
            dark: ScheduledTeam::new_pending_assignment_name("Dark"),
            light: ScheduledTeam::new_pending_assignment_name("Light"),
            start_time: start,
            court: court.to_string(),
            timing_rule: "Standard".to_string(),
            referee_assignments: None,
            description: None,
        }
    }

    /// Two courts, alternating: court 1 holds games 9 and 11, court 2 holds 10 and 12.
    fn two_court_schedule() -> Schedule {
        let mut games = GameList::new();
        for (number, court, start) in [
            (
                "9",
                "Court 1",
                time::macros::datetime!(2026-08-05 09:00 UTC),
            ),
            (
                "10",
                "Court 2",
                time::macros::datetime!(2026-08-05 09:00 UTC),
            ),
            (
                "11",
                "Court 1",
                time::macros::datetime!(2026-08-05 10:00 UTC),
            ),
            (
                "12",
                "Court 2",
                time::macros::datetime!(2026-08-05 10:00 UTC),
            ),
        ] {
            games.insert(number.to_string(), game_at(number, court, start));
        }
        Schedule {
            event_id: EventId::from_partial("1-A"),
            games,
            non_game_entries: vec![],
            groups: vec![],
            timing_rules: vec![],
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }

    #[test]
    fn a_restored_game_number_wins() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                Some(&"12".to_string()),
                None,
                None,
                None,
                Some("Court 1"),
            ),
            NextGameFromSchedule::Game("12".to_string())
        );
    }

    #[test]
    fn the_operators_out_of_order_pick_survives_a_refresh() {
        // The operator finished game 9 and picked game 12 by hand. A refresh must
        // re-resolve THAT game, not replace it with game 11.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                Some(&"12".to_string()),
                Some(&"9".to_string()),
                None,
                Some("Court 1"),
            ),
            NextGameFromSchedule::Game("12".to_string())
        );
    }

    #[test]
    fn the_anchor_finds_the_next_game_on_this_court() {
        // Game 10 starts at the same moment on the other court and must be ignored.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"9".to_string()),
                None,
                Some("Court 1")
            ),
            NextGameFromSchedule::Game("11".to_string())
        );
    }

    #[test]
    fn nothing_after_the_anchor_is_a_finished_court() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"11".to_string()),
                None,
                Some("Court 1")
            ),
            NextGameFromSchedule::CourtFinished
        );
    }

    #[test]
    fn a_consumed_restore_still_answers_finished() {
        // Scenario 4's Critical, as the two calls the call site actually makes.
        // First refresh: the restored game 11 is re-selected, and the one-shot is
        // spent doing it. Second refresh, after game 11 has been played: the
        // restore is gone and the engine holds no next game, which is exactly the
        // state the old code fell into — and it answered with the earliest game on
        // the court. The anchor must carry the answer instead.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                Some(&"11".to_string()),
                None,
                None,
                None,
                Some("Court 1")
            ),
            NextGameFromSchedule::Game("11".to_string())
        );
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"11".to_string()),
                None,
                Some("Court 1")
            ),
            NextGameFromSchedule::CourtFinished
        );
    }

    #[test]
    fn no_anchor_asks_the_operator_rather_than_offering_the_earliest_game() {
        // Supersedes decision 9. A replacement box brought out mid-day has no
        // anchor and would have been confidently offered game 9, played hours ago.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(&schedule, None, None, None, None, Some("Court 1")),
            NextGameFromSchedule::NeedsPick
        );
    }

    #[test]
    fn a_court_with_no_games_is_not_a_finished_court() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(&schedule, None, None, None, None, Some("Court 7")),
            NextGameFromSchedule::NothingScheduled
        );
    }

    #[test]
    fn a_game_added_to_a_finished_court_is_found_by_the_next_search() {
        let mut schedule = two_court_schedule();
        schedule.games.insert(
            "13".to_string(),
            game_at(
                "13",
                "Court 1",
                time::macros::datetime!(2026-08-05 11:00 UTC),
            ),
        );
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"11".to_string()),
                None,
                Some("Court 1")
            ),
            NextGameFromSchedule::Game("13".to_string())
        );
    }

    #[test]
    fn a_removed_anchor_still_searches_from_its_remembered_start_time() {
        // The anchor game was moved to another court. Its number is gone from the
        // schedule, but its start time was written down, so the search still
        // answers what is genuinely next here.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"absent".to_string()),
                Some(time::macros::datetime!(2026-08-05 09:00 UTC)),
                Some("Court 1"),
            ),
            NextGameFromSchedule::Game("11".to_string())
        );
    }

    #[test]
    fn the_schedules_copy_of_the_anchor_start_beats_the_remembered_one() {
        // The anchor is in this schedule, so ITS start time (09:00) sets the search
        // window. A remembered 10:00 must not win: it would look straight past game
        // 11 and park a court that still has a game to play.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"9".to_string()),
                Some(time::macros::datetime!(2026-08-05 10:00 UTC)),
                Some("Court 1"),
            ),
            NextGameFromSchedule::Game("11".to_string())
        );
    }

    #[test]
    fn an_unjudgeable_anchor_is_unknown_never_a_guess() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"absent".to_string()),
                None,
                Some("Court 1")
            ),
            NextGameFromSchedule::Unknown
        );
    }

    #[test]
    fn no_court_selected_is_unknown() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(&schedule, None, None, Some(&"9".to_string()), None, None),
            NextGameFromSchedule::Unknown
        );
    }
}

#[cfg(test)]
mod break_starts_nothing_tests {
    use super::break_starts_nothing;
    use uwh_common::game_snapshot::GamePeriod;

    #[test]
    fn blank_number_between_games_starts_nothing() {
        assert!(break_starts_nothing(GamePeriod::BetweenGames, ""));
    }

    #[test]
    fn a_real_upcoming_game_still_sounds() {
        assert!(!break_starts_nothing(GamePeriod::BetweenGames, "11"));
    }

    #[test]
    fn other_breaks_are_never_silenced() {
        // A game is in progress in every other break, so they always start play
        // again — whatever the next-game number says.
        for period in [
            GamePeriod::HalfTime,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::PreSuddenDeath,
        ] {
            assert!(!break_starts_nothing(period, ""), "{period:?}");
        }
    }
}

#[cfg(test)]
mod link_note_game_tests {
    use super::{GameConfig, LinkNoteGame, link_note_game};
    use crate::tournament_manager::{NextGameInfo, TournamentManager};
    use std::time::Duration;
    use uwh_common::game_snapshot::GamePeriod;

    /// Startup, before the schedule has been read: the engine knows nothing, so
    /// the note must be left exactly as it is. Writing anything here is what
    /// cost us twice — first a guessed `"1"` that made a finished court replay
    /// the day, then a blank that erased a mid-event operator's resume point.
    #[test]
    fn the_note_is_left_alone_until_the_schedule_is_known() {
        let tm = TournamentManager::new(GameConfig::default());

        // What the engine offers here is the arithmetic guess `game_number + 1`,
        // not a scheduled game — ignorance wearing the shape of an answer.
        assert_eq!(tm.next_game_number(), "1");

        assert_eq!(link_note_game(&tm), LinkNoteGame::Unknown);
    }

    /// A finished court is *knowledge*, and must be recorded as "no game" —
    /// which is what brings the refbox back into the finished state.
    #[test]
    fn a_finished_court_records_no_game() {
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_game_number("3");
        tm.set_no_next_game();

        assert_eq!(link_note_game(&tm), LinkNoteGame::Write(None));
    }

    #[test]
    fn a_scheduled_next_game_is_recorded() {
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_next_game(NextGameInfo {
            number: "11".to_string(),
            timing: None,
            start_time: None,
        });

        assert_eq!(
            link_note_game(&tm),
            LinkNoteGame::Write(Some("11".to_string()))
        );
    }

    #[test]
    fn a_game_in_progress_records_that_game() {
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_game_number("7");
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(60));

        assert_eq!(
            link_note_game(&tm),
            LinkNoteGame::Write(Some("7".to_string()))
        );
    }

    #[test]
    fn a_linked_engine_seeded_with_a_remembered_game_reports_it() {
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_schedule_linked(true);
        // What startup does with a note that holds a current game, before any
        // schedule has arrived: timing and start time are unknown and stay unknown.
        tm.set_next_game(NextGameInfo {
            number: "11".to_string(),
            timing: None,
            start_time: None,
        });
        assert_eq!(tm.next_game_number(), "11");
        assert_eq!(
            link_note_game(&tm),
            LinkNoteGame::Write(Some("11".to_string()))
        );
    }

    #[test]
    fn a_linked_engine_with_nothing_seeded_reports_no_game() {
        // The finished-court note: an anchor, no current game, nothing seeded.
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_schedule_linked(true);
        assert_eq!(tm.next_game_number(), "");
        assert_eq!(link_note_game(&tm), LinkNoteGame::Write(None));
    }
}

#[cfg(test)]
mod anchor_tests {
    use super::anchor_after_game_end;
    use time::macros::datetime;

    #[test]
    fn a_recorded_result_advances_the_anchor() {
        let start = datetime!(2026-08-17 14:00:00 UTC);
        let got = anchor_after_game_end(
            Some(&"6".to_string()),
            &"6".to_string(),
            Some(start),
            (None, None),
        );
        assert_eq!(got, (Some("6".to_string()), Some(start)));
    }

    #[test]
    fn an_abandoned_game_leaves_the_anchor_alone() {
        // No result was recorded, so as far as the tournament is concerned the
        // game has not happened. Re-offering it costs seconds; skipping it loses
        // the result and only surfaces at reconciliation.
        let prev_start = datetime!(2026-08-17 13:00:00 UTC);
        let current = (Some("5".to_string()), Some(prev_start));
        let got = anchor_after_game_end(
            None,
            &"6".to_string(),
            Some(datetime!(2026-08-17 14:00:00 UTC)),
            current.clone(),
        );
        assert_eq!(got, current);
    }

    #[test]
    fn a_result_recorded_for_a_different_game_leaves_the_anchor_alone() {
        let prev_start = datetime!(2026-08-17 13:00:00 UTC);
        let current = (Some("5".to_string()), Some(prev_start));
        let got = anchor_after_game_end(
            Some(&"5".to_string()),
            &"6".to_string(),
            Some(datetime!(2026-08-17 14:00:00 UTC)),
            current.clone(),
        );
        assert_eq!(got, current);
    }

    #[test]
    fn an_unscheduled_game_still_advances_the_number() {
        // A game the schedule does not know still counts as played. The search
        // then falls back to looking the anchor's time up, and answers Unknown if
        // it cannot — never a guess.
        let got =
            anchor_after_game_end(Some(&"6".to_string()), &"6".to_string(), None, (None, None));
        assert_eq!(got, (Some("6".to_string()), None));
    }
}

/// Fallback re-poll delay for the time updater when the clock is running but
/// the game state has no concrete next-update instant. This happens only in
/// degenerate zero-duration timing rules (e.g. the portal "FINALS" rule, whose
/// pre-overtime break is zero, producing a zero-length score-confirm pause).
/// Re-polling soon lets the state machine advance; it must never panic.
const UPDATER_NO_NEXT_TIME_FALLBACK: Duration = Duration::from_millis(100);

/// First delay before the clock updater retries a tick that failed.
const UPDATER_RETRY_BASE: Duration = Duration::from_millis(100);

/// Ceiling on the retry delay. A persistent fault therefore re-polls at most once a
/// second, which bounds how often it can write to the log without ever silencing it.
const UPDATER_RETRY_MAX: Duration = Duration::from_secs(1);

/// Delay before the clock updater retries after a failed tick: doubles per consecutive
/// failure from [`UPDATER_RETRY_BASE`], capped at [`UPDATER_RETRY_MAX`].
fn updater_retry_delay(consecutive_failures: u32) -> Duration {
    // 16 doublings of 100ms already far exceeds the cap; clamping the shift keeps the
    // multiply in range for any failure count.
    let shift = consecutive_failures.min(16);
    UPDATER_RETRY_BASE
        .saturating_mul(1u32 << shift)
        .min(UPDATER_RETRY_MAX)
}

/// Least time between full failure reports from the clock updater.
///
/// Only *repeat* reports are held back by this. The first failure of a run is always
/// reported immediately, and a held-back failure is never lost — the next report says
/// how many there were.
const UPDATER_REPORT_INTERVAL: Duration = Duration::from_secs(60);

/// How long after a failure a further failure still counts as the same fault.
///
/// This is why the back-off works on the fault this change exists to survive. That fault
/// *alternates* — a zero-length period fails at each period change and succeeds in
/// between — so a counter reset by any single success treats every failure as the first
/// one, never backs off, and logs at full rate forever. Elapsed time since the last
/// failure is the honest measure of "is this still the same problem", so that is what is
/// used, and success is not an input at all.
const UPDATER_FAULT_RUN_WINDOW: Duration = Duration::from_secs(60);

/// What the updater should do about a failure it has just recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FailureAction {
    /// How long to wait before ticking again.
    retry_after: Duration,
    /// Whether to write a full report, and how many failures went unreported since the
    /// last one. `None` means this failure was counted but not reported.
    report: Option<u32>,
}

/// The clock updater's failure policy: how fast to retry, and how often to report.
///
/// Extracted from the loop so it can be tested. Every previous attempt at this logic was
/// reasoned rather than verified, and each one was wrong in a new way — the retry rule,
/// the back-off reset and the log gating each had to be fixed after review.
#[derive(Debug, Default)]
struct UpdaterFailures {
    /// Failures in the current run, used only to size the back-off.
    consecutive: u32,
    last_failure: Option<Instant>,
    last_report: Option<Instant>,
    /// Failures counted since the last full report.
    unreported: u32,
}

impl UpdaterFailures {
    /// Record a failure at `now` and decide what to do about it.
    fn record(&mut self, now: Instant) -> FailureAction {
        let same_run = self
            .last_failure
            .is_some_and(|last| now.duration_since(last) < UPDATER_FAULT_RUN_WINDOW);
        if same_run {
            self.consecutive = self.consecutive.saturating_add(1);
        } else {
            self.consecutive = 0;
            // A new fault: report it in full straight away rather than holding it back
            // behind the previous fault's interval.
            self.last_report = None;
        }
        self.last_failure = Some(now);

        let due = self
            .last_report
            .is_none_or(|last| now.duration_since(last) >= UPDATER_REPORT_INTERVAL);
        let report = if due {
            self.last_report = Some(now);
            let held_back = self.unreported;
            self.unreported = 0;
            Some(held_back)
        } else {
            self.unreported = self.unreported.saturating_add(1);
            None
        };

        FailureAction {
            retry_after: updater_retry_delay(self.consecutive),
            report,
        }
    }
}

/// Decide when [`time_updater`] should next wake.
///
/// - Clock stopped: `None` — await the next clock-running change.
/// - Clock running with a concrete next-update instant: wake at that instant.
/// - Clock running but no next-update instant (degenerate state): re-poll after
///   [`UPDATER_NO_NEXT_TIME_FALLBACK`] so the state machine can advance.
///
/// Replaces an earlier `next_update_time(now).unwrap()` that crashed the whole
/// app (poisoning the shared game lock) when the value was absent.
fn next_updater_wake(
    clock_running: bool,
    next_update_time: Option<Instant>,
    now: Instant,
) -> Option<Instant> {
    clock_running.then(|| next_update_time.unwrap_or(now + UPDATER_NO_NEXT_TIME_FALLBACK))
}

#[cfg(test)]
mod updater_wake_tests {
    use super::*;

    #[test]
    fn retry_delay_starts_at_the_base_and_doubles() {
        assert_eq!(updater_retry_delay(0), UPDATER_RETRY_BASE);
        assert_eq!(updater_retry_delay(1), Duration::from_millis(200));
        assert_eq!(updater_retry_delay(2), Duration::from_millis(400));
        assert_eq!(updater_retry_delay(3), Duration::from_millis(800));
    }

    #[test]
    fn retry_delay_is_capped_so_a_stuck_fault_cannot_flood_the_log() {
        assert_eq!(updater_retry_delay(4), UPDATER_RETRY_MAX);
        assert_eq!(updater_retry_delay(50), UPDATER_RETRY_MAX);
        assert_eq!(updater_retry_delay(u32::MAX), UPDATER_RETRY_MAX);
    }

    #[test]
    fn first_failure_is_reported_at_once_with_nothing_held_back() {
        let mut failures = UpdaterFailures::default();
        let now = Instant::now();
        let action = failures.record(now);
        assert_eq!(action.report, Some(0));
        assert_eq!(action.retry_after, UPDATER_RETRY_BASE);
    }

    #[test]
    fn repeat_failures_are_held_back_and_counted_until_the_interval_passes() {
        let mut failures = UpdaterFailures::default();
        let start = Instant::now();
        assert_eq!(failures.record(start).report, Some(0));

        // Three more inside the interval: counted, not reported.
        for i in 1..=3 {
            let action = failures.record(start + Duration::from_secs(i));
            assert_eq!(action.report, None, "failure {i} should be held back");
        }

        // Once the interval passes, the next report accounts for all three.
        let action = failures.record(start + UPDATER_REPORT_INTERVAL);
        assert_eq!(action.report, Some(3));

        // And the count starts again.
        assert_eq!(
            failures
                .record(start + UPDATER_REPORT_INTERVAL + Duration::from_secs(1))
                .report,
            None
        );
    }

    /// The whole reason this policy exists. The zero-length-period fault fails at each
    /// period change and SUCCEEDS in between, so any rule that resets on success treats
    /// every failure as the first one: no back-off, and a full report every time.
    #[test]
    fn an_alternating_fault_still_backs_off() {
        let mut failures = UpdaterFailures::default();
        let start = Instant::now();

        // Ten failures a second apart, with successful ticks in the gaps that the policy
        // never hears about — because success is deliberately not an input.
        let mut last = UPDATER_RETRY_BASE;
        for i in 0..10 {
            last = failures.record(start + Duration::from_secs(i)).retry_after;
        }

        assert_eq!(
            last, UPDATER_RETRY_MAX,
            "an alternating fault must reach the retry ceiling, not sit at the base"
        );
    }

    #[test]
    fn a_failure_long_after_the_last_one_is_a_fresh_fault() {
        let mut failures = UpdaterFailures::default();
        let start = Instant::now();
        for i in 0..5 {
            failures.record(start + Duration::from_secs(i));
        }

        // Well past the run window: a new problem, not a continuation of the old one.
        let action = failures.record(start + UPDATER_FAULT_RUN_WINDOW + Duration::from_secs(10));
        assert_eq!(
            action.retry_after, UPDATER_RETRY_BASE,
            "a fresh fault must retry promptly rather than inherit the old back-off"
        );
        // Reported at once (not held behind the old fault's interval), and still
        // accounting for the four that were counted but never reported. No failure is
        // silently dropped, even across a fault boundary — the wording is "since the
        // last report", which stays true regardless of which fault they belonged to.
        assert_eq!(
            action.report,
            Some(4),
            "a fresh fault must be reported at once, and must not discard the unreported \
             failures that came before it"
        );
    }

    #[test]
    fn stopped_clock_has_no_scheduled_wake() {
        let now = Instant::now();
        assert_eq!(next_updater_wake(false, Some(now), now), None);
        assert_eq!(next_updater_wake(false, None, now), None);
    }

    #[test]
    fn running_clock_uses_concrete_next_time() {
        let now = Instant::now();
        let next = now + Duration::from_secs(1);
        assert_eq!(next_updater_wake(true, Some(next), now), Some(next));
    }

    #[test]
    fn running_clock_with_empty_next_time_falls_back_without_panicking() {
        // The FINALS-template crash case: clock running but no concrete next
        // update instant. Must NOT panic; schedules a short re-poll instead.
        let now = Instant::now();
        assert_eq!(
            next_updater_wake(true, None, now),
            Some(now + UPDATER_NO_NEXT_TIME_FALLBACK)
        );
    }
}

fn time_updater() -> impl Stream<Item = Message> {
    use iced::futures::SinkExt;
    debug!("Updater starting");

    iced::stream::channel(100, async |mut msg_tx| {
        let (tx, mut rx) = mpsc::channel(1);

        if msg_tx.try_send(Message::TimeUpdaterStarted(tx)).is_err() {
            error!("Clock updater could not register with the app; updater stopping");
            return;
        }

        let Some(tm) = rx.recv().await else {
            error!("Clock updater never received the game state; updater stopping");
            return;
        };
        let mut clock_running_receiver = tm.lock().get_start_stop_rx();
        let mut next_time = Some(Instant::now());
        let mut retry_after_failure: Option<Instant> = None;
        let mut failures = UpdaterFailures::default();

        loop {
            if let Some(retry_at) = retry_after_failure.take() {
                // A retry pause is a BOUND, so it must not be cancellable. Waiting on the
                // clock-latch channel here would let the failing tick cancel its own
                // pause — `send_clock_running` fires from inside several transitions, and
                // tokio's watch wakes on every send whether or not the value changed — so
                // a fault that touches the latch would spin the loop at full speed. On a
                // Raspberry Pi that is a pinned core for as long as the fault lasts.
                sleep_until(retry_at).await;
            } else if let Some(wake_at) = next_time {
                if wake_at > Instant::now() {
                    match timeout_at(wake_at, clock_running_receiver.changed()).await {
                        Err(_) => {}
                        Ok(Err(_)) => continue,
                        Ok(Ok(())) => {}
                    };
                } else {
                    if clock_running_receiver.has_changed().is_err() {
                        continue;
                    }
                }
            } else {
                debug!("Awaiting a new clock running message");
                if clock_running_receiver.changed().await.is_err() {
                    continue;
                }
            };

            // Read the latch rather than assuming. Waking on the timer used to mean
            // "assume the clock is still running", which is only true while `next_time`
            // is set exclusively for a running clock — an invariant the failure path
            // cannot honour, because a tick that fails as the clock stops still has to
            // retry or the screen, LED panel and overlay keep a stale running value.
            //
            // `borrow_and_update` rather than `borrow`: it marks the value seen, so a send
            // the previous tick made does not leave the next `changed()` resolving
            // instantly and skipping the wait.
            let clock_running = *clock_running_receiver.borrow_and_update();

            // One guarded call. Both a returned failure and an engine panic land here,
            // so there is a single place where a bad tick is handled — and no place
            // where the updater can force-unwrap the engine.
            let tick = catch_unwind(AssertUnwindSafe(|| {
                let mut tm_ = tm.lock();
                let now = Instant::now();
                let (kind, snapshot) = tm_.updater_tick(now)?;
                let next = next_updater_wake(clock_running, tm_.next_update_time(now), now);
                Ok::<_, TournamentManagerError>((kind, snapshot, next))
            }));

            // One failure path, not two. A returned error and a caught panic differ only
            // in how the reason reads; every decision after that — how long to wait,
            // whether to report, what to count — is identical, and keeping two copies of
            // it meant a fix applied to one arm and not the other.
            let (kind, snapshot, next) = match tick {
                Ok(Ok(values)) => values,
                failed => {
                    let reason = match failed {
                        Ok(Err(e)) => format!("failed: {e}"),
                        Err(payload) => format!("panicked: {}", panic_reason(&*payload)),
                        Ok(Ok(_)) => unreachable!("handled by the arm above"),
                    };
                    let now = Instant::now();
                    let action = failures.record(now);
                    match action.report {
                        Some(held_back) => {
                            let also = if held_back == 0 {
                                String::new()
                            } else {
                                format!(" ({held_back} further failures since the last report)")
                            };
                            // The state is read just after the tick released the lock, so
                            // it is the state moments after the failure rather than at the
                            // failing statement — close enough to diagnose, and honest
                            // about which it is.
                            error!(
                                "Clock updater tick {reason}{also}. Retrying in {:?}. \
                                 Game state just after the failure: {:#?}",
                                action.retry_after,
                                tm.lock()
                            );
                        }
                        None => trace!("Clock updater tick {reason} (report held back)"),
                    }
                    retry_after_failure = Some(now + action.retry_after);
                    continue;
                }
            };

            // Deliberately NOT reset on success. The fault this survives alternates —
            // fail at a period change, succeed in between — so clearing the policy here
            // would make every failure look like the first one again, which is the exact
            // defect this redesign exists to remove. `UpdaterFailures` ages itself out
            // via `UPDATER_FAULT_RUN_WINDOW` instead.
            next_time = next;

            let msg_type = match kind {
                TickKind::ConfirmScores => Message::ConfirmScores,
                TickKind::AutoConfirmScores => Message::AutoConfirmScores,
                TickKind::NewSnapshot => Message::NewSnapshot,
            };

            if msg_tx.send(msg_type(snapshot)).await.is_err() {
                debug!("App is no longer listening; clock updater stopping");
                break;
            }
        }
    })
}

/// Build a stream that forwards every `PortalEvent` emitted by the
/// background portal-manager task into the iced message loop. The
/// `shared` handle is cloned once by `subscription()` and passed here;
/// on first activation we `.take()` the Receiver out of the `Option`
/// so the stream owns it for the rest of the process's lifetime. The
/// subscription is registered with a stable ID, so iced 0.13 only
/// activates this factory once — a re-activation would find the
/// receiver already taken and emit nothing (which is safe but
/// indicates a bug). If the channel is closed (degraded mode or task
/// shutdown), we end the stream cleanly.
fn portal_event_stream(
    shared: Arc<Mutex<Option<mpsc::Receiver<PortalEvent>>>>,
) -> impl Stream<Item = Message> {
    use iced::futures::SinkExt;
    iced::stream::channel(100, async move |mut msg_tx| {
        // why this cannot panic: the guarded data is a plain Option and
        // no writer panics while holding it; poisoning simply yields the
        // last value, which we then `.take()` out.
        let rx_opt = shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(mut rx) = rx_opt else {
            debug!("portal_event_stream activated with no receiver; ending stream");
            return;
        };
        while let Some(ev) = rx.recv().await {
            if msg_tx.send(Message::PortalEvent(ev)).await.is_err() {
                break;
            }
        }
    })
}

#[cfg(test)]
mod countdown_beep_tests {
    use super::should_play_countdown_beep;
    use uwh_common::game_snapshot::GamePeriod;

    #[test]
    fn fires_each_second_10_down_to_1_in_each_break() {
        for p in [
            GamePeriod::BetweenGames,
            GamePeriod::HalfTime,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::PreSuddenDeath,
        ] {
            for s in 1..=10u32 {
                assert!(should_play_countdown_beep(p, s, s + 1, true), "{p:?} @ {s}");
            }
        }
    }

    #[test]
    fn silent_outside_window_when_disabled_or_unchanged() {
        assert!(!should_play_countdown_beep(
            GamePeriod::HalfTime,
            11,
            12,
            true
        ));
        assert!(!should_play_countdown_beep(
            GamePeriod::HalfTime,
            0,
            1,
            true
        ));
        assert!(!should_play_countdown_beep(
            GamePeriod::HalfTime,
            10,
            11,
            false
        ));
        assert!(!should_play_countdown_beep(
            GamePeriod::HalfTime,
            10,
            10,
            true
        ));
    }

    #[test]
    fn never_fires_during_playing_periods() {
        for p in [
            GamePeriod::FirstHalf,
            GamePeriod::SecondHalf,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::OvertimeSecondHalf,
            GamePeriod::SuddenDeath,
        ] {
            assert!(!should_play_countdown_beep(p, 5, 6, true), "{p:?}");
        }
    }
}

#[cfg(test)]
mod submission_gate_tests {
    use super::recorded_result_matches_ended_game;

    #[test]
    fn result_for_the_same_game_is_submitted() {
        let recorded = "16".to_string();
        assert!(recorded_result_matches_ended_game(
            Some(&recorded),
            &"16".to_string()
        ));
    }

    #[test]
    fn result_for_an_earlier_game_is_not_submitted() {
        // The forfeit incident: game 18 was abandoned mid-play, so the newest recorded
        // result belongs to game 16 and must not be posted against 18.
        let recorded = "16".to_string();
        assert!(!recorded_result_matches_ended_game(
            Some(&recorded),
            &"18".to_string()
        ));
    }

    #[test]
    fn no_recorded_result_is_not_submitted() {
        // First game of the session, or a fresh restart.
        assert!(!recorded_result_matches_ended_game(None, &"18".to_string()));
    }
}

#[cfg(test)]
mod source_tap_tests {
    use super::*;

    #[test]
    fn nothing_selected_switches_at_once() {
        assert_eq!(
            source_tap_outcome(false, false, false),
            SourceTapOutcome::SwitchNow
        );
    }

    /// The confirmation appears only when there is something to lose. A partial
    /// selection is not something to lose: the operator chose fewer
    /// interruptions over an always-identical button (spec, 2026-08-28).
    #[test]
    fn a_fully_linked_game_asks_first() {
        assert_eq!(
            source_tap_outcome(false, false, true),
            SourceTapOutcome::Confirm
        );
    }

    #[test]
    fn a_game_in_progress_refuses() {
        assert_eq!(
            source_tap_outcome(true, false, false),
            SourceTapOutcome::RefusedByGame
        );
    }

    #[test]
    fn queued_results_refuse() {
        assert_eq!(
            source_tap_outcome(false, true, false),
            SourceTapOutcome::RefusedByQueue
        );
    }

    /// A refusal outranks a confirmation. Asking "shall I clear your game?"
    /// about a switch that is going to be refused anyway is worse than useless:
    /// it offers a choice the refbox will not honour.
    #[test]
    fn a_refusal_outranks_a_confirmation() {
        assert_eq!(
            source_tap_outcome(true, false, true),
            SourceTapOutcome::RefusedByGame
        );
        assert_eq!(
            source_tap_outcome(false, true, true),
            SourceTapOutcome::RefusedByQueue
        );
    }

    /// The two refusals have a fixed order between them: a running game is the
    /// more urgent thing to say.
    #[test]
    fn a_game_in_progress_outranks_queued_results() {
        for fully_linked in [false, true] {
            assert_eq!(
                source_tap_outcome(true, true, fully_linked),
                SourceTapOutcome::RefusedByGame
            );
        }
    }

    #[test]
    fn reply_from_the_current_site_is_accepted() {
        assert!(reply_is_current(0, 0));
        assert!(reply_is_current(7, 7));
    }

    #[test]
    fn reply_from_a_departed_site_is_rejected() {
        // The refbox has moved on since the request went out.
        assert!(!reply_is_current(0, 1));
        assert!(!reply_is_current(3, 9));
    }

    #[test]
    fn a_stamp_from_the_future_is_also_rejected() {
        // Cannot happen today, but the rule is equality, not "not older than".
        // Anything but an exact match is data of uncertain origin, which this
        // guard exists to refuse.
        assert!(!reply_is_current(5, 2));
    }
}

#[cfg(test)]
mod reply_source_tests {
    use super::{GameSource, LinkCommit, link_commit, reply_source};

    /// A reply arriving after a merely STAGED remote-to-remote change still
    /// belongs to the committed source. Following the stage instead would file
    /// the departed site's schedule under the site just arrived at — the leak
    /// the per-source store exists to close.
    #[test]
    fn a_staged_remote_does_not_redirect_a_reply() {
        assert_eq!(
            reply_source(GameSource::Portal, Some(GameSource::Custom)),
            GameSource::Portal
        );
        assert_eq!(
            reply_source(GameSource::Custom, Some(GameSource::Portal)),
            GameSource::Custom
        );
    }

    /// Would have failed before the fix. No site serves Manual, so no request is
    /// ever issued for it; a reply landing while Manual is committed belongs to
    /// the remote the operator staged. Resolving it against Manual discarded it
    /// silently, leaving COURT and GAME unfillable.
    #[test]
    fn a_reply_over_committed_manual_resolves_against_the_staged_remote() {
        assert_eq!(
            reply_source(GameSource::Manual, Some(GameSource::Portal)),
            GameSource::Portal
        );
        assert_eq!(
            reply_source(GameSource::Manual, Some(GameSource::Custom)),
            GameSource::Custom
        );
    }

    /// With no editor open there is nothing staged, so Manual stays Manual —
    /// the fallback must not invent a source.
    #[test]
    fn manual_with_nothing_staged_stays_manual() {
        assert_eq!(reply_source(GameSource::Manual, None), GameSource::Manual);
        assert_eq!(reply_source(GameSource::Portal, None), GameSource::Portal);
    }

    #[test]
    fn an_owned_selection_is_committed_whatever_the_source() {
        for source in [GameSource::Portal, GameSource::Custom, GameSource::Manual] {
            assert_eq!(link_commit(true, source), LinkCommit::Staged);
        }
    }

    /// Applying Manual means there is no link, so the committed one goes.
    #[test]
    fn applying_manual_clears_the_committed_link() {
        assert_eq!(link_commit(false, GameSource::Manual), LinkCommit::Clear);
    }

    /// Would have failed before the fix: this returned `Clear`, which wiped the
    /// committed event/court/schedule and let `persist_link_session` delete
    /// portal_link.json — reachable with nothing worse than a portal event list
    /// that had not loaded yet.
    #[test]
    fn an_unowned_remote_selection_leaves_the_committed_link_alone() {
        assert_eq!(link_commit(false, GameSource::Portal), LinkCommit::Leave);
        assert_eq!(link_commit(false, GameSource::Custom), LinkCommit::Leave);
    }
}

#[cfg(test)]
mod picker_roster_game_tests {
    use super::{GamePeriod, GameSnapshot, picker_roster_game};

    fn snap(period: GamePeriod, game: &str, next: &str) -> GameSnapshot {
        GameSnapshot {
            current_period: period,
            game_number: game.to_string(),
            next_game_number: next.to_string(),
            ..GameSnapshot::default()
        }
    }

    /// Before the first kickoff of a session the app sits in BetweenGames with no
    /// prior game, and the picker must already offer the first game's roster --
    /// the requirement this change exists for. Before the fix nothing is offered
    /// there at all and every picker falls through to the 0-9 pad.
    #[test]
    fn before_the_first_kickoff_follows_the_upcoming_game() {
        assert_eq!(
            picker_roster_game(&snap(GamePeriod::BetweenGames, "0", "27")),
            Some(&"27".to_string()),
        );
    }

    /// An entry made anywhere in the break belongs to the game about to start, so
    /// the picker follows that game from the whistle -- it must not switch partway
    /// through the break. `is_old_game` marks the engine's changeover partway in;
    /// it must make no difference here.
    #[test]
    fn the_whole_break_follows_the_upcoming_game() {
        for is_old_game in [true, false] {
            let mut s = snap(GamePeriod::BetweenGames, "27", "15");
            s.is_old_game = is_old_game;
            assert_eq!(
                picker_roster_game(&s),
                Some(&"15".to_string()),
                "is_old_game={is_old_game} must not change which game the picker follows",
            );
        }
    }

    /// Compile-time guard for the list below. This match has no wildcard arm, so
    /// adding a `GamePeriod` variant stops this file compiling and forces whoever
    /// adds it to say what the picker should do in that period, instead of the
    /// new period silently escaping an enumerated list. `GamePeriod` cannot be
    /// iterated here -- that would need a derive in `uwh-common`, which this
    /// branch does not touch -- so this is the next best thing.
    fn is_between_games(period: GamePeriod) -> bool {
        match period {
            GamePeriod::BetweenGames => true,
            GamePeriod::FirstHalf
            | GamePeriod::HalfTime
            | GamePeriod::SecondHalf
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::PreSuddenDeath
            | GamePeriod::SuddenDeath => false,
        }
    }

    /// During play the copy pinned at kickoff is used, so a mid-game REFRESH
    /// cannot move the grid under the operator's hand. This is the guarantee the
    /// original grid design was built on and it must survive this change.
    #[test]
    fn during_play_keeps_the_kickoff_copy() {
        for period in [
            GamePeriod::FirstHalf,
            GamePeriod::HalfTime,
            GamePeriod::SecondHalf,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::OvertimeSecondHalf,
            GamePeriod::PreSuddenDeath,
            GamePeriod::SuddenDeath,
        ] {
            assert!(
                !is_between_games(period),
                "{period:?} is BetweenGames -- it does not belong in this list",
            );
            assert_eq!(
                picker_roster_game(&snap(period, "27", "15")),
                None,
                "{period:?} must keep the roster pinned at kickoff",
            );
        }
    }
}

#[cfg(test)]
mod rosters_for_scheduled_game_tests {
    use super::*;
    use time::OffsetDateTime;
    use uwh_common::uwhportal::schedule::{Game, ScheduledTeam};

    fn game_on(number: &str, court: &str) -> Game {
        Game {
            number: number.to_string(),
            dark: ScheduledTeam::new_team_id(TeamId::from_partial("dark")),
            light: ScheduledTeam::new_team_id(TeamId::from_partial("light")),
            start_time: OffsetDateTime::UNIX_EPOCH,
            court: court.to_string(),
            timing_rule: "RR".to_string(),
            referee_assignments: None,
            description: None,
        }
    }

    fn schedule_with(game: Game) -> Schedule {
        Schedule {
            event_id: EventId::from_full("events/1889-B").unwrap(),
            games: std::iter::once((game.number.clone(), game)).collect(),
            non_game_entries: Vec::new(),
            groups: Vec::new(),
            timing_rules: Vec::new(),
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }

    fn cached_rosters() -> BTreeMap<TeamId, Vec<u8>> {
        BTreeMap::from([
            (TeamId::from_partial("dark"), vec![3, 6, 9]),
            (TeamId::from_partial("light"), vec![2, 7]),
        ])
    }

    /// The ordinary case: a game on this court supplies both teams' numbers.
    #[test]
    fn a_game_on_this_court_supplies_both_rosters() {
        let schedule = schedule_with(game_on("27", "Court 1"));
        let out = rosters_for_scheduled_game(
            Some(&schedule),
            &cached_rosters(),
            Some("Court 1"),
            &"27".to_string(),
        );
        assert_eq!(out[Color::Black], vec![3, 6, 9]);
        assert_eq!(out[Color::White], vec![2, 7]);
    }

    /// When no next game is scheduled the engine synthesises a game number by
    /// incrementing the current one, and game numbers are unique across a whole
    /// event rather than per court -- so that invented number can name a real
    /// game being played somewhere else. Offering its two teams would be
    /// confidently wrong, with nothing on screen to say so. The number pad,
    /// which claims nothing, is the right answer instead.
    #[test]
    fn a_game_on_another_court_supplies_nothing() {
        let schedule = schedule_with(game_on("28", "Court 2"));
        let out = rosters_for_scheduled_game(
            Some(&schedule),
            &cached_rosters(),
            Some("Court 1"),
            &"28".to_string(),
        );
        assert!(
            out[Color::Black].is_empty(),
            "another court's dark team must not be offered",
        );
        assert!(
            out[Color::White].is_empty(),
            "another court's light team must not be offered",
        );
    }

    /// No court selected is not a mismatch. Every existing caller keeps the
    /// behaviour it has today rather than silently losing its grid in a state
    /// that has never been exercised.
    #[test]
    fn no_current_court_still_supplies_the_roster() {
        let schedule = schedule_with(game_on("27", "Court 1"));
        let out =
            rosters_for_scheduled_game(Some(&schedule), &cached_rosters(), None, &"27".to_string());
        assert_eq!(out[Color::Black], vec![3, 6, 9]);
        assert_eq!(out[Color::White], vec![2, 7]);
    }

    /// With no schedule there is nothing to look a roster up in, so every team
    /// gets the number pad. This is what "the portal is off means the pad" rests
    /// on, and it is asserted here rather than left to the invariant that Manual
    /// mode always coincides with an absent schedule -- that invariant lives
    /// three hops away in another module and nothing here pins it.
    #[test]
    fn no_schedule_supplies_nothing() {
        let out =
            rosters_for_scheduled_game(None, &cached_rosters(), Some("Court 1"), &"27".to_string());
        assert!(out[Color::Black].is_empty());
        assert!(out[Color::White].is_empty());
    }

    /// The other shape the synthesised number takes: one that is in no schedule
    /// at all.
    #[test]
    fn an_unknown_game_supplies_nothing() {
        let schedule = schedule_with(game_on("27", "Court 1"));
        let out = rosters_for_scheduled_game(
            Some(&schedule),
            &cached_rosters(),
            Some("Court 1"),
            &"99".to_string(),
        );
        assert!(out[Color::Black].is_empty());
        assert!(out[Color::White].is_empty());
    }
}
