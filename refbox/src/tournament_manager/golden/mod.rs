//! Golden-trace regression driver for `TournamentManager`.
//!
//! This module provides a fixed-step replay driver that runs test scenarios through
//! `TournamentManager` and emits a deduplicated, state-change-keyed text trace.
//! The resulting `Vec<String>` can be compared against a saved "golden" file to detect
//! regressions in the time engine.
//!
//! # Design notes
//!
//! * Fixed 100 ms step (finding #1 from the feasibility spike): `next_update_time` returns
//!   `now` on whole-second boundaries and would cause the replay to hang. Dense fixed-step
//!   ticks are used instead.
//!
//! * No timestamp in the trace output (finding #2): lines are keyed on observed state only,
//!   so the trace is stable even when the step size changes.
//!
//! * Fixed-step faithfulness: this driver is faithful as long as `update` recomputes state
//!   purely from `start_time + elapsed` without accumulating per-call state. If the engine
//!   ever accumulates per-call state, this driver could diverge from the real app.

use super::*;
use uwh_common::game_snapshot::{PenaltyTime, TimeoutSnapshot};

pub(super) mod scenarios;

// ─── Public types ─────────────────────────────────────────────────────────────

/// Every action a scenario can inject at a given time offset.
//
// Several variants are unused by the smoke test but are part of the stable public API
// of this driver module — Tasks 3+ will exercise them. Suppress the warning here rather
// than leaving them as `todo!()` stubs.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(super) enum Action {
    /// Set the active period and remaining clock time (test-only engine method).
    SetupPeriod(GamePeriod, Duration),
    /// Start the game clock (and any active timeout clock).
    StartClock,
    /// Stop the game clock.
    StopClock,
    /// Record a goal for the given colour (player number 0).
    ///
    /// Mirrors `Message::AddNewScore` in the non-SuddenDeath path: calls
    /// `tm.add_score(color, 0, now)` directly.  Do NOT use this variant in
    /// SuddenDeath — use [`Action::ScoreSuddenDeath`] + [`Action::ConfirmScore`] instead.
    AddScore(Color),
    /// Mirror the SuddenDeath score-entry in `app/mod.rs` (AddScoreComplete, SD branch):
    ///   the held score is incremented locally, then `tm.pause_for_confirm(now)` is called.
    ///   The engine score is NOT changed yet; the increment is conceptually held by the
    ///   operator until `ConfirmScore` fires.
    ///
    /// Cross-reference: `app/mod.rs` ~line 2048–2053.
    ScoreSuddenDeath(Color),
    /// Mirror the operator confirming a SuddenDeath score (`Message::ScoreConfirmation { correct: true }`):
    ///   recomputes the held score, calls `tm.set_scores(held, now)` then `tm.end_confirm_pause(now)`.
    ///
    /// Cross-reference: `app/mod.rs` ~line 2907–2911.
    ConfirmScore(Color),
    /// Start a timed penalty for `(color, player_number, kind)`.
    StartPenalty(Color, u8, PenaltyKind),
    /// Start a team timeout for the given colour.
    StartTeamTimeout(Color),
    /// Start a referee timeout.
    StartRefTimeout,
    /// Start a (non-rugby) penalty shot.
    StartPenaltyShot,
    /// Start a rugby penalty shot.
    StartRugbyPenaltyShot,
    /// End the current timeout and resume play.
    EndTimeout,
    /// Manually set the game clock to the given duration (clock must be stopped).
    SetGameClock(Duration),
}

/// A single replay scenario.
pub(super) struct Scenario {
    /// Human-readable identifier used in assertion messages.
    pub name: &'static str,
    /// Game configuration for this scenario.
    pub config: GameConfig,
    /// Timed actions: `(offset_secs, action)`.  The list must be sorted by offset ascending.
    pub actions: &'static [(u64, Action)],
    /// How many virtual seconds to run before stopping the replay.
    pub run_secs: u64,
}

// ─── Driver internals ─────────────────────────────────────────────────────────

