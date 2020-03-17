#![allow(dead_code)] // TODO: This is really ugly, needs to be removed

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GameSnapshot {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutState,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimeoutState {
    None,
    White(u16),
    Black(u16),
    Ref(u16),
    PenaltyShot(u16),
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
