use super::{style::Element, *};
use iced::{
    widget::{column, row, vertical_space},
    Length,
};

use uwh_common::game_snapshot::Color as GameColor;

pub(super) fn make_warning_add_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    foul: Infraction,
    team_warning: bool,
    ret_to_overview: bool,
) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (ButtonStyle::BlackSelected, ButtonStyle::White),
        GameColor::White => (ButtonStyle::Black, ButtonStyle::WhiteSelected),
    };

    let team_warning_style = if team_warning {
        ButtonStyle::BlueSelected
    } else {
        ButtonStyle::Blue
    };

    let mut exit_row = row![make_button("CANCEL")
        .style(ButtonStyle::Red)
        .width(Length::Fill)
        .on_press(Message::WarningEditComplete {
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
                .on_press(Message::WarningEditComplete {
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
            .on_press(Message::WarningEditComplete {
                canceled: false,
                deleted: false,
                ret_to_overview,
            }),
    );
    column![
        row![
            make_multi_label_message_button(
                ("TEAM", "WARNING"),
                Some(Message::ToggleBoolParameter(BoolGameParameter::TeamWarning))
            )
            .style(team_warning_style),
            make_button("BLACK")
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            make_button("WHITE")
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        vertical_space(Length::Fixed(SPACING)),
        make_penalty_dropdown(foul, true),
        vertical_space(Length::Fill),
        exit_row,
    ]
    .into()
}
