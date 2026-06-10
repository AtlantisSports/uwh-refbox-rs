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
        AddScore, ConfirmScore, EndTimeout, ResetGame, ScoreSuddenDeath, SetGameClock, SetupPeriod,
        StartClock, StartPenalty, StartPenaltyShot, StartPlayNow, StartRefTimeout,
        StartRugbyPenaltyShot, StartTeamTimeout, StopClock,
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

/// Short two-half regulation config with NO overtime/sudden-death, used by the
/// between-games scenario so a full game completes quickly and lands in
/// BetweenGames. Breaks are short so the auto-reset (which fires
/// `post_game_duration` into the between-games countdown) is reached fast.
fn between_games_config() -> GameConfig {
    GameConfig {
        half_play_duration: Duration::from_secs(3),
        half_time_duration: Duration::from_secs(2),
        overtime_allowed: false,
        sudden_death_allowed: false,
        post_game_duration: Duration::from_secs(2),
        nominal_break: Duration::from_secs(6),
        minimum_break: Duration::from_secs(4),
        // Short next-game slot so the between-games clock is a small, readable
        // number (the default game_block is 2880s). The break counts down from
        // ~slot-minus-game-length and the auto-reset fires post_game_duration in.
        game_block: Duration::from_secs(20),
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
//     score during SuddenDeath triggers the confirm-pause gate before ending the game.
//     Starts at OvertimeSecondHalf 5 s remaining.
//
//     The SD confirm-pause window = minimum_break / 2.  minimum_break is set to 20 s
//     here so the window is 10 s — wide enough for the ScoreSuddenDeath→ConfirmScore
//     gap (t=15 → t=17, 2 s) to always land inside it and exercise the manual path.
//
//     Expected trace:
//       OvertimeSecondHalf count-down → PreSuddenDeath break → SuddenDeath count-up
//       → conf_pause=Ns appears (ScoreSuddenDeath fires pause_for_confirm)
//       → ConfirmScore applies score + ends pause → BetweenGames
static SUDDEN_DEATH_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::OvertimeSecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
    // At ~t=5 we enter PreSuddenDeath (5 s break), then SuddenDeath starts (count-up).
    // ScoreSuddenDeath at t=15: calls pause_for_confirm; score held, not yet applied.
    // ConfirmScore at t=17: applies held score + ends pause → BetweenGames.
    (15, ScoreSuddenDeath(Color::Black)),
    (17, ConfirmScore(Color::Black)),
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

// ── Family 5 — Penalty interactions ──────────────────────────────────────────

// 21. penalty_started_during_timeout — start a team timeout, then while the timeout
//     is active start a penalty.  Pin whether the penalty clock ticks during the
//     timeout or waits until play resumes.
//
//     Half = 60 s.  Clock running.  Team timeout (Black) at t=5.
//     StartPenalty (White #2, ThirtySecond) at t=8 — timeout still active.
//     EndTimeout at t=20.  Run to 50 s to see penalty count down after timeout.
static PENALTY_STARTED_DURING_TIMEOUT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(60)),
    ),
    (0, StartClock),
    (5, StartTeamTimeout(Color::Black)),
    (8, StartPenalty(Color::White, 2, PenaltyKind::ThirtySecond)),
    (20, EndTimeout),
];

// 22. penalty_during_confirm_pause — a penalty is still active when the game reaches
//     end-of-game and enters the confirm pause.  SecondHalf 5 s remaining (no OT/SD)
//     so the game ends quickly.  Penalty (TwoMinute, Black #1) started at t=0 — it
//     will still have ~115 s remaining when the confirm pause fires.
//     Pin how the penalty renders in the conf_pause snapshot.
static PENALTY_DURING_CONFIRM_PAUSE_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::SecondHalf, Duration::from_secs(5)),
    ),
    (0, StartPenalty(Color::Black, 1, PenaltyKind::TwoMinute)),
    (0, StartClock),
];

// 23. manual_clock_edit_while_penalty_running — penalty counting down; stop the game
//     clock, set it to a new value, restart.  Pin that the penalty clock is unaffected
//     by the game-clock edit (penalty continues its own countdown).
//
//     Half = 60 s.  B#6 ThirtySecond starts at t=2.  StopClock at t=10.
//     SetGameClock(50 s) at t=10 (jumps clock forward to 50 s remaining).
//     StartClock at t=15.  Run to t=45 to observe penalty countdown through edit.
static MANUAL_CLOCK_EDIT_WHILE_PENALTY_RUNNING_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(60)),
    ),
    (0, StartClock),
    (2, StartPenalty(Color::Black, 6, PenaltyKind::ThirtySecond)),
    (10, StopClock),
    (10, SetGameClock(Duration::from_secs(50))),
    (15, StartClock),
];

