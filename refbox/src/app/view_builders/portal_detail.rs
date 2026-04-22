use super::*;
use crate::portal_manager::{DetailRow, HealthState};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

/// Render the portal detail page. The summary banner at the top of the
/// list reflects the current overall health; the row list below it is
/// produced by `PortalManager::detail_rows()` and contains (in order):
/// the token-expired banner if present, stuck items, young pending
/// items, and recent successes.
pub(in super::super) fn build_portal_detail_page<'a>(
    data: ViewData<'_, '_>,
    rows: Vec<DetailRow>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let summary_text = match portal_indicator.health {
        HealthState::Green => fl!("portal-summary-connected"),
        HealthState::Yellow => fl!("portal-summary-checking"),
        HealthState::Red => fl!("portal-summary-issues"),
    };
    let title_row = row![text(summary_text).size(SMALL_PLUS_TEXT)].spacing(SPACING);

    let mut rows_col = column![].spacing(SPACING);
    for row_data in rows {
        rows_col = rows_col.push(render_row(row_data));
    }

    let list_area = container(column![title_row, rows_col].spacing(SPACING))
        .width(Length::FillPortion(4))
        .height(Length::Fill)
        .padding(PADDING)
        .style(light_gray_container);

    let back = button(text(fl!("back")).size(SMALL_PLUS_TEXT))
        .on_press(Message::ClosePortalDetailPage)
        .padding(PADDING)
        .style(red_button);

    let side = column![Space::new(Length::Fill, Length::Fill), back]
        .align_x(Alignment::Center)
        .width(Length::FillPortion(1));

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator
        ),
        row![list_area, side].spacing(SPACING).height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn render_row<'a>(r: DetailRow) -> Element<'a, Message> {
    match r {
        DetailRow::TokenExpired => {
            button(text(fl!("portal-row-token-expired")).size(SMALL_PLUS_TEXT))
                .on_press(Message::OpenPortalTokenExpiredAction)
                .style(red_button)
                .padding(PADDING)
                .width(Length::Fill)
                .into()
        }
        DetailRow::Stuck {
            id,
            game_number,
            attempts,
        } => button(
            text(fl!(
                "portal-row-stuck",
                game = game_number,
                attempts = attempts
            ))
            .size(SMALL_PLUS_TEXT),
        )
        .on_press(Message::PortalRowTapped(id))
        .style(red_button)
        .padding(PADDING)
        .width(Length::Fill)
        .into(),
        DetailRow::Pending {
            id,
            game_number,
            attempts,
            retry_in_secs,
            stats_only,
        } => {
            let label = match (retry_in_secs, stats_only) {
                (Some(secs), false) => fl!(
                    "portal-row-pending",
                    game = game_number,
                    attempts = attempts,
                    secs = format!("{secs:02}")
                ),
                (None, _) => fl!(
                    "portal-row-pending-tap",
                    game = game_number,
                    attempts = attempts
                ),
                (Some(secs), true) => fl!(
                    "portal-row-pending-stats-only",
                    game = game_number,
                    secs = format!("{secs:02}")
                ),
            };
            button(text(label).size(SMALL_PLUS_TEXT))
                .on_press(Message::PortalRowTapped(id))
                .style(yellow_button)
                .padding(PADDING)
                .width(Length::Fill)
                .into()
        }
        DetailRow::RecentSuccess {
            game_number,
            submitted_mins_ago,
            ..
        } => button(
            text(fl!(
                "portal-row-recent",
                game = game_number,
                mins = submitted_mins_ago
            ))
            .size(SMALL_PLUS_TEXT),
        )
        .style(green_button)
        .padding(PADDING)
        .width(Length::Fill)
        .into(),
    }
}
