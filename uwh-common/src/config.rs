use serde::{Deserialize, Serialize};
use std::time::Duration;
use toml::Table;

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
    pub penalty_shot_duration: Duration,
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
            penalty_shot_duration: Duration::from_secs(45),
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

impl Game {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut team_timeouts_per_half,
            mut overtime_allowed,
            mut sudden_death_allowed,
            mut half_play_duration,
            mut half_time_duration,
            mut team_timeout_duration,
            mut penalty_shot_duration,
            mut ot_half_play_duration,
            mut ot_half_time_duration,
            mut pre_overtime_break,
            mut pre_sudden_death_duration,
            mut post_game_duration,
            mut nominal_break,
            mut minimum_break,
        } = Default::default();

        let process_duration = |old: &Table, name: &str, save: &mut Duration| {
            if let Some(half_play_duration) = old.get(name) {
                if let Some(half_play_duration) = half_play_duration.as_integer() {
                    if let Ok(half_play_duration) = half_play_duration.try_into() {
                        *save = Duration::from_secs(half_play_duration);
                    }
                }
            }
        };

        if let Some(old_team_timeouts_per_half) = old.get("team_timeouts_per_half") {
            if let Some(old_team_timeouts_per_half) = old_team_timeouts_per_half.as_integer() {
                if let Ok(old_team_timeouts_per_half) = old_team_timeouts_per_half.try_into() {
                    team_timeouts_per_half = old_team_timeouts_per_half;
                }
            }
        }
        if let Some(old_overtime_allowed) = old.get("overtime_allowed") {
            if let Some(old_overtime_allowed) = old_overtime_allowed.as_bool() {
                overtime_allowed = old_overtime_allowed;
            }
        }
        if let Some(old_sudden_death_allowed) = old.get("sudden_death_allowed") {
            if let Some(old_sudden_death_allowed) = old_sudden_death_allowed.as_bool() {
                sudden_death_allowed = old_sudden_death_allowed;
            }
        }
        process_duration(old, "half_play_duration", &mut half_play_duration);
        process_duration(old, "half_time_duration", &mut half_time_duration);
        process_duration(old, "team_timeout_duration", &mut team_timeout_duration);
        process_duration(old, "penalty_shot_duration", &mut penalty_shot_duration);
        process_duration(old, "ot_half_play_duration", &mut ot_half_play_duration);
        process_duration(old, "ot_half_time_duration", &mut ot_half_time_duration);
        process_duration(old, "pre_overtime_break", &mut pre_overtime_break);
        process_duration(
            old,
            "pre_sudden_death_duration",
            &mut pre_sudden_death_duration,
        );
        process_duration(old, "post_game_duration", &mut post_game_duration);
        process_duration(old, "nominal_break", &mut nominal_break);
        process_duration(old, "minimum_break", &mut minimum_break);

        Self {
            team_timeouts_per_half,
            overtime_allowed,
            sudden_death_allowed,
            half_play_duration,
            half_time_duration,
            team_timeout_duration,
            penalty_shot_duration,
            ot_half_play_duration,
            ot_half_time_duration,
            pre_overtime_break,
            pre_sudden_death_duration,
            post_game_duration,
            nominal_break,
            minimum_break,
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

    #[test]
    fn test_migrate_game() {
        let mut old: Table = Default::default();
        old.insert(
            "team_timeouts_per_half".to_string(),
            toml::Value::Integer(2),
        );
        old.insert("overtime_allowed".to_string(), toml::Value::Boolean(false));
        old.insert(
            "sudden_death_allowed".to_string(),
            toml::Value::Boolean(false),
        );
        old.insert("half_play_duration".to_string(), toml::Value::Integer(123));
        old.insert("half_time_duration".to_string(), toml::Value::Integer(45));
        old.insert(
            "team_timeout_duration".to_string(),
            toml::Value::Integer(67),
        );
        old.insert(
            "penalty_shot_duration".to_string(),
            toml::Value::Integer(89),
        );
        old.insert(
            "ot_half_play_duration".to_string(),
            toml::Value::Integer(234),
        );
        old.insert(
            "ot_half_time_duration".to_string(),
            toml::Value::Integer(56),
        );
        old.insert("pre_overtime_break".to_string(), toml::Value::Integer(78));
        old.insert(
            "pre_sudden_death_duration".to_string(),
            toml::Value::Integer(90),
        );
        old.insert("post_game_duration".to_string(), toml::Value::Integer(12));
        old.insert("nominal_break".to_string(), toml::Value::Integer(345));
        old.insert("minimum_break".to_string(), toml::Value::Integer(111));

        let gm = Game::migrate(&old);
        assert_eq!(gm.team_timeouts_per_half, 2);
        assert_eq!(gm.overtime_allowed, false);
        assert_eq!(gm.sudden_death_allowed, false);
        assert_eq!(gm.half_play_duration, Duration::from_secs(123));
        assert_eq!(gm.half_time_duration, Duration::from_secs(45));
        assert_eq!(gm.team_timeout_duration, Duration::from_secs(67));
        assert_eq!(gm.penalty_shot_duration, Duration::from_secs(89));
        assert_eq!(gm.ot_half_play_duration, Duration::from_secs(234));
        assert_eq!(gm.ot_half_time_duration, Duration::from_secs(56));
        assert_eq!(gm.pre_overtime_break, Duration::from_secs(78));
        assert_eq!(gm.pre_sudden_death_duration, Duration::from_secs(90));
        assert_eq!(gm.post_game_duration, Duration::from_secs(12));
        assert_eq!(gm.nominal_break, Duration::from_secs(345));
        assert_eq!(gm.minimum_break, Duration::from_secs(111));
    }
}