// ── Family 6 — Timeout interactions ──────────────────────────────────────────

// 24. timeout_ending_at_period_boundary — EndTimeout fires at the same virtual
//     second that the half expires.  Half = 20 s.  Team timeout (White) called at
//     t=15 (5 s of game clock remain).  EndTimeout at t=20 — the exact whole-second
//     at which 5 s of game time would have elapsed.  Pin the ordering of timeout-end
//     and period transition.
static TIMEOUT_ENDING_AT_PERIOD_BOUNDARY_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (15, StartTeamTimeout(Color::White)),
    // EndTimeout at t=20.  At this point 5 s of game time have elapsed since the
    // timeout was called; the 5 s remaining on the game clock were frozen during
    // the timeout, so the period has NOT yet ended — EndTimeout resumes the clock
    // and the 5 remaining seconds count down normally.
    (20, EndTimeout),
];

// 25. timeout_during_overtime — drive to OvertimeFirstHalf (tie at end of regulation,
//     overtime allowed), call a ref timeout during it, EndTimeout, resume.
//     (Team timeouts are only allowed in regulation halves, not OT; ref timeouts are
//     available in any play period.)
//     Starts at OvertimeFirstHalf 20 s remaining.  RefTimeout at t=8, EndTimeout
//     at t=18.  Run to 40 s to show OT half-time and OTSecondHalf.
static TIMEOUT_DURING_OVERTIME_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::OvertimeFirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (8, StartRefTimeout),
    (18, EndTimeout),
];

// ── Family 7 — Overtime / Sudden Death interactions ───────────────────────────

// 26. overtime_with_active_penalty — penalty started near end of SecondHalf ticks
//     across the regulation → PreOvertime → OvertimeFirstHalf transition.
//     SecondHalf 5 s remaining; OneMinute penalty for White #3 started at t=2.
//     At end of SecondHalf the penalty still has ~57 s remaining; it should
//     continue ticking through the break and into OT.
static OVERTIME_WITH_ACTIVE_PENALTY_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::SecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
    (2, StartPenalty(Color::White, 3, PenaltyKind::OneMinute)),
];

// 27. sudden_death_with_timeout — reach SuddenDeath (0-0 through OT),
//     call a ref timeout during SD, EndTimeout, resume.  Does NOT score (no SD
//     confirm flow needed).  Starts at OvertimeSecondHalf 5 s remaining.
//     RefTimeout at t=15 (into SuddenDeath), EndTimeout at t=25.
static SUDDEN_DEATH_WITH_TIMEOUT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::OvertimeSecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
    // ~t=5: OT second half ends → PreSuddenDeath (5 s break) → SuddenDeath starts
    (15, StartRefTimeout),
    (25, EndTimeout),
];

// ── Family 8 — Manual-edit / boundary edge cases ─────────────────────────────

// 28. manual_clock_edit_near_period_boundary — StopClock, SetGameClock to a small
//     value (3 s), StartClock; the clock rolls into the next period a few seconds
//     later.  Pin the period transition after a manual edit.
//
//     Half = 60 s.  StopClock at t=10.  SetGameClock(3 s) at t=10.
//     StartClock at t=15.  Run to 30 s; the period ends ~3 s after clock restart.
static MANUAL_CLOCK_EDIT_NEAR_PERIOD_BOUNDARY_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(60)),
    ),
    (0, StartClock),
    (10, StopClock),
    (10, SetGameClock(Duration::from_secs(3))),
    (15, StartClock),
];

// 29. manual_clock_edit_to_zero — StopClock, SetGameClock(Duration::ZERO), StartClock.
//     Pin whatever end-of-period logic the engine triggers when the clock is manually
//     set to zero.
//
//     Half = 60 s.  StopClock at t=5.  SetGameClock(0) at t=5.
//     StartClock at t=10.  Run to 25 s to observe resulting period/state.
static MANUAL_CLOCK_EDIT_TO_ZERO_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(60)),
    ),
    (0, StartClock),
    (5, StopClock),
    (5, SetGameClock(Duration::ZERO)),
    (10, StartClock),
];

