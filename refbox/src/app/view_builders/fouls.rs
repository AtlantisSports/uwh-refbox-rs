use super::*;
use collect_array::CollectArrayResult;
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Container, Space, button, column, row, text},
};

use uwh_common::color::Color as GameColor;

pub(in super::super) fn build_foul_overview_page<'a>(
    data: ViewData<'_, '_>,
    warnings: OptColorBundle<Vec<PrintableInfractionSummary>>,
    indices: OptColorBundle<usize>,
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
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::FoulOverviewComplete { canceled: true }),
            make_button(fl!("new"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                    origin: None,
                    color: None,
                    infraction: Infraction::Unknown,
                    ret_to_overview: true,
                })),
            make_button(fl!("done"))
                .style(green_button)
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
        Some(GameColor::Black) => fl!("dark-team-name-caps"),
        Some(GameColor::White) => fl!("light-team-name-caps"),
        None => fl!("equal"),
    };

    let title = text(title)
        .line_height(LINE_HEIGHT)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

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
                let printable = fl!(
                    "foul",
                    player_number = details
                        .player_number
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| String::from("none")),
                    infraction = inf_short_name(details.infraction)
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
                    .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                        origin: Some((color, i)),
                        color,
                        infraction: details.infraction,
                        ret_to_overview: true,
                    }))
                    .into()
            } else {
                button(Space::with_width(Length::Shrink))
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .width(Length::Fill)
                    .style(gray_button)
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
        Some(GameColor::Black) => black_container,
        Some(GameColor::White) => white_container,
        None => blue_container,
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
