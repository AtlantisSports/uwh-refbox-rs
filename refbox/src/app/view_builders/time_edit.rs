use super::{
    style::{self, SPACING},
    *,
};

use iced::{
    pure::{column, horizontal_space, row, vertical_space, Element},
    Alignment, Length,
};

use std::time::Duration;
use uwh_common::game_snapshot::{GameSnapshot, TimeoutSnapshot};

pub(in super::super) fn build_time_edit_view<'a>(
    snapshot: &GameSnapshot,
    time: Duration,
    timeout_time: Option<Duration>,
) -> Element<'a, Message> {
    let mut edit_row = row()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .push(horizontal_space(Length::Fill))
        .push(make_time_editor("GAME TIME", time, false))
        .push(horizontal_space(Length::Fill));

    if snapshot.timeout != TimeoutSnapshot::None {
        edit_row = edit_row
            .push(horizontal_space(Length::Fill))
            .push(make_time_editor("TIMEOUT", timeout_time.unwrap(), true))
            .push(horizontal_space(Length::Fill));
    }

    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, false).on_press(Message::NoAction))
        .push(vertical_space(Length::Fill))
        .push(edit_row)
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::TimeEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::TimeEditComplete { canceled: false }),
                ),
        )
        .into()
}
