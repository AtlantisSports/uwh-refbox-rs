use self::infraction::InfractionDetails;
use super::{APP_NAME, fl};
use crate::{
    config::{Config, Mode},
    penalty_editor::*,
    portal_manager::{ItemId, PortalEvent, PortalManager, SelectedEventId, UwhPortalIo},
    sound_controller::*,
    tournament_manager::{penalty::*, *},
};
use futures_lite::Stream;
use iced::{
    Element, Subscription, Task, Theme,
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
    sync::{Arc, Mutex},
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
        PortalTokenResponse, UwhPortalClient,
        schedule::{Event, EventId, GameNumber, Schedule},
    },
};

mod view_data;
use view_data::ViewData;

mod view_builders;
use view_builders::{shared_elements::portal_name_for_mode, *};

mod message;
use message::*;

pub mod theme;
use theme::*;

pub mod update_sender;
use update_sender::*;

pub(crate) mod languages;
use languages::*;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    config: Config,
    edited_settings: Option<EditableSettings>,
    page_entry_snapshot: Option<PageEntrySnapshot>,
    snapshot: GameSnapshot,
    pen_edit: ListEditor<Penalty, Color>,
    warn_edit: ListEditor<InfractionDetails, Color>,
    foul_edit: ListEditor<InfractionDetails, Option<Color>>,
    app_state: AppState,
    last_app_state: AppState,
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
    using_uwhportal: bool,
    events: Option<BTreeMap<EventId, Event>>,
    schedule: Option<Schedule>,
    current_event_id: Option<EventId>,
    current_court: Option<String>,
    sound: SoundController,
    sim_child: Option<Child>,
    list_all_events: bool,
    mouse_alarm_held: bool,
    spacebar_held: bool,
    alarm_delay_token: u64,
    portal_manager: PortalManager,
    /// Receiver half of the portal-manager background task's event
    /// channel. Wrapped in `Arc<Mutex<Option<_>>>` so an iced
    /// Subscription factory can clone the Arc, `.take()` the Receiver
    /// out once on its first activation, and drive the channel from the
    /// stream task without needing a `&mut` on `self` (which iced's
    /// `subscription(&self)` entry point cannot provide).
    portal_event_rx: Arc<Mutex<Option<mpsc::Receiver<PortalEvent>>>>,
    /// Set when the operator initiates portal re-login from the
    /// token-expired action page so a successful login returns them to
    /// the portal detail page instead of the default edit-config
    /// landing. Consumed (cleared) when the success handler reads it.
    /// Re-armed each time `PortalGoToLogin` fires, so an aborted login
    /// leaves the flag stale but harmless: the next successful login
    /// (from anywhere) would route to the detail page, which matches
    /// the operator's most recent intent.
    portal_login_return_to_detail: bool,
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
    pub sim_child: Option<Child>,
    pub require_https: bool,
    pub fullscreen: bool,
    pub list_all_events: bool,
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
    EditGameConfig(ConfigPage),
    ParameterEditor(LengthParameter, Duration),
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
    // Raised when the operator changes Mode across the portal boundary (Hockey ↔
    // Rugby). Carries the current and proposed modes so the confirmation page can
    // describe what will change. Real rendering and handler land in Tasks 7–8.
    #[allow(dead_code)] // construction site added in Task 7
    PortalTenantSwitch {
        from_mode: Mode,
        to_mode: Mode,
    },
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
        using_uwhportal: bool,
        current_event_id: Option<EventId>,
        current_court: Option<String>,
        schedule: Option<Schedule>,
    },
    App {
        using_uwhportal: bool,
        current_event_id: Option<EventId>,
        current_court: Option<String>,
        schedule: Option<Schedule>,
        mode: Mode,
        collect_scorer_cap_num: bool,
        track_fouls_and_warnings: bool,
        confirm_score: bool,
    },
    Display {
        white_on_right: bool,
        brightness: matrix_drawing::transmitted_data::Brightness,
        hide_time: bool,
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
                using_uwhportal,
                current_event_id,
                current_court,
                schedule,
            } => {
                edited.config = config;
                edited.game_number = game_number;
                edited.using_uwhportal = using_uwhportal;
                edited.current_event_id = current_event_id;
                edited.current_court = current_court;
                edited.schedule = schedule;
            }
            PageEntrySnapshot::App {
                using_uwhportal,
                current_event_id,
                current_court,
                schedule,
                mode,
                collect_scorer_cap_num,
                track_fouls_and_warnings,
                confirm_score,
            } => {
                edited.using_uwhportal = using_uwhportal;
                edited.current_event_id = current_event_id;
                edited.current_court = current_court;
                edited.schedule = schedule;
                edited.mode = mode;
                edited.collect_scorer_cap_num = collect_scorer_cap_num;
                edited.track_fouls_and_warnings = track_fouls_and_warnings;
                edited.confirm_score = confirm_score;
            }
            PageEntrySnapshot::Display {
                white_on_right,
                brightness,
                hide_time,
            } => {
                edited.white_on_right = white_on_right;
                edited.brightness = brightness;
                edited.hide_time = hide_time;
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
        }
    }
}

