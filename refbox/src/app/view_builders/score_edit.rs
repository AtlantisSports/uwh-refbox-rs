use super::{
    style::{self, LARGE_TEXT, PADDING, SPACING},
    *,
};

use iced::{
    alignment::Horizontal,
    pure::{column, container, horizontal_space, row, text, vertical_space, Element},
    Alignment, Length,
};

use uwh_common::game_snapshot::{Color as GameColor, GameSnapshot};

pub(in super::super) fn build_score_edit_view<'a>(
    snapshot: &GameSnapshot,
    scores: BlackWhiteBundle<u8>,
    is_confirmation: bool,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let cancel_btn_msg = if is_confirmation {
        None
    } else {
        Some(Message::ScoreEditComplete { canceled: true })
    };

    let black_edit = container(
        row()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(
                column()
                    .spacing(SPACING)
                    .push(
                        make_small_button("+", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::ChangeScore {
                                color: GameColor::Black,
                                increase: true,
                            }),
                    )
                    .push(
                        make_small_button("-", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::ChangeScore {
                                color: GameColor::Black,
                                increase: false,
                            }),
                    ),
            )
            .push(
                column()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .push("BLACK")
                    .push(text(scores.black.to_string()).size(LARGE_TEXT)),
            ),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(style::Container::Black);

    let white_edit = container(
        row()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(
                column()
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .push("WHITE")
                    .push(text(scores.white.to_string()).size(LARGE_TEXT)),
            )
            .push(
                column()
                    .spacing(SPACING)
                    .push(
                        make_small_button("+", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::ChangeScore {
                                color: GameColor::White,
                                increase: true,
                            }),
                    )
                    .push(
                        make_small_button("-", LARGE_TEXT)
                            .style(style::Button::Blue)
                            .on_press(Message::ChangeScore {
                                color: GameColor::White,
                                increase: false,
                            }),
                    ),
            ),
    )
    .padding(PADDING)
    .width(Length::FillPortion(2))
    .style(style::Container::White);

    let mut main_col = column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
        ))
        .push(vertical_space(Length::Fill));

    if is_confirmation {
        main_col = main_col
            .push(
                text("Please enter the final score")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill),
            )
            .push(vertical_space(Length::Fill));
    }

    main_col
        .push(
            row()
                .spacing(SPACING)
                .push(horizontal_space(Length::Fill))
                .push(black_edit)
                .push(horizontal_space(Length::Fill))
                .push(white_edit)
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(make_message_button("CANCEL", cancel_btn_msg).style(style::Button::Red))
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .on_press(Message::ScoreEditComplete { canceled: false }),
                ),
        )
        .into()
}
