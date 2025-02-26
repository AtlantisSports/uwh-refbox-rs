use super::{Color, Duration, GamePeriod, Instant, OffsetDateTime, Penalty, PenaltyKind};
use serde::Serialize;
use std::cmp::Ordering;
use time::format_description::well_known::{Iso8601, iso8601};

const CONFIG: iso8601::EncodedConfig = iso8601::Config::DEFAULT
    .set_year_is_six_digits(false)
    .encode();
const FORMAT: Iso8601<CONFIG> = Iso8601::<CONFIG>;
time::serde::format_description!(iso8601_short_year, OffsetDateTime, FORMAT);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameStats {
    game_number: u32,
    start_timestamp: Option<OffsetDateTime>,
    end_timestamp: Option<OffsetDateTime>,
    events: Vec<Event>,
}

impl GameStats {
    pub(crate) fn new(game_number: u32) -> Self {
        Self {
            game_number,
            start_timestamp: None,
            end_timestamp: None,
            events: Vec::new(),
        }
    }

    pub(crate) fn add_start_time(&mut self, now: Instant) {
        let timestamp = calculate_timestamp(now);
        self.start_timestamp = Some(timestamp);
    }

    pub(crate) fn add_end_time(&mut self, now: Instant) {
        let timestamp = calculate_timestamp(now);
        self.end_timestamp = Some(timestamp);
    }

    pub(crate) fn add_goal(
        &mut self,
        period: GamePeriod,
        time_left_in_period: Option<Duration>,
        color: Color,
        player_num: u8,
        instant: Instant,
    ) {
        let event = Event::Goal {
            player_cap_number: player_num,
            side: match color {
                Color::Black => "dark".to_string(),
                Color::White => "light".to_string(),
            },
            game_period: period,
            period_time: time_left_in_period.unwrap_or(Duration::ZERO).as_secs_f32(),
            occurred_on: calculate_timestamp(instant),
        };
        self.events.push(event);
    }

    pub(crate) fn add_penalty(&mut self, penalty: &Penalty, color: Color) {
        let event = Event::Penalty {
            player_cap_number: penalty.player_number,
            side: match color {
                Color::Black => "dark".to_string(),
                Color::White => "light".to_string(),
            },
            game_period: penalty.start_period,
            period_time: penalty.start_time.as_secs_f32(),
            occurred_on: calculate_timestamp(penalty.start_instant),
            duration: match penalty.kind {
                PenaltyKind::TotalDismissal => None,
                _ => Some(penalty.kind.as_duration().unwrap().as_secs()),
            },
            is_total_dismissal: penalty.kind == PenaltyKind::TotalDismissal,
        };
        self.events.push(event);
    }

    pub(crate) fn as_json(&self) -> String {
        let mut events = self.events.clone();
        events.sort_unstable_by_key(|event| match event {
            Event::Goal { occurred_on, .. } => *occurred_on,
            Event::Penalty { occurred_on, .. } => *occurred_on,
        });
        serde_json::to_string(&events).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "$type")]
enum Event {
    #[serde(rename = "goal")]
    Goal {
        #[serde(rename = "playerCapNumber")]
        player_cap_number: u8,
        side: String,
        #[serde(rename = "gamePeriod")]
        game_period: GamePeriod,
        #[serde(rename = "periodTime")]
        period_time: f32,
        #[serde(with = "iso8601_short_year")]
        #[serde(rename = "occurredOn")]
        occurred_on: OffsetDateTime,
    },
    #[serde(rename = "penalty")]
    Penalty {
        #[serde(rename = "playerCapNumber")]
        player_cap_number: u8,
        side: String,
        #[serde(rename = "gamePeriod")]
        game_period: GamePeriod,
        #[serde(rename = "periodTime")]
        period_time: f32,
        #[serde(with = "iso8601_short_year")]
        #[serde(rename = "occurredOn")]
        occurred_on: OffsetDateTime,
        duration: Option<u64>,
        #[serde(rename = "isTotalDismissal")]
        is_total_dismissal: bool,
    },
}

fn calculate_timestamp(instant: Instant) -> OffsetDateTime {
    let now = Instant::now();
    let mut timestamp = OffsetDateTime::now_utc();

    match instant.cmp(&now) {
        Ordering::Equal => {}
        Ordering::Less => {
            let duration = now - instant;
            timestamp -= duration;
        }
        Ordering::Greater => {
            let duration = instant - now;
            timestamp += duration;
        }
    }
    timestamp
}
