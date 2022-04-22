use super::{
    style::{
        self, BLACK, GREEN, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, ORANGE, PADDING, RED,
        SMALL_TEXT, SPACING, WHITE, YELLOW,
    },
    *,
};
use iced::{
    button, widget::svg, Align, Button, Color, Column, Container, Element, HorizontalAlignment,
    Length, Row, Space, Svg, Text, VerticalAlignment,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{
        Color as GameColor, GamePeriod, GameSnapshot, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
    },
};
use uwh_matrix_drawing::secs_to_time_string;

pub const PENALTY_LIST_LEN: usize = 3;

pub(super) fn build_main_view<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut MainViewStates,
    config: &GameConfig,
) -> Element<'a, Message> {
    let (period_text, period_color) = period_text_and_color(snapshot.current_period);
    let game_time_info = Column::new()
        .width(Length::Fill)
        .align_items(Align::Center)
        .push(Text::new(period_text).color(period_color))
        .push(
            Text::new(secs_to_time_string(snapshot.secs_in_period).trim())
                .color(period_color)
                .size(LARGE_TEXT),
        );

    let time_button_content: Element<'a, Message> = if snapshot.timeout == TimeoutSnapshot::None {
        game_time_info.width(Length::Fill).into()
    } else {
        let (text, color) = match snapshot.timeout {
            TimeoutSnapshot::Black(_) => ("BLK TIMEOUT", BLACK),
            TimeoutSnapshot::White(_) => ("WHT TIMEOUT", WHITE),
            TimeoutSnapshot::Ref(_) => ("REF TIMEOUT", YELLOW),
            TimeoutSnapshot::PenaltyShot(_) => ("PENALTY SHOT", RED),
            TimeoutSnapshot::None => unreachable!(),
        };
        Row::new()
            .spacing(SPACING)
            .push(game_time_info)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new(text).color(color))
                    .push(
                        Text::new(timeout_time_string(snapshot))
                            .color(color)
                            .size(LARGE_TEXT),
                    ),
            )
            .into()
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
                .size(SMALL_TEXT)
                .vertical_alignment(VerticalAlignment::Center)
                .horizontal_alignment(HorizontalAlignment::Left),
        )
        .padding(PADDING)
        .style(style::Button::LightGray)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::EditGameConfig),
    );

    let make_penalty_button =
        |state: &'a mut button::State, penalties: &[PenaltySnapshot]| -> Button<'a, _> {
            Button::new(
                state,
                Column::new()
                    .spacing(SPACING)
                    .push(
                        Text::new("Penalties")
                            .vertical_alignment(VerticalAlignment::Center)
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .width(Length::Fill),
                    )
                    .push(
                        Text::new(penalty_string(penalties))
                            .vertical_alignment(VerticalAlignment::Top)
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::PenaltyOverview)
        };

    let black_col = Column::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .width(Length::Fill)
        .push(
            Button::new(
                &mut states.black_score,
                Column::new()
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
            make_penalty_button(&mut states.black_penalties, &snapshot.b_penalties)
                .style(style::Button::Black),
        );

    let white_col = Column::new()
        .spacing(SPACING)
        .align_items(Align::Center)
        .width(Length::Fill)
        .push(
            Button::new(
                &mut states.white_score,
                Column::new()
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
            make_penalty_button(&mut states.white_penalties, &snapshot.w_penalties)
                .style(style::Button::White),
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

pub(super) fn build_penalty_overview_page<'a>(
    snapshot: &GameSnapshot,
    states: &'a mut PenaltyOverviewStates,
    penalties: BlackWhiteBundle<Vec<(String, FormatHint, PenaltyKind)>>,
    indices: BlackWhiteBundle<usize>,
) -> Element<'a, Message> {
    Column::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, &mut states.game_time).on_press(Message::EditTime))
        .push(
            Row::new()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_penalty_list(
                    &mut states.b_list,
                    penalties.black,
                    indices.black,
                    GameColor::Black,
                ))
                .push(make_penalty_list(
                    &mut states.w_list,
                    penalties.white,
                    indices.white,
                    GameColor::White,
                )),
        )
        .push(
            Row::new()
                .spacing(SPACING)
                .push(
                    make_button(&mut states.cancel, "CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::PenaltyOverviewComplete { canceled: true }),
                )
                .push(
                    make_button(&mut states.new, "NEW")
                        .style(style::Button::Blue)
                        .width(Length::Fill)
                        .on_press(Message::KeypadPage(KeypadPage::Penalty(
                            None,
                            GameColor::Black,
                            PenaltyKind::OneMinute,
                        ))),
                )
                .push(
                    make_button(&mut states.done, "DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::PenaltyOverviewComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_penalty_list(
    states: &mut PenaltyListStates,
    penalties: Vec<(String, FormatHint, PenaltyKind)>,
    index: usize,
    color: GameColor,
) -> Container<'_, Message> {
    let mut pen_col = Column::new().spacing(SPACING).width(Length::Fill).push(
        Text::new(format!("{} PENALTIES", color.to_string().to_uppercase()))
            .height(Length::Fill)
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center),
    );

    let num_pens = penalties.len();

    let iter = penalties
        .into_iter()
        .enumerate()
        .skip(index)
        .take(PENALTY_LIST_LEN)
        .map(Some)
        .chain([None].into_iter().cycle())
        .zip(states.list.iter_mut());

    for (pen, state) in iter {
        pen_col = pen_col.push(if let Some((i, (text, format, kind))) = pen {
            let mut text = Text::new(text)
                .vertical_alignment(VerticalAlignment::Center)
                .horizontal_alignment(HorizontalAlignment::Left)
                .width(Length::Fill);

            match format {
                FormatHint::NoChange => {}
                FormatHint::Edited => text = text.color(ORANGE),
                FormatHint::Deleted => text = text.color(RED),
                FormatHint::New => text = text.color(GREEN),
            }

            Button::new(state, text)
                .padding(PADDING)
                .min_height(MIN_BUTTON_SIZE)
                .width(Length::Fill)
                .style(style::Button::Gray)
                .on_press(Message::KeypadPage(KeypadPage::Penalty(
                    Some((color, i)),
                    color,
                    kind,
                )))
        } else {
            Button::new(state, Space::new(Length::Shrink, Length::Shrink))
                .min_height(MIN_BUTTON_SIZE)
                .width(Length::Fill)
                .style(style::Button::Gray)
                .on_press(Message::KeypadPage(KeypadPage::Penalty(
                    None,
                    color,
                    PenaltyKind::OneMinute,
                )))
        });
    }

    let top_len;
    let bottom_len;
    let can_scroll_up;
    let can_scroll_down;

    if num_pens <= PENALTY_LIST_LEN {
        top_len = 0;
        bottom_len = 0;
        can_scroll_up = false;
        can_scroll_down = false;
    } else {
        top_len = index as u16;
        bottom_len = (num_pens - PENALTY_LIST_LEN - index) as u16;
        can_scroll_up = index > 0;
        can_scroll_down = index + PENALTY_LIST_LEN < num_pens;
    }

    let top_len = match top_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let bottom_len = match bottom_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let mut up_btn = Button::new(
        &mut states.up,
        Container::new(Svg::new(svg::Handle::from_memory(
            &include_bytes!("../../arrow_drop_up_white_48dp.svg")[..],
        )))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y(),
    )
    .width(Length::Units(MIN_BUTTON_SIZE as u16))
    .height(Length::Units(MIN_BUTTON_SIZE as u16))
    .style(style::Button::Blue);

    let mut down_btn = Button::new(
        &mut states.down,
        Container::new(Svg::new(svg::Handle::from_memory(
            &include_bytes!("../../arrow_drop_down_white_48dp.svg")[..],
        )))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y(),
    )
    .width(Length::Units(MIN_BUTTON_SIZE as u16))
    .height(Length::Units(MIN_BUTTON_SIZE as u16))
    .style(style::Button::Blue);

    if can_scroll_up {
        up_btn = up_btn.on_press(Message::Scroll { color, up: true });
    }

    if can_scroll_down {
        down_btn = down_btn.on_press(Message::Scroll { color, up: false });
    }

    let cont_style = match color {
        GameColor::Black => style::Container::Black,
        GameColor::White => style::Container::White,
    };

    let scroll_bar = Row::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(Space::new(Length::Fill, Length::Shrink))
        .push(
            Container::new(
                Column::new()
                    .push(Space::new(Length::Shrink, top_len))
                    .push(
                        Container::new(Space::new(Length::Fill, Length::Fill))
                            .width(Length::Fill)
                            .height(Length::FillPortion(PENALTY_LIST_LEN as u16))
                            .style(style::Container::Gray),
                    )
                    .push(Space::new(Length::Shrink, bottom_len)),
            )
            .padding(PADDING)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(style::Container::LightGray),
        )
        .push(Space::new(Length::Fill, Length::Shrink));

    Container::new(
        Row::new()
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(pen_col)
            .push(
                Column::new()
                    .spacing(SPACING)
                    .width(Length::Units(MIN_BUTTON_SIZE as u16))
                    .height(Length::Fill)
                    .push(up_btn)
                    .push(scroll_bar)
                    .push(down_btn),
            ),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(cont_style)
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
                                        Button::new(
                                            &mut states.delete,
                                            Container::new(Svg::new(svg::Handle::from_memory(&include_bytes!(
                                                "../../backspace_white_48dp.svg"
                                            )[..]))).width(Length::Fill).center_x()
                                        )
                                        .padding((MIN_BUTTON_SIZE as u16 - MEDIUM_TEXT) / 2)
                                        .width(Length::Units(2 * MIN_BUTTON_SIZE as u16 + SPACING))
                                        .height(Length::Units(MIN_BUTTON_SIZE as u16))
                                        .style(style::Button::Blue)
                                        .on_press(Message::KeypadButtonPress(KeypadButton::Delete)),
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
                    KeypadPage::Penalty(origin, color, kind) => {
                        make_edit_penalty_page(&mut states.eidt_penalty, origin, color, kind)
                    }
                    KeypadPage::GameNumber => make_game_number_edit_page(&mut states.edit_game_num),
                    KeypadPage::TeamTimeouts(dur) => {
                        make_team_timeout_edit_page(&mut states.team_timeout, dur)
                    }
                }),
        )
        .into()
}

