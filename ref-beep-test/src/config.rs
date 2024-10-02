use std::time::Duration;

use crate::sound_controller::SoundSettings;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use time::UtcOffset;

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub intervals: Vec<u8>,
    pub hardware: Hardware,
    pub levels: BeepTest,
    pub uwhscores: UwhScores,
    pub sound: SoundSettings,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeepTest {
    #[serde(with = "secs_only_duration")]
    pub pre: Duration,
    pub count_3: u8,
    pub count_4: u8,
    pub count_5: u8,
    #[serde(with = "secs_only_duration")]
    pub level_0: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_1: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_2: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_3: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_4: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_5: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_6: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_7: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_8: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_9: Duration,
    #[serde(with = "secs_only_duration")]
    pub level_10: Duration,
}

impl Default for BeepTest {
    fn default() -> Self {
        Self {
            pre: Duration::from_nanos(1),
            count_3: 3,
            count_4: 4,
            count_5: 5,
            level_0: Duration::from_secs(10),
            level_1: Duration::from_secs(36),
            level_2: Duration::from_secs(34),
            level_3: Duration::from_secs(32),
            level_4: Duration::from_secs(30),
            level_5: Duration::from_secs(28),
            level_6: Duration::from_secs(26),
            level_7: Duration::from_secs(24),
            level_8: Duration::from_secs(22),
            level_9: Duration::from_secs(20),
            level_10: Duration::from_secs(18),
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
