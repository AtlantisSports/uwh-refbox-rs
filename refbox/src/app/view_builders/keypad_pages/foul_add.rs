use super::*;
use iced::{
    Length, Theme,
    widget::{
        Space,
        button::{Status, Style},
        column, row, vertical_space,
    },
};

type StyleFn = fn(&Theme, Status) -> Style;

use uwh_common::color::Color as GameColor;

pub(super) fn make_foul_add_page<'a>(
    origin: Option<(Option<GameColor>, usize)>,
    color: Option<GameColor>,
    foul: Infraction,
    ret_to_overview: bool,
) -> Element<'a, Message> {
    let (black_style, white_style, equal_style): (StyleFn, StyleFn, StyleFn) = match color {
        Some(GameColor::Black) => (black_selected_button, white_button, blue_button),
        Some(GameColor::White) => (black_button, white_selected_button, blue_button),
        None => (black_button, white_button, blue_selected_button),
    };

    let mut exit_row = row![
        make_button(fl!("cancel"))
            .style(red_button)
            .width(Length::Fill)
            .on_press(Message::FoulEditComplete {
                canceled: true,
                deleted: false,
                ret_to_overview
            })
    ]
    .spacing(SPACING);

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button(fl!("delete"))
                .style(orange_button)
                .width(Length::Fill)
                .on_press(Message::FoulEditComplete {
                    canceled: false,
                    deleted: true,
                    ret_to_overview,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button(fl!("done"))
            .style(green_button)
            .width(Length::Fill)
            .on_press(Message::FoulEditComplete {
                canceled: false,
                deleted: false,
                ret_to_overview,
            }),
    );
    column![
        row![
            make_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            button(centered_text("=").size(LARGE_TEXT))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .width(Length::Fill)
                .on_press(Message::ChangeColor(None))
                .style(equal_style),
            make_button(fl!("light-team-name-caps"))
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        Space::with_height(SPACING),
        make_penalty_dropdown(foul, true),
        vertical_space(),
        exit_row,
    ]
    .into()
}
