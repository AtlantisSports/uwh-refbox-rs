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
use iced::{
    Element, Length,
    widget::{column, horizontal_space, row, text},
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
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::EditLevels));

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

/// Stub for the Edit Levels sub-page. Implemented in Task 6.
pub(in super::super) fn build_beep_test_edit_levels_page<'a>() -> Element<'a, Message> {
    placeholder_page("TODO: Edit Levels (Task 6)")
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
