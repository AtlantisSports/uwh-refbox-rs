//! Canonical scenario library for the golden-trace regression guard.
//!
//! Every scenario is a pure data description; no engine logic lives here.
//! Call [`all`] to obtain the full list, then pass each [`Scenario`] to
//! [`super::run`] to obtain a trace.
//!
//! # Dead-code suppression
//!
//! The statics and `all()` are consumed by Task 4 integration tests that do
//! not exist yet.  The suppression mirrors the pattern already used for
//! `Action` in `mod.rs`.
#![allow(dead_code)]

use super::{Action, Scenario};
use std::time::Duration;
use uwh_common::{color::Color, config::Game as GameConfig, game_snapshot::GamePeriod};

use crate::tournament_manager::{
    golden::Action::{
        AddScore, EndTimeout, SetGameClock, SetupPeriod, StartClock, StartPenalty,
        StartPenaltyShot, StartRefTimeout, StartRugbyPenaltyShot, StartTeamTimeout, StopClock,
    },
    penalty::PenaltyKind,
};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Minimal config for two-half regulation games; override only what the
/// scenario needs.  Durations are deliberately short so traces are compact.
fn reg_config() -> GameConfig {
    GameConfig {
        half_play_duration: Duration::from_secs(20),
        half_time_duration: Duration::from_secs(8),
        overtime_allowed: false,
        sudden_death_allowed: false,
        pre_overtime_break: Duration::from_secs(5),
        ot_half_play_duration: Duration::from_secs(10),
        ot_half_time_duration: Duration::from_secs(5),
        pre_sudden_death_duration: Duration::from_secs(5),
        post_game_duration: Duration::from_secs(5),
        nominal_break: Duration::from_secs(10),
        minimum_break: Duration::from_secs(6),
        team_timeout_duration: Duration::from_secs(15),
        penalty_shot_duration: Duration::from_secs(15),
        ..Default::default()
    }
}

// ── Family 1 — Regulation flow ────────────────────────────────────────────────

// 1. regulation_full
static REGULATION_FULL_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
];

// 2. regulation_with_scores
static REGULATION_WITH_SCORES_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (5, AddScore(Color::Black)),
    (12, AddScore(Color::White)),
];

// ── Family 2 — Penalties over time ───────────────────────────────────────────

// 3. penalty_one_minute — OneMinute penalty that expires mid-half.
//    Half is 80 s; penalty starts at t=5 s; expires at t=65 s (well before end).
static PENALTY_ONE_MINUTE_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(80)),
    ),
    (0, StartClock),
    (5, StartPenalty(Color::Black, 3, PenaltyKind::OneMinute)),
];

// 4. penalty_crosses_break — penalty started late in FirstHalf, frozen across
//    HalfTime, resuming in SecondHalf.
//    Half = 20 s.  Penalty (OneMinute) starts at t=5 s; 15 s elapse in
//    FirstHalf, leaving 45 s.  HalfTime = 8 s (frozen).  Resumes in SecondHalf.
static PENALTY_CROSSES_BREAK_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (5, StartPenalty(Color::White, 9, PenaltyKind::OneMinute)),
];

// 5. penalty_concurrent — two simultaneous penalties: OneMinute (Black) and
//    TwoMinute (White), both starting at t=2 s.  Half = 90 s.
static PENALTY_CONCURRENT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(90)),
    ),
    (0, StartClock),
    (2, StartPenalty(Color::Black, 1, PenaltyKind::OneMinute)),
    (2, StartPenalty(Color::White, 2, PenaltyKind::TwoMinute)),
];

// 6. penalty_during_stoppage — spike scenario: FirstHalf 40 s, B#7 ThirtySecond
//    @2 s, StopClock @15, StartClock @18, run ~55 s.
static PENALTY_DURING_STOPPAGE_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
    ),
    (0, StartClock),
    (2, StartPenalty(Color::Black, 7, PenaltyKind::ThirtySecond)),
    (15, StopClock),
    (18, StartClock),
];

// 7. penalty_total_dismissal — TD renders as "TD" and never counts down.
static PENALTY_TOTAL_DISMISSAL_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
    ),
    (0, StartClock),
    (
        3,
        StartPenalty(Color::White, 5, PenaltyKind::TotalDismissal),
    ),
];

// 8. penalty_expires_at_boundary — ThirtySecond penalty started at t=0 in a
//    FirstHalf that is exactly 30 s long.  Both the half and the penalty should
//    reach 0 at the same moment (the period-end second).
//    This targets the off-by-one risk at the period boundary.
static PENALTY_EXPIRES_AT_BOUNDARY_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(30)),
    ),
    (0, StartPenalty(Color::Black, 4, PenaltyKind::ThirtySecond)),
    (0, StartClock),
];

// ── Family 3 — Timeouts & penalty shots ──────────────────────────────────────