/// Attempt to generate a snapshot; if the engine is momentarily unable to produce one
/// (e.g. it needs an `update` pass first), retry up to 4 times.
///
/// This mirrors the retry loop in `app/mod.rs` around line 4186:
/// ```text
/// let mut i = 0;
/// let snapshot = loop {
///     if i > 4 { panic!("No snapshot"); }
///     match tm_.generate_snapshot(now) {
///         Some(val) => break val,
///         None => { tm_.update(now).unwrap(); i += 1; }
///     }
/// };
/// ```
fn snapshot_with_retry(tm: &mut TournamentManager, now: Instant) -> GameSnapshot {
    let mut i = 0;
    loop {
        assert!(
            i <= 4,
            "no snapshot after 5 attempts (mirrors the panic path in app/mod.rs)"
        );
        match tm.generate_snapshot(now) {
            Some(s) => return s,
            None => {
                tm.update(now).unwrap();
                i += 1;
            }
        }
    }
}

/// One tick of the game loop, mirroring the per-frame tick in `app/mod.rs` (≈ lines 4174-4183):
/// ```text
/// if tm_.could_end_game(now).unwrap() {
///     tm_.pause_for_confirm(now).unwrap();
/// } else if tm_.pause_has_ended(now) {
///     tm_.end_confirm_pause(now).unwrap();
/// } else {
///     tm_.update(now).unwrap();
/// }
/// ```
/// Unlike the spike this function does NOT call or return `next_update_time`.
fn tick(tm: &mut TournamentManager, now: Instant) {
    if tm.could_end_game(now).unwrap() {
        tm.pause_for_confirm(now).unwrap();
    } else if tm.pause_has_ended(now) {
        tm.end_confirm_pause(now).unwrap();
    } else {
        tm.update(now).unwrap();
    }
}

// ─── KNOWN COUPLING POINT ────────────────────────────────────────────────────
//
// `apply_action` is a hand-copy of the real action handlers in `app/mod.rs`.
// If any handler in that file ever changes which `TournamentManager` methods it calls
// (argument list, call order, extra follow-up calls, etc.), the corresponding arm below
// MUST be updated in lockstep or the golden traces will silently stop reflecting the
// real application.
//
// Cross-reference targets in `app/mod.rs` (as of master at the time this was written):
//   StartClock           → Message::StartPlayNow / manual start_clock call
//   StopClock            → Message::EditTime  (stop_clock + clock_is_running check)
//   AddScore             → Message::AddNewScore  (add_score(color, 0, now); non-SD path)
//   ScoreSuddenDeath     → Message::AddScoreComplete SD branch (~line 2048–2053):
//                          hold score locally, pause_for_confirm(now)
//   ConfirmScore         → Message::ScoreConfirmation { correct: true } (~line 2907–2911):
//                          set_scores(held, now) + end_confirm_pause(now)
//   StartPenalty         → penalty_editor.rs add_to_tm → tm.start_penalty(...)
//   StartTeamTimeout     → Message::TeamTimeout  (start_team_timeout)
//   StartRefTimeout      → Message::RefTimeout   (start_ref_timeout)
//   StartPenaltyShot     → Message::PenaltyShot  (start_penalty_shot, UWH mode)
//   StartRugbyPenaltyShot→ Message::PenaltyShot  (start_rugby_penalty_shot, Rugby mode)
//   EndTimeout           → Message::EndTimeout   (end_timeout + update; no game-ending branch here)
//   SetGameClock         → Message::TimeEditComplete (set_game_clock_time)
//   SetupPeriod          → test-only; no real handler (uses pub(super) test method)
//
// CLOCK-LATCH COUPLING: the tick decision in `run()` reads the engine's start/stop watch
// channel via `tm.get_start_stop_rx()`, exactly as the real `time_updater` loop in
// `app/mod.rs` (~line 4132–4165) does.  `apply_action` does NOT maintain any separate
// `clock_running` bool; the engine owns that state and broadcasts it via the latch.
//
// ─────────────────────────────────────────────────────────────────────────────

