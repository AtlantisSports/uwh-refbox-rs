use super::*;
use iced::{
    Alignment, Element, Length,
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
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::Black,
                        increase: true,
                    }),
                make_small_button("-", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::Black,
                        increase: false,
                    }),
            ]
            .spacing(SPACING),
            column![
                text(fl!("dark-team-name-caps")),
                text(scores.black.to_string())
                    .size(LARGE_TEXT)
                    .line_height(LINE_HEIGHT)
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(black_container);

    let white_edit = container(
        row![
            column![
                text(fl!("light-team-name-caps")),
                text(scores.white.to_string())
                    .size(LARGE_TEXT)
                    .line_height(LINE_HEIGHT)
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_x(Alignment::Center),
            column![
                make_small_button("+", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::White,
                        increase: true,
                    }),
                make_small_button("-", LARGE_TEXT)
                    .style(blue_button)
                    .on_press(Message::ChangeScore {
                        color: GameColor::White,
                        increase: false,
                    }),
            ]
            .spacing(SPACING),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(white_container);

    let mut main_col = column![
        make_game_time_button(snapshot, false, is_confirmation, mode, clock_running),
        vertical_space()
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    if is_confirmation {
        main_col = main_col
            .push(
                text(fl!("final-score"))
                    .line_height(LINE_HEIGHT)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
            )
            .push(vertical_space());
    }

    main_col
        .push(
            row![
                horizontal_space(),
                black_edit,
                horizontal_space(),
                white_edit,
                horizontal_space()
            ]
            .spacing(SPACING),
        )
        .push(vertical_space())
        .push(
            row![
                make_message_button(fl!("cancel"), cancel_btn_msg).style(red_button),
                horizontal_space(),
                make_button(fl!("done"))
                    .style(green_button)
                    .on_press(Message::ScoreEditComplete { canceled: false }),
            ]
            .spacing(SPACING),
        )
        .into()
}