impl RefBoxApp {
    fn apply_snapshot(&mut self, mut new_snapshot: GameSnapshot) -> Task<Message> {
        let mut task = Task::none();
        if new_snapshot.current_period != self.snapshot.current_period {
            if new_snapshot.current_period == GamePeriod::BetweenGames {
                task = self.handle_game_end(&new_snapshot.game_number);
            } else if self.snapshot.current_period == GamePeriod::BetweenGames {
                self.handle_game_start(&new_snapshot.game_number);
            }
        }

        new_snapshot.event_id = self.current_event_id.clone();

        self.maybe_play_sound(&new_snapshot);
        self.update_sender
            .send_snapshot(
                new_snapshot.clone(),
                self.config.hardware.white_on_right,
                self.config.hardware.brightness,
            )
            .unwrap();
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

        if play_whistle {
            info!("Triggering whistle");
            self.sound.trigger_whistle();
        } else if play_buzzer {
            info!("Triggering buzzer");
            self.sound.trigger_buzzer();
        }
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

    fn check_uwhportal_auth(&self, event_id: &EventId) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // why this cannot panic: see `request_event_list` above.
            let request = client.lock().unwrap().verify_token(event_id);
            Task::future(async move {
                match request.await {
                    Ok(()) => {
                        info!("Portal token validated");
                        Message::RecvTokenValid(true)
                    }
                    Err(e) => {
                        error!("Portal token validity check failed: {e}");
                        Message::RecvTokenValid(false)
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    fn handle_game_start(&mut self, new_game_num: &GameNumber) {
        if self.using_uwhportal {
            debug!("Searching for next game info after game {new_game_num}");
            if let (Some(schedule), Some(pool)) = (&self.schedule, &self.current_court) {
                let this_game_start = match schedule.games.get(new_game_num) {
                    Some(g) => g.start_time,
                    None => {
                        error!("Could not find new game's start time (game {new_game_num}");
                        return;
                    }
                };

                let next_game = schedule
                    .games
                    .values()
                    .filter(|game| game.court == *pool)
                    .filter(|game| game.start_time > this_game_start)
                    .min_by_key(|game| game.start_time);

                let mut tm = self.tm.lock().unwrap();
                if let Some(next_game) = next_game {
                    let timing = schedule.get_game_timing(&next_game.number).cloned();
                    let info = NextGameInfo {
                        number: next_game.number.clone(),
                        timing,
                        start_time: Some(next_game.start_time),
                    };
                    tm.set_next_game(info);
                } else {
                    error!("Couldn't find a next game");
                }
                self.config.game = tm.config().clone();
            }
        } else {
            debug!("Skipped next game info search after game {new_game_num}");
        }
    }

    fn handle_game_end(&mut self, game_number: &GameNumber) -> Task<Message> {
        let mut tasks = vec![];
        if self.using_uwhportal {
            if let Some(info) = self.tm.lock().unwrap().last_game_info() {
                let stats = info.stats.as_json();
                let black_score = info.scores.black;
                let white_score = info.scores.white;

                info!(
                    "Game ended, scores: {:?} stats were: {:?}",
                    info.scores, stats
                );

                if let Some(ref event_id) = self.current_event_id {
                    let event_id_str = event_id.full().to_string();
                    tasks.push(self.request_schedule(event_id.clone()));
                    if let Err(e) = self.portal_manager.enqueue_game_end(
                        event_id_str,
                        game_number.to_string(),
                        black_score,
                        white_score,
                        stats,
                    ) {
                        error!("portal_manager.enqueue_game_end failed: {e}");
                    }
                } else {
                    error!("Missing current event id to handle game end");
                }
            } else {
                warn!("Game ended, but no last game info was available");
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

    fn apply_app_options(&mut self) {
        let Some(edited) = self.edited_settings.as_ref() else {
            return;
        };
        // Snapshot the fields we need so the immutable borrow on
        // `edited_settings` ends before we call `set_current_event_id`
        // (which takes `&mut self`).
        let using_uwhportal = edited.using_uwhportal;
        let event_id = edited.current_event_id.clone();
        let current_court = edited.current_court.clone();
        let schedule = edited.schedule.clone();
        let mode = edited.mode;
        let collect_scorer_cap_num = edited.collect_scorer_cap_num;
        let track_fouls_and_warnings = edited.track_fouls_and_warnings;
        let confirm_score = edited.confirm_score;

        self.using_uwhportal = using_uwhportal;
        // Route through set_current_event_id so portal_event_id stays in
        // sync (ADR 011 amendment 2026-04-23 dormant-until-linked).
        self.set_current_event_id(event_id);
        self.current_court = current_court;
        self.schedule = schedule;
        self.config.mode = mode;
        self.config.collect_scorer_cap_num = collect_scorer_cap_num;
        self.config.track_fouls_and_warnings = track_fouls_and_warnings;
        self.config.confirm_score = confirm_score;
    }

    fn apply_display_options(&mut self) {
        let Some(edited) = self.edited_settings.as_ref() else {
            return;
        };
        self.config.hardware.white_on_right = edited.white_on_right;
        self.config.hardware.brightness = edited.brightness;
        if self.config.hide_time != edited.hide_time {
            self.config.hide_time = edited.hide_time;
            self.update_sender
                .set_hide_time(self.config.hide_time)
                .unwrap();
        }
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

        let new_config = if edited.using_uwhportal {
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

            if edited.using_uwhportal {
                // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
                tm.apply_next_game_start(Instant::now()).unwrap();
            } else {
                tm.clear_scheduled_game_start();
            }

            std::mem::drop(tm);
            // Snapshot the fields we need so the immutable borrow on
            // `edited` ends before we call `set_current_event_id`
            // (which takes `&mut self`).
            let using_uwhportal = edited.using_uwhportal;
            let event_id = edited.current_event_id.clone();
            let current_court = edited.current_court.clone();
            let schedule = edited.schedule.clone();

            self.config.game = new_config;
            self.using_uwhportal = using_uwhportal;
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
            let next_game_info = if edited.using_uwhportal {
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

            if edited.using_uwhportal {
                // Safety: precondition checked above (period != BetweenGames / next-game info just set); error path is unreachable in this control flow.
                tm.apply_next_game_start(Instant::now()).unwrap();
            }
        }

        std::mem::drop(tm);
        self.using_uwhportal = edited.using_uwhportal;
        self.current_event_id = edited.current_event_id.clone();
        self.current_court = edited.current_court.clone();
        self.schedule = edited.schedule.clone();

        None
    }

    /// Handle the user's selection on a `*FromApply` ConfirmationPage. Mirrors the
    /// logic of `Message::ConfirmationSelected` for the global-Done variants, but
    /// commits only the Game slice and routes back into settings (not out to MainPage).
    fn apply_game_confirmation(&mut self, selection: ConfirmationOption) -> Task<Message> {
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
                self.revert_from_snapshot();
                AppState::EditGameConfig(ConfigPage::Main)
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

                if edited.using_uwhportal {
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
                // Extract the proposed mode from the in-flight PortalTenantSwitch state.
                // This arm is only reachable when the app_state is PortalTenantSwitch
                // (the view builder only offers RestartAndApply for that kind).
                let to_mode =
                    if let AppState::ConfirmationPage(ConfirmationKind::PortalTenantSwitch {
                        to_mode,
                        ..
                    }) = self.app_state
                    {
                        to_mode
                    } else {
                        unreachable!("RestartAndApply is only offered by PortalTenantSwitch pages")
                    };

                // Commit the new mode. The proposed mode was held only in the
                // ConfirmationKind variant and was never written to self.config, so
                // this is the first and only write.
                self.config.mode = to_mode;

                // Clear the current event id. This unpins the portal-health background
                // task from the old tenant's event so it stops probing after restart.
                self.set_current_event_id(None);

                // Flush the portal retry queue. Items queued under the old portal
                // tenant cannot be delivered to the new tenant — discard them so the
                // restarted app starts with a clean queue.
                if let Err(e) = crate::portal_manager::queue::save(
                    self.portal_manager.queue_dir(),
                    &crate::portal_manager::queue::QueueFile::empty(),
                ) {
                    error!("Failed to flush portal queue before restart: {e}");
                    // Continue with restart — the operator pressed Restart and we
                    // must not block. The queue will be treated as stale items for
                    // the new tenant, which the retry logic will eventually discard.
                }

                // Persist the new mode to disk so the restarted exe reads it.
                if let Err(e) = confy::store(APP_NAME, None, &self.config) {
                    error!("Failed to persist config before restart: {e}");
                    // Continue with restart anyway — the operator pressed Restart.
                }

                // Restart pattern mirrored from the existing language-switch path
                // (see Message::LanguageSelectComplete). Kill the simulator child
                // first so it does not linger as an orphan.
                if let Some(mut child) = self.sim_child.take() {
                    let _ = child.kill();
                }
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                std::process::exit(0);
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
            ConfigPage::Game => PageEntrySnapshot::Game {
                config: edited.config.clone(),
                game_number: edited.game_number.clone(),
                using_uwhportal: edited.using_uwhportal,
                current_event_id: edited.current_event_id.clone(),
                current_court: edited.current_court.clone(),
                schedule: edited.schedule.clone(),
            },
            ConfigPage::App => PageEntrySnapshot::App {
                using_uwhportal: edited.using_uwhportal,
                current_event_id: edited.current_event_id.clone(),
                current_court: edited.current_court.clone(),
                schedule: edited.schedule.clone(),
                mode: edited.mode,
                collect_scorer_cap_num: edited.collect_scorer_cap_num,
                track_fouls_and_warnings: edited.track_fouls_and_warnings,
                confirm_score: edited.confirm_score,
            },
            ConfigPage::Display => PageEntrySnapshot::Display {
                white_on_right: edited.white_on_right,
                brightness: edited.brightness,
                hide_time: edited.hide_time,
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
        confy::store(APP_NAME, None, &self.config).unwrap();
    }

    fn navigate_to_parent(&mut self, page: ConfigPage) {
        let parent = match page {
            ConfigPage::Game | ConfigPage::App | ConfigPage::User | ConfigPage::Language => {
                ConfigPage::Main
            }
            ConfigPage::Display | ConfigPage::Sound => ConfigPage::User,
            ConfigPage::Remotes(_, _) => ConfigPage::Sound,
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
        if let Some(mut child) = self.sim_child.take() {
            info!("Waiting for child");
            child.wait().unwrap();
        }
    }
}

impl RefBoxApp {
    pub(super) fn new(flags: RefBoxAppFlags) -> (Self, Task<Message>) {
        let RefBoxAppFlags {
            config,
            config_dir,
            serial_ports,
            binary_port,
            json_port,
            sim_child,
            require_https,
            fullscreen,
            list_all_events,
        } = flags;

        let mut tm = TournamentManager::new(config.game.clone());
        tm.start_clock(Instant::now());

        let portal_token = if !config.uwhportal.token.is_empty() {
            Some(config.uwhportal.token.as_str())
        } else {
            None
        };
        let (default_url, override_var) = match config.mode {
            Mode::Rugby => ("https://api.uwrportal.com", "UWR_PORTAL_URL_OVERRIDE"),
            Mode::Hockey6V6 | Mode::Hockey3V3 => {
                ("https://api.uwhportal.com", "UWH_PORTAL_URL_OVERRIDE")
            }
        };
        let url_override = std::env::var(override_var).ok();
        let portal_url = url_override.as_deref().unwrap_or(default_url).to_string();
        if url_override.is_some() {
            let portal_name = match config.mode {
                Mode::Rugby => "UWR",
                Mode::Hockey6V6 | Mode::Hockey3V3 => "UWH",
            };
            info!("{override_var} active for {portal_name} Portal: using {portal_url}");
        }
        #[cfg(debug_assertions)]
        let scramble_token_pending = std::env::var("UWH_PORTAL_SCRAMBLE_TOKEN").is_ok();
        #[cfg(debug_assertions)]
        if scramble_token_pending {
            warn!(
                "UWH_PORTAL_SCRAMBLE_TOKEN armed: in-memory token will be invalidated after first event link"
            );
        }
        let uwhportal_client =
            match UwhPortalClient::new(&portal_url, portal_token, require_https, REQUEST_TIMEOUT) {
                Ok(c) => Some(Arc::new(Mutex::new(c))),
                Err(e) => {
                    error!(
                        "Failed to start {} Portal Client: {e}",
                        portal_name_for_mode(config.mode)
                    );
                    None
                }
            };

        // Shared event id the background portal task consults for its
        // periodic `verify_token` check. Mirrors `current_event_id` on
        // `RefBoxApp`; both start `None` here and are kept in sync via
        // `set_current_event_id` on every subsequent write.
        let portal_event_id: SelectedEventId = Arc::new(Mutex::new(None));

        let tm = Arc::new(Mutex::new(tm));

        let update_sender =
            UpdateSender::new(serial_ports, binary_port, json_port, config.hide_time);

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
        // The production `UwhPortalIo` is built from a clone of the
        // shared `UwhPortalClient` handle so that token mutations on the
        // UI thread are immediately visible to the background retry task.
        // If the client failed to construct (only possible on a bad
        // https-only config), fall back to `NullIo` for the non-degraded
        // path — no portal calls will be made, but the queue can still
        // accept items and persist them for a later attempt.
        std::fs::create_dir_all(&config_dir).ok();

        // Try `config_dir` first, then `std::env::temp_dir()`, then fall
        // back to degraded mode. Each attempt gets its own freshly-built
        // `PortalTaskIo`: either the real `UwhPortalIo` (if we have a
        // portal client) or `NullIo` (if client construction failed
        // earlier). The I/O is generic in the `PortalTaskIo` impl so the
        // retry helper is a closure that owns the attempt.
        let try_new_manager = |dir: &std::path::Path| match &uwhportal_client {
            Some(client) => PortalManager::new(
                dir,
                UwhPortalIo::new(Arc::clone(client), Arc::clone(&portal_event_id)),
            ),
            None => PortalManager::new(dir, crate::portal_manager::NullIo),
        };

        let (portal_manager, portal_event_rx) = match try_new_manager(&config_dir) {
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
        };
        let portal_event_rx = Arc::new(Mutex::new(Some(portal_event_rx)));

        let new = Self {
            pen_edit: ListEditor::new(tm.clone()),
            warn_edit: ListEditor::new(tm.clone()),
            foul_edit: ListEditor::new(tm.clone()),
            tm,
            config,
            edited_settings: Default::default(),
            page_entry_snapshot: None,
            snapshot,
            app_state: AppState::MainPage,
            last_app_state: AppState::MainPage,
            last_message: Message::NoAction,
            update_sender,
            uwhportal_client,
            portal_event_id,
            using_uwhportal: false,
            events: None,
            schedule: None,
            current_event_id: None,
            current_court: None,
            sound,
            sim_child,
            list_all_events,
            mouse_alarm_held: false,
            spacebar_held: false,
            alarm_delay_token: 0,
            portal_manager,
            portal_event_rx,
            portal_login_return_to_detail: false,
            #[cfg(debug_assertions)]
            scramble_token_pending,
        };

        let task = Task::batch(vec![
            new.request_event_list(),
            if fullscreen {
                window::get_latest().and_then(|w| window::change_mode(w, window::Mode::Fullscreen))
            } else {
                Task::none()
            },
        ]);

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
                self.last_app_state = self.app_state.clone();
                self.app_state = AppState::TimeEdit(
                    was_running,
                    tm.game_clock_time(now).unwrap(),
                    tm.timeout_clock_time(now),
                );
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
                    AppState::ParameterEditor(_, ref mut dur) => (dur, false),
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
                    self.app_state = AppState::KeypadPage(KeypadPage::AddScore(color), 0);
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
                    KeypadPage::AddScore(_)
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
                    KeypadPage::TeamTimeouts(_, _) => {
                        self.config.game.num_team_timeouts_allowed as u32
                    }
                    KeypadPage::GameNumber => self
                        .edited_settings
                        .as_ref()
                        .unwrap()
                        .game_number
                        .parse()
                        .unwrap_or(0),
                    KeypadPage::PortalLogin(ref mut id, _) => {
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
            Message::ChangeColor(new_color) => {
                match self.app_state {
                    AppState::KeypadPage(KeypadPage::AddScore(ref mut color), _)
                    | AppState::KeypadPage(KeypadPage::Penalty(_, ref mut color, _, _), _)
                    | AppState::KeypadPage(KeypadPage::WarningAdd { ref mut color, .. }, _) => {
                        *color = new_color.expect("Invalid color value");
                    }
                    AppState::KeypadPage(KeypadPage::FoulAdd { ref mut color, .. }, _) => {
                        *color = new_color;
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
                    if let AppState::KeypadPage(KeypadPage::AddScore(color), player) =
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
                    PortalEvent::HealthChanged | PortalEvent::ItemUpdated => {
                        self.portal_manager.ui_tick();
                    }
                    PortalEvent::TokenStatus(valid) => {
                        self.portal_manager.on_token_status(valid);
                    }
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
                } else {
                    // Young pending row tapped — force an immediate retry.
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
            Message::PortalGoToLogin => {
                // Navigate to the existing portal login keypad, arming
                // the return-to-detail flag so a successful re-login
                // lands back on the detail page (mirroring the
                // `UwhPortalLinkFailed` GoBack handler, which reuses
                // the same client id-lookup pattern).
                let portal_id = match self.uwhportal_client.as_ref() {
                    Some(client) => {
                        // why this cannot panic: the guard is held only
                        // for a synchronous `id()` call and dropped
                        // immediately.
                        client.lock().unwrap().id()
                    }
                    None => {
                        // No portal client configured — there is
                        // nothing to log into. Fall back to the detail
                        // page so the operator is not stranded on an
                        // empty login screen.
                        warn!("PortalGoToLogin with no portal client configured");
                        self.app_state = AppState::PortalDetailPage { scroll_index: 0 };
                        trace!("AppState changed to {:?}", self.app_state);
                        return Task::none();
                    }
                };
                self.portal_login_return_to_detail = true;
                self.app_state = AppState::KeypadPage(KeypadPage::PortalLogin(portal_id, false), 0);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::RequestPortalRefresh => {
                if let AppState::GameDetailsPage(ref mut is_refreshing) = self.app_state {
                    *is_refreshing = true;
                }
                if let Some(ref event_id) = self.current_event_id {
                    self.request_schedule(event_id.clone())
                } else {
                    Task::none()
                }
            }
            Message::ShowWarnings => {
                self.app_state = AppState::WarningsSummaryPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::EditGameConfig => {
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
                    using_uwhportal: self.using_uwhportal,
                    uwhportal_token_valid,
                    current_event_id: self.current_event_id.clone(),
                    current_court: self.current_court.clone(),
                    schedule: self.schedule.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
                    confirm_score: self.config.confirm_score,
                    pending_language: None,
                    original_language: None,
                };

                self.edited_settings = Some(edited_settings);

                self.app_state = AppState::EditGameConfig(ConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                task
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
                match page {
                    ConfigPage::App => self.apply_app_options(),
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
                }
                self.page_entry_snapshot = None;
                self.persist_config();
                self.navigate_to_parent(page);
                Task::none()
            }
            Message::CancelConfigPage(page) => {
                self.revert_from_snapshot();
                self.navigate_to_parent(page);
                Task::none()
            }
            Message::ConfigEditComplete => {
                // Per-page Apply/Cancel chrome is the only commit path after ADR 009
                // Tasks 8-13. ConfigEditComplete only fires `canceled: true` now (from
                // the Settings Main back button and other escape paths); it just exits
                // settings to MainPage and drops the in-flight edit buffer.
                self.edited_settings = None;
                self.app_state = AppState::MainPage;
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::EditParameter(param) => {
                self.app_state = AppState::ParameterEditor(
                    param,
                    match param {
                        LengthParameter::Half => self.config.game.half_play_duration,
                        LengthParameter::HalfTime => self.config.game.half_time_duration,
                        LengthParameter::NominalBetweenGame => self.config.game.nominal_break,
                        LengthParameter::MinimumBetweenGame => self.config.game.minimum_break,
                        LengthParameter::PreOvertime => self.config.game.pre_overtime_break,
                        LengthParameter::OvertimeHalf => self.config.game.ot_half_play_duration,
                        LengthParameter::OvertimeHalfTime => self.config.game.ot_half_time_duration,
                        LengthParameter::PreSuddenDeath => {
                            self.config.game.pre_sudden_death_duration
                        }
                    },
                );
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::SelectParameter(param) => {
                let index = match param {
                    ListableParameter::Event => {
                        self.current_event_id.as_ref().and_then(|cur_event_id| {
                            self.events
                                .as_ref()?
                                .iter()
                                .enumerate()
                                .find(|(_, (event_id, _))| **event_id == *cur_event_id)
                                .map(|(i, _)| i)
                        })
                    }
                    ListableParameter::Court => self.current_court.as_ref().and_then(|cur_court| {
                        self.events
                            .as_ref()?
                            .get(self.current_event_id.as_ref()?)?
                            .courts
                            .as_ref()?
                            .iter()
                            .enumerate()
                            .find(|(_, court)| **court == *cur_court)
                            .map(|(i, _)| i)
                    }),
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
                        AppState::ParameterEditor(param, dur) => {
                            let edited_settings = self.edited_settings.as_mut().unwrap();
                            match param {
                                LengthParameter::Half => {
                                    edited_settings.config.half_play_duration = dur
                                }
                                LengthParameter::HalfTime => {
                                    edited_settings.config.half_time_duration = dur
                                }
                                LengthParameter::NominalBetweenGame => {
                                    edited_settings.config.nominal_break = dur
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
                    AppState::ParameterEditor(_, _) => AppState::EditGameConfig(ConfigPage::Game),
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

                    _ => {
                        let edited_settings = self.edited_settings.as_mut().unwrap();
                        match param {
                            BoolGameParameter::OvertimeAllowed => {
                                edited_settings.config.overtime_allowed ^= true
                            }
                            BoolGameParameter::SuddenDeathAllowed => {
                                edited_settings.config.sudden_death_allowed ^= true
                            }
                            BoolGameParameter::SingleHalf => {
                                edited_settings.config.single_half ^= true
                            }
                            BoolGameParameter::WhiteOnRight => {
                                edited_settings.white_on_right ^= true
                            }
                            BoolGameParameter::UsingUwhPortal => {
                                edited_settings.using_uwhportal ^= true
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
                            BoolGameParameter::TeamWarning
                            | BoolGameParameter::TimeoutsCountedPerHalf => {
                                unreachable!()
                            }
                            BoolGameParameter::ConfirmScore => {
                                edited_settings.confirm_score ^= true
                            }
                            BoolGameParameter::ManualAlarmEnabled => {
                                edited_settings.sound.manual_alarm_enabled ^= true
                            }
                        }
                    }
                };
                Task::none()
            }
            Message::CycleParameter(param) => {
                let settings = &mut self.edited_settings.as_mut().unwrap();
                match param {
                    CyclingParameter::BuzzerSound => settings.sound.buzzer_sound.cycle(),
                    CyclingParameter::RemoteBuzzerSound(idx) => {
                        settings.sound.remotes[idx].sound.cycle()
                    }
                    CyclingParameter::AlertVolume => settings.sound.whistle_vol.cycle(),
                    CyclingParameter::AboveWaterVol => settings.sound.above_water_vol.cycle(),
                    CyclingParameter::UnderWaterVol => settings.sound.under_water_vol.cycle(),
                    CyclingParameter::Mode => settings.mode.cycle(),
                    CyclingParameter::Brightness => settings.brightness.cycle(),
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
                        confy::store(crate::APP_NAME, None, &self.config).unwrap();
                        if needs_restart {
                            // Kill the simulator child before exiting so it doesn't linger.
                            if let Some(mut child) = self.sim_child.take() {
                                let _ = child.kill();
                            }
                            // Spawn a fresh copy of the app — it will read the saved language from
                            // config and start with the correct default font.
                            if let Ok(exe) = std::env::current_exe() {
                                let _ = std::process::Command::new(exe).spawn();
                            }
                            std::process::exit(0);
                        }
                        // Apply the new language to the running UI (same font family, no restart needed).
                        crate::request_language(&crate::LANGUAGE_LOADER, &[lang.as_lang_id()]);
                    }
                }
                settings.pending_language = None;
                settings.original_language = None;
                if let AppState::EditGameConfig(ref mut page) = self.app_state {
                    *page = ConfigPage::Main;
                }
                trace!("AppState changed to {:?}", self.app_state);
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
                if matches!(
                    self.app_state,
                    AppState::ConfirmationPage(
                        ConfirmationKind::GameConfigChangedFromApply(_)
                            | ConfirmationKind::GameNumberChangedFromApply
                            | ConfirmationKind::UwhPortalIncompleteFromApply
                            | ConfirmationKind::PortalTenantSwitch { .. }
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
                    tm.halt_clock(now, true).unwrap();
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
            Message::RecvEventList(e_list) => {
                let mut tasks = vec![];
                let e_map: BTreeMap<_, _> = e_list.into_iter().map(|e| (e.id.clone(), e)).collect();
                for event in e_map.values() {
                    tasks.push(self.request_teams_list(event.id.clone()));
                }
                self.events = Some(e_map);
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
            Message::RecvSchedule(event_id, mut schedule) => {
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

                let mut courts = BTreeSet::new();
                for game in schedule.games.values() {
                    if !courts.contains(&game.court) {
                        courts.insert(game.court.clone());
                    }
                }
                let courts: Vec<_> = courts.into_iter().collect();

                if courts.len() == 1 {
                    if let Some(ref mut edits) = self.edited_settings {
                        if edits.current_court.is_none() {
                            edits.current_court = Some(courts[0].clone());
                        }
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
                                        if let (Some(game), Some(timing)) = self
                                            .schedule
                                            .as_ref()
                                            .unwrap()
                                            .get_game_and_timing(&tm.next_game_number())
                                        {
                                            info!(
                                                "Setting upcoming game info from received schedule: {game:?}"
                                            );
                                            tm.set_next_game(NextGameInfo {
                                                number: game.number.clone(),
                                                timing: Some(timing.clone()),
                                                start_time: Some(game.start_time),
                                            });
                                            if let AppState::GameDetailsPage(
                                                ref mut is_refreshing,
                                            ) = self.app_state
                                            {
                                                *is_refreshing = false;
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
                Task::none()
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
                        self.config.uwhportal.token = token;
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

                        // If the re-login was initiated from the
                        // token-expired action page, return to the
                        // portal detail page (where the operator left
                        // off); otherwise land on the default
                        // edit-config Game page. The flag is consumed
                        // here so it does not leak into later logins.
                        if self.portal_login_return_to_detail {
                            self.portal_login_return_to_detail = false;
                            AppState::PortalDetailPage { scroll_index: 0 }
                        } else {
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
            Message::RecvTokenValid(valid) => {
                if let Some(ref mut settings) = self.edited_settings {
                    settings.uwhportal_token_valid = Some(valid);
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
            Message::NoAction => Task::none(),
        }
    }

    pub(super) fn view(&self) -> Element<'_, Message> {
        let data = ViewData {
            snapshot: &self.snapshot,
            mode: self.config.mode,
            clock_running: self.tm.lock().unwrap().clock_is_running(),
            teams: self.current_event_id.as_ref().and_then(|id| {
                self.events
                    .as_ref()
                    .and_then(|events| events.get(id).and_then(|event| event.teams.as_ref()))
            }),
            // The portal health indicator is dormant until an event is
            // linked: `Some` when the tile and its state are live,
            // `None` when the time banner falls back to the pre-feature
            // layout. See ADR 011 amendment 2026-04-23.
            portal_indicator: self
                .current_event_id
                .as_ref()
                .map(|_| self.portal_manager.indicator_state()),
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
                build_main_view(
                    data,
                    game_config,
                    self.using_uwhportal,
                    self.schedule.as_ref(),
                    self.config.track_fouls_and_warnings,
                    self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled,
                    self.mouse_alarm_held || self.spacebar_held,
                )
            }
            AppState::TimeEdit(_, time, timeout_time) =>
                build_time_edit_view(data, time, timeout_time),
            AppState::ScoreEdit {
                scores,
                is_confirmation,
            } =>
                build_score_edit_view(data, scores, is_confirmation, self.snapshot.conf_pause_time),
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
                build_keypad_page(data, page, player_num, self.config.track_fouls_and_warnings),
            AppState::GameDetailsPage(is_refreshing) => build_game_info_page(
                data,
                &self.config.game,
                self.using_uwhportal,
                is_refreshing,
                self.schedule.as_ref()
            ),
            AppState::WarningsSummaryPage => build_warnings_summary_page(data),
            AppState::EditGameConfig(page) => build_game_config_edit_page(
                data,
                self.edited_settings.as_ref().unwrap(),
                self.events.as_ref(),
                page,
                self.page_entry_snapshot.as_ref(),
            ),
            AppState::ParameterEditor(param, dur) => build_game_parameter_editor(data, param, dur),
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
        }]
        .spacing(SPACING)
        .padding(PADDING);

        match self.app_state {
            AppState::ScoreEdit {
                is_confirmation, ..
            } if is_confirmation => {}
            AppState::ConfirmScores(_) => {}
            _ => {
                main_view = main_view.push(build_timeout_ribbon(
                    &self.snapshot,
                    &self.tm,
                    self.config.mode,
                ));
            }
        }

        main_view.into()
    }

    pub(super) fn subscription(&self) -> Subscription<Message> {
        let time_sub = Subscription::run(time_updater);

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
            Subscription::batch([
                time_sub,
                portal_events,
                portal_tick,
                key_press,
                key_release,
                mouse_release,
            ])
        } else {
            Subscription::batch([time_sub, portal_events, portal_tick])
        }
    }

    pub fn application_style(&self, _theme: &Theme) -> Appearance {
        Appearance {
            background_color: WINDOW_BACKGROUND,
            text_color: BLACK,
        }
    }
}

fn font_family_id(lang: Language) -> u8 {
    match lang {
        Language::Korean | Language::Japanese | Language::Mandarin => 1,
        Language::Thai => 2,
        _ => 0,
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

                next_time = if clock_running {
                    Some(tm_.next_update_time(now).unwrap())
                } else {
                    None
                };

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
