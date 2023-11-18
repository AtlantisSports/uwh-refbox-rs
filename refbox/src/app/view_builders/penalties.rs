use super::{
    style::{self, GREEN, MIN_BUTTON_SIZE, ORANGE, PADDING, RED, SPACING},
    *,
};
use collect_array::CollectArrayResult;
use iced::{
    alignment::{Horizontal, Vertical},
    pure::{button, column, horizontal_space, row, text, widget::Container, Element},
    Length,
};

use uwh_common::game_snapshot::{Color as GameColor, GameSnapshot};

pub(in super::super) fn build_penalty_overview_page<'a>(
    snapshot: &GameSnapshot,
    penalties: BlackWhiteBundle<Vec<(String, FormatHint, PenaltyKind)>>,
    indices: BlackWhiteBundle<usize>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
        ))
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_penalty_list(
                    penalties.black,
                    indices.black,
                    GameColor::Black,
                ))
                .push(make_penalty_list(
                    penalties.white,
                    indices.white,
                    GameColor::White,
                )),
        )
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::PenaltyOverviewComplete { canceled: true }),
                )
                .push(
                    make_button("NEW")
                        .style(style::Button::Blue)
                        .width(Length::Fill)
                        .on_press(Message::KeypadPage(KeypadPage::Penalty(
                            None,
                            GameColor::Black,
                            PenaltyKind::OneMinute,
                        ))),
                )
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::PenaltyOverviewComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_penalty_list<'a>(
    penalties: Vec<(String, FormatHint, PenaltyKind)>,
    index: usize,
    color: GameColor,
) -> Container<'a, Message> {
    const PENALTY_LIST_LEN: usize = 3;

    let title = text(format!("{} PENALTIES", color.to_string().to_uppercase()))
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

    let num_pens = penalties.len();

    let buttons: CollectArrayResult<_, PENALTY_LIST_LEN> = penalties
        .into_iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(PENALTY_LIST_LEN)
        .map(|pen| {
            if let Some((i, (pen_text, format, kind))) = pen {
                let mut text = text(pen_text)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Left)
                    .width(Length::Fill);

                match format {
                    FormatHint::NoChange => {}
                    FormatHint::Edited => text = text.color(ORANGE),
                    FormatHint::Deleted => text = text.color(RED),
                    FormatHint::New => text = text.color(GREEN),
                }

                button(text)
                    .padding(PADDING)
                    .height(Length::Units(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(style::Button::Gray)
                    .on_press(Message::KeypadPage(KeypadPage::Penalty(
                        Some((color, i)),
                        color,
                        kind,
                    )))
                    .into()
            } else {
                button(horizontal_space(Length::Shrink))
                    .height(Length::Units(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(style::Button::Gray)
                    .on_press(Message::KeypadPage(KeypadPage::Penalty(
                        None,
                        color,
                        PenaltyKind::OneMinute,
                    )))
                    .into()
            }
        })
        .collect();

    let cont_style = match color {
        GameColor::Black => style::Container::Black,
        GameColor::White => style::Container::White,
    };

    let scroll_option = match color {
        GameColor::Black => ScrollOption::Black,
        GameColor::White => ScrollOption::White,
    };

    make_scroll_list(
        buttons.unwrap(),
        num_pens + 1,
        index,
        title,
        scroll_option,
        cont_style,
    )
}
