use super::{style::Element, *};
use iced::{
    widget::{column, row, vertical_space},
    Length,
};

pub(super) fn make_game_number_edit_page<'a>() -> Element<'a, Message> {
    column![
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
