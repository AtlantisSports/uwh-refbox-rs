//! View_builders for the BeepTest Settings sub-pages.
//!
//! Reachable when `app_state == AppState::BeepTestSettings(_)`. Each
//! function builds one sub-page: the 2x2 landing (this file's only fully
//! implemented page in Task 4 of the redesign), and stubs for Sound
//! Settings (Task 5), Edit Levels (Task 6), and App Mode (Task 7).
//!
//! The Language sub-page is not modelled here — the landing's Language
//! button routes the operator into the existing
//! `AppState::EditGameConfig(ConfigPage::Language)` flow via
//! `Message::BeepTestOpenLanguageSettings`.

use super::*;
use crate::app::BeepTestConfigPage;
use crate::config::Level;
use iced::{
    Element, Length,
    alignment::Horizontal,
    widget::{Column, Row, Space, button, column, container, horizontal_space, row, text},
};

/// 2x2 landing page for the BeepTest Settings hierarchy.
///
/// Layout (top to bottom):
/// - Row 1: [SOUND SETTINGS] [EDIT LEVELS]
/// - Row 2: [APP MODE]       [LANGUAGE]
/// - Filler
/// - Bottom row: [BACK]      [DONE]
///
/// Both `BACK` and `DONE` close the Settings hierarchy and return to the
/// BeepTest main view; they exist as two visually-distinct affordances
/// in the same position as the Hockey Settings page.
pub(in super::super) fn build_beep_test_settings_landing<'a>() -> Element<'a, Message> {
    let sound_button = make_button(fl!("sound-settings"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenSound);

    let edit_levels_button = make_button(fl!("beep-test-edit-levels"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenLevels);

    let app_mode_button = make_button(fl!("app-mode"))
        .style(light_gray_button)
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::AppMode));

    let language_button = make_button(fl!("language"))
        .style(light_gray_button)
        .on_press(Message::BeepTestOpenLanguageSettings);

    let row_top = row![sound_button, edit_levels_button]
        .spacing(SPACING)
        .height(Length::Fill);

    let row_bottom = row![app_mode_button, language_button]
        .spacing(SPACING)
        .height(Length::Fill);

    let back_button = make_button(fl!("back"))
        .style(red_button)
        .on_press(Message::BeepTestCloseSettings);

    let done_button = make_button(fl!("done"))
        .style(green_button)
        .on_press(Message::BeepTestCloseSettings);

    let bottom_row = row![back_button, horizontal_space(), done_button].spacing(SPACING);

    column![row_top, row_bottom, bottom_row]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}

