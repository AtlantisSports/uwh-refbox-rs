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
                penalty_edit_can_commit(color, infraction, track_fouls_and_warnings, player_num)
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

/// Returns true when the penalty entry can be saved. A player number is the
/// only requirement — penalties are always individual, and a team is always
/// selected on this page (`KeypadPage::Penalty` carries a plain `GameColor`,
/// so one of Black/White is highlighted from the moment the page opens).
///
/// The infraction is deliberately optional, even with "track fouls & warnings"
/// on and the picker visible. A penalty's essential content is the player and
/// the duration — poolside the exclusion clock may need to start before the
/// reason is settled — whereas a foul or warning is nothing but its infraction,
/// so those pages do still require one (`foul_add_can_commit`,
/// `warning_add_can_commit`). An infraction-less penalty is not a new state
/// either: it is what this page has always produced with tracking off, and
/// `Infraction::Unknown` is the enum's default, handled throughout.
///
/// The team, infraction, and tracking flag stay in the signature — the whole of
/// the page's state that could plausibly gate saving — even though the rule
/// ignores them. That is what lets `penalty_gate_depends_only_on_the_player_number`
/// pin their irrelevance, so reintroducing any of them fails a test rather than
/// silently re-blocking the operator.
fn penalty_edit_can_commit(
    _color: GameColor,
    _infraction: Infraction,
    _track_fouls_and_warnings: bool,
    player_num: u32,
) -> bool {
    player_num > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_iterator::all;

    #[test]
    fn penalty_gate_depends_only_on_the_player_number() {
        // Across every combination of team, infraction (including one already
        // picked, and every real infraction — not just Unknown), and the
        // fouls-and-warnings toggle: a player number is sufficient on its own,
        // and its absence is the only thing that blocks saving.
        for color in [GameColor::Black, GameColor::White] {
            for infraction in all::<Infraction>() {
                for tracking in [false, true] {
                    assert!(
                        penalty_edit_can_commit(color, infraction, tracking, 5),
                        "a player number must be enough: \
                         {color:?}, {infraction:?}, tracking={tracking}"
                    );
                    assert!(
                        !penalty_edit_can_commit(color, infraction, tracking, 0),
                        "no player number must block saving: \
                         {color:?}, {infraction:?}, tracking={tracking}"
                    );
                }
            }
        }
    }
}
