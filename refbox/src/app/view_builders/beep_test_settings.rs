//! View_builders for the BeepTest Settings sub-pages.
//!
//! Reachable when `app_state == AppState::BeepTestSettings(_)`. Each
//! function builds one sub-page: the 2x2 landing, the Sound Settings
//! page, the Edit Levels page, and the Language picker.
//!
//! All sub-pages follow the refbox UI standard: a `make_game_time_button`
//! anchors the top, controls below it use `make_value_button`, and editor
//! sub-pages end in a Cancel / Apply footer that disables Apply when the
//! staged edits match the live config. This parallels `configuration.rs`'s
//! `make_user_config_page`, `make_sound_config_page`, and
//! `make_app_config_page` patterns.
//!
//! App Mode is cycled directly on the landing tile (no separate sub-page);
//! a RESTART TO APPLY button appears on the landing's bottom row when the
//! staged mode differs from the live mode.

use super::*;
use crate::{
    config::{Config, Level},
    portal_manager::PortalIndicatorState,
};
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, Space, button, column, container, horizontal_space, row, text},
};
use uwh_common::game_snapshot::GameSnapshot;

/// 2x2 landing page for the BeepTest Settings hierarchy.
///
/// Layout (top to bottom):
/// - Game time banner (refbox standard)
/// - Row 1: [SOUND SETTINGS] [EDIT LEVELS]
/// - Row 2: [APP MODE = <staged>] [LANGUAGE]
/// - Filler rows
/// - Bottom row: [BACK]   [horizontal_space]   [RESTART TO APPLY (when staged mode != live)]
///
/// The APP MODE tile cycles the staged mode in place (no navigation). When
/// the staged mode differs from the live mode, a green RESTART TO APPLY
/// button appears at the right end of the bottom row; pressing it commits
/// the mode change and restarts the app.
///
/// `BACK` discards any staged mode change and returns to the BeepTest main
/// view, matching the standard set by `make_user_config_page` in
/// `configuration.rs`.
pub(in super::super) fn build_beep_test_settings_landing<'a>(
    snapshot: &GameSnapshot,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
    config: &Config,
    staged_mode: Mode,
) -> Element<'a, Message> {
    let sound_button = make_button(fl!("sound-settings"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenSound);

    let edit_levels_button = make_button(fl!("beep-test-edit-levels"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenLevels);

    // APP MODE cycles in place — no sub-page navigation. Uses the same
    // CycleParameter(Mode) handler the hockey-mode App config page uses.
    let app_mode_button = make_value_button(
        fl!("app-mode"),
        staged_mode.to_string(),
        (false, true),
        Some(Message::CycleParameter(CyclingParameter::Mode)),
    );

    let language_button = make_button(fl!("language"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenLanguage);

    let row_top = row![sound_button, edit_levels_button]
        .spacing(SPACING)
        .height(Length::Fill);

    let row_bottom = row![app_mode_button, language_button]
        .spacing(SPACING)
        .height(Length::Fill);

    let back_button = make_button(fl!("back"))
        .style(red_button)
        .on_press(Message::BeepTestCloseSettings);

    // Bottom row keeps a stable 3-cell layout. When the staged mode differs
    // from the live mode, the right cell becomes a green RESTART TO APPLY
    // button; otherwise it stays a filler so the BACK button doesn't shift.
    let bottom_row: Element<'a, Message> = if staged_mode != config.mode {
        let restart_button = make_button(fl!("restart-to-apply"))
            .style(green_button)
            .on_press(Message::BeepTestRestartToApply);
        row![back_button, horizontal_space(), restart_button]
            .spacing(SPACING)
            .into()
    } else {
        row![back_button, horizontal_space(), horizontal_space()]
            .spacing(SPACING)
            .into()
    };

    // 2 tile rows + 3 spacer rows = 5 Fill shares. The back row sits at the
    // very bottom (where the timeout ribbon would be in Hockey/Rugby), with
    // an extra Fill spacer replacing its old slot — this keeps each Fill
    // share close to button height (no inner-row whitespace) without leaving
    // dead space below the back row.
    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator
        ),
        row_top,
        row_bottom,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        bottom_row,
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Sound Settings sub-page for the BeepTest hierarchy.
///
/// Mirrors `make_sound_config_page` in `configuration.rs`: game time
/// banner at top, rows of `make_value_button` controls, and a
/// Cancel / Apply footer at the bottom. Apply disables when staged sound
/// settings match the live config.
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
    snapshot: &GameSnapshot,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
    config: &Config,
    sound: &SoundSettings,
) -> Element<'a, Message> {
    let sound_enabled = sound.sound_enabled;
    let whistle_vol_enabled = sound_enabled && sound.whistle_enabled;
    let has_changes = config.sound != *sound;

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

    let row_top = row![sound_enabled_btn, above_water_vol_btn, whistle_enabled_btn]
        .spacing(SPACING)
        .height(Length::Fill);

    let row_bottom = row![buzzer_sound_btn, below_water_vol_btn, whistle_vol_btn]
        .spacing(SPACING)
        .height(Length::Fill);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator
        ),
        row_top,
        row_bottom,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        make_beep_test_cancel_apply_footer(
            Message::BeepTestSoundSettingsCancel,
            Message::BeepTestSoundSettingsSave,
            has_changes,
        ),
    ]
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
/// Standard refbox column layout: game time banner at top, the
/// transposed levels table + per-level edit panel in the middle (sized
/// proportionally), and a Cancel / Apply footer at the bottom. Apply
/// disables when the staged levels match the live config.
///
/// Top middle: the same transposed level table from the main view, plus
/// an extra `[+NEW]` header at the end. Every header and every cell is
/// tappable: tapping any element in a column selects that level. The
/// selected column is highlighted with a distinct (blue) style to
/// distinguish it from the main view's yellow "active lap" highlight.
///
/// Bottom middle: a per-level edit panel showing the selected level's
/// duration and count, each with `[-]` `[+]` buttons, and a `REMOVE
/// LEVEL` button (disabled when only one level remains).
pub(in super::super) fn build_beep_test_edit_levels_page<'a>(
    snapshot: &GameSnapshot,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
    config: &Config,
    levels: &'a [Level],
    selected: usize,
) -> Element<'a, Message> {
    // Clamp the selected index defensively. The handlers in update()
    // already prevent out-of-range writes, but a render pass that
    // happens to see a stale snapshot (e.g. between Remove and the next
    // tick) should still produce a sane view.
    let selected = selected.min(levels.len().saturating_sub(1));

    let has_changes = config.beep_test.levels.as_slice() != levels;

    // ----- Transposed table with tappable headers + cells -----
    let table = build_editor_levels_table(levels, selected);

    // ----- Per-level edit panel -----
    let edit_panel = build_edit_panel(levels, selected);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator
        ),
        container(table).width(Length::Fill).height(Length::Fill),
        container(edit_panel)
            .width(Length::Fill)
            .height(Length::Fill),
        make_beep_test_cancel_apply_footer(
            Message::BeepTestEditLevelsCancel,
            Message::BeepTestEditLevelsSave,
            has_changes,
        ),
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

    for (band_idx, band_levels) in levels.chunks(EDIT_BAND_WIDTH).enumerate() {
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
        // Pad with filler cells on partially-populated bands so columns
        // stay aligned with full bands above.
        let cols_used_in_band = band_levels.len();
        for _ in cols_used_in_band..EDIT_BAND_WIDTH {
            header_row = header_row.push(filler_cell());
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
            for _ in cols_used_in_band..EDIT_BAND_WIDTH {
                cell_row = cell_row.push(filler_cell());
            }
            bands = bands.push(cell_row);
        }
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

/// Empty filler cell — keeps column widths consistent when a band is
/// partially filled or a column's count is shorter than the band's
/// tallest column.
fn filler_cell<'a>() -> Element<'a, Message> {
    Space::with_width(Length::Fill).into()
}

/// Build the per-level edit panel: a top management row with
/// `[+NEW]`, the `Selected: Level N` indicator, and `[REMOVE LEVEL]`,
/// followed by the Time and Count rows (each with current value and
/// `[-]` `[+]`).
fn build_edit_panel(levels: &[Level], selected: usize) -> Element<'_, Message> {
    // Safe to index because the caller already clamped `selected` to be
    // in range. If the list is empty we fall through to a placeholder
    // with just a `[+NEW]` button.
    let Some(level) = levels.get(selected) else {
        let add_button = make_smaller_button(fl!("beep-test-edit-new"))
            .style(green_button)
            .on_press(Message::BeepTestEditAddLevel);
        return container(add_button)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    // Top management row: [+NEW] | Selected: Level N | [REMOVE LEVEL]
    let add_button = make_smaller_button(fl!("beep-test-edit-new"))
        .style(green_button)
        .on_press(Message::BeepTestEditAddLevel);

    let remove_disabled = levels.len() <= 1;
    let remove_button = if remove_disabled {
        make_smaller_button(fl!("beep-test-edit-remove")).style(gray_button)
    } else {
        make_smaller_button(fl!("beep-test-edit-remove"))
            .style(red_button)
            .on_press(Message::BeepTestEditRemoveLevel)
    };

    let selected_label = container(
        text(fl!(
            "beep-test-edit-selected",
            level = (selected + 1).to_string()
        ))
        .size(MEDIUM_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill),
    )
    .style(light_gray_container)
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(XS_BUTTON_SIZE))
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let management_row = row![add_button, selected_label, remove_button].spacing(SPACING);

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

    column![management_row, time_row, count_row]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}