// 30. actions_at_exact_transition_instant — a StartTeamTimeout fires at offset 20,
//     the exact whole-second at which a 20 s FirstHalf would expire.  Tests the
//     fixed-step driver's handling of an action coinciding with a period transition.
//     Pin the result: does the timeout fire in FirstHalf or after the transition?
//
//     Half = 20 s.  StartClock at t=0.  StartTeamTimeout(Black) at t=20 (boundary).
//     EndTimeout at t=30.  Run to 60 s.
static ACTIONS_AT_EXACT_TRANSITION_INSTANT_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (20, StartTeamTimeout(Color::Black)),
    (30, EndTimeout),
];

// ── Family 9 — mutation-coverage gaps (follow-up A) ──────────────────────────
// Each scenario here closes a gap the cargo-mutants validation found: a path the
// original 30 scenarios never exercised, so a mutation there survived. See
// docs/superpowers/specs/2026-06-09-golden-trace-missing-scenarios-design.md.

// Sudden death reached directly from regulation (overtime disabled). Exercises the
// SecondHalf -> PreSuddenDeath confirm-pause branch (pause_for_confirm SD arm).
static SUDDEN_DEATH_NO_OVERTIME_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::SecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
];

// Manual clock edit that rewinds the clock to before a running penalty started,
// so the penalty's remaining time exceeds its full duration and the rebase branch
// in set_game_clock_time fires (resetting the penalty's start to the edited clock).
static MANUAL_CLOCK_EDIT_REWINDS_PENALTY_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (2, StartPenalty(Color::Black, 7, PenaltyKind::OneMinute)), // starts at clock=18s
    (5, StopClock),                                             // clock=15s
    (5, SetGameClock(Duration::from_secs(19))),                 // rewind to before penalty start
    (6, StartClock),
];

// Team timeout left to expire on the clock (no EndTimeout): update() ends it and
// resumes the game clock. Exercises the natural team-timeout-expiry path in update.
static TEAM_TIMEOUT_EXPIRES_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20)),
    ),
    (0, StartClock),
    (3, StartTeamTimeout(Color::Black)), // game clock stops at 17s; timeout 15s expires at t=18
];

// Rugby penalty shot still running when the period clock hits 0: the half is
// extended (game clock stops at 0), and ending the shot then drives the period
// transition through handle_rugby_pen_shot_end with the clock STOPPED -- the only
// path that reaches update:1348 and the clock-stopped block (1443/1478). No EndTimeout.
static RUGBY_PENALTY_SHOT_EXPIRES_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
    (1, StartRugbyPenaltyShot), // 15s shot; half clock hits 0 at t=5 -> extended; shot ends ~t=16
];

// Single-half game that ends tied and continues to overtime: exercises the
// single_half branch of end_first_half (-> PreOvertime, not game end).
static SINGLE_HALF_TO_OVERTIME_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(10)),
    ),
    (0, StartClock),
];

// Single-half game DECIDED by a score (overtime enabled but not needed): the half
// ends with a winner -> end_game. With a non-tie score, are_not_equal() is true so
// the `||` short-circuits, distinguishing it from `&&`: this is the config that
// exercises the second clause of end_first_half's single_half branch (1368).
static SINGLE_HALF_DECIDED_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(10)),
    ),
    (0, StartClock),
    (3, AddScore(Color::Black)), // 1-0: decided, so the single half ends the game
];

// Single-half game that ends in a DRAW with no overtime/sudden-death configured:
// the half ends the game directly. With a tie (are_not_equal() false), the second
// clause of end_first_half's single_half branch decides the outcome, so flipping the
// sudden-death term (1368) changes BetweenGames -> PreSuddenDeath (observable).
static SINGLE_HALF_DRAWN_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(10)),
    ),
    (0, StartClock),
];

