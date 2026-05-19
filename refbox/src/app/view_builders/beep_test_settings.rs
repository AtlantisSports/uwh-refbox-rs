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
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::Sound));

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

/// Stub for the Sound Settings sub-page. Implemented in Task 5.
pub(in super::super) fn build_beep_test_sound_settings_page<'a>() -> Element<'a, Message> {
    placeholder_page("TODO: Sound Settings (Task 5)")
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
