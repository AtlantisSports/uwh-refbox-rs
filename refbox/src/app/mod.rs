use self::infraction::InfractionDetails;

use super::APP_NAME;
use crate::{
    config::{Config, Mode},
    penalty_editor::*,
    sound_controller::*,
    tournament_manager::{penalty::*, *},
};
use iced::{Application, Command, Subscription, executor, widget::column};
use iced_futures::{
    futures::stream::{self, BoxStream},
    subscription::{EventStream, Recipe},
};
use iced_runtime::{command, window};
use log::*;
use std::{
    borrow::Cow,
    cmp::min,
    collections::{BTreeMap, BTreeSet},
    process::Child,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, watch},
    task,
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

mod view_builders;
use view_builders::*;

mod message;
use message::*;

pub mod style;
use style::{PADDING, SPACING};

pub mod update_sender;
use update_sender::*;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub type Element<'a, Message> = iced::Element<'a, Message, iced::Renderer<style::ApplicationTheme>>;

pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    config: Config,
    edited_settings: Option<EditableSettings>,
    snapshot: GameSnapshot,
    time_updater: TimeUpdater,
    pen_edit: ListEditor<Penalty, Color>,
    warn_edit: ListEditor<InfractionDetails, Color>,
    foul_edit: ListEditor<InfractionDetails, Option<Color>>,
    app_state: AppState,
    last_app_state: AppState,
    last_message: Message,
    update_sender: UpdateSender,
    message_listener: MessageListener,
    msg_tx: mpsc::UnboundedSender<Message>,
    uwhportal_client: Option<UwhPortalClient>,
    using_uwhportal: bool,
    events: Option<BTreeMap<EventId, Event>>,
    schedule: Option<Schedule>,
    current_event_id: Option<EventId>,
    current_court: Option<String>,
    sound: SoundController,
    sim_child: Option<Child>,
    fullscreen: bool,
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
    GameDetailsPage,
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
    fn apply_snapshot(&mut self, mut new_snapshot: GameSnapshot) {
        if new_snapshot.current_period != self.snapshot.current_period {
            if new_snapshot.current_period == GamePeriod::BetweenGames {
                self.handle_game_end(new_snapshot.game_number);
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

    fn request_event_list(&self) {
        if let Some(ref client) = self.uwhportal_client {
            let request = client.get_event_list(self.list_all_events);
            let msg_tx = self.msg_tx.clone();
            task::spawn(async move {
                match request.await {
                    Ok(events) => {
                        info!("Got event list");
                        msg_tx.send(Message::RecvEventList(events)).unwrap();
                    }
                    Err(e) => {
                        error!("Failed to get event list: {e}");
                    }
                }
            });
        }
    }

    fn request_teams_list(&self, event_id: EventId, event_slug: &str) {
        if let Some(ref client) = self.uwhportal_client {
            let request = client.get_event_schedule(event_slug);
            let msg_tx = self.msg_tx.clone();
            task::spawn(async move {
                match request.await {
                    Ok(teams) => {
                        info!("Got teams list");
                        msg_tx
                            .send(Message::RecvTeamsList(event_id, teams))
                            .unwrap();
                    }
                    Err(e) => {
                        error!("Failed to get teams list: {e}");
                    }
                }
            });
        }
    }

    fn request_schedule(&self, event_id: EventId) {
        if let Some(ref client) = self.uwhportal_client {
            let request = client.get_event_schedule_privileged(&event_id);
            let msg_tx = self.msg_tx.clone();
            task::spawn(async move {
                match request.await {
                    Ok(schedule) => {
                        info!("Got schedule");
                        msg_tx
                            .send(Message::RecvSchedule(event_id, schedule))
                            .unwrap();
                    }
                    Err(e) => {
                        error!("Failed to get schedule: {e}");
                    }
                }
            });
        }
    }

    fn post_game_score(&self, event_id: &EventId, game_number: u32, scores: BlackWhiteBundle<u8>) {
        if let Some(ref client) = self.uwhportal_client {
            let request = client.post_game_scores(event_id, game_number, scores, false);
            task::spawn(async move {
                match request.await {
                    Ok(()) => {
                        info!("Successfully posted game score");
                    }
                    Err(e) => {
                        error!("Failed to post game score: {e}");
                    }
                }
            });
        }
    }

    fn check_uwhportal_auth(&self) {
        if let Some(ref uwhportal_client) = self.uwhportal_client {
            let request = uwhportal_client.verify_token();
            tokio::spawn(async move {
                match request.await {
                    Ok(()) => info!("Successfully checked uwhportal token validity"),
                    Err(e) => error!("Failed to check uwhportal token validity: {e}"),
                }
            });
        }
    }

    fn post_game_stats(&self, event_id: &EventId, game_number: u32, stats: String) {
        if let Some(ref uwhportal_client) = self.uwhportal_client {
            let request = uwhportal_client.post_game_stats(event_id, game_number, stats);
            tokio::spawn(async move {
                match request.await {
                    Ok(()) => info!("Successfully posted game stats"),
                    Err(e) => error!("Failed to post game stats: {e}"),
                }
            });
        }
    }

    fn handle_game_start(&mut self, new_game_num: u32) {
        if self.using_uwhportal {
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
        }
    }

    fn handle_game_end(&self, game_number: u32) {
        if self.using_uwhportal {
            let mut stats = self
                .tm
                .lock()
                .unwrap()
                .last_game_stats()
                .map(|s| s.as_json());

            if let Some(ref stats) = stats {
                info!("Game ended, stats were: {:?}", stats);
            } else {
                warn!("Game ended, but no stats were available");
            }

            if let Some(ref event_id) = self.current_event_id {
                self.request_schedule(event_id.clone());
                if let Some(stats) = stats.take() {
                    self.post_game_stats(event_id, game_number, stats);
                }
            } else {
                error!("Missing current event id to handle game end");
            }
        }
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

impl Application for RefBoxApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = style::ApplicationTheme;
    type Flags = RefBoxAppFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let Self::Flags {
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

        let (msg_tx, rx) = mpsc::unbounded_channel();
        let message_listener = MessageListener {
            rx: Arc::new(Mutex::new(Some(rx))),
        };
        msg_tx.send(Message::Init).unwrap();

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

        let clock_running_receiver = tm.get_start_stop_rx();

        let tm = Arc::new(Mutex::new(tm));

        let update_sender =
            UpdateSender::new(serial_ports, binary_port, json_port, config.hide_time);

        let sound =
            SoundController::new(config.sound.clone(), update_sender.get_trigger_flash_fn());

        let snapshot = Default::default();

        (
            Self {
                pen_edit: ListEditor::new(tm.clone()),
                warn_edit: ListEditor::new(tm.clone()),
                foul_edit: ListEditor::new(tm.clone()),
                time_updater: TimeUpdater {
                    tm: tm.clone(),
                    clock_running_receiver,
                },
                tm,
                config,
                edited_settings: Default::default(),
                snapshot,
                app_state: AppState::MainPage,
                last_app_state: AppState::MainPage,
                last_message: Message::NoAction,
                update_sender,
                message_listener,
                msg_tx,
                uwhportal_client,
                using_uwhportal: false,
                events: None,
                schedule: None,
                current_event_id: None,
                current_court: None,
                sound,
                sim_child,
                fullscreen,
                list_all_events,
                touchscreen,
            },
            Command::single(command::Action::LoadFont {
                bytes: Cow::from(&include_bytes!("../../resources/Roboto-Medium.ttf")[..]),
                tagger: Box::new(|res| match res {
                    Ok(()) => {
                        info!("Loaded font");
                        Message::NoAction
                    }
                    Err(e) => panic!("Failed to load font: {e:?}"),
                }),
            }),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::from_recipe(self.time_updater.clone()),
            Subscription::from_recipe(self.message_listener.clone()),
        ])
    }

    fn title(&self) -> String {
        "UWH Ref Box".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        trace!("Handling message: {message:?}");

        if !message.is_repeatable() && (message == self.last_message) {
            warn!("Ignoring a repeated message: {message:?}");
            self.last_message = message.clone();
            return Command::none();
        } else {
            self.last_message = message.clone();
        }

        let command = if matches!(message, Message::Init) && self.fullscreen {
            Command::single(command::Action::Window(window::Action::ChangeMode(
                iced_core::window::Mode::Fullscreen,
            )))
        } else {
            Command::none()
        };

        match message {
            Message::Init => {
                self.request_event_list();
                self.check_uwhportal_auth();
            }
            Message::NewSnapshot(snapshot) => {
                self.apply_snapshot(snapshot);
            }
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
                    self.apply_snapshot(snapshot);
                    self.app_state = self.last_app_state.clone();
                    trace!("AppState changed to {:?}", self.app_state);
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
                self.apply_snapshot(snapshot);
            }
            Message::EditScores => {
                let tm = self.tm.lock().unwrap();
                self.app_state = AppState::ScoreEdit {
                    scores: tm.get_scores(),
                    is_confirmation: false,
                };
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::AddNewScore(color) => {
                if self.config.collect_scorer_cap_num {
                    self.app_state = AppState::KeypadPage(KeypadPage::AddScore(color), 0);
                } else {
                    let mut tm = self.tm.lock().unwrap();
                    let now = Instant::now();
                    if tm.current_period() == GamePeriod::SuddenDeath {
                        tm.stop_clock(now).unwrap();
                        let mut scores = tm.get_scores();
                        scores[color] = scores[color].saturating_add(1);

                        self.app_state = AppState::ConfirmScores(scores);
                    } else {
                        tm.add_score(color, 0, now);
                        let snapshot = tm.generate_snapshot(now).unwrap(); // TODO: Remove this unwrap
                        std::mem::drop(tm);
                        self.apply_snapshot(snapshot);
                        self.app_state = AppState::MainPage;
                    }
                }
                trace!("AppState changed to {:?}", self.app_state);
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
            }
            Message::ScoreEditComplete { canceled } => {
                let mut tm = self.tm.lock().unwrap();
                let mut now = Instant::now();

                self.app_state = if let AppState::ScoreEdit {
                    scores,
                    is_confirmation,
                } = self.app_state
                {
                    if is_confirmation {
                        if let Some((id, game)) = self.schedule.as_ref().and_then(|schedule| {
                            schedule
                                .games
                                .get(&tm.game_number())
                                .map(|n| (&schedule.event_id, n))
                        }) {
                            self.post_game_score(id, game.number, scores);
                        }

                        tm.set_scores(scores, now);
                        tm.start_clock(now);

                        // Update `tm` after game ends to get into Between Games
                        now += Duration::from_millis(2);
                        tm.update(now).unwrap();
                        AppState::MainPage
                    } else if !canceled {
                        if tm.current_period() == GamePeriod::SuddenDeath
                            && (scores.black != scores.white)
                        {
                            tm.stop_clock(now).unwrap();
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
                self.apply_snapshot(snapshot);

                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::PenaltyOverview => {
                if let Err(e) = self.pen_edit.start_session() {
                    warn!("Failed to start penalty edit session: {e}");
                    self.pen_edit.abort_session();
                    self.pen_edit.start_session().unwrap();
                }
                self.app_state = AppState::PenaltyOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::WarningOverview => {
                if let Err(e) = self.warn_edit.start_session() {
                    warn!("Failed to start warning edit session: {e}");
                    self.warn_edit.abort_session();
                    self.warn_edit.start_session().unwrap();
                }
                self.app_state = AppState::WarningOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
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
                self.apply_snapshot(snapshot);
                trace!("AppState changed to {:?}", self.app_state);
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
                self.apply_snapshot(snapshot);
                trace!("AppState changed to {:?}", self.app_state);
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
                self.apply_snapshot(snapshot);
                trace!("AppState changed to {:?}", self.app_state);
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
            }
            Message::AddScoreComplete { canceled } => {
                self.app_state = if !canceled {
                    if let AppState::KeypadPage(KeypadPage::AddScore(color), player) =
                        self.app_state
                    {
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();

                        let app_state = if tm.current_period() == GamePeriod::SuddenDeath {
                            tm.stop_clock(now).unwrap();
                            let mut scores = tm.get_scores();
                            scores[color] = scores[color].saturating_add(1);

                            AppState::ConfirmScores(scores)
                        } else {
                            tm.add_score(color, player.try_into().unwrap(), now);
                            AppState::MainPage
                        };
                        let snapshot = tm.generate_snapshot(now).unwrap();

                        std::mem::drop(tm);
                        self.apply_snapshot(snapshot);

                        app_state
                    } else {
                        unreachable!()
                    }
                } else {
                    AppState::MainPage
                };
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ShowGameDetails => {
                self.app_state = AppState::GameDetailsPage;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ShowWarnings => {
                self.app_state = AppState::WarningsSummaryPage;
                trace!("AppState changed to {:?}", self.app_state);
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
            }
            Message::ChangeConfigPage(new_page) => {
                if let AppState::EditGameConfig(ref mut page) = self.app_state {
                    *page = new_page;
                } else {
                    unreachable!();
                }
                trace!("AppState changed to {:?}", self.app_state);
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
            }
            Message::ParameterSelected(param, val) => {
                let edited_settings = self.edited_settings.as_mut().unwrap();
                match param {
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
                        self.request_schedule(id);
                    }
                    ListableParameter::Court => edited_settings.current_court = Some(val),
                    ListableParameter::Game => edited_settings.game_number = val.parse().unwrap(),
                };

                let next_page = match param {
                    ListableParameter::Event | ListableParameter::Court => ConfigPage::Game,
                    ListableParameter::Game => ConfigPage::Main,
                };

                self.app_state = AppState::EditGameConfig(next_page);
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ToggleBoolParameter(param) => match param {
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
                    if let AppState::KeypadPage(KeypadPage::TeamTimeouts(_, ref mut per_half), _) =
                        self.app_state
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
                        BoolGameParameter::SingleHalf => edited_settings.config.single_half ^= true,
                        BoolGameParameter::WhiteOnRight => edited_settings.white_on_right ^= true,
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
                        BoolGameParameter::ConfirmScore => edited_settings.confirm_score ^= true,
                    }
                }
            },
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
            }
            Message::TextParameterChanged(param, val) => {
                let settings = self.edited_settings.as_mut().unwrap();
                match param {
                    TextParameter::UwhportalToken => settings.uwhportal_token = val,
                }
            }
            Message::ApplyAuthChanges => {
                let settings = self.edited_settings.as_mut().unwrap();
                self.config.uwhportal.token = settings.uwhportal_token.clone();

                self.check_uwhportal_auth();

                self.app_state = AppState::EditGameConfig(ConfigPage::Game);
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::RequestRemoteId => {
                if let AppState::EditGameConfig(ConfigPage::Remotes(_, ref mut listening)) =
                    self.app_state
                {
                    let _msg_tx = self.msg_tx.clone();
                    self.sound.request_next_remote_id(move |id| {
                        _msg_tx.send(Message::GotRemoteId(id)).unwrap()
                    });
                    *listening = true;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
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
            }
            Message::DeleteRemote(index) => {
                if let Some(ref mut settings) = self.edited_settings {
                    settings.sound.remotes.remove(index);
                } else {
                    unreachable!()
                }
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

                self.app_state = match selection {
                    ConfirmationOption::DiscardChanges => AppState::MainPage,
                    ConfirmationOption::GoBack => AppState::EditGameConfig(ConfigPage::Main),
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
                        self.apply_snapshot(snapshot);
                        AppState::MainPage
                    }
                    ConfirmationOption::KeepGameAndApply => {
                        let edited_settings = self.edited_settings.as_ref().unwrap();
                        let mut tm = self.tm.lock().unwrap();
                        tm.set_game_number(edited_settings.game_number);
                        let snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                        std::mem::drop(tm);

                        self.apply_settings_change();

                        confy::store(APP_NAME, None, &self.config).unwrap();
                        self.apply_snapshot(snapshot);
                        AppState::MainPage
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ConfirmScores(snapshot) => {
                if self.config.confirm_score {
                    self.apply_snapshot(snapshot);
                    self.app_state = AppState::ConfirmScores(self.snapshot.scores);
                    trace!("AppState changed to {:?}", self.app_state);
                } else {
                    let mut tm = self.tm.lock().unwrap();
                    let now = Instant::now();
                    let scores = tm.get_scores();
                    tm.set_scores(scores, now);
                    tm.start_clock(now);
                    tm.update(now + Duration::from_millis(2)).unwrap(); // Need to update after game ends
                    self.app_state = AppState::MainPage;
                    trace!("AppState changed to {:?}", self.app_state);
                }
            }
            Message::ScoreConfirmation { correct } => {
                self.app_state = if let AppState::ConfirmScores(scores) = self.app_state {
                    if correct {
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();

                        if let Some(id) = self.schedule.as_ref().map(|schedule| &schedule.event_id)
                        {
                            self.post_game_score(id, tm.game_number(), scores);
                        }

                        tm.set_scores(scores, now);
                        tm.start_clock(now);
                        tm.update(now + Duration::from_millis(2)).unwrap(); // Need to update after game ends

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
                self.apply_snapshot(snapshot);
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
                self.apply_snapshot(snapshot);
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
                self.apply_snapshot(snapshot);
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
                self.apply_snapshot(snapshot);

                if would_end {
                    self.app_state = AppState::ConfirmScores(self.snapshot.scores);
                    trace!("AppState changed to {:?}", self.app_state);
                }

                if let AppState::TimeEdit(_, _, ref mut timeout) = self.app_state {
                    *timeout = None;
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::RecvEventList(e_list) => {
                let e_map: BTreeMap<_, _> = e_list.into_iter().map(|e| (e.id.clone(), e)).collect();
                for event in e_map.values() {
                    self.request_teams_list(event.id.clone(), &event.slug);
                }
                self.events = Some(e_map);
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
            }
            Message::StartClock => self.tm.lock().unwrap().start_clock(Instant::now()),
            Message::StopClock => self.tm.lock().unwrap().stop_clock(Instant::now()).unwrap(),
            Message::NoAction => {}
        };

        command
    }

    fn view(&self) -> Element<Message> {
        let clock_running = self.tm.lock().unwrap().clock_is_running();
        let teams = self.current_event_id.as_ref().and_then(|id| {
            self.events
                .as_ref()
                .and_then(|events| events.get(id).and_then(|event| event.teams.as_ref()))
        });
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
                    &self.snapshot,
                    game_config,
                    self.using_uwhportal,
                    self.schedule.as_ref().map(|s| &s.games),
                    teams,
                    &self.config,
                    clock_running,
                )
            }
            AppState::TimeEdit(_, time, timeout_time) => build_time_edit_view(
                &self.snapshot,
                time,
                timeout_time,
                self.config.mode,
                clock_running,
            ),
            AppState::ScoreEdit {
                scores,
                is_confirmation,
            } => build_score_edit_view(
                &self.snapshot,
                scores,
                is_confirmation,
                self.config.mode,
                clock_running,
            ),
            AppState::PenaltyOverview(indices) => build_penalty_overview_page(
                &self.snapshot,
                self.pen_edit.get_printable_lists(Instant::now()).unwrap(),
                indices,
                self.config.mode,
                clock_running,
            ),
            AppState::WarningOverview(indices) => build_warning_overview_page(
                &self.snapshot,
                self.warn_edit.get_printable_lists(Instant::now()).unwrap(),
                indices,
                self.config.mode,
                clock_running,
            ),
            AppState::FoulOverview(indices) => build_foul_overview_page(
                &self.snapshot,
                self.foul_edit.get_printable_lists(Instant::now()).unwrap(),
                indices,
                self.config.mode,
                clock_running,
            ),
            AppState::KeypadPage(page, player_num) => build_keypad_page(
                &self.snapshot,
                page,
                player_num,
                &self.config,
                clock_running,
            ),
            AppState::GameDetailsPage => build_game_info_page(
                &self.snapshot,
                &self.config.game,
                self.using_uwhportal,
                self.schedule.as_ref().map(|s| &s.games),
                teams,
                self.config.mode,
                clock_running,
            ),
            AppState::WarningsSummaryPage =>
                build_warnings_summary_page(&self.snapshot, self.config.mode, clock_running,),
            AppState::EditGameConfig(page) => build_game_config_edit_page(
                &self.snapshot,
                self.edited_settings.as_ref().unwrap(),
                self.events.as_ref(),
                page,
                self.config.mode,
                clock_running,
                self.uwhportal_client.as_ref().map(|c| c.token_validity()),
                self.touchscreen,
            ),
            AppState::ParameterEditor(param, dur) => build_game_parameter_editor(
                &self.snapshot,
                param,
                dur,
                self.config.mode,
                clock_running,
            ),
            AppState::ParameterList(param, index) => build_list_selector_page(
                &self.snapshot,
                param,
                index,
                self.edited_settings.as_ref().unwrap(),
                self.events.as_ref(),
                teams,
                self.config.mode,
                clock_running,
            ),
            AppState::ConfirmationPage(ref kind) => {
                build_confirmation_page(&self.snapshot, kind, self.config.mode, clock_running)
            }
            AppState::ConfirmScores(scores) => build_score_confirmation_page(
                &self.snapshot,
                scores,
                self.config.mode,
                clock_running,
            ),
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
}

#[derive(Clone, Debug)]
struct TimeUpdater {
    tm: Arc<Mutex<TournamentManager>>,
    clock_running_receiver: watch::Receiver<bool>,
}

impl Recipe for TimeUpdater {
    type Output = Message;

    fn hash(&self, state: &mut iced_core::Hasher) {
        use std::hash::Hash;

        "TimeUpdater".hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        debug!("Updater started");

        struct State {
            tm: Arc<Mutex<TournamentManager>>,
            clock_running_receiver: watch::Receiver<bool>,
            next_time: Option<Instant>,
        }

        let state = State {
            tm: self.tm.clone(),
            clock_running_receiver: self.clock_running_receiver.clone(),
            next_time: Some(Instant::now()),
        };

        Box::pin(stream::unfold(state, |mut state| async move {
            let mut clock_running = true;
            if let Some(next_time) = state.next_time {
                if next_time > Instant::now() {
                    match timeout_at(next_time, state.clock_running_receiver.changed()).await {
                        Err(_) => {}
                        Ok(Err(_)) => return None,
                        Ok(Ok(())) => {
                            clock_running = *state.clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                    };
                } else {
                    match state.clock_running_receiver.has_changed() {
                        Ok(true) => {
                            clock_running = *state.clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                        Ok(false) => {}
                        Err(_) => {
                            return None;
                        }
                    };
                }
            } else {
                debug!("Awaiting a new clock running message");
                match state.clock_running_receiver.changed().await {
                    Err(_) => return None,
                    Ok(()) => {
                        clock_running = *state.clock_running_receiver.borrow();
                        debug!("Received clock running message: {clock_running}");
                    }
                };
            };

            let mut tm = state.tm.lock().unwrap();
            let now = Instant::now();

            let msg_type = if tm.would_end_game(now).unwrap() {
                tm.halt_clock(now, false).unwrap();
                clock_running = false;
                Message::ConfirmScores
            } else {
                tm.update(now).unwrap();
                Message::NewSnapshot
            };

            let snapshot = match tm.generate_snapshot(now) {
                Some(val) => val,
                None => {
                    error!("Failed to generate snapshot. State:\n{tm:#?}");
                    panic!("No snapshot");
                }
            };

            state.next_time = if clock_running {
                Some(tm.next_update_time(now).unwrap())
            } else {
                None
            };

            drop(tm);

            Some((msg_type(snapshot), state))
        }))
    }
}

#[derive(Debug, Clone)]
struct MessageListener {
    rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Message>>>>,
}

impl Recipe for MessageListener {
    type Output = Message;

    fn hash(&self, state: &mut iced_core::Hasher) {
        use std::hash::Hash;

        "MessageListener".hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        info!("Message Listener started");

        let rx = self
            .rx
            .lock()
            .unwrap()
            .take()
            .expect("Listener has already been started");

        Box::pin(stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|msg| (msg, rx))
        }))
    }
}