fn make_add_score_page(states: &mut AddScoreStates, color: GameColor) -> Element<'_, Message> {
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

fn make_edit_penalty_page(
    states: &mut EditPenaltyStates,
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    kind: PenaltyKind,
) -> Element<'_, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (style::Button::BlackSelected, style::Button::White),
        GameColor::White => (style::Button::Black, style::Button::WhiteSelected),
    };

    let (one_min_style, two_min_style, five_min_style, td_style) = match kind {
        PenaltyKind::OneMinute => (
            style::Button::GreenSelected,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::Red,
        ),
        PenaltyKind::TwoMinute => (
            style::Button::Green,
            style::Button::YellowSelected,
            style::Button::Orange,
            style::Button::Red,
        ),
        PenaltyKind::FiveMinute => (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::OrangeSelected,
            style::Button::Red,
        ),
        PenaltyKind::TotalDismissal => (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::RedSelected,
        ),
    };

    let mut exit_row = Row::new().spacing(SPACING).push(
        make_button(&mut states.cancel, "CANCEL")
            .style(style::Button::Red)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: true,
                deleted: false,
            }),
    );

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button(&mut states.delete, "DELETE")
                .style(style::Button::Orange)
                .width(Length::Fill)
                .on_press(Message::PenaltyEditComplete {
                    canceled: false,
                    deleted: true,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button(&mut states.done, "DONE")
            .style(style::Button::Green)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: false,
                deleted: false,
            }),
    );

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
                    make_button(&mut states.one_min, "1m")
                        .style(one_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::OneMinute)),
                )
                .push(
                    make_button(&mut states.two_min, "2m")
                        .style(two_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::TwoMinute)),
                )
                .push(
                    make_button(&mut states.five_min, "5m")
                        .style(five_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::FiveMinute)),
                )
                .push(
                    make_button(&mut states.total_dismis, "TD")
                        .style(td_style)
                        .on_press(Message::ChangeKind(PenaltyKind::TotalDismissal)),
                ),
        )
        .push(Space::new(Length::Shrink, Length::Fill))
        .push(exit_row)
        .into()
}

