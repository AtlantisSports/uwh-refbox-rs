use super::uwhscores::*;
use super::APP_CONFIG_NAME;
use crate::{penalty_editor::*, tournament_manager::*};
use iced::{
    executor,
    pure::{column, Application, Element},
    Command, Subscription,
};
use iced_futures::{
    futures::stream::{self, BoxStream},
    subscription::Recipe,
};
use log::*;
use reqwest::{Client, Method, StatusCode};
use rodio::{
    source::{SineWave, Source, Zero},
    OutputStream, OutputStreamHandle, Sink,
};
use std::{
    cmp::min,
    collections::BTreeMap,
    hash::Hasher,
    process::Child,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, watch},
    task,
    time::{timeout_at, Duration, Instant},
};
use tokio_serial::SerialPortBuilder;
use uwh_common::{
    config::{Config, Game as GameConfig},
    drawing_support::*,
    game_snapshot::{Color as GameColor, GamePeriod, GameSnapshot, TimeoutSnapshot},
};

mod view_builders;
use view_builders::*;

pub mod style;
use style::{PADDING, SPACING, WINDOW_BACKGROUND};

pub mod update_sender;
use update_sender::*;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RETRIES: usize = 6;

pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    config: Config,
    edited_settings: Option<EditableSettings>,
    snapshot: GameSnapshot,
    time_updater: TimeUpdater,
    pen_edit: PenaltyEditor,
    app_state: AppState,
    last_app_state: AppState,
    last_message: Message,
    update_sender: UpdateSender,
    message_listener: MessageListener,
    msg_tx: mpsc::UnboundedSender<Message>,
    client: Option<Client>,
    using_uwhscores: bool,
    tournaments: Option<BTreeMap<u32, TournamentInfo>>,
    games: Option<BTreeMap<u32, GameInfo>>,
    current_tid: Option<u32>,
    current_pool: Option<String>,
    sound: Option<(OutputStream, OutputStreamHandle)>,
    sim_child: Option<Child>,
    fullscreen: bool,
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
    KeypadPage(KeypadPage, u16),
    EditGameConfig,
    ParameterEditor(LengthParameter, Duration),
    ParameterList(ListableParameter, usize),
    ConfirmationPage(ConfirmationKind),
    ConfirmScores(BlackWhiteBundle<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct EditableSettings {
    config: GameConfig,
    game_number: u32,
    white_on_right: bool,
    using_uwhscores: bool,
    current_tid: Option<u32>,
    current_pool: Option<String>,
    games: Option<BTreeMap<u32, GameInfo>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypadPage {
    AddScore(GameColor),
    Penalty(Option<(GameColor, usize)>, GameColor, PenaltyKind),
    GameNumber,
    TeamTimeouts(Duration),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfirmationKind {
    GameNumberChanged,
    GameConfigChanged,
    Error(String),
    UwhScoresIncomplete,
}

impl KeypadPage {
    fn max_val(&self) -> u16 {
        match self {
            Self::AddScore(_) | Self::Penalty(_, _, _) => 99,
            Self::GameNumber => 9999,
            Self::TeamTimeouts(_) => 999,
        }
    }

    fn text(&self) -> &'static str {
        match self {
            Self::AddScore(_) | Self::Penalty(_, _, _) => "PLAYER\nNUMBER:",
            Self::GameNumber => "GAME\nNUMBER:",
            Self::TeamTimeouts(_) => "NUM T/Os\nPER HALF:",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthParameter {
    Half,
    HalfTime,
    NominalBetweenGame,
    MinimumBetweenGame,
    PreOvertime,
    OvertimeHalf,
    OvertimeHalfTime,
    PreSuddenDeath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListableParameter {
    Tournament,
    Pool,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolGameParameter {
    OvertimeAllowed,
    SuddenDeathAllowed,
    WhiteOnRight,
    UsingUwhScores,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOption {
    Black,
    White,
    GameParameter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Init,
    NewSnapshot(GameSnapshot),
    EditTime,
    ChangeTime {
        increase: bool,
        secs: u64,
        timeout: bool,
    },
    TimeEditComplete {
        canceled: bool,
    },
    StartPlayNow,
    EditScores,
    ChangeScore {
        color: GameColor,
        increase: bool,
    },
    ScoreEditComplete {
        canceled: bool,
    },
    PenaltyOverview,
    Scroll {
        which: ScrollOption,
        up: bool,
    },
    PenaltyOverviewComplete {
        canceled: bool,
    },
    ChangeKind(PenaltyKind),
    PenaltyEditComplete {
        canceled: bool,
        deleted: bool,
    },
    KeypadPage(KeypadPage),
    KeypadButtonPress(KeypadButton),
    ChangeColor(GameColor),
    AddScoreComplete {
        canceled: bool,
    },
    EditGameConfig,
    ConfigEditComplete {
        canceled: bool,
    },
    EditParameter(LengthParameter),
    SelectParameter(ListableParameter),
    ParameterEditComplete {
        canceled: bool,
    },
    ParameterSelected(ListableParameter, usize),
    ToggleBoolParameter(BoolGameParameter),
    ConfirmationSelected(ConfirmationOption),
    BlackTimeout(bool),
    WhiteTimeout(bool),
    RefTimeout(bool),
    PenaltyShot(bool),
    EndTimeout,
    ConfirmScores(GameSnapshot),
    ScoreConfirmation {
        correct: bool,
    },
    RecvTournamentList(Vec<TournamentInfo>),
    RecvTournament(TournamentInfo),
    RecvGameList(Vec<GameInfo>),
    RecvGame(GameInfo),
    NoAction, // TODO: Remove once UI is functional
}

impl Message {
    fn is_repeatable(&self) -> bool {
        match self {
            Self::NewSnapshot(_)
            | Self::ChangeTime { .. }
            | Self::ChangeScore { .. }
            | Self::Scroll { .. }
            | Self::KeypadButtonPress(_)
            | Self::ToggleBoolParameter(_)
            | Self::RecvTournamentList(_)
            | Self::RecvTournament(_)
            | Self::RecvGameList(_)
            | Self::RecvGame(_)
            | Self::NoAction => true,

            Self::Init
            | Self::EditTime
            | Self::TimeEditComplete { .. }
            | Self::StartPlayNow
            | Self::EditScores
            | Self::ScoreEditComplete { .. }
            | Self::PenaltyOverview
            | Self::PenaltyOverviewComplete { .. }
            | Self::ChangeKind(_)
            | Self::PenaltyEditComplete { .. }
            | Self::KeypadPage(_)
            | Self::ChangeColor(_)
            | Self::AddScoreComplete { .. }
            | Self::EditGameConfig
            | Self::ConfigEditComplete { .. }
            | Self::EditParameter(_)
            | Self::SelectParameter(_)
            | Self::ParameterEditComplete { .. }
            | Self::ParameterSelected(_, _)
            | Self::ConfirmationSelected(_)
            | Self::BlackTimeout(_)
            | Self::WhiteTimeout(_)
            | Self::RefTimeout(_)
            | Self::PenaltyShot(_)
            | Self::EndTimeout
            | Self::ConfirmScores(_)
            | Self::ScoreConfirmation { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypadButton {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationOption {
    DiscardChanges,
    GoBack,
    EndGameAndApply,
    KeepGameAndApply,
}

impl RefBoxApp {
    fn apply_snapshot(&mut self, mut new_snapshot: GameSnapshot) {
        if new_snapshot.current_period != self.snapshot.current_period {
            if new_snapshot.current_period == GamePeriod::BetweenGames {
                self.handle_game_end(new_snapshot.next_game_number);
            } else if self.snapshot.current_period == GamePeriod::BetweenGames {
                self.handle_game_start(new_snapshot.game_number);
            }
        }
        if let Some(tid) = self.current_tid {
            new_snapshot.tournament_id = tid;
        }
        self.maybe_play_sound(&new_snapshot);
        self.update_sender
            .send_snapshot(new_snapshot.clone(), self.config.hardware.white_on_right)
            .unwrap();
        self.snapshot = new_snapshot;
    }

    fn maybe_play_sound(&self, new_snapshot: &GameSnapshot) {
        const NUM_SHORT_SOUNDS: u16 = 10;
        const SOUND_DUR: std::time::Duration = std::time::Duration::from_millis(250);
        const LOW_FREQ: f32 = 660.0;
        const MED_FREQ: f32 = 1000.0;
        const HIGH_FREQ: f32 = 1500.0;

        if let Some((_, ref handle)) = self.sound {
            let (play_short_sound, play_long_sound) = match new_snapshot.timeout {
                TimeoutSnapshot::Black(time) | TimeoutSnapshot::White(time) => {
                    match self.snapshot.timeout {
                        TimeoutSnapshot::Black(old_time) | TimeoutSnapshot::White(old_time) => (
                            time != old_time && time <= NUM_SHORT_SOUNDS,
                            time != old_time && time == 20,
                        ),
                        _ => (false, false),
                    }
                }
                TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => (false, false),
                TimeoutSnapshot::None => {
                    let prereqs = new_snapshot.current_period != GamePeriod::SuddenDeath
                        && new_snapshot.secs_in_period != self.snapshot.secs_in_period;

                    let is_warn_period = match new_snapshot.current_period {
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

                    (
                        prereqs && new_snapshot.secs_in_period <= NUM_SHORT_SOUNDS as u32,
                        prereqs && is_warn_period && new_snapshot.secs_in_period == 35,
                    )
                }
            };

            if play_long_sound {
                let sink = match Sink::try_new(handle) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to play long sound: {e}");
                        return;
                    }
                };

                sink.append(SineWave::new(LOW_FREQ).take_duration(SOUND_DUR));
                sink.append(SineWave::new(MED_FREQ).take_duration(SOUND_DUR));
                sink.append(SineWave::new(HIGH_FREQ).take_duration(SOUND_DUR));
                sink.append(Zero::<f32>::new(1, 48_000).take_duration(SOUND_DUR));
                sink.append(SineWave::new(LOW_FREQ).take_duration(SOUND_DUR));
                sink.append(SineWave::new(MED_FREQ).take_duration(SOUND_DUR));
                sink.append(SineWave::new(HIGH_FREQ).take_duration(SOUND_DUR));
                sink.detach();
            } else if play_short_sound {
                if let Err(e) = handle.play_raw(SineWave::new(LOW_FREQ).take_duration(SOUND_DUR)) {
                    error!("Failed to play short sound: {e}");
                };
            }
        }
    }

    fn do_get_request<T, F>(&self, url: String, short_name: String, on_success: F)
    where
        T: serde::de::DeserializeOwned,
        F: Fn(T) -> Message + Send + Sync + 'static,
    {
        if let Some(client) = &self.client {
            info!("Starting request for {short_name}");
            let request = client.request(Method::GET, url).build().unwrap();
            let client_ = client.clone();
            let msg_tx_ = self.msg_tx.clone();
            task::spawn(async move {
                let mut msg = None;
                for _ in 0..MAX_RETRIES {
                    msg = match client_.execute(request.try_clone().unwrap()).await {
                        Ok(resp) => {
                            if resp.status() != StatusCode::OK {
                                error!(
                                    "Got bad status code from uwhscores when requesting {}: {}",
                                    short_name,
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                continue;
                            } else {
                                match resp.json::<T>().await {
                                    Ok(parsed) => Some(on_success(parsed)),
                                    Err(e) => {
                                        error!("Couldn't desesrialize {}: {e}", short_name);
                                        None
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Request for {} failed: {e}", short_name);
                            info!("Maybe retrying");
                            continue;
                        }
                    };
                    break;
                }

                if let Some(msg) = msg {
                    msg_tx_.send(msg).unwrap();
                } else {
                    error!("Too many failures when requesting {short_name}, stopping");
                }
            });
        }
    }

    fn request_tournament_list(&self) {
        let url = format!("{}tournaments", self.config.uwhscores.url);
        self.do_get_request(
            url,
            "tournament list".to_string(),
            |parsed: TournamentListResponse| Message::RecvTournamentList(parsed.tournaments),
        );
    }

    fn request_tournament_details(&self, tid: u32) {
        let url = format!("{}tournaments/{tid}", self.config.uwhscores.url);
        self.do_get_request(
            url,
            format!("tournament details for tid {tid}"),
            |parsed: TournamentSingleResponse| Message::RecvTournament(parsed.tournament),
        );
    }

    fn request_game_list(&self, tid: u32) {
        let url = format!("{}tournaments/{tid}/games", self.config.uwhscores.url);
        self.do_get_request(
            url,
            format!("game list for tid {tid}"),
            |parsed: GameListResponse| Message::RecvGameList(parsed.games),
        );
    }

    fn request_game_details(&self, tid: u32, gid: u32) {
        let url = format!("{}tournaments/{tid}/games/{gid}", self.config.uwhscores.url);
        self.do_get_request(
            url,
            format!("game deatils for tid {tid} and gid {gid}"),
            |parsed: GameSingleResponse| Message::RecvGame(parsed.game),
        );
    }

    fn post_game_score(&self, game: &GameInfo, scores: BlackWhiteBundle<u8>) {
        if let Some(client) = &self.client {
            let tid = game.tid;
            let gid = game.gid;
            let post_url = format!("{}tournaments/{tid}/games/{gid}", self.config.uwhscores.url);

            info!("Starting login request");

            let login_request = client
                .request(Method::GET, format!("{}login", self.config.uwhscores.url))
                .basic_auth(
                    self.config.uwhscores.email.clone(),
                    Some(self.config.uwhscores.password.clone()),
                )
                .build()
                .unwrap();

            let post_data = GameScorePostData::new(GameScoreInfo {
                tid,
                gid,
                score_b: scores.black,
                score_w: scores.white,
                black_id: game.black_id,
                white_id: game.white_id,
            });

            let client_ = client.clone();
            task::spawn(async move {
                let mut login = None;
                for _ in 0..MAX_RETRIES {
                    login = match client_.execute(login_request.try_clone().unwrap()).await {
                        Ok(resp) => {
                            if resp.status() != StatusCode::OK {
                                error!(
                                    "Got bad status code from uwhscores when logging in: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                continue;
                            } else {
                                match resp.json::<LoginResponse>().await {
                                    Ok(parsed) => Some(parsed),
                                    Err(e) => {
                                        error!("Couldn't desesrialize login: {e}");
                                        return;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Login request failed: {e}");
                            info!("Maybe retrying");
                            continue;
                        }
                    };
                    break;
                }

                let login = if let Some(l) = login {
                    l
                } else {
                    error!("Too many failures when logging in to uwhscores, stopping");
                    return;
                };

                info!("Posting score: {post_data:?}");

                let post_request = client_
                    .request(Method::POST, post_url)
                    .basic_auth::<_, String>(login.token, None)
                    .json(&post_data)
                    .build()
                    .unwrap();

                for _ in 0..MAX_RETRIES {
                    match client_.execute(post_request.try_clone().unwrap()).await {
                        Ok(resp) => {
                            if resp.status() != StatusCode::OK {
                                error!(
                                    "Got bad status code from uwhscores when posting score: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                continue;
                            }
                        }
                        Err(e) => {
                            error!("Post score request failed: {e}");
                            info!("Maybe retrying");
                            continue;
                        }
                    };
                    break;
                }
            });
        }
    }

    fn handle_game_start(&mut self, new_game_num: u32) {
        if self.using_uwhscores {
            if let (Some(ref games), Some(ref pool)) = (&self.games, &self.current_pool) {
                let this_game_start = match games.get(&new_game_num) {
                    Some(g) => g.start_time,
                    None => {
                        error!("Could not find new game's start time (gid {new_game_num}");
                        return;
                    }
                };

                let next_game = games
                    .values()
                    .filter(|game| game.pool == *pool)
                    .filter(|game| game.start_time > this_game_start)
                    .min_by_key(|game| game.start_time);

                let mut tm = self.tm.lock().unwrap();
                if let Some(next_game) = next_game {
                    let info = NextGameInfo {
                        number: next_game.gid,
                        timing: next_game.timing_rules.clone(),
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

    fn handle_game_end(&self, next_game_num: u32) {
        if self.using_uwhscores {
            if let Some(tid) = self.current_tid {
                self.request_game_details(tid, next_game_num);
            } else {
                error!("Missing current tid to request game info");
            }
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
        } = flags;

        let sound = match OutputStream::try_default() {
            Ok(res) => Some(res),
            Err(e) => {
                error!("Failed to connect to sound output: {e}");
                None
            }
        };

        let (msg_tx, rx) = mpsc::unbounded_channel();
        let message_listener = MessageListener {
            rx: Arc::new(Mutex::new(Some(rx))),
        };
        msg_tx.send(Message::Init).unwrap();

        let mut tm = TournamentManager::new(config.game.clone());
        tm.set_timezone(config.uwhscores.timezone);
        tm.start_clock(Instant::now());

        let client = match Client::builder()
            .https_only(require_https)
            .timeout(REQUEST_TIMEOUT)
            .build()
        {
            Ok(c) => Some(c),
            Err(e) => {
                error!("Failed to start HTTP Client: {e}");
                None
            }
        };

        let clock_running_receiver = tm.get_start_stop_rx();

        let tm = Arc::new(Mutex::new(tm));

        let update_sender = UpdateSender::new(serial_ports, binary_port, json_port);

        let snapshot = Default::default();

        (
            Self {
                pen_edit: PenaltyEditor::new(tm.clone()),
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
                client,
                using_uwhscores: false,
                tournaments: None,
                games: None,
                current_tid: None,
                current_pool: None,
                sound,
                sim_child,
                fullscreen,
            },
            Command::none(),
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

    fn mode(&self) -> iced::window::Mode {
        if self.fullscreen {
            iced::window::Mode::Fullscreen
        } else {
            iced::window::Mode::Windowed
        }
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

        match message {
            Message::Init => self.request_tournament_list(),
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
                    AppState::KeypadPage(KeypadPage::TeamTimeouts(ref mut dur), _) => (dur, false),
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
                    if !canceled {
                        tm.set_game_clock_time(game_time).unwrap();
                        if let Some(time) = timeout_time {
                            tm.set_timeout_clock_time(time).unwrap();
                        }
                    }
                    if was_running {
                        let now = Instant::now();
                        tm.start_clock(now);
                        tm.update(now).unwrap();
                    }
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
                    scores: BlackWhiteBundle {
                        black: tm.get_b_score(),
                        white: tm.get_w_score(),
                    },
                    is_confirmation: false,
                };
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
                let now = Instant::now();

                self.app_state = if let AppState::ScoreEdit {
                    scores,
                    is_confirmation,
                } = self.app_state
                {
                    if is_confirmation {
                        if let Some(game) = self
                            .games
                            .as_ref()
                            .and_then(|games| games.get(&tm.game_number()))
                        {
                            self.post_game_score(game, scores);
                        }

                        tm.set_scores(scores.black, scores.white, now);
                        tm.start_clock(now);
                        tm.update(now + Duration::from_millis(2)).unwrap(); // Need to update after game ends
                        AppState::MainPage
                    } else if !canceled {
                        if tm.current_period() == GamePeriod::SuddenDeath
                            && (scores.black != scores.white)
                        {
                            tm.stop_clock(now).unwrap();
                            AppState::ConfirmScores(scores)
                        } else {
                            tm.set_scores(scores.black, scores.white, now);
                            AppState::MainPage
                        }
                    } else {
                        AppState::MainPage
                    }
                } else {
                    unreachable!()
                };

                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot);

                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::PenaltyOverview => {
                self.pen_edit.start_session().unwrap();
                self.app_state = AppState::PenaltyOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::Scroll { which, up } => {
                if let AppState::PenaltyOverview(ref mut indices) = self.app_state {
                    let idx = match which {
                        ScrollOption::Black => &mut indices.black,
                        ScrollOption::White => &mut indices.white,
                        ScrollOption::GameParameter => unreachable!(),
                    };
                    if up {
                        *idx = idx.saturating_sub(1);
                    } else {
                        *idx = idx.saturating_add(1);
                    }
                } else if let AppState::ParameterList(_, ref mut idx) = self.app_state {
                    debug_assert_eq!(which, ScrollOption::GameParameter);
                    if up {
                        *idx = idx.saturating_sub(1);
                    } else {
                        *idx = idx.saturating_add(1);
                    }
                } else {
                    unreachable!();
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::PenaltyOverviewComplete { canceled } => {
                if canceled {
                    self.pen_edit.abort_session();
                    self.app_state = AppState::MainPage;
                } else if let Err(e) = self.pen_edit.apply_changes(Instant::now()) {
                    let err_string = match e {
                        PenaltyEditorError::ListTooLong(colors) => format!("The {colors} penalty list(s) \
                            is/are too long. Some penalties will not be visible on the main page."),
                        e => format!("An error occurred while applying the changes to the penalties. \
                            Some of the changes may have been applied. Please retry any remaining changes.\n\n\
                            Error Message:\n{e}"),
                    };
                    error!("{err_string}");
                    self.pen_edit.abort_session();
                    self.app_state =
                        AppState::ConfirmationPage(ConfirmationKind::Error(err_string));
                } else {
                    self.app_state = AppState::MainPage;
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ChangeKind(new_kind) => {
                if let AppState::KeypadPage(KeypadPage::Penalty(_, _, ref mut kind), _) =
                    self.app_state
                {
                    *kind = new_kind;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::PenaltyEditComplete { canceled, deleted } => {
                if !canceled {
                    if let AppState::KeypadPage(
                        KeypadPage::Penalty(origin, color, kind),
                        player_num,
                    ) = self.app_state
                    {
                        if deleted {
                            if let Some((old_color, index)) = origin {
                                self.pen_edit.delete_penalty(old_color, index).unwrap();
                            } else {
                                unreachable!();
                            }
                        } else {
                            let player_num = player_num.try_into().unwrap();
                            if let Some((old_color, index)) = origin {
                                self.pen_edit
                                    .edit_penalty(old_color, index, color, player_num, kind)
                                    .unwrap();
                            } else {
                                self.pen_edit.add_penalty(color, player_num, kind).unwrap();
                            }
                        }
                    } else {
                        unreachable!();
                    }
                }
                self.app_state = AppState::PenaltyOverview(BlackWhiteBundle { black: 0, white: 0 });
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::KeypadPage(page) => {
                let init_val = match page {
                    KeypadPage::AddScore(_) | KeypadPage::Penalty(None, _, _) => 0,
                    KeypadPage::Penalty(Some((color, index)), _, _) => {
                        self.pen_edit
                            .get_penalty(color, index)
                            .unwrap()
                            .player_number as u16
                    }
                    KeypadPage::TeamTimeouts(_) => self.config.game.team_timeouts_per_half,
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
                    | AppState::KeypadPage(KeypadPage::Penalty(_, ref mut color, _), _) => {
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
                            let mut scores = BlackWhiteBundle {
                                black: tm.get_b_score(),
                                white: tm.get_w_score(),
                            };
                            scores[color] = scores[color].saturating_add(1);

                            AppState::ConfirmScores(scores)
                        } else {
                            match color {
                                GameColor::Black => tm.add_b_score(player.try_into().unwrap(), now),
                                GameColor::White => tm.add_w_score(player.try_into().unwrap(), now),
                            };
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
            Message::EditGameConfig => {
                let edited_settings = EditableSettings {
                    config: self.tm.lock().unwrap().config().clone(),
                    game_number: if self.snapshot.current_period == GamePeriod::BetweenGames {
                        self.snapshot.next_game_number
                    } else {
                        self.snapshot.game_number
                    },
                    white_on_right: self.config.hardware.white_on_right,
                    using_uwhscores: self.using_uwhscores,
                    current_tid: self.current_tid,
                    current_pool: self.current_pool.clone(),
                    games: self.games.clone(),
                };

                self.edited_settings = Some(edited_settings);

                self.app_state = AppState::EditGameConfig;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ConfigEditComplete { canceled } => {
                let mut tm = self.tm.lock().unwrap();

                let edited_settings = self.edited_settings.as_mut().unwrap();

                let mut uwhscores_incomplete = edited_settings.using_uwhscores
                    && (edited_settings.current_tid.is_none()
                        || edited_settings.current_pool.is_none()
                        || edited_settings.games.is_none());
                if edited_settings.using_uwhscores && !uwhscores_incomplete {
                    match edited_settings
                        .games
                        .as_ref()
                        .unwrap()
                        .get(&edited_settings.game_number)
                    {
                        Some(g) => {
                            uwhscores_incomplete =
                                g.pool != *edited_settings.current_pool.as_ref().unwrap()
                        }
                        None => uwhscores_incomplete = true,
                    };
                }

                self.app_state = if !canceled {
                    let new_config = if edited_settings.using_uwhscores && !uwhscores_incomplete {
                        match edited_settings
                            .games
                            .as_ref()
                            .unwrap()
                            .get(&edited_settings.game_number)
                            .unwrap()
                            .timing_rules
                        {
                            Some(ref rules) => rules.clone().into(),
                            None => tm.config().clone(),
                        }
                    } else {
                        edited_settings.config.clone()
                    };

                    if uwhscores_incomplete {
                        AppState::ConfirmationPage(ConfirmationKind::UwhScoresIncomplete)
                    } else if new_config != *tm.config() {
                        if tm.current_period() != GamePeriod::BetweenGames {
                            AppState::ConfirmationPage(ConfirmationKind::GameConfigChanged)
                        } else {
                            tm.set_config(new_config.clone()).unwrap();
                            self.config.game = new_config;

                            let game = edited_settings
                                .games
                                .as_ref()
                                .and_then(|games| games.get(&edited_settings.game_number));
                            let timing = game.and_then(|g| g.timing_rules.clone());
                            let start_time = game.map(|g| g.start_time);

                            tm.set_next_game(NextGameInfo {
                                number: edited_settings.game_number,
                                timing,
                                start_time,
                            });

                            if edited_settings.using_uwhscores {
                                tm.apply_next_game_start(Instant::now()).unwrap();
                            } else {
                                tm.clear_scheduled_game_start();
                            }

                            let edited_settings = self.edited_settings.take().unwrap();
                            self.config.hardware.white_on_right = edited_settings.white_on_right;
                            self.using_uwhscores = edited_settings.using_uwhscores;
                            self.current_tid = edited_settings.current_tid;
                            self.current_pool = edited_settings.current_pool;
                            self.games = edited_settings.games;

                            confy::store(APP_CONFIG_NAME, None, &self.config).unwrap();
                            AppState::MainPage
                        }
                    } else if edited_settings.game_number != self.snapshot.game_number {
                        if tm.current_period() != GamePeriod::BetweenGames {
                            AppState::ConfirmationPage(ConfirmationKind::GameNumberChanged)
                        } else {
                            let edited_settings = self.edited_settings.take().unwrap();
                            self.config.hardware.white_on_right = edited_settings.white_on_right;
                            self.using_uwhscores = edited_settings.using_uwhscores;
                            self.current_tid = edited_settings.current_tid;
                            self.current_pool = edited_settings.current_pool;
                            self.games = edited_settings.games;

                            confy::store(APP_CONFIG_NAME, None, &self.config).unwrap();

                            let next_game_info = if edited_settings.using_uwhscores {
                                NextGameInfo {
                                    number: edited_settings.game_number,
                                    timing: self.games.as_ref().and_then(|games| {
                                        games
                                            .get(&edited_settings.game_number)?
                                            .timing_rules
                                            .clone()
                                    }),
                                    start_time: self.games.as_ref().and_then(|games| {
                                        Some(games.get(&edited_settings.game_number)?.start_time)
                                    }),
                                }
                            } else {
                                NextGameInfo {
                                    number: edited_settings.game_number,
                                    timing: None,
                                    start_time: None,
                                }
                            };

                            tm.set_next_game(next_game_info);

                            if edited_settings.using_uwhscores {
                                tm.apply_next_game_start(Instant::now()).unwrap();
                            }

                            AppState::MainPage
                        }
                    } else {
                        let edited_settings = self.edited_settings.take().unwrap();
                        self.config.hardware.white_on_right = edited_settings.white_on_right;
                        self.using_uwhscores = edited_settings.using_uwhscores;
                        self.current_tid = edited_settings.current_tid;
                        self.current_pool = edited_settings.current_pool;
                        self.games = edited_settings.games;

                        confy::store(APP_CONFIG_NAME, None, &self.config).unwrap();
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
                    ListableParameter::Tournament => self.current_tid.and_then(|cur_tid| {
                        self.tournaments
                            .as_ref()?
                            .iter()
                            .enumerate()
                            .find(|(_, (tid, _))| **tid == cur_tid)
                            .map(|(i, _)| i)
                    }),
                    ListableParameter::Pool => self.current_pool.as_ref().and_then(|cur_pool| {
                        self.tournaments
                            .as_ref()?
                            .get(&self.current_tid?)?
                            .pools
                            .as_ref()?
                            .iter()
                            .enumerate()
                            .find(|(_, pool)| **pool == *cur_pool)
                            .map(|(i, _)| i)
                    }),
                    ListableParameter::Game => self.games.as_ref().and_then(|games| {
                        let pool = self
                            .edited_settings
                            .as_ref()
                            .and_then(|edit| edit.current_pool.clone())?;

                        games
                            .iter()
                            .filter(|(_, game)| game.pool == pool)
                            .enumerate()
                            .find(|(_, (gid, _))| {
                                **gid == self.edited_settings.as_ref().unwrap().game_number
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
                        AppState::KeypadPage(KeypadPage::TeamTimeouts(len), num) => {
                            edited_settings.config.team_timeout_duration = len;
                            edited_settings.config.team_timeouts_per_half = num;
                        }
                        _ => unreachable!(),
                    }
                }

                self.app_state = AppState::EditGameConfig;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ParameterSelected(param, val) => {
                let edited_settings = self.edited_settings.as_mut().unwrap();
                match param {
                    ListableParameter::Tournament => {
                        edited_settings.current_tid = Some(val as u32);
                        self.request_tournament_details(val as u32);
                        self.request_game_list(val as u32);
                    }
                    ListableParameter::Pool => {
                        edited_settings.current_pool = Some(
                            self.tournaments
                                .as_ref()
                                .unwrap()
                                .get(&edited_settings.current_tid.unwrap())
                                .unwrap()
                                .pools
                                .as_ref()
                                .unwrap()[val]
                                .clone(),
                        )
                    }
                    ListableParameter::Game => edited_settings.game_number = val as u32,
                };

                self.app_state = AppState::EditGameConfig;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ToggleBoolParameter(param) => {
                let edited_settings = self.edited_settings.as_mut().unwrap();
                match param {
                    BoolGameParameter::OvertimeAllowed => {
                        edited_settings.config.overtime_allowed ^= true
                    }
                    BoolGameParameter::SuddenDeathAllowed => {
                        edited_settings.config.sudden_death_allowed ^= true
                    }
                    BoolGameParameter::WhiteOnRight => edited_settings.white_on_right ^= true,
                    BoolGameParameter::UsingUwhScores => edited_settings.using_uwhscores ^= true,
                }
            }
            Message::ConfirmationSelected(selection) => {
                let config_changed = if let AppState::ConfirmationPage(ref kind) = self.app_state {
                    kind == &ConfirmationKind::GameConfigChanged
                } else {
                    unreachable!()
                };

                self.app_state = match selection {
                    ConfirmationOption::DiscardChanges => AppState::MainPage,
                    ConfirmationOption::GoBack => AppState::EditGameConfig,
                    ConfirmationOption::EndGameAndApply => {
                        let edited_settings = self.edited_settings.take().unwrap();
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();
                        tm.reset_game(now);
                        if config_changed {
                            tm.set_config(self.config.game.clone()).unwrap();
                        }

                        let game = edited_settings
                            .games
                            .as_ref()
                            .and_then(|games| games.get(&edited_settings.game_number));
                        let timing = game.and_then(|g| g.timing_rules.clone());
                        let start_time = game.map(|g| g.start_time);

                        tm.set_next_game(NextGameInfo {
                            number: edited_settings.game_number,
                            timing,
                            start_time,
                        });

                        if edited_settings.using_uwhscores {
                            tm.apply_next_game_start(Instant::now()).unwrap();
                        } else {
                            tm.clear_scheduled_game_start();
                        }

                        self.config.hardware.white_on_right = edited_settings.white_on_right;
                        self.using_uwhscores = edited_settings.using_uwhscores;
                        self.current_tid = edited_settings.current_tid;
                        self.current_pool = edited_settings.current_pool;
                        self.games = edited_settings.games;

                        confy::store(APP_CONFIG_NAME, None, &self.config).unwrap();
                        let snapshot = tm.generate_snapshot(now).unwrap();
                        std::mem::drop(tm);
                        self.apply_snapshot(snapshot);
                        AppState::MainPage
                    }
                    ConfirmationOption::KeepGameAndApply => {
                        let edited_settings = self.edited_settings.take().unwrap();
                        let mut tm = self.tm.lock().unwrap();
                        tm.set_game_number(edited_settings.game_number);
                        let snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                        std::mem::drop(tm);

                        self.config.hardware.white_on_right = edited_settings.white_on_right;
                        self.using_uwhscores = edited_settings.using_uwhscores;
                        self.current_tid = edited_settings.current_tid;
                        self.current_pool = edited_settings.current_pool;
                        self.games = edited_settings.games;

                        confy::store(APP_CONFIG_NAME, None, &self.config).unwrap();
                        self.apply_snapshot(snapshot);
                        AppState::MainPage
                    }
                };
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ConfirmScores(snapshot) => {
                self.apply_snapshot(snapshot);

                let scores = BlackWhiteBundle {
                    black: self.snapshot.b_score,
                    white: self.snapshot.w_score,
                };

                self.app_state = AppState::ConfirmScores(scores);
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ScoreConfirmation { correct } => {
                self.app_state = if let AppState::ConfirmScores(scores) = self.app_state {
                    if correct {
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();

                        if let Some(game) = self
                            .games
                            .as_ref()
                            .and_then(|games| games.get(&tm.game_number()))
                        {
                            self.post_game_score(game, scores);
                        }

                        tm.set_scores(scores.black, scores.white, now);
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
            Message::BlackTimeout(switch) => {
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                if switch {
                    tm.switch_to_b_timeout().unwrap();
                } else {
                    tm.start_b_timeout(now).unwrap();
                }
                if let AppState::TimeEdit(_, _, ref mut time) = self.app_state {
                    *time = Some(tm.timeout_clock_time(now).unwrap());
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot);
            }
            Message::WhiteTimeout(switch) => {
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                if switch {
                    tm.switch_to_w_timeout().unwrap();
                } else {
                    tm.start_w_timeout(now).unwrap();
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
                    tm.switch_to_ref_timeout().unwrap();
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
                    tm.switch_to_penalty_shot().unwrap();
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
                tm.end_timeout(Instant::now()).unwrap();
                let snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot);
            }
            Message::RecvTournamentList(t_list) => {
                let t_map = t_list
                    .into_iter()
                    .filter(|t| t.is_active == 1)
                    .map(|t| (t.tid, t))
                    .collect();
                self.tournaments = Some(t_map);
            }
            Message::RecvTournament(tournament) => {
                if let Some(tid) = self.current_tid.or_else(|| {
                    self.edited_settings
                        .as_ref()
                        .and_then(|edits| edits.current_tid)
                }) {
                    if tid != tournament.tid {
                        warn!(
                            "Received tournament data, but for the wrong tid: {}",
                            tournament.tid
                        )
                    }
                } else {
                    warn!("Received tournament data, but there is no current tid")
                }

                if let Some(ref mut tournaments) = self.tournaments {
                    tournaments.insert(tournament.tid, tournament);
                } else {
                    warn!(
                        "Received info for tid {}, but there is no tournament list yet",
                        tournament.tid
                    );
                    self.tournaments = Some(BTreeMap::from([(tournament.tid, tournament)]));
                }
            }
            Message::RecvGameList(games_list) => {
                let games_map = games_list.into_iter().map(|g| (g.gid, g)).collect();
                if let Some(ref mut edits) = self.edited_settings {
                    edits.games = Some(games_map);
                } else {
                    self.games = Some(games_map);
                }
            }
            Message::RecvGame(game) => {
                if let Some(ref mut games) = self.games {
                    games.insert(game.gid, game);
                } else {
                    warn!(
                        "Received info for gid {}, but there is no game list yet",
                        game.gid
                    );
                    self.games = Some(BTreeMap::from([(game.gid, game)]));
                }
            }
            Message::NoAction => {}
        };

        Command::none()
    }

    fn background_color(&self) -> iced::Color {
        WINDOW_BACKGROUND
    }

    fn view(&self) -> Element<Message> {
        let mut main_view = column()
            .spacing(SPACING)
            .padding(PADDING)
            .push(match self.app_state {
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

                    let config = if let Some(ref c) = new_config {
                        c
                    } else {
                        &self.config.game
                    };

                    build_main_view(&self.snapshot, config, self.using_uwhscores, &self.games)
                }
                AppState::TimeEdit(_, time, timeout_time) => {
                    build_time_edit_view(&self.snapshot, time, timeout_time)
                }
                AppState::ScoreEdit {
                    scores,
                    is_confirmation,
                } => build_score_edit_view(&self.snapshot, scores, is_confirmation),
                AppState::PenaltyOverview(indices) => build_penalty_overview_page(
                    &self.snapshot,
                    self.pen_edit.get_printable_lists(Instant::now()).unwrap(),
                    indices,
                ),
                AppState::KeypadPage(page, player_num) => {
                    build_keypad_page(&self.snapshot, page, player_num)
                }
                AppState::EditGameConfig => build_game_config_edit_page(
                    &self.snapshot,
                    self.edited_settings.as_ref().unwrap(),
                    &self.tournaments,
                ),
                AppState::ParameterEditor(param, dur) => {
                    build_game_parameter_editor(&self.snapshot, param, dur)
                }
                AppState::ParameterList(param, index) => build_list_selector_page(
                    &self.snapshot,
                    param,
                    index,
                    self.edited_settings.as_ref().unwrap(),
                    &self.tournaments,
                ),
                AppState::ConfirmationPage(ref kind) => {
                    build_confirmation_page(&self.snapshot, kind)
                }
                AppState::ConfirmScores(scores) => {
                    build_score_confirmation_page(&self.snapshot, scores)
                }
            });

        match self.app_state {
            AppState::ScoreEdit {
                is_confirmation, ..
            } if is_confirmation => {}
            AppState::ConfirmScores(_) => {}
            _ => {
                main_view = main_view.push(build_timeout_ribbon(&self.snapshot, &self.tm));
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

impl<H: Hasher, I> Recipe<H, I> for TimeUpdater {
    type Output = Message;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        "TimeUpdater".hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
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
                tm.halt_clock(now).unwrap();
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

impl<H: Hasher, I> Recipe<H, I> for MessageListener {
    type Output = Message;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        "MessageListener".hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
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

// impl Clone for MessageListener {
//     fn clone(&self) -> Self {
//         Self { rx: None }
//     }
// }