fn apply_action(tm: &mut TournamentManager, action: Action, now: Instant) {
    match action {
        Action::SetupPeriod(period, clock_time) => {
            tm.set_period_and_game_clock_time(period, clock_time);
        }
        Action::StartClock => {
            tm.start_clock(now);
        }
        Action::StopClock => {
            tm.stop_clock(now).unwrap();
        }
        Action::AddScore(color) => {
            // Mirrors Message::AddNewScore without the collect_scorer_cap_num path.
            // Non-SuddenDeath path only — use ScoreSuddenDeath + ConfirmScore for SD goals.
            tm.add_score(color, 0, now);
        }
        Action::ScoreSuddenDeath(color) => {
            // Mirrors app/mod.rs AddScoreComplete SD branch (~line 2048–2053):
            // the new score is held locally (not yet sent to the engine); the engine
            // is told to enter a confirmation pause so the operator can verify.
            let mut s = tm.get_scores();
            s[color] = s[color].saturating_add(1);
            // NOTE: `s` is the held score — do NOT call tm.set_scores here.
            // The held value is stored implicitly; ConfirmScore recomputes it.
            let _ = s; // suppress unused-variable warning; ConfirmScore will recompute
            tm.pause_for_confirm(now).unwrap();
        }
        Action::ConfirmScore(color) => {
            // Mirrors app/mod.rs ScoreConfirmation { correct: true } (~line 2907–2911):
            // recompute the held score (same increment as ScoreSuddenDeath), apply it
            // to the engine, then end the confirmation pause.
            let mut s = tm.get_scores();
            s[color] = s[color].saturating_add(1);
            tm.set_scores(s, now);
            tm.end_confirm_pause(now).unwrap();
        }
        Action::StartPenalty(color, player_number, kind) => {
            // Mirrors penalty_editor.rs add_to_tm (Infraction::Unknown is the
            // default when no specific infraction is tracked).
            tm.start_penalty(color, player_number, kind, now, Infraction::Unknown)
                .unwrap();
        }
        Action::StartTeamTimeout(color) => {
            // Mirrors Message::TeamTimeout { switch: false }.
            // Does NOT touch the latch — the engine's start_team_timeout does not
            // call send_clock_running, so the latch remains true (tick loop keeps
            // firing, driving the timeout countdown).
            tm.start_team_timeout(color, now).unwrap();
        }
        Action::StartRefTimeout => {
            // Mirrors Message::RefTimeout { switch: false }.
            // Same latch note as StartTeamTimeout.
            tm.start_ref_timeout(now).unwrap();
        }
        Action::StartPenaltyShot => {
            // Mirrors Message::PenaltyShot { switch: false } in UWH mode.
            // Same latch note as StartTeamTimeout.
            tm.start_penalty_shot(now).unwrap();
        }
        Action::StartRugbyPenaltyShot => {
            // Mirrors Message::PenaltyShot { switch: false } in Rugby mode.
            // Same latch note as StartTeamTimeout.
            tm.start_rugby_penalty_shot(now).unwrap();
        }
        Action::EndTimeout => {
            // Mirrors Message::EndTimeout (non-game-ending branch only):
            //   tm.end_timeout(now).unwrap();
            //   tm.update(now).unwrap();
            // The game-ending branch (halt_clock) is not implemented here;
            // scenarios that need to end the game during a timeout should use
            // StopClock + the normal confirm flow.
            tm.end_timeout(now).unwrap();
            tm.update(now).unwrap();
        }
        Action::SetGameClock(duration) => {
            // Mirrors Message::TimeEditComplete (clock must already be stopped)
            tm.set_game_clock_time(duration).unwrap();
        }
    }
}

