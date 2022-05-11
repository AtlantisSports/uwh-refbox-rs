use super::{
    style::{
        self, BLACK, GREEN, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, ORANGE, PADDING, RED,
        SMALL_TEXT, SPACING, WHITE, YELLOW,
    },
    *,
};
use iced::{
    alignment::{Horizontal, Vertical},
    pure::{
        button, column, container, horizontal_space, row, text, vertical_space,
        widget::{
            svg::{self, Svg},
            Button, Container, Row,
        },
        Element,
    },
    Alignment, Color, Length,
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
    config: &GameConfig,
) -> Element<'a, Message> {
    let (period_text, period_color) = period_text_and_color(snapshot.current_period);
    let game_time_info = column()
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .push(text(period_text).color(period_color))
        .push(
            text(secs_to_time_string(snapshot.secs_in_period).trim())
                .color(period_color)
                .size(LARGE_TEXT),
        );

    let time_button_content: Element<'a, Message> = if snapshot.timeout == TimeoutSnapshot::None {
        game_time_info.width(Length::Fill).into()
    } else {
        let (per_text, color) = match snapshot.timeout {
            TimeoutSnapshot::Black(_) => ("BLK TIMEOUT", BLACK),
            TimeoutSnapshot::White(_) => ("WHT TIMEOUT", WHITE),
            TimeoutSnapshot::Ref(_) => ("REF TIMEOUT", YELLOW),
            TimeoutSnapshot::PenaltyShot(_) => ("PENALTY SHOT", RED),
            TimeoutSnapshot::None => unreachable!(),
        };
        row()
            .spacing(SPACING)
            .push(game_time_info)
            .push(
                column()
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .push(text(per_text).color(color))
                    .push(
                        text(timeout_time_string(snapshot))
                            .color(color)
                            .size(LARGE_TEXT),
                    ),
            )
            .into()
    };

    let mut center_col = column().spacing(SPACING).width(Length::Fill).push(
        button(time_button_content)
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
                make_button("END TIMEOUT")
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
                        make_button("START NOW")
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
        button(
            text(config_string(snapshot, config))
                .size(SMALL_TEXT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Left),
        )
        .padding(PADDING)
        .style(style::Button::LightGray)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::EditGameConfig),
    );

    let make_penalty_button = |penalties: &[PenaltySnapshot]| {
        button(
            column()
                .spacing(SPACING)
                .push(
                    text("Penalties")
                        .vertical_alignment(Vertical::Center)
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill),
                )
                .push(
                    text(penalty_string(penalties))
                        .vertical_alignment(Vertical::Top)
                        .horizontal_alignment(Horizontal::Left)
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

    let black_col = column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .push(
            button(
                column()
                    .align_items(Alignment::Center)
                    .width(Length::Fill)
                    .push("BLACK")
                    .push(text(snapshot.b_score.to_string()).size(LARGE_TEXT)),
            )
            .padding(PADDING)
            .width(Length::Fill)
            .style(style::Button::Black)
            .on_press(Message::EditScores),
        )
        .push(
            make_button("SCORE\nBLACK")
                .style(style::Button::Black)
                .on_press(Message::KeypadPage(KeypadPage::AddScore(GameColor::Black))),
        )
        .push(make_penalty_button(&snapshot.b_penalties).style(style::Button::Black));

    let white_col = column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .push(
            button(
                column()
                    .align_items(Alignment::Center)
                    .width(Length::Fill)
                    .push("WHITE")
                    .push(text(snapshot.w_score.to_string()).size(LARGE_TEXT)),
            )
            .padding(PADDING)
            .width(Length::Fill)
            .style(style::Button::White)
            .on_press(Message::EditScores),
        )
        .push(
            make_button("SCORE\nWHITE")
                .style(style::Button::White)
                .on_press(Message::KeypadPage(KeypadPage::AddScore(GameColor::White))),
        )
        .push(make_penalty_button(&snapshot.w_penalties).style(style::Button::White));

    row()
        .spacing(0)
        .height(Length::Fill)
        .push(
            row()
                .width(Length::Fill)
                .spacing(0)
                .push(black_col)
                .push(horizontal_space(Length::Units(3 * SPACING / 4))),
        )
        .push(
            row()
                .width(Length::FillPortion(2))
                .spacing(0)
                .push(horizontal_space(Length::Units(SPACING / 4)))
                .push(center_col)
                .push(horizontal_space(Length::Units(SPACING / 4))),
        )
        .push(
            row()
                .width(Length::Fill)
                .spacing(0)
                .push(horizontal_space(Length::Units(3 * SPACING / 4)))
                .push(white_col),
        )
        .into()
}

pub(super) fn build_time_edit_view<'a>(
    snapshot: &GameSnapshot,
    time: Duration,
    timeout_time: Option<Duration>,
) -> Element<'a, Message> {
    let mut edit_row = row()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .push(horizontal_space(Length::Fill))
        .push(make_time_editor("GAME TIME", time, false))
        .push(horizontal_space(Length::Fill));

    if snapshot.timeout != TimeoutSnapshot::None {
        edit_row = edit_row
            .push(horizontal_space(Length::Fill))
            .push(make_time_editor("TIMEOUT", timeout_time.unwrap(), true))
            .push(horizontal_space(Length::Fill));
    }

    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot).on_press(Message::NoAction))
        .push(vertical_space(Length::Fill))
        .push(edit_row)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::TimeEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::TimeEditComplete { canceled: false }),
                ),
        )
        .into()
}

