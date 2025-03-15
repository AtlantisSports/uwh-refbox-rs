use super::{
    super::{
        super::{config::Level, snapshot::BeepTestSnapshot},
        message::*,
    },
    *,
};
use iced::{
    Alignment, Length,
    alignment::{Horizontal, Vertical},
    widget::{
        Button, Column, Container, Row, Text, button, column, container, horizontal_space, row,
        text,
    },
};
use matrix_drawing::secs_to_long_time_string;
use std::fmt::Write;

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
        iced::widget::Row::with_children(vec![$(iced::Element::from($x)),+])
    );
}

pub(super) fn make_time_button<'a>(snapshot: &BeepTestSnapshot) -> Row<'a, Message> {
    macro_rules! make_time_view {
        ($base:ident, $time_text:ident) => {
            $base
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .push($time_text)
        };
    }

    let button_height = Length::Fixed(MIN_BUTTON_SIZE);

    let make_time_view_col = |time, style| {
        let time = text(time).style(style).size(LARGE_TEXT);
        let r = column![].spacing(SPACING);
        make_time_view!(r, time)
    };

    let time_text = secs_to_long_time_string(snapshot.secs_in_period);
    let time_text = time_text.trim().to_owned();

    let mut content = row![]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_y(Alignment::Center);

    content = content.push(make_time_view_col(time_text, yellow_text));

    let time_button = button(content)
        .width(Length::Fill)
        .height(button_height)
        .style(gray_button)
        .padding(PADDING);

    let time_row = row![time_button]
        .height(button_height)
        .width(Length::Fill)
        .spacing(SPACING);

    time_row
}

pub(super) fn make_info_container<'a>(snapshot: &BeepTestSnapshot) -> Container<'a, Message> {
    let boxheight: f32 = 385.0;
    let info = container(
        text(config_string(snapshot))
            .size(SMALL_TEXT)
            .align_y(Vertical::Top)
            .align_x(Horizontal::Left),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(boxheight))
    .style(light_gray_container);

    info
}

pub(super) fn make_button<'a, Message: Clone>(
    label: impl text::IntoFragment<'a>,
) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub fn centered_text<'a>(label: impl text::IntoFragment<'a>) -> Text<'a> {
    text(label)
        .align_y(Vertical::Center)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

pub(super) fn make_value_button<'a, Message: 'a + Clone>(
    first_label: impl text::IntoFragment<'a>,
    second_label: impl text::IntoFragment<'a>,
    large_text: (bool, bool),
    message: Option<Message>,
) -> Button<'a, Message> {
    let mut button = button(
        row![
            text(first_label)
                .size(if large_text.0 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .height(Length::Fill)
                .align_y(Vertical::Center),
            horizontal_space(),
            text(second_label)
                .size(if large_text.1 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .height(Length::Fill)
                .align_y(Vertical::Center),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center)
        .padding(PADDING),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .style(light_gray_button);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

pub(super) fn bool_string(val: bool) -> String {
    match val {
        true => "YES".to_string(),
        false => "NO".to_string(),
    }
}

pub(super) fn config_string(snapshot: &BeepTestSnapshot) -> String {
    let mut result = String::new();

    writeln!(&mut result, "Lap: {}", snapshot.lap_count).unwrap();

    writeln!(
        &mut result,
        "Lap Duration: {}",
        snapshot.total_time_in_period
    )
    .unwrap();

    writeln!(&mut result, "Current Period: {}", snapshot.current_period).unwrap();

    writeln!(&mut result).unwrap();

    writeln!(
        &mut result,
        "Next Lap Duration: {}",
        snapshot.time_in_next_period
    )
    .unwrap();

    result
}

// This function puts each value in a container of the background color (table), and then puts all the
// rows and columns on a black container, giving it a grid (chart)
// second_chart is for cases when making a second chart if there are more levels than fit in one
pub fn build_levels_table(levels: &[Level], second_chart: bool) -> Container<Message> {
    pub const CHART_PADDING: f32 = 2.0;

    let mut table = Column::new().spacing(CHART_PADDING).padding(0);

    let headers: Row<Message> = Row::new()
        .spacing(CHART_PADDING)
        .push(
            Container::new(
                Text::new("Level")
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .align_x(Horizontal::Center),
            )
            .style(square_light_gray_container)
            .padding(CHART_PADDING),
        )
        .spacing(CHART_PADDING)
        .push(
            Container::new(
                Text::new("Count")
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .align_x(Horizontal::Center),
            )
            .style(square_light_gray_container)
            .padding(CHART_PADDING),
        )
        .spacing(CHART_PADDING)
        .push(
            Container::new(
                Text::new("Duration")
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .align_x(Horizontal::Center),
            )
            .style(square_light_gray_container)
            .padding(CHART_PADDING),
        )
        .spacing(CHART_PADDING);

    table = table.push(headers).spacing(CHART_PADDING);

    for (index, level) in levels.iter().enumerate() {
        let level_row: Row<Message> = Row::new()
            .spacing(7)
            .push(
                Container::new(
                    Text::new(if second_chart {
                        format!("{}", index + 14)
                    } else {
                        format!("{}", index + 1)
                    })
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .align_x(Horizontal::Center),
                )
                .style(square_light_gray_container)
                .padding(CHART_PADDING),
            )
            .spacing(CHART_PADDING)
            .push(
                Container::new(
                    Text::new(format!("{}", level.count))
                        .width(Length::Fixed(69.0))
                        .size(16)
                        .align_x(Horizontal::Center),
                )
                .style(square_light_gray_container)
                .padding(CHART_PADDING),
            )
            .spacing(CHART_PADDING)
            .push(
                Container::new(
                    Text::new(format!("{:?}", level.duration))
                        .width(Length::Fixed(69.0))
                        .size(16)
                        .align_x(Horizontal::Center),
                )
                .style(square_light_gray_container)
                .padding(CHART_PADDING),
            )
            .spacing(CHART_PADDING);

        table = table.push(level_row).spacing(CHART_PADDING);
    }

    let chart: Container<Message> = Container::new(table)
        .style(square_black_container)
        .padding(CHART_PADDING);

    chart
}
