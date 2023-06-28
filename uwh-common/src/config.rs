use serde::{Deserialize, Serialize};
use std::time::Duration;

// Due to requirements of the TOML language, items stored as tables in TOML (like `Duration`s) need
// to be after items that are not stored as tables (`u16`, `u32`, `bool`, `String`)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub team_timeouts_per_half: u16,
    pub overtime_allowed: bool,
    pub sudden_death_allowed: bool,
    #[serde(with = "secs_only_duration")]
    pub half_play_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub half_time_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub team_timeout_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub ot_half_play_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub ot_half_time_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub pre_overtime_break: Duration,
    #[serde(with = "secs_only_duration")]
    pub pre_sudden_death_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub post_game_duration: Duration,
    #[serde(with = "secs_only_duration")]
    pub nominal_break: Duration,
    #[serde(with = "secs_only_duration")]
    pub minimum_break: Duration,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            team_timeouts_per_half: 1,
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            team_timeout_duration: Duration::from_secs(60),
            ot_half_play_duration: Duration::from_secs(300),
            ot_half_time_duration: Duration::from_secs(180),
            pre_overtime_break: Duration::from_secs(180),
            pre_sudden_death_duration: Duration::from_secs(60),
            post_game_duration: Duration::from_secs(120),
            nominal_break: Duration::from_secs(900),
            minimum_break: Duration::from_secs(240),
        }
    }
}

mod secs_only_duration {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(dur.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Duration::from_secs(u64::deserialize(deserializer)?))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ser_game() {
        let gm: Game = Default::default();
        let serialized = toml::to_string(&gm).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(gm));
    }
}