pub(super) fn build_score_edit_view<'a>(
    snapshot: &GameSnapshot,
    black: u8,
    white: u8,
) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(horizontal_space(Length::Fill))
                .push(
                    container(
                        row()
                            .spacing(SPACING)
                            .align_items(Alignment::Center)
                            .push(
                                column()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button("+", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::Black,
                                                increase: true,
                                            }),
                                    )
                                    .push(
                                        make_small_button("-", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::Black,
                                                increase: false,
                                            }),
                                    ),
                            )
                            .push(
                                column()
                                    .spacing(SPACING)
                                    .width(Length::Fill)
                                    .align_items(Alignment::Center)
                                    .push("BLACK")
                                    .push(text(black.to_string()).size(LARGE_TEXT)),
                            ),
                    )
                    .padding(PADDING)
                    .width(Length::FillPortion(2))
                    .style(style::Container::Black),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    container(
                        row()
                            .spacing(SPACING)
                            .align_items(Alignment::Center)
                            .push(
                                column()
                                    .spacing(SPACING)
                                    .width(Length::Fill)
                                    .align_items(Alignment::Center)
                                    .push("WHITE")
                                    .push(text(white.to_string()).size(LARGE_TEXT)),
                            )
                            .push(
                                column()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button("+", LARGE_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::ChangeScore {
                                                color: GameColor::White,
                                                increase: true,
                                            }),
                                    )
                                    .push(
                                        make_small_button("-", LARGE_TEXT)
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
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ScoreEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ScoreEditComplete { canceled: false }),
                ),
        )
        .into()
}

