use chrono::naive::NaiveDateTime;
use serde::{Deserialize, Deserializer};
use serde_derive::{Deserialize, Serialize};
use tokio::time::Duration;
use uwh_common::config::Game as GameConfig;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TournamentInfo {
    pub end_date: NaiveDateTime,
    pub is_active: u8,
    pub location: String,
    pub name: String,
    pub pools: Option<Vec<String>>,
    pub start_date: NaiveDateTime,
    pub tid: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TournamentListResponse {
    pub tournaments: Vec<TournamentInfo>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TournamentSingleResponse {
    pub tournament: TournamentInfo,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub black: String,
    pub game_type: String,
    pub gid: u32,
    pub pool: String,
    #[serde(deserialize_with = "deser_with_null_to_default")]
    pub score_b: u8,
    #[serde(deserialize_with = "deser_with_null_to_default")]
    pub score_w: u8,
    pub start_time: NaiveDateTime,
    pub tid: u32,
    pub timing_rules: Option<TimingRules>,
    pub white: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TimingRules {
    pub game_timeouts: GameTimeouts,
    #[serde(deserialize_with = "deser_secs_to_dur")]
    pub half_duration: Duration,
    #[serde(deserialize_with = "deser_secs_to_dur")]
    pub half_time_duration: Duration,
    #[serde(deserialize_with = "deser_secs_to_dur")]
    pub min_game_break: Duration,
    pub overtime_allowed: bool,
    pub sudden_death_allowed: bool,
}

impl Into<GameConfig> for TimingRules {
    fn into(self) -> GameConfig {
        GameConfig {
            team_timeouts_per_half: self.game_timeouts.allowed,
            team_timeout_duration: self.game_timeouts.duration,
            half_play_duration: self.half_duration,
            half_time_duration: self.half_time_duration,
            minimum_break: self.min_game_break,
            has_overtime: self.overtime_allowed,
            sudden_death_allowed: self.sudden_death_allowed,
            ..Default::default()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameTimeouts {
    pub allowed: u16,
    #[serde(deserialize_with = "deser_secs_to_dur")]
    pub duration: Duration,
    pub per_half: bool,
}

fn deser_secs_to_dur<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(Duration::from_secs)
}

// Deserialize noramlly, but use the value's default if `null` is found
fn deser_with_null_to_default<'de, D, T: Deserialize<'de> + Default>(
    deserializer: D,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(|val| val.unwrap_or_default())
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameListResponse {
    pub games: Vec<GameInfo>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameSingleResponse {
    pub game: GameInfo,
}
