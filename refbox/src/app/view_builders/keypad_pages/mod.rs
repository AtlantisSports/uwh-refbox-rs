use super::{
    super::Config,
    style::{
        Button, ButtonStyle, ContainerStyle, Element, SvgStyle, LARGE_TEXT, LINE_HEIGHT,
        MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, SPACING,
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

mod foul_add;
use foul_add::*;

mod warning_add;
use warning_add::*;

pub(in super::super) fn build_keypad_page<'a>(
    snapshot: &GameSnapshot,
    page: KeypadPage,
    player_num: u16,
    config: &Config,
    clock_running: bool,
) -> Element<'a, Message> {
    let enabled = match page {
        KeypadPage::WarningAdd { team_warning, .. } => !team_warning,
        KeypadPage::FoulAdd { color, .. } => color.is_some(),
        _ => true,
    };

    let setup_keypad_button =
        |button: Button<'a, Message>, message: Message| -> Button<'a, Message> {
            let button = if enabled {
                button.on_press(message)
            } else {
                button
            };
            button.style(ButtonStyle::Blue)
        };

    column![
        make_game_time_button(snapshot, false, false, config.mode, clock_running),
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
                        setup_keypad_button(
                            make_small_button("7", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Seven,)
                        ),
                        setup_keypad_button(
                            make_small_button("8", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Eight,)
                        ),
                        setup_keypad_button(
                            make_small_button("9", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Nine,)
                        ),
                    ]
                    .spacing(SPACING),
                    row![
                        setup_keypad_button(
                            make_small_button("4", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Four,)
                        ),
                        setup_keypad_button(
                            make_small_button("5", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Five,)
                        ),
                        setup_keypad_button(
                            make_small_button("6", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Six,)
                        ),
                    ]
                    .spacing(SPACING),
                    row![
                        setup_keypad_button(
                            make_small_button("1", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::One,)
                        ),
                        setup_keypad_button(
                            make_small_button("2", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Two,)
                        ),
                        setup_keypad_button(
                            make_small_button("3", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Three,)
                        ),
                    ]
                    .spacing(SPACING),
                    row![
                        setup_keypad_button(
                            make_small_button("0", MEDIUM_TEXT),
                            Message::KeypadButtonPress(KeypadButton::Zero,)
                        ),
                        setup_keypad_button(
                            button(
                                container(
                                    Svg::new(svg::Handle::from_memory(
                                        &include_bytes!("../../../../resources/backspace.svg")[..],
                                    ))
                                    .style(if enabled {
                                        SvgStyle::White
                                    } else {
                                        SvgStyle::Disabled
                                    })
                                    .height(Length::Fixed(MEDIUM_TEXT * 1.2)),
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .style(ContainerStyle::Transparent)
                                .center_x()
                                .center_y(),
                            )
                            .width(Length::Fixed(2.0 * MIN_BUTTON_SIZE + SPACING))
                            .height(Length::Fixed(MIN_BUTTON_SIZE)),
                            Message::KeypadButtonPress(KeypadButton::Delete,)
                        ),
                    ]
                    .spacing(SPACING),
                ]
                .spacing(SPACING),
            )
            .style(if enabled {
                ContainerStyle::LightGray
            } else {
                ContainerStyle::Disabled
            })
            .padding(PADDING),
            match page {
                KeypadPage::AddScore(color) => make_score_add_page(color),
                KeypadPage::Penalty(origin, color, kind, foul, expanded) => {
                    make_penalty_edit_page(origin, color, kind, config, foul, expanded)
                }
                KeypadPage::GameNumber => make_game_number_edit_page(),
                KeypadPage::TeamTimeouts(dur) => make_team_timeout_edit_page(dur),
                KeypadPage::FoulAdd {
                    origin,
                    color,
                    infraction,
                    expanded,
                    ret_to_overview,
                } => make_foul_add_page(origin, color, infraction, expanded, ret_to_overview),
                KeypadPage::WarningAdd {
                    origin,
                    color,
                    infraction,
                    expanded,
                    team_warning,
                    ret_to_overview,
                } => make_warning_add_page(
                    origin,
                    color,
                    infraction,
                    expanded,
                    team_warning,
                    ret_to_overview
                ),
            }
        ]
        .spacing(SPACING)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
