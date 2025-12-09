use super::super::message::Message;
use crate::app::theme::*;
use iced::{
    Element, Length, Alignment,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, container, row, text, Button, Column, Container, Row, Text},
};
use std::time::{Duration, Instant};
use std::fmt::Write;
use matrix_drawing::secs_to_long_time_string;
use serde::{Deserialize, Serialize};
use derivative::Derivative;

// Copy exact structures from original beep test
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Level {
    pub count: u8,
    #[serde(with = "secs_only_duration")]
    pub duration: Duration,
}

impl Level {
    #[allow(dead_code)]
    pub fn migrate(_old: &toml::Value) -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeepTest {
    #[serde(with = "secs_only_duration")]
    pub pre: Duration,
    pub levels: Vec<Level>,
}

impl Default for BeepTest {
    fn default() -> Self {
        Self {
            pre: Duration::from_secs(10),
            levels: vec![
                Level { count: 3, duration: Duration::from_secs(36) },
                Level { count: 3, duration: Duration::from_secs(34) },
                Level { count: 3, duration: Duration::from_secs(32) },
                Level { count: 4, duration: Duration::from_secs(30) },
                Level { count: 4, duration: Duration::from_secs(28) },
                Level { count: 5, duration: Duration::from_secs(26) },
                Level { count: 5, duration: Duration::from_secs(24) },
                Level { count: 6, duration: Duration::from_secs(22) },
                Level { count: 6, duration: Duration::from_secs(20) },
                Level { count: 7, duration: Duration::from_secs(18) },
                Level { count: 7, duration: Duration::from_secs(16) },
                Level { count: 8, duration: Duration::from_secs(14) },
                Level { count: 8, duration: Duration::from_secs(12) },
                Level { count: 9, duration: Duration::from_secs(10) },
                Level { count: 9, duration: Duration::from_secs(8) },
            ],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
pub struct BeepTestSnapshot {
    pub current_period: BeepTestPeriod,
    pub secs_in_period: u32,
    pub next_period_len_secs: u32,
    pub lap_count: u8,
    pub total_time_in_period: u32,
    pub time_in_next_period: u32,
}

#[derive(Derivative, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derivative(Debug, Default, Clone, Copy)]
pub enum BeepTestPeriod {
    #[derivative(Default)]
    Pre,
    Level(usize),
}

#[allow(dead_code)]
impl BeepTestPeriod {
    pub fn duration(self, config: &BeepTest) -> Option<Duration> {
        match self {
            Self::Pre => Some(Duration::from_secs(10)),
            Self::Level(0) => Some(config.pre),
            Self::Level(i) => config.levels.get(i - 1).map(|l| l.duration),
        }
    }

    pub fn count(self, config: &BeepTest) -> Option<u8> {
        match self {
            Self::Pre | Self::Level(0) => Some(1),
            Self::Level(i) => config.levels.get(i - 1).map(|l| l.count),
        }
    }

    pub fn next_period(self, config: &BeepTest) -> BeepTestPeriod {
        match self {
            Self::Pre => Self::Level(0),
            Self::Level(i) => {
                if i < config.levels.len() {
                    Self::Level(i + 1)
                } else {
                    Self::Pre
                }
            }
        }
    }

    pub fn next_test_period_dur(self, config: &BeepTest) -> Option<Duration> {
        self.next_period(config).duration(config)
    }
}

impl std::fmt::Display for BeepTestPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pre => write!(f, "Pre"),
            Self::Level(i) => write!(f, "Level {i}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BeepTestState {
    config: BeepTest,
    snapshot: BeepTestSnapshot,
    clock_running: bool,
    start_time: Option<Instant>,
}

#[allow(clippy::derivable_impls)]
impl Default for BeepTestState {
    fn default() -> Self {
        Self {
            config: BeepTest::default(),
            snapshot: BeepTestSnapshot::default(),
            clock_running: false,
            start_time: None,
        }
    }
}

impl BeepTestState {
    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::BeepTestStart => {
                self.clock_running = true;
                self.start_time = Some(Instant::now());
                iced::Task::none()
            }
            Message::BeepTestStop => {
                self.clock_running = false;
                self.start_time = None;
                iced::Task::none()
            }
            Message::BeepTestReset => {
                *self = Self::default();
                iced::Task::none()
            }
            Message::BeepTestUpdate => {
                if self.clock_running && self.start_time.is_some() {
                    // Update timer logic would go here
                    // For now, just decrement seconds
                    if self.snapshot.secs_in_period > 0 {
                        self.snapshot.secs_in_period -= 1;
                        self.start_time = Some(Instant::now());
                    } else {
                        // Period ended, advance to next
                        self.advance_period();
                    }
                }
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn advance_period(&mut self) {
        let current_period = self.snapshot.current_period;
        match &current_period {
            BeepTestPeriod::Pre => {
                // Move to Level 1
                self.snapshot.current_period = BeepTestPeriod::Level(1);
                self.snapshot.lap_count = 1;
                if let Some(level) = self.config.levels.first() {
                    self.snapshot.secs_in_period = level.duration.as_secs() as u32;
                    self.snapshot.total_time_in_period = level.duration.as_secs() as u32;
                }
                if let Some(level) = self.config.levels.get(1) {
                    self.snapshot.time_in_next_period = level.duration.as_secs() as u32;
                }
            }
            BeepTestPeriod::Level(level_idx) => {
                if let Some(level) = self.config.levels.get(level_idx - 1) {
                    if self.snapshot.lap_count < level.count {
                        // Next lap in same level
                        self.snapshot.lap_count += 1;
                        self.snapshot.secs_in_period = level.duration.as_secs() as u32;
                    } else {
                        // Next level
                        if let Some(next_level) = self.config.levels.get(*level_idx) {
                            self.snapshot.current_period = BeepTestPeriod::Level(level_idx + 1);
                            self.snapshot.lap_count = 1;
                            self.snapshot.secs_in_period = next_level.duration.as_secs() as u32;
                            self.snapshot.total_time_in_period = next_level.duration.as_secs() as u32;
                            if let Some(future_level) = self.config.levels.get(level_idx + 1) {
                                self.snapshot.time_in_next_period = future_level.duration.as_secs() as u32;
                            }
                        } else {
                            // Test complete, stop
                            self.clock_running = false;
                        }
                    }
                }
            }
        }
        self.start_time = Some(Instant::now());
    }

    pub fn view(&self) -> Element<'_, Message> {
        build_main_view(&self.snapshot, self.clock_running, &self.config)
    }
}

// Copy exact functions from original beep test shared_elements.rs
pub fn make_time_button<'a>(snapshot: &BeepTestSnapshot) -> Row<'a, Message> {
    let button_height = Length::Fixed(MIN_BUTTON_SIZE);

    let time_text = secs_to_long_time_string(snapshot.secs_in_period).to_string();
    let time_text = time_text.trim().to_owned();

    let time_view = text(time_text)
        .style(yellow_text)
        .size(LARGE_TEXT)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    let content = column![time_view]
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .spacing(SPACING)
        .height(Length::Fill);

    let time_button = button(content)
        .width(Length::Fill)
        .height(button_height)
        .style(gray_button)
        .padding(PADDING);

    row![time_button]
        .height(button_height)
        .width(Length::Fill)
        .spacing(SPACING)
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

pub fn make_button<'a>(
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

pub fn config_string(snapshot: &BeepTestSnapshot) -> String {
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

// Copy exact build_levels_table function from original
pub fn build_levels_table(levels: &[Level], second_chart: bool) -> Container<'_, Message> {
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

// Copy exact build_main_view function from original main_view.rs
pub fn build_main_view<'a>(
    snapshot: &BeepTestSnapshot,
    clock_running: bool,
    beep_test: &'a BeepTest,
) -> Element<'a, Message> {
    let time = make_time_button(snapshot);

    let mut content = column![time].spacing(SPACING);

    let start_pause = if !clock_running {
        make_button("START")
            .on_press(Message::BeepTestStart)
            .style(green_button)
    } else {
        make_button("PAUSE")
            .on_press(Message::BeepTestStop)
            .style(orange_button)
    };

    let reset = make_button("RESET")
        .on_press(Message::BeepTestReset)
        .style(red_button);

    content = content.push(row![start_pause, reset].spacing(SPACING));

    let lap_info = make_info_container(snapshot);

    let chart = build_levels_table(&beep_test.levels, false);

    let settings = make_button("SETTINGS")
        .style(gray_button)
        .on_press(Message::BeepTestShowSettings);

    let back_to_refbox = make_button("BACK TO REFBOX")
        .style(blue_button)
        .on_press(Message::BeepTestBackToRefbox);

    if beep_test.levels.len() > 13 {
        let chart_first_col = build_levels_table(&beep_test.levels[..13], false);
        let chart_second_col = build_levels_table(&beep_test.levels[13..], true);
        let chart_row = row![chart_first_col, chart_second_col].spacing(SPACING);
        content = content.push(row![lap_info, chart_row].spacing(SPACING));
    } else {
        let gap1 = Space::with_width(Length::Fixed(115.0));
        let gap2 = Space::with_width(Length::Fixed(115.0));
        let chart_row = row![gap1, chart, gap2];
        content = content.push(row![lap_info, chart_row].spacing(SPACING));
    }

    content = content.push(row![settings, back_to_refbox].spacing(SPACING));

    content.into()
}

// Build the beep test view
pub fn build_beep_test_view(state: &BeepTestState) -> Element<'_, Message> {
    state.view()
}

// Serde helper module (simplified version)
mod secs_only_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
