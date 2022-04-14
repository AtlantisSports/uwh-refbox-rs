use super::{
    style::{
        self, BLACK, GREEN, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, RED, SPACING, WHITE,
        YELLOW,
    },
    *,
};
use iced::{
    button, Align, Button, Color, Column, Container, Element, HorizontalAlignment, Length, Row,
    Space, Text, VerticalAlignment,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{Color as GameColor, GamePeriod, GameSnapshot, TimeoutSnapshot},
};
use uwh_matrix_drawing::secs_to_time_string;

pub(super) fn build_main_view<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut MainViewStates,
    config: &GameConfig,
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
                            .on_press(Message::StartPlayNow),
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
        Button::new(
            &mut states.game_config,
            Text::new(config_string(snapshot, config))
                .vertical_alignment(VerticalAlignment::Center)
                .horizontal_alignment(HorizontalAlignment::Left),
        )
        .padding(PADDING)
        .style(style::Button::LightGray)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::EditGameConfig),
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

pub(super) fn build_time_edit_view<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut TimeEditViewStates,
    time: Duration,
    timeout_time: Option<Duration>,
) -> Element<'a, Message> {
    let mut edit_row = Row::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .push(Space::new(Length::Fill, Length::Shrink))
        .push(make_time_editor(
            &mut states.game_time_edit,
            "GAME TIME",
            time,
            false,
        ))
        .push(Space::new(Length::Fill, Length::Shrink));

    if snapshot.timeout != TimeoutSnapshot::None {
        edit_row = edit_row
            .push(Space::new(Length::Fill, Length::Shrink))
            .push(make_time_editor(
                &mut states.timeout_time_edit,
                "TIMEOUT",
                timeout_time.unwrap(),
                true,
            ))
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

pub(super) fn build_score_edit_view<'a>(
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

pub(super) fn build_keypad_page<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut KeypadPageStates,
    page: KeypadPage,
    player_num: u16,
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
                                        Text::new(page.text())
                                            .horizontal_alignment(HorizontalAlignment::Left)
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
                                        make_small_button(&mut states.delete, "<-", MEDIUM_TEXT)
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
                        make_add_score_page(&mut states.add_score, color)
                    }
                    KeypadPage::GameNumber => make_game_number_edit_page(&mut states.edit_game_num),
                    KeypadPage::TeamTimeouts(dur) => {
                        make_team_timeout_edit_page(&mut states.team_timeout, dur)
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

fn make_game_number_edit_page<'a>(states: &'a mut EditGameNumStates) -> Element<'a, Message> {
    Column::new()
        .spacing(SPACING)
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_team_timeout_edit_page<'a>(
    states: &'a mut EditTeamTimeoutStates,
    duration: Duration,
) -> Element<'a, Message> {
    Column::new()
        .spacing(SPACING)
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(make_time_editor(
                    &mut states.length_edit,
                    "TIMEOUT LENGTH",
                    duration,
                    false,
                ))
                .push(Space::new(Length::Fill, Length::Shrink)),
        )
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}

pub(super) fn build_game_config_edit_page<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut GameConfigEditStates,
    config: &GameConfig,
    game_number: u16,
) -> Element<'a, Message> {
    Column::new()
        .spacing(SPACING)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::EditTime))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(make_value_button(
                    &mut states.using_uwhscores,
                    "USING UWHSCORES:",
                    "NO",
                    Some(Message::NoAction),
                ))
                .push(make_value_button(
                    &mut states.game_number,
                    "GAME NUMBER:",
                    game_number.to_string(),
                    Some(Message::KeypadPage(KeypadPage::GameNumber)),
                )),
        )
        .push(
            Row::new()
                .spacing(SPACING)
                .push(make_value_button(
                    &mut states.half_length,
                    "HALF LENGTH:",
                    time_string(config.half_play_duration),
                    Some(Message::EditParameter(GameParameter::HalfLength)),
                ))
                .push(make_value_button(
                    &mut states.overtime_allowed,
                    "OVERTIME\nALLOWED:",
                    bool_string(config.has_overtime),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::OvertimeAllowed,
                    )),
                ))
                .push(make_value_button(
                    &mut states.sd_allowed,
                    "SUDDEN DEATH\nALLOWED:",
                    bool_string(config.sudden_death_allowed),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::SuddenDeathAllowed,
                    )),
                )),
        )
        .push(
            Row::new()
                .spacing(SPACING)
                .push(make_value_button(
                    &mut states.half_time_length,
                    "HALF TIME\nLENGTH:",
                    time_string(config.half_time_duration),
                    Some(Message::EditParameter(GameParameter::HalfTimeLength)),
                ))
                .push(make_value_button(
                    &mut states.pre_ot_break,
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    if config.has_overtime {
                        Some(Message::EditParameter(GameParameter::PreOvertimeLength))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    &mut states.pre_sd_break,
                    "PRE SD\nBREAK LENGTH:",
                    time_string(config.pre_sudden_death_duration),
                    if config.sudden_death_allowed {
                        Some(Message::EditParameter(GameParameter::PreSuddenDeathLength))
                    } else {
                        None
                    },
                )),
        )
        .push(
            Row::new()
                .spacing(SPACING)
                .push(make_value_button(
                    &mut states.nom_between_games,
                    "NOMINAL BRK\nBTWN GAMES:",
                    time_string(config.nominal_break),
                    Some(Message::EditParameter(
                        GameParameter::NominalBetweenGameLength,
                    )),
                ))
                .push(make_value_button(
                    &mut states.ot_half_length,
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    if config.has_overtime {
                        Some(Message::EditParameter(GameParameter::OvertimeHalfLength))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    &mut states.team_timeouts,
                    "NUM TEAM T/Os\nALLWD PER HALF:",
                    config.team_timeouts_per_half.to_string(),
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        Duration::from_secs(config.team_timeout_duration.into()),
                    ))),
                )),
        )
        .push(
            Row::new()
                .spacing(SPACING)
                .push(make_value_button(
                    &mut states.min_between_games,
                    "MINIMUM BRK\nBTWN GAMES:",
                    time_string(config.minimum_break),
                    Some(Message::EditParameter(
                        GameParameter::MinimumBetweenGameLength,
                    )),
                ))
                .push(make_value_button(
                    &mut states.ot_half_time_length,
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    if config.has_overtime {
                        Some(Message::EditParameter(
                            GameParameter::OvertimeHalfTimeLength,
                        ))
                    } else {
                        None
                    },
                ))
                .push(
                    Row::new()
                        .spacing(SPACING)
                        .width(Length::Fill)
                        .push(
                            make_button(&mut states.cancel, "CANCEL")
                                .style(style::Button::Red)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: true }),
                        )
                        .push(
                            make_button(&mut states.done, "DONE")
                                .style(style::Button::Green)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: false }),
                        ),
                ),
        )
        .into()
}

