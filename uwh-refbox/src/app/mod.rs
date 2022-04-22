use super::APP_CONFIG_NAME;
use crate::{penalty_editor::*, tournament_manager::*};
use async_std::{
    channel::{unbounded, Receiver, TryRecvError},
    task,
};
use iced::{
    button, executor, futures::FutureExt, Application, Clipboard, Column, Command, Element,
    Subscription,
};
use iced_futures::{
    futures::{
        select,
        stream::{self, BoxStream},
    },
    subscription::Recipe,
};
use log::*;
use std::{
    cmp::min,
    hash::Hasher,
    io::Write,
    net::{Shutdown, TcpStream},
    process::Child,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uwh_common::{
    config::Config,
    game_snapshot::{Color as GameColor, GamePeriod, GameSnapshot},
};
use uwh_matrix_drawing::transmitted_data::TransmittedData;

mod view_builders;
use view_builders::*;

pub mod style;
use style::{PADDING, SPACING, WINDOW_BACKGORUND};

#[derive(Debug)]
pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    config: Config,
    edited_game_num: u16,
    snapshot: GameSnapshot,
    time_updater: TimeUpdater,
    pen_edit: PenaltyEditor,
    states: States,
    app_state: AppState,
    last_app_state: AppState,
    sim_sender: Option<TcpStream>,
    sim_child: Option<Child>,
}

#[derive(Debug, Clone)]
enum AppState {
    MainPage,
    TimeEdit(bool, Duration, Option<Duration>),
    ScoreEdit { black: u8, white: u8 },
    PenaltyOverview { black_idx: usize, white_idx: usize },
    KeypadPage(KeypadPage, u16),
    EditGameConfig,
    ParameterEditor(LengthParameter, Duration),
    ConfirmationPage(ConfirmationKind),
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub enum BoolGameParameter {
    OvertimeAllowed,
    SuddenDeathAllowed,
}

#[derive(Debug, Clone)]
pub enum Message {
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
        color: GameColor,
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
    ParameterEditComplete {
        canceled: bool,
    },
    ToggleBoolParameter(BoolGameParameter),
    ConfirmationSelected(ConfirmationOption),
    BlackTimeout(bool),
    WhiteTimeout(bool),
    RefTimeout(bool),
    PenaltyShot(bool),
    EndTimeout,
    NoAction, // TODO: Remove once UI is functional
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
    fn apply_snapshot(&mut self, snapshot: GameSnapshot) {
        let mut connection_lost = false;
        if let Some(ref mut sender) = self.sim_sender {
            match sender.write_all(
                &TransmittedData {
                    snapshot: snapshot.clone().into(),
                    white_on_right: self.config.hardware.white_on_right,
                }
                .encode()
                .unwrap(),
            ) {
                Ok(_) => {}
                Err(_) => {
                    error!("Connection to simulator lost");
                    connection_lost = true;
                }
            }
        }
        if connection_lost {
            self.sim_sender = None;
        }
        self.snapshot = snapshot;
    }
}

impl Drop for RefBoxApp {
    fn drop(&mut self) {
        if let Some(socket) = self.sim_sender.take() {
            socket.shutdown(Shutdown::Both).unwrap();
        }
        if let Some(mut child) = self.sim_child.take() {
            info!("Waiting for child");
            child.wait().unwrap();
        }
    }
}

impl Application for RefBoxApp {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = (Config, Option<TcpStream>, Option<Child>);

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (config, sim_sender, sim_child) = flags;

        let mut tm = TournamentManager::new(config.game.clone());
        tm.start_clock(Instant::now());

        let (clk_run_tx, clk_run_rx) = unbounded();
        tm.add_start_stop_sender(clk_run_tx);

        let tm = Arc::new(Mutex::new(tm));

        let snapshot = Default::default();

