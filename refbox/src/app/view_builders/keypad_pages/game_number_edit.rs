use super::{
    style::{self, SPACING},
    *,
};

use iced::{
    pure::{column, row, vertical_space, Element},
    Length,
};

pub(super) fn make_game_number_edit_page<'a>() -> Element<'a, Message> {
    column()
        .spacing(SPACING)
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
