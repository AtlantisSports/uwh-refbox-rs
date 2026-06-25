use super::*;
use iced::{
    Alignment, Length,
    alignment::Horizontal,
    widget::{column, horizontal_space, row, text, vertical_space},
};
use std::time::Duration;

pub(in super::super) fn build_time_edit_view<'a>(
    data: ViewData<'_, '_>,
    time: Duration,
    timeout_time: Option<Duration>,
    old_time: Duration,
    old_timeout_time: Option<Duration>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let mut edit_row = row![
        horizontal_space(),
        make_time_editor(fl!("game-time"), time, false, None),
        horizontal_space()
    ]
    .spacing(SPACING)
    .align_y(Alignment::Center);

    if snapshot.timeout.is_some() {
        edit_row = edit_row
            .push(horizontal_space())
            .push(make_time_editor(
                fl!("timeout"),
                timeout_time.unwrap(),
                true,
                None,
            ))
            .push(horizontal_space());
    }

    let has_changes = time_edit_has_changes(time, timeout_time, old_time, old_timeout_time);

    column![
        make_game_time_button(
            snapshot,
            false,
            true,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        edit_row,
        text(fl!("Note-Game-time-is-paused"))
            .size(SMALL_TEXT)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        vertical_space(),
        row![
            make_button(cancel_or_back_label(has_changes))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    has_changes.then_some(Message::TimeEditComplete { canceled: false }),
                ),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Returns true when either the game or timeout clock differs from the values
/// captured when the time-edit screen was opened.
///
/// The comparison is on whole seconds — the same precision shown on screen via
/// `time_string` — so that zeroing the clock and rebuilding it back to the
/// displayed value counts as "no change", even though the original may have
/// carried a sub-second remainder that the display never showed.
fn time_edit_has_changes(
    time: Duration,
    timeout_time: Option<Duration>,
    old_time: Duration,
    old_timeout_time: Option<Duration>,
) -> bool {
    time.as_secs() != old_time.as_secs()
        || timeout_time.map(|d| d.as_secs()) != old_timeout_time.map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::time_edit_has_changes;
    use std::time::Duration;

    #[test]
    fn no_change_is_false() {
        let t = Duration::from_secs(432);
        assert!(!time_edit_has_changes(t, None, t, None));
        assert!(!time_edit_has_changes(
            t,
            Some(Duration::from_secs(30)),
            t,
            Some(Duration::from_secs(30))
        ));
    }

    #[test]
    fn game_time_change_is_true() {
        assert!(time_edit_has_changes(
            Duration::from_secs(433),
            None,
            Duration::from_secs(432),
            None
        ));
    }

    #[test]
    fn timeout_change_is_true() {
        let t = Duration::from_secs(432);
        // A timeout started during edit: original None, now Some.
        assert!(time_edit_has_changes(
            t,
            Some(Duration::from_secs(60)),
            t,
            None
        ));
    }

    #[test]
    fn round_trip_back_to_original_is_false() {
        // +1s then -1s returns to the exact original duration.
        let original = Duration::from_secs(432);
        let after_round_trip = original + Duration::from_secs(1) - Duration::from_secs(1);
        assert!(!time_edit_has_changes(
            after_round_trip,
            None,
            original,
            None
        ));
    }

    #[test]
    fn zeroing_the_clock_is_a_change() {
        // Pressing "= 0" drops the clock to (near) zero, which differs from the
        // original displayed value, so Apply must be enabled to commit it.
        let original = Duration::from_secs(897);
        let zeroed = Duration::from_micros(1);
        assert!(time_edit_has_changes(zeroed, None, original, None));
    }

    #[test]
    fn sub_second_original_matches_displayed_whole_seconds() {
        // Original is 432.6s but displays as "7:12" (432 whole seconds). Zeroing
        // and rebuilding to an exact 432.0s lands on the same displayed value, so
        // it must count as "no change".
        let original = Duration::from_millis(432_600);
        let rebuilt = Duration::from_secs(432);
        assert!(!time_edit_has_changes(rebuilt, None, original, None));
    }
}
