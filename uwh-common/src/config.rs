use serde::{Deserialize, Serialize};
use std::time::Duration;
use toml::Table;

// Due to requirements of the TOML language, items stored as tables in TOML (like `Duration`s) need
// to be after items that are not stored as tables (`u16`, `u32`, `bool`, `String`)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub num_team_timeouts_allowed: u16,
    /// Whether team timeouts are counted per half or per game
    pub timeouts_counted_per_half: bool,
    pub overtime_allowed: bool,
    pub sudden_death_allowed: bool,
    pub single_half: bool,
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
    #[serde(with = "secs_only_duration")]
    pub game_block: Duration,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            num_team_timeouts_allowed: 1,
            timeouts_counted_per_half: true,
            overtime_allowed: true,
            sudden_death_allowed: true,
            single_half: false,
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
            game_block: Duration::from_secs(2880),
        }
    }
}

impl Game {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut num_team_timeouts_allowed,
            mut timeouts_counted_per_half,
            mut overtime_allowed,
            mut sudden_death_allowed,
            mut single_half,
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
            mut game_block,
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

        if let Some(old_num_team_timeouts_allowed) = old.get("num_team_timeouts_allowed") {
            if let Some(old_num_team_timeouts_allowed) = old_num_team_timeouts_allowed.as_integer()
            {
                if let Ok(old_num_team_timeouts_allowed) = old_num_team_timeouts_allowed.try_into()
                {
                    num_team_timeouts_allowed = old_num_team_timeouts_allowed;
                }
            }
        } else if let Some(old_num_team_timeouts_allowed) = old.get("team_timeouts_per_half") {
            if let Some(old_num_team_timeouts_allowed) = old_num_team_timeouts_allowed.as_integer()
            {
                if let Ok(old_num_team_timeouts_allowed) = old_num_team_timeouts_allowed.try_into()
                {
                    num_team_timeouts_allowed = old_num_team_timeouts_allowed;
                    timeouts_counted_per_half = true;
                }
            }
        }
        if let Some(old_timeouts_counted_per_half) = old.get("timeouts_counted_per_half") {
            if let Some(old_timeouts_counted_per_half) = old_timeouts_counted_per_half.as_bool() {
                timeouts_counted_per_half = old_timeouts_counted_per_half;
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
        if let Some(old_single_half) = old.get("single_half") {
            if let Some(old_single_half) = old_single_half.as_bool() {
                single_half = old_single_half;
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
        let had_game_block = old
            .get("game_block")
            .and_then(toml::Value::as_integer)
            .is_some();
        process_duration(old, "game_block", &mut game_block);
        if !had_game_block {
            // Preserve the prior scheduling cadence: derive from this config's play durations.
            let regulation = if single_half {
                half_play_duration
            } else {
                2 * half_play_duration + half_time_duration
            };
            game_block = regulation + nominal_break;
        }

        Self {
            num_team_timeouts_allowed,
            timeouts_counted_per_half,
            overtime_allowed,
            sudden_death_allowed,
            single_half,
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
            game_block,
        }
    }
}

impl Game {
    /// Playing time excluding overtime: two halves + half-time, or a single period.
    pub fn regulation_play(&self) -> Duration {
        if self.single_half {
            self.half_play_duration
        } else {
            2 * self.half_play_duration + self.half_time_duration
        }
    }

    /// Smallest Game Block that fits the game plus the minimum break.
    pub fn game_block_minimum(&self) -> Duration {
        self.regulation_play() + self.minimum_break
    }

    /// Total team-timeout time both teams could use in a game (referee timeouts excluded).
    pub fn team_timeout_allotment(&self) -> Duration {
        let periods: u32 = if self.timeouts_counted_per_half && !self.single_half {
            2
        } else {
            1
        };
        // num_team_timeouts_allowed is u16; compute in u32 to avoid overflow, then scale Duration.
        let count = 2 * periods * u32::from(self.num_team_timeouts_allowed);
        count * self.team_timeout_duration
    }

    /// Slack between the Game Block and the math minimum (saturating at zero).
    pub fn game_block_buffer(&self) -> Duration {
        self.game_block.saturating_sub(self.game_block_minimum())
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
        assert_eq!(gm.num_team_timeouts_allowed, 2);
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

    #[test]
    fn test_migrate_game_block_derived_when_absent() {
        let mut old: Table = Default::default();
        old.insert("half_play_duration".to_string(), toml::Value::Integer(900));
        old.insert("half_time_duration".to_string(), toml::Value::Integer(180));
        old.insert("nominal_break".to_string(), toml::Value::Integer(900));
        old.insert("minimum_break".to_string(), toml::Value::Integer(240));
        let gm = Game::migrate(&old);
        assert_eq!(gm.game_block, Duration::from_secs(2880)); // 1980 + 900
    }

    #[test]
    fn test_migrate_game_block_kept_when_present() {
        let mut old: Table = Default::default();
        old.insert("half_play_duration".to_string(), toml::Value::Integer(900));
        old.insert("half_time_duration".to_string(), toml::Value::Integer(180));
        old.insert("nominal_break".to_string(), toml::Value::Integer(900));
        old.insert("minimum_break".to_string(), toml::Value::Integer(240));
        old.insert("game_block".to_string(), toml::Value::Integer(1500));
        let gm = Game::migrate(&old);
        assert_eq!(gm.game_block, Duration::from_secs(1500));
    }

    #[test]
    fn test_migrate_game_block_derived_single_half() {
        let mut old: Table = Default::default();
        old.insert("single_half".to_string(), toml::Value::Boolean(true));
        old.insert("half_play_duration".to_string(), toml::Value::Integer(600));
        old.insert("half_time_duration".to_string(), toml::Value::Integer(180));
        old.insert("nominal_break".to_string(), toml::Value::Integer(120));
        let gm = Game::migrate(&old);
        assert_eq!(gm.game_block, Duration::from_secs(720)); // single period 600 + 120
    }

    #[test]
    fn test_game_block_helpers_two_period() {
        let g = Game {
            single_half: false,
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            minimum_break: Duration::from_secs(240),
            num_team_timeouts_allowed: 1,
            team_timeout_duration: Duration::from_secs(60),
            timeouts_counted_per_half: false,
            game_block: Duration::from_secs(2880),
            ..Default::default()
        };
        assert_eq!(g.regulation_play(), Duration::from_secs(1980)); // 2*900+180
        assert_eq!(g.game_block_minimum(), Duration::from_secs(2220)); // 1980+240
        // per-game, both teams, 1 each * 60s = 120s
        assert_eq!(g.team_timeout_allotment(), Duration::from_secs(120));
        assert_eq!(g.game_block_buffer(), Duration::from_secs(660)); // 2880-2220
    }

    #[test]
    fn test_game_block_helpers_single_period_and_per_half() {
        let g = Game {
            single_half: true,
            half_play_duration: Duration::from_secs(600),
            half_time_duration: Duration::from_secs(180), // ignored when single_half
            minimum_break: Duration::from_secs(120),
            num_team_timeouts_allowed: 2,
            team_timeout_duration: Duration::from_secs(60),
            timeouts_counted_per_half: true, // single period => counted once
            game_block: Duration::from_secs(800),
            ..Default::default()
        };
        assert_eq!(g.regulation_play(), Duration::from_secs(600)); // single period
        assert_eq!(g.game_block_minimum(), Duration::from_secs(720)); // 600+120
        // per-half but single period => 1 period; 2 teams * 2 * 60 = 240
        assert_eq!(g.team_timeout_allotment(), Duration::from_secs(240));
        assert_eq!(g.game_block_buffer(), Duration::from_secs(80)); // 800-720
    }
}
