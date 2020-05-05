#![allow(dead_code)] // TODO: This is really ugly, needs to be removed

use std::{cmp::Ordering, cmp::PartialOrd};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GameSnapshot {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutSnapshot,
    pub b_score: u8,
    pub w_score: u8,
    pub penalties: Vec<PenaltySnapshot>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PenaltySnapshot {
    pub color: Color,
    pub player_number: u8,
    pub time: PenaltyTime,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GamePeriod {
    BetweenGames,
    FirstHalf,
    HalfTime,
    SecondHalf,
    PreOvertime,
    OvertimeFirstHalf,
    OvertimeHalfTime,
    OvertimeSecondHalf,
    PreSuddenDeath,
    SuddenDeath,
}

impl std::fmt::Display for GamePeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GamePeriod::BetweenGames => write!(f, "Between Games"),
            GamePeriod::FirstHalf => write!(f, "First Half"),
            GamePeriod::HalfTime => write!(f, "Half Time"),
            GamePeriod::SecondHalf => write!(f, "Second Half"),
            GamePeriod::PreOvertime => write!(f, "Pre Overtime"),
            GamePeriod::OvertimeFirstHalf => write!(f, "Overtime First Half"),
            GamePeriod::OvertimeHalfTime => write!(f, "Overtime Half Time"),
            GamePeriod::OvertimeSecondHalf => write!(f, "Overtime Second Half"),
            GamePeriod::PreSuddenDeath => write!(f, "Pre Sudden Death"),
            GamePeriod::SuddenDeath => write!(f, "Sudden Death"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimeoutSnapshot {
    None,
    White(u16),
    Black(u16),
    Ref(u16),
    PenaltyShot(u16),
}

impl std::fmt::Display for TimeoutSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TimeoutSnapshot::None => write!(f, "No Timeout"),
            TimeoutSnapshot::Black(_) => write!(f, "Black Timeout"),
            TimeoutSnapshot::White(_) => write!(f, "White Timeout"),
            TimeoutSnapshot::Ref(_) => write!(f, "Ref Timeout"),
            TimeoutSnapshot::PenaltyShot(_) => write!(f, "Penalty Shot"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Color {
    Black,
    White,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PenaltyTime {
    Seconds(u16),
    TotalDismissal,
}

impl Ord for PenaltyTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            PenaltyTime::TotalDismissal => match other {
                PenaltyTime::TotalDismissal => Ordering::Equal,
                PenaltyTime::Seconds(_) => Ordering::Greater,
            },
            PenaltyTime::Seconds(mine) => match other {
                PenaltyTime::Seconds(theirs) => mine.cmp(theirs),
                PenaltyTime::TotalDismissal => Ordering::Less,
            },
        }
    }
}

impl PartialOrd for PenaltyTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
