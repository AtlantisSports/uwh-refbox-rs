use super::penalty::PenaltyKind;
use tokio::time::Duration;
use uwh_common::{
    color::Color,
    game_snapshot::{GamePeriod, PenaltyTime},
};

/// What the live updates need that the game snapshot does not carry. Built by
/// `TournamentManager::live_details`, which only reads the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDetails {
    /// The goal the stream is showing now: team, cap, game clock at the goal (whole seconds).
    pub recent_goal: Option<(Color, u8, u32)>,
    /// EVERY penalty the engine holds, served ones included, Black's in list order then White's.
    pub penalties: Vec<EnginePenalty>,
    /// `Some` only during a White/Black team timeout: its clock.
    pub team_timeout: Option<TeamTimeout>,
}

/// The clock of a White/Black team timeout, read from the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamTimeout {
    pub running: bool,
    /// Whole seconds left: 0 once the countdown has passed zero (the timeout ends only at the
    /// next update), saturating at `u16::MAX` only for a real reading that large.
    pub seconds_left: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnginePenalty {
    pub team: Color,
    pub cap: u8,
    pub kind: PenaltyKind,
    /// From `Penalty::as_snapshot`: `Seconds(0)` when served.
    pub time: PenaltyTime,
    /// The penalty's `start_period` and `start_time`: its identity.
    pub given: (GamePeriod, Duration),
}
