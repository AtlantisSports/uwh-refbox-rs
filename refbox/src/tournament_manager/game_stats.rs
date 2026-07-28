use super::{
    Color, Duration, GamePeriod, Infraction, InfractionDetails, Instant, OffsetDateTime, Penalty,
    PenaltyKind,
};
use serde::Serialize;
use std::cmp::Ordering;
use time::format_description::well_known::{Iso8601, iso8601};
use uwh_common::uwhportal::schedule::GameNumber;

const CONFIG: iso8601::EncodedConfig = iso8601::Config::DEFAULT
    .set_year_is_six_digits(false)
    .encode();
const FORMAT: Iso8601<CONFIG> = Iso8601::<CONFIG>;
time::serde::format_description!(iso8601_short_year, OffsetDateTime, FORMAT);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameStats {
    game_number: GameNumber,
    start_timestamp: Option<OffsetDateTime>,
    end_timestamp: Option<OffsetDateTime>,
    events: Vec<Event>,
}

impl GameStats {
    pub(crate) fn new<S: ToString>(game_number: S) -> Self {
        Self {
            game_number: game_number.to_string(),
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

    pub(crate) fn add_foul(&mut self, details: &InfractionDetails, color: Option<Color>) {
        let event = Event::Foul {
            player_cap_number: details.player_number,
            side: color.map(side_str),
            game_period: details.start_period,
            period_time: details.start_time.as_secs_f32(),
            occurred_on: calculate_timestamp(details.start_instant),
            called: details.infraction,
        };
        self.events.push(event);
    }

    pub(crate) fn as_json(&self) -> String {
        let mut events = self.events.clone();
        events.sort_unstable_by_key(|event| match event {
            Event::Goal { occurred_on, .. } => *occurred_on,
            Event::Penalty { occurred_on, .. } => *occurred_on,
            Event::Foul { occurred_on, .. } => *occurred_on,
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
    #[serde(rename = "foul")]
    Foul {
        #[serde(rename = "playerCapNumber")]
        player_cap_number: Option<u8>,
        side: Option<String>,
        #[serde(rename = "gamePeriod")]
        game_period: GamePeriod,
        #[serde(rename = "periodTime")]
        period_time: f32,
        #[serde(with = "iso8601_short_year")]
        #[serde(rename = "occurredOn")]
        occurred_on: OffsetDateTime,
        called: Infraction,
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

fn side_str(color: Color) -> String {
    match color {
        Color::Black => "dark".to_string(),
        Color::White => "light".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(player: Option<u8>, infraction: Infraction) -> InfractionDetails {
        InfractionDetails {
            player_number: player,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: Instant::now(),
            infraction,
        }
    }

    fn events_of(stats: &GameStats) -> Vec<serde_json::Value> {
        serde_json::from_str(&stats.as_json()).unwrap()
    }

    #[test]
    fn foul_events_serialize_with_type_side_player_and_called() {
        let mut stats = GameStats::new("1");
        // Player foul against White.
        stats.add_foul(
            &details(Some(7), Infraction::Obstruction),
            Some(Color::White),
        );
        // Team-level, neither-side foul ("both at fault"), no player.
        stats.add_foul(&details(None, Infraction::DelayOfGame), None);

        let events = events_of(&stats);
        let fouls: Vec<&serde_json::Value> =
            events.iter().filter(|e| e["$type"] == "foul").collect();
        assert_eq!(fouls.len(), 2);

        let white = fouls.iter().find(|e| e["side"] == "light").unwrap();
        assert_eq!(white["playerCapNumber"], 7);
        assert_eq!(white["called"], "Obstruction");
        assert_eq!(white["gamePeriod"], "FirstHalf");

        let neither = fouls.iter().find(|e| e["side"].is_null()).unwrap();
        assert!(neither["playerCapNumber"].is_null());
        assert_eq!(neither["called"], "DelayOfGame");
    }
}