        (
            Self {
                pen_edit: PenaltyEditor::new(tm.clone()),
                time_updater: TimeUpdater {
                    tm: tm.clone(),
                    clock_running_receiver: Some(clk_run_rx),
                },
                tm,
                config,
                edited_game_num: 0,
                snapshot,
                states: Default::default(),
                app_state: AppState::MainPage,
                last_app_state: AppState::MainPage,
                sim_sender,
                sim_child,
            },
            Command::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::from_recipe(self.time_updater.clone()).map(Message::NewSnapshot)
    }

    fn title(&self) -> String {
        "UWH Ref Box".into()
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        trace!("Handling message: {message:?}");
        match message {
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
                let dur = match self.app_state {
                    AppState::TimeEdit(_, ref mut game_dur, ref mut timeout_dur) => {
                        if timeout {
                            timeout_dur.as_mut().unwrap()
                        } else {
                            game_dur
                        }
                    }
                    AppState::ParameterEditor(_, ref mut dur) => dur,
                    AppState::KeypadPage(KeypadPage::TeamTimeouts(ref mut dur), _) => dur,
                    _ => unreachable!(),
                };
                if increase {
                    *dur = min(
                        Duration::from_secs(5999),
                        dur.saturating_add(Duration::from_secs(secs)),
                    );
                } else {
                    *dur = dur.saturating_sub(Duration::from_secs(secs));
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
                    black: tm.get_b_score(),
                    white: tm.get_w_score(),
                };
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ChangeScore { color, increase } => {
                if let AppState::ScoreEdit {
                    ref mut black,
                    ref mut white,
                } = self.app_state
                {
                    match color {
                        GameColor::Black => {
                            if increase {
                                *black = black.saturating_add(1);
                            } else {
                                *black = black.saturating_sub(1);
                            }
                        }
                        GameColor::White => {
                            if increase {
                                *white = white.saturating_add(1);
                            } else {
                                *white = white.saturating_sub(1);
                            }
                        }
                    }
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ScoreEditComplete { canceled } => {
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                if let AppState::ScoreEdit { black, white } = self.app_state {
                    if !canceled {
                        tm.set_scores(black, white, now);
                    }
                } else {
                    unreachable!()
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot);
                self.app_state = AppState::MainPage;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::PenaltyOverview => {
                self.pen_edit.start_session().unwrap();
                self.app_state = AppState::PenaltyOverview {
                    black_idx: 0,
                    white_idx: 0,
                };
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::Scroll { color, up } => {
                if let AppState::PenaltyOverview {
                    ref mut black_idx,
                    ref mut white_idx,
                } = self.app_state
                {
                    let idx = match color {
                        GameColor::Black => black_idx,
                        GameColor::White => white_idx,
                    };
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
                    let err_string = format!("An error occurred while applying the changes to the penalties. \
                            Some of the changes may have been applied. Please retry any remaining changes.\n\n\
                            Error Message:\n{e}");
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
                self.app_state = AppState::PenaltyOverview {
                    black_idx: 0,
                    white_idx: 0,
                };
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
                    KeypadPage::GameNumber => self.edited_game_num,
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
                if !canceled {
                    if let AppState::KeypadPage(KeypadPage::AddScore(color), player) =
                        self.app_state
                    {
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();
                        match color {
                            GameColor::Black => tm.add_b_score(player.try_into().unwrap(), now),
                            GameColor::White => tm.add_w_score(player.try_into().unwrap(), now),
                        };
                        let snapshot = tm.generate_snapshot(now).unwrap();
                        std::mem::drop(tm);
                        self.apply_snapshot(snapshot);
                    } else {
                        unreachable!()
                    }
                }
                self.app_state = AppState::MainPage;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::EditGameConfig => {
                self.config.game = self.tm.lock().unwrap().config().clone();
                self.edited_game_num = self.snapshot.game_number;
                self.app_state = AppState::EditGameConfig;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ConfigEditComplete { canceled } => {
                let mut tm = self.tm.lock().unwrap();
                self.app_state = if !canceled {
                    if self.config.game != *tm.config() {
                        if tm.current_period() != GamePeriod::BetweenGames {
                            AppState::ConfirmationPage(ConfirmationKind::GameConfigChanged)
                        } else {
                            tm.set_config(self.config.game.clone()).unwrap();
                            confy::store(APP_CONFIG_NAME, &self.config).unwrap();
                            AppState::MainPage
                        }
                    } else if self.edited_game_num != self.snapshot.game_number {
                        if tm.current_period() == GamePeriod::BetweenGames {
                            tm.set_next_game_number(self.edited_game_num);
                            AppState::MainPage
                        } else {
                            AppState::ConfirmationPage(ConfirmationKind::GameNumberChanged)
                        }
                    } else {
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
                    Duration::from_secs(match param {
                        LengthParameter::Half => self.config.game.half_play_duration.into(),
                        LengthParameter::HalfTime => self.config.game.half_time_duration.into(),
                        LengthParameter::NominalBetweenGame => {
                            self.config.game.nominal_break.into()
                        }
                        LengthParameter::MinimumBetweenGame => {
                            self.config.game.minimum_break.into()
                        }
                        LengthParameter::PreOvertime => self.config.game.pre_overtime_break.into(),
                        LengthParameter::OvertimeHalf => {
                            self.config.game.ot_half_play_duration.into()
                        }
                        LengthParameter::OvertimeHalfTime => {
                            self.config.game.ot_half_time_duration.into()
                        }
                        LengthParameter::PreSuddenDeath => {
                            self.config.game.pre_sudden_death_duration.into()
                        }
                    }),
                );
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ParameterEditComplete { canceled } => {
                if !canceled {
                    match self.app_state {
                        AppState::ParameterEditor(param, dur) => {
                            let val: u16 = dur.as_secs().try_into().unwrap();
                            match param {
                                LengthParameter::Half => self.config.game.half_play_duration = val,
                                LengthParameter::HalfTime => {
                                    self.config.game.half_time_duration = val
                                }
                                LengthParameter::NominalBetweenGame => {
                                    self.config.game.nominal_break = val
                                }
                                LengthParameter::MinimumBetweenGame => {
                                    self.config.game.minimum_break = val
                                }
                                LengthParameter::PreOvertime => {
                                    self.config.game.pre_overtime_break = val
                                }
                                LengthParameter::OvertimeHalf => {
                                    self.config.game.ot_half_play_duration = val
                                }
                                LengthParameter::OvertimeHalfTime => {
                                    self.config.game.ot_half_time_duration = val
                                }
                                LengthParameter::PreSuddenDeath => {
                                    self.config.game.pre_sudden_death_duration = val
                                }
                            }
                        }
                        AppState::KeypadPage(KeypadPage::GameNumber, num) => {
                            self.edited_game_num = num;
                        }
                        AppState::KeypadPage(KeypadPage::TeamTimeouts(len), num) => {
                            self.config.game.team_timeout_duration =
                                len.as_secs().try_into().unwrap();
                            self.config.game.team_timeouts_per_half = num;
                        }
                        _ => unreachable!(),
                    }
                }

                self.app_state = AppState::EditGameConfig;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ToggleBoolParameter(param) => match param {
                BoolGameParameter::OvertimeAllowed => self.config.game.has_overtime ^= true,
                BoolGameParameter::SuddenDeathAllowed => {
                    self.config.game.sudden_death_allowed ^= true
                }
            },
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
                        let mut tm = self.tm.lock().unwrap();
                        let now = Instant::now();
                        tm.reset_game(now);
                        if config_changed {
                            tm.set_config(self.config.game.clone()).unwrap();
                            confy::store(APP_CONFIG_NAME, &self.config).unwrap();
                        }
                        tm.set_game_number(self.edited_game_num);
                        let snapshot = tm.generate_snapshot(now).unwrap();
                        std::mem::drop(tm);
                        self.apply_snapshot(snapshot);
                        AppState::MainPage
                    }
                    ConfirmationOption::KeepGameAndApply => {
                        let mut tm = self.tm.lock().unwrap();
                        tm.set_game_number(self.edited_game_num);
                        let snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                        std::mem::drop(tm);
                        self.apply_snapshot(snapshot);
                        AppState::MainPage
                    }
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
            Message::NoAction => {}
        };

        Command::none()
    }

    fn background_color(&self) -> iced::Color {
        WINDOW_BACKGORUND
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .spacing(SPACING)
            .padding(PADDING)
            .push(match self.app_state {
                AppState::MainPage => build_main_view(
                    &self.snapshot,
                    &mut self.states.main_view,
                    &self.config.game,
                ),
                AppState::TimeEdit(_, time, timeout_time) => build_time_edit_view(
                    &self.snapshot,
                    &mut self.states.time_edit_view,
                    time,
                    timeout_time,
                ),
                AppState::ScoreEdit { black, white } => build_score_edit_view(
                    &self.snapshot,
                    &mut self.states.score_edit_view,
                    black,
                    white,
                ),
                AppState::PenaltyOverview {
                    black_idx,
                    white_idx,
                } => build_penalty_overview_page(
                    &self.snapshot,
                    &mut self.states.penalty_overview,
                    self.pen_edit.get_printable_lists(Instant::now()).unwrap(),
                    BlackWhiteBundle {
                        black: black_idx,
                        white: white_idx,
                    },
                ),
                AppState::KeypadPage(page, player_num) => build_keypad_page(
                    &self.snapshot,
                    &mut self.states.keypad_page,
                    page,
                    player_num,
                ),
                AppState::EditGameConfig => build_game_config_edit_page(
                    &self.snapshot,
                    &mut self.states.game_config_edit,
                    &self.config.game,
                    self.edited_game_num,
                ),
                AppState::ParameterEditor(param, dur) => build_game_parameter_editor(
                    &self.snapshot,
                    &mut self.states.game_param_edit,
                    param,
                    dur,
                ),
                AppState::ConfirmationPage(ref kind) => build_confirmation_page(
                    &self.snapshot,
                    &mut self.states.confirmation_page,
                    kind,
                ),
            })
            .push(build_timeout_ribbon(
                &self.snapshot,
                &mut self.states.timeout_ribbon,
                &self.tm,
            ))
            .into()
    }
}

#[derive(Clone, Debug)]
struct TimeUpdater {
    tm: Arc<Mutex<TournamentManager>>,
    clock_running_receiver: Option<Receiver<bool>>,
}

impl<H: Hasher, I> Recipe<H, I> for TimeUpdater {
    type Output = GameSnapshot;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        "TimeUpdater".hash(state);
    }

    fn stream(
        mut self: Box<Self>,
        _input: BoxStream<'static, I>,
    ) -> BoxStream<'static, Self::Output> {
        debug!("Updater started");

        struct State {
            tm: Arc<Mutex<TournamentManager>>,
            clock_running_receiver: Receiver<bool>,
            next_time: Option<Instant>,
        }

        let state = State {
            tm: self.tm.clone(),
            clock_running_receiver: self.clock_running_receiver.take().unwrap(),
            next_time: Some(Instant::now()),
        };

        Box::pin(stream::unfold(state, |mut state| async move {
            let mut clock_running = true;
            if let Some(next_time) = state.next_time {
                if let Some(dur) = next_time.checked_duration_since(Instant::now()) {
                    select! {
                        _ = Box::pin(task::sleep(dur)).fuse() => {},
                        res = Box::pin(state.clock_running_receiver.recv()).fuse() => match res {
                            Err(_) => return None,
                            Ok(val) => {
                                debug!("Received clock running message: {val}");
                                clock_running = val;
                            },
                        },
                    };
                } else {
                    match state.clock_running_receiver.try_recv() {
                        Ok(val) => {
                            debug!("Received clock running message: {val}");
                            clock_running = val;
                        }
                        Err(TryRecvError::Empty) => {}
                        Err(TryRecvError::Closed) => {
                            return None;
                        }
                    };
                }
            } else {
                debug!("Awaiting a new clock running message");
                match state.clock_running_receiver.recv().await {
                    Err(_) => return None,
                    Ok(val) => {
                        debug!("Received clock running message: {val}");
                        clock_running = val
                    }
                };
            };

            let mut tm = state.tm.lock().unwrap();
            let now = Instant::now();
            tm.update(now).unwrap();
            let snapshot = tm
                .generate_snapshot(now)
                .expect("Failed to generate snapshot");

            state.next_time = if clock_running {
                Some(tm.next_update_time(now).unwrap())
            } else {
                None
            };

            drop(tm);

            Some((snapshot, state))
        }))
    }
}

#[derive(Clone, Default, Debug)]
struct States {
    main_view: MainViewStates,
    time_edit_view: TimeEditViewStates,
    score_edit_view: ScoreEditViewStates,
    timeout_ribbon: TimeoutRibbonStates,
    penalty_overview: PenaltyOverviewStates,
    keypad_page: KeypadPageStates,
    game_config_edit: GameConfigEditStates,
    game_param_edit: GameParamEditStates,
    confirmation_page: ConfirmationPageStates,
}

#[derive(Clone, Default, Debug)]
struct MainViewStates {
    black_score: button::State,
    white_score: button::State,
    black_new_score: button::State,
    white_new_score: button::State,
    black_penalties: button::State,
    white_penalties: button::State,
    game_config: button::State,
    game_time: button::State,
    start_now: button::State,
}

#[derive(Clone, Default, Debug)]
struct TimeEditorStates {
    min_up: button::State,
    min_down: button::State,
    sec_up: button::State,
    sec_down: button::State,
}

#[derive(Clone, Default, Debug)]
struct TimeEditViewStates {
    game_time: button::State,
    game_time_edit: TimeEditorStates,
    timeout_time_edit: TimeEditorStates,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct ScoreEditViewStates {
    game_time: button::State,
    b_up: button::State,
    b_down: button::State,
    w_up: button::State,
    w_down: button::State,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct GameConfigEditStates {
    game_time: button::State,
    using_uwhscores: button::State,
    game_number: button::State,
    half_length: button::State,
    half_time_length: button::State,
    nom_between_games: button::State,
    min_between_games: button::State,
    overtime_allowed: button::State,
    pre_ot_break: button::State,
    ot_half_length: button::State,
    ot_half_time_length: button::State,
    sd_allowed: button::State,
    pre_sd_break: button::State,
    team_timeouts: button::State,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct PenaltyOverviewStates {
    game_time: button::State,
    b_list: PenaltyListStates,
    w_list: PenaltyListStates,
    new: button::State,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct PenaltyListStates {
    up: button::State,
    down: button::State,
    list: [button::State; PENALTY_LIST_LEN],
}

#[derive(Clone, Default, Debug)]
struct KeypadPageStates {
    game_time: button::State,
    zero: button::State,
    one: button::State,
    two: button::State,
    three: button::State,
    four: button::State,
    five: button::State,
    six: button::State,
    seven: button::State,
    eight: button::State,
    nine: button::State,
    delete: button::State,
    add_score: AddScoreStates,
    eidt_penalty: EditPenaltyStates,
    edit_game_num: EditGameNumStates,
    team_timeout: EditTeamTimeoutStates,
}

#[derive(Clone, Default, Debug)]
struct AddScoreStates {
    black: button::State,
    white: button::State,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct EditPenaltyStates {
    black: button::State,
    white: button::State,
    one_min: button::State,
    two_min: button::State,
    five_min: button::State,
    total_dismis: button::State,
    done: button::State,
    delete: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct EditGameNumStates {
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct EditTeamTimeoutStates {
    length_edit: TimeEditorStates,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct GameParamEditStates {
    game_time: button::State,
    length_edit: TimeEditorStates,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct ConfirmationPageStates {
    game_time: button::State,
    button_1: button::State,
    button_2: button::State,
    button_3: button::State,
    button_4: button::State,
}

#[derive(Clone, Default, Debug)]
struct TimeoutRibbonStates {
    white_timeout: button::State,
    ref_timeout: button::State,
    penalty_shot: button::State,
    black_timeout: button::State,
}
