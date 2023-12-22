use crate::sound_controller::SoundSettings;
use derivative::Derivative;
use enum_derive_2018::{EnumDisplay, EnumFromStr};
use macro_attr_2018::macro_attr;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::UtcOffset;
use toml::Table;
pub use uwh_common::config::Game;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardware {
    pub screen_x: i32,
    pub screen_y: i32,
    pub white_on_right: bool,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            screen_x: 945,
            screen_y: 691,
            white_on_right: false,
        }
    }
}

impl Hardware {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut screen_x,
            mut screen_y,
            mut white_on_right,
        } = Default::default();

        get_integer_value(old, "screen_x", &mut screen_x);
        get_integer_value(old, "screen_y", &mut screen_y);
        get_boolean_value(old, "white_on_right", &mut white_on_right);

        Self {
            screen_x,
            screen_y,
            white_on_right,
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

impl UwhScores {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut url,
            mut email,
            mut password,
            timezone,
        } = Default::default();

        get_string_value(old, "url", &mut url);
        get_string_value(old, "email", &mut email);
        get_string_value(old, "password", &mut password);

        Self {
            url,
            email,
            password,
            timezone,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UwhPortal {
    pub url: String,
    pub token: String,
}

impl Default for UwhPortal {
    fn default() -> Self {
        Self {
            url: "https://api.uwhportal.com".to_string(),
            token: String::new(),
        }
    }
}

impl UwhPortal {
    pub fn migrate(old: &Table) -> Self {
        let Self { mut url, mut token } = Default::default();
        get_string_value(old, "url", &mut url);
        get_string_value(old, "token", &mut token);
        Self { url, token }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub mode: Mode,
    pub hide_time: bool,
    pub collect_scorer_cap_num: bool,
    pub game: Game,
    pub hardware: Hardware,
    pub uwhscores: UwhScores,
    pub uwhportal: UwhPortal,
    pub sound: SoundSettings,
}

impl Config {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut mode,
            mut hide_time,
            mut collect_scorer_cap_num,
            mut game,
            mut hardware,
            mut uwhscores,
            mut uwhportal,
            mut sound,
        } = Default::default();

        if let Some(old_mode) = old.get("mode") {
            if let Some(old_mode) = old_mode.as_str() {
                if let Ok(old_mode) = old_mode.parse() {
                    mode = old_mode;
                }
            }
        }
        get_boolean_value(old, "hide_time", &mut hide_time);
        get_boolean_value(old, "collect_scorer_cap_num", &mut collect_scorer_cap_num);
        if let Some(old_game) = old.get("game") {
            if let Some(old_game) = old_game.as_table() {
                game = Game::migrate(old_game);
            }
        }
        if let Some(old_hardware) = old.get("hardware") {
            if let Some(old_hardware) = old_hardware.as_table() {
                hardware = Hardware::migrate(old_hardware);
            }
        }
        if let Some(old_uwhscores) = old.get("uwhscores") {
            if let Some(old_uwhscores) = old_uwhscores.as_table() {
                uwhscores = UwhScores::migrate(old_uwhscores);
            }
        }
        if let Some(old_uwhportal) = old.get("uwhportal") {
            if let Some(old_uwhportal) = old_uwhportal.as_table() {
                uwhportal = UwhPortal::migrate(old_uwhportal);
            }
        }
        if let Some(old_sound) = old.get("sound") {
            if let Some(old_sound) = old_sound.as_table() {
                sound = SoundSettings::migrate(old_sound);
            }
        }

        Self {
            mode,
            hide_time,
            collect_scorer_cap_num,
            game,
            hardware,
            uwhscores,
            uwhportal,
            sound,
        }
    }
}

macro_attr! {
    #[derive(Debug, Clone, Copy, Derivative, PartialEq, Eq, Serialize, Deserialize, EnumDisplay!, EnumFromStr!)]
    #[derivative(Default)]
    pub enum Mode {
        #[derivative(Default)]
        Hockey6V6,
        Hockey3V3,
        Rugby,
    }
}

fn get_integer_value<T: DeserializeOwned + TryFrom<i64>>(table: &Table, key: &str, save: &mut T) {
    if let Some(value) = table.get(key) {
        if let Some(value) = value.as_integer() {
            if let Ok(value) = value.try_into() {
                *save = value;
            }
        }
    }
}

fn get_boolean_value(table: &Table, key: &str, save: &mut bool) {
    if let Some(value) = table.get(key) {
        if let Some(value) = value.as_bool() {
            *save = value;
        }
    }
}

fn get_string_value(table: &Table, key: &str, save: &mut String) {
    if let Some(value) = table.get(key) {
        if let Some(value) = value.as_str() {
            *save = value.to_string();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::sound_controller::Volume;

    use super::*;
    use std::time::Duration;

    #[test]
    fn test_ser_hardware() {
        let hw: Hardware = Default::default();
        let serialized = toml::to_string(&hw).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_ser_uwhscores() {
        let u: UwhScores = Default::default();
        let serialized = toml::to_string(&u).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(u));
    }

    #[test]
    fn test_ser_uwhportal() {
        let u: UwhPortal = Default::default();
        let serialized = toml::to_string(&u).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(u));
    }

    #[test]
    fn test_ser_config() {
        let config: Config = Default::default();
        let serialized = toml::to_string(&config).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(config));
    }

    #[test]
    fn test_migrate_hardware() {
        let mut old: Table = Default::default();
        old.insert("screen_x".to_string(), toml::Value::Integer(123));
        old.insert("screen_y".to_string(), toml::Value::Integer(456));
        old.insert("white_on_right".to_string(), toml::Value::Boolean(true));
        let hw = Hardware::migrate(&old);
        assert_eq!(hw.screen_x, 123);
        assert_eq!(hw.screen_y, 456);
        assert_eq!(hw.white_on_right, true);
    }

    #[test]
    fn test_migrate_uwhscores() {
        let mut old: Table = Default::default();
        old.insert(
            "url".to_string(),
            toml::Value::String("https://localhost/api/v1/".to_string()),
        );
        old.insert(
            "email".to_string(),
            toml::Value::String("test@test.com".to_string()),
        );
        old.insert(
            "password".to_string(),
            toml::Value::String("password".to_string()),
        );
        let u = UwhScores::migrate(&old);
        assert_eq!(u.url, "https://localhost/api/v1/");
        assert_eq!(u.email, "test@test.com");
        assert_eq!(u.password, "password");
    }

    #[test]
    fn test_migrate_uwhportal() {
        let mut old: Table = Default::default();
        old.insert(
            "url".to_string(),
            toml::Value::String("https://localhost/api/v1/".to_string()),
        );
        old.insert(
            "token".to_string(),
            toml::Value::String("token".to_string()),
        );
        let u = UwhPortal::migrate(&old);
        assert_eq!(u.url, "https://localhost/api/v1/");
        assert_eq!(u.token, "token");
    }

    #[test]
    fn test_migrate_config() {
        let mut old: Table = Default::default();
        old.insert("mode".to_string(), toml::Value::String("Rugby".to_string()));
        old.insert("hide_time".to_string(), toml::Value::Boolean(true));
        old.insert(
            "collect_scorer_cap_num".to_string(),
            toml::Value::Boolean(true),
        );
        let mut game: Table = Default::default();
        game.insert("half_play_duration".to_string(), toml::Value::Integer(123));
        old.insert("game".to_string(), toml::Value::Table(game));
        let mut hardware: Table = Default::default();
        hardware.insert("screen_x".to_string(), toml::Value::Integer(123));
        hardware.insert("screen_y".to_string(), toml::Value::Integer(456));
        hardware.insert("white_on_right".to_string(), toml::Value::Boolean(true));
        old.insert("hardware".to_string(), toml::Value::Table(hardware));
        let mut uwhscores: Table = Default::default();
        uwhscores.insert(
            "url".to_string(),
            toml::Value::String("https://localhost/api/v1/".to_string()),
        );
        uwhscores.insert(
            "email".to_string(),
            toml::Value::String("testing@test.com".to_string()),
        );
        uwhscores.insert(
            "password".to_string(),
            toml::Value::String("password".to_string()),
        );
        uwhscores.insert(
            "timezone".to_string(),
            toml::Value::String("UTC".to_string()),
        );
        old.insert("uwhscores".to_string(), toml::Value::Table(uwhscores));
        let mut uwhportal: Table = Default::default();
        uwhportal.insert(
            "url".to_string(),
            toml::Value::String("https://localhost/api/v1/".to_string()),
        );
        uwhportal.insert(
            "token".to_string(),
            toml::Value::String("token".to_string()),
        );
        old.insert("uwhportal".to_string(), toml::Value::Table(uwhportal));
        let mut sound: Table = Default::default();
        sound.insert("sound_enabled".to_string(), toml::Value::Boolean(false));
        sound.insert(
            "whistle_vol".to_string(),
            toml::Value::String("Max".to_string()),
        );
        old.insert("sound".to_string(), toml::Value::Table(sound));
        let config = Config::migrate(&old);
        assert_eq!(config.mode, Mode::Rugby);
        assert_eq!(config.hide_time, true);
        assert_eq!(config.collect_scorer_cap_num, true);
        assert_eq!(config.game.half_play_duration, Duration::from_secs(123));
        assert_eq!(config.hardware.screen_x, 123);
        assert_eq!(config.hardware.screen_y, 456);
        assert_eq!(config.hardware.white_on_right, true);
        assert_eq!(config.uwhscores.url, "https://localhost/api/v1/");
        assert_eq!(config.uwhscores.email, "testing@test.com");
        assert_eq!(config.uwhscores.password, "password");
        assert_eq!(config.uwhscores.timezone, UtcOffset::UTC);
        assert_eq!(config.uwhportal.url, "https://localhost/api/v1/");
        assert_eq!(config.uwhportal.token, "token");
        assert_eq!(config.sound.sound_enabled, false);
        assert_eq!(config.sound.whistle_vol, Volume::Max);
    }
}
