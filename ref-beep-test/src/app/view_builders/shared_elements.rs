use std::fmt::Write;

use crate::{
    app::style::{ApplicationTheme, ContainerStyle},
    config::{BeepTest, Level},
    snapshot::BeepTestSnapshot,
};

use super::super::{
    message::*,
    style::{
        Button, ButtonStyle, Container, Row, Text, TextStyle, LARGE_TEXT, LINE_HEIGHT, MEDIUM_TEXT,
        MIN_BUTTON_SIZE, PADDING, SMALL_TEXT, SPACING,
    },
};
use embedded_graphics::prelude::Size as SizeGraphics;
use embedded_graphics::primitives::CornerRadii;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        button, canvas::path::lyon_path::geom::Size, column, container, horizontal_space, row,
        text, Column,
    },
    Alignment, BorderRadius, Length,
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

pub(super) fn make_info_container<'a>(snapshot: &BeepTestSnapshot) -> Container<'a, Message> {
    let boxheight: f32 = 385.0;
    let info = container(
        text(config_string(snapshot))
            .size(SMALL_TEXT)
            .line_height(LINE_HEIGHT)
            .vertical_alignment(Vertical::Top)
            .horizontal_alignment(Horizontal::Left),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(boxheight))
    .style(ContainerStyle::LightGray);

    info
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

pub(super) fn make_value_button<'a, Message: 'a + Clone, T: ToString, U: ToString>(
    first_label: T,
    second_label: U,
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
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center),
            horizontal_space(Length::Fill),
            text(second_label)
                .size(if large_text.1 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center),
        ]
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .padding(PADDING),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .style(ButtonStyle::LightGray);

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

    let mut table: Column<Message, iced::Renderer<ApplicationTheme>> =
        Column::new().spacing(CHART_PADDING).padding(0);

    let headers: Row<Message> = Row::new()
        .spacing(CHART_PADDING)
        .push(
            Container::new(
                Text::new(format!("Level"))
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .horizontal_alignment(Horizontal::Center),
            )
            .style(ContainerStyle::SquareLightGray)
            .padding(CHART_PADDING),
        )
        .spacing(CHART_PADDING)
        .push(
            Container::new(
                Text::new(format!("Count"))
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .horizontal_alignment(Horizontal::Center),
            )
            .style(ContainerStyle::SquareLightGray)
            .padding(CHART_PADDING),
        )
        .spacing(CHART_PADDING)
        .push(
            Container::new(
                Text::new(format!("Duration"))
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .horizontal_alignment(Horizontal::Center),
            )
            .style(ContainerStyle::SquareLightGray)
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
                    .horizontal_alignment(Horizontal::Center),
                )
                .style(ContainerStyle::SquareLightGray)
                .padding(CHART_PADDING),
            )
            .spacing(CHART_PADDING)
            .push(
                Container::new(
                    Text::new(format!("{}", level.count))
                        .width(Length::Fixed(69.0))
                        .size(16)
                        .horizontal_alignment(Horizontal::Center),
                )
                .style(ContainerStyle::SquareLightGray)
                .padding(CHART_PADDING),
            )
            .spacing(CHART_PADDING)
            .push(
                Container::new(
                    Text::new(format!("{:?}", level.duration))
                        .width(Length::Fixed(69.0))
                        .size(16)
                        .horizontal_alignment(Horizontal::Center),
                )
                .style(ContainerStyle::SquareLightGray)
                .padding(CHART_PADDING),
            )
            .spacing(CHART_PADDING);

        table = table.push(level_row).spacing(CHART_PADDING);
    }

    let chart: Container<Message> = Container::new(table)
        .style(ContainerStyle::SquareBlack)
        .padding(CHART_PADDING);

    chart
}