/// Language picker sub-page for the BeepTest hierarchy.
///
/// Mirrors `make_language_select_page` in `configuration.rs` (same 15
/// languages, same selected-state highlighting, same font/script
/// handling for CJK/Thai/Latin) but uses the BeepTest 7-row layout: game
/// time banner at top, four rows of language buttons, and the Cancel /
/// Apply footer at the very bottom. There is no timeout ribbon (BeepTest
/// has no concept of timeouts).
///
/// The selected language lives in `settings.pending_language` (seeded by
/// `BeepTestEditOpenLanguage`) and the original language lives in
/// `settings.original_language`. Apply enables when these differ. When
/// the font family changes between original and selected (Latin ↔ CJK ↔
/// Thai), the Apply button label and style reflect a restart-required
/// commit; otherwise it is a normal "Done" green button.
pub(in super::super) fn build_beep_test_language_picker<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let selected = settings.pending_language.unwrap_or(Language::English);
    let original = settings.original_language.unwrap_or(Language::English);
    let has_changes = settings.pending_language != settings.original_language;

    let cjk_font = iced_core::Font {
        family: iced_core::font::Family::Name("WenQuanYi Zen Hei"),
        weight: iced_core::font::Weight::Normal,
        stretch: iced_core::font::Stretch::Normal,
        style: iced_core::font::Style::Normal,
    };

    let thai_font = iced_core::Font {
        family: iced_core::font::Family::Name("Noto Sans Thai"),
        weight: iced_core::font::Weight::Normal,
        stretch: iced_core::font::Stretch::Normal,
        style: iced_core::font::Style::Normal,
    };

    let latin_font = iced_core::Font {
        family: iced_core::font::Family::Name("Roboto"),
        weight: iced_core::font::Weight::Medium,
        stretch: iced_core::font::Stretch::Normal,
        style: iced_core::font::Style::Normal,
    };

    // Font for Cancel/Done/Restart text so they render in the target
    // language's script regardless of the app's current default font.
    let selected_font: Option<iced_core::Font> = match selected {
        Language::Korean | Language::Japanese | Language::Mandarin => Some(cjk_font),
        Language::Thai => Some(thai_font),
        _ => Some(latin_font),
    };

    // A restart is needed when switching between Latin and CJK / Thai font families.
    let needs_restart = font_family_id(original) != font_family_id(selected);

    let lang_btn = |lang: Language,
                    label: &'static str,
                    font: Option<iced_core::Font>|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        let label_widget = {
            let t = centered_text(label);
            if let Some(f) = font { t.font(f) } else { t }
        };
        button(label_widget)
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .style(style)
            .width(Length::Fill)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Button variant for unverified translations: shows native name plus a
    // small "(UNVERIFIED)" note in that language's own script.
    let lang_btn_note = |lang: Language,
                         main: NameLines<&'static str>,
                         note: &'static str,
                         font: Option<iced_core::Font>|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_lang_button_with_note(main, note, font)
            .style(style)
            .width(Length::Fill)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Cancel / Done(Restart) footer. The labels use the selected language's
    // own translation of CANCEL / DONE / RESTART (so a user mid-switch can
    // read them) and the appropriate font for the selected script.
    let make_label = |content: &'static str, font: Option<iced_core::Font>| {
        let t = text(content)
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .width(Length::Shrink);
        let t: iced::widget::Text<'a, _, _> = if let Some(f) = font { t.font(f) } else { t };
        container(t).center(Length::Fill)
    };

    let cancel_btn = button(make_label(selected.cancel_text(), selected_font))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::BeepTestLanguageCancel);

    let confirm_msg = has_changes.then_some(Message::BeepTestLanguageApply);
    let confirm_btn: Element<'a, Message> = if needs_restart {
        button(make_label(selected.restart_text(), selected_font))
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .style(blue_button)
            .width(Length::Fill)
            .on_press_maybe(confirm_msg)
            .into()
    } else {
        button(make_label(selected.done_text(), selected_font))
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(confirm_msg)
            .into()
    };

    // Languages sorted alphabetically by romanized native name (same order
    // as `make_language_select_page` in `configuration.rs`).
    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator
        ),
        row![
            lang_btn_note(
                Language::Indonesian,
                NameLines::OneLineSmall("BAHASA INDONESIA"),
                "(BELUM DIVERIFIKASI)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Malay,
                NameLines::OneLineSmall("BAHASA MELAYU"),
                "(BELUM DISAHKAN)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::German,
                NameLines::OneLine("DEUTSCH"),
                "(NICHT VERIFIZIERT)",
                Some(latin_font),
            ),
            lang_btn(Language::English, "ENGLISH", None),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            lang_btn(Language::Spanish, "ESPAÑOL", None),
            lang_btn_note(
                Language::Tagalog,
                NameLines::OneLine("FILIPINO"),
                "(HINDI PA NA-VERIFY)",
                Some(latin_font),
            ),
            lang_btn(Language::French, "FRANÇAIS", None),
            lang_btn_note(
                Language::Korean,
                NameLines::OneLine("한국어"),
                "(검증되지 않음)",
                Some(cjk_font),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            lang_btn_note(
                Language::Italian,
                NameLines::OneLine("ITALIANO"),
                "(NON VERIFICATO)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Dutch,
                NameLines::OneLine("NEDERLANDS"),
                "(NIET GEVERIFIEERD)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Japanese,
                NameLines::OneLine("日本語"),
                "(未検証)",
                Some(cjk_font),
            ),
            lang_btn_note(
                Language::Portuguese,
                NameLines::OneLine("PORTUGUÊS"),
                "(NÃO VERIFICADO)",
                Some(latin_font),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            lang_btn_note(
                Language::Thai,
                NameLines::OneLine("ภาษาไทย"),
                "(ยังไม่ได้ตรวจสอบ)",
                Some(thai_font),
            ),
            lang_btn_note(
                Language::Turkish,
                NameLines::OneLine("TÜRKÇE"),
                "(DOĞRULANMAMIŞ)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Mandarin,
                NameLines::OneLine("中文"),
                "(未验证)",
                Some(cjk_font),
            ),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![cancel_btn, horizontal_space(), confirm_btn].spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Same as `font_family_id` in `configuration.rs` / `mod.rs`. Kept inline
/// here to avoid widening the visibility of either definition just to share
/// a 5-line mapping; the alternative is an unnecessary cross-module export.
fn font_family_id(lang: Language) -> u8 {
    match lang {
        Language::Korean | Language::Japanese | Language::Mandarin => 1,
        Language::Thai => 2,
        _ => 0,
    }
}

/// Cancel / Apply footer for BeepTest Settings editor sub-pages.
///
/// Mirrors `make_cancel_apply_footer` in `configuration.rs`: red Cancel
/// on the left, green Apply on the right (using the existing `apply`
/// translation key — the same label the game-mode editor sub-pages
/// show). Apply omits its `on_press` when `has_changes` is false, which
/// produces the disabled / grayed-out appearance per refbox convention.
fn make_beep_test_cancel_apply_footer<'a>(
    cancel_message: Message,
    apply_message: Message,
    has_changes: bool,
) -> Element<'a, Message> {
    let cancel = make_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(cancel_message);

    let apply = make_button(fl!("apply"))
        .style(green_button)
        .width(Length::Fill);
    let apply = if has_changes {
        apply.on_press(apply_message)
    } else {
        apply
    };

    row![cancel, horizontal_space(), apply]
        .spacing(SPACING)
        .into()
}
