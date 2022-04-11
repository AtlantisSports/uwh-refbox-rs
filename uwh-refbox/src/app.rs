use crate::style::{
    self, BLACK, GREEN, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, RED, SPACING, WHITE,
    WINDOW_BACKGORUND, YELLOW,
};
use crate::tournament_manager::*;
use async_std::{
    channel::{unbounded, Receiver, TryRecvError},
    task,
};
use iced::{
    button, executor, futures::FutureExt, Align, Application, Button, Clipboard, Color, Column,
    Command, Container, Element, HorizontalAlignment, Length, Row, Space, Subscription, Text,
    VerticalAlignment,
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
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uwh_common::{
    config::Game,
    game_snapshot::{Color as GameColor, GamePeriod, GameSnapshot, TimeoutSnapshot},
};
use uwh_matrix_drawing::secs_to_time_string;

#[derive(Debug)]
pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    snapshot: GameSnapshot,
    time_updater: TimeUpdater,
    states: States,
    app_state: AppState,
    last_app_state: AppState,
}

#[derive(Debug, Clone)]
enum AppState {
    MainPage,
    TimeEdit(bool, Duration, Option<Duration>),
    ScoreEdit { black: u8, white: u8 },
    KeypadPage(KeypadPage, u8),
}

#[derive(Debug, Clone)]
pub enum KeypadPage {
    AddScore(GameColor),
}

#[derive(Debug, Clone)]
pub enum Message {
    NewSnapshot(GameSnapshot),
    EditTime,
    IncreaseTime { secs: u64, timeout: bool },
    DecreaseTime { secs: u64, timeout: bool },
    TimeEditComplete { canceled: bool },
    EditScores,
    ChangeScore { color: GameColor, increase: bool },
    ScoreEditComplete { canceled: bool },
    KeypadPage(KeypadPage),
    KeypadButtonPress(KeypadButton),
    ChangeColor(GameColor),
    AddScoreComplete { canceled: bool },
    BlackTimeout(bool),
    WhiteTimeout(bool),
    RefTimeout(bool),
    PenaltyShot(bool),
    EndTimeout,
    NoAction, // TODO: Remove once UI is functional
}

#[derive(Debug, Clone, Copy)]
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

impl Application for RefBoxApp {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = Game;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let mut tm = TournamentManager::new(flags);
        tm.start_clock(Instant::now());

        let (clk_run_tx, clk_run_rx) = unbounded();
        tm.add_start_stop_sender(clk_run_tx);

        let tm = Arc::new(Mutex::new(tm));

        let snapshot = Default::default();

