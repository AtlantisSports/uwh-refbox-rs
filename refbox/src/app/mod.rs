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
use reqwest::{Client, Method, StatusCode};
use std::{
    borrow::Cow,
    cmp::min,
    collections::BTreeMap,
    pin::Pin,
    process::Child,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, watch},
    task,
    time::{Duration, Instant, sleep_until, timeout_at},
};
use tokio_serial::SerialPortBuilder;
use uwh_common::{
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{Color, GamePeriod, GameSnapshot, Infraction, TimeoutSnapshot},
    uwhportal::UwhPortalClient,
    uwhscores::*,
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
const MAX_RETRIES: usize = 6;

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
    client: Option<Client>,
    uwhscores_token: Arc<Mutex<Option<String>>>,
    uwhscores_auth_valid_for: Option<Vec<u32>>,
    uwhportal_client: Option<UwhPortalClient>,
    using_uwhscores: bool,
    tournaments: Option<BTreeMap<u32, TournamentInfo>>,
    games: Option<BTreeMap<u32, GameInfo>>,
    current_tid: Option<u32>,
    current_pool: Option<String>,
    sound: SoundController,
    sim_child: Option<Child>,
    fullscreen: bool,
    list_all_tournaments: bool,
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
    pub list_all_tournaments: bool,
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
    UwhScoresIncomplete,
}

