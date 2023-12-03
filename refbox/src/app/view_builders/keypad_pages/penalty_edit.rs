use super::{style::Element, *};
use iced::{
    widget::{column, row, vertical_space},
    Length,
};

use uwh_common::game_snapshot::Color as GameColor;

pub(super) fn make_penalty_edit_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    kind: PenaltyKind,
    mode: Mode,
) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (ButtonStyle::BlackSelected, ButtonStyle::White),
        GameColor::White => (ButtonStyle::Black, ButtonStyle::WhiteSelected),
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

    let (green_style, yellow_style, orange_style, td_style) = if kind == green {
        (
            ButtonStyle::GreenSelected,
            ButtonStyle::Yellow,
            ButtonStyle::Orange,
            ButtonStyle::Red,
        )
    } else if kind == yellow {
        (
            ButtonStyle::Green,
            ButtonStyle::YellowSelected,
            ButtonStyle::Orange,
            ButtonStyle::Red,
        )
    } else if kind == orange {
        (
            ButtonStyle::Green,
            ButtonStyle::Yellow,
            ButtonStyle::OrangeSelected,
            ButtonStyle::Red,
        )
    } else if kind == PenaltyKind::TotalDismissal {
        (
            ButtonStyle::Green,
            ButtonStyle::Yellow,
            ButtonStyle::Orange,
            ButtonStyle::RedSelected,
        )
    } else {
        (
            ButtonStyle::Green,
            ButtonStyle::Yellow,
            ButtonStyle::Orange,
            ButtonStyle::Red,
        )
    };

    let mut exit_row = row![make_button("CANCEL")
        .style(ButtonStyle::Red)
        .width(Length::Fill)
        .on_press(Message::PenaltyEditComplete {
            canceled: true,
            deleted: false,
        }),]
    .spacing(SPACING);

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button("DELETE")
                .style(ButtonStyle::Orange)
                .width(Length::Fill)
                .on_press(Message::PenaltyEditComplete {
                    canceled: false,
                    deleted: true,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button("DONE")
            .style(ButtonStyle::Green)
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

    column![
        vertical_space(Length::Fill),
        row![
            make_button("BLACK")
                .style(black_style)
                .on_press(Message::ChangeColor(GameColor::Black)),
            make_button("WHITE")
                .style(white_style)
                .on_press(Message::ChangeColor(GameColor::White)),
        ]
        .spacing(SPACING),
        vertical_space(Length::Fill),
        row![
            make_button(green_label)
                .style(green_style)
                .on_press(Message::ChangeKind(green)),
            make_button(yellow_label)
                .style(yellow_style)
                .on_press(Message::ChangeKind(yellow)),
            make_button(orange_label)
                .style(orange_style)
                .on_press(Message::ChangeKind(orange)),
            make_button("TD")
                .style(td_style)
                .on_press(Message::ChangeKind(PenaltyKind::TotalDismissal)),
        ]
        .spacing(SPACING),
        vertical_space(Length::Fill),
        exit_row,
    ]
    .spacing(SPACING)
    .into()
}
