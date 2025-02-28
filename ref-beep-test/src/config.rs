use std::time::Duration;

use crate::sound_controller::SoundSettings;
use derivative::Derivative;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use toml::Table;

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub hardware: Hardware,
    pub beep_test: BeepTest,
    pub sound: SoundSettings,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardware {
    pub screen_x: i32,
    pub screen_y: i32,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            screen_x: 945,
            screen_y: 691,
        }
    }
}

impl Hardware {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut screen_x,
            mut screen_y,
        } = Default::default();

        get_integer_value(old, "screen_x", &mut screen_x);
        get_integer_value(old, "screen_y", &mut screen_y);

        Self { screen_x, screen_y }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Level {
    pub count: u8,
    #[serde(with = "secs_only_duration")]
    pub duration: Duration,
}

impl Level {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut count,
            mut duration,
        } = Default::default();

        if let Some(value) = old.get("count") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                count = value;
            }
        }

        if let Some(value) = old.get("duration") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                duration = Duration::from_secs(value);
            }
        }

        Self { count, duration }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeepTest {
    #[serde(with = "secs_only_duration")]
    pub pre: Duration,
    pub levels: Vec<Level>,
}

impl Default for BeepTest {
    fn default() -> Self {
        Self {
            pre: Duration::from_secs(10),
            levels: vec![
                Level {
                    count: 3,
                    duration: Duration::from_secs(36),
                },
                Level {
                    count: 3,
                    duration: Duration::from_secs(34),
                },
                Level {
                    count: 3,
                    duration: Duration::from_secs(32),
                },
                Level {
                    count: 4,
                    duration: Duration::from_secs(30),
                },
                Level {
                    count: 4,
                    duration: Duration::from_secs(28),
                },
                Level {
                    count: 4,
                    duration: Duration::from_secs(26),
                },
                Level {
                    count: 4,
                    duration: Duration::from_secs(24),
                },
                Level {
                    count: 4,
                    duration: Duration::from_secs(22),
                },
                Level {
                    count: 5,
                    duration: Duration::from_secs(20),
                },
                Level {
                    count: 4,
                    duration: Duration::from_secs(18),
                },
            ],
        }
    }
}

impl BeepTest {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut pre,
            mut levels,
        } = Default::default();

        if let Some(value) = old.get("pre") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                pre = Duration::from_secs(value);
            }
        }

        if let Some(values) = old.get("levels") {
            if let Some(values) = values.as_array() {
                for value in values {
                    if let Some(table) = value.as_table() {
                        levels.push(Level::migrate(table))
                    }
                }
            }
        }

        Self { pre, levels }
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

impl Config {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut sound,
            mut hardware,
            mut beep_test,
        } = Default::default();

        if let Some(old_sound) = old.get("sound") {
            if let Some(old_sound) = old_sound.as_table() {
                sound = SoundSettings::migrate(old_sound);
            }
        }

        if let Some(old_hardware) = old.get("hardware") {
            if let Some(old_hardware) = old_hardware.as_table() {
                hardware = Hardware::migrate(old_hardware);
            }
        }

        if let Some(old_beep_test) = old.get("beep_test") {
            if let Some(old_beep_test) = old_beep_test.as_table() {
                beep_test = BeepTest::migrate(old_beep_test);
            }
        }

        Self {
            sound,
            hardware,
            beep_test,
        }
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
