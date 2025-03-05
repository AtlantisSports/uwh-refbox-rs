use super::{
    style::{ButtonStyle, ContainerStyle, Element, SMALL_PLUS_TEXT, SPACING},
    *,
};
use iced::widget::scrollable;
use iced::{
    Length,
    alignment::{Horizontal, Vertical},
    widget::{column, container, row, text},
};

pub(in super::super) fn build_warnings_summary_page<'a>(
    data: ViewData<'_, '_>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let warnings_container = container(column![
        text(fl!("warnings"))
            .size(SMALL_PLUS_TEXT)
            .vertical_alignment(Vertical::Top)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
        scrollable(
            row(snapshot
                .warnings
                .iter()
                .map(|(color, warns)| column(
                    warns
                        .iter()
                        .rev()
                        .map(|warning| make_warning_container(warning, Some(color)).into())
                        .collect()
                )
                .spacing(SPACING)
                .width(Length::Fill)
                .padding(PADDING)
                .into())
                .collect())
            .spacing(SPACING)
        )
    ])
    .style(ContainerStyle::LightGray)
    .width(Length::Fill)
    .height(Length::Fill);

    let foul_lists: OptColorBundle<Option<Element<Message>>> = snapshot
        .fouls
        .iter()
        .map(|(color, fouls)| {
            (
                color,
                Some(
                    column(
                        fouls
                            .iter()
                            .rev()
                            .map(|fouls| make_warning_container(fouls, color).into())
                            .collect(),
                    )
                    .into(),
                ),
            )
        })
        .collect();

    let fouls_container = container(column![
        text(fl!("fouls"))
            .size(SMALL_PLUS_TEXT)
            .vertical_alignment(Vertical::Top)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
        scrollable(column![
            row![foul_lists.black.unwrap(), foul_lists.white.unwrap()].spacing(SPACING),
            foul_lists.equal.unwrap()
        ])
        .height(Length::Fill)
    ])
    .style(ContainerStyle::LightGray)
    .width(Length::Fill)
    .height(Length::Fill);

    let warnings_and_fouls_row = row![
        warnings_container.width(Length::Fill),
        fouls_container.width(Length::Fill)
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running,),
        warnings_and_fouls_row.height(Length::Fill),
        row![
            make_button(fl!("back"))
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            make_button(fl!("edit-warnings"))
                .style(ButtonStyle::Blue)
                .width(Length::Fill)
                .on_press(Message::WarningOverview),
            make_button(fl!("edit-fouls"))
                .style(ButtonStyle::Orange)
                .width(Length::Fill)
                .on_press(Message::FoulOverview),
        ]
        .spacing(SPACING)
        .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
