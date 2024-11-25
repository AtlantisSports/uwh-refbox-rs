use super::super::snapshot::BeepTestSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    CycleParameter(CyclingParameter),
    ToggleBoolParameter(BoolGameParameter),
    Reset,
    Start,
    Stop,
    ShowSettings,
    NewSnapshot(BeepTestSnapshot),
    EditComplete,
    NoAction, // TODO: Remove once UI is functional
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclingParameter {
    BuzzerSound,
    AlertVolume,
    AboveWaterVol,
    UnderWaterVol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolGameParameter {
    SoundEnabled,
    RefAlertEnabled,
}

impl Message {
    pub fn is_repeatable(&self) -> bool {
        match self {
            Self::NoAction | Self::CycleParameter(_) | Self::ToggleBoolParameter(_) => true,
            Self::Start
            | Self::Reset
            | Self::Stop
            | Self::ShowSettings
            | Self::NewSnapshot(_)
            | Self::EditComplete => false,
        }
    }
}
