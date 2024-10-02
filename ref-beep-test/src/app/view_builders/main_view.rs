use super::super::super::snapshot::BeepTestSnapshot;
use iced::widget::{column, row};

use crate::app::message::Message;

use super::{
    super::style::{ButtonStyle, Element, SPACING},
    shared_elements::{make_button, make_time_button},
};

pub(in super::super) fn build_main_view<'a>(
    snapshot: &BeepTestSnapshot,
    clock_running: bool,
) -> Element<'a, Message> {
    let time = make_time_button(snapshot);

    let mut content = column![time].spacing(SPACING);

    let start_pause = if !clock_running {
        make_button("START")
            .on_press(Message::Start)
            .style(ButtonStyle::Green)
    } else {
        make_button("PAUSE")
            .on_press(Message::Stop)
            .style(ButtonStyle::Orange)
    };

    let reset = make_button("RESET")
        .on_press(Message::Reset)
        .style(ButtonStyle::Red);

    content = content.push(row![start_pause, reset].spacing(SPACING));

    content.into()
}
