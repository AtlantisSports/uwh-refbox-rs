use super::{
    style::{ButtonStyle, ContainerStyle, Element, SMALL_PLUS_TEXT, SPACING},
    *,
};
use iced::widget::scrollable;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{column, container, horizontal_space, row, text},
    Length,
};

use uwh_common::game_snapshot::{Color as GameColor, GameSnapshot};

pub(in super::super) fn build_warnings_summary_page<'a>(
    snapshot: &GameSnapshot,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let warnings_container = container(column![
        text("WARNINGS")
            .size(SMALL_PLUS_TEXT)
            .vertical_alignment(Vertical::Top)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
        scrollable(
            row![
                column(
                    snapshot
                        .b_warnings
                        .iter()
                        .rev()
                        .map(
                            |warning| make_warning_container(warning, Some(GameColor::Black))
                                .into()
                        )
                        .collect()
                )
                .spacing(SPACING)
                .width(Length::Fill)
                .padding(PADDING),
                column(
                    snapshot
                        .w_warnings
                        .iter()
                        .rev()
                        .map(
                            |warning| make_warning_container(warning, Some(GameColor::White))
                                .into()
                        )
                        .collect()
                )
                .spacing(SPACING)
                .width(Length::Fill)
                .padding(PADDING),
            ]
            .spacing(SPACING)
        )
    ])
    .style(ContainerStyle::LightGray)
    .width(Length::Fill)
    .height(Length::Fill);

    let fouls_container = container(column![
        text("FOULS")
            .size(SMALL_PLUS_TEXT)
            .vertical_alignment(Vertical::Top)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
        scrollable(column![
            row![
                column(
                    snapshot
                        .b_fouls
                        .iter()
                        .rev()
                        .map(|fouls| make_warning_container(fouls, Some(GameColor::Black)).into())
                        .collect()
                )
                .spacing(SPACING)
                .width(Length::Fill)
                .padding(PADDING),
                column(
                    snapshot
                        .w_fouls
                        .iter()
                        .rev()
                        .map(|fouls| make_warning_container(fouls, Some(GameColor::White)).into())
                        .collect()
                )
                .spacing(SPACING)
                .width(Length::Fill)
                .padding(PADDING),
            ]
            .spacing(SPACING),
            column(
                snapshot
                    .equal_fouls
                    .iter()
                    .rev()
                    .map(|fouls| row![
                        horizontal_space(Length::Fill),
                        make_warning_container(fouls, None),
                        horizontal_space(Length::Fill),
                    ]
                    .width(Length::Fill)
                    .into())
                    .collect()
            )
            .spacing(SPACING)
            .width(Length::Fill)
            .padding(PADDING)
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
            make_button("BACK")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            make_button("EDIT WARNINGS")
                .style(ButtonStyle::Blue)
                .width(Length::Fill)
                .on_press(Message::WarningOverview),
            make_button("EDIT FOULS")
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