pub(super) fn build_penalty_overview_page<'a>(
    snapshot: &GameSnapshot,
    penalties: BlackWhiteBundle<Vec<(String, FormatHint, PenaltyKind)>>,
    indices: BlackWhiteBundle<usize>,
) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_penalty_list(
                    penalties.black,
                    indices.black,
                    GameColor::Black,
                ))
                .push(make_penalty_list(
                    penalties.white,
                    indices.white,
                    GameColor::White,
                )),
        )
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::PenaltyOverviewComplete { canceled: true }),
                )
                .push(
                    make_button("NEW")
                        .style(style::Button::Blue)
                        .width(Length::Fill)
                        .on_press(Message::KeypadPage(KeypadPage::Penalty(
                            None,
                            GameColor::Black,
                            PenaltyKind::OneMinute,
                        ))),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::PenaltyOverviewComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_penalty_list<'a>(
    penalties: Vec<(String, FormatHint, PenaltyKind)>,
    index: usize,
    color: GameColor,
) -> Container<'a, Message> {
    let mut pen_col = column().spacing(SPACING).width(Length::Fill).push(
        text(format!("{} PENALTIES", color.to_string().to_uppercase()))
            .height(Length::Fill)
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center)
            .vertical_alignment(Vertical::Center),
    );

    let num_pens = penalties.len();

    let iter = penalties
        .into_iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(PENALTY_LIST_LEN);

    for pen in iter {
        pen_col = pen_col.push(if let Some((i, (pen_text, format, kind))) = pen {
            let mut text = text(pen_text)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Left)
                .width(Length::Fill);

            match format {
                FormatHint::NoChange => {}
                FormatHint::Edited => text = text.color(ORANGE),
                FormatHint::Deleted => text = text.color(RED),
                FormatHint::New => text = text.color(GREEN),
            }

            button(text)
                .padding(PADDING)
                .height(Length::Units(MIN_BUTTON_SIZE))
                .width(Length::Fill)
                .style(style::Button::Gray)
                .on_press(Message::KeypadPage(KeypadPage::Penalty(
                    Some((color, i)),
                    color,
                    kind,
                )))
        } else {
            button(horizontal_space(Length::Shrink))
                .height(Length::Units(MIN_BUTTON_SIZE))
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

    if num_pens < PENALTY_LIST_LEN {
        top_len = 0;
        bottom_len = 0;
        can_scroll_up = false;
        can_scroll_down = false;
    } else {
        top_len = index as u16;
        bottom_len = (1 + num_pens - PENALTY_LIST_LEN - index) as u16;
        can_scroll_up = index > 0;
        can_scroll_down = index + PENALTY_LIST_LEN <= num_pens;
    }

    let top_len = match top_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let bottom_len = match bottom_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let mut up_btn = button(
        container(Svg::new(svg::Handle::from_memory(
            &include_bytes!("../../arrow_drop_up_white_48dp.svg")[..],
        )))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y(),
    )
    .width(Length::Units(MIN_BUTTON_SIZE))
    .height(Length::Units(MIN_BUTTON_SIZE))
    .style(style::Button::Blue);

    let mut down_btn = button(
        container(Svg::new(svg::Handle::from_memory(
            &include_bytes!("../../arrow_drop_down_white_48dp.svg")[..],
        )))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y(),
    )
    .width(Length::Units(MIN_BUTTON_SIZE))
    .height(Length::Units(MIN_BUTTON_SIZE))
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

    let scroll_bar = row()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(horizontal_space(Length::Fill))
        .push(
            container(
                column()
                    .push(vertical_space(top_len))
                    .push(
                        container(vertical_space(Length::Fill))
                            .width(Length::Fill)
                            .height(Length::FillPortion(PENALTY_LIST_LEN as u16))
                            .style(style::Container::Gray),
                    )
                    .push(vertical_space(bottom_len)),
            )
            .padding(PADDING)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(style::Container::LightGray),
        )
        .push(horizontal_space(Length::Fill));

    container(
        row()
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(pen_col)
            .push(
                column()
                    .spacing(SPACING)
                    .width(Length::Units(MIN_BUTTON_SIZE))
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
    page: KeypadPage,
    player_num: u16,
) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(
                    container(
                        column()
                            .spacing(SPACING)
                            .push(
                                row()
                                    .align_items(Alignment::Center)
                                    .height(Length::Fill)
                                    .width(Length::Units(3 * MIN_BUTTON_SIZE + 2 * SPACING))
                                    .push(
                                        text(page.text())
                                            .horizontal_alignment(Horizontal::Left)
                                            .vertical_alignment(Vertical::Center),
                                    )
                                    .push(
                                        text(player_num.to_string())
                                            .size(LARGE_TEXT)
                                            .width(Length::Fill)
                                            .horizontal_alignment(Horizontal::Right)
                                            .vertical_alignment(Vertical::Center),
                                    ),
                            )
                            .push(
                                row()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button("7", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Seven,
                                            )),
                                    )
                                    .push(
                                        make_small_button("8", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Eight,
                                            )),
                                    )
                                    .push(
                                        make_small_button("9", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Nine,
                                            )),
                                    ),
                            )
                            .push(
                                row()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button("4", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Four,
                                            )),
                                    )
                                    .push(
                                        make_small_button("5", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Five,
                                            )),
                                    )
                                    .push(
                                        make_small_button("6", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Six,
                                            )),
                                    ),
                            )
                            .push(
                                row()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button("1", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::One,
                                            )),
                                    )
                                    .push(
                                        make_small_button("2", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Two,
                                            )),
                                    )
                                    .push(
                                        make_small_button("3", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Three,
                                            )),
                                    ),
                            )
                            .push(
                                row()
                                    .spacing(SPACING)
                                    .push(
                                        make_small_button("0", MEDIUM_TEXT)
                                            .style(style::Button::Blue)
                                            .on_press(Message::KeypadButtonPress(
                                                KeypadButton::Zero,
                                            )),
                                    )
                                    .push(
                                        button(
                                            container(Svg::new(svg::Handle::from_memory(&include_bytes!(
                                                "../../backspace_white_48dp.svg"
                                            )[..]))).width(Length::Fill).center_x()
                                        )
                                        .padding((MIN_BUTTON_SIZE - MEDIUM_TEXT) / 2)
                                        .width(Length::Units(2 * MIN_BUTTON_SIZE + SPACING))
                                        .height(Length::Units(MIN_BUTTON_SIZE))
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
                        make_add_score_page(color)
                    }
                    KeypadPage::Penalty(origin, color, kind) => {
                        make_edit_penalty_page(origin, color, kind)
                    }
                    KeypadPage::GameNumber => make_game_number_edit_page(),
                    KeypadPage::TeamTimeouts(dur) => {
                        make_team_timeout_edit_page(dur)
                    }
                }),
        )
        .into()
}

