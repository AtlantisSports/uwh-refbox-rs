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

use uwh_common::game_snapshot::{Color as GameColor, GameSnapshot};

pub(in super::super) fn build_warning_overview_page<'a>(
    snapshot: &GameSnapshot,
    warnings: BlackWhiteBundle<Vec<PrintableInfractionSummary>>,
    indices: BlackWhiteBundle<usize>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
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
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::WarningOverviewComplete { canceled: true }),
            make_button("NEW")
                .style(ButtonStyle::Blue)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::WarningAdd {
                    origin: None,
                    color: GameColor::Black,
                    infraction: Infraction::Unknown,
                    team_warning: false,
                    ret_to_overview: true,
                })),
            make_button("DONE")
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

    let title = text(format!("{} WARNINGS", color.to_string().to_uppercase()))
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
                let mut text = text(details.text)
                    .line_height(LINE_HEIGHT)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Left)
                    .width(Length::Fill);

                match details.hint {
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
                        team_warning: details.team,
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
