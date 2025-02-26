use super::{style::Element, *};
use iced::{
    Length,
    widget::{column, row, vertical_space},
};

use uwh_common::game_snapshot::Color as GameColor;

pub(super) fn make_score_add_page<'a>(color: GameColor) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (ButtonStyle::BlackSelected, ButtonStyle::White),
        GameColor::White => (ButtonStyle::Black, ButtonStyle::WhiteSelected),
    };

    column![
        vertical_space(Length::Fill),
        row![
            make_button("BLACK")
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            make_button("WHITE")
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        vertical_space(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::AddScoreComplete { canceled: true }),
            make_button("DONE")
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::AddScoreComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
