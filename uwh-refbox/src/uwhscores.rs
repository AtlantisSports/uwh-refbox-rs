use serde::{Deserialize, Deserializer};
use serde_derive::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use tokio::time::Duration;
use uwh_common::config::Game as GameConfig;

time::serde::format_description!(
    rfc3339_no_subsec_no_offest,
    PrimitiveDateTime,
    "[year]-[month]-[day]T[hour]:[minute]:[second]"
);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TournamentInfo {
    #[serde(with = "rfc3339_no_subsec_no_offest")]
    pub end_date: PrimitiveDateTime,
    pub is_active: u8,
    pub location: String,
    pub name: String,
    pub pools: Option<Vec<String>>,
    #[serde(with = "rfc3339_no_subsec_no_offest")]
    pub start_date: PrimitiveDateTime,
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
    #[serde(deserialize_with = "deser_with_null_to_default")]
    pub black_id: u32,
    pub game_type: String,
    pub gid: u32,
    pub pool: String,
    #[serde(deserialize_with = "deser_with_null_to_default")]
    pub score_b: u8,
    #[serde(deserialize_with = "deser_with_null_to_default")]
    pub score_w: u8,
    #[serde(with = "rfc3339_no_subsec_no_offest")]
    pub start_time: PrimitiveDateTime,
    pub tid: u32,
    pub timing_rules: Option<TimingRules>,
    pub white: String,
    #[serde(deserialize_with = "deser_with_null_to_default")]
    pub white_id: u32,
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

#[allow(clippy::from_over_into)]
impl Into<GameConfig> for TimingRules {
    fn into(self) -> GameConfig {
        GameConfig {
            team_timeouts_per_half: self.game_timeouts.allowed,
            team_timeout_duration: self.game_timeouts.duration,
            half_play_duration: self.half_duration,
            half_time_duration: self.half_time_duration,
            minimum_break: self.min_game_break,
            overtime_allowed: self.overtime_allowed,
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub ttl: u64,
    pub user_id: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameScoreInfo {
    pub tid: u32,
    pub gid: u32,
    pub score_b: u8,
    pub score_w: u8,
    pub black_id: u32,
    pub white_id: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameScorePostData {
    pub game_score: GameScoreInfo,
}

impl GameScorePostData {
    pub fn new(game_score: GameScoreInfo) -> Self {
        Self { game_score }
    }
}