pub(super) fn build_game_parameter_editor<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut GameParamEditStates,
    param: GameParameter,
    length: Duration,
) -> Element<'a, Message> {
    let (title, hint) = match param {
        GameParameter::HalfLength => ("HALF LEN", "The length of a half during regular play"),
        GameParameter::HalfTimeLength => ("HALF TIME LEN", "The length of the Half Time period"),
        GameParameter::NominalBetweenGameLength => ("NOM BREAK", "If a game runs exactly as long as scheduled, this is the length of the break between games"),
        GameParameter::MinimumBetweenGameLength => ("MIN BREAK", "If a game runs longer than scheduled, this is the minimum time between games that the system will allot. If the games fall behind, the system will automatically try to catch up after subsequent games, always respecting this minimum time between games."),
        GameParameter::PreOvertimeLength => ("PRE OT BREAK", "If overtime is enabled and needed, this is the length of the break between Second Half and Overtime First Half"),
        GameParameter::OvertimeHalfLength => ("OT HALF LEN", "The length of a half during overtime"),
        GameParameter::OvertimeHalfTimeLength => ("OT HLF TM LEN", "The length of Overtime Half Time"),
        GameParameter::PreSuddenDeathLength => ("PRE SD BREAK", "The length of the break between the preceeding play period and Sudden Death"),
    };

    Column::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::EditTime))
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(make_time_editor(
            &mut states.length_edit,
            title,
            length,
            false,
        ))
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Text::new(String::from("Help: ") + hint)
                .horizontal_alignment(HorizontalAlignment::Center),
        )
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}

