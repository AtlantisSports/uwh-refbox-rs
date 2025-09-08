use super::*;
use iced::{
    Length,
    widget::{Space, column, row},
};

// 7-row layout: rows 1-6 are empty spacers; row 7 holds a right-half button.
pub(in super::super) fn build_beep_test_home_page<'a>() -> Element<'a, Message> {
    let bottom_row = row![
        // left half empty
        Space::with_width(Length::FillPortion(1)),
        // right half: button fills this half only
        make_button("RETURN TO REFBOX")
            .style(blue_button)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::ReturnToRefBox)
    ]
    .spacing(SPACING)
    .width(Length::Fill)
    .height(Length::FillPortion(1));

    column![
        // Rows 1..6 are empty to match overall spacing
        Space::with_height(Length::FillPortion(1)),
        Space::with_height(Length::FillPortion(1)),
        Space::with_height(Length::FillPortion(1)),
        Space::with_height(Length::FillPortion(1)),
        Space::with_height(Length::FillPortion(1)),
        Space::with_height(Length::FillPortion(1)),
        // Row 7: button row
        bottom_row,
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
