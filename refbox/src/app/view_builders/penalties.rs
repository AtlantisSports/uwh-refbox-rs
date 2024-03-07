use super::{
    style::{
        ButtonStyle, Container, ContainerStyle, Element, LINE_HEIGHT, MIN_BUTTON_SIZE, PADDING,
        SPACING,
    },
    *,
};
use crate::app::style::TextStyle;
use collect_array::CollectArrayResult;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{button, column, horizontal_space, row, text},
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
    let default_pen_len = match mode {
        Mode::Hockey3V3 => PenaltyKind::ThirtySecond,
        Mode::Hockey6V6 => PenaltyKind::OneMinute,
        Mode::Rugby => PenaltyKind::TwoMinute,
    };

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            make_penalty_list(
                penalties.black,
                indices.black,
                GameColor::Black,
                default_pen_len
            ),
            make_penalty_list(
                penalties.white,
                indices.white,
                GameColor::White,
                default_pen_len
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::PenaltyOverviewComplete { canceled: true }),
            make_button("NEW")
                .style(ButtonStyle::Blue)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::Penalty(
                    None,
                    GameColor::Black,
                    default_pen_len,
                ))),
            make_button("DONE")
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::PenaltyOverviewComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_penalty_list<'a>(
    penalties: Vec<(String, FormatHint, PenaltyKind)>,
    index: usize,
    color: GameColor,
    default_pen_len: PenaltyKind,
) -> Container<'a, Message> {
    const PENALTY_LIST_LEN: usize = 3;

    let title = text(format!("{} PENALTIES", color.to_string().to_uppercase()))
        .line_height(LINE_HEIGHT)
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
                    .line_height(LINE_HEIGHT)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Left)
                    .width(Length::Fill);

                match format {
                    FormatHint::NoChange => {}
                    FormatHint::Edited => text = text.style(TextStyle::Orange),
                    FormatHint::Deleted => text = text.style(TextStyle::Red),
                    FormatHint::New => text = text.style(TextStyle::Green),
                }

                button(text)
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(ButtonStyle::Gray)
                    .on_press(Message::KeypadPage(KeypadPage::Penalty(
                        Some((color, i)),
                        color,
                        kind,
                    )))
                    .into()
            } else {
                button(horizontal_space(Length::Shrink))
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(ButtonStyle::Gray)
                    .on_press(Message::KeypadPage(KeypadPage::Penalty(
                        None,
                        color,
                        default_pen_len,
                    )))
                    .into()
            }
        })
        .collect();

    let cont_style = match color {
        GameColor::Black => ContainerStyle::Black,
        GameColor::White => ContainerStyle::White,
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
