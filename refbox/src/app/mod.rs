use self::infraction::InfractionDetails;
use super::{APP_NAME, fl};
use crate::{
    config::{Config, Mode},
    penalty_editor::*,
    sound_controller::*,
    tournament_manager::{penalty::*, *},
};
use futures_lite::Stream;
use iced::{Element, Subscription, Task, Theme, application::Appearance, widget::column, window};
use log::*;
use std::{
    cmp::min,
    collections::{BTreeMap, BTreeSet},
    process::Child,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant, timeout_at},
};
use tokio_serial::SerialPortBuilder;
use uwh_common::{
    bundles::*,
    color::Color,
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{GamePeriod, GameSnapshot, Infraction, TimeoutSnapshot},
    uwhportal::{
        UwhPortalClient,
        schedule::{Event, EventId, Schedule},
    },
};

mod view_data;
use view_data::ViewData;

mod view_builders;
use view_builders::*;

mod message;
use message::*;

pub mod theme;
use theme::*;

pub mod update_sender;
use update_sender::*;

mod languages;
use languages::*;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    config: Config,
    edited_settings: Option<EditableSettings>,
    snapshot: GameSnapshot,
    pen_edit: ListEditor<Penalty, Color>,
    warn_edit: ListEditor<InfractionDetails, Color>,
    foul_edit: ListEditor<InfractionDetails, Option<Color>>,
    app_state: AppState,
    last_app_state: AppState,
    last_message: Message,
    update_sender: UpdateSender,
    uwhportal_client: Option<UwhPortalClient>,
    using_uwhportal: bool,
    events: Option<BTreeMap<EventId, Event>>,
    schedule: Option<Schedule>,
    current_event_id: Option<EventId>,
    current_court: Option<String>,
    sound: SoundController,
    sim_child: Option<Child>,
    list_all_events: bool,
    touchscreen: bool,
}

