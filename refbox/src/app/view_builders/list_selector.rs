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

pub(in super::super) fn build_list_selector_page<'a>(
    data: ViewData<'_, '_>,
    param: ListableParameter,
    index: usize,
    settings: &EditableSettings,
    events: Option<&BTreeMap<EventId, Event>>,
) -> Element<'a, Message> {
    const LIST_LEN: usize = 4;
    const TEAM_NAME_LEN_LIMIT: usize = 15;

    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        portal_indicator,
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
        ($iter:ident, $transform:ident) => {
            $iter
                .skip(index)
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

    let (num_items, buttons): (usize, CollectArrayResult<_, LIST_LEN>) = match param {
        ListableParameter::Event => {
            let list = events.as_ref().unwrap();
            let num_items = list.len();
            let sorted = sorted_events_for_picker(list);
            let iter = sorted.into_iter();
            let transform = |e: &Event| (e.name.clone(), e.id.full().to_string());
            (num_items, make_buttons!(iter, transform))
        }
        ListableParameter::Court => {
            let list = events
                .as_ref()
                .unwrap()
                .get(settings.current_event_id.as_ref().unwrap())
                .unwrap()
                .courts
                .as_ref()
                .unwrap();
            let num_items = list.len();
            let iter = list.iter();
            let transform = |p: &String| (p.clone(), p.clone());
            (num_items, make_buttons!(iter, transform))
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
            (num_items, make_buttons!(iter, transform))
        }
    };

    let scroll_list = make_scroll_list(
        buttons.unwrap(),
        num_items,
        index,
        title,
        ScrollOption::GameParameter,
        light_gray_container,
    )
    .width(Length::FillPortion(4));

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
            scroll_list,
            column![
                vertical_space(),
                make_button(fl!("cancel"))
                    .style(red_button)
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
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
}
