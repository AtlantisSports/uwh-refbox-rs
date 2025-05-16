use super::*;
use iced::{
    Length,
    widget::{column, row, vertical_space},
};

pub(super) fn make_portal_login_page<'a>(id: u32, requested: bool) -> Element<'a, Message> {
    column![
        text(fl!("portal-login-instructions", id = id)).width(Length::Fill),
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(if !requested {
                fl!("done")
            } else {
                fl!("loading")
            })
            .style(green_button)
            .width(Length::Fill)
            .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
