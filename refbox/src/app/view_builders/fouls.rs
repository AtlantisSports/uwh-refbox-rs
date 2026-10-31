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
        portal_indicator,
        ..
    } = data;

    let has_changes = any_pending_change(
        warnings
            .black
            .iter()
            .chain(warnings.equal.iter())
            .chain(warnings.white.iter())
            .map(|w| w.format_hint),
    );

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row![
            make_foul_list(warnings.black, indices.black, Some(GameColor::Black)),
            make_foul_list(warnings.equal, indices.equal, None),
            make_foul_list(warnings.white, indices.white, Some(GameColor::White))
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_chrome_button(cancel_or_back_label(has_changes))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::FoulOverviewComplete { canceled: true }),
            make_chrome_button(fl!("new"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                    origin: None,
                    color: None,
                    infraction: Infraction::Unknown,
                    ret_to_overview: true,
                })),
            make_chrome_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    has_changes.then_some(Message::FoulOverviewComplete { canceled: false }),
                ),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// The `len` rows a list page shows from row `scroll` on, last-added entry
/// first, padded with `None`. Each row carries the entry's position in the
/// stored list (in the order entries were added; a team change re-adds at the
/// end), which is what an edit must be opened with: the on-screen row counts
/// from the last-added entry, so opening with it would edit, save over or
/// delete a different entry.
pub(super) fn visible_rows_newest_first<T>(
    items: Vec<T>,
    scroll: usize,
    len: usize,
) -> impl Iterator<Item = Option<(usize, T)>> {
    items
        .into_iter()
        .enumerate()
        .rev()
        .skip(scroll)
        .map(Some)
        .chain(std::iter::repeat_with(|| None))
        .take(len)
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
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    let num_pens = fouls.len();

    let rows = visible_rows_newest_first(fouls, index, FOUL_LIST_LEN);

    let buttons: CollectArrayResult<_, FOUL_LIST_LEN> = rows
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_row_opens_the_entry_it_shows() {
        // Stored oldest first: #11 was recorded before #6, so #6 is shown on
        // top and must open stored entry 1.
        let rows = |scroll| visible_rows_newest_first(vec!["#11", "#6"], scroll, 3).collect();
        let at_top: Vec<_> = rows(0);
        assert_eq!(at_top, vec![Some((1, "#6")), Some((0, "#11")), None]);
        let scrolled: Vec<_> = rows(1);
        assert_eq!(scrolled, vec![Some((0, "#11")), None, None]);
    }
}
