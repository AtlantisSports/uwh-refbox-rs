use crate::config::Game as GameConfig;
use serde::{Deserialize, Deserializer, Serialize};
use std::time::Duration;
use time::PrimitiveDateTime;

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
    #[serde(deserialize_with = "deser_or_default")]
    // TODO: Can we get the -1's fixed on uwhscores?
    pub score_b: u8,
    #[serde(deserialize_with = "deser_or_default")]
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
    pub pre_sudden_death_break: Option<u64>,
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
            pre_sudden_death_duration: if let Some(len) = self.pre_sudden_death_break {
                Duration::from_secs(len)
            } else {
                GameConfig::default().pre_sudden_death_duration
            },
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

// Deserialize noramlly, but use the value's default if an error occurs
fn deser_or_default<'de, D, T: Deserialize<'de> + Default>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(T::deserialize(deserializer).unwrap_or_default())
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

#[cfg(test)]
mod test {
    use super::*;
    use log::*;
    use reqwest::{blocking::Client, Method, StatusCode};
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn initialize() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    #[test]
    #[ignore]
    fn test_all_uwhscores_requests() {
        // NOTE: At last test, this took ~180s to complete
        const REQUEST_TIMEOUT: Duration = Duration::from_secs(1);
        const URL: &str = "https://uwhscores.com/api/v1/";
        initialize();

        let client = Client::builder()
            .https_only(true)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap();

        info!("Getting the list of tournaments");
        let request = client
            .request(Method::GET, format!("{URL}tournaments"))
            .build()
            .unwrap();
        let resp = client.execute(request).unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let tournaments = resp.json::<TournamentListResponse>().unwrap().tournaments;

        info!("Testing all the tournaments");
        for t in tournaments {
            info!("Getting details for tid {} ({})", t.tid, t.name);
            let request = client
                .request(Method::GET, format!("{URL}tournaments/{}", t.tid))
                .build()
                .unwrap();
            let resp = client.execute(request).unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let _t_info = resp.json::<TournamentSingleResponse>().unwrap().tournament;

            info!("Getting game list");
            let request = client
                .request(Method::GET, format!("{URL}tournaments/{}/games", t.tid))
                .build()
                .unwrap();
            let resp = client.execute(request).unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let games = resp.json::<GameListResponse>().unwrap().games;

            info!("Getting the details of each game");
            for g in games {
                info!("Getting details of game {}", g.gid);
                let request = client
                    .request(
                        Method::GET,
                        format!("{URL}tournaments/{}/games/{}", t.tid, g.gid),
                    )
                    .build()
                    .unwrap();
                let resp = client.execute(request).unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                resp.json::<GameSingleResponse>().unwrap();
            }

            // TODO: test gettting team details once added
        }
    }
}