        (
            Self {
                time_updater: TimeUpdater {
                    tm: tm.clone(),
                    clock_running_receiver: Some(clk_run_rx),
                },
                tm,
                snapshot,
                states: Default::default(),
                app_state: AppState::MainPage,
                last_app_state: AppState::MainPage,
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
            Message::NewSnapshot(snapshot) => self.snapshot = snapshot,
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
            Message::IncreaseTime { secs, timeout } => {
                if let AppState::TimeEdit(_, ref mut dur, ref mut t_o_dur) = self.app_state {
                    if !timeout {
                        *dur = min(
                            Duration::from_secs(5999),
                            dur.saturating_add(Duration::from_secs(secs)),
                        );
                    } else if let Some(ref mut t_o_dur) = t_o_dur {
                        *t_o_dur = min(
                            Duration::from_secs(5999),
                            t_o_dur.saturating_add(Duration::from_secs(secs)),
                        );
                    } else {
                        unreachable!()
                    }
                    trace!("AppState changed to {:?}", self.app_state);
                } else {
                    unreachable!()
                };
            }
            Message::DecreaseTime { secs, timeout } => {
                if let AppState::TimeEdit(_, ref mut dur, ref mut t_o_dur) = self.app_state {
                    if !timeout {
                        *dur = dur.saturating_sub(Duration::from_secs(secs));
                    } else if let Some(ref mut t_o_dur) = t_o_dur {
                        *t_o_dur = t_o_dur.saturating_sub(Duration::from_secs(secs));
                    } else {
                        unreachable!()
                    }
                    trace!("AppState changed to {:?}", self.app_state);
                } else {
                    unreachable!()
                };
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
                        tm.start_clock(Instant::now());
                    }
                    self.app_state = self.last_app_state.clone();
                    trace!("AppState changed to {:?}", self.app_state);
                } else {
                    unreachable!();
                }
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
                if let AppState::ScoreEdit { black, white } = self.app_state {
                    if !canceled {
                        tm.set_scores(black, white, Instant::now());
                    }
                } else {
                    unreachable!()
                }
                self.snapshot = tm.generate_snapshot(Instant::now()).unwrap();
                self.app_state = AppState::MainPage;
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::KeypadPage(page) => {
                self.app_state = AppState::KeypadPage(page, 0);
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::KeypadButtonPress(key) => {
                if let AppState::KeypadPage(_, ref mut val) = self.app_state {
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
                    if new_val < 100 {
                        *val = new_val;
                    }
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::ChangeColor(new_color) => {
                if let AppState::KeypadPage(KeypadPage::AddScore(ref mut color), _) = self.app_state
                {
                    *color = new_color;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
            }
            Message::AddScoreComplete { canceled } => {
                if !canceled {
                    if let AppState::KeypadPage(KeypadPage::AddScore(color), player) =
                        self.app_state
                    {
                        match color {
                            GameColor::Black => {
                                self.tm.lock().unwrap().add_b_score(player, Instant::now())
                            }
                            GameColor::White => {
                                self.tm.lock().unwrap().add_w_score(player, Instant::now())
                            }
                        };
                    } else {
                        unreachable!()
                    }
                }
                self.app_state = AppState::MainPage;
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
                self.snapshot = tm.generate_snapshot(Instant::now()).unwrap();
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
                self.snapshot = tm.generate_snapshot(Instant::now()).unwrap();
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
                self.snapshot = tm.generate_snapshot(Instant::now()).unwrap();
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
                self.snapshot = tm.generate_snapshot(Instant::now()).unwrap();
            }
            Message::EndTimeout => {
                let mut tm = self.tm.lock().unwrap();
                tm.end_timeout(Instant::now()).unwrap();
                self.snapshot = tm.generate_snapshot(Instant::now()).unwrap();
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
                AppState::MainPage => build_main_view(&self.snapshot, &mut self.states.main_view),
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
                AppState::KeypadPage(ref page, player_num) => build_keypad_page(
                    &self.snapshot,
                    &mut self.states.keypad_page,
                    page,
                    player_num,
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

fn build_main_view<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut MainViewStates,
) -> Element<'a, Message> {
    let (period_text, period_color) = period_text_and_color(snapshot.current_period);
    let game_time_info = Column::new()
        .spacing(SPACING)
        .width(Length::Fill)
        .align_items(Align::Center)
        .push(Text::new(period_text).color(period_color))
        .push(
            Text::new(time_string(snapshot.secs_in_period))
                .color(period_color)
                .size(LARGE_TEXT),
        );

    let time_button_content: Element<'a, Message> = match snapshot.timeout {
        TimeoutSnapshot::Black(_) => Row::new()
            .spacing(SPACING)
            .push(game_time_info)
            .push(
                Column::new()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new("BLACK TIMEOUT").color(BLACK))
                    .push(
                        Text::new(timeout_time_string(snapshot))
                            .color(BLACK)
                            .size(LARGE_TEXT),
                    ),
            )
            .into(),
        TimeoutSnapshot::White(_) => Row::new()
            .spacing(SPACING)
            .push(game_time_info)
            .push(
                Column::new()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new("WHITE TIMEOUT").color(WHITE))
                    .push(
                        Text::new(timeout_time_string(snapshot))
                            .color(WHITE)
                            .size(LARGE_TEXT),
                    ),
            )
            .into(),
        TimeoutSnapshot::Ref(_) => Row::new()
            .spacing(SPACING)
            .push(game_time_info)
            .push(
                Column::new()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new("REF TIMEOUT").color(YELLOW))
                    .push(
                        Text::new(timeout_time_string(snapshot))
                            .color(YELLOW)
                            .size(LARGE_TEXT),
                    ),
            )
            .into(),
        TimeoutSnapshot::PenaltyShot(_) => Row::new()
            .spacing(SPACING)
            .push(game_time_info)
            .push(
                Column::new()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new("PENALTY SHOT").color(RED))
                    .push(
                        Text::new(timeout_time_string(snapshot))
                            .color(RED)
                            .size(LARGE_TEXT),
                    ),
            )
            .into(),
        TimeoutSnapshot::None => game_time_info.width(Length::Fill).into(),
    };

    let mut center_col = Column::new().spacing(SPACING).width(Length::Fill).push(
        Button::new(&mut states.game_time, time_button_content)
            .padding(PADDING)
            .width(Length::Fill)
            .style(style::Button::Gray)
            .on_press(Message::EditTime),
    );

    match snapshot.timeout {
        TimeoutSnapshot::White(_)
        | TimeoutSnapshot::Black(_)
        | TimeoutSnapshot::Ref(_)
        | TimeoutSnapshot::PenaltyShot(_) => {
            center_col = center_col.push(
                make_button(&mut states.start_now, "END TIMEOUT")
                    .style(style::Button::Yellow)
                    .on_press(Message::EndTimeout),
            )
        }
        TimeoutSnapshot::None => {
            match snapshot.current_period {
                GamePeriod::BetweenGames
                | GamePeriod::HalfTime
                | GamePeriod::PreOvertime
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::PreSuddenDeath => {
                    center_col = center_col.push(
                        make_button(&mut states.start_now, "START NOW")
                            .style(style::Button::Green)
                            .on_press(Message::NoAction),
                    )
                }
                GamePeriod::FirstHalf
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::SuddenDeath => {}
            };
        }
    };

    center_col = center_col.push(
        Container::new(Text::new("Game Information"))
            .style(style::Container::LightGray)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill),
    );

