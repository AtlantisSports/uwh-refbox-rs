//! View_builders for the BeepTest Settings sub-pages.
//!
//! Reachable when `app_state == AppState::BeepTestSettings(_)`. Each
//! function builds one sub-page: the 2x2 landing, the Sound Settings
//! page, the Edit Levels page, and the Language picker.
//!
//! Most sub-pages use `make_value_button` for controls -- Edit Levels is the
//! exception, built on read-only tiles with their own `[-]` / `[+]` beside
//! them. Editor
//! sub-pages end in a Cancel / Apply footer that disables Apply when the
//! staged edits match the live config. This parallels `configuration.rs`'s
//! `make_user_config_page`, `make_sound_config_page`, and
//! `make_app_config_page` patterns.
//!
//! App Mode is cycled directly on the landing tile (no separate sub-page);
//! a RESTART TO APPLY button appears on the landing's bottom row when the
//! staged mode differs from the live mode.

use super::beep_test::beep_test_value_tile;
use super::fit_text::fit_text;
use super::*;
use crate::config::{BeepTestPreset, Config, Level};
use crate::sim_frame::{FrontDisplayLayout, effective_beep_layout};
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{
        Column, Image, Row, Space, button, column, container, horizontal_space, image, row, text,
    },
};

/// Landing page for the BeepTest Settings hierarchy.
///
/// Grid (top to bottom):
/// - Row 1: [APP MODE = <staged>] [EDIT LEVELS]
/// - Row 2: [SOUND SETTINGS]      [DISPLAY LAYOUT]
/// - Rows 3-4: left column [LANGUAGE] over a blank cell; right column a
///   beep-test PREVIEW spanning both rows.
/// - Bottom row: [BACK]   [horizontal_space]   [RESTART TO APPLY (when staged
///   mode != live mode and no test has run)] — unchanged.
///
/// DISPLAY LAYOUT cycles the in-memory beep-test layout live (no Apply). It is
/// grayed and forced to Default when a real LED panel is connected (the panel
/// only renders Default) or once a beep test has run. APP MODE, EDIT LEVELS,
/// and LANGUAGE are gated on `!has_run`; SOUND SETTINGS stays live.
pub(in super::super) fn build_beep_test_settings_landing<'a>(
    config: &Config,
    staged_mode: Mode,
    has_run: bool,
    beep_test_layout: FrontDisplayLayout,
    has_led_panel: bool,
) -> Element<'a, Message> {
    let sound_button = make_tile_button(fl!("sound-settings"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenSound);

    let edit_levels_button = if has_run {
        make_tile_button(fl!("beep-test-edit-levels")).style(gray_button)
    } else {
        make_tile_button(fl!("beep-test-edit-levels"))
            .style(light_gray_button)
            .on_press(Message::BeepTestEditOpenLevels)
    };

    let app_mode_button = make_value_button(
        fl!("app-mode"),
        staged_mode.to_string(),
        (false, true),
        if has_run {
            None
        } else {
            Some(Message::CycleParameter(CyclingParameter::Mode))
        },
    );

    let language_button = if has_run {
        make_tile_button(fl!("language")).style(gray_button)
    } else {
        make_tile_button(fl!("language"))
            .style(light_gray_button)
            .on_press(Message::BeepTestEditOpenLanguage)
    };

    // DISPLAY LAYOUT — cycles the in-memory beep-test layout (live-apply, not
    // persisted). Grayed + forced to Default with a panel connected or after a run.
    let effective_layout = effective_beep_layout(has_led_panel, beep_test_layout);
    let layout_label = match effective_layout {
        FrontDisplayLayout::Default => fl!("layout-default"),
        FrontDisplayLayout::Classic => fl!("layout-classic"),
        FrontDisplayLayout::BigTime => fl!("layout-big-time"),
        FrontDisplayLayout::Corners => fl!("layout-corners"),
        FrontDisplayLayout::ScoresOnly => fl!("layout-scores-only"),
    };
    let display_layout_button = make_value_button(
        fl!("front-display-layout"),
        layout_label,
        (false, true),
        if has_led_panel || has_run {
            None
        } else {
            Some(Message::BeepTestCycleDisplayLayout)
        },
    );

    // Static preview of the effective layout's beep-test appearance (white-on-left).
    let preview = container(
        Image::new(beep_test_layout_preview_handle(effective_layout))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let row1 = row![app_mode_button, edit_levels_button]
        .spacing(SPACING)
        .height(Length::Fill);
    let row2 = row![sound_button, display_layout_button]
        .spacing(SPACING)
        .height(Length::Fill);

    // Rows 3-4: LANGUAGE over a blank cell on the left; preview on the right
    // spanning both rows. FillPortion(2) gives this band the height of two tile
    // rows, so the preview reads as a 2-row-tall cell.
    let lower_left = column![
        row![language_button].spacing(SPACING).height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    let rows_34 = row![lower_left, preview]
        .spacing(SPACING)
        .height(Length::FillPortion(2));

    let back_button = make_chrome_button(fl!("back"))
        .style(red_button)
        .on_press(Message::BeepTestCloseSettings);

    // Bottom row unchanged: BACK on the left, and a blue RESTART TO APPLY at the
    // right when the staged App Mode differs from the live mode and no test has
    // run yet; otherwise a filler keeps BACK from shifting.
    let bottom_row: Element<'a, Message> = if staged_mode != config.mode && !has_run {
        let restart_button = make_chrome_button(fl!("restart-to-apply"))
            .style(blue_button)
            .on_press(Message::BeepTestRestartToApply);
        row![back_button, horizontal_space(), restart_button]
            .spacing(SPACING)
            .into()
    } else {
        row![back_button, horizontal_space(), horizontal_space()]
            .spacing(SPACING)
            .into()
    };

    // 2 single tile rows (1 share each) + the preview band (2 shares) + 2 spacer
    // rows (1 share each) = 6 Fill shares. That keeps the tiles near button
    // height, but it does NOT match the sibling config pages: they run 4 shares
    // under a game-time banner and land taller (~118px vs ~94px at the default
    // window). The
    // tiles stretch to fill their share; it is this share arithmetic, not the
    // tiles, that keeps them near button height — add or remove a Fill row here
    // and every tile resizes. The spacers absorb the gap above the footer; the
    // footer stays pinned at the bottom.
    column![
        row1,
        row2,
        rows_34,
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
/// Mirrors `make_sound_config_page` in `configuration.rs`: rows of
/// `make_value_button` controls and a Cancel / Apply footer at the bottom.
/// Apply disables when staged sound settings match the live config.
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

    // BUZZER SOUND — gated by SOUND ENABLED. Opens the full-page picker.
    let buzzer_sound_btn = make_value_button(
        fl!("buzzer-sound"),
        sound.buzzer_sound.to_string().to_uppercase(),
        (false, true),
        if sound_enabled {
            Some(Message::BeepTestEditOpenBuzzer)
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
        row_top,
        row_bottom,
        row![horizontal_space()].height(Length::Fill),
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

/// Maximum number of levels a schedule may contain. The table reserves exactly
/// this many columns; the court-length preset strip is a separate column beside
/// it, taking a third of whatever width the page has (see `body` in
/// `build_beep_test_edit_levels_page`), and does not grow or shrink with this
/// cap. This value matches the Full presets' level
/// count (10) — see `BeepTestPreset::FULL_LAP_COUNTS` in `config.rs`.
pub(in super::super) const MAX_LEVELS: usize = 10;

/// Padding inside each cell of the editor's transposed table, between the cell
/// edge and its number. Matches the main view's TABLE_CELL_SPACING so the two
/// tables read alike. This is NOT the spacing between cells: that is the
/// standard `SPACING`, which is what makes two stacked cells plus their gap
/// come to exactly one `MIN_BUTTON_SIZE` row.
const EDIT_TABLE_CELL_SPACING: f32 = 2.0;

/// Most laps one level may be given from this screen. The COUNT `+` disables at
/// this value and `Message::BeepTestEditCountInc` enforces the same cap, both
/// reading this constant so the two cannot drift apart. The height budget the
/// page is guarded against depends on it: one more lap is one more table layer.
pub(in super::super) const MAX_LAPS_PER_LEVEL: u8 = 5;

/// Cell layers the table reserves: one header layer plus one lap layer per lap
/// at the cap.
///
/// Fixed rather than following the tallest staged level. The table therefore
/// always occupies the same number of page rows, which is what lets the preset
/// strip beside it share one row grid — a table that shrank with the lap count
/// would put everything below it on a half-row boundary half the time. The cost
/// is that the bottom lap layer sits empty whenever no level uses every lap.
const TABLE_LAYER_SLOTS: usize = MAX_LAPS_PER_LEVEL as usize + 1;

/// Cell layers stacked inside one page row. Two of them plus the standard gap
/// between them make a table row exactly as tall as a button row.
const LAYERS_PER_ROW: usize = 2;

/// Page rows the levels table occupies.
///
/// `MAX_LAPS_PER_LEVEL` has to be odd for this to divide evenly — an even lap
/// cap makes `TABLE_LAYER_SLOTS` odd and leaves the table half a row tall, which
/// no amount of alignment can rescue. Pinned by the second `const _: () =
/// assert!` below.
const TABLE_ROWS: usize = TABLE_LAYER_SLOTS / LAYERS_PER_ROW;

/// Page rows the parameter editor occupies: LEVEL, TIME and COUNT.
const EDITOR_ROWS: usize = 3;

/// Page rows the preset strip occupies: the PRESET heading over one row per
/// court length.
const PRESET_STRIP_ROWS: usize = PRESET_ROWS.len() + 1;

// The page's alignment invariant, checked at compile time.
//
// Both columns divide the same body height by their own number of filling rows,
// so if the two counts differ the rows come out different heights and nothing
// lines up. That is not something the renderer can report — it just looks
// subtly wrong — so a build error is the right place to catch it: add a court
// length without a row for it on the other side and the crate stops compiling.
const _: () = assert!(TABLE_ROWS + EDITOR_ROWS == PRESET_STRIP_ROWS);

// TABLE_ROWS is an integer division, so an odd number of layer slots would
// silently lose one. A lap cap of 4 gives exactly that: 5 slots, two rows, and
// the last lap invisible. The cap has to stay odd.
const _: () = assert!(TABLE_ROWS * LAYERS_PER_ROW == TABLE_LAYER_SLOTS);

/// Edit Levels sub-page.
///
/// Two columns over a Cancel / Apply footer, with Apply disabled while the
/// staged levels match the live config.
///
/// Left column, top: the transposed level table from the main view. Every
/// header and every cell is tappable, and tapping any element in a column
/// selects that level. The level headers are green and the selected one yellow,
/// matching the main beep-test screen's table; the lap times below stay light
/// gray, with the selected column's blue. The table reserves `MAX_LEVELS`
/// columns (10, matching the Full presets).
///
/// Left column, beneath it: the per-level edit panel — one row each for LEVEL,
/// TIME and COUNT, each a read-only tile with `[-]` and `[+]` beside it. See
/// `build_parameter_rows`.
///
/// Right column: the ten court-length preset buttons, running down the full
/// height of the page beside both of the above. That is what lets them keep a
/// full row height: ten buttons cannot fit beside the table alone. The preset
/// matching the staged levels renders highlighted; see `build_preset_panel`.
///
/// Every row on the page fills, so they all come out at whatever height the
/// screen divides into, and two stacked table cells plus their gap come to the
/// same height as one of them -- so the table, the editor and the preset strip
/// all share one rhythm.
///
/// What the renderer cannot enforce is that the two columns divide that height
/// into the *same number* of rows; the `const _: () = assert!` pair above pins
/// that at compile time, and
/// `a_table_cell_stays_tall_enough_to_read_at_the_default_screen_height` pins
/// the size those rows actually come out at on the default screen.
pub(in super::super) fn build_beep_test_edit_levels_page<'a>(
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

    // ----- The page's row grid -----
    //
    // Refbox's standard body layout: the page is a whole number of equal filling
    // rows over a fixed-height footer, and every cell in a row takes an equal
    // share of it (see `make_tile_button`). Nothing here computes a height. Rows
    // are whatever the screen allows, which is what makes the page fit a Pi panel
    // of any height without arithmetic, and what makes the two columns line up:
    //
    //   left column  = TABLE_ROWS + EDITOR_ROWS  filling rows
    //   right column = PRESET_STRIP_ROWS         filling rows
    //
    // Those two totals must be equal, or the columns divide the same height into
    // different numbers of rows and nothing lines up. That is the page's one
    // invariant, and the first `const _: () = assert!` above pins it.
    let mut schedule_column = Column::new().spacing(SPACING).height(Length::Fill);
    for table_row in build_editor_levels_table(levels, selected) {
        schedule_column = schedule_column.push(table_row);
    }
    for parameter_row in build_parameter_rows(levels, selected) {
        schedule_column = schedule_column.push(parameter_row);
    }

    // The strip runs beside the table *and* the parameters. Beside the table
    // alone it could not hold ten preset buttons at a full row height.
    let body = row![
        container(schedule_column)
            .width(Length::FillPortion(10))
            .height(Length::Fill),
        container(build_preset_panel(levels))
            .width(Length::FillPortion(5))
            .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    column![
        body,
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

/// The preset rows as they appear in the strip, longest pool first: the referee
/// (26-lap) schedule on the left of each row, the full (37-lap) one on the
/// right.
///
/// 25yd sits between 23m and 22m because 25 yards is 22.86m — the order is by
/// real pool length, not by the number printed on the label.
///
/// Written out rather than derived from `BeepTestPreset::ALL` because this is a
/// display decision: which schedule shares a row with which, and in what order.
/// `preset_rows_cover_every_preset_exactly_once` checks the list against `ALL`,
/// so a new court length cannot be added to the config and quietly go missing
/// from this screen.
const PRESET_ROWS: [(BeepTestPreset, BeepTestPreset); 5] = [
    (BeepTestPreset::Ref25, BeepTestPreset::Full25),
    (BeepTestPreset::Ref23, BeepTestPreset::Full23),
    (BeepTestPreset::Ref25Yd, BeepTestPreset::Full25Yd),
    (BeepTestPreset::Ref22, BeepTestPreset::Full22),
    (BeepTestPreset::Ref21, BeepTestPreset::Full21),
];

/// The court-length preset buttons: a PRESET heading over a 2-column grid in the
/// strip down the right of the page, one row per pool length, ordered by
/// `PRESET_ROWS`.
///
/// Each button is one `MIN_BUTTON_SIZE` tall, the same as an editor row and the
/// same as two stacked table cells plus the gap between them, so the strip and
/// the table sit on one shared row rhythm: preset row 1 spans table layers 1
/// and 2, preset row 2 spans layers 3 and 4, and so on.
///
/// The preset whose schedule matches the staged levels is highlighted with the
/// same blue the selected-level column uses, so the screen reads back which
/// schedule is loaded. Hand-editing any time or lap count makes the staged
/// levels stop matching, and the highlight drops on the next render — that is
/// the whole mechanism, no separate "edited" flag is needed.
fn build_preset_panel(levels: &[Level]) -> Element<'_, Message> {
    let active = BeepTestPreset::detect_levels(levels);

    let preset_button = move |preset: BeepTestPreset| {
        let style = if Some(preset) == active {
            blue_selected_button
        } else {
            light_gray_button
        };
        let label = if preset.is_ref() {
            format!(
                "{} {}",
                fl!("beep-test-preset-ref"),
                preset.distance_label()
            )
        } else {
            preset.distance_label().to_string()
        };
        // `fit_text` with `no_wrap`, not `centered_text`: a preset button is one
        // page row tall, so there is no room for the second line plain text wraps
        // to -- and these labels are long. "REF 25YD" all but fills the button in
        // English, and the translations run past it ("WASIT 25YD", "SR 25YD" and
        // the Thai and Korean forms). Shrinking is legible; a clipped second line
        // is not, and it hides which pool the button is for.
        //
        // Not `make_tile_button`, which is otherwise exactly this: it cannot say
        // `no_wrap`, and on this row grid that is the property that matters.
        // `centered_text` also pairs `align_y(Center)` with `height(Fill)`, the
        // stale-anchor combination `portal_detail::row_text_centered` warns about.
        button(fit_text(label).no_wrap())
            .style(style)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::BeepTestEditSelectPreset(preset))
    };

    // The heading is one of the strip's rows, not decoration above them, so the
    // buttons below it land on the page's row grid. Plain text rather than a
    // tile, so it reads as a title and not as another control; `no_wrap`, so a
    // long translation shrinks instead of breaking onto a second line and
    // overflowing its row.
    let mut strip = Column::new()
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            container(
                fit_text(fl!("beep-test-preset-heading"))
                    .size(MEDIUM_TEXT)
                    .no_wrap(),
            )
            .width(Length::Fill)
            .height(Length::Fill),
        );

    for (ref_preset, full_preset) in PRESET_ROWS {
        strip = strip.push(
            row![preset_button(ref_preset), preset_button(full_preset)]
                .spacing(SPACING)
                .height(Length::Fill),
        );
    }
    strip.into()
}

/// Build the editor's transposed levels table, as the `TABLE_ROWS` page rows it
/// occupies.
///
/// Returns the rows rather than one element so the caller can push them as
/// siblings of the parameter rows: both columns of the page then hold the same
/// number of filling children with the same number of gaps between them, which
/// is what makes every row boundary shared. Wrapping the table in a single
/// element instead would give the left column four children to the strip's six,
/// and the two would divide the same height differently.
///
/// Mirrors the main view's table, but every header and value cell is a button
/// that fires `Message::BeepTestEditSelectLevel(i)` and the selected column uses
/// a blue highlight. Levels are added and removed from the LEVEL row of
/// `build_parameter_rows`, not from anything in this table.
fn build_editor_levels_table(levels: &[Level], selected: usize) -> Vec<Element<'_, Message>> {
    // Every layer the table reserves, header first, as a flat list. Each is a
    // filling row of cells, so two of them stacked in a page row split it evenly.
    let mut layers: Vec<Element<'_, Message>> = Vec::with_capacity(TABLE_LAYER_SLOTS);

    let mut header = Row::new().spacing(SPACING).height(Length::Fill);
    for col_idx in 0..levels.len() {
        header = header.push(editor_header_cell(
            (col_idx + 1).to_string(),
            col_idx,
            col_idx == selected,
        ));
    }
    for _ in levels.len()..MAX_LEVELS {
        header = header.push(filler_cell());
    }
    layers.push(header.into());

    // One layer per reserved lap slot, not per lap actually staged: the table
    // keeps its height so the preset strip can keep its row grid. A level with
    // fewer laps draws filler in the slots it does not reach.
    //
    // Laps beyond the cap are not drawn. The screen cannot stage them (COUNT's
    // `+` stops at the cap and the handler enforces it), but `Level::migrate`
    // validates nothing, so a hand-edited config file still can — and those laps
    // are then invisible here. Clamping on load is the proper fix and belongs in
    // config validation; before this page reserved a fixed height, such a config
    // pushed the footer off the screen instead, which was worse.
    for lap in 0..usize::from(MAX_LAPS_PER_LEVEL) {
        let mut cells = Row::new().spacing(SPACING).height(Length::Fill);
        for (col_idx, level) in levels.iter().enumerate() {
            if lap < usize::from(level.count) {
                cells = cells.push(editor_value_cell(
                    level.duration.as_secs().to_string(),
                    col_idx,
                    col_idx == selected,
                ));
            } else {
                cells = cells.push(filler_cell());
            }
        }
        for _ in levels.len()..MAX_LEVELS {
            cells = cells.push(filler_cell());
        }
        layers.push(cells.into());
    }

    // Group the layers into page rows, LAYERS_PER_ROW at a time.
    let mut layers = layers.into_iter();
    let mut rows = Vec::with_capacity(TABLE_ROWS);
    for _ in 0..TABLE_ROWS {
        let mut page_row = Column::new().spacing(SPACING).height(Length::Fill);
        for _ in 0..LAYERS_PER_ROW {
            if let Some(layer) = layers.next() {
                page_row = page_row.push(layer);
            }
        }
        rows.push(page_row.into());
    }
    rows
}

/// A tappable column-header cell showing a level number. Green, turning yellow
/// for the level being edited. Firing `BeepTestEditSelectLevel(zero_based)`
/// selects the column.
///
/// Green rather than the light gray the value cells use, because a row of level
/// numbers in the same colour as the lap times beneath them is hard to pick out
/// as a header at a glance. This matches the main beep-test screen, whose level
/// headers are green (`green_container`) and whose live cell is yellow
/// (`yellow_container`), so the two tables read the same way.
///
/// Yellow means different things on the two screens — "the level you are
/// editing" here, "the lap running now" there — but they never appear together,
/// and both are the one thing on their table that the operator is looking for.
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
        yellow_button
    } else {
        green_button
    };
    button(
        container(inner)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .style(style)
    .padding(EDIT_TABLE_CELL_SPACING)
    .width(Length::Fill)
    .height(Length::Fill)
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
    .height(Length::Fill)
    .on_press(Message::BeepTestEditSelectLevel(zero_based))
    .into()
}

/// Empty filler cell — keeps column widths consistent when a band is
/// partially filled or a column's count is shorter than the band's
/// tallest column.
fn filler_cell<'a>() -> Element<'a, Message> {
    Space::new(Length::Fill, Length::Fill).into()
}

/// One `[-]` or `[+]` button. Grey and inert exactly when `message` is `None`,
/// so "unavailable" is a property of having nothing to do rather than something
/// each call site has to remember to style.
fn step_button<'a>(label: &'static str, message: Option<Message>) -> Element<'a, Message> {
    let mut b = make_tile_button(label).style(if message.is_some() {
        blue_button
    } else {
        gray_button
    });
    if let Some(message) = message {
        b = b.on_press(message);
    }
    b.into()
}

