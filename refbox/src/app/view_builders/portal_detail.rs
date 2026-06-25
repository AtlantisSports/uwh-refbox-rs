use super::*;
use crate::portal_manager::DetailRow;
use collect_array::CollectArrayResult;
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{button, column, container, horizontal_space, row, text},
};

/// Maximum number of detail-page rows visible at once before scroll
/// arrows become active. Matches the Manage Remotes page's list size.
const PORTAL_DETAIL_LIST_LEN: usize = 4;

/// Render the portal detail page. The scrollable list lives in the
/// left 5/6 of the page, with scroll arrows down the right side
/// (matching the Manage Remotes layout). A single red 1/3-width BACK
/// button anchors the bottom-left, with the remaining two thirds of
/// the bottom row left blank.
///
/// Rows produced by `PortalManager::detail_rows()` come in
/// fixed order: the token-expired row first (if present), then stuck
/// items (oldest first), then young pending items (oldest first),
/// then recent successes (newest first, capped at RECENT_SUCCESS_CAP).
pub(in super::super) fn build_portal_detail_page<'a>(
    data: ViewData<'_, '_>,
    rows: Vec<DetailRow>,
    scroll_index: usize,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let title = text(fl!(
        "portal-summary-title",
        portal = portal_name_for_mode(mode)
    ))
    .height(Length::Fill)
    .width(Length::Fill)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .size(MEDIUM_TEXT);

    let num_items = rows.len();

    // RETRY ALL is actionable only when there is at least one unsent
    // game (a stuck/red or pending/yellow row). Recent-success and
    // token-expired rows don't count.
    let has_unsent = rows
        .iter()
        .any(|r| matches!(r, DetailRow::Stuck { .. } | DetailRow::Pending { .. }));

    let row_buttons: CollectArrayResult<_, PORTAL_DETAIL_LIST_LEN> = rows
        .into_iter()
        .skip(scroll_index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(PORTAL_DETAIL_LIST_LEN)
        .map(|slot| match slot {
            Some(row_data) => render_row(row_data),
            None => container(horizontal_space())
                .width(Length::Fill)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(disabled_container)
                .into(),
        })
        .collect();

    let list = make_scroll_list(
        row_buttons.unwrap(),
        num_items,
        scroll_index,
        title,
        ScrollOption::PortalDetail,
        light_gray_container,
    )
    .height(Length::Fill)
    .width(Length::FillPortion(5));

    let back = make_button(fl!("back"))
        .on_press(Message::ClosePortalDetailPage)
        .style(red_button);

    // Blue safe-action button, anchored bottom-right opposite BACK.
    // Grayed (no on_press) when there is nothing unsent to retry.
    let retry_all = make_button(fl!("portal-retry-all"))
        .on_press_maybe(has_unsent.then_some(Message::PortalRetryAll))
        .style(blue_button);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None,
        ),
        list,
        row![back, horizontal_space(), retry_all,]
            .spacing(SPACING)
            .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Build a row's text with the centering pattern that avoids iced 0.13's
/// stale paragraph-position cache. See commit 8a8d018 — pairing
/// `align_y(Center)` with `height(Fill)` on a `text` widget caches an
/// anchor that bleeds across renders, so we wrap the text in a
/// container whose `center(Length::Fill)` does the centering instead.
fn row_text_centered<'a>(label: String) -> Element<'a, Message> {
    container(
        text(label)
            .size(SMALL_PLUS_TEXT)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
    )
    .center(Length::Fill)
    .into()
}

fn render_row<'a>(r: DetailRow) -> Element<'a, Message> {
    match r {
        DetailRow::TokenExpired => button(row_text_centered(fl!("portal-row-token-expired")))
            .on_press(Message::EditGameConfigPage(ConfigPage::Game))
            .style(red_button)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .into(),
        DetailRow::Stuck {
            id, game_number, ..
        } => button(row_text_centered(fl!(
            "portal-row-stuck",
            game = game_number
        )))
        .on_press(Message::PortalRowTapped(id))
        .style(red_button)
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .into(),
        DetailRow::Pending {
            id,
            game_number,
            attempts,
        } => {
            // Concatenate the localized "(attempt N)" suffix onto the
            // base pending label. Kept as a separate translation key so
            // RTL/CJK locales can reposition it without rewriting the
            // base "not sent, tap to retry" message.
            let label = format!(
                "{} {}",
                fl!("portal-row-pending", game = game_number),
                fl!("portal-row-attempt-suffix", attempts = attempts),
            );
            button(row_text_centered(label))
                .on_press(Message::PortalRowTapped(id))
                .style(yellow_button)
                .padding(PADDING)
                .width(Length::Fill)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .into()
        }
        DetailRow::RecentSuccess {
            game_number,
            submitted_mins_ago,
            ..
        } => container(row_text_centered(fl!(
            "portal-row-recent",
            game = game_number,
            mins = submitted_mins_ago
        )))
        .style(green_container)
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .into(),
    }
}
