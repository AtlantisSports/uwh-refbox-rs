use super::{
    style::{self, SPACING},
    *,
};

use iced::{
    pure::{column, row, vertical_space, Element},
    Length,
};

use uwh_common::game_snapshot::Color as GameColor;

pub(super) fn make_score_add_page<'a>(color: GameColor) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (style::Button::BlackSelected, style::Button::White),
        GameColor::White => (style::Button::Black, style::Button::WhiteSelected),
    };

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
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::AddScoreComplete { canceled: true }),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::AddScoreComplete { canceled: false }),
                ),
        )
        .into()
}
