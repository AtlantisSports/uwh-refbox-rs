//! View_builder for the beep-test screen.
//!
//! Shows a refbox-standard time bar, a `[LEVEL: N]` / `[LAPS: N]` widget
//! row, the transposed levels table (one column per user level, with a
//! cell per lap), and the [RESET] [SETTINGS] [START/STOP] bottom row.
//!
//! Reachable when `config.mode == Mode::BeepTest`.
//!
//! ## Cadence semantics
//!
//! - `BeepTestPeriod::Level(0)` is the 10-second warmup (1 lap). The
//!   `[LEVEL: N]` widget reads "Level 0"; the `[LAPS: N]` widget reads
//!   "LAPS: 1"; the table has no highlighted cell.
//! - `BeepTestPeriod::Level(i)` for `i in 1..=config.levels.len()` is the
//!   i-th user-defined level (reading `config.levels[i-1]`). The widget
//!   row reads the level number and the within-level lap; column `i` of
//!   the table is the active column, and the cell at row
//!   `(within_level_lap - 1)` is highlighted as active.
//!
//! The snapshot's `lap_count` field is cumulative across the whole run
//! (it counts laps completed by the engine, including the warmup). The
//! within-level lap is derived from it and the level config below.

use super::*;
use crate::beep_test::snapshot::{BeepTestPeriod, BeepTestSnapshot};
use crate::config::BeepTest;
use iced::{
    Element, Length,
    alignment::Horizontal,
    widget::{Column, Container, Row, Space, button, column, container, row, text},
};
use matrix_drawing::secs_to_long_time_string;

/// Maximum number of level columns in a single horizontal band of the
/// levels table. When `config.levels.len()` exceeds this, the table wraps
/// onto additional bands stacked vertically.
const BAND_WIDTH: usize = 10;

/// Threshold (number of bands) above which the timer bar and widget row
/// collapse into a single combined row to free up vertical space for the
/// table.
const BAND_COUNT_COLLAPSE_THRESHOLD: usize = 2;

/// Spacing between cells in the levels table. Smaller than the global
/// `SPACING` so the table reads as a tight grid.
const TABLE_CELL_SPACING: f32 = 2.0;