    let black_col = Column::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .width(Length::Fill)
        .push(
            Button::new(
                &mut states.black_score,
                Column::new()
                    .spacing(SPACING)
                    .align_items(Align::Center)
                    .width(Length::Fill)
                    .push(Text::new("BLACK"))
                    .push(Text::new(snapshot.b_score.to_string()).size(LARGE_TEXT)),
            )
            .padding(PADDING)
            .width(Length::Fill)
            .style(style::Button::Black)
            .on_press(Message::EditScores),
        )
        .push(
            make_button(&mut states.black_new_score, "SCORE\nBLACK")
                .style(style::Button::Black)
                .on_press(Message::KeypadPage(KeypadPage::AddScore(GameColor::Black))),
        )
        .push(
            make_button(&mut states.black_penalties, "Penalties")
                .style(style::Button::Black)
                .on_press(Message::NoAction)
                .height(Length::Fill),
        );

    let white_col = Column::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .width(Length::Fill)
        .push(
            Button::new(
                &mut states.white_score,
                Column::new()
                    .spacing(SPACING)
                    .align_items(Align::Center)
                    .width(Length::Fill)
                    .push(Text::new("WHITE"))
                    .push(Text::new(snapshot.w_score.to_string()).size(LARGE_TEXT)),
            )
            .padding(PADDING)
            .width(Length::Fill)
            .style(style::Button::White)
            .on_press(Message::EditScores),
        )
        .push(
            make_button(&mut states.white_new_score, "SCORE\nWHITE")
                .style(style::Button::White)
                .on_press(Message::KeypadPage(KeypadPage::AddScore(GameColor::White))),
        )
        .push(
            make_button(&mut states.white_penalties, "Penalties")
                .style(style::Button::White)
                .on_press(Message::NoAction)
                .height(Length::Fill),
        );