fn make_add_score_page<'a>(color: GameColor) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (style::Button::BlackSelected, style::Button::White),
        GameColor::White => (style::Button::Black, style::Button::WhiteSelected),
    };

    column()
        .spacing(SPACING)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("BLACK")
                        .style(black_style)
                        .on_press(Message::ChangeColor(GameColor::Black)),
                )
                .push(
                    make_button("WHITE")
                        .style(white_style)
                        .on_press(Message::ChangeColor(GameColor::White)),
                ),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::AddScoreComplete { canceled: true }),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::AddScoreComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_edit_penalty_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    kind: PenaltyKind,
) -> Element<'a, Message> {
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

    let mut exit_row = row().spacing(SPACING).push(
        make_button("CANCEL")
            .style(style::Button::Red)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: true,
                deleted: false,
            }),
    );

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button("DELETE")
                .style(style::Button::Orange)
                .width(Length::Fill)
                .on_press(Message::PenaltyEditComplete {
                    canceled: false,
                    deleted: true,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button("DONE")
            .style(style::Button::Green)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: false,
                deleted: false,
            }),
    );

    column()
        .spacing(SPACING)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("BLACK")
                        .style(black_style)
                        .on_press(Message::ChangeColor(GameColor::Black)),
                )
                .push(
                    make_button("WHITE")
                        .style(white_style)
                        .on_press(Message::ChangeColor(GameColor::White)),
                ),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("1m")
                        .style(one_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::OneMinute)),
                )
                .push(
                    make_button("2m")
                        .style(two_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::TwoMinute)),
                )
                .push(
                    make_button("5m")
                        .style(five_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::FiveMinute)),
                )
                .push(
                    make_button("TD")
                        .style(td_style)
                        .on_press(Message::ChangeKind(PenaltyKind::TotalDismissal)),
                ),
        )
        .push(vertical_space(Length::Fill))
        .push(exit_row)
        .into()
}

fn make_game_number_edit_page<'a>() -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_team_timeout_edit_page<'a>(duration: Duration) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .push(horizontal_space(Length::Fill))
                .push(make_time_editor("TIMEOUT LENGTH", duration, false))
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}

