use super::*;
use iced::{
    Alignment, Element, Length,
    alignment::Horizontal,
    widget::{column, container, horizontal_space, row, text, vertical_space},
};
use uwh_common::color::Color as GameColor;

pub(in super::super) fn build_score_edit_view<'a>(
    data: ViewData<'_, '_>,
    scores: BlackWhiteBundle<u8>,
    is_confirmation: bool,
    confirmation_time: Option<u32>,
    old_scores: BlackWhiteBundle<u8>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let cancel_btn_msg = if is_confirmation {
        None
    } else {
        Some(Message::ScoreEditComplete { canceled: true })
    };

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
                text(fl!("dark-team-name-caps")),
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
                text(fl!("light-team-name-caps")),
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
            clock_running,
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
        make_button(fl!("done"))
            .style(green_button)
            .on_press(Message::ScoreEditComplete { canceled: false })
    } else {
        make_button(fl!("apply"))
            .style(green_button)
            .on_press_maybe(
                score_edit_has_changes(scores, old_scores)
                    .then_some(Message::ScoreEditComplete { canceled: false }),
            )
    };

    // Confirmation mode keeps the (disabled) "Cancel" label — the operator is
    // committing a final score, not navigating back. Edit mode swaps to "Back"
    // when the scores are unchanged.
    let cancel_label = if is_confirmation {
        fl!("cancel")
    } else {
        cancel_or_back_label(score_edit_has_changes(scores, old_scores))
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
                make_button(cancel_label)
                    .on_press_maybe(cancel_btn_msg)
                    .style(red_button),
                horizontal_space(),
                confirm_btn,
            ]
            .spacing(SPACING),
        )
        .into()
}

/// Returns true when the edited scores differ from the scores shown when the
/// Score EDIT screen was opened (the tournament-manager scores, which are not
/// changed until Apply). Only consulted in edit mode — never in confirmation,
/// where the operator must always be able to commit the final score.
fn score_edit_has_changes(scores: BlackWhiteBundle<u8>, old: BlackWhiteBundle<u8>) -> bool {
    scores != old
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_no_change_is_false() {
        let s = BlackWhiteBundle { black: 3, white: 5 };
        assert!(!score_edit_has_changes(s, s));
    }

    #[test]
    fn score_change_is_true() {
        assert!(score_edit_has_changes(
            BlackWhiteBundle { black: 4, white: 5 },
            BlackWhiteBundle { black: 3, white: 5 }
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
        assert!(!score_edit_has_changes(after, original));
    }
}
