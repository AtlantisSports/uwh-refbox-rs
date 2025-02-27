use super::{
    style::{ButtonStyle, ContainerStyle, Element, LARGE_TEXT, LINE_HEIGHT, PADDING, SPACING},
    *,
};

use iced::{
    Alignment, Length,
    alignment::Horizontal,
    widget::{column, container, horizontal_space, row, text, vertical_space},
};

use uwh_common::color::Color as GameColor;

pub(in super::super) fn build_score_edit_view<'a>(
    data: ViewData<'_, '_>,
    scores: BlackWhiteBundle<u8>,
    is_confirmation: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let cancel_btn_msg = if is_confirmation {
        None
    } else {
        Some(Message::ScoreEditComplete { canceled: true })
    };

    let black_edit = container(
        row![
            column![
                make_small_button("+", LARGE_TEXT)
                    .style(ButtonStyle::Blue)
                    .on_press(Message::ChangeScore {
                        color: GameColor::Black,
                        increase: true,
                    }),
                make_small_button("-", LARGE_TEXT)
                    .style(ButtonStyle::Blue)
                    .on_press(Message::ChangeScore {
                        color: GameColor::Black,
                        increase: false,
                    }),
            ]
            .spacing(SPACING),
            column![
                "BLACK",
                text(scores.black.to_string())
                    .size(LARGE_TEXT)
                    .line_height(LINE_HEIGHT)
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_items(Alignment::Center),
        ]
        .spacing(SPACING)
        .align_items(Alignment::Center),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(ContainerStyle::Black);

    let white_edit = container(
        row![
            column![
                "WHITE",
                text(scores.white.to_string())
                    .size(LARGE_TEXT)
                    .line_height(LINE_HEIGHT)
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_items(Alignment::Center),
            column![
                make_small_button("+", LARGE_TEXT)
                    .style(ButtonStyle::Blue)
                    .on_press(Message::ChangeScore {
                        color: GameColor::White,
                        increase: true,
                    }),
                make_small_button("-", LARGE_TEXT)
                    .style(ButtonStyle::Blue)
                    .on_press(Message::ChangeScore {
                        color: GameColor::White,
                        increase: false,
                    }),
            ]
            .spacing(SPACING),
        ]
        .spacing(SPACING)
        .align_items(Alignment::Center),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(ContainerStyle::White);

    let mut main_col = column![
        make_game_time_button(snapshot, false, is_confirmation, mode, clock_running),
        vertical_space(Length::Fill)
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    if is_confirmation {
        main_col = main_col
            .push(
                text("Please enter the final score")
                    .line_height(LINE_HEIGHT)
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill),
            )
            .push(vertical_space(Length::Fill));
    }

    main_col
        .push(
            row![
                horizontal_space(Length::Fill),
                black_edit,
                horizontal_space(Length::Fill),
                white_edit,
                horizontal_space(Length::Fill)
            ]
            .spacing(SPACING),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row![
                make_message_button("CANCEL", cancel_btn_msg).style(ButtonStyle::Red),
                horizontal_space(Length::Fill),
                make_button("DONE")
                    .style(ButtonStyle::Green)
                    .on_press(Message::ScoreEditComplete { canceled: false }),
            ]
            .spacing(SPACING),
        )
        .into()
}
