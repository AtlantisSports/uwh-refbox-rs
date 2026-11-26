use super::*;
use iced::{
    Alignment, Element, Length,
    alignment::Horizontal,
    widget::{column, container, horizontal_space, row, text, vertical_space},
};
use uwh_common::{
    color::Color as GameColor,
    uwhportal::schedule::{GameNumber, Schedule, TeamList},
};

/// The names shown under BLACK and WHITE, and the sign that SWAP COLOURS may be
/// offered: `Some` only when `can_swap` (the game comes from a site and an event
/// is linked) and the game has a real team on both sides. `None` leaves the page
/// exactly as it was before the button existed.
pub(in super::super) fn score_edit_team_names(
    can_swap: bool,
    schedule: Option<&Schedule>,
    game_number: &GameNumber,
    teams: Option<&TeamList>,
) -> Option<BlackWhiteBundle<String>> {
    if !can_swap {
        return None;
    }
    super::super::colour_swap::swappable_teams(schedule, game_number)?;
    let game = schedule?.games.get(game_number)?;
    Some(BlackWhiteBundle {
        black: side_name(
            &game.dark,
            game.dark_partner.as_ref(),
            teams,
            super::game_info_table::TEAM_NAME_LEN_LIMIT,
        ),
        white: side_name(
            &game.light,
            game.light_partner.as_ref(),
            teams,
            super::game_info_table::TEAM_NAME_LEN_LIMIT,
        ),
    })
}

pub(in super::super) fn build_score_edit_view<'a>(
    data: ViewData<'_, '_>,
    scores: BlackWhiteBundle<u8>,
    is_confirmation: bool,
    confirmation_time: Option<u32>,
    old_scores: BlackWhiteBundle<u8>,
    team_names: Option<BlackWhiteBundle<String>>,
    colours_swapped: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock,
        portal_indicator,
        ..
    } = data;

    let cancel_btn_msg = if is_confirmation {
        None
    } else {
        Some(Message::ScoreEditComplete { canceled: true })
    };

    // While a swap is pending, each name shows under the colour it will wear.
    let shown_names = team_names.as_ref().map(|names| {
        if colours_swapped {
            (names.white.clone(), names.black.clone())
        } else {
            (names.black.clone(), names.white.clone())
        }
    });
    let mut black_caption = column![text(fl!("dark-team-name-caps"))]
        .spacing(SPACING)
        .align_x(Alignment::Center);
    let mut white_caption = column![text(fl!("light-team-name-caps"))]
        .spacing(SPACING)
        .align_x(Alignment::Center);
    if let Some((black_name, white_name)) = shown_names {
        black_caption = black_caption.push(text(black_name).align_x(Horizontal::Center));
        white_caption = white_caption.push(text(white_name).align_x(Horizontal::Center));
    }

    let black_edit = container(
        row![
            column![
                make_small_button("+", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::Black,
                        increase: true,
                    }),
                make_small_button("-", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::Black,
                        increase: false,
                    }),
            ]
            .spacing(SPACING),
            column![
                black_caption,
                text(scores.black.to_string()).size(LARGE_TEXT)
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(black_container);

    let white_edit = container(
        row![
            column![
                white_caption,
                text(scores.white.to_string()).size(LARGE_TEXT)
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_x(Alignment::Center),
            column![
                make_small_button("+", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::White,
                        increase: true,
                    }),
                make_small_button("-", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::White,
                        increase: false,
                    }),
            ]
            .spacing(SPACING),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(white_container);

    let mut main_col = column![
        make_game_time_button(
            snapshot,
            false,
            is_confirmation,
            mode,
            clock,
            portal_indicator,
            None
        ),
        vertical_space()
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    if is_confirmation {
        main_col = main_col.push(
            text(fl!("final-score"))
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        );
        if let Some(time) = confirmation_time {
            let time = time_string(Duration::from_secs(time as u64));
            main_col = main_col.push(
                text(fl!("confirmation-count-down", countdown = time))
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
            );
        }
        main_col = main_col.push(vertical_space());
    }

    // Confirmation mode keeps "Done" and stays always-clickable (the operator
    // must be able to commit the final score). Edit mode uses "Apply" and grays
    // out until the scores actually differ from what was shown on open.
    let confirm_btn = if is_confirmation {
        make_chrome_button(fl!("done"))
            .style(green_button)
            .on_press(Message::ScoreEditComplete { canceled: false })
    } else {
        make_chrome_button(fl!("apply"))
            .style(green_button)
            .on_press_maybe(
                score_edit_has_changes(scores, old_scores, colours_swapped)
                    .then_some(Message::ScoreEditComplete { canceled: false }),
            )
    };

    // Confirmation mode keeps the (disabled) "Cancel" label — the operator is
    // committing a final score, not navigating back. Edit mode swaps to "Back"
    // when the scores are unchanged.
    let cancel_label = if is_confirmation {
        fl!("cancel")
    } else {
        cancel_or_back_label(score_edit_has_changes(scores, old_scores, colours_swapped))
    };

    // SWAP COLOURS sits centred between the two footer buttons in both forms;
    // without swappable teams the gap stays empty, as before.
    let middle: Element<'a, Message> = if team_names.is_some() {
        make_chrome_button(fl!("swap-colours"))
            .style(blue_button)
            .on_press(Message::SwapColours)
            .into()
    } else {
        horizontal_space().into()
    };

    main_col
        .push(
            row![
                horizontal_space(),
                black_edit,
                horizontal_space(),
                white_edit,
                horizontal_space()
            ]
            .spacing(SPACING),
        )
        .push(vertical_space())
        .push(
            row![
                make_chrome_button(cancel_label)
                    .on_press_maybe(cancel_btn_msg)
                    .style(red_button),
                middle,
                confirm_btn,
            ]
            .spacing(SPACING),
        )
        .into()
}

/// Returns true when the edited scores differ from the scores shown when the
/// Score EDIT screen was opened (the tournament-manager scores, which are not
/// changed until Apply), or when a colour swap is pending. Only consulted in
/// edit mode — never in confirmation, where the operator must always be able to
/// commit the final score.
fn score_edit_has_changes(
    scores: BlackWhiteBundle<u8>,
    old: BlackWhiteBundle<u8>,
    colours_swapped: bool,
) -> bool {
    scores != old || colours_swapped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_no_change_is_false() {
        let s = BlackWhiteBundle { black: 3, white: 5 };
        assert!(!score_edit_has_changes(s, s, false));
    }

    #[test]
    fn score_change_is_true() {
        assert!(score_edit_has_changes(
            BlackWhiteBundle { black: 4, white: 5 },
            BlackWhiteBundle { black: 3, white: 5 },
            false
        ));
    }

    #[test]
    fn score_round_trip_is_false() {
        // +1 then -1 returns to the original bundle.
        let original = BlackWhiteBundle { black: 2, white: 2 };
        let after = BlackWhiteBundle {
            black: original.black + 1 - 1,
            white: original.white,
        };
        assert!(!score_edit_has_changes(after, original, false));
    }

    #[test]
    fn a_swap_alone_counts_as_a_change() {
        let s = BlackWhiteBundle { black: 3, white: 5 };
        let s2 = BlackWhiteBundle { black: 4, white: 5 };
        assert!(score_edit_has_changes(s, s, true));
        assert!(!score_edit_has_changes(s, s, false));
        assert!(score_edit_has_changes(s2, s, false));
    }
}
