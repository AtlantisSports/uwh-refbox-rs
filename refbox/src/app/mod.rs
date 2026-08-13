use self::infraction::InfractionDetails;
use super::{APP_NAME, fl};
use crate::{
    beep_test::{cadence::TournamentManager as BeepTestManager, snapshot::BeepTestSnapshot},
    config::{Config, CustomSite, GameSource, Mode, RemoteSource},
    penalty_editor::*,
    portal_manager::{ItemId, PortalEvent, PortalManager, SelectedEventId, UwhPortalIo},
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
    cmp::min,
    collections::{BTreeMap, BTreeSet},
    process::Child,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant, sleep, timeout_at},
};
use tokio_serial::SerialPortBuilder;
use uwh_common::{
    bundles::*,
    color::Color,
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{GamePeriod, GameSnapshot, Infraction, TimeoutSnapshot},
    uwhportal::{
        PortalTokenResponse, RosterPlayer, UwhPortalClient,
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
    tm: Arc<Mutex<TournamentManager>>,
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
    /// Where `uwhportal_client` actually points, kept in step with every
    /// rebuild. Read to decide whether an Apply would repoint the refbox at a
    /// different site (and so needs the clock and queue guards).
    current_site: SiteTarget,
    /// `--allow-http` inverted, as passed at launch. Governs the built-in
    /// portal only — a custom site derives TLS from the scheme that was typed.
    require_https: bool,
    source: GameSource,
    events: Option<BTreeMap<EventId, Event>>,
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
    /// One-shot: the event whose schedule to fetch once the event list lands
    /// during a startup link restore. Deferred (rather than fetched at startup)
    /// so the schedule arrives after `self.events` is populated — `RecvSchedule`
    /// requires the event to be present there. Cleared on first use.
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
    base_url: String,
    /// `https_only` on the HTTP client. Fixed when the client is built, so a
    /// change here means building a new client rather than editing the old one.
    require_https: bool,
    /// The whole address this came from, which for a custom site includes the
    /// event. Carried so that comparing two targets asks "is this a different
    /// address?" rather than only "is this a different host?" — editing just
    /// the event in the URL must meet the same guards as editing the host.
    address: String,
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
                    base_url: parsed.base_url,
                    address: custom_site.url.trim().to_string(),
                })
        }
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
    if url_override.is_some() {
        info!(
            "{override_var} active for {} Portal: using {base_url}",
            portal_name_for_mode(mode)
        );
    }
    SiteTarget {
        kind: SiteKind::Portal,
        address: base_url.clone(),
        base_url,
        require_https,
    }
}