/// One row of the per-level editor: a read-only tile carrying the parameter's
/// name and its current value, then that parameter's `[-]` and `[+]`.
///
/// Every row is one `MIN_BUTTON_SIZE` tall, matching a preset button and
/// matching two stacked table cells plus the gap between them, so the whole page
/// sits on one row rhythm. The 2:1:1 width split gives the tile the width the
/// old label column had and leaves both buttons the width they already had.
fn edit_panel_row<'a>(
    label: String,
    value: String,
    dec: Element<'a, Message>,
    inc: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        beep_test_value_tile(label, value)
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        container(dec)
            .width(Length::FillPortion(1))
            .height(Length::Fill),
        container(inc)
            .width(Length::FillPortion(1))
            .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Build the parameter rows: `EDITOR_ROWS` identical page rows, one each for
/// LEVEL, TIME and COUNT.
///
/// LEVEL's `[-]` and `[+]` remove and add a level rather than stepping the
/// selection — which level is being edited is chosen by tapping its column in
/// the table above.
///
/// The readout still moves when they are pressed, which is worth knowing because
/// it makes the row read like navigation when it is not: `[+]` copies the level
/// being edited, inserts the copy directly after it and selects the copy, so the
/// tile goes from LEVEL 1 to LEVEL 2 while a level has in fact been added.
///
/// `[-]` deletes the level being edited. The selection then lands on the level
/// before it — except on the first level, where the index cannot go below zero
/// and the selection lands on what was the level after it instead.
///
/// Both buttons carry the same styling as TIME's and COUNT's, so nothing on this
/// row is coloured to mark it destructive. Nothing here reaches
/// the config until APPLY, and re-tapping a preset restores the whole schedule,
/// so a mis-tap costs an edit in progress rather than a saved setup.
///
/// A button is grey and inert exactly when its action is unavailable: REMOVE at
/// one level, ADD at `MAX_LEVELS`, TIME `-` at one second, COUNT `-` at one lap,
/// COUNT `+` at `MAX_LAPS_PER_LEVEL`.
fn build_parameter_rows(levels: &[Level], selected: usize) -> Vec<Element<'_, Message>> {
    let add = (levels.len() < MAX_LEVELS).then_some(Message::BeepTestEditAddLevel);
    let remove = (levels.len() > 1).then_some(Message::BeepTestEditRemoveLevel);

    // One source per value: the string on the tile and the rule that greys the
    // button beside it both read the same Option, so a display cannot drift away
    // from the guard that belongs to it.
    //
    // `selected` is already clamped by the caller, so `None` here means the
    // schedule is empty. The screen cannot reach that state (REMOVE is inert at
    // one level), but a render against a stale snapshot could: every value then
    // reads as a dash and only ADD stays live, rather than the rows vanishing.
    let level = levels.get(selected);
    let dash = || "-".to_string();
    let secs = level.map(|l| l.duration.as_secs());
    let laps = level.map(|l| l.count);

    vec![
        edit_panel_row(
            fl!("beep-test-top-level-label"),
            level.map_or_else(dash, |_| (selected + 1).to_string()),
            step_button("-", remove),
            step_button("+", add),
        ),
        edit_panel_row(
            fl!("beep-test-edit-time"),
            secs.map_or_else(dash, |s| s.to_string()),
            step_button(
                "-",
                secs.is_some_and(|s| s > 1)
                    .then_some(Message::BeepTestEditDurationDec),
            ),
            step_button("+", secs.map(|_| Message::BeepTestEditDurationInc)),
        ),
        edit_panel_row(
            fl!("beep-test-edit-count"),
            laps.map_or_else(dash, |c| c.to_string()),
            step_button(
                "-",
                laps.is_some_and(|c| c > 1)
                    .then_some(Message::BeepTestEditCountDec),
            ),
            step_button(
                "+",
                laps.is_some_and(|c| c < MAX_LAPS_PER_LEVEL)
                    .then_some(Message::BeepTestEditCountInc),
            ),
        ),
    ]
}