pub(in super::super) fn build_beep_test_page<'a>(
    snapshot: &BeepTestSnapshot,
    config: &'a BeepTest,
    clock_running: bool,
    has_run: bool,
) -> Element<'a, Message> {
    // ----- Highlight state -----
    //
    // Compute the active level (1-indexed into config.levels, i.e. the
    // table column index) and the active within-level lap (1-indexed).
    // `None` means no highlight (warmup / Level(0)).
    let (active_level, active_within_lap) = match snapshot.current_period {
        BeepTestPeriod::Level(0) => (None, None),
        BeepTestPeriod::Level(i) => {
            let within = within_level_lap(snapshot.lap_count, i, config);
            (Some(i), Some(within))
        }
    };

    // ----- Widget content -----
    let time_text = secs_to_long_time_string(snapshot.secs_in_period)
        .trim()
        .to_owned();

    // Level label uses the existing `BeepTestPeriod::Display` impl
    // ("Level 0", "Level 1", ...) via the existing translation key.
    let level_label = match snapshot.current_period {
        BeepTestPeriod::Level(i) => fl!("beep-test-level", level = i.to_string()),
    };

    // Laps shown is the within-level lap. For the warmup (Level 0) this
    // is always 1 since the warmup is one lap; for `Level(i >= 1)` it is
    // derived from the cumulative `lap_count`.
    let laps_value: u32 = match snapshot.current_period {
        BeepTestPeriod::Level(0) => 1,
        BeepTestPeriod::Level(_) => active_within_lap.unwrap_or(1),
    };
    let laps_label = fl!("beep-test-laps", laps = laps_value.to_string());

    // ----- Table -----
    let levels_table = build_levels_table(&config.levels, active_level, active_within_lap);
    let band_count = config.levels.len().div_ceil(BAND_WIDTH).max(1);

    // ----- Header rows -----
    //
    // When the table needs more than two bands, collapse the timer and
    // info widgets into a single horizontal row to free up vertical
    // space for the table.
    let header: Element<'a, Message> = if band_count > BAND_COUNT_COLLAPSE_THRESHOLD {
        row![
            time_bar(time_text),
            info_widget(level_label),
            info_widget(laps_label),
        ]
        .spacing(SPACING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .into()
    } else {
        column![
            time_bar(time_text),
            row![info_widget(level_label), info_widget(laps_label),].spacing(SPACING),
        ]
        .spacing(SPACING)
        .into()
    };

    // ----- Bottom action row (preserved from the absorption branch) -----
    let start_stop = if clock_running {
        make_button(fl!("beep-test-stop"))
            .style(orange_button)
            .on_press(Message::BeepTestStop)
    } else {
        make_button(fl!("beep-test-start"))
            .style(green_button)
            .on_press(Message::BeepTestStart)
    };

    // Reset is disabled until the operator has pressed Start at least
    // once in this session.
    let reset = if has_run {
        make_button(fl!("beep-test-reset"))
            .style(red_button)
            .on_press(Message::BeepTestReset)
    } else {
        make_button(fl!("beep-test-reset")).style(gray_button)
    };

    let settings = make_button(fl!("settings"))
        .style(light_gray_button)
        .on_press(Message::EditGameConfig);

    let bottom_row = row![reset, settings, start_stop].spacing(SPACING);

    column![
        header,
        container(levels_table)
            .width(Length::Fill)
            .height(Length::Fill),
        bottom_row,
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Refbox-standard time banner: yellow large digits centered on a
/// light-gray button face, matching the game-mode time button.
fn time_bar<'a>(time_text: String) -> Container<'a, Message> {
    container(
        text(time_text)
            .size(LARGE_TEXT)
            .style(yellow_text)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
    )
    .style(light_gray_container)
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .center_x(Length::Fill)
    .center_y(Length::Fill)
}

/// One info widget in the row between the time bar and the table (e.g.
/// `[LEVEL: 1]`, `[LAPS: 5]`). Light-gray container matching the
/// refbox-standard info-tile styling used elsewhere.
fn info_widget<'a>(label: String) -> Container<'a, Message> {
    container(
        text(label)
            .size(MEDIUM_TEXT)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
    )
    .style(light_gray_container)
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .center_x(Length::Fill)
    .center_y(Length::Fill)
}

/// Build the transposed levels table.
///
/// One column per user level (1-indexed for the operator). Each column
/// has a header showing the level number and `count[i]` cells stacked
/// vertically, each showing the duration in seconds. Bands of up to
/// `BAND_WIDTH` columns wrap onto additional rows when there are more
/// user levels than `BAND_WIDTH`.
fn build_levels_table(
    levels: &[crate::config::Level],
    active_level: Option<usize>,
    active_within_lap: Option<u32>,
) -> Element<'_, Message> {
    let mut bands = Column::new().spacing(SPACING);

    for band_chunk in levels.chunks(BAND_WIDTH).enumerate() {
        let (band_idx, band_levels) = band_chunk;
        let level_index_offset = band_idx * BAND_WIDTH; // 0-based config index of first level in this band
        let max_count = band_levels
            .iter()
            .map(|l| l.count as usize)
            .max()
            .unwrap_or(0);

        // Header row: level numbers (1-indexed). The active level's
        // header is highlighted yellow; padding cells fill any gap so
        // the band's columns stay the same width.
        let mut header_row = Row::new().spacing(TABLE_CELL_SPACING);
        for (col_idx, _level) in band_levels.iter().enumerate() {
            let level_number = level_index_offset + col_idx + 1; // 1-indexed
            let is_active_column = active_level == Some(level_number);
            header_row = header_row.push(header_cell(level_number.to_string(), is_active_column));
        }
        // Right-pad the header so partially-filled bands align with full
        // bands above them.
        for _ in band_levels.len()..BAND_WIDTH {
            header_row = header_row.push(filler_cell());
        }
        bands = bands.push(header_row);

        // Cell rows: stacked vertically. Row 0 is the first lap, row 1
        // the second, etc. A column has `level.count` cells; rows
        // beyond a column's count are empty space.
        for row_idx in 0..max_count {
            let mut cell_row = Row::new().spacing(TABLE_CELL_SPACING);
            for (col_idx, level) in band_levels.iter().enumerate() {
                let level_number = level_index_offset + col_idx + 1;
                if row_idx < level.count as usize {
                    let is_active_column = active_level == Some(level_number);
                    let cell_state = if is_active_column {
                        match active_within_lap {
                            Some(within) if (within as usize) == row_idx + 1 => CellState::Active,
                            Some(within) if (within as usize) > row_idx + 1 => CellState::Completed,
                            _ => CellState::Default,
                        }
                    } else {
                        CellState::Default
                    };
                    cell_row =
                        cell_row.push(value_cell(level.duration.as_secs().to_string(), cell_state));
                } else {
                    cell_row = cell_row.push(filler_cell());
                }
            }
            for _ in band_levels.len()..BAND_WIDTH {
                cell_row = cell_row.push(filler_cell());
            }
            bands = bands.push(cell_row);
        }
    }

    container(bands)
        .padding(TABLE_CELL_SPACING)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Visual state of a value cell in the levels table.
#[derive(Clone, Copy)]
enum CellState {
    /// Future or stopped — render in the default cell style.
    Default,
    /// The currently-running lap within the active level.
    Active,
    /// A lap within the active level that has already completed.
    Completed,
}

/// A column-header cell showing a level number.
fn header_cell<'a>(label: String, is_active: bool) -> Element<'a, Message> {
    let inner = text(label)
        .size(SMALL_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    if is_active {
        // No `on_press` — the button is purely a styled rectangle.
        button(
            container(inner)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .style(yellow_button)
        .padding(TABLE_CELL_SPACING)
        .width(Length::Fill)
        .into()
    } else {
        container(inner)
            .style(light_gray_container)
            .padding(TABLE_CELL_SPACING)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

/// One data cell in the levels table.
fn value_cell<'a>(value: String, state: CellState) -> Element<'a, Message> {
    let inner = text(value)
        .size(SMALL_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    match state {
        CellState::Default => container(inner)
            .style(light_gray_container)
            .padding(TABLE_CELL_SPACING)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        CellState::Active => button(
            container(inner)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .style(yellow_button)
        .padding(TABLE_CELL_SPACING)
        .width(Length::Fill)
        .into(),
        CellState::Completed => container(inner)
            .style(disabled_container)
            .padding(TABLE_CELL_SPACING)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    }
}

/// Empty filler cell — keeps column widths consistent when a band is
/// partially filled or a column's count is shorter than the band's
/// tallest column.
fn filler_cell<'a>() -> Element<'a, Message> {
    Space::with_width(Length::Fill).into()
}

/// Compute the within-level lap (1-indexed) given the snapshot's
/// cumulative `lap_count` and the active level index.
///
/// The engine's `lap_count` is cumulative across the whole run (it is
/// incremented every time a lap completes, including the warmup). The
/// within-level lap is what the operator wants to see, so we subtract
/// the laps belonging to prior levels.
///
/// Laps completed in prior levels:
/// - Level(0) (warmup) contributes 1 lap.
/// - Level(i) for i in 1..=N contributes `levels[i-1].count` laps.
///
/// While in Level(i) (i >= 1), the engine has already incremented
/// `lap_count` for every prior period's laps. The within-level lap is
/// `lap_count - laps_before(i) + 1`, where `laps_before(i)` is the sum
/// of all laps in periods Level(0)..Level(i-1).
fn within_level_lap(lap_count: u8, level: usize, config: &BeepTest) -> u32 {
    if level == 0 {
        // Warmup is always lap 1 of 1.
        return 1;
    }
    // Warmup contributes 1 lap; prior user levels contribute their counts.
    let prior_user_levels = level.saturating_sub(1); // Level(1) has 0 prior user levels, Level(2) has 1, etc.
    let prior_user_laps: u32 = config
        .levels
        .iter()
        .take(prior_user_levels)
        .map(|l| u32::from(l.count))
        .sum();
    let laps_before = 1 + prior_user_laps;
    let lap_count = u32::from(lap_count);
    lap_count.saturating_sub(laps_before).saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Level;
    use std::time::Duration;

    fn cfg(counts: &[u8]) -> BeepTest {
        BeepTest {
            pre: Duration::from_secs(10),
            levels: counts
                .iter()
                .map(|&c| Level {
                    count: c,
                    duration: Duration::from_secs(30),
                })
                .collect(),
        }
    }

    #[test]
    fn within_level_lap_warmup_is_one() {
        let c = cfg(&[3, 2]);
        assert_eq!(within_level_lap(0, 0, &c), 1);
    }

    #[test]
    fn within_level_lap_first_level_starts_at_one() {
        let c = cfg(&[3, 2]);
        // After the warmup lap completes, lap_count = 1, period = Level(1).
        assert_eq!(within_level_lap(1, 1, &c), 1);
    }

    #[test]
    fn within_level_lap_first_level_progresses() {
        let c = cfg(&[3, 2]);
        // Level(1) lap 1 → lap_count goes to 2 (still in Level(1)).
        assert_eq!(within_level_lap(2, 1, &c), 2);
        // Level(1) lap 2 → lap_count goes to 3 (still in Level(1)).
        assert_eq!(within_level_lap(3, 1, &c), 3);
    }

    #[test]
    fn within_level_lap_second_level_starts_at_one() {
        let c = cfg(&[3, 2]);
        // After Level(1)'s third lap, lap_count = 4, period = Level(2).
        assert_eq!(within_level_lap(4, 2, &c), 1);
        assert_eq!(within_level_lap(5, 2, &c), 2);
    }
}
