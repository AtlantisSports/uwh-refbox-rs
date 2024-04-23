use super::{style::Element, *};
use iced::{
    widget::{column, row, vertical_space},
    Length,
};

use uwh_common::game_snapshot::Color as GameColor;

pub(super) fn make_foul_add_page<'a>(
    origin: Option<(Option<GameColor>, usize)>,
    color: Option<GameColor>,
    foul: Infraction,
    expanded: bool,
    ret_to_overview: bool,
) -> Element<'a, Message> {
    let (black_style, white_style, equal_style) = match color {
        Some(GameColor::Black) => (
            ButtonStyle::BlackSelected,
            ButtonStyle::White,
            ButtonStyle::Blue,
        ),
        Some(GameColor::White) => (
            ButtonStyle::Black,
            ButtonStyle::WhiteSelected,
            ButtonStyle::Blue,
        ),
        None => (
            ButtonStyle::Black,
            ButtonStyle::White,
            ButtonStyle::BlueSelected,
        ),
    };

    let mut exit_row = row![make_button("CANCEL")
        .style(ButtonStyle::Red)
        .width(Length::Fill)
        .on_press(Message::FoulEditComplete {
            canceled: true,
            deleted: false,
            ret_to_overview
        }),]
    .spacing(SPACING);

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button("DELETE")
                .style(ButtonStyle::Orange)
                .width(Length::Fill)
                .on_press(Message::FoulEditComplete {
                    canceled: false,
                    deleted: true,
                    ret_to_overview,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button("DONE")
            .style(ButtonStyle::Green)
            .width(Length::Fill)
            .on_press(Message::FoulEditComplete {
                canceled: false,
                deleted: false,
                ret_to_overview,
            }),
    );
    column![
        row![
            make_button("BLACK")
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            button(centered_text("=").size(LARGE_TEXT))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .width(Length::Fill)
                .on_press(Message::ChangeColor(None))
                .style(equal_style),
            make_button("WHITE")
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        vertical_space(Length::Fixed(SPACING)),
        make_penalty_dropdown(foul, expanded),
        vertical_space(Length::Fill),
        exit_row,
    ]
    .into()
}
