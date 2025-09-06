// BeepTest view builder - copied from beep-test standalone application
use super::*;
use crate::app::message::{BeepTestBoolParameter, BeepTestCyclingParameter};
use crate::config::{BeepTest, BeepTestPeriod, Level};
use iced::{
    Alignment, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Button, Container, Row, button, column, container, horizontal_space, row, text},
};
use matrix_drawing::secs_to_long_time_string;
use std::fmt::Write;

// BeepTest-specific functions
fn bool_string(b: bool) -> String {
    if b {
        "YES".to_string()
    } else {
        "NO".to_string()
    }
}

// Make value button - copied from original beep-test standalone application
pub fn make_value_button<'a>(
    first_label: impl Into<String>,
    second_label: impl Into<String>,
    large_text: (bool, bool),
    message: Option<Message>,
) -> Button<'a, Message> {
    let mut button = button(
        iced::widget::Row::new()
            .spacing(SPACING)
            .align_y(Alignment::Center)
            .padding(PADDING)
            .push(
                text(first_label.into())
                    .size(if large_text.0 {
                        MEDIUM_TEXT
                    } else {
                        SMALL_TEXT
                    })
                    .height(Length::Fill)
                    .align_y(Vertical::Center),
            )
            .push(horizontal_space())
            .push(
                text(second_label.into())
                    .size(if large_text.1 {
                        MEDIUM_TEXT
                    } else {
                        SMALL_TEXT
                    })
                    .height(Length::Fill)
                    .align_y(Vertical::Center),
            ),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .style(light_gray_button);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

// BeepTest snapshot structure - copied from beep-test standalone application
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BeepTestSnapshot {
    pub current_period: BeepTestPeriod,
    pub secs_in_period: u32,
    pub next_period_len_secs: u32,
    pub lap_count: u8,
    pub total_time_in_period: u32,
    pub time_in_next_period: u32,
}

impl Default for BeepTestSnapshot {
    fn default() -> Self {
        Self {
            current_period: BeepTestPeriod::Pre,
            secs_in_period: 10,
            next_period_len_secs: 10,
            lap_count: 1,
            total_time_in_period: 10,
            time_in_next_period: 10,
        }
    }
}

// BeepTest state management
#[derive(Debug, Clone, Default)]
pub struct BeepTestState {
    pub config: BeepTest,
    pub snapshot: BeepTestSnapshot,
    pub clock_running: bool,
    #[allow(dead_code)]
    pub start_time: Option<std::time::Instant>,
    pub previous_mode: Option<Mode>,
}

impl BeepTestState {
    pub fn view(&self) -> Element<'_, Message> {
        build_beep_test_view(self)
    }
}

pub fn make_time_button<'a>(snapshot: &BeepTestSnapshot) -> Row<'a, Message> {
    let button_height = Length::Fixed(MIN_BUTTON_SIZE);
    let time_text = secs_to_long_time_string(snapshot.secs_in_period);
    let time_text = time_text.trim().to_owned();

    // Create time display text
    let time_display = text(time_text)
        .style(yellow_text)
        .size(LARGE_TEXT)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill);

    // Put it in a column for proper alignment
    let content = iced::widget::Column::new()
        .spacing(SPACING)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .push(time_display);

    // Wrap in a button with gray styling
    let time_button = button(content)
        .width(Length::Fill)
        .height(button_height)
        .style(gray_button)
        .padding(PADDING);

    // Return as a row like the original
    iced::widget::Row::new()
        .height(button_height)
        .width(Length::Fill)
        .spacing(SPACING)
        .push(time_button)
}

pub fn make_info_container<'a>(snapshot: &BeepTestSnapshot) -> Container<'a, Message> {
    let boxheight: f32 = 385.0;
    container(
        text(config_string(snapshot))
            .size(SMALL_TEXT)
            .align_y(Vertical::Top)
            .align_x(Horizontal::Left),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(boxheight))
    .style(light_gray_container)
}

