use super::{
    style::{
        self, BLACK, BORDER_RADIUS, GREEN, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, ORANGE,
        PADDING, RED, SMALL_TEXT, SPACING, WHITE, YELLOW,
    },
    *,
};
use collect_array::CollectArrayResult;
use iced::{
    alignment::{Horizontal, Vertical},
    pure::{
        button, column, container, horizontal_space, row, text, vertical_space,
        widget::{Button, Container, Row, Text},
        Element,
    },
    Alignment, Length,
};
use matrix_drawing::{secs_to_long_time_string, secs_to_time_string};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{
        Color as GameColor, GamePeriod, GameSnapshot, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
    },
};

pub(super) fn build_main_view<'a>(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
) -> Element<'a, Message> {
    let time_button = make_game_time_button(snapshot, true, true).on_press(Message::EditTime);

    let mut center_col = column()
        .spacing(SPACING)
        .width(Length::Fill)
        .push(time_button);

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
            text(config_string(snapshot, config, using_uwhscores, games))
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

    let mut black_score_btn = button(
        column()
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .push("BLACK")
            .push(text(snapshot.b_score.to_string()).size(LARGE_TEXT)),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .style(style::Button::Black);

    let mut black_new_score_btn = make_button("SCORE\nBLACK").style(style::Button::Black);

    let mut white_score_btn = button(
        column()
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .push("WHITE")
            .push(text(snapshot.w_score.to_string()).size(LARGE_TEXT)),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .style(style::Button::White);

    let mut white_new_score_btn = make_button("SCORE\nWHITE").style(style::Button::White);

    if snapshot.current_period != GamePeriod::BetweenGames {
        black_score_btn = black_score_btn.on_press(Message::EditScores);
        black_new_score_btn = black_new_score_btn
            .on_press(Message::KeypadPage(KeypadPage::AddScore(GameColor::Black)));
        white_score_btn = white_score_btn.on_press(Message::EditScores);
        white_new_score_btn = white_new_score_btn
            .on_press(Message::KeypadPage(KeypadPage::AddScore(GameColor::White)));
    }

    let black_col = column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .push(black_score_btn)
        .push(black_new_score_btn)
        .push(make_penalty_button(&snapshot.b_penalties).style(style::Button::Black));

    let white_col = column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .push(white_score_btn)
        .push(white_new_score_btn)
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
        .push(make_game_time_button(snapshot, false, false).on_press(Message::NoAction))
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
    scores: BlackWhiteBundle<u8>,
    is_confirmation: bool,
) -> Element<'a, Message> {
    let cancel_btn_msg = if is_confirmation {
        None
    } else {
        Some(Message::ScoreEditComplete { canceled: true })
    };

    let black_edit = container(
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
                    .push(text(scores.black.to_string()).size(LARGE_TEXT)),
            ),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(style::Container::Black);

    let white_edit = container(
        row()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(
                column()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .push("WHITE")
                    .push(text(scores.white.to_string()).size(LARGE_TEXT)),
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
    .style(style::Container::White);

    let mut main_col = column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill));

    if is_confirmation {
        main_col = main_col
            .push(
                text("Please enter the final score")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill),
            )
            .push(vertical_space(Length::Fill));
    }

    main_col
        .push(
            row()
                .spacing(SPACING)
                .push(horizontal_space(Length::Fill))
                .push(black_edit)
                .push(horizontal_space(Length::Fill))
                .push(white_edit)
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(make_message_button("CANCEL", cancel_btn_msg).style(style::Button::Red))
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
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
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
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
    const PENALTY_LIST_LEN: usize = 3;

    let title = text(format!("{} PENALTIES", color.to_string().to_uppercase()))
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

    let num_pens = penalties.len();

    let buttons: CollectArrayResult<_, PENALTY_LIST_LEN> = penalties
        .into_iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(PENALTY_LIST_LEN)
        .map(|pen| {
            if let Some((i, (pen_text, format, kind))) = pen {
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
            }
        })
        .collect();

    let cont_style = match color {
        GameColor::Black => style::Container::Black,
        GameColor::White => style::Container::White,
    };

    let scroll_option = match color {
        GameColor::Black => ScrollOption::Black,
        GameColor::White => ScrollOption::White,
    };

    make_scroll_list(
        buttons.unwrap(),
        num_pens + 1,
        index,
        title,
        scroll_option,
        cont_style,
    )
}