// ── Family 10 — between-games lifecycle ───────────────────────────────────────
//
// between_games_auto_reset — exercises is_old_game and the auto-reset at
// mod.rs:1180-1182. StartPlayNow begins a real game (start_game sets
// has_reset = false → is_old_game = true → old?=Y). The game plays FirstHalf →
// HalfTime → SecondHalf, ends 0-0, and enters BetweenGames with old?=Y. Once the
// between-games clock counts down past reset_game_time the engine auto-resets,
// flipping is_old_game → old?=N. run_secs stops a few seconds after the flip.
//
// WHY a full game must complete (not just SetupPeriod(BetweenGames, …)): the
// auto-reset only fires while `!has_reset`, and `has_reset` is set false ONLY by
// `start_game` (mod.rs:1076). SetupPeriod does not touch `has_reset`, so a
// constructed-then-SetupPeriod game keeps `has_reset = true`, the reset never
// fires, and is_old_game never flips. StartPlayNow → start_game is the only
// faithful way to reach the precondition.
//
// CAVEAT (validation): unlike the other scenarios, this one cannot be compared
// against the human baseline 46ec0973 — the between-games clock is computed via
// the post-baseline Game Block feature (`game_block` did not exist then). Its
// trace therefore pins CURRENT behavior, not a trusted baseline. Its value rests
// on (a) structural correctness (the Y→N flip at reset_game_time) and (b)
// demonstrated mutation sensitivity: the cargo-mutants sweep confirms the two
// reset mutants at mod.rs:1181/1182 and the is_old_game assembly mutant at
// mod.rs:2196 are all caught by the `old?` column.
static BETWEEN_GAMES_AUTO_RESET_ACTIONS: &[(u64, Action)] = &[(0, StartPlayNow)];

