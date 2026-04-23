use super::*;
use crate::portal_manager::ItemId;
use iced::{
    Alignment, Element, Length, Theme,
    alignment::Horizontal,
    widget::{
        button::{Status, Style},
        column, container, row, text, vertical_space,
    },
};

/// Render the attention-action page for a single stuck queued item.
///
/// The portal API collapses all failure causes (409 conflict, 401 token
/// expiry, 5xx, network) into a single opaque error — see the ADR 011
/// amendment dated 2026-04-21 — so the operator is offered only two
/// actions: RETRY THIS GAME RESULT (resubmit with force=true) and
/// DISCARD THIS GAME RESULT (two-tap confirmation). BACK returns to the
/// portal detail page (the list the operator came from), via
/// `Message::ClosePortalAttentionAction` — not to the main application
/// screen.
///
/// Layout:
/// - Game time banner at top
/// - Title (MEDIUM_TEXT)
/// - Three-line informational note: problem, stored score, remediation
/// - Button row: BACK (red) | Discard (yellow → red on confirm) | Retry (green)
pub(in super::super) fn build_portal_attention_action<'a>(
    data: ViewData<'_, '_>,
    id: ItemId,
    game_number: String,
    black_score: u8,
    white_score: u8,
    discard_armed: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let banner = make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
    );

    let title = text(fl!("portal-page-title-attention", game = game_number))
        .size(MEDIUM_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);

    let note = container(
        column![
            text(fl!("portal-page-attention-info")).size(SMALL_PLUS_TEXT),
            text(fl!(
                "portal-page-attention-score",
                white = white_score,
                black = black_score
            ))
            .size(SMALL_PLUS_TEXT),
            text(fl!("portal-page-attention-remediation")).size(SMALL_PLUS_TEXT),
        ]
        .spacing(SPACING)
        .width(Length::Fill),
    )
    .style(light_gray_container)
    .padding(PADDING)
    .width(Length::Fill);

    // Discard starts yellow (caution: irreversible local delete) and
    // turns red once armed (second tap confirms the discard).
    let discard_label = if discard_armed {
        fl!("portal-action-discard-confirm")
    } else {
        fl!("portal-action-discard")
    };
    let discard_style: fn(&Theme, Status) -> Style = if discard_armed {
        red_button
    } else {
        yellow_button
    };
    let discard = make_button(discard_label)
        .on_press(Message::PortalDiscardTapped(id.clone()))
        .style(discard_style);

    let retry = make_button(fl!("portal-action-force-submit"))
        .on_press(Message::PortalForceSubmit(id))
        .style(green_button);

    let back = make_button(fl!("back"))
        .on_press(Message::ClosePortalAttentionAction)
        .style(red_button);

    let button_row = row![back, discard, retry,]
        .spacing(SPACING)
        .width(Length::Fill);

    column![banner, title, note, vertical_space(), button_row,]
        .spacing(SPACING)
        .padding(PADDING)
        .align_x(Alignment::Center)
        .height(Length::Fill)
        .into()
}