/// Render a `GameSnapshot` as a single-line state string with no timestamp.
///
/// Format: `period=<P> | clock=<secs>s | timeout=<...> | conf_pause=<none|Ns> | pens=[<...>]`
///
/// Penalties are sorted by (remaining desc, color asc, player# asc) so the
/// order is stable and human-readable.
fn render(snap: &GameSnapshot) -> String {
    let period = format!("{:?}", snap.current_period);

    let timeout = match snap.timeout {
        None => "none".to_string(),
        Some(TimeoutSnapshot::Black(s)) => format!("Black:{s}s"),
        Some(TimeoutSnapshot::White(s)) => format!("White:{s}s"),
        Some(TimeoutSnapshot::Ref(s)) => format!("Ref:{s}s"),
        Some(TimeoutSnapshot::PenaltyShot(s)) => format!("PenaltyShot:{s}s"),
    };

    let conf_pause = match snap.conf_pause_time {
        None => "none".to_string(),
        Some(n) => format!("{n}s"),
    };

    let mut pens: Vec<(i64, char, u8, String)> = Vec::new();
    for (color, list) in snap.penalties.iter() {
        let cchar = match color {
            Color::Black => 'B',
            Color::White => 'W',
        };
        for p in list {
            let (sortkey, disp) = match p.time {
                PenaltyTime::Seconds(n) => (n as i64, format!("{n}")),
                PenaltyTime::TotalDismissal => (i64::MAX, "TD".to_string()),
            };
            pens.push((
                sortkey,
                cchar,
                p.player_number,
                format!("{cchar}#{}:{disp}", p.player_number),
            ));
        }
    }
    // Sort: remaining descending, then color ascending (B before W), then player# ascending.
    pens.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let pens_str = pens
        .iter()
        .map(|x| x.3.clone())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "period={period:<13} | clock={:>3}s | timeout={timeout:<12} | conf_pause={conf_pause:<6} | pens=[{pens_str}]",
        snap.secs_in_period
    )
}

/// Run a scenario through `TournamentManager` and return the deduplicated state trace.
///
/// The trace contains one entry per observed state *change*; unchanged consecutive states
/// are collapsed into the previous entry. There is no timestamp column.
///
/// # Fixed-step loop
///
/// ```text
/// const STEP: Duration = Duration::from_millis(100);
/// ```
///
/// The step is 100 ms (finding #1 from the spike). At each step:
/// 1. Apply any `Scenario::actions` whose offset falls within `(prev_elapsed, elapsed]`.
///    Record the new state immediately after each action if it changed.
/// 2. Read the engine's start/stop latch (`*rx.borrow()`). If true, call `tick()` and
///    record if the state changed.
///
/// # Clock-latch faithfulness
///
/// This driver mirrors the real `time_updater` loop in `app/mod.rs` (~line 4132):
///   ```text
///   let mut clock_running_receiver = tm.lock().unwrap().get_start_stop_rx();
///   ```
/// The latch is read fresh after every action (actions may flip it) and at every tick
/// boundary. The engine is the sole authority on whether the tick loop fires — there is
/// no separate hand-tracked bool in this driver.
pub(super) fn run(scenario: &Scenario) -> Vec<String> {
    const STEP: Duration = Duration::from_millis(100);
    // NOTE: fixed-step is faithful only while `update` is idempotent w.r.t. call
    // frequency (it recomputes state from start_time+elapsed). If the engine ever
    // accumulates per-call state, this driver could diverge from the real app.

    let mut tm = TournamentManager::new(scenario.config.clone());
    let base = Instant::now();
    // Mirror the real time_updater: read the engine's start/stop watch channel.
    // The latch starts `false`; actions that call start_clock flip it to `true`.
    let rx = tm.get_start_stop_rx();
    let mut trace: Vec<String> = Vec::new();
    let mut last: Option<String> = None;

    // Helper: push render(snapshot) onto trace iff it differs from the last entry.
    macro_rules! record {
        ($now:expr) => {{
            let snap = snapshot_with_retry(&mut tm, $now);
            let line = render(&snap);
            if last.as_deref() != Some(&line) {
                trace.push(line.clone());
                last = Some(line);
            }
        }};
    }

    // Apply setup actions: any action at offset 0 is treated as a setup step.
    // These run before the main loop, at virtual time t=base.
    let mut action_index = 0;
    while action_index < scenario.actions.len() && scenario.actions[action_index].0 == 0 {
        let (_, action) = scenario.actions[action_index];
        apply_action(&mut tm, action, base);
        action_index += 1;
    }
    record!(base);

    // Main loop: advance virtual time from STEP to run_secs inclusive.
    let end = Duration::from_secs(scenario.run_secs);
    let mut elapsed = Duration::ZERO;

    while elapsed < end {
        elapsed = (elapsed + STEP).min(end);
        let now = base + elapsed;

        // Apply all actions whose offset falls within the current step window.
        while action_index < scenario.actions.len() {
            let (offset_secs, action) = scenario.actions[action_index];
            let action_at = Duration::from_secs(offset_secs);
            if action_at > elapsed {
                break;
            }
            // Actions at exactly their offset instant, not at `now`.
            let action_now = base + action_at;
            apply_action(&mut tm, action, action_now);
            record!(action_now);
            action_index += 1;
        }

        // Tick the engine at the step boundary if the engine's latch says running.
        // Read the latch AFTER applying due actions (an action may have flipped it).
        if *rx.borrow() {
            tick(&mut tm, now);
            record!(now);
        }
    }

    trace
}

