use super::*;
use collect_array::CollectArrayResult;
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, row, text, vertical_space},
};

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
            let iter = list.values().rev();
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
            let list = &schedule.games;
            let court = settings.current_court.clone().unwrap();
            let num_items = list.values().filter(|g| g.court == court).count();
            let iter = list.values().filter(|g| g.court == court);
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
        make_game_time_button(snapshot, false, false, mode, clock_running),
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
