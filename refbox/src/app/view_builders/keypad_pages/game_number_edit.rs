use super::*;
use iced::{
    Length,
    widget::{column, row, vertical_space},
};

pub(super) fn make_game_number_edit_page<'a>(
    value: u32,
    original: Option<String>,
) -> Element<'a, Message> {
    column![
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    game_number_has_changes(value, original.as_deref())
                        .then_some(Message::ParameterEditComplete { canceled: false }),
                ),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}

/// Returns true when the typed game number differs from the one stored when the
/// editor opened — i.e. when pressing Apply would actually change the stored
/// value. `value.to_string()` is exactly what `ParameterEditComplete` writes
/// back to the edited settings.
fn game_number_has_changes(value: u32, original: Option<&str>) -> bool {
    match original {
        Some(o) => value.to_string() != o,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_number_is_no_change() {
        assert!(!game_number_has_changes(5, Some("5")));
        assert!(!game_number_has_changes(12, Some("12")));
    }

    #[test]
    fn different_number_is_change() {
        assert!(game_number_has_changes(5, Some("6")));
    }

    #[test]
    fn missing_original_enables_apply() {
        // Defensive: the GameNumber keypad is only reached with edited settings
        // present, but if the original is unknown, don't block committing.
        assert!(game_number_has_changes(5, None));
    }
}
