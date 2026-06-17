use super::*;
use iced::{
    Length, Theme,
    widget::{
        button::{Status, Style},
        column, row, vertical_space,
    },
};
use uwh_common::color::Color as GameColor;

type StyleFn = fn(&Theme, Status) -> Style;

pub(super) fn make_penalty_edit_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    kind: PenaltyKind,
    mode: Mode,
    track_fouls_and_warnings: bool,
    infraction: Infraction,
    player_num: u32,
) -> Element<'a, Message> {
    let (black_style, white_style): (StyleFn, StyleFn) = match color {
        GameColor::Black => (black_selected_button, white_button),
        GameColor::White => (black_button, white_selected_button),
    };

    let (green, yellow, orange) = match mode {
        Mode::Hockey6V6 => (
            PenaltyKind::OneMinute,
            PenaltyKind::TwoMinute,
            PenaltyKind::FiveMinute,
        ),

        Mode::Hockey3V3 => (
            PenaltyKind::ThirtySecond,
            PenaltyKind::OneMinute,
            PenaltyKind::TwoMinute,
        ),

        Mode::Rugby => (
            PenaltyKind::TwoMinute,
            PenaltyKind::FourMinute,
            PenaltyKind::FiveMinute,
        ),

        Mode::BeepTest => unreachable!("BeepTest mode does not edit penalties"),
    };

    let (green_style, yellow_style, orange_style, td_style): (StyleFn, StyleFn, StyleFn, StyleFn) =
        if kind == green {
            (
                green_selected_button,
                yellow_button,
                orange_button,
                red_button,
            )
        } else if kind == yellow {
            (
                green_button,
                yellow_selected_button,
                orange_button,
                red_button,
            )
        } else if kind == orange {
            (
                green_button,
                yellow_button,
                orange_selected_button,
                red_button,
            )
        } else if kind == PenaltyKind::TotalDismissal {
            (
                green_button,
                yellow_button,
                orange_button,
                red_selected_button,
            )
        } else {
            (green_button, yellow_button, orange_button, red_button)
        };

    let mut exit_row = row![
        make_smaller_button(fl!("cancel"))
            .style(red_button)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: true,
                deleted: false,
            })
    ]
    .spacing(SPACING);

    if origin.is_some() {
        exit_row = exit_row.push(
            make_smaller_button(fl!("delete"))
                .style(orange_button)
                .width(Length::Fill)
                .on_press(Message::PenaltyEditComplete {
                    canceled: false,
                    deleted: true,
                }),
        );
    }

    exit_row = exit_row.push(
        make_smaller_button(fl!("done"))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(
                penalty_edit_can_commit(infraction, track_fouls_and_warnings, player_num)
                    .then_some(Message::PenaltyEditComplete {
                        canceled: false,
                        deleted: false,
                    }),
            ),
    );

    let green_label = fl!("penalty-kind", kind = green.fluent());
    let yellow_label = fl!("penalty-kind", kind = yellow.fluent());
    let orange_label = fl!("penalty-kind", kind = orange.fluent());

    let mut content = column![
        row![
            make_smaller_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            make_smaller_button(fl!("light-team-name-caps"))
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING)
    ];

    content = content.push(vertical_space());

    if track_fouls_and_warnings {
        content = content.push(make_penalty_dropdown(infraction, false));
    }

    content = content.push(vertical_space());

    content = content.push(
        row![
            make_smaller_button(green_label)
                .style(green_style)
                .on_press(Message::ChangeKind(green)),
            make_smaller_button(yellow_label)
                .style(yellow_style)
                .on_press(Message::ChangeKind(yellow)),
            make_smaller_button(orange_label)
                .style(orange_style)
                .on_press(Message::ChangeKind(orange)),
            make_smaller_button(fl!("total-dismissal"))
                .style(td_style)
                .on_press(Message::ChangeKind(PenaltyKind::TotalDismissal)),
        ]
        .spacing(SPACING),
    );

    content = content.push(vertical_space());

    content = content.push(exit_row);
    content.into()
}

/// Returns true when the penalty entry can be saved: a player number is always
/// required (penalties are always individual). The infraction is required only
/// when "track fouls & warnings" is on — that is exactly when the infraction
/// picker is shown on this screen.
fn penalty_edit_can_commit(
    infraction: Infraction,
    track_fouls_and_warnings: bool,
    player_num: u32,
) -> bool {
    player_num > 0 && (!track_fouls_and_warnings || !matches!(infraction, Infraction::Unknown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn penalty_needs_number() {
        // Tracking off: only a number is required.
        assert!(!penalty_edit_can_commit(Infraction::Unknown, false, 0));
        assert!(penalty_edit_can_commit(Infraction::Unknown, false, 5));
    }

    #[test]
    fn penalty_needs_infraction_when_tracking_on() {
        assert!(!penalty_edit_can_commit(Infraction::Unknown, true, 5));
        assert!(penalty_edit_can_commit(
            Infraction::StickInfringement,
            true,
            5
        ));
    }

    #[test]
    fn penalty_tracking_on_still_needs_number() {
        assert!(!penalty_edit_can_commit(
            Infraction::StickInfringement,
            true,
            0
        ));
    }
}
