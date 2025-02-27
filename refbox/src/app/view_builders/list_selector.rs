use super::{
    style::{ButtonStyle, ContainerStyle, Element, LINE_HEIGHT, MIN_BUTTON_SIZE, PADDING, SPACING},
    *,
};
use collect_array::CollectArrayResult;
use iced::{
    Length,
    alignment::{Horizontal, Vertical},
    widget::{button, column, horizontal_space, row, text, vertical_space},
};

use uwh_common::game_snapshot::GameSnapshot;

pub(in super::super) fn build_list_selector_page<'a>(
    snapshot: &GameSnapshot,
    param: ListableParameter,
    index: usize,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    const LIST_LEN: usize = 4;
    const TEAM_NAME_LEN_LIMIT: usize = 15;

    let title = match param {
        ListableParameter::Tournament => "SELECT TOURNAMENT",
        ListableParameter::Pool => "SELECT COURT",
        ListableParameter::Game => "SELECT GAME",
    };

    let title = text(title)
        .line_height(LINE_HEIGHT)
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

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
                            .line_height(LINE_HEIGHT)
                            .vertical_alignment(Vertical::Center)
                            .horizontal_alignment(Horizontal::Left)
                            .width(Length::Fill);

                        button(text)
                            .padding(PADDING)
                            .height(Length::Fixed(MIN_BUTTON_SIZE))
                            .width(Length::Fill)
                            .style(ButtonStyle::Gray)
                            .on_press(Message::ParameterSelected(param, msg_val))
                            .into()
                    } else {
                        button(horizontal_space(Length::Shrink))
                            .height(Length::Fixed(MIN_BUTTON_SIZE))
                            .width(Length::Fill)
                            .style(ButtonStyle::Gray)
                            .into()
                    }
                })
                .collect()
        };
    }

    let (num_items, buttons): (usize, CollectArrayResult<_, LIST_LEN>) = match param {
        ListableParameter::Tournament => {
            let list = tournaments.as_ref().unwrap();
            let num_items = list.len();
            let iter = list.values().rev();
            let transform = |t: &TournamentInfo| (t.name.clone(), t.tid as usize);
            (num_items, make_buttons!(iter, transform))
        }
        ListableParameter::Pool => {
            let list = tournaments
                .as_ref()
                .unwrap()
                .get(&settings.current_tid.unwrap())
                .unwrap()
                .pools
                .as_ref()
                .unwrap();
            let num_items = list.len();
            let iter = list.iter().enumerate();
            let transform = |(i, p): (usize, &String)| (p.clone(), i);
            (num_items, make_buttons!(iter, transform))
        }
        ListableParameter::Game => {
            let list = settings.games.as_ref().unwrap();
            let pool = settings.current_pool.clone().unwrap();
            let num_items = list.values().filter(|g| g.pool == pool).count();
            let iter = list.values().filter(|g| g.pool == pool);
            let transform = |g| (game_string_long(g, TEAM_NAME_LEN_LIMIT), g.gid as usize);
            (num_items, make_buttons!(iter, transform))
        }
    };

    let scroll_list = make_scroll_list(
        buttons.unwrap(),
        num_items,
        index,
        title,
        ScrollOption::GameParameter,
        ContainerStyle::LightGray,
    )
    .width(Length::FillPortion(4));

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            scroll_list,
            column![
                vertical_space(Length::Fill),
                make_button("CANCEL")
                    .style(ButtonStyle::Red)
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
