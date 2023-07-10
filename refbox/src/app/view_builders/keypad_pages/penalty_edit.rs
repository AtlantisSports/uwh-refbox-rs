use crate::config::Mode;

use super::{
    style::{self, SPACING},
    *,
};

use iced::{
    pure::{column, row, vertical_space, Element},
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
        GameColor::Black => (style::Button::BlackSelected, style::Button::White),
        GameColor::White => (style::Button::Black, style::Button::WhiteSelected),
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
            style::Button::GreenSelected,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::Red,
        )
    } else if kind == yellow {
        (
            style::Button::Green,
            style::Button::YellowSelected,
            style::Button::Orange,
            style::Button::Red,
        )
    } else if kind == orange {
        (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::OrangeSelected,
            style::Button::Red,
        )
    } else if kind == PenaltyKind::TotalDismissal {
        (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::RedSelected,
        )
    } else {
        (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::Red,
        )
    };

    let mut exit_row = row().spacing(SPACING).push(
        make_button("CANCEL")
            .style(style::Button::Red)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: true,
                deleted: false,
            }),
    );

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button("DELETE")
                .style(style::Button::Orange)
                .width(Length::Fill)
                .on_press(Message::PenaltyEditComplete {
                    canceled: false,
                    deleted: true,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button("DONE")
            .style(style::Button::Green)
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

    column()
        .spacing(SPACING)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("BLACK")
                        .style(black_style)
                        .on_press(Message::ChangeColor(GameColor::Black)),
                )
                .push(
                    make_button("WHITE")
                        .style(white_style)
                        .on_press(Message::ChangeColor(GameColor::White)),
                ),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button(green_label)
                        .style(green_style)
                        .on_press(Message::ChangeKind(green)),
                )
                .push(
                    make_button(yellow_label)
                        .style(yellow_style)
                        .on_press(Message::ChangeKind(yellow)),
                )
                .push(
                    make_button(orange_label)
                        .style(orange_style)
                        .on_press(Message::ChangeKind(orange)),
                )
                .push(
                    make_button("TD")
                        .style(td_style)
                        .on_press(Message::ChangeKind(PenaltyKind::TotalDismissal)),
                ),
        )
        .push(vertical_space(Length::Fill))
        .push(exit_row)
        .into()
}
