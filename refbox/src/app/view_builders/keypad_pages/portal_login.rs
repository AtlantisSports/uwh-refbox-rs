use super::*;
use crate::config::Mode;
use iced::{
    Length,
    widget::{column, row, vertical_space},
};

pub(super) fn make_portal_login_page<'a>(
    id: u32,
    requested: bool,
    mode: Mode,
    source: GameSource,
) -> Element<'a, Message> {
    // A custom site gets generic wording. The Portal's own instructions name
    // its menus — Event Management, Referee Management, the + button — and
    // refbox knows only the address the operator typed, never what a
    // third-party site calls its admin screens. Manual never reaches this page.
    let instructions = if source == GameSource::Custom {
        fl!("custom-login-instructions", id = id)
    } else {
        fl!(
            "portal-login-instructions",
            id = id,
            portal = portal_name_for_mode(mode)
        )
    };

    column![
        text(instructions).width(Length::Fill),
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
