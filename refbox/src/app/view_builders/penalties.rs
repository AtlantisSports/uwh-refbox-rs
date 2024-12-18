use super::*;
use collect_array::CollectArrayResult;
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Container, Space, button, column, row, text},
};

use uwh_common::color::Color as GameColor;

pub(in super::super) fn build_penalty_overview_page<'a>(
    data: ViewData<'_, '_>,
    penalties: BlackWhiteBundle<Vec<PrintablePenaltySummary>>,
    indices: BlackWhiteBundle<usize>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

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
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::PenaltyOverviewComplete { canceled: true }),
            make_button(fl!("new"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::Penalty(
                    None,
                    GameColor::Black,
                    default_pen_len,
                    Infraction::Unknown,
                ))),
            make_button(fl!("done"))
                .style(green_button)
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
    penalties: Vec<PrintablePenaltySummary>,
    index: usize,
    color: GameColor,
    default_pen_len: PenaltyKind,
) -> Container<'a, Message> {
    const PENALTY_LIST_LEN: usize = 3;

    let color_text = match color {
        GameColor::Black => fl!("black-penalties"),
        GameColor::White => fl!("white-penalties"),
    };

    let title = text(color_text.to_string().to_uppercase())
        .line_height(LINE_HEIGHT)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    let num_pens = penalties.len();

    let buttons: CollectArrayResult<_, PENALTY_LIST_LEN> = penalties
        .into_iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(PENALTY_LIST_LEN)
        .map(|pen| {
            if let Some((i, details)) = pen {
                let printable = fl!(
                    "penalty",
                    player_number = details.player_number,
                    time = details.time.fluent(),
                    kind = details.kind.fluent()
                );
                let mut text = text(printable)
                    .line_height(LINE_HEIGHT)
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Left)
                    .width(Length::Fill);

                match details.format_hint {
                    FormatHint::NoChange => {}
                    FormatHint::Edited => text = text.style(orange_text),
                    FormatHint::Deleted => text = text.style(red_text),
                    FormatHint::New => text = text.style(green_text),
                }

                button(text)
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(gray_button)
                    .on_press(Message::KeypadPage(KeypadPage::Penalty(
                        Some((color, i)),
                        color,
                        details.kind,
                        details.infraction,
                    )))
                    .into()
            } else {
                button(Space::with_width(Length::Shrink))
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(gray_button)
                    .on_press(Message::KeypadPage(KeypadPage::Penalty(
                        None,
                        color,
                        default_pen_len,
                        Infraction::Unknown,
                    )))
                    .into()
            }
        })
        .collect();

    let cont_style = match color {
        GameColor::Black => black_container,
        GameColor::White => white_container,
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
