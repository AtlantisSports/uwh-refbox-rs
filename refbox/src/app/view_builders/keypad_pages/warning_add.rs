use super::*;
use iced::{
    Length, Theme,
    widget::{
        Space,
        button::{Status, Style},
        column, row, vertical_space,
    },
};
use uwh_common::color::Color as GameColor;

type StyleFn = fn(&Theme, Status) -> Style;

pub(super) fn make_warning_add_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    foul: Infraction,
    team_warning: bool,
    ret_to_overview: bool,
) -> Element<'a, Message> {
    let (black_style, white_style): (StyleFn, StyleFn) = match color {
        GameColor::Black => (black_selected_button, white_button),
        GameColor::White => (black_button, white_selected_button),
    };

    let team_warning_style = if team_warning {
        blue_selected_button
    } else {
        blue_button
    };

    let mut exit_row = row![
        make_button(fl!("cancel"))
            .style(red_button)
            .width(Length::Fill)
            .on_press(Message::WarningEditComplete {
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
                .on_press(Message::WarningEditComplete {
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
            .on_press(Message::WarningEditComplete {
                canceled: false,
                deleted: false,
                ret_to_overview,
            }),
    );
    column![
        row![
            make_multi_label_message_button(
                (fl!("team-warning-line-1"), fl!("team-warning-line-2")),
                Some(Message::ToggleBoolParameter(BoolGameParameter::TeamWarning))
            )
            .style(team_warning_style),
            make_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
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
