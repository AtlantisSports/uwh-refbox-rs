use crate::config::Mode;

use super::{
    style::{
        ButtonStyle, ContainerStyle, Element, SvgStyle, LARGE_TEXT, LINE_HEIGHT, MEDIUM_TEXT,
        MIN_BUTTON_SIZE, PADDING, SPACING,
    },
    *,
};

use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        button, column, container, row,
        svg::{self, Svg},
        text,
    },
    Alignment, Length,
};

use uwh_common::game_snapshot::GameSnapshot;

mod score_add;
use score_add::*;

mod penalty_edit;
use penalty_edit::*;

mod game_number_edit;
use game_number_edit::*;

mod team_timeout_edit;
use team_timeout_edit::*;

pub(in super::super) fn build_keypad_page<'a>(
    snapshot: &GameSnapshot,
    page: KeypadPage,
    player_num: u16,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        row![
            container(
                column![
                    row![
                        text(page.text())
                            .line_height(LINE_HEIGHT)
                            .horizontal_alignment(Horizontal::Left)
                            .vertical_alignment(Vertical::Center),
                        text(player_num.to_string())
                            .size(LARGE_TEXT)
                            .line_height(LINE_HEIGHT)
                            .width(Length::Fill)
                            .horizontal_alignment(Horizontal::Right)
                            .vertical_alignment(Vertical::Center),
                    ]
                    .align_items(Alignment::Center)
                    .height(Length::Fill)
                    .width(Length::Fixed(3.0 * MIN_BUTTON_SIZE + 2.0 * SPACING)),
                    row![
                        make_small_button("7", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Seven,)),
                        make_small_button("8", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Eight,)),
                        make_small_button("9", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Nine,)),
                    ]
                    .spacing(SPACING),
                    row![
                        make_small_button("4", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Four,)),
                        make_small_button("5", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Five,)),
                        make_small_button("6", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Six,)),
                    ]
                    .spacing(SPACING),
                    row![
                        make_small_button("1", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::One,)),
                        make_small_button("2", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Two,)),
                        make_small_button("3", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Three,)),
                    ]
                    .spacing(SPACING),
                    row![
                        make_small_button("0", MEDIUM_TEXT)
                            .style(ButtonStyle::Blue)
                            .on_press(Message::KeypadButtonPress(KeypadButton::Zero,)),
                        button(
                            container(
                                Svg::new(svg::Handle::from_memory(
                                    &include_bytes!("../../../../resources/backspace.svg")[..],
                                ))
                                .style(SvgStyle::White)
                                .height(Length::Fixed(MEDIUM_TEXT * 1.2)),
                            )
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .style(ContainerStyle::Transparent)
                            .center_x()
                            .center_y(),
                        )
                        .width(Length::Fixed(2.0 * MIN_BUTTON_SIZE + SPACING))
                        .height(Length::Fixed(MIN_BUTTON_SIZE))
                        .style(ButtonStyle::Blue)
                        .on_press(Message::KeypadButtonPress(KeypadButton::Delete,)),
                    ]
                    .spacing(SPACING),
                ]
                .spacing(SPACING),
            )
            .style(ContainerStyle::LightGray)
            .padding(PADDING),
            match page {
                KeypadPage::AddScore(color) => make_score_add_page(color),
                KeypadPage::Penalty(origin, color, kind) => {
                    make_penalty_edit_page(origin, color, kind, mode)
                }
                KeypadPage::GameNumber => make_game_number_edit_page(),
                KeypadPage::TeamTimeouts(dur) => make_team_timeout_edit_page(dur),
            }
        ]
        .spacing(SPACING)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
