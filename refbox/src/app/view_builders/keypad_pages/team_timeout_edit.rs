use super::{
    style::{self, SPACING},
    *,
};

use iced::{
    pure::{column, horizontal_space, row, vertical_space, Element},
    Length,
};

use std::time::Duration;

pub(super) fn make_team_timeout_edit_page<'a>(duration: Duration) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .push(horizontal_space(Length::Fill))
                .push(make_time_editor("TIMEOUT LENGTH", duration, false))
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}