/// Buzzer picker sub-page for the BeepTest hierarchy.
///
/// Mirrors the BeepTest Language picker layout: 3 rows of 4 sound
/// buttons (from the shared `make_buzzer_grid_rows`, which the main Sound
/// settings picker also uses), three trailing filler rows for vertical
/// balance, and a Cancel | TEST | Apply footer.
/// There is no `make_game_time_button` header — BeepTest sub-pages have
/// no timeout ribbon.
///
/// `sound` is the staged `SoundSettings` from `edited_settings`
/// (seeded by `BeepTestEditOpenSound`). The selected sound is
/// `sound.buzzer_sound`. Apply enables when the staged sound differs
/// from the live `config.sound.buzzer_sound`.
pub(in super::super) fn build_beep_test_buzzer_picker<'a>(
    config: &Config,
    sound: &crate::sound_controller::SoundSettings,
) -> Element<'a, Message> {
    let selected = sound.buzzer_sound;
    let has_changes = config.sound.buzzer_sound != selected;

    // 12 sounds in 3 rows of 4, mirroring the Language picker's row structure.
    // Shared with the main Sound settings buzzer picker; only the message differs.
    let mut col = Column::new().spacing(SPACING).height(Length::Fill);
    for r in make_buzzer_grid_rows(selected, Message::BeepTestSelectBuzzer) {
        col = col.push(r);
    }

    // Three trailing filler rows for vertical balance. This page has no top
    // "next game" ribbon (unlike the main Sound buzzer picker), so the extra
    // filler keeps the footer from riding up under the sound grid.
    col = col
        .push(row![horizontal_space()].height(Length::Fill))
        .push(row![horizontal_space()].height(Length::Fill))
        .push(row![horizontal_space()].height(Length::Fill));

    // Footer: Cancel | TEST | Apply (Apply gated by has_changes).
    let cancel = make_chrome_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::BeepTestBuzzerCancel);
    let test = make_chrome_button(fl!("test"))
        .style(blue_button)
        .width(Length::Fill)
        .on_press(Message::BeepTestTestBuzzer);
    let apply = {
        let b = make_chrome_button(fl!("apply"))
            .style(green_button)
            .width(Length::Fill);
        if has_changes {
            b.on_press(Message::BeepTestBuzzerSave)
        } else {
            b
        }
    };

    col.push(row![cancel, test, apply].spacing(SPACING)).into()
}