#[derive(Debug)]
pub struct RefBoxAppFlags {
    pub config: Config,
    pub serial_ports: Vec<SerialPortBuilder>,
    pub binary_port: u16,
    pub json_port: u16,
    pub sim_child: Option<Child>,
    pub require_https: bool,
    pub fullscreen: bool,
    pub list_all_events: bool,
    pub touchscreen: bool,
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
    KeypadPage(KeypadPage, u16),
    GameDetailsPage(bool),
    WarningsSummaryPage,
    EditGameConfig(ConfigPage),
    ParameterEditor(LengthParameter, Duration),
    ParameterList(ListableParameter, usize),
    ConfirmationPage(ConfirmationKind),
    ConfirmScores(BlackWhiteBundle<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfirmationKind {
    GameNumberChanged,
    GameConfigChanged(GameConfig),
    Error(String),
    UwhPortalIncomplete,
}

impl RefBoxApp {
    fn apply_snapshot(&mut self, mut new_snapshot: GameSnapshot) -> Task<Message> {
        let mut task = Task::none();
        if new_snapshot.current_period != self.snapshot.current_period {
            if new_snapshot.current_period == GamePeriod::BetweenGames {
                task = self.handle_game_end(new_snapshot.game_number);
            } else if self.snapshot.current_period == GamePeriod::BetweenGames {
                self.handle_game_start(new_snapshot.game_number);
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
        if let Some(ref client) = self.uwhportal_client {
            let request = client.get_event_list(self.list_all_events);
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

    fn request_teams_list(&self, event_id: EventId, event_slug: &str) -> Task<Message> {
        if let Some(ref client) = self.uwhportal_client {
            let request = client.get_event_schedule(event_slug);
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
        if let Some(ref client) = self.uwhportal_client {
            let request = client.get_event_schedule_privileged(&event_id);
            Task::future(async move {
                match request.await {
                    Ok(schedule) => {
                        info!("Got schedule");
                        Message::RecvSchedule(event_id, schedule)
                    }
                    Err(e) => {
                        error!("Failed to get schedule: {e}");
                        Message::NoAction
                    }
                }
            })
        } else {
            Task::none()
        }
    }

    fn post_game_score(
        &self,
        event_id: &EventId,
        game_number: u32,
        scores: BlackWhiteBundle<u8>,
    ) -> Task<Message> {
        if let Some(ref client) = self.uwhportal_client {
            let request = client.post_game_scores(event_id, game_number, scores, false);
            Task::future(async move {
                match request.await {
                    Ok(()) => {
                        info!("Successfully posted game score");
                    }
                    Err(e) => {
                        error!("Failed to post game score: {e}");
                    }
                }
                Message::NoAction
            })
        } else {
            Task::none()
        }
    }

    fn check_uwhportal_auth(&self) -> Task<Message> {
        if let Some(ref uwhportal_client) = self.uwhportal_client {
            let request = uwhportal_client.verify_token();
            Task::future(async move {
                match request.await {
                    Ok(()) => info!("Successfully checked uwhportal token validity"),
                    Err(e) => error!("Failed to check uwhportal token validity: {e}"),
                }
                Message::NoAction
            })
        } else {
            Task::none()
        }
    }

    fn post_game_stats(
        &self,
        event_id: &EventId,
        game_number: u32,
        stats: String,
    ) -> Task<Message> {
        if let Some(ref uwhportal_client) = self.uwhportal_client {
            let request = uwhportal_client.post_game_stats(event_id, game_number, stats);
            Task::future(async move {
                match request.await {
                    Ok(()) => info!("Successfully posted game stats"),
                    Err(e) => error!("Failed to post game stats: {e}"),
                }
                Message::NoAction
            })
        } else {
            Task::none()
        }
    }

    fn handle_game_start(&mut self, new_game_num: u32) {
        if self.using_uwhportal {
            debug!("Searching for next game info after game {new_game_num}");
            if let (Some(schedule), Some(pool)) = (&self.schedule, &self.current_court) {
                let this_game_start = match schedule.games.get(&new_game_num) {
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
                    let timing = schedule.get_game_timing(next_game.number).cloned();
                    let info = NextGameInfo {
                        number: next_game.number,
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

    fn handle_game_end(&self, game_number: u32) -> Task<Message> {
        let mut tasks = vec![];
        if self.using_uwhportal {
            if let Some(info) = self.tm.lock().unwrap().last_game_info() {
                let stats = info.stats.as_json();

                info!(
                    "Game ended, scores: {:?} stats were: {:?}",
                    info.scores, stats
                );

                if let Some(ref event_id) = self.current_event_id {
                    tasks.push(self.request_schedule(event_id.clone()));
                    tasks.push(self.post_game_score(event_id, game_number, info.scores));
                    tasks.push(self.post_game_stats(event_id, game_number, stats));
                } else {
                    error!("Missing current event id to handle game end");
                }
            } else {
                warn!("Game ended, but no last game info was available");
            }
        }

        Task::batch(tasks)
    }

    fn apply_settings_change(&mut self) {
        let edited_settings = self.edited_settings.take().unwrap();

        let EditableSettings {
            white_on_right,
            brightness,
            using_uwhportal,
            current_event_id,
            current_court: current_pool,
            schedule: games,
            sound,
            mode,
            collect_scorer_cap_num,
            hide_time,
            config: _config,
            game_number: _game_number,
            track_fouls_and_warnings,
            uwhportal_token: _,
            confirm_score,
        } = edited_settings;

        self.config.hardware.white_on_right = white_on_right;
        self.config.hardware.brightness = brightness;
        self.using_uwhportal = using_uwhportal;
        self.current_event_id = current_event_id;
        self.current_court = current_pool;
        self.schedule = games;
        self.config.sound = sound;
        self.sound.update_settings(self.config.sound.clone());
        self.config.mode = mode;
        self.config.collect_scorer_cap_num = collect_scorer_cap_num;
        self.config.track_fouls_and_warnings = track_fouls_and_warnings;
        self.config.confirm_score = confirm_score;

        if self.config.hide_time != hide_time {
            self.config.hide_time = hide_time;
            self.update_sender
                .set_hide_time(self.config.hide_time)
                .unwrap();
        }
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
            serial_ports,
            binary_port,
            json_port,
            sim_child,
            require_https,
            fullscreen,
            list_all_events,
            touchscreen,
        } = flags;

        let mut tm = TournamentManager::new(config.game.clone());
        tm.start_clock(Instant::now());

        let portal_token = if !config.uwhportal.token.is_empty() {
            Some(config.uwhportal.token.as_str())
        } else {
            None
        };
        let uwhportal_client = match UwhPortalClient::new(
            &config.uwhportal.url,
            portal_token,
            require_https,
            REQUEST_TIMEOUT,
        ) {
            Ok(c) => Some(c),
            Err(e) => {
                error!("Failed to start UWH Portal Client: {e}");
                None
            }
        };

        let tm = Arc::new(Mutex::new(tm));

        let update_sender =
            UpdateSender::new(serial_ports, binary_port, json_port, config.hide_time);

        let sound =
            SoundController::new(config.sound.clone(), update_sender.get_trigger_flash_fn());

        let snapshot = Default::default();

        let new = Self {
            pen_edit: ListEditor::new(tm.clone()),
            warn_edit: ListEditor::new(tm.clone()),
            foul_edit: ListEditor::new(tm.clone()),
            tm,
            config,
            edited_settings: Default::default(),
            snapshot,
            app_state: AppState::MainPage,
            last_app_state: AppState::MainPage,
            last_message: Message::NoAction,
            update_sender,
            uwhportal_client,
            using_uwhportal: false,
            events: None,
            schedule: None,
            current_event_id: None,
            current_court: None,
            sound,
            sim_child,
            list_all_events,
            touchscreen,
        };

        let task = Task::batch(vec![
            new.request_event_list(),
            new.check_uwhportal_auth(),
            if fullscreen {
                window::get_latest().and_then(|w| window::change_mode(w, window::Mode::Fullscreen))
            } else {
                Task::none()
            },
        ]);

        (new, task)
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
                let task = if let AppState::TimeEdit(was_running, game_time, timeout_time) =
                    self.app_state
                {
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
                };
                task
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
                            ScrollOption::GameParameter | ScrollOption::Equal => unreachable!(),
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
                            ScrollOption::GameParameter => unreachable!(),
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
            Message::KeypadPage(page) => {
                let init_val = match page {
                    KeypadPage::AddScore(_)
                    | KeypadPage::Penalty(None, _, _, _)
                    | KeypadPage::FoulAdd { origin: None, .. }
                    | KeypadPage::WarningAdd { origin: None, .. } => 0,
                    KeypadPage::Penalty(Some((color, index)), _, _, _) => {
                        self.pen_edit.get_item(color, index).unwrap().player_number as u16
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
                    KeypadPage::TeamTimeouts(_, _) => self.config.game.num_team_timeouts_allowed,
                    KeypadPage::GameNumber => self
                        .edited_settings
                        .as_ref()
                        .unwrap()
                        .game_number
                        .try_into()
                        .unwrap_or(0),
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
                let edited_settings = EditableSettings {
                    config: self.tm.lock().unwrap().config().clone(),
                    game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                        self.snapshot.next_game_number
                    } else {
                        self.snapshot.game_number
                    },
                    white_on_right: self.config.hardware.white_on_right,
                    brightness: self.config.hardware.brightness,
                    using_uwhportal: self.using_uwhportal,
                    uwhportal_token: self.config.uwhportal.token.clone(),
                    current_event_id: self.current_event_id.clone(),
                    current_court: self.current_court.clone(),
                    schedule: self.schedule.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
                    confirm_score: self.config.confirm_score,
                };

                self.edited_settings = Some(edited_settings);

                self.app_state = AppState::EditGameConfig(ConfigPage::Main);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ChangeConfigPage(new_page) => {
                if let AppState::EditGameConfig(ref mut page) = self.app_state {
                    *page = new_page;
                } else {
                    unreachable!();
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ConfigEditComplete { canceled } => {
                self.app_state = if !canceled {
                    let mut tm = self.tm.lock().unwrap();

                    let edited_settings = self.edited_settings.as_mut().unwrap();

                    let mut uwhportal_incomplete = edited_settings.using_uwhportal
                        && (edited_settings.current_event_id.is_none()
                            || edited_settings.current_court.is_none()
                            || edited_settings.schedule.is_none());
                    if edited_settings.using_uwhportal && !uwhportal_incomplete {
                        match edited_settings
                            .schedule
                            .as_ref()
                            .unwrap()
                            .games
                            .get(&edited_settings.game_number)
                        {
                            Some(g) => {
                                uwhportal_incomplete =
                                    g.court != *edited_settings.current_court.as_ref().unwrap()
                            }
                            None => uwhportal_incomplete = true,
                        };
                    }

                    let new_config = if edited_settings.using_uwhportal && !uwhportal_incomplete {
                        edited_settings
                            .schedule
                            .as_ref()
                            .and_then(|schedule| {
                                schedule.get_game_timing(edited_settings.game_number)
                            })
                            .cloned()
                            .map(|tr| tr.into())
                            .unwrap_or_else(|| tm.config().clone())
                    } else {
                        edited_settings.config.clone()
                    };

                    if uwhportal_incomplete {
                        AppState::ConfirmationPage(ConfirmationKind::UwhPortalIncomplete)
                    } else if new_config != *tm.config() {
                        if tm.current_period() != GamePeriod::BetweenGames {
                            AppState::ConfirmationPage(ConfirmationKind::GameConfigChanged(
                                new_config,
                            ))
                        } else {
                            tm.set_config(new_config.clone()).unwrap();
                            self.config.game = new_config;

                            let (game, timing) = edited_settings
                                .schedule
                                .as_ref()
                                .map(|schedule| {
                                    schedule.get_game_and_timing(edited_settings.game_number)
                                })
                                .unwrap_or((None, None));
                            let start_time = game.map(|g| g.start_time);

                            tm.set_next_game(NextGameInfo {
                                number: edited_settings.game_number,
                                timing: timing.cloned(),
                                start_time,
                            });

                            if edited_settings.using_uwhportal {
                                tm.apply_next_game_start(Instant::now()).unwrap();
                            } else {
                                tm.clear_scheduled_game_start();
                            }

                            drop(tm);
                            self.apply_settings_change();

                            confy::store(APP_NAME, None, &self.config).unwrap();
                            AppState::MainPage
                        }
                    } else if edited_settings.game_number != self.snapshot.game_number {
                        if tm.current_period() != GamePeriod::BetweenGames {
                            AppState::ConfirmationPage(ConfirmationKind::GameNumberChanged)
                        } else {
                            let next_game_info = if edited_settings.using_uwhportal {
                                let (game, timing) = edited_settings
                                    .schedule
                                    .as_ref()
                                    .map(|schedule| {
                                        schedule.get_game_and_timing(edited_settings.game_number)
                                    })
                                    .unwrap_or((None, None));
                                NextGameInfo {
                                    number: edited_settings.game_number,
                                    timing: timing.cloned(),
                                    start_time: game.map(|g| g.start_time),
                                }
                            } else {
                                NextGameInfo {
                                    number: edited_settings.game_number,
                                    timing: None,
                                    start_time: None,
                                }
                            };

                            tm.set_next_game(next_game_info);

                            if edited_settings.using_uwhportal {
                                tm.apply_next_game_start(Instant::now()).unwrap();
                            }

                            drop(tm);
                            self.apply_settings_change();

                            confy::store(APP_NAME, None, &self.config).unwrap();

                            AppState::MainPage
                        }
                    } else {
                        drop(tm);
                        self.apply_settings_change();

                        confy::store(APP_NAME, None, &self.config).unwrap();
                        AppState::MainPage
                    }
                } else {
                    AppState::MainPage
                };
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
                if !canceled {
                    let edited_settings = self.edited_settings.as_mut().unwrap();
                    match self.app_state {
                        AppState::ParameterEditor(param, dur) => match param {
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
                        },
                        AppState::KeypadPage(KeypadPage::GameNumber, num) => {
                            edited_settings.game_number = num.into();
                        }
                        AppState::KeypadPage(KeypadPage::TeamTimeouts(len, per_half), num) => {
                            edited_settings.config.team_timeout_duration = len;
                            edited_settings.config.num_team_timeouts_allowed = num;
                            edited_settings.config.timeouts_counted_per_half = per_half;
                        }
                        _ => unreachable!(),
                    }
                }

                let next_page = match self.app_state {
                    AppState::ParameterEditor(_, _) => ConfigPage::Game,
                    AppState::KeypadPage(KeypadPage::GameNumber, _) => ConfigPage::Main,
                    AppState::KeypadPage(KeypadPage::TeamTimeouts(_, _), _) => ConfigPage::Game,
                    AppState::ParameterList(param, _) => match param {
                        ListableParameter::Game => ConfigPage::Main,
                        ListableParameter::Event | ListableParameter::Court => ConfigPage::Game,
                    },
                    _ => unreachable!(),
                };

                self.app_state = AppState::EditGameConfig(next_page);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::ParameterSelected(param, val) => {
                let edited_settings = self.edited_settings.as_mut().unwrap();
                let task = match param {
                    ListableParameter::Event => {
                        let id = EventId::from_full(val).unwrap();
                        edited_settings.current_event_id = Some(id.clone());
                        if let Some(pools) = self
                            .events
                            .as_ref()
                            .and_then(|events| events.get(&id).and_then(|e| e.courts.as_ref()))
                        {
                            if pools.len() == 1 {
                                if let Some(ref mut edits) = self.edited_settings {
                                    if edits.current_court.is_none() {
                                        edits.current_court = Some(pools[0].clone());
                                    }
                                }
                            }
                        }
                        self.request_schedule(id)
                    }
                    ListableParameter::Court => {
                        edited_settings.current_court = Some(val);
                        Task::none()
                    }
                    ListableParameter::Game => {
                        edited_settings.game_number = val.parse().unwrap();
                        Task::none()
                    }
                };

                let next_page = match param {
                    ListableParameter::Event | ListableParameter::Court => ConfigPage::Game,
                    ListableParameter::Game => ConfigPage::Main,
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
                    CyclingParameter::Language => {
                        let mut language =
                            Language::from_lang_id(&crate::LANGUAGE_LOADER.current_languages()[0]);
                        language.cycle();
                        crate::request_language(&crate::LANGUAGE_LOADER, &[language.as_lang_id()]);
                    }
                }
                Task::none()
            }
            Message::TextParameterChanged(param, val) => {
                let settings = self.edited_settings.as_mut().unwrap();
                match param {
                    TextParameter::UwhportalToken => settings.uwhportal_token = val,
                }
                Task::none()
            }
            Message::ApplyAuthChanges => {
                let settings = self.edited_settings.as_mut().unwrap();
                self.config.uwhportal.token = settings.uwhportal_token.clone();
                if let Some(client) = self.uwhportal_client.as_mut() {
                    client.set_token(&settings.uwhportal_token);
                }

                let task = self.check_uwhportal_auth();

                self.app_state = AppState::EditGameConfig(ConfigPage::Game);
                trace!("AppState changed to {:?}", self.app_state);
                task
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
                let new_config = if let AppState::ConfirmationPage(
                    ConfirmationKind::GameConfigChanged(ref config),
                ) = self.app_state
                {
                    Some(config.clone())
                } else {
                    None
                };

                let (app_state, task) = match selection {
                    ConfirmationOption::DiscardChanges => (AppState::MainPage, Task::none()),
                    ConfirmationOption::GoBack => {
                        (AppState::EditGameConfig(ConfigPage::Main), Task::none())
                    }
                    ConfirmationOption::EndGameAndApply => {
                        let edited_settings = self.edited_settings.as_ref().unwrap();
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();
                        tm.reset_game(now);
                        if let Some(config) = new_config {
                            tm.set_config(config.clone()).unwrap();
                            self.config.game = config;
                        }

                        let (game, timing) = edited_settings
                            .schedule
                            .as_ref()
                            .map(|schedule| {
                                schedule.get_game_and_timing(edited_settings.game_number)
                            })
                            .unwrap_or((None, None));
                        let start_time = game.map(|g| g.start_time);

                        tm.set_next_game(NextGameInfo {
                            number: edited_settings.game_number,
                            timing: timing.cloned(),
                            start_time,
                        });

                        if edited_settings.using_uwhportal {
                            tm.apply_next_game_start(now).unwrap();
                        } else {
                            tm.clear_scheduled_game_start();
                        }

                        std::mem::drop(tm);
                        self.apply_settings_change();

                        confy::store(APP_NAME, None, &self.config).unwrap();
                        let snapshot = self.tm.lock().unwrap().generate_snapshot(now).unwrap(); // TODO: Remove this unwrap
                        (AppState::MainPage, self.apply_snapshot(snapshot))
                    }
                    ConfirmationOption::KeepGameAndApply => {
                        let edited_settings = self.edited_settings.as_ref().unwrap();
                        let mut tm = self.tm.lock().unwrap();
                        tm.set_game_number(edited_settings.game_number);
                        let snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                        std::mem::drop(tm);

                        self.apply_settings_change();

                        confy::store(APP_NAME, None, &self.config).unwrap();
                        (AppState::MainPage, self.apply_snapshot(snapshot))
                    }
                };
                self.app_state = app_state;
                trace!("AppState changed to {:?}", self.app_state);
                task
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
                    tasks.push(self.request_teams_list(event.id.clone(), &event.slug));
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
            Message::RecvSchedule(event_id, schedule) => {
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
                                            .get_game_and_timing(tm.next_game_number())
                                        {
                                            info!(
                                                "Setting upcoming game info from received schedule: {game:?}"
                                            );
                                            tm.set_next_game(NextGameInfo {
                                                number: game.number,
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
            Message::NoAction => Task::none(),
        }
    }

    pub(super) fn view(&self) -> Element<Message> {
        let data = ViewData {
            snapshot: &self.snapshot,
            mode: self.config.mode,
            clock_running: self.tm.lock().unwrap().clock_is_running(),
            teams: self.current_event_id.as_ref().and_then(|id| {
                self.events
                    .as_ref()
                    .and_then(|events| events.get(id).and_then(|event| event.teams.as_ref()))
            }),
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
                    self.schedule.as_ref().map(|s| &s.games),
                    self.config.track_fouls_and_warnings,
                )
            }
            AppState::TimeEdit(_, time, timeout_time) =>
                build_time_edit_view(data, time, timeout_time),
            AppState::ScoreEdit {
                scores,
                is_confirmation,
            } => build_score_edit_view(data, scores, is_confirmation),
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
                self.schedule.as_ref().map(|s| &s.games)
            ),
            AppState::WarningsSummaryPage => build_warnings_summary_page(data),
            AppState::EditGameConfig(page) => build_game_config_edit_page(
                data,
                self.edited_settings.as_ref().unwrap(),
                self.events.as_ref(),
                page,
                self.uwhportal_client.as_ref().map(|c| c.token_validity()),
                self.touchscreen,
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
            AppState::ConfirmScores(scores) => build_score_confirmation_page(data, scores),
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
        Subscription::run(time_updater)
    }

    pub fn application_style(&self, _theme: &Theme) -> Appearance {
        Appearance {
            background_color: WINDOW_BACKGROUND,
            text_color: BLACK,
        }
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
