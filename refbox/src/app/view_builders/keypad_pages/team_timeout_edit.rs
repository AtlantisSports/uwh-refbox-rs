use super::{style::Element, *};
use iced::{
    Length,
    widget::{column, horizontal_space, row, vertical_space},
};
use style::SMALL_PLUS_TEXT;

use std::time::Duration;

pub(super) fn make_team_timeout_edit_page<'a>(
    duration: Duration,
    timeouts_counted_per_half: bool,
) -> Element<'a, Message> {
    let (half_style, half_message, game_style, game_message) = if timeouts_counted_per_half {
        (
            ButtonStyle::BlueSelected,
            Message::NoAction,
            ButtonStyle::Blue,
            Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
        )
    } else {
        (
            ButtonStyle::Blue,
            Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
            ButtonStyle::BlueSelected,
            Message::NoAction,
        )
    };

    column![
        row![
            text("TIMEOUTS\nCOUNTED PER:")
                .size(SMALL_PLUS_TEXT)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .vertical_alignment(Vertical::Center),
            make_button("HALF")
                .style(half_style)
                .width(Length::Fill)
                .on_press(half_message),
            make_button("GAME")
                .style(game_style)
                .width(Length::Fill)
                .on_press(game_message),
        ]
        .spacing(SPACING),
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            make_time_editor("TIMEOUT LENGTH", duration, false),
            horizontal_space(Length::Fill)
        ],
        vertical_space(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button("DONE")
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
