use super::*;
use crate::portal_manager::ItemId;
use iced::{
    Element, Length,
    widget::{Space, button, column, text},
};

/// Render the attention-action page for a single stuck queued item.
///
/// The portal API collapses all failure causes (409 conflict, 401 token
/// expiry, 5xx, network) into a single opaque error — see the ADR 011
/// amendment dated 2026-04-21 — so the operator is offered only two
/// actions: FORCE THIS GAME RESULT (resubmit with force=true) and
/// DISCARD THIS SUBMISSION (two-tap confirmation). BACK returns to the
/// portal detail page (the list the operator came from), via
/// `Message::ClosePortalAttentionAction` — not to the main application
/// screen.
pub(in super::super) fn build_portal_attention_action<'a>(
    data: ViewData<'_, '_>,
    id: ItemId,
    game_number: String,
    black_score: u8,
    white_score: u8,
    attempts: u32,
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

    let title = text(fl!("portal-page-title-attention", game = game_number)).size(MEDIUM_TEXT);
    let info = text(fl!(
        "portal-page-attention-info",
        attempts = attempts,
        black = black_score,
        white = white_score
    ))
    .size(SMALL_PLUS_TEXT);

    let force = button(text(fl!("portal-action-force-submit")).size(SMALL_PLUS_TEXT))
        .on_press(Message::PortalForceSubmit(id.clone()))
        .style(green_button)
        .padding(PADDING)
        .width(Length::Fill);

    let discard: Element<'a, Message> = if discard_armed {
        button(text(fl!("portal-action-discard-confirm")).size(SMALL_PLUS_TEXT))
            .on_press(Message::PortalDiscardTapped(id.clone()))
            .style(yellow_button)
            .padding(PADDING)
            .width(Length::Fill)
            .into()
    } else {
        button(text(fl!("portal-action-discard")).size(SMALL_PLUS_TEXT))
            .on_press(Message::PortalDiscardTapped(id.clone()))
            .style(red_button)
            .padding(PADDING)
            .width(Length::Fill)
            .into()
    };

    let back = button(text(fl!("back")).size(SMALL_PLUS_TEXT))
        .on_press(Message::ClosePortalAttentionAction)
        .style(gray_button)
        .padding(PADDING)
        .width(Length::Fill);

    column![
        banner,
        title,
        info,
        force,
        discard,
        Space::new(Length::Fill, Length::Fill),
        back,
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .height(Length::Fill)
    .into()
}
