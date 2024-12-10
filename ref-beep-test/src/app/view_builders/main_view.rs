use super::{super::super::snapshot::BeepTestSnapshot, shared_elements::make_info_container};
use iced::{
    widget::{column, row, Space},
    Length,
};

use crate::{
    app::{message::Message, view_builders::shared_elements::build_levels_table},
    config::BeepTest,
};

use super::{
    super::style::{ButtonStyle, Element, SPACING},
    shared_elements::{make_button, make_time_button},
};

pub(in super::super) fn build_main_view<'a>(
    snapshot: &BeepTestSnapshot,
    clock_running: bool,
    beep_test: &'a BeepTest,
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

    let lap_info = make_info_container(snapshot);

    let chart = build_levels_table(&beep_test.levels, false);

    let settings = make_button("SETTINGS")
        .on_press(Message::ShowSettings)
        .style(ButtonStyle::Gray);

    if beep_test.levels.len() > 13 {
        let chart_first_col = build_levels_table(&beep_test.levels[..13], false);
        let chart_second_col = build_levels_table(&beep_test.levels[13..], true);
        let chart_row = row![chart_first_col, chart_second_col].spacing(SPACING);
        content = content.push(row![lap_info, chart_row].spacing(SPACING));
    } else {
        let gap1 = Space::with_width(Length::Fixed(115.0));
        let gap2 = Space::with_width(Length::Fixed(115.0));
        let chart_row = row![gap1, chart, gap2];
        content = content.push(row![lap_info, chart_row].spacing(SPACING));
    }

    content = content.push(row![settings].spacing(SPACING));

    content.into()
}
