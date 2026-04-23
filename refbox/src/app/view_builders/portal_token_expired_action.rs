use super::*;
use iced::{
    Element, Length,
    widget::{Space, button, column, text},
};

/// Render the token-expired action page. Reached by tapping the
/// "Portal login expired" row on the detail page. Explains the
/// situation and offers two actions: GO TO LOGIN (opens the existing
/// portal login keypad; on success the app returns here via the
/// detail page) and BACK (returns to the detail page via
/// `Message::ClosePortalTokenExpiredAction`).
pub(in super::super) fn build_portal_token_expired_action<'a>(
    data: ViewData<'_, '_>,
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

    let title = text(fl!("portal-page-title-token-expired")).size(MEDIUM_TEXT);
    let body = text(fl!(
        "portal-page-body-token-expired",
        portal = portal_name_for_mode(mode)
    ))
    .size(SMALL_PLUS_TEXT);

    let login = button(text(fl!("portal-action-go-to-login")).size(SMALL_PLUS_TEXT))
        .on_press(Message::PortalGoToLogin)
        .style(blue_button)
        .padding(PADDING)
        .width(Length::Fill);

    let back = button(text(fl!("back")).size(SMALL_PLUS_TEXT))
        .on_press(Message::ClosePortalTokenExpiredAction)
        .style(gray_button)
        .padding(PADDING)
        .width(Length::Fill);

    column![
        banner,
        title,
        body,
        login,
        Space::new(Length::Fill, Length::Fill),
        back,
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .height(Length::Fill)
    .into()
}
