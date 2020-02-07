#![allow(dead_code)] // TODO: This is really ugly, needs to be removed

pub struct DrawableGameState {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutState,
    pub b_score: u8,
    pub w_score: u8,
    pub penalties: Vec<DrawablePenalty>,
}

pub struct DrawablePenalty {
    pub color: Color,
    pub player_number: u8,
    pub time: PenaltyTime,
}

pub enum GamePeriod {
    BetweenGames,
    FirstHalf,
    HalfTime,
    SecondHalf,
    PreOvertime,
    Overtime,
    PreSuddenDeath,
    SuddenDeath,
}

pub enum TimeoutState {
    None,
    White(u16),
    Black(u16),
    Ref(u16),
    PenaltyShot(u16),
}

pub enum Color {
    Black,
    White,
}

pub enum PenaltyTime {
    Seconds(u16),
    TotalDismissal,
}
