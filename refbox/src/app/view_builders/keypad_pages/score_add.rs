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

pub(super) fn make_score_add_page<'a>(color: GameColor) -> Element<'a, Message> {
    let (black_style, white_style): (StyleFn, StyleFn) = match color {
        GameColor::Black => (black_selected_button, white_button),
        GameColor::White => (black_button, white_selected_button),
    };

    column![
        vertical_space(),
        row![
            make_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            make_button(fl!("light-team-name-caps"))
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::AddScoreComplete { canceled: true }),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::AddScoreComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