/// Sound Settings sub-page for the BeepTest hierarchy.
///
/// Layout (3 columns × 2 rows of controls, plus a title row and a
/// Cancel/Save action row at the bottom):
///
/// ```text
/// [ SOUND SETTINGS                                          ]
/// [ SOUND ENABLED  | ABOVE WATER VOL | WHISTLE ENABLED      ]
/// [ BUZZER SOUND   | BELOW WATER VOL | WHISTLE VOL          ]
/// [ CANCEL                                            SAVE  ]
/// ```
///
/// Disabled-gating:
/// - SOUND ENABLED is always interactive.
/// - When `sound.sound_enabled == false`, the other five controls render
///   disabled (no `on_press`).
/// - WHISTLE VOL is additionally gated by `sound.whistle_enabled`: it
///   renders disabled when whistle is off, regardless of sound-enabled.
///
/// The controls reuse the existing `ToggleBoolParameter` and
/// `CycleParameter` messages used by the hockey-mode Sound configuration
/// page; those handlers mutate `edited_settings.sound`, which is seeded
/// by `Message::BeepTestEditOpenSound` before this page is reached.
pub(in super::super) fn build_beep_test_sound_settings_page<'a>(
    sound: &SoundSettings,
) -> Element<'a, Message> {
    let sound_enabled = sound.sound_enabled;
    let whistle_vol_enabled = sound_enabled && sound.whistle_enabled;

    // SOUND ENABLED — always interactive.
    let sound_enabled_btn = make_value_button(
        fl!("sound-enabled"),
        bool_string(sound.sound_enabled),
        (false, true),
        Some(Message::ToggleBoolParameter(
            BoolGameParameter::SoundEnabled,
        )),
    );

    // ABOVE WATER VOL — gated by SOUND ENABLED.
    let above_water_vol_btn = make_value_button(
        fl!("above-water-volume"),
        sound.above_water_vol.to_string(),
        (false, true),
        if sound_enabled {
            Some(Message::CycleParameter(CyclingParameter::AboveWaterVol))
        } else {
            None
        },
    );

    // WHISTLE ENABLED — gated by SOUND ENABLED.
    let whistle_enabled_btn = make_value_button(
        fl!("whistle-enabled"),
        bool_string(sound.whistle_enabled),
        (false, true),
        if sound_enabled {
            Some(Message::ToggleBoolParameter(
                BoolGameParameter::RefAlertEnabled,
            ))
        } else {
            None
        },
    );

    // BUZZER SOUND — gated by SOUND ENABLED.
    let buzzer_sound_btn = make_value_button(
        fl!("buzzer-sound"),
        sound.buzzer_sound.to_string().to_uppercase(),
        (false, true),
        if sound_enabled {
            Some(Message::CycleParameter(CyclingParameter::BuzzerSound))
        } else {
            None
        },
    );

    // BELOW WATER VOL — gated by SOUND ENABLED. Refbox calls this
    // "UNDERWATER VOLUME" in its existing translation keys; we reuse those
    // strings so all 15 locales stay in sync.
    let below_water_vol_btn = make_value_button(
        fl!("underwater-volume"),
        sound.under_water_vol.to_string(),
        (false, true),
        if sound_enabled {
            Some(Message::CycleParameter(CyclingParameter::UnderWaterVol))
        } else {
            None
        },
    );

    // WHISTLE VOL — gated by BOTH SOUND ENABLED and WHISTLE ENABLED.
    let whistle_vol_btn = make_value_button(
        fl!("whistle-volume"),
        sound.whistle_vol.to_string(),
        (false, true),
        if whistle_vol_enabled {
            Some(Message::CycleParameter(CyclingParameter::AlertVolume))
        } else {
            None
        },
    );

    let title = text(fl!("sound-settings"))
        .size(MEDIUM_TEXT)
        .width(Length::Fill);

    let row_top = row![sound_enabled_btn, above_water_vol_btn, whistle_enabled_btn]
        .spacing(SPACING)
        .height(Length::Fill);

    let row_bottom = row![buzzer_sound_btn, below_water_vol_btn, whistle_vol_btn]
        .spacing(SPACING)
        .height(Length::Fill);

    let cancel_button = make_button(fl!("cancel"))
        .style(red_button)
        .on_press(Message::BeepTestSoundSettingsCancel);

    let save_button = make_button(fl!("save"))
        .style(green_button)
        .on_press(Message::BeepTestSoundSettingsSave);

    let bottom_row = row![cancel_button, horizontal_space(), save_button].spacing(SPACING);

    column![title, row_top, row_bottom, bottom_row]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}

/// Maximum number of level columns in a single horizontal band of the
/// editor's transposed table. Mirrors the main view's BAND_WIDTH so the
/// editor reads as the same table the operator already knows.
const EDIT_BAND_WIDTH: usize = 10;

/// Spacing between cells in the editor's transposed table. Matches the
/// main view's TABLE_CELL_SPACING so the editor reads as a tight grid.
const EDIT_TABLE_CELL_SPACING: f32 = 2.0;

