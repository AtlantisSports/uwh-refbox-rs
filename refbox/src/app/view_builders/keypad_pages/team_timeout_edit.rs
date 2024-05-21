use super::{style::Element, *};
use iced::{
    widget::{column, horizontal_space, row, vertical_space},
    Length,
};

use std::time::Duration;

pub(super) fn make_team_timeout_edit_page<'a>(duration: Duration) -> Element<'a, Message> {
    column![
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            make_time_editor(fl!("timeout-length"), duration, false),
            horizontal_space(Length::Fill)
        ],
        vertical_space(Length::Fill),
        row![
            make_button(fl!("cancel"))
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(fl!("done"))
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