fn make_scroll_list<'a, const LIST_LEN: usize>(
    buttons: [Button<'a, Message>; LIST_LEN],
    num_items: usize,
    index: usize,
    title: Text,
    scroll_option: ScrollOption,
    cont_style: impl iced::pure::widget::container::StyleSheet + 'a,
) -> Container<'a, Message> {
    let mut main_col = column().spacing(SPACING).width(Length::Fill).push(title);

    for button in buttons {
        main_col = main_col.push(button);
    }

    let top_len;
    let bottom_len;
    let can_scroll_up;
    let can_scroll_down;

    if num_items <= LIST_LEN {
        top_len = 0;
        bottom_len = 0;
        can_scroll_up = false;
        can_scroll_down = false;
    } else {
        top_len = index as u16;
        bottom_len = (num_items - LIST_LEN - index) as u16;
        can_scroll_up = index > 0;
        can_scroll_down = index + LIST_LEN < num_items;
    }

    let top_len = match top_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let bottom_len = match bottom_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let mut up_btn = make_small_button("\u{25b2}", MEDIUM_TEXT).style(style::Button::Blue);

    let mut down_btn = make_small_button("\u{25be}", MEDIUM_TEXT).style(style::Button::Blue);

    if can_scroll_up {
        up_btn = up_btn.on_press(Message::Scroll {
            which: scroll_option,
            up: true,
        });
    }

    if can_scroll_down {
        down_btn = down_btn.on_press(Message::Scroll {
            which: scroll_option,
            up: false,
        });
    }

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
                            .height(Length::FillPortion(LIST_LEN as u16))
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
            .push(main_col)
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

pub(super) fn build_list_selector_page<'a>(
    snapshot: &GameSnapshot,
    param: ListableParameter,
    index: usize,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
) -> Element<'a, Message> {
    const LIST_LEN: usize = 4;
    const TEAM_NAME_LEN_LIMIT: usize = 15;

    let title = match param {
        ListableParameter::Tournament => "SELECT TOURNAMENT",
        ListableParameter::Pool => "SELECT COURT",
        ListableParameter::Game => "SELECT GAME",
    };

    let title = text(title)
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

    // (btn_text, msg_val)

    macro_rules! make_buttons {
        ($iter:ident, $transform:ident) => {
            $iter
                .skip(index)
                .map($transform)
                .map(Some)
                .chain([None].into_iter().cycle())
                .take(LIST_LEN)
                .map(|pen| {
                    if let Some((btn_text, msg_val)) = pen {
                        let text = text(btn_text)
                            .vertical_alignment(Vertical::Center)
                            .horizontal_alignment(Horizontal::Left)
                            .width(Length::Fill);

                        button(text)
                            .padding(PADDING)
                            .height(Length::Units(MIN_BUTTON_SIZE))
                            .width(Length::Fill)
                            .style(style::Button::Gray)
                            .on_press(Message::ParameterSelected(param, msg_val))
                    } else {
                        button(horizontal_space(Length::Shrink))
                            .height(Length::Units(MIN_BUTTON_SIZE))
                            .width(Length::Fill)
                            .style(style::Button::Gray)
                    }
                })
                .collect()
        };
    }

    let (num_items, buttons): (usize, CollectArrayResult<_, LIST_LEN>) = match param {
        ListableParameter::Tournament => {
            let list = tournaments.as_ref().unwrap();
            let num_items = list.len();
            let iter = list.values().rev();
            let transform = |t: &TournamentInfo| (t.name.clone(), t.tid as usize);
            (num_items, make_buttons!(iter, transform))
        }
        ListableParameter::Pool => {
            let list = tournaments
                .as_ref()
                .unwrap()
                .get(&settings.current_tid.unwrap())
                .unwrap()
                .pools
                .as_ref()
                .unwrap();
            let num_items = list.len();
            let iter = list.iter().enumerate();
            let transform = |(i, p): (usize, &String)| (p.clone(), i);
            (num_items, make_buttons!(iter, transform))
        }
        ListableParameter::Game => {
            let list = settings.games.as_ref().unwrap();
            let pool = settings.current_pool.clone().unwrap();
            let num_items = list.values().filter(|g| g.pool == pool).count();
            let iter = list.values().filter(|g| g.pool == pool);
            let transform = |g| (game_string_long(g, TEAM_NAME_LEN_LIMIT), g.gid as usize);
            (num_items, make_buttons!(iter, transform))
        }
    };

    let scroll_list = make_scroll_list(
        buttons.unwrap(),
        num_items,
        index,
        title,
        ScrollOption::GameParameter,
        style::Container::LightGray,
    )
    .width(Length::FillPortion(4));

    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .width(Length::Fill)
                .push(scroll_list)
                .push(
                    column()
                        .width(Length::Fill)
                        .push(vertical_space(Length::Fill))
                        .push(
                            make_button("CANCEL")
                                .style(style::Button::Red)
                                .width(Length::Fill)
                                .height(Length::Units(MIN_BUTTON_SIZE))
                                .on_press(Message::ParameterEditComplete { canceled: true }),
                        ),
                ),
        )
        .into()
}