    Row::new()
        .spacing(0)
        .height(Length::Fill)
        .push(
            Row::new()
                .width(Length::Fill)
                .spacing(0)
                .push(black_col)
                .push(Space::new(Length::Units(3 * SPACING / 4), Length::Fill)),
        )
        .push(
            Row::new()
                .width(Length::FillPortion(2))
                .spacing(0)
                .push(Space::new(Length::Units(SPACING / 4), Length::Fill))
                .push(center_col)
                .push(Space::new(Length::Units(SPACING / 4), Length::Fill)),
        )
        .push(
            Row::new()
                .width(Length::Fill)
                .spacing(0)
                .push(Space::new(Length::Units(3 * SPACING / 4), Length::Fill))
                .push(white_col),
        )
        .into()
}

fn build_time_edit_view<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut TimeEditViewStates,
    time: Duration,
    timeout_time: Option<Duration>,
) -> Element<'a, Message> {
    let mut edit_row = Row::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .push(Space::new(Length::Fill, Length::Shrink))
        .push(
            Column::new()
                .spacing(SPACING)
                .push(
                    make_small_button(&mut states.min_up, "+", LARGE_TEXT)
                        .style(style::Button::Blue)
                        .on_press(Message::IncreaseTime {
                            secs: 60,
                            timeout: false,
                        }),
                )
                .push(
                    make_small_button(&mut states.min_down, "-", LARGE_TEXT)
                        .style(style::Button::Blue)
                        .on_press(Message::DecreaseTime {
                            secs: 60,
                            timeout: false,
                        }),
                ),
        )
        .push(
            Column::new()
                .spacing(SPACING)
                .align_items(Align::Center)
                .width(Length::Units(200))
                .push(Text::new("GAME TIME"))
                .push(Text::new(time_string(time.as_secs().try_into().unwrap())).size(LARGE_TEXT)),
        )
        .push(
            Column::new()
                .spacing(SPACING)
                .push(
                    make_small_button(&mut states.sec_up, "+", LARGE_TEXT)
                        .style(style::Button::Blue)
                        .on_press(Message::IncreaseTime {
                            secs: 1,
                            timeout: false,
                        }),
                )
                .push(
                    make_small_button(&mut states.sec_down, "-", LARGE_TEXT)
                        .style(style::Button::Blue)
                        .on_press(Message::DecreaseTime {
                            secs: 1,
                            timeout: false,
                        }),
                ),
        )
        .push(Space::new(Length::Fill, Length::Shrink));

    if snapshot.timeout != TimeoutSnapshot::None {
        edit_row = edit_row
            .push(Space::new(Length::Fill, Length::Shrink))
            .push(
                Column::new()
                    .spacing(SPACING)
                    .push(
                        make_small_button(&mut states.timeout_min_up, "+", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::IncreaseTime {
                                secs: 60,
                                timeout: true,
                            }),
                    )
                    .push(
                        make_small_button(&mut states.timeout_min_down, "-", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::DecreaseTime {
                                secs: 60,
                                timeout: true,
                            }),
                    ),
            )
            .push(
                Column::new()
                    .spacing(SPACING)
                    .align_items(Align::Center)
                    .width(Length::Units(200))
                    .push(Text::new("TIMEOUT"))
                    .push(
                        Text::new(time_string(
                            timeout_time.unwrap().as_secs().try_into().unwrap(),
                        ))
                        .size(LARGE_TEXT),
                    ),
            )
            .push(
                Column::new()
                    .spacing(SPACING)
                    .push(
                        make_small_button(&mut states.timeout_sec_up, "+", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::IncreaseTime {
                                secs: 1,
                                timeout: true,
                            }),
                    )
                    .push(
                        make_small_button(&mut states.timeout_sec_down, "-", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::DecreaseTime {
                                secs: 1,
                                timeout: true,
                            }),
                    ),
            )
            .push(Space::new(Length::Fill, Length::Shrink));
    }

    Column::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::NoAction))
        .push(Space::new(Length::Fill, Length::Fill))
        .push(edit_row)
        .push(Space::new(Length::Fill, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::TimeEditComplete { canceled: true }),
                )
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::TimeEditComplete { canceled: false }),
                ),
        )
        .into()
}

