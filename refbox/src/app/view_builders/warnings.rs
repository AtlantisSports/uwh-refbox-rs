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
    Length,
    alignment::{Horizontal, Vertical},
    widget::{button, column, horizontal_space, row, text},
};

use uwh_common::color::Color as GameColor;

pub(in super::super) fn build_warning_overview_page<'a>(
    data: ViewData<'_, '_>,
    warnings: BlackWhiteBundle<Vec<PrintableInfractionSummary>>,
    indices: BlackWhiteBundle<usize>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            make_warning_list(
                warnings.black.into_iter().rev().collect(),
                indices.black,
                GameColor::Black
            ),
            make_warning_list(
                warnings.white.into_iter().rev().collect(),
                indices.white,
                GameColor::White
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_button(fl!("cancel"))
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::WarningOverviewComplete { canceled: true }),
            make_button(fl!("new"))
                .style(ButtonStyle::Blue)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::WarningAdd {
                    origin: None,
                    color: GameColor::Black,
                    infraction: Infraction::Unknown,
                    team_warning: false,
                    ret_to_overview: true,
                })),
            make_button(fl!("done"))
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::WarningOverviewComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_warning_list<'a>(
    warnings: Vec<PrintableInfractionSummary>,
    index: usize,
    color: GameColor,
) -> Container<'a, Message> {
    const WARNING_LIST_LEN: usize = 3;

    let color_text = match color {
        GameColor::Black => fl!("black-warnings"),
        GameColor::White => fl!("white-warnings"),
    };

    let title = text(color_text.to_string().to_uppercase())
        .line_height(LINE_HEIGHT)
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

    let num_pens = warnings.len();

    let buttons: CollectArrayResult<_, WARNING_LIST_LEN> = warnings
        .into_iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(WARNING_LIST_LEN)
        .map(|foul| {
            if let Some((i, details)) = foul {
                let printable = fl!(
                    "warning",
                    player_number = details
                        .player_number
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| String::from("none")),
                    infraction = inf_short_name(details.infraction)
                );

                let mut text = text(printable)
                    .line_height(LINE_HEIGHT)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Left)
                    .width(Length::Fill);

                match details.format_hint {
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
                    .on_press(Message::KeypadPage(KeypadPage::WarningAdd {
                        origin: Some((color, i)),
                        color,
                        infraction: details.infraction,
                        team_warning: details.player_number.is_none(),
                        ret_to_overview: true,
                    }))
                    .into()
            } else {
                button(horizontal_space(Length::Shrink))
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(ButtonStyle::Gray)
                    .on_press(Message::KeypadPage(KeypadPage::WarningAdd {
                        origin: None,
                        color,
                        infraction: Infraction::Unknown,
                        team_warning: false,
                        ret_to_overview: true,
                    }))
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
