use crate::config::BeepTest;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{cmp::min, fmt::Display, time::Duration};
use uwh_common::{
    bundles::BlackWhiteBundle,
    drawing_support::MAX_STRINGABLE_SECS,
    game_snapshot::{GamePeriod, GameSnapshotNoHeap},
};

#[derive(Debug, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
pub struct BeepTestSnapshot {
    pub current_period: BeepTestPeriod,
    pub secs_in_period: u32,
    pub next_period_len_secs: u32,
    pub lap_count: u8,
    pub total_time_in_period: u32,
    pub time_in_next_period: u32,
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
            scores: BlackWhiteBundle {
                black: 0,
                white: snapshot.lap_count,
            },
            ..Default::default()
        }
    }
}

#[derive(Derivative, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derivative(Debug, Default, Clone, Copy)]
pub enum BeepTestPeriod {
    #[derivative(Default)]
    Pre,
    Level(usize),
}

impl BeepTestPeriod {
    pub fn duration(self, config: &BeepTest) -> Option<Duration> {
        match self {
            Self::Pre => Some(Duration::from_secs(10)),
            Self::Level(0) => Some(config.pre),
            Self::Level(i) => config.levels.get(i - 1).map(|l| l.duration),
        }
    }

    pub fn count(self, config: &BeepTest) -> Option<u8> {
        match self {
            Self::Pre | Self::Level(0) => Some(1),
            Self::Level(i) => config.levels.get(i - 1).map(|l| l.count),
        }
    }

    pub fn next_period(self, config: &BeepTest) -> BeepTestPeriod {
        match self {
            Self::Pre => Self::Level(0),
            Self::Level(i) => {
                if i < config.levels.len() {
                    Self::Level(i + 1)
                } else {
                    Self::Pre
                }
            }
        }
    }

    pub fn next_test_period_dur(self, config: &BeepTest) -> Option<Duration> {
        self.next_period(config).duration(config)
    }
}

impl Display for BeepTestPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pre => write!(f, "Pre"),
            Self::Level(i) => write!(f, "Level {i}"),
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