pub(super) fn build_confirmation_page<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut ConfirmationPageStates,
    kind: ConfirmationKind,
) -> Element<'a, Message> {
    let header_text = match kind {
        ConfirmationKind::GameConfigChanged => "The game configuration can not be changed while a game is in progress.\n\nWhat would you like to do?",
        ConfirmationKind::GameNumberChanged => "How would you like to apply this game number change?",
            };

    let buttons = match kind {
        ConfirmationKind::GameConfigChanged => vec![
            (
                "GO BACK TO EDITOR",
                style::Button::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                style::Button::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                "END CURRENT GAME AND APPLY CHANGES",
                style::Button::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::GameNumberChanged => vec![
            (
                "GO BACK TO EDITOR",
                style::Button::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                style::Button::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                "KEEP CURRENT GAME AND APPLY CHANGE",
                style::Button::Orange,
                ConfirmationOption::KeepGameAndApply,
            ),
            (
                "END CURRENT GAME AND APPLY CHANGE",
                style::Button::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
    };

    let buttons = buttons
        .into_iter()
        .zip([
            &mut states.button_1,
            &mut states.button_2,
            &mut states.button_3,
            &mut states.button_4,
        ])
        .map(|((text, style, option), state)| {
            make_button(state, text)
                .style(style)
                .on_press(Message::ConfirmationSelected(option))
        });

    let mut button_col = Column::new().spacing(SPACING).width(Length::Fill);

    for button in buttons {
        button_col = button_col.push(button);
    }

    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Align::Center)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::EditTime))
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(
            Row::new()
                .push(Space::new(Length::Fill, Length::Shrink))
                .push(
                    Container::new(
                        Column::new()
                            .spacing(SPACING)
                            .width(Length::Fill)
                            .align_items(Align::Center)
                            .push(
                                Text::new(header_text)
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .push(button_col),
                    )
                    .width(Length::FillPortion(3))
                    .style(style::Container::LightGray)
                    .padding(PADDING),
                )
                .push(Space::new(Length::Fill, Length::Shrink)),
        )
        .push(Space::new(Length::Shrink, Length::Fill))
        .into()
}

pub(super) fn build_timeout_ribbon<'a>(
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

fn make_time_editor<'a, T: Into<String>>(
    states: &'a mut TimeEditorStates,
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'a, Message> {
    Container::new(
        Column::new()
            .spacing(SPACING)
            .align_items(Align::Center)
            .push(Text::new(title).size(MEDIUM_TEXT))
            .push(
                Row::new()
                    .spacing(SPACING)
                    .align_items(Align::Center)
                    .push(
                        Column::new()
                            .spacing(SPACING)
                            .push(
                                make_small_button(&mut states.min_up, "+", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: true,
                                        secs: 60,
                                        timeout,
                                    }),
                            )
                            .push(
                                make_small_button(&mut states.min_down, "-", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: false,
                                        secs: 60,
                                        timeout,
                                    }),
                            ),
                    )
                    .push(
                        Text::new(time_string(time.as_secs().try_into().unwrap()))
                            .size(LARGE_TEXT)
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .width(Length::Units(200)),
                    )
                    .push(
                        Column::new()
                            .spacing(SPACING)
                            .push(
                                make_small_button(&mut states.sec_up, "+", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: true,
                                        secs: 1,
                                        timeout,
                                    }),
                            )
                            .push(
                                make_small_button(&mut states.sec_down, "-", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: false,
                                        secs: 1,
                                        timeout,
                                    }),
                            ),
                    ),
            ),
    )
    .style(style::Container::LightGray)
    .padding(PADDING)
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

fn bool_string(val: bool) -> String {
    match val {
        true => "YES".to_string(),
        false => "NO".to_string(),
    }
}

fn config_string(snapshot: &GameSnapshot, config: &GameConfig) -> String {
    let mut result = if snapshot.current_period == GamePeriod::BetweenGames {
        format!(
            "Previous Game: {}\nUpcoming Game: {}\n",
            snapshot.game_number, snapshot.next_game_number
        )
    } else {
        format!("Current Game: {}\n\n", snapshot.game_number)
    };
    result += &format!(
        "Half Length: {}\n\
         Half Time Length: {}\n\
         Overtime Allowed: {}\n",
        time_string(config.half_play_duration),
        time_string(config.half_time_duration),
        bool_string(config.has_overtime),
    );
    result += &if config.has_overtime {
        format!(
            "Pre-Overtime Break Length: {}\n\
             Overtime Half Length: {}\n\
             Overtime Half Time Length: {}\n",
            time_string(config.pre_overtime_break),
            time_string(config.ot_half_play_duration),
            time_string(config.ot_half_time_duration),
        )
    } else {
        String::new()
    };
    result += &format!(
        "Sudden Death Allowed: {}\n",
        bool_string(config.sudden_death_allowed)
    );
    result += &if config.sudden_death_allowed {
        format!(
            "Pre-Sudden-Death Break Length: {}\n",
            time_string(config.pre_sudden_death_duration)
        )
    } else {
        String::new()
    };
    result += &format!(
        "Team Timeouts Allowed Per Half: {}\n",
        config.team_timeouts_per_half
    );
    result += &if config.team_timeouts_per_half != 0 {
        format!(
            "Team Timeout Duration: {}\n",
            time_string(config.team_timeout_duration)
        )
    } else {
        String::new()
    };
    result += &format!(
        "Nominal Time Between Games: {}\n\
         Minimum Time Between Games: {}\n",
        time_string(config.nominal_break),
        time_string(config.minimum_break),
    );
    result
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

fn make_button<'a, Message: Clone, T: Into<String>>(
    state: &'a mut button::State,
    label: T,
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

fn make_small_button<'a, Message: Clone, T: Into<String>>(
    state: &'a mut button::State,
    label: T,
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

fn make_value_button<'a, Message: 'a + Clone, T: Into<String>, U: Into<String>>(
    state: &'a mut button::State,
    first_label: T,
    second_label: U,
    message: Option<Message>,
) -> Button<'a, Message> {
    let mut button = Button::new(
        state,
        Row::new()
            .spacing(SPACING)
            .align_items(Align::Center)
            .push(Text::new(first_label).vertical_alignment(VerticalAlignment::Center))
            .push(Space::new(Length::Fill, Length::Shrink))
            .push(
                Text::new(second_label)
                    .size(MEDIUM_TEXT)
                    .vertical_alignment(VerticalAlignment::Center),
            ),
    )
    .padding(PADDING)
    .min_height(MIN_BUTTON_SIZE)
    .min_width(MIN_BUTTON_SIZE)
    .width(Length::Fill)
    .style(style::Button::LightGray);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}
