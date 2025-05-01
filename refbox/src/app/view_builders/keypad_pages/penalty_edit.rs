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
            .on_press(Message::PenaltyEditComplete {
                canceled: false,
                deleted: false,
            }),
    );

    let labels: Vec<&str> = [green, yellow, orange]
        .iter()
        .map(|kind| match kind {
            PenaltyKind::ThirtySecond => "30s",
            PenaltyKind::OneMinute => "1m",
            PenaltyKind::TwoMinute => "2m",
            PenaltyKind::FourMinute => "4m",
            PenaltyKind::FiveMinute => "5m",
            PenaltyKind::TotalDismissal => "TD",
        })
        .collect();

    let green_label = labels[0];
    let yellow_label = labels[1];
    let orange_label = labels[2];

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
