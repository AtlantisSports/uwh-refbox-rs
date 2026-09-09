use super::*;
use iced::{
    Length, Theme,
    widget::{
        button::{Status, Style},
        column, row, vertical_space,
    },
};

type StyleFn = fn(&Theme, Status) -> Style;

/// The court grid draws exactly four buttons, in two rows of two. Two across is
/// the same cell CANCEL and APPLY use -- the widest this column can offer, and
/// wide enough for the longest translated label. Four across would be 149px
/// each, narrower than any four-across row in the app. Raising the cap
/// therefore means choosing a new layout, so the two are coupled here rather
/// than letting a fifth court silently fall off the page.
const _: () = assert!(
    crate::config::MAX_COURTS == 4,
    "the court grid draws two rows of two"
);

pub(super) fn make_game_number_edit_page<'a>(
    value: u32,
    courts: u8,
    original: Option<(String, u8)>,
) -> Element<'a, Message> {
    let court_button = |count: u8| -> Element<'a, Message> {
        let selected = count == courts;
        let style: StyleFn = if selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_chrome_button(fl!("court-count", count = count))
            .style(style)
            .width(Length::Fill)
            // Pressing the lit button does nothing, exactly as pressing the
            // already-selected HALF does on the team-timeout page: one count is
            // always selected, so there is no "off" to toggle to.
            .on_press(if selected {
                Message::NoAction
            } else {
                Message::SetCourtCount(count)
            })
            .into()
    };

    column![
        row![court_button(1), court_button(2)].spacing(SPACING),
        row![court_button(3), court_button(4)].spacing(SPACING),
        vertical_space(),
        row![
            make_chrome_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_chrome_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    game_number_has_changes(value, courts, original.as_ref())
                        .then_some(Message::ParameterEditComplete { canceled: false }),
                ),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}

/// Returns true when the staged values differ from those stored when the editor
/// opened -- i.e. when pressing Apply would actually change something. Both
/// values count: a court-count change alone must enable Apply, or it could only
/// ever be committed alongside an edit to the number.
fn game_number_has_changes(value: u32, courts: u8, original: Option<&(String, u8)>) -> bool {
    match original {
        Some((number, original_courts)) => {
            value.to_string() != *number || courts != *original_courts
        }
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_number_is_no_change() {
        assert!(!game_number_has_changes(5, 1, Some(&("5".to_string(), 1))));
        assert!(!game_number_has_changes(
            12,
            1,
            Some(&("12".to_string(), 1))
        ));
    }

    #[test]
    fn different_number_is_change() {
        assert!(game_number_has_changes(5, 1, Some(&("6".to_string(), 1))));
    }

    #[test]
    fn a_changed_court_count_is_a_change() {
        // Without this, APPLY stays grey and the court count can only ever be
        // committed by also editing the game number.
        assert!(game_number_has_changes(5, 2, Some(&("5".to_string(), 1))));
    }

    #[test]
    fn missing_original_enables_apply() {
        // Defensive: the GameNumber keypad is only reached with edited settings
        // present, but if the original is unknown, don't block committing.
        assert!(game_number_has_changes(5, 1, None));
    }
}