pub fn make_button<'a>(label: impl Into<String>) -> Button<'a, Message> {
    button(centered_text(label.into()))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub fn centered_text<'a>(label: String) -> iced::widget::Text<'a> {
    text(label)
        .align_y(Vertical::Center)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

// This function puts each value in a container of the background color (table), and then puts all the
// rows and columns on a black container, giving it a grid (chart)
// second_chart is for cases when making a second chart if there are more levels than fit in one
pub fn build_levels_table(levels: &[Level], second_chart: bool) -> Container<'_, Message> {
    const CHART_PADDING: f32 = 2.0;

    let mut table = column![].spacing(CHART_PADDING);

    let headers = row![
        container(
            text("Level")
                .width(Length::Fixed(69.0))
                .size(16)
                .align_x(Horizontal::Center)
        )
        .style(square_light_gray_container)
        .padding(CHART_PADDING),
        container(
            text("Count")
                .width(Length::Fixed(69.0))
                .size(16)
                .align_x(Horizontal::Center)
        )
        .style(square_light_gray_container)
        .padding(CHART_PADDING),
        container(
            text("Duration")
                .width(Length::Fixed(69.0))
                .size(16)
                .align_x(Horizontal::Center)
        )
        .style(square_light_gray_container)
        .padding(CHART_PADDING)
    ]
    .spacing(CHART_PADDING);

    table = table.push(headers);

    for (index, level) in levels.iter().enumerate() {
        let level_row = row![
            container(
                text(if second_chart {
                    format!("{}", index + 14)
                } else {
                    format!("{}", index + 1)
                })
                .width(Length::Fixed(69.0))
                .size(16)
                .align_x(Horizontal::Center)
            )
            .style(square_light_gray_container)
            .padding(CHART_PADDING),
            container(
                text(format!("{}", level.count))
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .align_x(Horizontal::Center)
            )
            .style(square_light_gray_container)
            .padding(CHART_PADDING),
            container(
                text(format!("{:?}", level.duration))
                    .width(Length::Fixed(69.0))
                    .size(16)
                    .align_x(Horizontal::Center)
            )
            .style(square_light_gray_container)
            .padding(CHART_PADDING)
        ]
        .spacing(CHART_PADDING);

        table = table.push(level_row);
    }

    container(table)
        .style(square_black_container)
        .padding(CHART_PADDING)
}

fn config_string(snapshot: &BeepTestSnapshot) -> String {
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

pub fn build_beep_test_view(state: &BeepTestState) -> Element<'_, Message> {
    let snapshot = &state.snapshot;
    let config = &state.config;

    // Get clock running state from BeepTestState
    let clock_running = state.clock_running;

    // Large timer display at the top - exactly like original
    let time = make_time_button(snapshot);
    let mut content = iced::widget::Column::new().spacing(SPACING).push(time);

    // START and RESET buttons side by side - exactly like original
    let start_pause = if !clock_running {
        make_button("START")
            .on_press(Message::BeepTestStart)
            .style(green_button)
    } else {
        make_button("PAUSE")
            .on_press(Message::BeepTestPause)
            .style(orange_button)
    };

    let reset = make_button("RESET")
        .on_press(Message::BeepTestReset)
        .style(red_button);

    let buttons_row = iced::widget::Row::new()
        .spacing(SPACING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .push(start_pause)
        .push(reset);
    content = content.push(buttons_row);

    // Left info panel and right levels table - exactly like original
    let lap_info = make_info_container(snapshot);
    let chart = build_levels_table(&config.levels, false);

    // Settings and Return buttons split like START/RESET buttons at top
    let settings = make_button("SETTINGS")
        .on_press(Message::BeepTestSettings)
        .style(gray_button);

    let return_button = make_button("RETURN TO REFBOX")
        .on_press(Message::BeepTestReturnToRefbox)
        .style(blue_button);

    // Layout logic exactly matching original BeepTest
    if config.levels.len() > 13 {
        let chart_first_col = build_levels_table(&config.levels[..13], false);
        let chart_second_col = build_levels_table(&config.levels[13..], true);
        let chart_row = iced::widget::Row::new()
            .spacing(SPACING)
            .push(chart_first_col)
            .push(chart_second_col);
        let main_row = iced::widget::Row::new()
            .spacing(SPACING)
            .push(lap_info)
            .push(chart_row);
        content = content.push(main_row);
    } else {
        // Critical: Use the same fixed spacing gaps as original
        let gap1 = iced::widget::Space::with_width(Length::Fixed(115.0));
        let gap2 = iced::widget::Space::with_width(Length::Fixed(115.0));
        let chart_row = iced::widget::Row::new().push(gap1).push(chart).push(gap2);
        let main_row = iced::widget::Row::new()
            .spacing(SPACING)
            .push(lap_info)
            .push(chart_row);
        content = content.push(main_row);
    }

    // Settings and Return buttons as split row - exactly like START/RESET buttons
    let bottom_buttons_row = iced::widget::Row::new()
        .spacing(SPACING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .push(settings)
        .push(return_button);
    content = content.push(bottom_buttons_row);

    content.padding(PADDING).into()
}

// BeepTest settings page - adapted from original beep-test standalone application
pub fn build_beep_test_settings_view<'a>(
    snapshot: &BeepTestSnapshot,
    edited_settings: &crate::app::BeepTestEditableSettings,
) -> Element<'a, Message> {
    let crate::app::BeepTestEditableSettings { sound, .. } = edited_settings;

    let mut content = iced::widget::Column::new().spacing(SPACING);

    // Large timer display at the top - exactly like original
    let time = make_time_button(snapshot);
    content = content.push(time);

    // Sound settings rows - exactly like original
    let sound_row1 = iced::widget::Row::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_value_button(
            "SOUND\nENABLED:",
            bool_string(sound.sound_enabled),
            (false, true),
            Some(Message::BeepTestToggleBoolParameter(
                BeepTestBoolParameter::SoundEnabled,
            )),
        ))
        .push(make_value_button(
            "WHISTLE\nVOLUME:",
            sound.whistle_vol.to_string().to_uppercase(),
            (false, true),
            Some(Message::BeepTestCycleParameter(
                BeepTestCyclingParameter::WhistleVolume,
            )),
        ));
    content = content.push(sound_row1);

    let sound_row2 = iced::widget::Row::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_value_button(
            "WHISTLE\nENABLED:",
            bool_string(sound.whistle_enabled),
            (false, true),
            Some(Message::BeepTestToggleBoolParameter(
                BeepTestBoolParameter::WhistleEnabled,
            )),
        ))
        .push(make_value_button(
            "ABOVE WATER\nVOLUME:",
            sound.above_water_vol.to_string().to_uppercase(),
            (false, true),
            Some(Message::BeepTestCycleParameter(
                BeepTestCyclingParameter::AboveWaterVolume,
            )),
        ));
    content = content.push(sound_row2);

    let sound_row3 = iced::widget::Row::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_value_button(
            "BUZZER\nSOUND:",
            format!("{:?}", sound.buzzer_sound).to_uppercase(),
            (false, true),
            Some(Message::BeepTestCycleParameter(
                BeepTestCyclingParameter::BuzzerSound,
            )),
        ))
        .push(make_value_button(
            "UNDER WATER\nVOLUME:",
            sound.under_water_vol.to_string().to_uppercase(),
            (false, true),
            Some(Message::BeepTestCycleParameter(
                BeepTestCyclingParameter::UnderWaterVolume,
            )),
        ));
    content = content.push(sound_row3);

    // DONE button row - exactly like original
    let done_row = iced::widget::Row::new()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(horizontal_space())
        .push(horizontal_space())
        .push(
            make_button("DONE")
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::BeepTestSettingsDone),
        );
    content = content.push(done_row);

    content.height(Length::Fill).padding(PADDING).into()
}
