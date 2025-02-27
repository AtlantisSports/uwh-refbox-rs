use super::{
    style::{ButtonStyle, LINE_HEIGHT, SMALL_TEXT, SPACING},
    *,
};
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
        horizontal_space(Length::Fill),
        make_time_editor("GAME TIME", time, false),
        horizontal_space(Length::Fill)
    ]
    .spacing(SPACING)
    .align_items(Alignment::Center);

    if snapshot.timeout.is_some() {
        edit_row = edit_row
            .push(horizontal_space(Length::Fill))
            .push(make_time_editor("TIMEOUT", timeout_time.unwrap(), true))
            .push(horizontal_space(Length::Fill));
    }

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        vertical_space(Length::Fill),
        text("Note: Game time is paused while on this screen")
            .size(SMALL_TEXT)
            .line_height(LINE_HEIGHT)
            .width(Length::Fill)
            .horizontal_alignment(Horizontal::Center),
        vertical_space(Length::Fill),
        edit_row,
        vertical_space(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button("DONE")
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