fn make_game_number_edit_page(states: &mut EditGameNumStates) -> Element<'_, Message> {
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

fn make_team_timeout_edit_page(
    states: &mut EditTeamTimeoutStates,
    duration: Duration,
) -> Element<'_, Message> {
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
                    Some(Message::EditParameter(LengthParameter::Half)),
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
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
                ))
                .push(make_value_button(
                    &mut states.pre_ot_break,
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    if config.has_overtime {
                        Some(Message::EditParameter(LengthParameter::PreOvertime))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    &mut states.pre_sd_break,
                    "PRE SD\nBREAK LENGTH:",
                    time_string(config.pre_sudden_death_duration),
                    if config.sudden_death_allowed {
                        Some(Message::EditParameter(LengthParameter::PreSuddenDeath))
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
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                ))
                .push(make_value_button(
                    &mut states.ot_half_length,
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    if config.has_overtime {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    &mut states.team_timeouts,
                    "NUM TEAM T/Os\nALLWD PER HALF:",
                    config.team_timeouts_per_half.to_string(),
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        config.team_timeout_duration,
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
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                ))
                .push(make_value_button(
                    &mut states.ot_half_time_length,
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    if config.has_overtime {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
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
    param: LengthParameter,
    length: Duration,
) -> Element<'a, Message> {
    let (title, hint) = match param {
        LengthParameter::Half => ("HALF LEN", "The length of a half during regular play"),
        LengthParameter::HalfTime => ("HALF TIME LEN", "The length of the Half Time period"),
        LengthParameter::NominalBetweenGame => ("NOM BREAK", "If a game runs exactly as long as scheduled, this is the length of the break between games"),
        LengthParameter::MinimumBetweenGame => ("MIN BREAK", "If a game runs longer than scheduled, this is the minimum time between games that the system will allot. If the games fall behind, the system will automatically try to catch up after subsequent games, always respecting this minimum time between games."),
        LengthParameter::PreOvertime => ("PRE OT BREAK", "If overtime is enabled and needed, this is the length of the break between Second Half and Overtime First Half"),
        LengthParameter::OvertimeHalf => ("OT HALF LEN", "The length of a half during overtime"),
        LengthParameter::OvertimeHalfTime => ("OT HLF TM LEN", "The length of Overtime Half Time"),
        LengthParameter::PreSuddenDeath => ("PRE SD BREAK", "The length of the break between the preceeding play period and Sudden Death"),
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
                .size(SMALL_TEXT)
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
    kind: &ConfirmationKind,
) -> Element<'a, Message> {
    let header_text = match kind {
        ConfirmationKind::GameConfigChanged => "The game configuration can not be changed while a game is in progress.\n\nWhat would you like to do?",
        ConfirmationKind::GameNumberChanged => "How would you like to apply this game number change?",
        ConfirmationKind::Error(string) => string,
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
        ConfirmationKind::Error(_) => {
            vec![(
                "OK",
                style::Button::Green,
                ConfirmationOption::DiscardChanges,
            )]
        }
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
    let in_timeout = !matches!(snapshot.timeout, TimeoutSnapshot::None);

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
            "SWITCH TO\nPEN SHOT"
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
            Text::new(secs_to_time_string(snapshot.secs_in_period).trim())
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

fn make_time_editor<T: Into<String>>(
    states: &mut TimeEditorStates,
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'_, Message> {
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
                        Text::new(time_string(time))
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

fn time_string(time: Duration) -> String {
    secs_to_time_string(time.as_secs()).trim().to_string()
}

fn timeout_time_string(snapshot: &GameSnapshot) -> String {
    match snapshot.timeout {
        TimeoutSnapshot::Black(secs)
        | TimeoutSnapshot::White(secs)
        | TimeoutSnapshot::Ref(secs)
        | TimeoutSnapshot::PenaltyShot(secs) => secs_to_time_string(secs).trim().to_string(),
        TimeoutSnapshot::None => String::new(),
    }
}

fn bool_string(val: bool) -> String {
    match val {
        true => "YES".to_string(),
        false => "NO".to_string(),
    }
}

fn penalty_string(penalties: &[PenaltySnapshot]) -> String {
    use std::fmt::Write;

    let mut string = String::new();

    for pen in penalties.iter() {
        write!(&mut string, "#{} - ", pen.player_number).unwrap();
        match pen.time {
            PenaltyTime::Seconds(secs) => {
                if secs != 0 {
                    writeln!(&mut string, "{}:{:02}", secs / 60, secs % 60).unwrap();
                } else {
                    string.push_str("Served\n");
                }
            }
            PenaltyTime::TotalDismissal => string.push_str("DSMS\n"),
        }
    }
    // if the string is not empty, the last char is a '\n' that we don't want
    string.pop();
    string
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

fn make_button<Message: Clone, T: Into<String>>(
    state: &mut button::State,
    label: T,
) -> Button<'_, Message> {
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

fn make_small_button<Message: Clone, T: Into<String>>(
    state: &mut button::State,
    label: T,
    size: u16,
) -> Button<'_, Message> {
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
            .push(
                Text::new(first_label)
                    .size(SMALL_TEXT)
                    .vertical_alignment(VerticalAlignment::Center),
            )
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