impl RefBoxApp {
    fn apply_snapshot(&mut self, mut new_snapshot: GameSnapshot) {
        if new_snapshot.current_period != self.snapshot.current_period {
            if new_snapshot.current_period == GamePeriod::BetweenGames {
                self.handle_game_end(new_snapshot.game_number, new_snapshot.next_game_number);
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
        let (play_whistle, play_buzzer) = match new_snapshot.timeout {
            TimeoutSnapshot::Black(time) | TimeoutSnapshot::White(time) => {
                match self.snapshot.timeout {
                    TimeoutSnapshot::Black(old_time) | TimeoutSnapshot::White(old_time) => (
                        time != old_time && time == 15,
                        time != old_time && time == 0,
                    ),
                    _ => (false, false),
                }
            }
            TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => (false, false),
            TimeoutSnapshot::None => {
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

    fn do_get_request<T, F>(
        &self,
        url: String,
        short_name: String,
        on_success: F,
        on_error: Option<Message>,
    ) where
        T: serde::de::DeserializeOwned,
        F: Fn(T) -> Message + Send + Sync + 'static,
    {
        if let Some(client) = &self.client {
            info!("Starting request for {short_name}");
            let mut request = client.request(Method::GET, url);
            if let Some(token) = self.uwhscores_token.lock().unwrap().as_deref() {
                if !token.is_empty() {
                    request = request.basic_auth::<_, String>(token.to_string(), None);
                }
            }
            let request = request.build().unwrap();
            let client_ = client.clone();
            let msg_tx_ = self.msg_tx.clone();

            let mut delay_until = None;

            task::spawn(async move {
                let mut msg = None;
                for _ in 0..MAX_RETRIES {
                    if let Some(time) = delay_until.take() {
                        sleep_until(time).await;
                    }

                    let start = Instant::now();
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
                            delay_until = Some(start + REQUEST_TIMEOUT);
                            continue;
                        }
                    };
                    break;
                }

                if let Some(msg) = msg {
                    msg_tx_.send(msg).unwrap();
                } else {
                    error!("Too many failures when requesting {short_name}, stopping");
                    if let Some(on_error) = on_error {
                        msg_tx_.send(on_error).unwrap();
                    }
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
            None,
        );
    }

    fn request_tournament_details(&self, tid: u32) {
        let url = format!("{}tournaments/{tid}", self.config.uwhscores.url);
        self.do_get_request(
            url,
            format!("tournament details for tid {tid}"),
            |parsed: TournamentSingleResponse| Message::RecvTournament(parsed.tournament),
            None,
        );
    }

    fn request_game_list(&self, tid: u32) {
        let url = format!("{}tournaments/{tid}/games", self.config.uwhscores.url);
        self.do_get_request(
            url,
            format!("game list for tid {tid}"),
            |parsed: GameListResponse| Message::RecvGameList(parsed.games),
            None,
        );
    }

    fn request_game_details(&self, tid: u32, gid: u32) {
        let url = format!("{}tournaments/{tid}/games/{gid}", self.config.uwhscores.url);
        self.do_get_request(
            url,
            format!("game deatils for tid {tid} and gid {gid}"),
            |parsed: GameSingleResponse| Message::RecvGame(parsed.game),
            None,
        );
    }

    fn uwhscores_login(&self) -> Option<Pin<Box<impl std::future::Future<Output = ()> + use<>>>> {
        if let Some(client) = &self.client {
            let client_ = client.clone();
            let login_request = client_
                .request(Method::GET, format!("{}login", self.config.uwhscores.url))
                .basic_auth(
                    self.config.uwhscores.email.clone(),
                    Some(self.config.uwhscores.password.clone()),
                )
                .build()
                .unwrap();

            let uwhscores_token = self.uwhscores_token.clone();
            Some(Box::pin(async move {
                info!("Starting login request");

                for _ in 0..MAX_RETRIES {
                    match client_.execute(login_request.try_clone().unwrap()).await {
                        Ok(resp) => {
                            if resp.status() != StatusCode::OK {
                                error!(
                                    "Got bad status code from uwhscores when logging in: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                *uwhscores_token.lock().unwrap() = Some(String::new());
                                continue;
                            } else {
                                match resp.json::<LoginResponse>().await {
                                    Ok(parsed) => {
                                        info!("Successfully logged in to uwhscores");
                                        *uwhscores_token.lock().unwrap() = Some(parsed.token);
                                        return;
                                    }
                                    Err(e) => {
                                        error!("Couldn't deserialize login: {e}");
                                        *uwhscores_token.lock().unwrap() = Some(String::new());
                                        return;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Login request failed: {e}");
                            info!("Maybe retrying");
                            *uwhscores_token.lock().unwrap() = Some(String::new());
                            continue;
                        }
                    };
                }
            }))
        } else {
            None
        }
    }

    fn post_game_score(&self, game: &GameInfo, scores: BlackWhiteBundle<u8>) {
        if let Some(client) = &self.client {
            let tid = game.tid;
            let gid = game.gid;
            let post_url = format!("{}tournaments/{tid}/games/{gid}", self.config.uwhscores.url);

            let login_request = self.uwhscores_login().unwrap();
            let mut login_request_2 = Some(self.uwhscores_login().unwrap());

            let post_data = GameScorePostData::new(GameScoreInfo {
                tid,
                gid,
                score_b: scores.black,
                score_w: scores.white,
                black_id: game.black_id,
                white_id: game.white_id,
            });

            let post_request = client
                .request(Method::POST, post_url.clone())
                .json(&post_data);

            let client_ = client.clone();
            let uwhscores_token = self.uwhscores_token.clone();
            task::spawn(async move {
                let token = uwhscores_token.lock().unwrap().clone();

                let mut token = match token {
                    Some(t) if !t.is_empty() => t,
                    _ => {
                        login_request.await;
                        if let Some(t) = uwhscores_token.lock().unwrap().as_deref() {
                            t.to_string()
                        } else {
                            error!(
                                "Failed to get uwhscores token. Aborting post score: {post_data:?}"
                            );
                            return;
                        }
                    }
                };

                info!("Posting score: {post_data:?}");

                for _ in 0..MAX_RETRIES {
                    let request = post_request
                        .try_clone()
                        .unwrap()
                        .basic_auth::<_, String>(token.clone(), None)
                        .build()
                        .unwrap();

                    match client_.execute(request).await {
                        Ok(resp) => match resp.status() {
                            StatusCode::OK => {
                                info!("Successfully posted score");
                                return;
                            }
                            StatusCode::UNAUTHORIZED => {
                                error!(
                                    "Got unauthorized status code from uwhscores when posting score: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                if let Some(f) = login_request_2.take() {
                                    f.await;
                                }
                                token = if let Some(token) =
                                    uwhscores_token.lock().unwrap().as_deref()
                                {
                                    token.to_string()
                                } else {
                                    error!(
                                        "Failed to get uwhscores token. Aborting post score: {post_data:?}"
                                    );
                                    return;
                                };
                                continue;
                            }
                            _ => {
                                error!(
                                    "Got bad status code from uwhscores when posting score: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                continue;
                            }
                        },
                        Err(e) => {
                            error!("Post score request failed: {e}");
                            info!("Maybe retrying");
                            continue;
                        }
                    };
                }
            });
        }
    }

    fn check_uwhscores_auth(&self) {
        if let Some(client) = &self.client {
            info!("Starting request for uwhscores auth");

            let login_request = self.uwhscores_login().unwrap();
            let mut login_request_2 = Some(self.uwhscores_login().unwrap());

            let user_request = client.request(
                Method::GET,
                format!("{}users/me", self.config.uwhscores.url),
            );
            let client_ = client.clone();
            let uwhscores_token = self.uwhscores_token.clone();
            let msg_tx_ = self.msg_tx.clone();

            let mut delay_until = None;

            task::spawn(async move {
                let token = uwhscores_token.lock().unwrap().clone();

                let mut token = match token {
                    Some(t) if !t.is_empty() => t,
                    _ => {
                        login_request.await;
                        if let Some(t) = uwhscores_token.lock().unwrap().as_deref() {
                            t.to_string()
                        } else {
                            error!("Failed to get uwhscores token. Aborting uwhscores auth check");
                            return;
                        }
                    }
                };

                for _ in 0..MAX_RETRIES {
                    if let Some(time) = delay_until.take() {
                        sleep_until(time).await;
                    }

                    let request = user_request
                        .try_clone()
                        .unwrap()
                        .basic_auth::<_, String>(token.clone(), None)
                        .build()
                        .unwrap();

                    let start = Instant::now();
                    match client_.execute(request).await {
                        Ok(resp) => match resp.status() {
                            StatusCode::OK => match resp.json::<UserResponse>().await {
                                Ok(parsed) => {
                                    msg_tx_
                                        .send(Message::UwhScoresAuthChecked(
                                            parsed.user.tournaments,
                                        ))
                                        .unwrap();
                                    return;
                                }
                                Err(e) => {
                                    error!("Couldn't desesrialize uwhscores auth: {e}");
                                }
                            },
                            StatusCode::UNAUTHORIZED => {
                                error!(
                                    "Got unauthorized status code from uwhscores when requesting uwhscores auth: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                if let Some(f) = login_request_2.take() {
                                    f.await;
                                }
                                token = if let Some(token) =
                                    uwhscores_token.lock().unwrap().as_deref()
                                {
                                    token.to_string()
                                } else {
                                    error!(
                                        "Failed to get uwhscores token. Aborting uwhscores auth check"
                                    );
                                    msg_tx_.send(Message::UwhScoresAuthChecked(vec![])).unwrap();
                                    return;
                                };
                                continue;
                            }
                            _ => {
                                error!(
                                    "Got bad status code from uwhscores when requesting uwhscores auth: {}",
                                    resp.status()
                                );
                                info!("Maybe retrying");
                                continue;
                            }
                        },
                        Err(e) => {
                            error!("Request for uwhscores auth failed: {e}");
                            info!("Maybe retrying");
                            delay_until = Some(start + REQUEST_TIMEOUT);
                            continue;
                        }
                    };
                    break;
                }

                error!("Too many failures when requesting uwhscores auth, stopping");
                msg_tx_.send(Message::UwhScoresAuthChecked(vec![])).unwrap();
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

    fn post_game_stats(&self, tid: u32, gid: u32, stats: String) {
        if let Some(ref uwhportal_client) = self.uwhportal_client {
            let request = uwhportal_client.post_game_stats(tid, gid, stats);
            tokio::spawn(async move {
                match request.await {
                    Ok(()) => info!("Successfully posted game stats"),
                    Err(e) => error!("Failed to post game stats: {e}"),
                }
            });
        }
    }

    fn handle_game_start(&mut self, new_game_num: u32) {
        if self.using_uwhscores {
            if let (Some(games), Some(pool)) = (&self.games, &self.current_pool) {
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

    fn handle_game_end(&self, game_number: u32, next_game_num: u32) {
        if self.using_uwhscores {
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

            if let Some(tid) = self.current_tid {
                self.request_game_details(tid, next_game_num);
                if let Some(stats) = stats.take() {
                    self.post_game_stats(tid, game_number, stats);
                }
            } else {
                error!("Missing current tid to handle game end");
            }
        }
    }

    fn apply_settings_change(&mut self) {
        let edited_settings = self.edited_settings.take().unwrap();

        let EditableSettings {
            white_on_right,
            using_uwhscores,
            current_tid,
            current_pool,
            games,
            sound,
            mode,
            collect_scorer_cap_num,
            hide_time,
            config: _config,
            game_number: _game_number,
            track_fouls_and_warnings,
            uwhscores_email: _,
            uwhscores_password: _,
            uwhportal_token: _,
        } = edited_settings;

        self.config.hardware.white_on_right = white_on_right;
        self.using_uwhscores = using_uwhscores;
        self.current_tid = current_tid;
        self.current_pool = current_pool;
        self.games = games;
        self.config.sound = sound;
        self.sound.update_settings(self.config.sound.clone());
        self.config.mode = mode;
        self.config.collect_scorer_cap_num = collect_scorer_cap_num;
        self.config.track_fouls_and_warnings = track_fouls_and_warnings;

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
            list_all_tournaments,
            touchscreen,
        } = flags;

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
                client,
                uwhscores_token: Arc::new(Mutex::new(None)),
                uwhscores_auth_valid_for: None,
                uwhportal_client,
                using_uwhscores: false,
                tournaments: None,
                games: None,
                current_tid: None,
                current_pool: None,
                sound,
                sim_child,
                fullscreen,
                list_all_tournaments,
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
                self.request_tournament_list();
                self.check_uwhscores_auth();
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
                        if let Some(game) = self
                            .games
                            .as_ref()
                            .and_then(|games| games.get(&tm.game_number()))
                        {
                            self.post_game_score(game, scores);
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
                    using_uwhscores: self.using_uwhscores,
                    uwhscores_email: self.config.uwhscores.email.clone(),
                    uwhscores_password: self.config.uwhscores.password.clone(),
                    uwhportal_token: self.config.uwhportal.token.clone(),
                    current_tid: self.current_tid,
                    current_pool: self.current_pool.clone(),
                    games: self.games.clone(),
                    sound: self.config.sound.clone(),
                    mode: self.config.mode,
                    hide_time: self.config.hide_time,
                    collect_scorer_cap_num: self.config.collect_scorer_cap_num,
                    track_fouls_and_warnings: self.config.track_fouls_and_warnings,
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
                            AppState::ConfirmationPage(ConfirmationKind::GameConfigChanged(
                                new_config,
                            ))
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

                            drop(tm);
                            self.apply_settings_change();

                            confy::store(APP_NAME, None, &self.config).unwrap();
                            AppState::MainPage
                        }
                    } else if edited_settings.game_number != self.snapshot.game_number {
                        if tm.current_period() != GamePeriod::BetweenGames {
                            AppState::ConfirmationPage(ConfirmationKind::GameNumberChanged)
                        } else {
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
                        AppState::KeypadPage(KeypadPage::TeamTimeouts(len, per_half), num) => {
                            edited_settings.config.team_timeout_duration = len;
                            edited_settings.config.num_team_timeouts_allowed = num;
                            edited_settings.config.timeouts_counted_per_half = per_half;
                        }
                        _ => unreachable!(),
                    }
                }

                let next_page = match self.app_state {
                    AppState::ParameterEditor(_, _) => ConfigPage::Tournament,
                    AppState::KeypadPage(KeypadPage::GameNumber, _) => ConfigPage::Main,
                    AppState::KeypadPage(KeypadPage::TeamTimeouts(_, _), _) => {
                        ConfigPage::Tournament
                    }
                    AppState::ParameterList(param, _) => match param {
                        ListableParameter::Game => ConfigPage::Main,
                        ListableParameter::Tournament | ListableParameter::Pool => {
                            ConfigPage::Tournament
                        }
                    },
                    _ => unreachable!(),
                };

                self.app_state = AppState::EditGameConfig(next_page);
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

                let next_page = match param {
                    ListableParameter::Tournament | ListableParameter::Pool => {
                        ConfigPage::Tournament
                    }
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
                        BoolGameParameter::WhiteOnRight => edited_settings.white_on_right ^= true,
                        BoolGameParameter::UsingUwhScores => {
                            edited_settings.using_uwhscores ^= true
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
                }
            }
            Message::TextParameterChanged(param, val) => {
                let settings = self.edited_settings.as_mut().unwrap();
                match param {
                    TextParameter::UwhscoresEmail => settings.uwhscores_email = val,
                    TextParameter::UwhscoresPassword => settings.uwhscores_password = val,
                    TextParameter::UwhportalToken => settings.uwhportal_token = val,
                }
            }
            Message::ApplyAuthChanges => {
                let settings = self.edited_settings.as_mut().unwrap();
                self.config.uwhscores.email = settings.uwhscores_email.clone();
                self.config.uwhscores.password = settings.uwhscores_password.clone();
                self.config.uwhportal.token = settings.uwhportal_token.clone();

                self.uwhscores_token.lock().unwrap().take();
                self.uwhscores_auth_valid_for = None;
                self.check_uwhscores_auth();

                self.app_state = AppState::EditGameConfig(ConfigPage::Tournament);
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
                    let scores = BlackWhiteBundle {
                        black: self.snapshot.b_score,
                        white: self.snapshot.w_score,
                    };

                    self.app_state = AppState::ConfirmScores(scores);
                    trace!("AppState changed to {:?}", self.app_state);
                }

                if let AppState::TimeEdit(_, _, ref mut timeout) = self.app_state {
                    *timeout = None;
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::RecvTournamentList(t_list) => {
                let active_filter = if self.list_all_tournaments {
                    |_: &TournamentInfo| true
                } else {
                    |t: &TournamentInfo| t.is_active == 1
                };
                let t_map = t_list
                    .into_iter()
                    .filter(active_filter)
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
                    if let Some(pools) = tournament.pools.as_ref() {
                        if pools.len() == 1 {
                            if let Some(ref mut edits) = self.edited_settings {
                                if edits.current_pool.is_none() {
                                    edits.current_pool = Some(pools[0].clone());
                                }
                            }
                        }
                    }
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
            Message::StartClock => self.tm.lock().unwrap().start_clock(Instant::now()),
            Message::StopClock => self.tm.lock().unwrap().stop_clock(Instant::now()).unwrap(),
            Message::UwhScoresAuthChecked(valid) => self.uwhscores_auth_valid_for = Some(valid),
            Message::NoAction => {}
        };

        command
    }

    fn view(&self) -> Element<Message> {
        let clock_running = self.tm.lock().unwrap().clock_is_running();
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
                    self.using_uwhscores,
                    &self.games,
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
                self.using_uwhscores,
                &self.games,
                self.config.mode,
                clock_running,
            ),
            AppState::WarningsSummaryPage =>
                build_warnings_summary_page(&self.snapshot, self.config.mode, clock_running,),
            AppState::EditGameConfig(page) => build_game_config_edit_page(
                &self.snapshot,
                self.edited_settings.as_ref().unwrap(),
                &self.tournaments,
                page,
                self.config.mode,
                clock_running,
                &self.uwhscores_auth_valid_for,
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
                &self.tournaments,
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
