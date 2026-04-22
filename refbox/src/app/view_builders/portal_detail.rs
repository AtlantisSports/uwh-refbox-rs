use super::*;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

/// Scaffolding view for the portal detail page. Task 13 ships this as
/// a minimal title + BACK button; the row list, row ordering, and
/// per-row action buttons land in Task 14.
pub(in super::super) fn build_portal_detail_page<'a>(
    data: ViewData<'_, '_>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let title = text("PORTAL").size(MEDIUM_TEXT);
    let back = button(text(fl!("back")).size(SMALL_PLUS_TEXT))
        .on_press(Message::ClosePortalDetailPage)
        .padding(PADDING)
        .style(red_button);

    let list_area = container(title)
        .width(Length::FillPortion(4))
        .height(Length::Fill)
        .padding(PADDING);

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