pub(super) fn build_game_config_edit_page<'a>(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    game_number: u16,
) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .push(make_game_time_button(snapshot).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "USING UWHSCORES:",
                    "NO",
                    Some(Message::NoAction),
                ))
                .push(make_value_button(
                    "GAME NUMBER:",
                    game_number.to_string(),
                    Some(Message::KeypadPage(KeypadPage::GameNumber)),
                )),
        )
        .push(
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "HALF LENGTH:",
                    time_string(config.half_play_duration),
                    Some(Message::EditParameter(LengthParameter::Half)),
                ))
                .push(make_value_button(
                    "OVERTIME\nALLOWED:",
                    bool_string(config.has_overtime),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::OvertimeAllowed,
                    )),
                ))
                .push(make_value_button(
                    "SUDDEN DEATH\nALLOWED:",
                    bool_string(config.sudden_death_allowed),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::SuddenDeathAllowed,
                    )),
                )),
        )
        .push(
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "HALF TIME\nLENGTH:",
                    time_string(config.half_time_duration),
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
                ))
                .push(make_value_button(
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    if config.has_overtime {
                        Some(Message::EditParameter(LengthParameter::PreOvertime))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
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
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "NOMINAL BRK\nBTWN GAMES:",
                    time_string(config.nominal_break),
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    if config.has_overtime {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "NUM TEAM T/Os\nALLWD PER HALF:",
                    config.team_timeouts_per_half.to_string(),
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        config.team_timeout_duration,
                    ))),
                )),
        )
        .push(
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "MINIMUM BRK\nBTWN GAMES:",
                    time_string(config.minimum_break),
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    if config.has_overtime {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
                    } else {
                        None
                    },
                ))
                .push(
                    row()
                        .spacing(SPACING)
                        .width(Length::Fill)
                        .push(
                            make_button("CANCEL")
                                .style(style::Button::Red)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: true }),
                        )
                        .push(
                            make_button("DONE")
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

    column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(make_time_editor(title, length, false))
        .push(vertical_space(Length::Fill))
        .push(
            text(String::from("Help: ") + hint)
                .size(SMALL_TEXT)
                .horizontal_alignment(Horizontal::Center),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}

pub(super) fn build_confirmation_page<'a>(
    snapshot: &GameSnapshot,
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

    let buttons = buttons.into_iter().map(|(text, style, option)| {
        make_button(text)
            .style(style)
            .on_press(Message::ConfirmationSelected(option))
    });

    let mut button_col = column().spacing(SPACING).width(Length::Fill);

    for button in buttons {
        button_col = button_col.push(button);
    }

    column()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .push(make_game_time_button(snapshot).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .push(horizontal_space(Length::Fill))
                .push(
                    container(
                        column()
                            .spacing(SPACING)
                            .width(Length::Fill)
                            .align_items(Alignment::Center)
                            .push(text(header_text).horizontal_alignment(Horizontal::Center))
                            .push(button_col),
                    )
                    .width(Length::FillPortion(3))
                    .style(style::Container::LightGray)
                    .padding(PADDING),
                )
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .into()
}

pub(super) fn build_timeout_ribbon<'a>(
    snapshot: &GameSnapshot,
    tm: &Arc<Mutex<TournamentManager>>,
) -> Row<'a, Message> {
    let in_timeout = !matches!(snapshot.timeout, TimeoutSnapshot::None);

    let mut black = make_button(if in_timeout {
        "SWITCH TO\nBLACK"
    } else {
        "BLACK\nTIMEOUT"
    })
    .style(style::Button::Black);

    let mut white = make_button(if in_timeout {
        "SWITCH TO\nWHITE"
    } else {
        "WHITE\nTIMEOUT"
    })
    .style(style::Button::White);

    let mut referee = make_button(if in_timeout {
        "SWITCH TO\nREF"
    } else {
        "REF\nTIMEOUT"
    })
    .style(style::Button::Yellow);

    let mut penalty = make_button(if in_timeout {
        "SWITCH TO\nPEN SHOT"
    } else {
        "PENALTY\nSHOT"
    })
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

    row()
        .spacing(SPACING)
        .push(black)
        .push(referee)
        .push(penalty)
        .push(white)
}

