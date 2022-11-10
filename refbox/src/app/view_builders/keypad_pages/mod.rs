use super::{
    style::{self, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, SPACING},
    *,
};

use iced::{
    alignment::{Horizontal, Vertical},
    pure::{column, container, row, text, Element},
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
                    KeypadPage::AddScore(color) => make_score_add_page(color),
                    KeypadPage::Penalty(origin, color, kind) => {
                        make_penalty_edit_page(origin, color, kind)
                    }
                    KeypadPage::GameNumber => make_game_number_edit_page(),
                    KeypadPage::TeamTimeouts(dur) => make_team_timeout_edit_page(dur),
                }),
        )
        .into()
}
