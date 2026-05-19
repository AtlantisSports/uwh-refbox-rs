//! View_builder for the beep-test screen.
//!
//! Shows the cadence timer, level indicator, lap count, read-only levels
//! table, and start/stop and reset controls. Reachable when
//! `config.mode == Mode::BeepTest`.

use super::*;
use crate::beep_test::snapshot::{BeepTestPeriod, BeepTestSnapshot};
use crate::config::BeepTest;
use iced::{
    Element, Length,
    alignment::Horizontal,
    widget::{Column, Container, Row, Text, button, column, container, row, text},
};
use matrix_drawing::secs_to_long_time_string;

/// Width of a single cell in the read-only levels table.
const TABLE_CELL_WIDTH: f32 = 80.0;
/// Spacing between cells in the read-only levels table.
const TABLE_CELL_SPACING: f32 = 2.0;

pub(in super::super) fn build_beep_test_page<'a>(
    snapshot: &BeepTestSnapshot,
    config: &'a BeepTest,
    clock_running: bool,
) -> Element<'a, Message> {
    // Big MM:SS countdown.
    let time_text = secs_to_long_time_string(snapshot.secs_in_period)
        .trim()
        .to_owned();
    let time_display = button(
        container(text(time_text).size(LARGE_TEXT).style(yellow_text))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(gray_button)
    .padding(PADDING);

    // Level indicator: "LEVEL N".
    let level_label = match snapshot.current_period {
        BeepTestPeriod::Level(i) => fl!("beep-test-level", level = i.to_string()),
    };
    let level_display = container(text(level_label).size(MEDIUM_TEXT))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .style(light_gray_container)
        .padding(PADDING);

    // Lap count.
    let lap_display = container(
        text(fl!("beep-test-laps", laps = snapshot.lap_count.to_string())).size(MEDIUM_TEXT),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(light_gray_container)
    .padding(PADDING);

    let right_col = column![time_display, level_display, lap_display,]
        .spacing(SPACING)
        .width(Length::Fill);

    let levels_table = build_levels_table(&config.levels);

    // Single toggle button: shows START (green) when stopped, STOP (orange)
    // when running. Matches the standalone beep-test's one-button pattern,
    // which is the operator-friendly choice — one click always does the
    // right thing.
    let start_stop = if clock_running {
        make_button(fl!("beep-test-stop"))
            .style(orange_button)
            .on_press(Message::BeepTestStop)
    } else {
        make_button(fl!("beep-test-start"))
            .style(green_button)
            .on_press(Message::BeepTestStart)
    };

    let reset = make_button(fl!("beep-test-reset"))
        .style(red_button)
        .on_press(Message::BeepTestReset);

    let settings = make_button(fl!("settings"))
        .style(light_gray_button)
        .on_press(Message::EditGameConfig);

    // Bottom action row: [RESET] [SETTINGS] [START/STOP] — mirrors the
    // Cancel | (middle) | Apply convention used throughout the refbox UI.
    let bottom_row = row![reset, settings, start_stop].spacing(SPACING);

    column![
        row![levels_table, right_col]
            .spacing(SPACING)
            .height(Length::Fill),
        bottom_row,
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Build the read-only levels table.
///
/// Columns: LEVEL number, COUNT (laps), DURATION (per lap, in seconds).
/// Cells are placed on a dark background to give a grid appearance,
/// mirroring the standalone beep-test's layout.
fn build_levels_table(levels: &[crate::config::Level]) -> Container<'_, Message> {
    let mut table = Column::new().spacing(TABLE_CELL_SPACING);

    let headers: Row<Message> = row![
        header_cell(fl!("beep-test-column-level")),
        header_cell(fl!("beep-test-column-count")),
        header_cell(fl!("beep-test-column-duration")),
    ]
    .spacing(TABLE_CELL_SPACING);
    table = table.push(headers);

    for (index, level) in levels.iter().enumerate() {
        let row_widget: Row<Message> = row![
            value_cell((index + 1).to_string()),
            value_cell(level.count.to_string()),
            value_cell(level.duration.as_secs().to_string()),
        ]
        .spacing(TABLE_CELL_SPACING);
        table = table.push(row_widget);
    }

    container(table)
        .style(black_container)
        .padding(TABLE_CELL_SPACING)
}

/// One header cell in the levels table.
fn header_cell<'a>(label: String) -> Container<'a, Message> {
    container(
        Text::new(label)
            .width(Length::Fixed(TABLE_CELL_WIDTH))
            .size(SMALL_TEXT)
            .align_x(Horizontal::Center),
    )
    .style(light_gray_container)
    .padding(TABLE_CELL_SPACING)
}

/// One data cell in the levels table.
fn value_cell<'a>(value: String) -> Container<'a, Message> {
    container(
        Text::new(value)
            .width(Length::Fixed(TABLE_CELL_WIDTH))
            .size(SMALL_TEXT)
            .align_x(Horizontal::Center),
    )
    .style(light_gray_container)
    .padding(TABLE_CELL_SPACING)
}