fn make_game_time_button<'a>(snapshot: &GameSnapshot) -> Button<'a, Message> {
    let (period_text, period_color) = period_text_and_color(snapshot.current_period);
    let mut content = row()
        .spacing(SPACING)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .push(
            text(period_text)
                .color(period_color)
                .width(Length::Fill)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Right),
        )
        .push(
            text(secs_to_time_string(snapshot.secs_in_period).trim())
                .color(period_color)
                .size(LARGE_TEXT)
                .width(Length::Fill)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Left),
        );

    if let Some((timeout_text, color)) = match snapshot.timeout {
        TimeoutSnapshot::White(_) => Some(("WHITE TIMEOUT", WHITE)),
        TimeoutSnapshot::Black(_) => Some(("BLACK TIMEOUT", BLACK)),
        TimeoutSnapshot::Ref(_) => Some(("REF TIMEOUT", YELLOW)),
        TimeoutSnapshot::PenaltyShot(_) => Some(("PENALTY SHOT", RED)),
        TimeoutSnapshot::None => None,
    } {
        content = content
            .push(
                text(timeout_text)
                    .color(color)
                    .width(Length::Fill)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Right),
            )
            .push(
                text(timeout_time_string(snapshot))
                    .width(Length::Fill)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Left)
                    .color(color)
                    .size(LARGE_TEXT),
            );
    };

    button(content)
        .width(Length::Fill)
        .height(Length::Units(MIN_BUTTON_SIZE))
        .style(style::Button::Gray)
}

fn make_time_editor<'a, T: Into<String>>(
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'a, Message> {
    container(
        column()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(text(title).size(MEDIUM_TEXT))
            .push(
                row()
                    .spacing(SPACING)
                    .align_items(Alignment::Center)
                    .push(
                        column()
                            .spacing(SPACING)
                            .push(
                                make_small_button("+", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: true,
                                        secs: 60,
                                        timeout,
                                    }),
                            )
                            .push(
                                make_small_button("-", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: false,
                                        secs: 60,
                                        timeout,
                                    }),
                            ),
                    )
                    .push(
                        text(time_string(time))
                            .size(LARGE_TEXT)
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Units(200)),
                    )
                    .push(
                        column()
                            .spacing(SPACING)
                            .push(
                                make_small_button("+", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: true,
                                        secs: 1,
                                        timeout,
                                    }),
                            )
                            .push(
                                make_small_button("-", LARGE_TEXT)
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

fn make_button<'a, Message: Clone, T: Into<String>>(label: T) -> Button<'a, Message> {
    button(
        text(label)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
    )
    .padding(PADDING)
    .height(Length::Units(MIN_BUTTON_SIZE))
    .width(Length::Fill)
}

fn make_small_button<'a, Message: Clone, T: Into<String>>(
    label: T,
    size: u16,
) -> Button<'a, Message> {
    button(
        text(label)
            .size(size)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
    )
    .width(Length::Units(MIN_BUTTON_SIZE))
    .height(Length::Units(MIN_BUTTON_SIZE))
}

fn make_value_button<'a, Message: 'a + Clone, T: Into<String>, U: Into<String>>(
    first_label: T,
    second_label: U,
    message: Option<Message>,
) -> Button<'a, Message> {
    let mut button = button(
        row()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(
                text(first_label)
                    .size(SMALL_TEXT)
                    .vertical_alignment(Vertical::Center),
            )
            .push(horizontal_space(Length::Fill))
            .push(
                text(second_label)
                    .size(MEDIUM_TEXT)
                    .vertical_alignment(Vertical::Center),
            ),
    )
    .padding(PADDING)
    .height(Length::Units(MIN_BUTTON_SIZE))
    .width(Length::Fill)
    .style(style::Button::LightGray);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}
