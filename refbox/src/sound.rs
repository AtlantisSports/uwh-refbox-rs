use derivative::Derivative;
use enum_derive_2018::EnumDisplay;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SoundSettings {
    #[derivative(Default(value = "true"))]
    pub sound_enabled: bool,
    #[derivative(Default(value = "true"))]
    pub ref_warn_enabled: bool,
    pub buzzer_sound: BuzzerSound,
    #[derivative(Default(value = "Volume::Medium"))]
    pub ref_warn_vol: Volume,
    pub above_water_vol: Volume,
    pub under_water_vol: Volume,
    pub remotes: Vec<RemoteInfo>,
}

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!)]
    #[derivative(Default)]
    pub enum BuzzerSound {
        #[derivative(Default)]
        Buzz,
        Tweedle,
    }
}

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!)]
    #[derivative(Default)]
    pub enum Volume {
        Off,
        Low,
        Medium,
        High,
        #[derivative(Default)]
        Max,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub id: u32,
    pub sound: Option<BuzzerSound>,
}
