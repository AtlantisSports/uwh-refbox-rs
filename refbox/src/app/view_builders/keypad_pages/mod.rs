use super::{
    theme::{LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, SPACING},
    *,
};
use iced::{
    Alignment, Length,
    alignment::{Horizontal, Vertical},
    widget::{
        button,
        button::Button,
        column, container, row,
        svg::{self, Svg},
        text,
    },
};

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

mod portal_login;
use portal_login::*;

pub(in super::super) fn build_keypad_page<'a>(
    data: ViewData<'_, '_>,
    page: KeypadPage,
    player_num: u32,
    track_fouls_and_warnings: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

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
            button.style(blue_button)
        };

    let text_displayed = match page {
        KeypadPage::WarningAdd { team_warning, .. } => {
            if team_warning {
                "TEAM".to_string()
            } else {
                player_num.to_string()
            }
        }
        KeypadPage::AddScore(_) => {
            if player_num == 0 {
                "TEAM".to_string()
            } else {
                player_num.to_string()
            }
        }
        KeypadPage::FoulAdd { color, .. } => {
            if color.is_none() {
                "TEAM".to_string()
            } else {
                player_num.to_string()
            }
        }
        KeypadPage::GameNumber
        | KeypadPage::Penalty(_, _, _, _)
        | KeypadPage::TeamTimeouts(_, _)
        | KeypadPage::PortalLogin(_, _) => player_num.to_string(),
    };

    let text_size = if text_displayed == "TEAM" || matches!(page, KeypadPage::PortalLogin(_, _)) {
        MEDIUM_TEXT
    } else {
        LARGE_TEXT
    };

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            container(
                column![
                    row![
                        text(page.text())
                            .align_x(Horizontal::Left)
                            .align_y(Vertical::Center),
                        text(text_displayed)
                            .size(text_size)
                            .width(Length::Fill)
                            .align_x(Horizontal::Right)
                            .align_y(Vertical::Center),
                    ]
                    .align_y(Alignment::Center)
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
                            Message::KeypadButtonPress(KeypadButton::Zero),
                        ),
                        setup_keypad_button(
                            button(
                                container(
                                    Svg::new(svg::Handle::from_memory(
                                        &include_bytes!("../../../../resources/backspace.svg")[..],
                                    ))
                                    .style(if enabled { white_svg } else { disabled_svg })
                                    .height(Length::Fixed(MEDIUM_TEXT * 1.2)),
                                )
                                .style(transparent_container)
                                .center(Length::Fill),
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
                light_gray_container
            } else {
                disabled_container
            })
            .padding(PADDING),
            match page {
                KeypadPage::AddScore(color) => make_score_add_page(color),
                KeypadPage::Penalty(origin, color, kind, foul) => {
                    make_penalty_edit_page(
                        origin,
                        color,
                        kind,
                        mode,
                        track_fouls_and_warnings,
                        foul,
                    )
                }
                KeypadPage::GameNumber => make_game_number_edit_page(),
                KeypadPage::TeamTimeouts(dur, per_half) =>
                    make_team_timeout_edit_page(dur, per_half),
                KeypadPage::FoulAdd {
                    origin,
                    color,
                    infraction,
                    ret_to_overview,
                } => make_foul_add_page(origin, color, infraction, ret_to_overview),
                KeypadPage::WarningAdd {
                    origin,
                    color,
                    infraction,
                    team_warning,
                    ret_to_overview,
                } =>
                    make_warning_add_page(origin, color, infraction, team_warning, ret_to_overview),
                KeypadPage::PortalLogin(id, requested) => make_portal_login_page(id, requested),
            }
        ]
        .spacing(SPACING)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
