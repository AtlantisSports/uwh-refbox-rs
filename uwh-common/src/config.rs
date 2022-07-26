use log::*;
use serde_derive::{Deserialize, Serialize};
use std::{fs::read_to_string, path::Path, time::Duration};
use time::UtcOffset;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardware {
    pub screen_x: i32,
    pub screen_y: i32,
    pub white_on_right: bool,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            screen_x: 1024,
            screen_y: 768,
            white_on_right: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UwhScores {
    pub url: String,
    pub email: String,
    pub password: String,
    pub timezone: UtcOffset,
}

impl Default for UwhScores {
    fn default() -> Self {
        Self {
            url: "https://uwhscores.com/api/v1/".to_string(),
            email: String::new(),
            password: String::new(),
            timezone: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }
}

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
    pub overtime_break_duration: Duration,
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
            overtime_break_duration: Duration::from_secs(60),
            pre_sudden_death_duration: Duration::from_secs(60),
            post_game_duration: Duration::from_secs(120),
            nominal_break: Duration::from_secs(900),
            minimum_break: Duration::from_secs(240),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub game: Game,
    pub hardware: Hardware,
    pub uwhscores: UwhScores,
}

impl Config {
    pub fn new_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config_file = match read_to_string(path) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to read config file: {}", e);
                return Err(Box::new(e));
            }
        };

        match toml::from_str(&config_file) {
            Ok(c) => Ok(c),
            Err(e) => {
                error!("Failed to parse config file: {}", e);
                Err(Box::new(e))
            }
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
mod test {
    use super::*;
    use indoc::indoc;

    const HW_STRING: &str = indoc!(
        r#"screen_x = 1024
           screen_y = 768
           has_xbee = false
           has_rs485 = false
           white_on_right = false"#
    );

    const UWHSCORES_STRING: &str = indoc!(
        r#"url = "https://uwhscores.com/api/v1/"
           email = ""
           password = ""
           timezone = "+00:00:00""#
    );

    const GAME_STRING: &str = indoc!(
        r#"half_play_duration = 900
           half_time_duration = 180
           team_timeout_duration = 60
           overtime_allowed = true
           ot_half_play_duration = 300
           ot_half_time_duration = 180
           pre_overtime_break = 180
           overtime_break_duration = 60
           pre_sudden_death_duration = 60
           sudden_death_allowed = true
           team_timeouts_per_half = 1
           post_game_duration = 120
           nominal_break = 900
           minimum_break = 240"#
    );

    #[test]
    fn test_deser_hardware() {
        let hw: Hardware = Default::default();
        let deser = toml::from_str(HW_STRING);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_ser_hardware() {
        let hw: Hardware = Default::default();
        toml::to_string(&hw).unwrap();
    }

    #[test]
    fn test_deser_uwhscores() {
        let u: UwhScores = Default::default();
        let deser = toml::from_str(UWHSCORES_STRING);
        assert_eq!(deser, Ok(u));
    }

    #[test]
    fn test_ser_uwhscores() {
        let u: UwhScores = Default::default();
        toml::to_string(&u).unwrap();
    }

    #[test]
    fn test_deser_game() {
        let gm: Game = Default::default();
        let deser = toml::from_str(GAME_STRING);
        assert_eq!(deser, Ok(gm));
    }

    #[test]
    fn test_ser_game() {
        let gm: Game = Default::default();
        toml::to_string(&gm).unwrap();
    }

    #[test]
    fn test_deser_config() {
        let config: Config = Default::default();
        let deser = toml::from_str(&format!(
            "[game]\n{}\n[hardware]\n{}\n[uwhscores]\n{}",
            GAME_STRING, HW_STRING, UWHSCORES_STRING
        ));
        assert_eq!(deser, Ok(config));
    }

    #[test]
    fn test_ser_config() {
        let config: Config = Default::default();
        toml::to_string(&config).unwrap();
    }
}