/// Edit Levels sub-page.
///
/// Top half: the same transposed level table from the main view, plus
/// an extra `[+NEW]` header at the end. Every header and every cell is
/// tappable: tapping any element in a column selects that level. The
/// selected column is highlighted with a distinct (blue) style to
/// distinguish it from the main view's yellow "active lap" highlight.
///
/// Bottom half: a per-level edit panel showing the selected level's
/// duration and count, each with `[-]` `[+]` buttons, and a `REMOVE
/// LEVEL` button (disabled when only one level remains).
///
/// Save commits the staged levels to live config and persists; Cancel
/// discards the staged edits. Both navigate back to the Settings
/// landing.
pub(in super::super) fn build_beep_test_edit_levels_page(
    levels: &[Level],
    selected: usize,
) -> Element<'_, Message> {
    // Clamp the selected index defensively. The handlers in update()
    // already prevent out-of-range writes, but a render pass that
    // happens to see a stale snapshot (e.g. between Remove and the next
    // tick) should still produce a sane view.
    let selected = selected.min(levels.len().saturating_sub(1));

    // ----- Transposed table with tappable headers + cells -----
    let table = build_editor_levels_table(levels, selected);

    // ----- Per-level edit panel -----
    let edit_panel = build_edit_panel(levels, selected);

    // ----- Save / Cancel bottom row -----
    let cancel_button = make_button(fl!("cancel"))
        .style(red_button)
        .on_press(Message::BeepTestEditLevelsCancel);

    let save_button = make_button(fl!("save"))
        .style(green_button)
        .on_press(Message::BeepTestEditLevelsSave);

    let bottom_row = row![cancel_button, horizontal_space(), save_button].spacing(SPACING);

    column![
        container(table)
            .width(Length::Fill)
            .height(Length::FillPortion(3)),
        container(edit_panel)
            .width(Length::Fill)
            .height(Length::FillPortion(2)),
        bottom_row,
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Build the editor's transposed levels table. Mirrors the main view's
/// table, but every header and value cell is a button that fires
/// `Message::BeepTestEditSelectLevel(i)`, the selected column uses a
/// blue highlight, and an extra `[+NEW]` header at the end of the last
/// band appends a new level.
fn build_editor_levels_table(levels: &[Level], selected: usize) -> Element<'_, Message> {
    let mut bands = Column::new().spacing(SPACING);

    // Number of bands the populated columns occupy. When levels.len() is
    // an exact multiple of EDIT_BAND_WIDTH, the [+NEW] header lives in a
    // new partial band by itself; otherwise it shares the last band.
    let populated_band_count = levels.len().div_ceil(EDIT_BAND_WIDTH).max(1);

    for (band_idx, band_levels) in levels.chunks(EDIT_BAND_WIDTH).enumerate() {
        let is_last_band = band_idx + 1 == populated_band_count;
        let level_index_offset = band_idx * EDIT_BAND_WIDTH;
        let max_count = band_levels
            .iter()
            .map(|l| l.count as usize)
            .max()
            .unwrap_or(0);

        // Header row: column headers (1-indexed for the operator).
        let mut header_row = Row::new().spacing(EDIT_TABLE_CELL_SPACING);
        for (col_idx, _level) in band_levels.iter().enumerate() {
            let level_number = level_index_offset + col_idx + 1;
            let zero_based = level_index_offset + col_idx;
            let is_selected = zero_based == selected;
            header_row = header_row.push(editor_header_cell(
                level_number.to_string(),
                zero_based,
                is_selected,
            ));
        }
        // Pad with filler cells on bands that aren't fully populated.
        let cols_used_in_band = band_levels.len();
        let cols_padding = if is_last_band && cols_used_in_band < EDIT_BAND_WIDTH {
            // Reserve one cell at the end of the last band for [+NEW].
            EDIT_BAND_WIDTH - cols_used_in_band - 1
        } else {
            EDIT_BAND_WIDTH.saturating_sub(cols_used_in_band)
        };
        for _ in 0..cols_padding {
            header_row = header_row.push(filler_cell());
        }
        // Add the [+NEW] header at the end of the last band (if there's room).
        if is_last_band && cols_used_in_band < EDIT_BAND_WIDTH {
            header_row = header_row.push(editor_new_cell());
        }
        bands = bands.push(header_row);

        // Cell rows: stacked vertically. Each cell is a tappable button
        // that selects the column it belongs to. Empty rows beyond a
        // column's count render as filler.
        for row_idx in 0..max_count {
            let mut cell_row = Row::new().spacing(EDIT_TABLE_CELL_SPACING);
            for (col_idx, level) in band_levels.iter().enumerate() {
                let zero_based = level_index_offset + col_idx;
                if row_idx < level.count as usize {
                    let is_selected = zero_based == selected;
                    cell_row = cell_row.push(editor_value_cell(
                        level.duration.as_secs().to_string(),
                        zero_based,
                        is_selected,
                    ));
                } else {
                    cell_row = cell_row.push(filler_cell());
                }
            }
            // Pad with filler cells to keep band widths consistent. The
            // [+NEW] column's body is always empty.
            let body_padding = EDIT_BAND_WIDTH.saturating_sub(cols_used_in_band);
            for _ in 0..body_padding {
                cell_row = cell_row.push(filler_cell());
            }
            bands = bands.push(cell_row);
        }

        // If the band is fully populated AND it's the last band, the
        // [+NEW] header needs its own header row to live on (a new
        // partial band by itself). Add that here.
        if is_last_band && cols_used_in_band == EDIT_BAND_WIDTH {
            let mut new_band = Row::new().spacing(EDIT_TABLE_CELL_SPACING);
            new_band = new_band.push(editor_new_cell());
            for _ in 1..EDIT_BAND_WIDTH {
                new_band = new_band.push(filler_cell());
            }
            bands = bands.push(new_band);
        }
    }

    // Edge case: empty levels list (shouldn't happen — handlers prevent
    // it — but render a usable [+NEW] anyway).
    if levels.is_empty() {
        let mut new_band = Row::new().spacing(EDIT_TABLE_CELL_SPACING);
        new_band = new_band.push(editor_new_cell());
        for _ in 1..EDIT_BAND_WIDTH {
            new_band = new_band.push(filler_cell());
        }
        bands = bands.push(new_band);
    }

    container(bands)
        .padding(EDIT_TABLE_CELL_SPACING)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// A tappable column-header cell showing a level number. Highlighted
/// blue when the column is selected; light-gray otherwise. Firing
/// `BeepTestEditSelectLevel(zero_based)` selects the column.
fn editor_header_cell<'a>(
    label: String,
    zero_based: usize,
    is_selected: bool,
) -> Element<'a, Message> {
    let inner = text(label)
        .size(SMALL_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    let style = if is_selected {
        blue_button
    } else {
        light_gray_button
    };
    button(
        container(inner)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .style(style)
    .padding(EDIT_TABLE_CELL_SPACING)
    .width(Length::Fill)
    .on_press(Message::BeepTestEditSelectLevel(zero_based))
    .into()
}

/// A tappable data cell in the editor's table. Highlighted blue when
/// its column is selected; light-gray otherwise.
fn editor_value_cell<'a>(
    value: String,
    zero_based: usize,
    is_selected: bool,
) -> Element<'a, Message> {
    let inner = text(value)
        .size(SMALL_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    let style = if is_selected {
        blue_button
    } else {
        light_gray_button
    };
    button(
        container(inner)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .style(style)
    .padding(EDIT_TABLE_CELL_SPACING)
    .width(Length::Fill)
    .on_press(Message::BeepTestEditSelectLevel(zero_based))
    .into()
}

/// The `[+NEW]` header at the end of the last band. Appends a new
/// level when tapped.
fn editor_new_cell<'a>() -> Element<'a, Message> {
    let inner = text(fl!("beep-test-edit-new"))
        .size(SMALL_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    button(
        container(inner)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .style(green_button)
    .padding(EDIT_TABLE_CELL_SPACING)
    .width(Length::Fill)
    .on_press(Message::BeepTestEditAddLevel)
    .into()
}

/// Empty filler cell — keeps column widths consistent when a band is
/// partially filled or a column's count is shorter than the band's
/// tallest column.
fn filler_cell<'a>() -> Element<'a, Message> {
    Space::with_width(Length::Fill).into()
}

/// Build the per-level edit panel: the `Selected: Level N` label, the
/// Time and Count rows (each with current value and `[-]` `[+]`), and
/// the `REMOVE LEVEL` button.
fn build_edit_panel(levels: &[Level], selected: usize) -> Element<'_, Message> {
    // Safe to index because the caller already clamped `selected` to be
    // in range. If the list is empty we fall through to a placeholder.
    let Some(level) = levels.get(selected) else {
        return container(
            text(fl!("beep-test-edit-new"))
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        )
        .style(light_gray_container)
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    };

    let selected_label = text(fl!(
        "beep-test-edit-selected",
        level = (selected + 1).to_string()
    ))
    .size(MEDIUM_TEXT)
    .align_x(Horizontal::Center)
    .width(Length::Fill);

    // Time row: label, value, [-] [+]
    let duration_secs = level.duration.as_secs();
    let time_dec_disabled = duration_secs <= 1;
    let time_dec = {
        let mut b = make_smaller_button("-").style(if time_dec_disabled {
            gray_button
        } else {
            blue_button
        });
        if !time_dec_disabled {
            b = b.on_press(Message::BeepTestEditDurationDec);
        }
        b
    };
    let time_inc = make_smaller_button("+")
        .style(blue_button)
        .on_press(Message::BeepTestEditDurationInc);
    let time_row = row![
        container(
            text(fl!("beep-test-edit-time"))
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Left)
                .width(Length::Fill),
        )
        .padding(PADDING)
        .width(Length::FillPortion(2))
        .height(Length::Fixed(XS_BUTTON_SIZE)),
        container(
            text(duration_secs.to_string())
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        )
        .style(light_gray_container)
        .padding(PADDING)
        .width(Length::FillPortion(2))
        .height(Length::Fixed(XS_BUTTON_SIZE)),
        container(time_dec)
            .width(Length::FillPortion(1))
            .height(Length::Fixed(XS_BUTTON_SIZE)),
        container(time_inc)
            .width(Length::FillPortion(1))
            .height(Length::Fixed(XS_BUTTON_SIZE)),
    ]
    .spacing(SPACING);

    // Count row: label, value, [-] [+]
    let count_dec_disabled = level.count <= 1;
    let count_dec = {
        let mut b = make_smaller_button("-").style(if count_dec_disabled {
            gray_button
        } else {
            blue_button
        });
        if !count_dec_disabled {
            b = b.on_press(Message::BeepTestEditCountDec);
        }
        b
    };
    let count_inc = make_smaller_button("+")
        .style(blue_button)
        .on_press(Message::BeepTestEditCountInc);
    let count_row = row![
        container(
            text(fl!("beep-test-edit-count"))
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Left)
                .width(Length::Fill),
        )
        .padding(PADDING)
        .width(Length::FillPortion(2))
        .height(Length::Fixed(XS_BUTTON_SIZE)),
        container(
            text(level.count.to_string())
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        )
        .style(light_gray_container)
        .padding(PADDING)
        .width(Length::FillPortion(2))
        .height(Length::Fixed(XS_BUTTON_SIZE)),
        container(count_dec)
            .width(Length::FillPortion(1))
            .height(Length::Fixed(XS_BUTTON_SIZE)),
        container(count_inc)
            .width(Length::FillPortion(1))
            .height(Length::Fixed(XS_BUTTON_SIZE)),
    ]
    .spacing(SPACING);

    // Remove row: a single wide REMOVE LEVEL button, disabled when
    // only one level remains.
    let remove_disabled = levels.len() <= 1;
    let remove_button = if remove_disabled {
        make_smaller_button(fl!("beep-test-edit-remove")).style(gray_button)
    } else {
        make_smaller_button(fl!("beep-test-edit-remove"))
            .style(red_button)
            .on_press(Message::BeepTestEditRemoveLevel)
    };

    column![selected_label, time_row, count_row, remove_button,]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}

/// Stub for the App Mode sub-page. Implemented in Task 7.
pub(in super::super) fn build_beep_test_app_mode_page<'a>() -> Element<'a, Message> {
    placeholder_page("TODO: App Mode (Task 7)")
}

/// Shared shell for the sub-page stubs: shows the placeholder text and a
/// Back button so the operator is never trapped.
fn placeholder_page(label: &str) -> Element<'_, Message> {
    let back_button = make_button(fl!("back"))
        .style(red_button)
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::Main));

    column![
        text(label).size(MEDIUM_TEXT),
        row![back_button, horizontal_space(), horizontal_space()].spacing(SPACING),
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .height(Length::Fill)
    .into()
}