fn build_score_edit_view<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut ScoreEditViewStates,
    black: u8,
    white: u8,
) -> Element<'a, Message> {
    Column::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::EditTime))
        .push(Space::new(Length::Fill, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(
                    Container::new(
                        Row::new()
                            .spacing(SPACING)
                            .align_items(Align::Center)
                            .push(
                                Column::new()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button(&mut states.b_up, "+", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::Black,
                                                increase: true,
                                            }),
                                    )
                                    .push(
                                        make_small_button(&mut states.b_down, "-", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::Black,
                                                increase: false,
                                            }),
                                    ),
                            )
                            .push(
                                Column::new()
                                    .spacing(SPACING)
                                    .width(Length::Fill)
                                    .align_items(Align::Center)
                                    .push(Text::new("BLACK"))
                                    .push(Text::new(black.to_string()).size(LARGE_TEXT)),
                            ),
                    )
                    .padding(PADDING)
                    .width(Length::FillPortion(2))
                    .style(style::Container::Black),
                )
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(
                    Container::new(
                        Row::new()
                            .spacing(SPACING)
                            .align_items(Align::Center)
                            .push(
                                Column::new()
                                    .spacing(SPACING)
                                    .width(Length::Fill)
                                    .align_items(Align::Center)
                                    .push(Text::new("WHITE"))
                                    .push(Text::new(white.to_string()).size(LARGE_TEXT)),
                            )
                            .push(
                                Column::new()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button(&mut states.w_up, "+", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::White,
                                                increase: true,
                                            }),
                                    )
                                    .push(
                                        make_small_button(&mut states.w_down, "-", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::White,
                                                increase: false,
                                            }),
                                    ),
                            ),
                    )
                    .padding(PADDING)
                    .width(Length::FillPortion(2))
                    .style(style::Container::White),
                )
                .push(Space::new(Length::Fill, Length::Shrink)),
        )
        .push(Space::new(Length::Fill, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ScoreEditComplete { canceled: true }),
                )
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ScoreEditComplete { canceled: false }),
                ),
        )
        .into()
}

fn build_keypad_page<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut KeypadPageStates,
    page: &KeypadPage,
    player_num: u8,
) -> Element<'a, Message> {
    Column::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::EditTime))
        .push(
            Row::new()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(
                    Container::new(
                        Column::new()
                            .spacing(SPACING)
                            .push(
                                Row::new()
                                    .spacing(SPACING)
                                    .align_items(Align::Center)
                                    .height(Length::Fill)
                                    .width(Length::Units(3 * MIN_BUTTON_SIZE as u16 + 2 * SPACING))
                                    .push(
                                        Text::new("PLAYER NUMBER:")
                                            .horizontal_alignment(HorizontalAlignment::Center)
                                            .vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .push(
                                        Text::new(player_num.to_string())
                                            .size(LARGE_TEXT)
                                            .width(Length::Fill)
                                            .horizontal_alignment(HorizontalAlignment::Right)
                                            .vertical_alignment(VerticalAlignment::Center),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button(&mut states.seven, "7", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Seven,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.eight, "8", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Eight,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.nine, "9", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Nine,
                                            )),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button(&mut states.four, "4", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Four,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.five, "5", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Five,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.six, "6", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Six,
                                            )),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button(&mut states.one, "1", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::One,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.two, "2", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Two,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.three, "3", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Three,
                                            )),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button(&mut states.zero, "0", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Zero,
                                            )),
                                    )
                                    .push(
                                        make_small_button(&mut states.delete, "⬅︎", MEDIUM_TEXT)
                                            .width(Length::Units(
                                                2 * MIN_BUTTON_SIZE as u16 + SPACING,
                                            ))
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Delete,
                                            )),
                                    ),
                            ),
                    )
                    .style(style::Container::LightGray)
                    .padding(PADDING),
                )
                .push(match page {
                    KeypadPage::AddScore(color) => {
                        make_add_score_page(&mut states.add_score, *color)
                    }
                }),
        )
        .into()
}

