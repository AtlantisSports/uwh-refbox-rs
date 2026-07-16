use super::*;
use iced::{
    Length,
    widget::{column, horizontal_space, row, vertical_space},
};

pub(in super::super) fn build_power_page<'a>(data: ViewData<'_, '_>) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    // `editing_time = true` keeps the banner clock non-interactive here (no
    // red flashing, no jump to the time editor), matching the confirmation
    // pages.
    let banner = make_game_time_button(
        snapshot,
        false,
        true,
        mode,
        clock_running,
        portal_indicator,
        None,
    );

    let actions = row![
        make_button(fl!("shut-down"))
            .style(orange_button)
            .width(Length::Fill)
            .on_press(Message::PowerAction(PowerAction::ShutDownPi)),
        make_button(fl!("restart-pi"))
            .style(yellow_button)
            .width(Length::Fill)
            .on_press(Message::PowerAction(PowerAction::RestartPi)),
        make_button(fl!("restart-refbox"))
            .style(blue_button)
            .width(Length::Fill)
            .on_press(Message::PowerAction(PowerAction::RestartRefbox)),
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    // Back sits bottom-left at one-third width, matching every other page's
    // Back button; the two empty thirds keep it left-aligned.
    let back_row = row![
        make_button(fl!("back"))
            .style(red_button)
            .width(Length::Fill)
            .on_press(Message::ShowGameDetails),
        horizontal_space(),
        horizontal_space(),
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    column![banner, actions, vertical_space(), back_row]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}