// 9. team_timeout — StartTeamTimeout(Black) mid-half, EndTimeout, resume play.
static TEAM_TIMEOUT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
    ),
    (0, StartClock),
    (10, StartTeamTimeout(Color::Black)),
    (20, EndTimeout),
];

// 10. ref_timeout — StartRefTimeout mid-half, EndTimeout.
static REF_TIMEOUT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
    ),
    (0, StartClock),
    (10, StartRefTimeout),
    (20, EndTimeout),
];

// 11. penalty_shot — StartPenaltyShot, EndTimeout.
static PENALTY_SHOT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
    ),
    (0, StartClock),
    (10, StartPenaltyShot),
    (20, EndTimeout),
];

// 12. rugby_penalty_shot — StartRugbyPenaltyShot (game clock keeps running), EndTimeout.
static RUGBY_PENALTY_SHOT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
    ),
    (0, StartClock),
    (10, StartRugbyPenaltyShot),
    (20, EndTimeout),
];

// 13. timeout_near_period_end — team timeout a few seconds before first half ends.
//     Half = 20 s; timeout called at t=15 (5 s left on game clock); EndTimeout at t=25.
static TIMEOUT_NEAR_PERIOD_END_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (15, StartTeamTimeout(Color::White)),
    (25, EndTimeout),
];

// 14. timeout_freezes_penalty — penalty ticking, then team timeout mid-countdown,
//     then EndTimeout.  The penalty value must be frozen during the timeout and
//     resume from the frozen value after.
static TIMEOUT_FREEZES_PENALTY_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(80)),
    ),
    (0, StartClock),
    (2, StartPenalty(Color::Black, 8, PenaltyKind::OneMinute)),
    (10, StartTeamTimeout(Color::White)),
    (25, EndTimeout),
];

// ── Family 4 — Overtime & sudden death ───────────────────────────────────────

// 15. overtime_full — 0-0 at end of regulation; OT enabled; run through all
//     four OT periods.  Starts at SecondHalf 5 s remaining so we quickly reach
//     the end of regulation.
static OVERTIME_FULL_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::SecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
];

// 16. sudden_death — 0-0 through all of overtime → PreSuddenDeath → SuddenDeath;
//     AddScore ends the game.  Starts at OvertimeSecondHalf 5 s remaining.
static SUDDEN_DEATH_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::OvertimeSecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
    // After ~5 s we'll be in PreSuddenDeath, then SuddenDeath starts (count-up).
    // Add a goal at t=15 (well into SuddenDeath) to end the game.
    (15, AddScore(Color::Black)),
];

// 17. overtime_score_ends — a score during OvertimeSecondHalf breaks the tie
//     and ends the game (OT second half ends with non-equal scores → end_game).
//     Starts with OvertimeSecondHalf 20 s remaining; score at t=8.
static OVERTIME_SCORE_ENDS_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::OvertimeSecondHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (8, AddScore(Color::White)),
];

// ── Special sub-cases ─────────────────────────────────────────────────────────

// 18. score_confirm_pause — drive to end-of-game via SecondHalf expiry;
//     the could_end_game → pause_for_confirm path must fire.
//     Config: overtime_allowed=false, sudden_death_allowed=false.
//     SecondHalf 5 s remaining; scores 0-0 at end; driver pauses for confirm.
//     minimum_break=6 s → confirm pause ≈ 3 s; run 20 s to capture the pause
//     and then BetweenGames.
static SCORE_CONFIRM_PAUSE_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::SecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
];

// 19. single_half — single_half=true; one period (FirstHalf) instead of two.
//     Ends correctly after the single half.  Add a score so the game is
//     unequal → end_game fires directly (no confirm pause path for FirstHalf).
static SINGLE_HALF_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (5, AddScore(Color::Black)),
];

// 20. manual_clock_edit — StopClock, SetGameClock(new value), StartClock;
//     the clock must jump to the new value and continue counting from there.
static MANUAL_CLOCK_EDIT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(60)),
    ),
    (0, StartClock),
    (10, StopClock),
    (10, SetGameClock(Duration::from_secs(100))),
    (15, StartClock),
];

// ── Public entry point ────────────────────────────────────────────────────────

