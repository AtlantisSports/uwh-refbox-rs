use super::*;
use iced::{
    Alignment, Length,
    alignment::Horizontal,
    widget::{column, horizontal_space, row, text, vertical_space},
};
use std::time::Duration;

pub(in super::super) fn build_time_edit_view<'a>(
    data: ViewData<'_, '_>,
    time: Duration,
    timeout_time: Option<Duration>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let mut edit_row = row![
        horizontal_space(),
        make_time_editor(fl!("game-time"), time, false),
        horizontal_space()
    ]
    .spacing(SPACING)
    .align_y(Alignment::Center);

    if snapshot.timeout.is_some() {
        edit_row = edit_row
            .push(horizontal_space())
            .push(make_time_editor(
                fl!("timeout"),
                timeout_time.unwrap(),
                true,
            ))
            .push(horizontal_space());
    }

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        vertical_space(),
        text(fl!("Note-Game-time-is-paused"))
            .size(SMALL_TEXT)
            .line_height(LINE_HEIGHT)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        vertical_space(),
        edit_row,
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
