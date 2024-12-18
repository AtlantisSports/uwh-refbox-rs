use super::*;
use iced::{
    Length, Theme,
    widget::{
        button::{Status, Style},
        column, horizontal_space, row, vertical_space,
    },
};
use std::time::Duration;

type StyleFn = fn(&Theme, Status) -> Style;

pub(super) fn make_team_timeout_edit_page<'a>(
    duration: Duration,
    timeouts_counted_per_half: bool,
) -> Element<'a, Message> {
    let (half_style, half_message, game_style, game_message): (StyleFn, _, StyleFn, _) =
        if timeouts_counted_per_half {
            (
                blue_selected_button,
                Message::NoAction,
                blue_button,
                Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
            )
        } else {
            (
                blue_button,
                Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
                blue_selected_button,
                Message::NoAction,
            )
        };

    column![
        row![
            text(fl!("timeouts-counted-per"))
                .size(SMALL_PLUS_TEXT)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .align_y(Vertical::Center),
            make_button(fl!("half"))
                .style(half_style)
                .width(Length::Fill)
                .on_press(half_message),
            make_button(fl!("game"))
                .style(game_style)
                .width(Length::Fill)
                .on_press(game_message),
        ]
        .spacing(SPACING),
        vertical_space(),
        row![
            horizontal_space(),
            make_time_editor(fl!("timeout-length"), duration, false),
            horizontal_space()
        ],
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