fn make_add_score_page<'a>(
    states: &'a mut AddScoreStates,
    color: GameColor,
) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (style::Button::BlackSelected, style::Button::White),
        GameColor::White => (style::Button::Black, style::Button::WhiteSelected),
    };

    Column::new()
        .spacing(SPACING)
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.black, "BLACK")
                        .style(black_style)
                        .on_press(Message::ChangeColor(GameColor::Black)),
                )
                .push(
                    make_button(&mut states.white, "WHITE")
                        .style(white_style)
                        .on_press(Message::ChangeColor(GameColor::White)),
                ),
        )
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::AddScoreComplete { canceled: true }),
                )
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::AddScoreComplete { canceled: false }),
                ),
        )
        .into()
}

fn build_timeout_ribbon<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut TimeoutRibbonStates,
    tm: &Arc<Mutex<TournamentManager>>,
) -> Row<'a, Message> {
    let in_timeout = match snapshot.timeout {
        TimeoutSnapshot::None => false,
        _ => true,
    };

    let mut black = make_button(
        &mut states.black_timeout,
        if in_timeout {
            "SWITCH TO\nBLACK"
        } else {
            "BLACK\nTIMEOUT"
        },
    )
    .style(style::Button::Black);

    let mut white = make_button(
        &mut states.white_timeout,
        if in_timeout {
            "SWITCH TO\nWHITE"
        } else {
            "WHITE\nTIMEOUT"
        },
    )
    .style(style::Button::White);

    let mut referee = make_button(
        &mut states.ref_timeout,
        if in_timeout {
            "SWITCH TO\nREF"
        } else {
            "REF\nTIMEOUT"
        },
    )
    .style(style::Button::Yellow);

    let mut penalty = make_button(
        &mut states.penalty_shot,
        if in_timeout {
            "SWITCH TO\nPENALTY SHOT"
        } else {
            "PENALTY\nSHOT"
        },
    )
    .style(style::Button::Red);

    let tm = tm.lock().unwrap();
    if (in_timeout & tm.can_switch_to_b_timeout().is_ok())
        | (!in_timeout & tm.can_start_b_timeout().is_ok())
    {
        black = black.on_press(Message::BlackTimeout(in_timeout));
    }
    if (in_timeout & tm.can_switch_to_w_timeout().is_ok())
        | (!in_timeout & tm.can_start_w_timeout().is_ok())
    {
        white = white.on_press(Message::WhiteTimeout(in_timeout));
    }
    if (in_timeout & tm.can_switch_to_ref_timeout().is_ok())
        | (!in_timeout & tm.can_start_ref_timeout().is_ok())
    {
        referee = referee.on_press(Message::RefTimeout(in_timeout));
    }
    if (in_timeout & tm.can_switch_to_penalty_shot().is_ok())
        | (!in_timeout & tm.can_start_penalty_shot().is_ok())
    {
        penalty = penalty.on_press(Message::PenaltyShot(in_timeout));
    }

    Row::new()
        .spacing(SPACING)
        .push(black)
        .push(referee)
        .push(penalty)
        .push(white)
}

fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    state: &'a mut button::State,
) -> Button<'a, Message> {
    let (period_text, period_color) = period_text_and_color(snapshot.current_period);
    let mut content = Row::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .align_items(Align::Center)
        .push(
            Text::new(period_text)
                .color(period_color)
                .width(Length::Fill)
                .vertical_alignment(VerticalAlignment::Center)
                .horizontal_alignment(HorizontalAlignment::Right),
        )
        .push(
            Text::new(time_string(snapshot.secs_in_period))
                .color(period_color)
                .size(LARGE_TEXT)
                .width(Length::Fill)
                .vertical_alignment(VerticalAlignment::Center)
                .horizontal_alignment(HorizontalAlignment::Left),
        );

    if let Some((text, color)) = match snapshot.timeout {
        TimeoutSnapshot::White(_) => Some(("WHITE TIMEOUT", WHITE)),
        TimeoutSnapshot::Black(_) => Some(("BLACK TIMEOUT", BLACK)),
        TimeoutSnapshot::Ref(_) => Some(("REF TIMEOUT", YELLOW)),
        TimeoutSnapshot::PenaltyShot(_) => Some(("PENALTY SHOT", RED)),
        TimeoutSnapshot::None => None,
    } {
        content = content
            .push(
                Text::new(text)
                    .color(color)
                    .width(Length::Fill)
                    .vertical_alignment(VerticalAlignment::Center)
                    .horizontal_alignment(HorizontalAlignment::Right),
            )
            .push(
                Text::new(timeout_time_string(snapshot))
                    .width(Length::Fill)
                    .vertical_alignment(VerticalAlignment::Center)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .color(color)
                    .size(LARGE_TEXT),
            );
    };

    Button::new(state, content)
        .width(Length::Fill)
        .min_height(MIN_BUTTON_SIZE)
        .style(style::Button::Gray)
}

