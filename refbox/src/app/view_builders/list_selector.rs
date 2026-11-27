use super::*;
use collect_array::CollectArrayResult;
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, row, text, vertical_space},
};

fn sorted_events_for_picker(events: &BTreeMap<EventId, Event>) -> Vec<&Event> {
    let mut sorted: Vec<&Event> = events.values().collect();
    sorted.sort_by(|a, b| {
        a.date_range
            .start
            .cmp(&b.date_range.start)
            .then(a.date_range.end.cmp(&b.date_range.end))
            .then(a.id.cmp(&b.id))
    });
    sorted
}

pub(in super::super) const PARAMETER_LIST_LEN: usize = 4;

/// Where the visible window of a parameter list starts. A start past the last
/// full window strands every item above it when the whole list fits on screen
/// (both scroll arrows go dead), so the opener and the rows both clamp through
/// this one rule. The arrows in `make_scroll_list` clamp with the same formula
/// on their own copy of the list length: change one, change both.
pub(in super::super) fn list_window_start(position: usize, num_items: usize) -> usize {
    position.min(num_items.saturating_sub(PARAMETER_LIST_LEN))
}

/// Where SELECT GAME opens: on the selected game's row, clamped by
/// [`list_window_start`]. `court_games` must be that court's game numbers in
/// the order the picker draws them; a selected game not among them opens at
/// the top.
pub(in super::super) fn game_picker_start<'a>(
    court_games: impl Iterator<Item = &'a str>,
    selected: &str,
) -> usize {
    let games: Vec<&str> = court_games.collect();
    let position = games.iter().position(|g| *g == selected).unwrap_or(0);
    list_window_start(position, games.len())
}