// ─── Golden file harness ──────────────────────────────────────────────────────
//
// These functions are only compiled in test builds. They are defined here
// (not inside the `tests` module) so that Task 4 integration tests can call
// them via `super::golden_path(...)` / `super::check_or_bless(...)` without
// re-exporting from the `tests` module.

#[cfg(test)]
fn golden_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tournament_manager/golden_traces")
        .join(format!("{name}.trace"))
}

/// Compare `trace` against the saved golden file for `name`, or bless it.
///
/// * `bless = true`  — create the directory if needed, write the file, return `Ok(())`.
/// * `bless = false` — read the file; missing → `Err`; mismatch → `Err` with a diff.
///
/// Callers that want to honour the `UPDATE_GOLDEN` env var should pass
/// `bless: std::env::var("UPDATE_GOLDEN").is_ok()`.  The parameter is
/// explicit so that unit tests can control bless mode without touching the
/// process-global env var (which is unsafe in Rust 2024 and inherently racy
/// under parallel test execution).
#[cfg(test)]
fn check_or_bless(name: &str, trace: &[String], bless: bool) -> std::result::Result<(), String> {
    let path = golden_path(name);

    if bless {
        // Create the directory if it doesn't exist yet.
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("failed to create golden_traces dir: {e}"))?;
        }
        // Write the trace, one line per entry, with a trailing newline.
        let content = format!("{}\n", trace.join("\n"));
        std::fs::write(&path, content)
            .map_err(|e| format!("failed to write golden file '{}': {e}", path.display()))?;
        return Ok(());
    }

    // Compare mode: read the file.
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "no golden file for scenario '{name}' at '{}'; \
             re-run with UPDATE_GOLDEN=1 to create it",
            path.display()
        )
    })?;

    // Split into lines and strip the single trailing empty line introduced by the
    // trailing '\n' we wrote during bless.
    let mut expected: Vec<&str> = raw.lines().collect();
    if expected.last() == Some(&"") {
        expected.pop();
    }

    // Line-by-line comparison.
    let actual: Vec<&str> = trace.iter().map(String::as_str).collect();

    if expected == actual {
        return Ok(());
    }

    // Build a human-readable diff string.
    const MAX_SHOWN: usize = 10;
    let mut diff = format!(
        "golden trace mismatch for scenario '{name}':\n  \
         expected {} lines, got {} lines\n",
        expected.len(),
        actual.len()
    );

    let max_len = expected.len().max(actual.len());
    let mut shown = 0;
    let mut extra = 0;
    for i in 0..max_len {
        let exp = expected.get(i).copied().unwrap_or("<missing>");
        let act = actual.get(i).copied().unwrap_or("<missing>");
        if exp != act {
            if shown < MAX_SHOWN {
                diff.push_str(&format!(
                    "  line {}:\n    - {}\n    + {}\n",
                    i + 1,
                    exp,
                    act
                ));
                shown += 1;
            } else {
                extra += 1;
            }
        }
    }
    if extra > 0 {
        diff.push_str(&format!("  ... and {extra} more differences\n"));
    }

    Err(diff)
}