fn time_string(time: u16) -> String {
    secs_to_time_string(time).trim().to_string()
}

fn timeout_time_string(snapshot: &GameSnapshot) -> String {
    match snapshot.timeout {
        TimeoutSnapshot::Black(secs)
        | TimeoutSnapshot::White(secs)
        | TimeoutSnapshot::Ref(secs)
        | TimeoutSnapshot::PenaltyShot(secs) => secs_to_time_string(secs).trim().to_string(),
        TimeoutSnapshot::None => return String::new(),
    }
}

fn period_text_and_color(period: GamePeriod) -> (&'static str, Color) {
    match period {
        GamePeriod::BetweenGames => ("NEXT GAME", YELLOW),
        GamePeriod::FirstHalf => ("FIRST HALF", GREEN),
        GamePeriod::HalfTime => ("HALF TIME", YELLOW),
        GamePeriod::SecondHalf => ("SECOND HALF", GREEN),
        GamePeriod::PreOvertime => ("PRE OVERTIME BREAK", YELLOW),
        GamePeriod::OvertimeFirstHalf => ("OVERTIME FIRST HALF", GREEN),
        GamePeriod::OvertimeHalfTime => ("OVERITME HALF TIME", YELLOW),
        GamePeriod::OvertimeSecondHalf => ("OVERTIME SECOND HALF", GREEN),
        GamePeriod::PreSuddenDeath => ("PRE SUDDEN DEATH BREAK", YELLOW),
        GamePeriod::SuddenDeath => ("SUDDEN DEATH", GREEN),
    }
}

fn make_button<'a, Message: Clone>(
    state: &'a mut button::State,
    label: &str,
) -> Button<'a, Message> {
    Button::new(
        state,
        Text::new(label)
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center)
            .width(Length::Fill),
    )
    .padding(PADDING)
    .min_height(MIN_BUTTON_SIZE)
    .min_width(MIN_BUTTON_SIZE)
    .width(Length::Fill)
}

fn make_small_button<'a, Message: Clone>(
    state: &'a mut button::State,
    label: &str,
    size: u16,
) -> Button<'a, Message> {
    Button::new(
        state,
        Text::new(label)
            .size(size)
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center)
            .width(Length::Fill),
    )
    .width(Length::Units(MIN_BUTTON_SIZE as u16))
    .height(Length::Units(MIN_BUTTON_SIZE as u16))
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
    keypad_page: KeypadPageStates,
}

#[derive(Clone, Default, Debug)]
struct MainViewStates {
    black_score: button::State,
    white_score: button::State,
    black_new_score: button::State,
    white_new_score: button::State,
    black_penalties: button::State,
    white_penalties: button::State,
    game_time: button::State,
    start_now: button::State,
}

#[derive(Clone, Default, Debug)]
struct TimeEditViewStates {
    game_time: button::State,
    min_up: button::State,
    min_down: button::State,
    sec_up: button::State,
    sec_down: button::State,
    timeout_min_up: button::State,
    timeout_min_down: button::State,
    timeout_sec_up: button::State,
    timeout_sec_down: button::State,
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
}

#[derive(Clone, Default, Debug)]
struct AddScoreStates {
    black: button::State,
    white: button::State,
    done: button::State,
    cancel: button::State,
}

#[derive(Clone, Default, Debug)]
struct TimeoutRibbonStates {
    white_timeout: button::State,
    ref_timeout: button::State,
    penalty_shot: button::State,
    black_timeout: button::State,
}
