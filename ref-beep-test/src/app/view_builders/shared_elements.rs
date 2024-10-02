use super::super::{
    super::snapshot::BeepTestSnapshot,
    message::*,
    style::{
        Button, ButtonStyle, Row, Text, TextStyle, LARGE_TEXT, LINE_HEIGHT, MIN_BUTTON_SIZE,
        PADDING, SPACING,
    },
};
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{button, column, row, text},
    Alignment, Length,
};
use matrix_drawing::secs_to_long_time_string;

macro_rules! column {
    () => (
        iced::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Column::with_children(vec![$($crate::app::Element::from($x)),+])
    );
}

macro_rules! row {
    () => (
        iced::widget::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Row::with_children(vec![$($crate::app::Element::from($x)),+])
    );
}

pub(super) fn make_time_button<'a>(snapshot: &BeepTestSnapshot) -> Row<'a, Message> {
    macro_rules! make_time_view {
        ($base:ident, $time_text:ident) => {
            $base
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .push($time_text)
        };
    }

    let button_height = Length::Fixed(MIN_BUTTON_SIZE);
    let button_style = ButtonStyle::Gray;

    let make_time_view_col = |time, style| {
        let time = text(time)
            .line_height(LINE_HEIGHT)
            .style(style)
            .size(LARGE_TEXT);
        let r = column![].spacing(SPACING);
        make_time_view!(r, time)
    };

    let color = TextStyle::Yellow;

    let time_text = secs_to_long_time_string(snapshot.secs_in_period);
    let time_text = time_text.trim();

    let mut content = row![]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center);

    content = content.push(make_time_view_col(time_text, color));

    let time_button = button(content)
        .width(Length::Fill)
        .height(button_height)
        .style(button_style)
        .padding(PADDING);

    let time_row = row![time_button]
        .height(button_height)
        .width(Length::Fill)
        .spacing(SPACING);

    time_row
}

pub(super) fn make_button<'a, Message: Clone, T: ToString>(label: T) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub fn centered_text<'a, T: ToString>(label: T) -> Text<'a> {
    text(label)
        .line_height(LINE_HEIGHT)
        .vertical_alignment(Vertical::Center)
        .horizontal_alignment(Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}