pub(super) fn build_keypad_page<'a>(
    snapshot: &GameSnapshot,
    page: KeypadPage,
    player_num: u16,
) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
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
                                        make_small_button("\u{232b}", LARGE_TEXT)
                                            .width(Length::Units(2 * MIN_BUTTON_SIZE + SPACING))
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
                    KeypadPage::AddScore(color) => make_add_score_page(color),
                    KeypadPage::Penalty(origin, color, kind) => {
                        make_edit_penalty_page(origin, color, kind)
                    }
                    KeypadPage::GameNumber => make_game_number_edit_page(),
                    KeypadPage::TeamTimeouts(dur) => make_team_timeout_edit_page(dur),
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
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
) -> Element<'a, Message> {
    const NO_SELECTION_TXT: &str = "None Selected";
    const LOADING_TXT: &str = "Loading...";

    let EditableSettings {
        config,
        game_number,
        white_on_right,
        using_uwhscores,
        current_tid,
        current_pool,
        games,
    } = settings;

    let using_uwhscores = *using_uwhscores;

    let white_inner = container("WHITE")
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container::White);
    let black_inner = container("BLACK")
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container::Black);
    let white_spacer = container("")
        .width(Length::Units(BORDER_RADIUS.ceil() as u16))
        .height(Length::Fill)
        .style(style::Container::WhiteSharpCorner);
    let black_spacer = container("")
        .width(Length::Units(BORDER_RADIUS.ceil() as u16))
        .height(Length::Fill)
        .style(style::Container::BlackSharpCorner);

    // `white_on_right` is based on the view from the front of the panels, so for the ref's point
    // of view we need to reverse the direction
    let sides = if !white_on_right {
        // White to Ref's right
        let white_outer = container(
            row()
                .push(white_spacer)
                .push(white_inner)
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::White);
        let black_outer = container(
            row()
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16)))
                .push(black_inner)
                .push(black_spacer),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::Black);
        row().push(black_outer).push(white_outer)
    } else {
        // White to Ref's left
        let white_outer = container(
            row()
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16)))
                .push(white_inner)
                .push(white_spacer),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::White);
        let black_outer = container(
            row()
                .push(black_spacer)
                .push(black_inner)
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::Black);
        row().push(white_outer).push(black_outer)
    };

    let sides_btn = button(sides.width(Length::Fill).height(Length::Fill))
        .height(Length::Units(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .padding(0)
        .style(style::Button::Gray)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    let rows: [Element<Message>; 4] = if using_uwhscores {
        let tournament_label = if let Some(ref tournaments) = tournaments {
            if let Some(tid) = current_tid {
                match tournaments.get(tid) {
                    Some(t) => t.name.clone(),
                    None => NO_SELECTION_TXT.to_string(),
                }
            } else {
                NO_SELECTION_TXT.to_string()
            }
        } else {
            LOADING_TXT.to_string()
        };

        let tournament_btn_msg = if tournaments.is_some() {
            Some(Message::SelectParameter(ListableParameter::Tournament))
        } else {
            None
        };

        let pool_label = if let Some(tournament) = tournaments
            .as_ref()
            .and_then(|tournaments| tournaments.get(&(*current_tid)?))
        {
            if tournament.pools.is_some() {
                if let Some(ref pool) = current_pool {
                    pool.clone()
                } else {
                    NO_SELECTION_TXT.to_string()
                }
            } else {
                LOADING_TXT.to_string()
            }
        } else {
            String::new()
        };

        let pool_btn_msg = tournaments
            .as_ref()
            .and_then(|tourns| tourns.get(&(*current_tid)?)?.pools.as_ref())
            .map(|_| Message::SelectParameter(ListableParameter::Pool));

        [
            make_value_button("TOURNAMENT:", tournament_label, true, tournament_btn_msg).into(),
            make_value_button("COURT:", pool_label, true, pool_btn_msg).into(),
            vertical_space(Length::Units(MIN_BUTTON_SIZE)).into(),
            row()
                .spacing(SPACING)
                .push(horizontal_space(Length::Fill))
                .push(horizontal_space(Length::Fill))
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
                )
                .into(),
        ]
    } else {
        [
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "HALF LENGTH:",
                    time_string(config.half_play_duration),
                    true,
                    Some(Message::EditParameter(LengthParameter::Half)),
                ))
                .push(make_value_button(
                    "OVERTIME\nALLOWED:",
                    bool_string(config.overtime_allowed),
                    true,
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::OvertimeAllowed,
                    )),
                ))
                .push(make_value_button(
                    "SUDDEN DEATH\nALLOWED:",
                    bool_string(config.sudden_death_allowed),
                    true,
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::SuddenDeathAllowed,
                    )),
                ))
                .into(),
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "HALF TIME\nLENGTH:",
                    time_string(config.half_time_duration),
                    true,
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
                ))
                .push(make_value_button(
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    true,
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::PreOvertime))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "PRE SD\nBREAK LENGTH:",
                    time_string(config.pre_sudden_death_duration),
                    true,
                    if config.sudden_death_allowed {
                        Some(Message::EditParameter(LengthParameter::PreSuddenDeath))
                    } else {
                        None
                    },
                ))
                .into(),
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "NOMINAL BRK\nBTWN GAMES:",
                    time_string(config.nominal_break),
                    true,
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    true,
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "NUM TEAM T/Os\nALLWD PER HALF:",
                    config.team_timeouts_per_half.to_string(),
                    true,
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        config.team_timeout_duration,
                    ))),
                ))
                .into(),
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "MINIMUM BRK\nBTWN GAMES:",
                    time_string(config.minimum_break),
                    true,
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    true,
                    if config.overtime_allowed {
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
                )
                .into(),
        ]
    };

    let game_btn_msg = if using_uwhscores {
        if current_tid.is_some() && current_pool.is_some() {
            Some(Message::SelectParameter(ListableParameter::Game))
        } else {
            None
        }
    } else {
        Some(Message::KeypadPage(KeypadPage::GameNumber))
    };

    let mut game_large_text = true;
    let game_label = if using_uwhscores {
        if let (Some(_), Some(cur_pool)) = (current_tid, current_pool) {
            if let Some(ref games) = games {
                match games.get(game_number) {
                    Some(game) => {
                        if game.pool == *cur_pool {
                            game_string_short(game)
                        } else {
                            game_large_text = false;
                            NO_SELECTION_TXT.to_string()
                        }
                    }
                    None => {
                        game_large_text = false;
                        NO_SELECTION_TXT.to_string()
                    }
                }
            } else {
                LOADING_TXT.to_string()
            }
        } else {
            String::new()
        }
    } else {
        game_number.to_string()
    };

    let mut col = column()
        .spacing(SPACING)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "USING UWHSCORES:",
                    bool_string(using_uwhscores),
                    true,
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::UsingUwhScores,
                    )),
                ))
                .push(sides_btn)
                .push(make_value_button(
                    "GAME:",
                    game_label,
                    game_large_text,
                    game_btn_msg,
                )),
        );

    for row in rows {
        col = col.push(row);
    }

    col.into()
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
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
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
        ConfirmationKind::UwhScoresIncomplete => "When UWHScores is enabled, all fields must be filled out."
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
        ConfirmationKind::UwhScoresIncomplete => vec![
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
        ],
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
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
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