/// Return every scenario in the library.
///
/// Task 4 will iterate this list to run each scenario against its golden file.
pub(super) fn all() -> Vec<Scenario> {
    vec![
        // ── Family 1 — Regulation flow ──────────────────────────────────────
        Scenario {
            name: "regulation_full",
            config: reg_config(),
            actions: REGULATION_FULL_ACTIONS,
            run_secs: 50, // FirstHalf 20s + HalfTime 8s + SecondHalf (partial)
        },
        Scenario {
            name: "regulation_with_scores",
            config: reg_config(),
            actions: REGULATION_WITH_SCORES_ACTIONS,
            run_secs: 50,
        },
        // ── Family 2 — Penalties over time ───────────────────────────────────
        Scenario {
            name: "penalty_one_minute",
            config: GameConfig {
                half_play_duration: Duration::from_secs(80),
                ..reg_config()
            },
            actions: PENALTY_ONE_MINUTE_ACTIONS,
            run_secs: 75, // penalty expires at ~t=65, well before half ends at 80s
        },
        Scenario {
            name: "penalty_crosses_break",
            config: reg_config(), // half=20s, halftime=8s
            actions: PENALTY_CROSSES_BREAK_ACTIONS,
            run_secs: 65, // 20s first half + 8s halftime + 37s into second half (penalty expires)
        },
        Scenario {
            name: "penalty_concurrent",
            config: GameConfig {
                half_play_duration: Duration::from_secs(90),
                ..reg_config()
            },
            actions: PENALTY_CONCURRENT_ACTIONS,
            run_secs: 80, // OneMinute expires ~t=62, TwoMinute still running
        },
        Scenario {
            name: "penalty_during_stoppage",
            config: GameConfig {
                half_play_duration: Duration::from_secs(40),
                ..reg_config()
            },
            actions: PENALTY_DURING_STOPPAGE_ACTIONS,
            run_secs: 55,
        },
        Scenario {
            name: "penalty_total_dismissal",
            config: GameConfig {
                half_play_duration: Duration::from_secs(40),
                ..reg_config()
            },
            actions: PENALTY_TOTAL_DISMISSAL_ACTIONS,
            run_secs: 50,
        },
        Scenario {
            name: "penalty_expires_at_boundary",
            config: GameConfig {
                half_play_duration: Duration::from_secs(30),
                ..reg_config()
            },
            actions: PENALTY_EXPIRES_AT_BOUNDARY_ACTIONS,
            run_secs: 45, // see the boundary and the period transition
        },
        // ── Family 3 — Timeouts & penalty shots ─────────────────────────────
        Scenario {
            name: "team_timeout",
            config: reg_config(),
            actions: TEAM_TIMEOUT_ACTIONS,
            run_secs: 45,
        },
        Scenario {
            name: "ref_timeout",
            config: reg_config(),
            actions: REF_TIMEOUT_ACTIONS,
            run_secs: 45,
        },
        Scenario {
            name: "penalty_shot",
            config: reg_config(),
            actions: PENALTY_SHOT_ACTIONS,
            run_secs: 45,
        },
        Scenario {
            name: "rugby_penalty_shot",
            config: reg_config(),
            actions: RUGBY_PENALTY_SHOT_ACTIONS,
            run_secs: 45,
        },
        Scenario {
            name: "timeout_near_period_end",
            config: reg_config(),
            actions: TIMEOUT_NEAR_PERIOD_END_ACTIONS,
            run_secs: 55, // timeout near end of first half, then into second half
        },
        Scenario {
            name: "timeout_freezes_penalty",
            config: GameConfig {
                half_play_duration: Duration::from_secs(80),
                ..reg_config()
            },
            actions: TIMEOUT_FREEZES_PENALTY_ACTIONS,
            run_secs: 80,
        },
        // ── Family 4 — Overtime & sudden death ──────────────────────────────
        Scenario {
            name: "overtime_full",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: false,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(10),
                ot_half_time_duration: Duration::from_secs(5),
                ..reg_config()
            },
            actions: OVERTIME_FULL_ACTIONS,
            run_secs: 45, // 5s SecondHalf + 5s PreOT + 10s OTFirst + 5s OTHalfTime + 10s OTSecond
        },
        Scenario {
            name: "sudden_death",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: true,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(10),
                ot_half_time_duration: Duration::from_secs(5),
                pre_sudden_death_duration: Duration::from_secs(5),
                ..reg_config()
            },
            actions: SUDDEN_DEATH_ACTIONS,
            run_secs: 30,
        },
        Scenario {
            name: "overtime_score_ends",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: false,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(20),
                ot_half_time_duration: Duration::from_secs(5),
                ..reg_config()
            },
            actions: OVERTIME_SCORE_ENDS_ACTIONS,
            run_secs: 30,
        },
        // ── Special sub-cases ────────────────────────────────────────────────
        Scenario {
            name: "score_confirm_pause",
            config: reg_config(), // overtime_allowed=false, sudden_death_allowed=false
            actions: SCORE_CONFIRM_PAUSE_ACTIONS,
            run_secs: 20,
        },
        Scenario {
            name: "single_half",
            config: GameConfig {
                single_half: true,
                overtime_allowed: false,
                sudden_death_allowed: false,
                half_play_duration: Duration::from_secs(20),
                ..reg_config()
            },
            actions: SINGLE_HALF_ACTIONS,
            run_secs: 35,
        },
        Scenario {
            name: "manual_clock_edit",
            config: GameConfig {
                half_play_duration: Duration::from_secs(120),
                ..reg_config()
            },
            actions: MANUAL_CLOCK_EDIT_ACTIONS,
            run_secs: 60,
        },
    ]
}
