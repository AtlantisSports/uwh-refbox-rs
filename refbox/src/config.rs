use crate::sound_controller::SoundSettings;
use derivative::Derivative;
use enum_derive_2018::EnumDisplay;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};
use time::UtcOffset;
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub game: Game,
    pub hardware: Hardware,
    pub uwhscores: UwhScores,
    pub sound: SoundSettings,
    pub mode: Mode,
}
macro_attr! {
    #[derive(Debug, Clone, Copy, Derivative, PartialEq, Eq, Serialize, Deserialize, EnumDisplay!)]
    #[derivative(Default)]
    pub enum Mode {
        #[derivative(Default)]
        Hockey6V6,
        Hockey3V3,
        Rugby,
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
    fn test_ser_config() {
        let config: Config = Default::default();
        let serialized = toml::to_string(&config).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(config));
    }
}