// manual_reset_game — exercises the operator's manual end-game reset (`reset_game`).
// StartPlayNow begins a real game (old?=Y); a Black goal makes the score B1/W0; then
// ResetGame ends the game → BetweenGames with the score zeroed (B0/W0) and has_reset=true
// (old?=N), clock restarted at minimum_break. Kills the `reset_game`-body mutant (mod.rs:202):
// with the reset stubbed out, none of period/score/old? change. Reuses between_games_config
// (short two-half, no OT/SD) so the reset lands mid-FirstHalf and the trace is compact.
static MANUAL_RESET_GAME_ACTIONS: &[(u64, Action)] = &[
    (0, StartPlayNow),
    (1, AddScore(Color::Black)),
    (2, ResetGame),
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
            run_secs: 55, // FirstHalf 20s + HalfTime 8s + SecondHalf 20s + conf_pause 3s + margin
        },
        Scenario {
            name: "regulation_with_scores",
            config: reg_config(),
            actions: REGULATION_WITH_SCORES_ACTIONS,
            run_secs: 55, // same timing as regulation_full; scores don't change period transitions
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
                // minimum_break=20 s → SD confirm-pause window = 20/2 = 10 s.
                // This ensures ScoreSuddenDeath (t=15) → ConfirmScore (t=17) lands
                // inside the window and exercises the manual confirm path.
                minimum_break: Duration::from_secs(20),
                ..reg_config()
            },
            actions: SUDDEN_DEATH_ACTIONS,
            run_secs: 35,
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
        // ── Family 5 — Penalty interactions ─────────────────────────────────
        Scenario {
            name: "penalty_started_during_timeout",
            config: GameConfig {
                half_play_duration: Duration::from_secs(60),
                ..reg_config()
            },
            actions: PENALTY_STARTED_DURING_TIMEOUT_ACTIONS,
            run_secs: 50,
        },
        Scenario {
            name: "penalty_during_confirm_pause",
            config: reg_config(), // overtime_allowed=false, sudden_death_allowed=false
            actions: PENALTY_DURING_CONFIRM_PAUSE_ACTIONS,
            run_secs: 20,
        },
        Scenario {
            name: "manual_clock_edit_while_penalty_running",
            config: GameConfig {
                half_play_duration: Duration::from_secs(60),
                ..reg_config()
            },
            actions: MANUAL_CLOCK_EDIT_WHILE_PENALTY_RUNNING_ACTIONS,
            run_secs: 45,
        },
        // ── Family 6 — Timeout interactions ─────────────────────────────────
        Scenario {
            name: "timeout_ending_at_period_boundary",
            config: reg_config(), // half=20s
            actions: TIMEOUT_ENDING_AT_PERIOD_BOUNDARY_ACTIONS,
            run_secs: 55, // through second half and into end-of-game
        },
        Scenario {
            name: "timeout_during_overtime",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: false,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(20),
                ot_half_time_duration: Duration::from_secs(5),
                ..reg_config()
            },
            actions: TIMEOUT_DURING_OVERTIME_ACTIONS,
            run_secs: 40, // OTFirstHalf + OTHalfTime + into OTSecondHalf
        },
        // ── Family 7 — Overtime / Sudden Death interactions ──────────────────
        Scenario {
            name: "overtime_with_active_penalty",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: false,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(15),
                ot_half_time_duration: Duration::from_secs(5),
                ..reg_config()
            },
            actions: OVERTIME_WITH_ACTIVE_PENALTY_ACTIONS,
            run_secs: 50, // penalty (60s) ticks through OT break and into OTFirstHalf
        },
        Scenario {
            name: "sudden_death_with_timeout",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: true,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(5),
                ot_half_time_duration: Duration::from_secs(3),
                pre_sudden_death_duration: Duration::from_secs(5),
                minimum_break: Duration::from_secs(20),
                ..reg_config()
            },
            actions: SUDDEN_DEATH_WITH_TIMEOUT_ACTIONS,
            run_secs: 40,
        },
        // ── Family 8 — Manual-edit / boundary edge cases ─────────────────────
        Scenario {
            name: "manual_clock_edit_near_period_boundary",
            config: GameConfig {
                half_play_duration: Duration::from_secs(60),
                ..reg_config()
            },
            actions: MANUAL_CLOCK_EDIT_NEAR_PERIOD_BOUNDARY_ACTIONS,
            run_secs: 30,
        },
        Scenario {
            name: "manual_clock_edit_to_zero",
            config: GameConfig {
                half_play_duration: Duration::from_secs(60),
                ..reg_config()
            },
            actions: MANUAL_CLOCK_EDIT_TO_ZERO_ACTIONS,
            run_secs: 25,
        },
        Scenario {
            name: "actions_at_exact_transition_instant",
            config: reg_config(), // half=20s
            actions: ACTIONS_AT_EXACT_TRANSITION_INSTANT_ACTIONS,
            run_secs: 60,
        },
        // ── Family 9 — mutation-coverage gaps (follow-up A) ──────────────────
        Scenario {
            name: "sudden_death_no_overtime",
            config: GameConfig {
                overtime_allowed: false,
                sudden_death_allowed: true,
                ..reg_config()
            },
            actions: SUDDEN_DEATH_NO_OVERTIME_ACTIONS,
            run_secs: 18, // SecondHalf 5s + conf_pause ~2.5s + PreSuddenDeath 5s + into SuddenDeath
        },
        Scenario {
            name: "manual_clock_edit_rewinds_penalty",
            config: reg_config(), // FirstHalf 20s
            actions: MANUAL_CLOCK_EDIT_REWINDS_PENALTY_ACTIONS,
            run_secs: 25,
        },
        Scenario {
            name: "team_timeout_expires",
            config: reg_config(), // team_timeout_duration=15s, FirstHalf 20s
            actions: TEAM_TIMEOUT_EXPIRES_ACTIONS,
            run_secs: 25,
        },
        Scenario {
            name: "rugby_penalty_shot_expires",
            config: reg_config(), // penalty_shot_duration=15s; FirstHalf set to 5s so the shot extends it
            actions: RUGBY_PENALTY_SHOT_EXPIRES_ACTIONS,
            run_secs: 28, // shot ends ~t=16 -> HalfTime 8s -> into SecondHalf
        },
        Scenario {
            name: "single_half_to_overtime",
            config: GameConfig {
                single_half: true,
                overtime_allowed: true,
                sudden_death_allowed: false,
                ..reg_config()
            },
            actions: SINGLE_HALF_TO_OVERTIME_ACTIONS,
            run_secs: 25,
        },
        Scenario {
            name: "single_half_decided",
            config: GameConfig {
                single_half: true,
                overtime_allowed: true,
                sudden_death_allowed: false,
                ..reg_config()
            },
            actions: SINGLE_HALF_DECIDED_ACTIONS,
            run_secs: 20,
        },
        Scenario {
            name: "single_half_drawn",
            config: GameConfig {
                single_half: true,
                overtime_allowed: false,
                sudden_death_allowed: false,
                ..reg_config()
            },
            actions: SINGLE_HALF_DRAWN_ACTIONS,
            run_secs: 20,
        },
        // ── Family 10 — between-games lifecycle ──────────────────────────────
        Scenario {
            name: "between_games_auto_reset",
            config: between_games_config(),
            actions: BETWEEN_GAMES_AUTO_RESET_ACTIONS,
            run_secs: 14,
        },
        Scenario {
            name: "manual_reset_game",
            config: between_games_config(),
            actions: MANUAL_RESET_GAME_ACTIONS,
            run_secs: 7,
        },
    ]
}