/// Build a client for `target`, using the credential that belongs to that site.
///
/// `None` when the client cannot be built at all, which leaves the refbox in
/// its existing degraded mode (red indicator, nothing sent) rather than holding
/// a client pointed somewhere unintended.
fn build_site_client(target: &SiteTarget, config: &Config) -> Option<UwhPortalClient> {
    let token = match target.kind {
        SiteKind::Portal => config.uwhportal.token.as_str(),
        SiteKind::Custom => config.custom_site.token.as_str(),
    };
    let token = (!token.is_empty()).then_some(token);
    match UwhPortalClient::new(
        &target.base_url,
        token,
        target.require_https,
        REQUEST_TIMEOUT,
    ) {
        Ok(c) => Some(c),
        Err(e) => {
            error!("Failed to start the client for {}: {e}", target.base_url);
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
                    prereqs && is_whistle_period && new_snapshot.secs_in_period == 30,
                    prereqs && is_buzz_period && new_snapshot.secs_in_period == 0,
                )
            }
        };

        let play_countdown = new_snapshot.timeout.is_none()
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

    fn request_event_list(&self) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // why this cannot panic: the `UwhPortalClient` is only mutated by
            // `set_token`/`clear_token`, neither of which panics, so the
            // mutex is never poisoned in practice.
            let request = client
                .lock()
                .unwrap()
                .get_event_list(self.list_all_events, true);
            Task::future(async move {
                match request.await {
                    Ok(events) => {
                        info!("Got event list");
                        Message::RecvEventList(events)
                    }
                    Err(e) => {
                        error!("Failed to get event list: {e}");
                        Message::NoAction
                    }
                }
            })
        } else {
            Task::none()
        }
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
            // why this cannot panic: see `request_event_list` above.
            let request = client.lock().unwrap().get_event_teams(&event_id);
            Task::future(async move {
                match request.await {
                    Ok(teams) => {
                        info!("Got teams list");
                        Message::RecvTeamsList(event_id, teams)
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
            // why this cannot panic: see `request_event_list` above.
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
                        Message::RecvTeamRoster(team_id, numbers)
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
        if let Some(client) = &self.uwhportal_client {
            // why this cannot panic: see `request_event_list` above.
            let guard = client.lock().unwrap();
            let schedule_req = guard.get_event_schedule_privileged(&event_id);
            let names_req = guard.get_event_referee_name_map_from_referees(&event_id);
            drop(guard);
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
                Message::RecvSchedule(event_id, schedule)
            })
        } else {
            Task::none()
        }
    }

    fn request_uwhportal_token(&self, event_id: &EventId, code: u32) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // why this cannot panic: see `request_event_list` above.
            let request = client.lock().unwrap().login_to_portal(event_id, code);
            let portal_name = portal_name_for_mode(self.config.mode);
            Task::future(async move {
                match request.await {
                    Ok(token) => {
                        info!("Got a response from {portal_name} Portal token request");
                        Message::RecvPortalToken(token)
                    }
                    Err(e) => {
                        error!("Failed to get {portal_name} portal token: {e}");
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
        // Safety: Mutex poison only occurs if another thread already panicked; the refbox treats that as fatal (matches the 20+ identical sites in this file).
        if self.tm.lock().unwrap().current_period() != GamePeriod::BetweenGames {
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
            error!(
                "Cannot point the refbox at {}: no client was started",
                target.base_url
            );
            return;
        };
        let Some(new_client) = build_site_client(&target, &self.config) else {
            return;
        };
        // Logs the whole address, not just the host: a custom site and the
        // developer override can share a host, and "which site am I on?" is the
        // first question asked of this line.
        let kind = match target.kind {
            SiteKind::Portal => "portal",
            SiteKind::Custom => "custom site",
        };
        info!("Pointing the refbox at the {kind}: {}", target.address);
        // why this cannot panic: the guard is held only for the assignment
        // below, which cannot panic, so the mutex is never poisoned here.
        *shared.lock().unwrap() = new_client;
        self.current_site = target;
    }

    /// Re-seed the ACCESS TOKEN indicator after the refbox has been pointed at
    /// a different site.
    ///
    /// Without this the row keeps the verdict it reached about the *previous*
    /// site, which is worse than saying nothing: the credential is per-site, so
    /// an OK earned from the portal would sit above a third-party site the
    /// refbox has never authenticated to. Mirrors the seeding done when the
    /// settings editor is opened.
    fn refresh_token_indicator(&mut self) -> Task<Message> {
        let has_token = match self.uwhportal_client.as_ref() {
            // why this cannot panic: the guard is held only for a synchronous
            // `has_token()` call and dropped immediately.
            Some(client) => client.lock().unwrap().has_token(),
            None => false,
        };
        let (valid, task) = match (has_token, self.current_event_id.clone()) {
            (true, Some(id)) => (None, self.check_uwhportal_auth(&id)),
            // No credential for this site, or no event to check it against:
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
    /// — so the events-map entry the portal path gets from that list has to be
    /// made here. That entry is not bookkeeping: `RecvTeamsList` and
    /// `RecvSchedule` both store *into* it and log an error when it is missing,
    /// and the court picker reads its court list from it, so without one the
    /// court list stays permanently empty.
    ///
    /// Safe to call more than once: the entry is only created when absent, so
    /// re-adopting refreshes the data without discarding what has arrived.
    fn adopt_custom_event(&mut self) -> Task<Message> {
        let parsed = match custom_site::parse_custom_site(&self.config.custom_site.url) {
            Ok(parsed) => parsed,
            Err(e) => {
                // Only reachable from a hand-edited config file: every path that
                // commits a URL validates it first.
                error!("Saved custom site address is not usable ({e:?}); no event adopted");
                return Task::none();
            }
        };
        let event_id = parsed.event_id;
        let event_changed = self.current_event_id.as_ref() != Some(&event_id);

        let events = self.events.get_or_insert_with(BTreeMap::new);
        events.entry(event_id.clone()).or_insert_with(|| Event {
            id: event_id.clone(),
            // Only ever shown in the event picker, which a custom site never
            // opens. The id keeps it recognisable in a log rather than blank.
            name: event_id.partial().to_string(),
            slug: String::new(),
            // A custom site serves no event-level date range, and the only
            // reader sorts the event picker — unreachable here. Both ends are
            // set to now rather than invented.
            date_range: DateRange {
                start: time::OffsetDateTime::now_utc(),
                end: time::OffsetDateTime::now_utc(),
            },
            teams: None,
            schedule: None,
            courts: None,
        });

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

    fn check_uwhportal_auth(&self, event_id: &EventId) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // why this cannot panic: see `request_event_list` above.
            let has_token = client.lock().unwrap().has_token();
            if !has_token {
                // Never ask a site to vouch for a credential we do not hold.
                // Only the site can enforce a token, and a permissive one
                // answers an unauthenticated probe with `200` — which arrives
                // as a green OK painted over nothing. Report FAILED here
                // instead, without sending the request.
                return Task::done(Message::RecvTokenValid(event_id.clone(), false));
            }
            // why this cannot panic: see `request_event_list` above.
            let request = client.lock().unwrap().verify_token(event_id);
            // Tag the result with the event it was checked for so the handler
            // can drop a late reply for a previously-selected event.
            let event_id = event_id.clone();
            Task::future(async move {
                match request.await {
                    Ok(()) => {
                        info!("Portal token validated");
                        Message::RecvTokenValid(event_id, true)
                    }
                    Err(e) => {
                        error!("Portal token validity check failed: {e}");
                        Message::RecvTokenValid(event_id, false)
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    /// Both teams' cap numbers for a scheduled game, read from the session
    /// cache. Empty vectors where the slot has no portal team assigned (a
    /// placeholder such as "winner of A") or no roster has arrived — those
    /// teams get the number pad.
    fn rosters_for_game(&self, game_num: &GameNumber) -> BlackWhiteBundle<Vec<u8>> {
        let mut out = BlackWhiteBundle {
            black: Vec::new(),
            white: Vec::new(),
        };

        if let Some(schedule) = &self.schedule {
            if let Some(game) = schedule.games.get(game_num) {
                for (color, team) in [(Color::Black, &game.dark), (Color::White, &game.light)] {
                    if let Some(numbers) = team.assigned().and_then(|id| self.team_rosters.get(id))
                    {
                        out[color] = numbers.clone();
                    }
                }
            }
        }

        out
    }

    fn handle_game_start(&mut self, new_game_num: &GameNumber) -> Task<Message> {
        // Fix this game's rosters now. From here until the next kickoff they do
        // not change: a REFRESH mid-game re-pulls the event, but must not move
        // the grid under the operator's hand.
        self.game_rosters = self.rosters_for_game(new_game_num);

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

                let next_game = schedule
                    .games
                    .values()
                    .filter(|game| game.court == *pool)
                    .filter(|game| game.start_time > this_game_start)
                    .min_by_key(|game| game.start_time);

                let mut tm = self.tm.lock().unwrap();
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
                    error!("Couldn't find a next game");
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
                // Safety: Mutex poison only occurs if another thread already panicked; the refbox treats that as fatal (matches the 20+ identical sites in this file).
                let tm = self.tm.lock().unwrap();
                tm.last_game_info()
                    .map(|info| (info.game_number.clone(), info.scores, info.stats.as_json()))
            };

            match recorded {
                Some((recorded_game, scores, stats))
                    if recorded_result_matches_ended_game(Some(&recorded_game), game_number) =>
                {
                    info!("Game ended, scores: {scores:?} stats were: {stats:?}");

                    if let Some(ref event_id) = self.current_event_id {
                        let event_id_str = event_id.full().to_string();
                        tasks.push(self.request_schedule(event_id.clone()));
                        if let Err(e) = self.portal_manager.enqueue_game_end(
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

        #[cfg(debug_assertions)]
        if self.scramble_token_pending && new_is_some {
            if let Some(client) = self.uwhportal_client.as_ref() {
                let mut guard = client
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                guard.set_token("invalid-debug-token");
                warn!("UWH_PORTAL_SCRAMBLE_TOKEN: in-memory token replaced after event linked");
            }
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
                // The game the operator is on: the upcoming game between games,
                // otherwise the current game number from the live snapshot.
                let game_number = if self.snapshot.current_period == GamePeriod::BetweenGames {
                    Some(self.snapshot.next_game_number.clone())
                } else {
                    Some(self.snapshot.game_number.clone())
                };
                let note = LinkSessionFile {
                    version: LinkSessionFile::CURRENT_VERSION,
                    event_id,
                    court: self.current_court.clone(),
                    game_number,
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

    fn apply_app_options(&mut self) -> Option<ConfirmationKind> {
        let edited = self.edited_settings.as_ref()?;
        // Snapshot the fields we need so the immutable borrow on
        // `edited_settings` ends before we call `set_current_event_id`
        // (which takes `&mut self`).
        let source = edited.source;
        let event_id = edited.current_event_id.clone();
        let current_court = edited.current_court.clone();
        let schedule = edited.schedule.clone();
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

        // Committed here, while `edited` is still borrowed, so the six toggles
        // need no per-field locals. `config` and `edited_settings` are disjoint
        // fields, so this mutable borrow and the immutable one above coexist.
        let hide_time_changed = commit_app_toggles(&mut self.config, edited);

        self.commit_source(source);
        // Route through set_current_event_id so portal_event_id stays in
        // sync (ADR 011 amendment 2026-04-23 dormant-until-linked).
        self.set_current_event_id(event_id);
        self.current_court = current_court;
        self.schedule = schedule;
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
        self.commit_source(GameSource::Manual);
        // Route through set_current_event_id so portal_event_id stays in sync (ADR 011).
        self.set_current_event_id(None);
        self.current_court = None;
        self.schedule = None;
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
        let edited = self.edited_settings.as_ref()?;

        if edited.uwhportal_incomplete() {
            return Some(ConfirmationKind::UwhPortalIncompleteFromApply);
        }

        // Safety: Mutex poison only occurs if another thread already panicked; the refbox treats that as fatal (matches the 20+ identical sites in this file).
        let mut tm = self.tm.lock().unwrap();

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

        let new_config = if edited.uses_remote() {
            edited
                .schedule
                .as_ref()
                .and_then(|schedule| schedule.get_game_timing(&edited.game_number))
                .cloned()
                .map(|tr| tr.into())
                .unwrap_or_else(|| tm.config().clone())
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
            // Snapshot the fields we need so the immutable borrow on
            // `edited` ends before we call `set_current_event_id`
            // (which takes `&mut self`).
            let source = edited.source;
            let event_id = edited.current_event_id.clone();
            let current_court = edited.current_court.clone();
            let schedule = edited.schedule.clone();

            self.config.game = new_config;
            self.commit_source(source);
            // Route through set_current_event_id so portal_event_id stays in
            // sync (ADR 011 amendment 2026-04-23 dormant-until-linked).
            self.set_current_event_id(event_id);
            self.current_court = current_court;
            self.schedule = schedule;
            return None;
        }

        if edited.game_number != self.snapshot.game_number {
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
        // Snapshot the fields we need so the immutable borrow on `edited` ends
        // before we call `set_current_event_id` (which takes `&mut self`).
        let source = edited.source;
        let event_id = edited.current_event_id.clone();
        let current_court = edited.current_court.clone();
        let schedule = edited.schedule.clone();

        self.commit_source(source);
        // Route through set_current_event_id so portal_event_id stays in sync
        // for the background health check (ADR 011 amendment 2026-04-23).
        self.set_current_event_id(event_id);
        self.current_court = current_court;
        self.schedule = schedule;

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
                // Safety: Mutex poison only occurs if another thread already panicked; the refbox treats that as fatal (matches the 20+ identical sites in this file).
                let mut tm = self.tm.lock().unwrap();
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
                self.persist_config();
                // Safety: snapshot generation only fails before the tournament manager is initialised, which happens in RefBoxApp::new().
                let new_snapshot = self.tm.lock().unwrap().generate_snapshot(now).unwrap();
                task = self.apply_snapshot(new_snapshot);
                AppState::EditGameConfig(ConfigPage::Main)
            }
            ConfirmationOption::KeepGameAndApply => {
                // Safety: *FromApply confirmations are only raised while edited_settings is Some; the invariant is enforced by apply_game_options.
                let edited = self.edited_settings.as_ref().unwrap();
                // Safety: Mutex poison only occurs if another thread already panicked; the refbox treats that as fatal (matches the 20+ identical sites in this file).
                let mut tm = self.tm.lock().unwrap();
                tm.set_game_number(&edited.game_number);
                // Safety: snapshot generation only fails before the tournament manager is initialised, which happens in RefBoxApp::new().
                let new_snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                std::mem::drop(tm);
                self.page_entry_snapshot = None;
                self.persist_config();
                task = self.apply_snapshot(new_snapshot);
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
                    if let Err(e) = crate::portal_manager::queue::save(
                        self.portal_manager.queue_dir(),
                        &crate::portal_manager::queue::QueueFile::empty(),
                    ) {
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
                    // Safety: Mutex poison only occurs if another thread already
                    // panicked; the refbox treats that as fatal (matches the 20+
                    // identical sites in this file).
                    let mut tm = self.tm.lock().unwrap();
                    tm.reset_game(now);
                    // `set_config` must run BEFORE `reset_to_manual_break` because
                    // `reset_to_manual_break` reads `self.config.nominal_break` to
                    // set the break clock.  After `reset_game` the period is
                    // `BetweenGames`, so `set_config` cannot error.
                    tm.set_config(manual_config.clone()).unwrap();
                    // Overrides reset_game's minimum-break clock with the nominal
                    // break; also resets the game number to "0", clears the
                    // next-game / grid, and starts the break counting down.
                    tm.reset_to_manual_break(now);
                }
                self.clear_portal_selections_to_manual(manual_config);
                // Clear the page-entry snapshot like the sibling apply arms so a
                // later path can't read a stale "has changes" state.
                self.page_entry_snapshot = None;
                // Safety: snapshot generation only fails before the tournament
                // manager is initialised, which happens in `RefBoxApp::new()`.
                let new_snapshot = self.tm.lock().unwrap().generate_snapshot(now).unwrap();
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
                    // Safety: Mutex poison — same rationale as the EndGameAndApply arm.
                    let mut tm = self.tm.lock().unwrap();
                    tm.clear_portal_next_game();
                }
                self.clear_portal_selections_to_manual(manual_config);
                // Clear the page-entry snapshot like the sibling apply arms so a
                // later path can't read a stale "has changes" state.
                self.page_entry_snapshot = None;
                // Safety: snapshot generation only fails before the tournament
                // manager is initialised, which happens in `RefBoxApp::new()`.
                let new_snapshot = self
                    .tm
                    .lock()
                    .unwrap()
                    .generate_snapshot(Instant::now())
                    .unwrap();
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

        let uwhportal_token_valid = if let Some(ref client) = self.uwhportal_client {
            // why this cannot panic: the guard is held only for a synchronous
            // `has_token()` call and dropped immediately.
            let has_token = client.lock().unwrap().has_token();
            if has_token {
                if let Some(event_id) = self.current_event_id.as_ref() {
                    task = self.check_uwhportal_auth(event_id);
                    None
                } else {
                    Some(false)
                }
            } else {
                Some(false)
            }
        } else {
            Some(false)
        };

        let edited_settings = EditableSettings {
            config: self.tm.lock().unwrap().config().clone(),
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

        let mut tm = TournamentManager::new(config.game.clone());
        tm.start_clock(Instant::now());

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
            if site_target(
                GameSource::Custom,
                config.mode,
                &config.custom_site,
                require_https,
            )
            .is_some()
            {
                GameSource::Custom
            } else {
                error!(
                    "Saved custom site address is not usable ({:?}); starting with manual games",
                    config.custom_site.url
                );
                GameSource::Manual
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
        let uwhportal_client =
            build_site_client(&current_site, &config).map(|c| Arc::new(Mutex::new(c)));

        // Shared event id the background portal task consults for its
        // periodic `verify_token` check. Mirrors `current_event_id` on
        // `RefBoxApp`; both start `None` here and are kept in sync via
        // `set_current_event_id` on every subsequent write.
        let portal_event_id: SelectedEventId = Arc::new(Mutex::new(None));

        let tm = Arc::new(Mutex::new(tm));

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
                PortalManager::new_degraded()
            }
            // Have a client: build the real uploader, trying `config_dir` first,
            // then `std::env::temp_dir()`, then falling back to degraded mode if
            // even the temp dir refuses I/O. Each attempt gets its own freshly
            // built `UwhPortalIo`, so the retry helper is a closure.
            Some(client) => {
                let try_new_manager = |dir: &std::path::Path| {
                    PortalManager::new(
                        dir,
                        UwhPortalIo::new(Arc::clone(client), Arc::clone(&portal_event_id)),
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
                                PortalManager::new_degraded()
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
            current_site,
            require_https,
            source: startup_source,
            events: None,
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
                        note.game_number
                    );
                    new.source = GameSource::Portal;
                    new.current_court = note.court.clone();
                    new.pending_restore_game = note.game_number.clone();
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

        // Portal subsystem stays dormant until the operator turns Using-UWH-Portal
        // ON (or a recent link was restored above): no event-list fetch fires at
        // startup unless the runtime flag is true.
        // See ADR 017 (Portal Data Lifecycle) for the dormancy contract.
        let mut startup_tasks = vec![if fullscreen {
            window::get_latest().and_then(|w| window::change_mode(w, window::Mode::Fullscreen))
        } else {
            Task::none()
        }];
        // A custom site names its event in the URL, so the event list is not
        // needed — and would be answered by a third-party site that has no
        // reason to serve one. It adopts its own event instead, which is also
        // what brings its schedule and teams back after a restart.
        if new.source == GameSource::Custom {
            startup_tasks.push(new.adopt_custom_event());
        } else if new.uses_remote() {
            startup_tasks.push(new.request_event_list());
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
                let mut tm = self.tm.lock().unwrap();
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
                    let mut tm = self.tm.lock().unwrap();
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
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                tm.start_play_now(now).unwrap();
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
            Message::EditScores => {
                let tm = self.tm.lock().unwrap();
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
                    let mut tm = self.tm.lock().unwrap();
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
                let mut tm = self.tm.lock().unwrap();
                let mut now = Instant::now();

                self.app_state = if let AppState::ScoreEdit {
                    scores,
                    is_confirmation,
                } = self.app_state
                {
                    if is_confirmation {
                        tm.set_scores(scores, now);
                        tm.end_confirm_pause(now).unwrap();
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
                let snapshot = self
                    .tm
                    .lock()
                    .unwrap()
                    .generate_snapshot(Instant::now())
                    .unwrap();
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
                let snapshot = self
                    .tm
                    .lock()
                    .unwrap()
                    .generate_snapshot(Instant::now())
                    .unwrap();
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
                let snapshot = self
                    .tm
                    .lock()
                    .unwrap()
                    .generate_snapshot(Instant::now())
                    .unwrap();
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
                                .unwrap()
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
                                .unwrap()
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
                        // Safety: Mutex poison only occurs if another thread already panicked; the refbox treats that as fatal (matches the 20+ identical sites in this file).
                        if self.tm.lock().unwrap().current_period() != GamePeriod::BetweenGames {
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
                        let mut tm = self.tm.lock().unwrap();
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
                // to refresh AND a client to fetch it with; otherwise nothing
                // would arrive to clear the flag. The no-client case is real:
                // a degraded startup (see PortalManager::new_degraded) can
                // still have an event linked from a restored link note, and
                // `request_schedule` returns Task::none() with no client, so
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
                    // check against. Changing source resets it to FAILED, and
                    // until this line there was nothing to verify it with, so it
                    // would have sat on FAILED with a perfectly good saved token
                    // — keeping the court and game pickers greyed and the Game
                    // page's APPLY blocked for a reason that was an artifact.
                    task = Task::batch(vec![task, self.refresh_token_indicator()]);
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
                let edited_settings = self.edited_settings.as_mut().unwrap();
                let task = match param {
                    ListableParameter::Event => {
                        let id = EventId::from_full(val).unwrap();
                        // Set the new event id and clear court / game number / schedule
                        // that were filtered by the previous event so the user re-picks
                        // against the new event's data.
                        edited_settings.select_event(id.clone());

                        if let Some(ref client) = self.uwhportal_client {
                            // why this cannot panic: the guard is held only for a
                            // synchronous `has_token()` call and dropped immediately.
                            let has_token = client.lock().unwrap().has_token();
                            if has_token {
                                edited_settings.uwhportal_token_valid = None;
                            } else {
                                edited_settings.uwhportal_token_valid = Some(false);
                            }
                        } else {
                            edited_settings.uwhportal_token_valid = Some(false);
                        };

                        if let Some(pools) = self
                            .events
                            .as_ref()
                            .and_then(|events| events.get(&id).and_then(|e| e.courts.as_ref()))
                        {
                            if pools.len() == 1 {
                                if let Some(ref mut edits) = self.edited_settings {
                                    edits.current_court = Some(pools[0].clone());
                                }
                            }
                        }
                        Task::batch(vec![
                            self.check_uwhportal_auth(&id),
                            self.request_schedule(id),
                        ])
                    }
                    ListableParameter::Court => {
                        // Set the new court and clear the game number that was filtered
                        // by the previous court so the user re-picks from the new
                        // court's filtered list.
                        edited_settings.select_court(val);
                        Task::none()
                    }
                    ListableParameter::Game => {
                        edited_settings.game_number = val;
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
            Message::CustomSiteUrlChanged(url) => {
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

                let mut trigger_event_list_fetch = false;
                // Per ADR 017 (Portal Data Lifecycle): on manual -> remote,
                // start the pickers from a blank slate (see
                // `clear_for_remote_switch`) and kick off the event-list fetch
                // immediately so the picker has data ready when the operator
                // navigates to it.
                if !was_using && edited_settings.uses_remote() {
                    edited_settings.clear_for_remote_switch();
                    // Only the portal has a list to fetch. A custom site names
                    // its event in the URL and adopts it when the source is
                    // applied, so asking a third-party site for an event list
                    // would be a call it has no reason to answer.
                    trigger_event_list_fetch = new_source == GameSource::Portal;
                }
                if was_using && !edited_settings.uses_remote() {
                    // remote -> manual is a clean slate (reverses ADR 017's
                    // "no proactive clearing").
                    edited_settings.current_event_id = None;
                    edited_settings.current_court = None;
                    edited_settings.schedule = None;
                    edited_settings.game_number = String::new();
                }

                if trigger_event_list_fetch {
                    self.request_event_list()
                } else {
                    Task::none()
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
                        let needs_restart = font_family_id(original) != font_family_id(lang);
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
                // `ConfirmationKind::UwhPortalLinkFailed` (which offers GoBack)
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
                    let mut tm = self.tm.lock().unwrap();
                    let now = Instant::now();
                    // Safe: end_confirm_pause's only Err is NotPaused, which can't occur here —
                    // Message::ConfirmScores is only dispatched while a confirm-pause is active.
                    tm.end_confirm_pause(now).unwrap();
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
                        let mut tm = self.tm.lock().unwrap();

                        tm.set_scores(scores, now);
                        // Safe: end_confirm_pause's only Err is NotPaused, which can't occur here —
                        // Message::ScoreConfirmation is only dispatched while a confirm-pause is active.
                        tm.end_confirm_pause(now).unwrap();
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
                let mut tm = self.tm.lock().unwrap();
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
                let mut tm = self.tm.lock().unwrap();
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
                let mut tm = self.tm.lock().unwrap();
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
                let mut tm = self.tm.lock().unwrap();
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
                let mut tm = self.tm.lock().unwrap();
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
                for event in e_map.values() {
                    tasks.push(self.request_teams_list(event.id.clone()));
                }
                self.events = Some(e_map);
                // Startup link restore: now that the event list is populated,
                // fetch the schedule for the restored event so RecvSchedule can
                // re-select the remembered game and start its scheduled countdown.
                if let Some(event_id) = self.pending_restore_schedule.take() {
                    let in_list = self
                        .events
                        .as_ref()
                        .is_some_and(|m| m.contains_key(&event_id));
                    if in_list {
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
            Message::RecvTeamsList(event_id, teams) => {
                if let Some(ref mut events) = self.events {
                    if let Some(event) = events.get_mut(&event_id) {
                        event.teams = Some(teams);
                    } else {
                        error!(
                            "Received teams for event_id {}, it is not in the event list",
                            event_id.full()
                        );
                    }
                } else {
                    error!(
                        "Received teams for event_id {}, but there is no event list yet",
                        event_id.full()
                    );
                }
                Task::none()
            }
            Message::RecvTeamRoster(team_id, numbers) => {
                self.team_rosters.insert(team_id, numbers);
                Task::none()
            }
            Message::RecvSchedule(event_id, mut schedule) => {
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

                if let Some(ref mut events) = self.events {
                    if let Some(event) = events.get_mut(&event_id) {
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
                                    let mut tm = self.tm.lock().unwrap();
                                    if tm.current_period() == GamePeriod::BetweenGames {
                                        // On a startup link restore, re-select the
                                        // remembered game; otherwise pick the default
                                        // next game by number.
                                        let restore_num = self.pending_restore_game.take();
                                        let lookup_num = restore_num
                                            .clone()
                                            .unwrap_or_else(|| tm.next_game_number());
                                        if let (Some(game), Some(timing)) = self
                                            .schedule
                                            .as_ref()
                                            .unwrap()
                                            .get_game_and_timing(&lookup_num)
                                        {
                                            info!(
                                                "Setting upcoming game info from received schedule: {game:?}"
                                            );
                                            tm.set_next_game(NextGameInfo {
                                                number: game.number.clone(),
                                                timing: Some(timing.clone()),
                                                start_time: Some(game.start_time),
                                            });
                                            if restore_num.is_some() {
                                                // Start the live countdown to the
                                                // scheduled start so a restored session
                                                // is ready to go (same path the normal
                                                // between-games transition uses).
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
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        error!(
                            "Received schedule for event_id {}, it is not in the event list",
                            event_id.full()
                        );
                    }
                } else {
                    error!(
                        "Received schedule for event_id {}, but there is no event list yet",
                        event_id.full()
                    );
                }
                Task::batch(roster_tasks)
            }
            Message::RecvPortalToken(token_response) => {
                let mut task = Task::none();
                self.app_state = match token_response {
                    PortalTokenResponse::Success(token) => {
                        info!("Portal token request succeeded");
                        if let Some(client) = self.uwhportal_client.as_ref() {
                            // why this cannot panic: the guard is held only for a
                            // synchronous `set_token()` call and dropped immediately.
                            client.lock().unwrap().set_token(&token);
                        }
                        // Save it against the site it was issued by. Writing it
                        // to the portal's slot regardless would both destroy the
                        // operator's real Portal login and leave the custom site
                        // with no saved credential for the next launch.
                        match self.current_site.kind {
                            SiteKind::Portal => self.config.uwhportal.token = token,
                            SiteKind::Custom => self.config.custom_site.token = token,
                        }
                        if let Some(ref mut settings) = self.edited_settings {
                            settings.uwhportal_token_valid = Some(true);
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
                    r @ PortalTokenResponse::NoPendingLink
                    | r @ PortalTokenResponse::InvalidCode => {
                        warn!("Portal token request failed: {:?}", r);
                        AppState::ConfirmationPage(ConfirmationKind::UwhPortalLinkFailed(r))
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
                task
            }
            Message::RecvTokenValid(event_id, valid) => {
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
                self.tm.lock().unwrap().start_clock(Instant::now());
                Task::none()
            }
            Message::StopClock => {
                self.tm.lock().unwrap().stop_clock(Instant::now()).unwrap();
                Task::none()
            }
            Message::TimeUpdaterStarted(tx) => {
                let tm = self.tm.clone();
                tx.blocking_send(tm).unwrap();
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
                let mut tm = self.tm.lock().unwrap();
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
                    config: self.tm.lock().unwrap().config().clone(),
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
                    config: self.tm.lock().unwrap().config().clone(),
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
                    let needs_restart = font_family_id(original) != font_family_id(lang);
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
                    config: self.tm.lock().unwrap().config().clone(),
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
                    config: self.tm.lock().unwrap().config().clone(),
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
                            if level.count < 5 {
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
                // 10 seconds for all three presets, so only `levels` is staged.
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
                        if let Err(e) = crate::portal_manager::queue::save(
                            self.portal_manager.queue_dir(),
                            &crate::portal_manager::queue::QueueFile::empty(),
                        ) {
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
        // teams have already loaded into `self.events[new_id].teams` but whose
        // commit is still pending Apply. Resolve teams against the in-edit
        // event id (when present) so the picker shows real team names during edit;
        // fall back to the committed `current_event_id` outside of edit.
        let active_event_id = self
            .edited_settings
            .as_ref()
            .and_then(|edits| edits.current_event_id.as_ref())
            .or(self.current_event_id.as_ref());
        let data = ViewData {
            snapshot: &self.snapshot,
            mode: self.config.mode,
            source: self.source,
            clock_running: self.tm.lock().unwrap().clock_is_running(),
            teams: active_event_id.and_then(|id| {
                self.events
                    .as_ref()
                    .and_then(|events| events.get(id).and_then(|event| event.teams.as_ref()))
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
                        .unwrap()
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
                    self.tm.lock().unwrap().behind_schedule_shown(Instant::now())
                } else {
                    std::time::Duration::ZERO
                };
                build_main_view(
                    data,
                    game_config,
                    self.uses_remote(),
                    self.schedule.as_ref(),
                    self.config.track_fouls_and_warnings,
                    self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled,
                    self.mouse_alarm_held || self.spacebar_held,
                    behind_schedule,
                    self.tm
                        .lock()
                        .unwrap()
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
            AppState::KeypadPage(page, player_num) =>
                build_keypad_page(
                    data,
                    page,
                    player_num,
                    self.config.track_fouls_and_warnings,
                    self.edited_settings.as_ref().map(|e| e.game_number.clone()),
                    &self.game_rosters,
                ),
            AppState::GameDetailsPage(is_refreshing) => build_game_info_page(
                data,
                &self.config.game,
                self.uses_remote(),
                is_refreshing,
                self.schedule.as_ref(),
                self.tm
                    .lock()
                    .unwrap()
                    .last_game_info()
                    .map(|i| (i.game_number.clone(), i.scores)),
            ),
            AppState::WarningsSummaryPage => build_warnings_summary_page(data),
            AppState::PowerPage => build_power_page(data),
            AppState::EditGameConfig(page) => build_game_config_edit_page(
                data,
                self.edited_settings.as_ref().unwrap(),
                self.events.as_ref(),
                page,
                self.page_entry_snapshot.as_ref(),
                self.power_controls_visible(),
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
                self.events.as_ref(),
            ),
            AppState::ConfirmationPage(ref kind) => {
                build_confirmation_page(data, kind)
            }
            AppState::ConfirmScores(scores) =>
                build_score_confirmation_page(data, scores, self.snapshot.conf_pause_time),
            AppState::PortalDetailPage { scroll_index } =>
                build_portal_detail_page(data, self.portal_manager.detail_rows(), scroll_index,),
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
                    build_portal_detail_page(data, self.portal_manager.detail_rows(), 0)
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

/// Copy the six plain `Config` toggles owned by the App Options page out of the
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
/// `source`, `mode` and the portal selections are deliberately NOT handled here.
/// Each carries side effects that differ between the two paths — `commit_source`,
/// `set_current_event_id`, the portal-queue flush — and needs `&mut self`.
fn commit_app_toggles(config: &mut Config, edited: &EditableSettings) -> bool {
    config.collect_scorer_cap_num = edited.collect_scorer_cap_num;
    config.track_fouls_and_warnings = edited.track_fouls_and_warnings;
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

    /// All six toggles set to the opposite of their `Config` default, so a
    /// missing assignment cannot pass by coincidence.
    fn all_flipped() -> EditableSettings {
        EditableSettings {
            collect_scorer_cap_num: false,
            track_fouls_and_warnings: true,
            show_behind_schedule_time: false,
            confirm_score: false,
            audible_countdown: true,
            hide_time: true,
            ..Default::default()
        }
    }

    #[test]
    fn all_six_toggles_are_copied() {
        let mut config = Config::default();

        commit_app_toggles(&mut config, &all_flipped());

        assert!(!config.collect_scorer_cap_num);
        assert!(config.track_fouls_and_warnings);
        assert!(!config.show_behind_schedule_time);
        assert!(!config.confirm_score);
        assert!(config.audible_countdown);
        assert!(config.hide_time);
    }

    #[test]
    fn all_six_toggles_are_copied_the_other_way() {
        // Guards against a hardcoded assignment rather than a copy: the same six
        // fields driven back in the opposite direction. All six start `true` so
        // that committing an all-default (all-false) EditableSettings has to
        // change every one of them — starting from `all_flipped()` would leave
        // three already false, and their assertions would then pass even with
        // the assignment deleted.
        let mut config = Config::default();
        config.collect_scorer_cap_num = true;
        config.track_fouls_and_warnings = true;
        config.show_behind_schedule_time = true;
        config.confirm_score = true;
        config.audible_countdown = true;
        config.hide_time = true;
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
    fn only_the_six_toggles_are_written() {
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
        assert_eq!(target.base_url, "http://scoreboard.local:8099");
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
        assert_eq!(target.base_url, "https://scoreboard.example");
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
            assert!(!target.base_url.contains("scoreboard.local"));
        }
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
            game_number: Some("G1".into()),
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

fn font_family_id(lang: Language) -> u8 {
    match lang {
        Language::Korean | Language::Japanese | Language::Mandarin => 1,
        Language::Thai => 2,
        _ => 0,
    }
}

/// Fallback re-poll delay for the time updater when the clock is running but
/// the game state has no concrete next-update instant. This happens only in
/// degenerate zero-duration timing rules (e.g. the portal "FINALS" rule, whose
/// pre-overtime break is zero, producing a zero-length score-confirm pause).
/// Re-polling soon lets the state machine advance; it must never panic.
const UPDATER_NO_NEXT_TIME_FALLBACK: Duration = Duration::from_millis(100);

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

        msg_tx.try_send(Message::TimeUpdaterStarted(tx)).unwrap();

        let tm = rx.recv().await.unwrap();
        let mut clock_running_receiver = tm.lock().unwrap().get_start_stop_rx();
        let mut next_time = Some(Instant::now());

        loop {
            let mut clock_running = true;
            if let Some(next_time) = next_time {
                if next_time > Instant::now() {
                    match timeout_at(next_time, clock_running_receiver.changed()).await {
                        Err(_) => {}
                        Ok(Err(_)) => continue,
                        Ok(Ok(())) => {
                            clock_running = *clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                    };
                } else {
                    match clock_running_receiver.has_changed() {
                        Ok(true) => {
                            clock_running = *clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                        Ok(false) => {}
                        Err(_) => {
                            continue;
                        }
                    };
                }
            } else {
                debug!("Awaiting a new clock running message");
                match clock_running_receiver.changed().await {
                    Err(_) => continue,
                    Ok(()) => {
                        clock_running = *clock_running_receiver.borrow();
                        debug!("Received clock running message: {clock_running}");
                    }
                };
            };

            let (msg_type, snapshot) = {
                let mut tm_ = tm.lock().unwrap();
                let now = Instant::now();

                let msg_type = if tm_.could_end_game(now).unwrap() {
                    tm_.pause_for_confirm(now).unwrap();
                    Message::ConfirmScores
                } else if tm_.pause_has_ended(now) {
                    tm_.end_confirm_pause(now).unwrap();
                    Message::AutoConfirmScores
                } else {
                    tm_.update(now).unwrap();
                    Message::NewSnapshot
                };

                let mut i = 0;
                let snapshot = loop {
                    if i > 4 {
                        error!(
                            "Failed to generate snapshot after 5 attempts. State: {:#?}",
                            tm_
                        );
                        panic!("No snapshot");
                    }
                    match tm_.generate_snapshot(now) {
                        Some(val) => break val,
                        None => {
                            warn!("Failed to generate snapshot. Updating and trying again");
                            tm_.update(now).unwrap();
                            i += 1;
                        }
                    }
                };

                next_time = next_updater_wake(clock_running, tm_.next_update_time(now), now);

                (msg_type, snapshot)
            };

            msg_tx.send(msg_type(snapshot)).await.unwrap();
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