// ─── Smoke test ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The spike scenario: FirstHalf 40 s, penalty B#7 @2 s, stop@15/start@18, run 55 s.
    ///
    /// Assertions:
    ///   1. Two consecutive runs produce identical traces (determinism).
    ///   2. The trace contains a `HalfTime` line (the period transition fired).
    ///   3. The trace contains a fully-counted-down B#7 penalty (`B#7:0`).
    #[test]
    fn smoke_test_spike_scenario() {
        static ACTIONS: &[(u64, Action)] = &[
            (
                0,
                Action::SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(40)),
            ),
            (0, Action::StartClock),
            (
                2,
                Action::StartPenalty(Color::Black, 7, PenaltyKind::ThirtySecond),
            ),
            (15, Action::StopClock),
            (18, Action::StartClock),
        ];

        let scenario = Scenario {
            name: "spike-scenario",
            config: GameConfig {
                half_play_duration: Duration::from_secs(40),
                half_time_duration: Duration::from_secs(10),
                overtime_allowed: false,
                sudden_death_allowed: false,
                ..Default::default()
            },
            actions: ACTIONS,
            run_secs: 55,
        };

        let trace1 = run(&scenario);
        let trace2 = run(&scenario);

        println!("=== Golden trace for '{}' ===", scenario.name);
        for (i, line) in trace1.iter().enumerate() {
            println!("{i:>3}: {line}");
        }

        // 1. Determinism: two runs must be identical.
        assert_eq!(
            trace1, trace2,
            "golden trace is non-deterministic for scenario '{}'",
            scenario.name
        );

        // 2. HalfTime must appear in the trace.
        assert!(
            trace1.iter().any(|l| l.contains("HalfTime")),
            "expected 'HalfTime' in trace for scenario '{}', got:\n{}",
            scenario.name,
            trace1.join("\n")
        );

        // 3. B#7 penalty must reach 0 seconds remaining.
        assert!(
            trace1.iter().any(|l| l.contains("B#7:0")),
            "expected 'B#7:0' in trace for scenario '{}', got:\n{}",
            scenario.name,
            trace1.join("\n")
        );
    }

    /// Self-test for the golden file read/write/compare harness.
    ///
    /// Uses a unique synthetic scenario name (`__harness_selftest__`) to avoid
    /// colliding with real golden files.  The bless/compare cycle is driven by
    /// explicit `bless: bool` parameters so that no process-global env var is
    /// touched and the test is safe to run in parallel with other tests.
    ///
    /// Steps:
    ///   1. Remove any leftover temp file from a previous run.
    ///   2. Bless (write) a small synthetic trace.
    ///   3. Confirm read-back matches exactly → `Ok(())`.
    ///   4. Confirm a modified trace is detected as a mismatch → `Err(...)`.
    ///   5. Clean up the temp file.
    #[test]
    fn harness_selftest() {
        let name = "__harness_selftest__";
        let path = golden_path(name);

        // Step 1: Remove any leftover from a prior run so bless always starts fresh.
        let _ = std::fs::remove_file(&path);

        // Step 2: Bless a small synthetic trace.
        let synthetic: Vec<String> = vec![
            "period=FirstHalf     | clock= 40s | timeout=none         | conf_pause=none   | pens=[]".to_string(),
            "period=FirstHalf     | clock= 30s | timeout=none         | conf_pause=none   | pens=[]".to_string(),
            "period=HalfTime      | clock= 10s | timeout=none         | conf_pause=none   | pens=[]".to_string(),
        ];
        check_or_bless(name, &synthetic, true).expect("bless should succeed");

        // The file must now exist.
        assert!(path.exists(), "golden file should exist after bless");

        // Step 3: Read-back must match exactly.
        check_or_bless(name, &synthetic, false).expect("compare after bless should return Ok(())");

        // Step 4: A modified trace must produce a meaningful Err.
        let mut modified = synthetic.clone();
        modified[1] =
            "period=FirstHalf     | clock= 25s | timeout=none         | pens=[]".to_string();
        let err = check_or_bless(name, &modified, false)
            .expect_err("compare of modified trace should return Err");
        assert!(
            err.contains("__harness_selftest__"),
            "error message should name the scenario; got: {err}"
        );
        assert!(
            err.contains("line 2"),
            "error message should identify the first differing line; got: {err}"
        );

        // Step 5: Clean up.
        let _ = std::fs::remove_file(&path);
    }
}
