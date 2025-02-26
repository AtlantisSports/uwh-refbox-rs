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

pub(in super::super) fn build_foul_overview_page<'a>(
    snapshot: &GameSnapshot,
    warnings: OptColorBundle<Vec<PrintableInfractionSummary>>,
    indices: OptColorBundle<usize>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            make_foul_list(
                warnings.black.into_iter().rev().collect(),
                indices.black,
                Some(GameColor::Black)
            ),
            make_foul_list(
                warnings.equal.into_iter().rev().collect(),
                indices.equal,
                None
            ),
            make_foul_list(
                warnings.white.into_iter().rev().collect(),
                indices.white,
                Some(GameColor::White)
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::FoulOverviewComplete { canceled: true }),
            make_button("NEW")
                .style(ButtonStyle::Blue)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                    origin: None,
                    color: None,
                    infraction: Infraction::Unknown,
                    ret_to_overview: true,
                })),
            make_button("DONE")
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::FoulOverviewComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_foul_list<'a>(
    fouls: Vec<PrintableInfractionSummary>,
    index: usize,
    color: Option<GameColor>,
) -> Container<'a, Message> {
    const FOUL_LIST_LEN: usize = 3;

    let title = match color {
        Some(color) => color.to_string().to_uppercase(),
        None => "EQUAL".to_string(),
    };

    let title = text(title)
        .line_height(LINE_HEIGHT)
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

    let num_pens = fouls.len();

    let buttons: CollectArrayResult<_, FOUL_LIST_LEN> = fouls
        .into_iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(FOUL_LIST_LEN)
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
                    .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                        origin: Some((color, i)),
                        color,
                        infraction: details.infraction,
                        ret_to_overview: true,
                    }))
                    .into()
            } else {
                button(horizontal_space(Length::Shrink))
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(ButtonStyle::Gray)
                    .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                        origin: None,
                        color,
                        infraction: Infraction::Unknown,
                        ret_to_overview: true,
                    }))
                    .into()
            }
        })
        .collect();

    let cont_style = match color {
        Some(GameColor::Black) => ContainerStyle::Black,
        Some(GameColor::White) => ContainerStyle::White,
        None => ContainerStyle::Blue,
    };

    let scroll_option = match color {
        Some(GameColor::Black) => ScrollOption::Black,
        Some(GameColor::White) => ScrollOption::White,
        None => ScrollOption::Equal,
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
