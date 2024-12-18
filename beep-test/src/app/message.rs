use super::super::{snapshot::BeepTestSnapshot, tournament_manager::TournamentManager};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum Message {
    CycleParameter(CyclingParameter),
    ToggleBoolParameter(BoolGameParameter),
    Reset,
    Start,
    Stop,
    ShowSettings,
    NewSnapshot(BeepTestSnapshot),
    EditComplete,
    TimeUpdaterStarted(Sender<Arc<Mutex<TournamentManager>>>),
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
            | Self::EditComplete
            | Self::TimeUpdaterStarted(_) => false,
        }
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::CycleParameter(a), Self::CycleParameter(b)) => a == b,
            (Self::ToggleBoolParameter(a), Self::ToggleBoolParameter(b)) => a == b,
            (Self::TimeUpdaterStarted(a), Self::TimeUpdaterStarted(b)) => a.same_channel(b),
            (Self::NewSnapshot(a), Self::NewSnapshot(b)) => a == b,

            (Self::Reset, Self::Reset)
            | (Self::Start, Self::Start)
            | (Self::Stop, Self::Stop)
            | (Self::ShowSettings, Self::ShowSettings)
            | (Self::EditComplete, Self::EditComplete)
            | (Self::NoAction, Self::NoAction) => true,

            (Self::CycleParameter(_), _)
            | (Self::ToggleBoolParameter(_), _)
            | (Self::TimeUpdaterStarted(_), _)
            | (Self::NewSnapshot(_), _)
            | (Self::Reset, _)
            | (Self::Start, _)
            | (Self::Stop, _)
            | (Self::ShowSettings, _)
            | (Self::EditComplete, _)
            | (Self::NoAction, _) => false,
        }
    }
}

impl Eq for Message {}
