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
) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (style::Button::BlackSelected, style::Button::White),
        GameColor::White => (style::Button::Black, style::Button::WhiteSelected),
    };

    let (one_min_style, two_min_style, five_min_style, td_style) = match kind {
        PenaltyKind::OneMinute => (
            style::Button::GreenSelected,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::Red,
        ),
        PenaltyKind::TwoMinute => (
            style::Button::Green,
            style::Button::YellowSelected,
            style::Button::Orange,
            style::Button::Red,
        ),
        PenaltyKind::FiveMinute => (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::OrangeSelected,
            style::Button::Red,
        ),
        PenaltyKind::TotalDismissal => (
            style::Button::Green,
            style::Button::Yellow,
            style::Button::Orange,
            style::Button::RedSelected,
        ),
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
                    make_button("1m")
                        .style(one_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::OneMinute)),
                )
                .push(
                    make_button("2m")
                        .style(two_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::TwoMinute)),
                )
                .push(
                    make_button("5m")
                        .style(five_min_style)
                        .on_press(Message::ChangeKind(PenaltyKind::FiveMinute)),
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