/// Language picker sub-page for the BeepTest hierarchy.
///
/// Mirrors `make_language_select_page` in `configuration.rs` (same 15
/// languages, same selected-state highlighting, same font/script
/// handling for CJK/Thai/Latin) but uses the BeepTest layout: four rows
/// of language buttons, a filler row, and the Cancel / Apply footer at
/// the very bottom. There is no timeout ribbon (BeepTest has no concept
/// of timeouts).
///
/// The selected language lives in `settings.pending_language` (seeded by
/// `BeepTestEditOpenLanguage`) and the original language lives in
/// `settings.original_language`. Apply enables when these differ. When
/// the font family changes between original and selected (Latin ↔ CJK ↔
/// Thai), the Apply button label and style reflect a restart-required
/// commit; otherwise it is a normal "Done" green button.
pub(in super::super) fn build_beep_test_language_picker<'a>(
    settings: &EditableSettings,
) -> Element<'a, Message> {
    let selected = settings.pending_language.unwrap_or(Language::English);
    let original = settings.original_language.unwrap_or(Language::English);
    let has_changes = settings.pending_language != settings.original_language;

    // Font for Cancel/Done/Restart text so they render in the target
    // language's script regardless of the app's current default font.
    let selected_font: Option<iced_core::Font> = Some(font_for(selected));

    // A restart is needed when switching between Latin and CJK / Thai font families.
    let needs_restart = original.ui_font() != selected.ui_font();

    // Cancel / Apply(Restart) footer. The labels use the selected language's
    // own translation of CANCEL / APPLY / RESTART (so a user mid-switch can
    // read them) and the appropriate font for the selected script.
    let make_label = |content: &'static str, font: Option<iced_core::Font>| {
        let t = text(content)
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .width(Length::Shrink);
        let t: iced::widget::Text<'a, _, _> = if let Some(f) = font { t.font(f) } else { t };
        container(t).center(Length::Fill)
    };

    let footer_label = if has_changes {
        selected.cancel_text()
    } else {
        selected.back_text()
    };
    let cancel_btn = button(make_label(footer_label, selected_font))
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
        button(make_label(selected.apply_text(), selected_font))
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(confirm_msg)
            .into()
    };

    let [lang_row_1, lang_row_2, lang_row_3, lang_row_4] = make_language_grid_rows(selected);

    column![
        lang_row_1,
        lang_row_2,
        lang_row_3,
        lang_row_4,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![cancel_btn, horizontal_space(), confirm_btn].spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// The embedded beep-test preview picture for a layout (white-on-left only —
/// beep test has no sides control). Exhaustive match, mirroring
/// `layout_preview_handle` in `configuration.rs`; adding a `FrontDisplayLayout`
/// variant won't compile until its `beep-*.png` is added here and generated via
/// `just capture-previews`.
fn beep_test_layout_preview_handle(layout: FrontDisplayLayout) -> image::Handle {
    macro_rules! preview {
        ($stem:literal) => {
            &include_bytes!(concat!(
                "../../../resources/layout-previews/",
                $stem,
                ".png"
            ))[..]
        };
    }
    let bytes: &'static [u8] = match layout {
        FrontDisplayLayout::Default => preview!("beep-default"),
        FrontDisplayLayout::Classic => preview!("beep-classic"),
        FrontDisplayLayout::BigTime => preview!("beep-big-time"),
        FrontDisplayLayout::Corners => preview!("beep-corners"),
        FrontDisplayLayout::ScoresOnly => preview!("beep-scores-only"),
    };
    image::Handle::from_bytes(bytes)
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
    let cancel = make_chrome_button(cancel_or_back_label(has_changes))
        .style(red_button)
        .width(Length::Fill)
        .on_press(cancel_message);

    let apply = make_chrome_button(fl!("apply"))
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

#[cfg(test)]
mod test {
    use super::{
        EDIT_TABLE_CELL_SPACING, EDITOR_ROWS, LAYERS_PER_ROW, MAX_LAPS_PER_LEVEL, MAX_LEVELS,
        MIN_BUTTON_SIZE, PADDING, PRESET_ROWS, SMALL_TEXT, SPACING, TABLE_ROWS,
    };
    use crate::config::BeepTestPreset;

    // PRESET_ROWS is the display order, written out by hand. A court length
    // added to the config but forgotten here would simply never appear on the
    // screen, with every config test still green.
    #[test]
    fn preset_rows_cover_every_preset_exactly_once() {
        let listed: Vec<BeepTestPreset> = PRESET_ROWS
            .iter()
            .flat_map(|(left, right)| [*left, *right])
            .collect();

        assert_eq!(
            listed.len(),
            BeepTestPreset::ALL.len(),
            "PRESET_ROWS shows {} presets but there are {}",
            listed.len(),
            BeepTestPreset::ALL.len()
        );
        for preset in BeepTestPreset::ALL {
            assert_eq!(
                listed.iter().filter(|p| **p == preset).count(),
                1,
                "{preset:?} should appear exactly once in PRESET_ROWS"
            );
        }

        // Each row is one court length: the referee schedule on the left, the
        // full one on the right. Getting this the wrong way round would pair
        // REF 25M with the 23M button and read as a labelling bug.
        // Longest pool first, which is what PRESET_ROWS promises and what puts
        // 25yd (22860mm) between 23m and 22m rather than beside 25m.
        let lengths: Vec<u32> = PRESET_ROWS
            .iter()
            .map(|(left, _)| left.court_millimetres())
            .collect();
        let mut longest_first = lengths.clone();
        longest_first.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(
            lengths, longest_first,
            "PRESET_ROWS should run longest pool first"
        );

        for (left, right) in PRESET_ROWS {
            assert!(
                left.is_ref(),
                "{left:?} is on the left, so it must be a ref"
            );
            assert!(
                !right.is_ref(),
                "{right:?} is on the right, so it must be a full schedule"
            );
            assert_eq!(
                left.court_millimetres(),
                right.court_millimetres(),
                "a row must pair one court length with itself"
            );
        }
    }

    // The other half of the pair guarded by every_preset_fits_within_max_levels.
    // BeepTestEditSelectPreset stages a preset's levels wholesale and applies no
    // lap cap, while the table only reserves MAX_LAPS_PER_LEVEL lap layers. A
    // preset carrying a level of more laps than that would have those laps
    // silently not drawn -- the operator would load a schedule and see less of it
    // than they staged, with the height test below still passing.
    #[test]
    fn every_preset_stays_within_the_lap_cap() {
        for preset in BeepTestPreset::ALL {
            for (i, level) in preset.config().levels.iter().enumerate() {
                assert!(
                    level.count <= MAX_LAPS_PER_LEVEL,
                    "{preset:?} level {} has {} laps, over the {MAX_LAPS_PER_LEVEL} lap layers \
                     the table reserves",
                    i + 1,
                    level.count
                );
            }
        }
    }

    // Filling rows cannot push the footer off the screen — they shrink instead,
    // which is the whole reason this page no longer needs a height budget. What
    // they can do is get too short to read, and nothing in the layout notices.
    //
    // So pin the size the default screen actually yields: a table cell layer has
    // to hold one line of SMALL_TEXT plus the cell's own padding. Adding a row to
    // either column shows up here as cells dropping under that, which is how such
    // a change gets caught here rather than on the Pi at a tournament.
    //
    // Two things this does NOT cover, both pre-existing and neither fixable from
    // this page:
    //
    // * The budget is the DEFAULT window height. A screen configured shorter has
    //   less to give, and this test cannot know it.
    // * `Level::migrate` validates nothing, so a hand-edited config file can
    //   carry a level of any lap count. Those laps are simply not drawn now that
    //   the table reserves a fixed number of lap layers (see
    //   `build_editor_levels_table`), and nothing tells the operator they are
    //   missing. The fix belongs in config validation, not here.
    #[test]
    fn a_table_cell_stays_tall_enough_to_read_at_the_default_screen_height() {
        // The window is not resizable: its height comes from Hardware::screen_y.
        // Read from the type rather than restated, or changing the default would
        // leave this guarding a height nothing runs at.
        let budget = crate::config::Hardware::default().screen_y as f32 - 2.0 * PADDING;
        let body = budget - SPACING - MIN_BUTTON_SIZE; // less the gap and the footer

        let rows = (TABLE_ROWS + EDITOR_ROWS) as f32;
        let row = (body - (rows - 1.0) * SPACING) / rows;
        // Two layers and the gap between them share a page row.
        let layer = (row - SPACING) / LAYERS_PER_ROW as f32;

        // iced lays a line of text out at 1.3x its size.
        let needed = SMALL_TEXT * 1.3 + 2.0 * EDIT_TABLE_CELL_SPACING;
        assert!(
            layer >= needed,
            "with {rows} page rows a table cell comes out {layer}pt on the default screen, \
             under the {needed}pt a line of SMALL_TEXT needs"
        );
    }

    // BeepTestEditAddLevel enforces this cap at runtime (see
    // Message::BeepTestEditAddLevel in app/mod.rs), but BeepTestEditSelectPreset
    // does not — it stages a preset's levels wholesale and relies on every
    // preset already fitting under the cap. That is safe only because Full is
    // exactly MAX_LEVELS levels today; nothing else ties the two together, so
    // this test is the guard against either constant changing out from under
    // the other.
    #[test]
    fn every_preset_fits_within_max_levels() {
        for preset in BeepTestPreset::ALL {
            let levels = preset.config().levels;
            assert!(
                levels.len() <= MAX_LEVELS,
                "{preset:?} has {} levels, which exceeds MAX_LEVELS ({MAX_LEVELS})",
                levels.len()
            );
        }
    }
}