pub(super) fn build_score_confirmation_page<'a>(
    snapshot: &GameSnapshot,
    scores: BlackWhiteBundle<u8>,
) -> Element<'a, Message> {
    let header = text(format!(
        "Is this score correct?\n\nBlack: {}        White: {}\n",
        scores.black, scores.white
    ))
    .horizontal_alignment(Horizontal::Center);

    let options = row()
        .spacing(SPACING)
        .width(Length::Fill)
        .push(
            make_button("YES")
                .style(style::Button::Green)
                .on_press(Message::ScoreConfirmation { correct: true }),
        )
        .push(
            make_button("NO")
                .style(style::Button::Red)
                .on_press(Message::ScoreConfirmation { correct: false }),
        );

    column()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
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
                            .push(header)
                            .push(options),
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
    let tm = tm.lock().unwrap();

    let black = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "BLACK\nTIMEOUT",
            tm.can_start_b_timeout()
                .ok()
                .map(|_| Message::BlackTimeout(false)),
        )
        .style(style::Button::Black),
        TimeoutSnapshot::Black(_) => make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
            .style(style::Button::Yellow),
        TimeoutSnapshot::White(_) | TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button(
                "SWITCH TO\nBLACK",
                tm.can_switch_to_b_timeout()
                    .ok()
                    .map(|_| Message::BlackTimeout(true)),
            )
            .style(style::Button::Black)
        }
    };

    let white = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "WHITE\nTIMEOUT",
            tm.can_start_w_timeout()
                .ok()
                .map(|_| Message::WhiteTimeout(false)),
        )
        .style(style::Button::White),
        TimeoutSnapshot::White(_) => make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
            .style(style::Button::Yellow),
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button(
                "SWITCH TO\nWHITE",
                tm.can_switch_to_w_timeout()
                    .ok()
                    .map(|_| Message::WhiteTimeout(true)),
            )
            .style(style::Button::White)
        }
    };

    let referee = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "REF\nTIMEOUT",
            tm.can_start_ref_timeout()
                .ok()
                .map(|_| Message::RefTimeout(false)),
        )
        .style(style::Button::Yellow),
        TimeoutSnapshot::Ref(_) => make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
            .style(style::Button::Yellow),
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::White(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button(
                "SWITCH TO\nREF",
                tm.can_switch_to_ref_timeout()
                    .ok()
                    .map(|_| Message::RefTimeout(true)),
            )
            .style(style::Button::Yellow)
        }
    };

    let penalty = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "PENALTY\nSHOT",
            tm.can_start_penalty_shot()
                .ok()
                .map(|_| Message::PenaltyShot(false)),
        )
        .style(style::Button::Red),
        TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
                .style(style::Button::Yellow)
        }
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::White(_) | TimeoutSnapshot::Ref(_) => {
            make_message_button(
                "SWITCH TO\nPEN SHOT",
                tm.can_switch_to_penalty_shot()
                    .ok()
                    .map(|_| Message::PenaltyShot(true)),
            )
            .style(style::Button::Red)
        }
    };

    drop(tm);

    row()
        .spacing(SPACING)
        .push(black)
        .push(referee)
        .push(penalty)
        .push(white)
}

fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    tall: bool,
    allow_red: bool,
) -> Button<'a, Message> {
    let make_red = if !allow_red {
        false
    } else {
        match snapshot.timeout {
            TimeoutSnapshot::Black(time) | TimeoutSnapshot::White(time) => {
                (time <= 10 && (time % 2 == 0) && (time != 0)) || time == 15
            }
            TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => false,
            TimeoutSnapshot::None => {
                let is_warn_period = match snapshot.current_period {
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

                snapshot.current_period != GamePeriod::SuddenDeath
                    && ((snapshot.secs_in_period <= 10
                        && (snapshot.secs_in_period % 2 == 0)
                        && (snapshot.secs_in_period != 0))
                        || (is_warn_period && snapshot.secs_in_period == 30))
            }
        }
    };

    let (mut period_text, period_color) = {
        let (text, color) = match snapshot.current_period {
            GamePeriod::BetweenGames => ("NEXT GAME", YELLOW),
            GamePeriod::FirstHalf => ("FIRST HALF", GREEN),
            GamePeriod::HalfTime => ("HALF TIME", YELLOW),
            GamePeriod::SecondHalf => ("SECOND HALF", GREEN),
            GamePeriod::PreOvertime => ("PRE OVERTIME BREAK", YELLOW),
            GamePeriod::OvertimeFirstHalf => ("OVERTIME FIRST HALF", GREEN),
            GamePeriod::OvertimeHalfTime => ("OVERTIME HALF TIME", YELLOW),
            GamePeriod::OvertimeSecondHalf => ("OVERTIME SECOND HALF", GREEN),
            GamePeriod::PreSuddenDeath => ("PRE SUDDEN DEATH BREAK", YELLOW),
            GamePeriod::SuddenDeath => ("SUDDEN DEATH", GREEN),
        };

        if make_red {
            (text, BLACK)
        } else {
            (text, color)
        }
    };

    if tall && (snapshot.timeout != TimeoutSnapshot::None) {
        match snapshot.current_period {
            GamePeriod::PreOvertime => period_text = "PRE OT BREAK",
            GamePeriod::OvertimeFirstHalf => period_text = "OT FIRST HALF",
            GamePeriod::OvertimeHalfTime => period_text = "OT HALF TIME",
            GamePeriod::OvertimeSecondHalf => period_text = "OT 2ND HALF",
            GamePeriod::PreSuddenDeath => period_text = "PRE SD BREAK",
            _ => {}
        };
    }

    macro_rules! make_time_view {
        ($base:ident, $per_text:ident, $time_text:ident) => {
            $base
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .push($per_text)
                .push($time_text)
        };
    }

    let make_time_view_row = |period_text, time_text, color| {
        let per = text(period_text)
            .color(color)
            .width(Length::Fill)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Right);
        let time = text(time_text)
            .color(color)
            .size(LARGE_TEXT)
            .width(Length::Fill)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Left);
        let r = row().spacing(SPACING);
        make_time_view!(r, per, time)
    };

    let make_time_view_col = |period_text, time_text, color| {
        let per = text(period_text).color(color);
        let time = text(time_text).color(color).size(LARGE_TEXT);
        let c = column();
        make_time_view!(c, per, time)
    };

    let mut content = row()
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center);

    let timeout_info = match snapshot.timeout {
        TimeoutSnapshot::White(_) => Some((
            if tall { "WHT TIMEOUT" } else { "WHITE TIMEOUT" },
            if make_red { BLACK } else { WHITE },
        )),
        TimeoutSnapshot::Black(_) => {
            Some((if tall { "BLK TIMEOUT" } else { "BLACK TIMEOUT" }, BLACK))
        }
        TimeoutSnapshot::Ref(_) => Some(("REF TIMEOUT", YELLOW)),
        TimeoutSnapshot::PenaltyShot(_) => Some(("PENALTY SHOT", RED)),
        TimeoutSnapshot::None => None,
    };

    let time_text = secs_to_long_time_string(snapshot.secs_in_period);
    let time_text = time_text.trim();

    if tall {
        content = content.push(make_time_view_col(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_col(
                timeout_text,
                &timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    } else {
        content = content.push(make_time_view_row(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_row(
                timeout_text,
                &timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    }

    let button_height = if tall {
        Length::Shrink
    } else {
        Length::Units(MIN_BUTTON_SIZE)
    };

    let button_style = if make_red {
        style::Button::Red
    } else {
        style::Button::Gray
    };

    button(content)
        .width(Length::Fill)
        .height(button_height)
        .padding(PADDING)
        .style(button_style)
}

fn make_time_editor<'a, T: Into<String>>(
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'a, Message> {
    let wide = time > Duration::from_secs(MAX_STRINGABLE_SECS as u64);

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
                            .width(Length::Units(if wide { 300 } else { 200 })),
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
    secs_to_long_time_string(time.as_secs()).trim().to_string()
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

fn game_string_short(game: &GameInfo) -> String {
    format!("{}{}", game.game_type, game.gid)
}

fn game_string_long(game: &GameInfo, len_limit: usize) -> String {
    const ELIPSIS: [char; 3] = ['.', '.', '.'];

    macro_rules! limit {
        ($orig:ident) => {
            if $orig.len() > len_limit {
                Cow::Owned($orig.chars().take(len_limit - 1).chain(ELIPSIS).collect())
            } else {
                Cow::Borrowed($orig)
            }
        };
    }

    let black = &game.black;
    let black = limit!(black);
    let white = &game.white;
    let white = limit!(white);

    format!("{}{} - {} vs {}", game.game_type, game.gid, black, white)
}

fn config_string(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
) -> String {
    const TEAM_NAME_LEN_LIMIT: usize = 12;

    let mut result = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if using_uwhscores {
            if let Some(games) = games {
                prev_game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_long(game, TEAM_NAME_LEN_LIMIT),
                    None if snapshot.game_number == 0 => "None".to_string(),
                    None => format!("Error ({})", snapshot.game_number),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game_string_long(game, TEAM_NAME_LEN_LIMIT),
                    None => format!("Error ({})", snapshot.next_game_number),
                };
            } else {
                prev_game = if snapshot.game_number == 0 {
                    "None".to_string()
                } else {
                    format!("Error ({})", snapshot.game_number)
                };
                next_game = format!("Error ({})", snapshot.next_game_number);
            }
        } else {
            prev_game = if snapshot.game_number == 0 {
                "None".to_string()
            } else {
                snapshot.game_number.to_string()
            };
            next_game = snapshot.next_game_number.to_string();
        }

        format!("Last Game: {}\nNext Game: {}\n", prev_game, next_game)
    } else {
        let game;
        if using_uwhscores {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_long(game, TEAM_NAME_LEN_LIMIT),
                    None => format!("Error ({})", snapshot.game_number),
                };
            } else {
                game = format!("Error ({})", snapshot.game_number);
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        format!("Game: {}\n\n", game)
    };
    result += &format!(
        "Half Length: {}\n\
         Half Time Length: {}\n\
         Overtime Allowed: {}\n",
        time_string(config.half_play_duration),
        time_string(config.half_time_duration),
        bool_string(config.overtime_allowed),
    );
    result += &if config.overtime_allowed {
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

fn make_message_button<'a, Message: Clone, T: Into<String>>(
    label: T,
    message: Option<Message>,
) -> Button<'a, Message> {
    if let Some(msg) = message {
        make_button(label).on_press(msg)
    } else {
        make_button(label)
    }
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
    second_is_large: bool,
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
                    .size(if second_is_large {
                        MEDIUM_TEXT
                    } else {
                        SMALL_TEXT
                    })
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