pub(in super::super) fn build_list_selector_page<'a>(
    data: ViewData<'_, '_>,
    param: ListableParameter,
    index: usize,
    settings: &EditableSettings,
    events: &EventStore,
) -> Element<'a, Message> {
    const LIST_LEN: usize = PARAMETER_LIST_LEN;
    const TEAM_NAME_LEN_LIMIT: usize = 15;

    let ViewData {
        snapshot,
        mode,
        clock,
        teams,
        portal_indicator,
        ..
    } = data;

    let title = match param {
        ListableParameter::Event => fl!("select-event"),
        ListableParameter::Court => fl!("select-court"),
        ListableParameter::Game => fl!("select-game"),
    };

    let title = text(title)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    // (btn_text, msg_val)

    macro_rules! make_buttons {
        ($iter:ident, $transform:ident, $start:expr) => {
            $iter
                .skip($start)
                .map($transform)
                .map(Some)
                .chain([None].into_iter().cycle())
                .take(LIST_LEN)
                .map(|pen| {
                    if let Some((btn_text, msg_val)) = pen {
                        let text = text(btn_text)
                            .align_y(Vertical::Center)
                            .align_x(Horizontal::Left)
                            .width(Length::Fill);

                        button(text)
                            .padding(PADDING)
                            .height(Length::Fixed(MIN_BUTTON_SIZE))
                            .width(Length::Fill)
                            .style(gray_button)
                            .on_press(Message::ParameterSelected(param, msg_val))
                            .into()
                    } else {
                        button(Space::with_width(Length::Shrink))
                            .height(Length::Fixed(MIN_BUTTON_SIZE))
                            .width(Length::Fill)
                            .style(gray_button)
                            .into()
                    }
                })
                .collect()
        };
    }

    let (num_items, start, buttons): (usize, usize, CollectArrayResult<_, LIST_LEN>) = match param {
        ListableParameter::Event => {
            // Only the portal offers a list, so this can only ever be the
            // portal's own events. `unwrap` is safe for the same reason it was
            // before: the tile has no press action unless this is `Some`.
            let list = events.selectable(settings.source).unwrap();
            let num_items = list.len();
            let sorted = sorted_events_for_picker(list);
            let iter = sorted.into_iter();
            let transform = |e: &Event| (e.name.clone(), e.id.full().to_string());
            let start = list_window_start(index, num_items);
            (num_items, start, make_buttons!(iter, transform, start))
        }
        ListableParameter::Court => {
            // `pool_btn_msg` in `configuration.rs` — the `token_rejected` guard —
            // gates the Court tile's press action on this same
            // events.get(*source, id)?.courts chain, so all three unwraps below
            // are `Some` whenever this arm is reachable. Cited by name, not line:
            // the previous line citation had already drifted onto a different
            // button.

            let list = events
                .get(settings.source, settings.current_event_id.as_ref().unwrap())
                .unwrap()
                .courts
                .as_ref()
                .unwrap();
            let num_items = list.len();
            let iter = list.iter();
            let transform = |p: &String| (p.clone(), p.clone());
            let start = list_window_start(index, num_items);
            (num_items, start, make_buttons!(iter, transform, start))
        }
        ListableParameter::Game => {
            let schedule = settings.schedule.as_ref().unwrap();
            let court = settings.current_court.clone().unwrap();
            let list: Vec<_> = schedule
                .games
                .values()
                .filter(|g| g.court == court)
                .collect();
            let num_items = list.len();
            let iter = list.into_iter();
            let transform = |g| {
                (
                    game_string_long(g, teams, TEAM_NAME_LEN_LIMIT),
                    g.number.to_string(),
                )
            };
            let start = list_window_start(index, num_items);
            (num_items, start, make_buttons!(iter, transform, start))
        }
    };

    let scroll_list = make_scroll_list(
        buttons.unwrap(),
        num_items,
        start,
        title,
        ScrollOption::GameParameter,
        light_gray_container,
    )
    .width(Length::FillPortion(4));

    column![
        make_game_time_button(snapshot, false, false, mode, clock, portal_indicator, None),
        row![
            scroll_list,
            column![
                vertical_space(),
                make_chrome_button(fl!("cancel"))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
            ]
            .width(Length::Fill),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;
    use uwh_common::uwhportal::schedule::DateRange;

    fn test_event(
        partial_id: &str,
        start: time::OffsetDateTime,
        end: time::OffsetDateTime,
    ) -> Event {
        Event {
            id: EventId::from_partial(partial_id),
            name: format!("Event {partial_id}"),
            slug: partial_id.to_string(),
            date_range: DateRange { start, end },
            teams: None,
            schedule: None,
            courts: None,
        }
    }

    #[test]
    fn sorts_events_by_start_date_ascending() {
        let mut events = BTreeMap::new();
        let later = test_event(
            "later",
            datetime!(2026-06-01 0:00 UTC),
            datetime!(2026-06-03 0:00 UTC),
        );
        let sooner = test_event(
            "sooner",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-17 0:00 UTC),
        );
        events.insert(later.id.clone(), later);
        events.insert(sooner.id.clone(), sooner);

        let sorted = sorted_events_for_picker(&events);
        assert_eq!(sorted[0].slug, "sooner");
        assert_eq!(sorted[1].slug, "later");
    }

    #[test]
    fn breaks_ties_on_end_date_ascending() {
        let mut events = BTreeMap::new();
        let longer = test_event(
            "longer",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-20 0:00 UTC),
        );
        let shorter = test_event(
            "shorter",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-16 0:00 UTC),
        );
        events.insert(longer.id.clone(), longer);
        events.insert(shorter.id.clone(), shorter);

        let sorted = sorted_events_for_picker(&events);
        assert_eq!(sorted[0].slug, "shorter");
        assert_eq!(sorted[1].slug, "longer");
    }

    #[test]
    fn breaks_ties_on_event_id() {
        let mut events = BTreeMap::new();
        let aaa = test_event(
            "aaa",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-17 0:00 UTC),
        );
        let bbb = test_event(
            "bbb",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-17 0:00 UTC),
        );
        events.insert(aaa.id.clone(), aaa);
        events.insert(bbb.id.clone(), bbb);

        let sorted = sorted_events_for_picker(&events);
        // EventId Ord is lexicographic on the full id ("events/aaa" < "events/bbb")
        assert_eq!(sorted[0].slug, "aaa");
        assert_eq!(sorted[1].slug, "bbb");
    }

    #[test]
    fn a_list_that_fits_always_starts_at_the_top() {
        // 2, 3 and 4 games, opened on the last one: the bug stranded the rest
        assert_eq!(list_window_start(1, 2), 0);
        assert_eq!(list_window_start(2, 3), 0);
        assert_eq!(list_window_start(3, 4), 0);
        assert_eq!(list_window_start(2, 4), 0); // the PO's case: court 1/3/6/7, opened on 6
    }

    #[test]
    fn a_longer_list_opened_near_its_end_shows_a_full_last_window() {
        assert_eq!(list_window_start(4, 5), 1);
        assert_eq!(list_window_start(2, 5), 1);
        assert_eq!(list_window_start(5, 6), 2);
    }

    #[test]
    fn a_longer_list_opened_early_keeps_its_position() {
        assert_eq!(list_window_start(0, 5), 0);
        assert_eq!(list_window_start(1, 5), 1);
        assert_eq!(list_window_start(3, 30), 3);
    }

    #[test]
    fn select_game_opens_at_the_top_of_a_court_that_fits() {
        // The PO's case: court 1 holds games 1/3/6/7, picker opened on 6
        let court = ["1", "3", "6", "7"];
        assert_eq!(game_picker_start(court.iter().copied(), "6"), 0);
        assert_eq!(game_picker_start(court.iter().copied(), "7"), 0);
        assert_eq!(game_picker_start(court[..2].iter().copied(), "3"), 0);
        assert_eq!(game_picker_start(court[..3].iter().copied(), "6"), 0);
    }

    #[test]
    fn select_game_on_a_longer_court_keeps_the_selected_game_in_view() {
        let court = ["1", "3", "6", "7", "9", "12"];
        assert_eq!(game_picker_start(court.iter().copied(), "12"), 2);
        assert_eq!(game_picker_start(court.iter().copied(), "3"), 1);
        assert_eq!(game_picker_start(court.iter().copied(), "1"), 0);
    }

    #[test]
    fn select_game_with_no_matching_game_opens_at_the_top() {
        let court = ["1", "3", "6", "7", "9", "12"];
        assert_eq!(game_picker_start(court.iter().copied(), "99"), 0);
        assert_eq!(game_picker_start(court.iter().copied(), ""), 0);
    }

    #[test]
    fn an_empty_list_starts_at_the_top() {
        assert_eq!(list_window_start(0, 0), 0);
        assert_eq!(list_window_start(3, 0), 0);
    }
}
