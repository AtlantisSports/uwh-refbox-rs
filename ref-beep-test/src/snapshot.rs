use std::{cmp::min, time::Duration};

use derivative::Derivative;
use serde::{Deserialize, Serialize};
use time::Duration as SignedDuration;
use uwh_common::{
    drawing_support::MAX_STRINGABLE_SECS,
    game_snapshot::{GamePeriod, GameSnapshotNoHeap},
};

use crate::config::BeepTest;

#[derive(Debug, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
pub struct BeepTestSnapshot {
    pub current_period: BeepTestPeriod,
    pub secs_in_period: u32,
    pub next_period_len_secs: u32,
}

impl From<BeepTestSnapshot> for GameSnapshotNoHeap {
    fn from(snapshot: BeepTestSnapshot) -> Self {
        let current_period = GamePeriod::BetweenGames;
        Self {
            current_period,
            secs_in_period: min(
                snapshot
                    .secs_in_period
                    .try_into()
                    .unwrap_or(MAX_STRINGABLE_SECS),
                MAX_STRINGABLE_SECS,
            ),
            ..Default::default()
        }
    }
}

#[derive(Derivative, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derivative(Debug, Default, Clone, Copy)]
pub enum BeepTestPeriod {
    #[derivative(Default)]
    Pre,
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    Level8,
    Level9,
    Level10,
}

impl BeepTestPeriod {
    pub fn duration(self, config: &BeepTest) -> Option<Duration> {
        match self {
            Self::Pre => Some(config.pre),
            Self::Level0 => Some(config.level_0),
            Self::Level1 => Some(config.level_1),
            Self::Level2 => Some(config.level_2),
            Self::Level3 => Some(config.level_3),
            Self::Level4 => Some(config.level_4),
            Self::Level5 => Some(config.level_5),
            Self::Level6 => Some(config.level_6),
            Self::Level7 => Some(config.level_7),
            Self::Level8 => Some(config.level_8),
            Self::Level9 => Some(config.level_9),
            Self::Level10 => Some(config.level_10),
        }
    }

    pub fn time_elapsed_at(self, time: Duration, config: &BeepTest) -> Option<SignedDuration> {
        match self {
            p @ Self::Pre
            | p @ Self::Level0
            | p @ Self::Level1
            | p @ Self::Level2
            | p @ Self::Level3
            | p @ Self::Level4
            | p @ Self::Level5
            | p @ Self::Level6
            | p @ Self::Level7
            | p @ Self::Level8
            | p @ Self::Level9
            | p @ Self::Level10 => p
                .duration(config)
                .and_then(|d| d.try_into().ok().map(|sd: SignedDuration| sd - time)),
        }
    }

    pub fn next_period(self) -> Option<BeepTestPeriod> {
        match self {
            Self::Pre => Some(Self::Level0),
            Self::Level0 => Some(Self::Level1),
            Self::Level1 => Some(Self::Level2),
            Self::Level2 => Some(Self::Level3),
            Self::Level3 => Some(Self::Level4),
            Self::Level4 => Some(Self::Level5),
            Self::Level5 => Some(Self::Level6),
            Self::Level6 => Some(Self::Level7),
            Self::Level7 => Some(Self::Level8),
            Self::Level8 => Some(Self::Level9),
            Self::Level9 => Some(Self::Level10),
            Self::Level10 => Some(Self::Pre),
        }
    }

    pub fn next_test_period_dur(self, config: &BeepTest) -> Option<Duration> {
        match self.next_period()? {
            Self::Pre => Some(config.pre),
            Self::Level0 => Some(config.level_0),
            Self::Level1 => Some(config.level_1),
            Self::Level2 => Some(config.level_2),
            Self::Level3 => Some(config.level_3),
            Self::Level4 => Some(config.level_4),
            Self::Level5 => Some(config.level_5),
            Self::Level6 => Some(config.level_6),
            Self::Level7 => Some(config.level_7),
            Self::Level8 => Some(config.level_8),
            Self::Level9 => Some(config.level_9),
            Self::Level10 => Some(config.level_10),
        }
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum TimeSnapshot {
    #[derivative(Default)]
    None,
    Running(u16),
}

impl core::fmt::Display for TimeSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            TimeSnapshot::None => write!(f, "Stopped"),
            TimeSnapshot::Running(_) => write!(f, "Running"),
        }
    }
}
