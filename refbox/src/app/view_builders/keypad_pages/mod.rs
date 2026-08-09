use super::{
    theme::{MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, SPACING},
    *,
};
use iced::{
    Length,
    alignment::{Horizontal, Vertical},
    widget::{
        Space, button,
        button::Button,
        column, container, row,
        svg::{self, Svg},
        text,
    },
};
use uwh_common::bundles::BlackWhiteBundle;
use uwh_common::color::Color as GameColor;

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

mod player_grid;
use player_grid::*;

mod portal_login;
use portal_login::*;

pub(in super::super) fn build_keypad_page<'a>(
    data: ViewData<'_, '_>,
    page: KeypadPage,
    player_num: u32,
    track_fouls_and_warnings: bool,
    original_game_number: Option<String>,
    rosters: &BlackWhiteBundle<Vec<u8>>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    // Single source of truth for every question the panel asks about this page.
    let role = panel_role(&page);
    let enabled = role.is_enabled();

    // The team-timeout settings page does not use the shared number pad; it
    // renders as a full-width panel below the game-time bar.
    if let KeypadPage::TeamTimeouts(dur, per_half) = &page {
        let (dur, per_half) = (*dur, *per_half);
        return column![
            make_game_time_button(
                snapshot,
                false,
                false,
                mode,
                clock_running,
                portal_indicator,
                None
            ),
            make_team_timeout_edit_page(dur, per_half, player_num),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into();
    }

    // An empty slice means "no usable roster": the number pad is shown, exactly
    // as it is today. That covers the portal being off, an unassigned team slot,
    // a fetch that never succeeded, a roster with no cap numbers — and every
    // page that names no team, since those roles carry no colour.
    const NO_ROSTER: &[u8] = &[];
    let panel_numbers: &[u8] = match role {
        PanelRole::Player(color) => &rosters[color],
        PanelRole::TeamEntry | PanelRole::NotPlayer => NO_ROSTER,
    };

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row![
            container(if show_grid(panel_numbers, mode, player_num) {
                make_player_grid(panel_numbers, mode, player_num, enabled)
            } else {
                make_number_pad(&page, player_num, enabled, role.is_player_page())
            })
            .style(if enabled {
                light_gray_container
            } else {
                disabled_container
            })
            .padding(PADDING)
            // Fixed to the pad's own width (3 columns of MIN_BUTTON_SIZE) so the
            // panel is the same box whichever child it holds, and DONE/CANCEL
            // cannot shift sideways when a page shows a grid for one team and
            // the pad for the other.
            .width(Length::Fixed(
                3.0 * MIN_BUTTON_SIZE + 2.0 * SPACING + 2.0 * PADDING
            ))
            // The player pages get a full-height panel so grid and pad can both
            // bottom-justify against it (and, in Rugby, so the grid's rows can
            // share it). The pad-only pages stay content-sized, exactly as they
            // are today.
            .height(if role.is_player_page() {
                Length::Fill
            } else {
                Length::Shrink
            }),
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
                        player_num,
                    )
                }
                KeypadPage::GameNumber =>
                    make_game_number_edit_page(player_num, original_game_number),
                KeypadPage::TeamTimeouts(_, _) => {
                    unreachable!("TeamTimeouts is handled by the early return above")
                }
                KeypadPage::FoulAdd {
                    origin,
                    color,
                    infraction,
                    ret_to_overview,
                } => make_foul_add_page(origin, color, infraction, ret_to_overview, player_num),
                KeypadPage::WarningAdd {
                    origin,
                    color,
                    infraction,
                    team_warning,
                    ret_to_overview,
                } => make_warning_add_page(
                    origin,
                    color,
                    infraction,
                    team_warning,
                    ret_to_overview,
                    player_num,
                ),
                KeypadPage::PortalLogin(id, requested) => {
                    make_portal_login_page(id, requested, mode)
                }
            }
        ]
        .spacing(SPACING)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// What the left-hand panel is being asked for on a page.
///
/// Everything the panel needs to decide — whether it is enabled, whose roster
/// it shows, and whether it is full height with its buttons at the bottom —
/// derives from this one value. Keeping them separate let them drift: three
/// places each re-deriving "is this a player page" from the page variants is
/// three places to update when a variant is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelRole {
    /// Asks which player, with a team selected. Shows that team's grid, or the
    /// pad when there is no usable roster for them.
    Player(GameColor),
    /// Asks which player, but this entry belongs to no player — an "equal"
    /// foul, or a team warning. The panel is greyed out.
    TeamEntry,
    /// Not about a player at all: game number, timeouts per half, portal login.
    /// Free digit entry, always enabled, panel sized to its contents.
    NotPlayer,
}

impl PanelRole {
    /// A page that asks which player, whether or not one is currently named.
    /// These panels are full height with their buttons pinned to the bottom, so
    /// the grid and the pad cannot disagree about where the buttons sit.
    fn is_player_page(self) -> bool {
        matches!(self, Self::Player(_) | Self::TeamEntry)
    }

    /// The panel greys out only where the entry has no player to name.
    fn is_enabled(self) -> bool {
        !matches!(self, Self::TeamEntry)
    }
}

fn panel_role(page: &KeypadPage) -> PanelRole {
    match page {
        KeypadPage::AddScore(color) | KeypadPage::Penalty(_, color, _, _) => {
            PanelRole::Player(*color)
        }
        KeypadPage::FoulAdd { color, .. } => match color {
            Some(color) => PanelRole::Player(*color),
            None => PanelRole::TeamEntry,
        },
        KeypadPage::WarningAdd {
            color,
            team_warning,
            ..
        } => {
            if *team_warning {
                PanelRole::TeamEntry
            } else {
                PanelRole::Player(*color)
            }
        }
        KeypadPage::GameNumber | KeypadPage::TeamTimeouts(_, _) | KeypadPage::PortalLogin(_, _) => {
            PanelRole::NotPlayer
        }
    }
}

fn make_number_pad<'a>(
    page: &KeypadPage,
    player_num: u32,
    enabled: bool,
    bottom_justified: bool,
) -> Element<'a, Message> {
    let setup_keypad_button =
        |button: Button<'a, Message>, message: Message| -> Button<'a, Message> {
            let button = if enabled {
                button.on_press(message)
            } else {
                button
            };
            button.style(blue_button)
        };

    let text_displayed = match *page {
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

    let text_size = MEDIUM_TEXT;

    let label = row![
        text(page.text()).align_x(Horizontal::Left),
        Space::with_width(Length::Fill),
        text(text_displayed).size(text_size),
    ]
    .width(Length::Fixed(3.0 * MIN_BUTTON_SIZE + 2.0 * SPACING));

    let digits = column![
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
    .spacing(SPACING);

    if bottom_justified {
        // On the four player pages the panel swaps between this pad and the
        // number grid as the operator toggles teams, so both are pinned to the
        // bottom of a full-height panel. The button block then lands in exactly
        // the same place either way and nothing moves under their finger.
        column![label, Space::with_height(Length::Fill), digits]
            .spacing(SPACING)
            .height(Length::Fill)
            .into()
    } else {
        // The pages that only ever show the pad — game number, timeouts per
        // half, portal login — keep their content-sized panel unchanged.
        column![label, digits].spacing(SPACING).into()
    }
}
