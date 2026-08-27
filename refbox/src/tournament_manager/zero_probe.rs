//! Regression sweep for degenerate period lengths.
//!
//! Drives the SHIPPING tick (`TournamentManager::updater_tick`) — not a copy of it — so
//! this test cannot drift away from what the app actually runs.
//!
//! COVERAGE, stated precisely: every combination of the **seven period-length**
//! parameters in [`FIELDS`]. It does NOT vary `team_timeout_duration`,
//! `penalty_shot_duration`, `post_game_duration`, `nominal_break` or `game_block`, and it
//! never enters a timeout state — so the timeout arms of `next_update_time` and the
//! rugby-penalty-shot arm of `could_end_game` are not exercised here. A green run means
//! "no zero-length *period* crashes the tick", not "nothing crashes the tick".
//!
//! It also advances simulated time by a fixed floor each iteration, which the real
//! updater does not do, so it cannot reproduce a busy-spin on a next-update instant that
//! is persistently in the past.
//!
//! History: a zero-length period crashed the refbox in the field three times
//! (2026-06-25, 2026-06-27, 2026-07-13). The crash was a force-unwrap in the clock
//! updater that poisoned the shared game lock and killed the application. The tick may
//! still legitimately FAIL on a degenerate config; what it must never do is crash.

use super::*;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};

const TIMER_FLOOR: Duration = Duration::from_micros(250);
const ITERATION_CAP: u32 = 40_000;

use crate::panic_text::panic_reason;

/// This driver's own stepping cadence when the engine offers no next-update instant.
/// It mirrors the app's fallback in spirit but is deliberately the harness's constant,
/// not a copy of the shipping one — this is a simulation clock, not the real cadence.
const DRIVER_FALLBACK_STEP: Duration = Duration::from_millis(100);

/// What driving one config produced.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Every tick succeeded. The run either used the whole window or stopped early
    /// because the clock latch went off, which is a legitimate resting state.
    Clean,
    /// One or more ticks reported a failure. Records the first, with the state it left,
    /// plus how many of the run's ticks failed — the difference between "stumbled once
    /// and carried on" and "never recovered", which is the whole point of this branch.
    Reported {
        reason: String,
        period: GamePeriod,
        failed_ticks: u32,
        total_ticks: u32,
    },
    /// The tick crashed. This is the bug; it must never happen.
    Crashed(String),
    /// The driver never settled. Also a failure of this test.
    NoProgress,
}

fn base_config() -> GameConfig {
    GameConfig {
        half_play_duration: Duration::from_secs(6),
        half_time_duration: Duration::from_secs(4),
        pre_overtime_break: Duration::from_secs(3),
        ot_half_play_duration: Duration::from_secs(4),
        ot_half_time_duration: Duration::from_secs(2),
        pre_sudden_death_duration: Duration::from_secs(2),
        minimum_break: Duration::from_secs(5),
        nominal_break: Duration::from_secs(6),
        post_game_duration: Duration::from_secs(3),
        game_block: Duration::from_secs(40),
        overtime_allowed: true,
        sudden_death_allowed: true,
        ..Default::default()
    }
}

/// Each period-length parameter, paired with the setter that zeroes it. Name and setter
/// live in ONE table on purpose: they used to be separate — a name list and a hand-written
/// chain of bit tests — so adding a parameter would have labelled it in the report while
/// never actually applying it, and the extra configs would have run as silent duplicates.
#[allow(clippy::type_complexity)]
const FIELDS: [(&str, fn(&mut GameConfig, Duration)); 7] = [
    ("half_play", |c, d| c.half_play_duration = d),
    ("half_time", |c, d| c.half_time_duration = d),
    ("pre_ot_break", |c, d| c.pre_overtime_break = d),
    ("ot_half_play", |c, d| c.ot_half_play_duration = d),
    ("ot_half_time", |c, d| c.ot_half_time_duration = d),
    ("pre_sd_break", |c, d| c.pre_sudden_death_duration = d),
    ("min_break", |c, d| c.minimum_break = d),
];

fn apply_zeroes(cfg: &mut GameConfig, mask: usize, zero: Duration) {
    for (i, (_, set)) in FIELDS.iter().enumerate() {
        if mask & (1 << i) != 0 {
            set(cfg, zero);
        }
    }
}

fn mask_name(mask: usize) -> String {
    let names: Vec<&str> = FIELDS
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, (name, _))| *name)
        .collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join("+")
    }
}

/// Drive one config through `secs` of simulated time using the shipping tick.
fn drive(config: GameConfig, decided: bool, secs: u64) -> Outcome {
    let caught = panic::catch_unwind(AssertUnwindSafe(move || {
        let mut tm = TournamentManager::new(config);
        let base = Instant::now();
        tm.start_play_now(base).unwrap();
        if decided {
            tm.add_score(Color::Black, 3, base);
        }
        // Hoisted: `get_start_stop_rx` clones the receiver, and this is the largest test
        // in the crate by iteration count.
        let running = tm.get_start_stop_rx();
        let deadline = base + Duration::from_secs(secs);
        let mut now = base;
        let mut iters = 0u32;
        let mut failed_ticks = 0u32;
        let mut first_failure: Option<(String, GamePeriod)> = None;

        while now < deadline {
            iters += 1;
            if iters > ITERATION_CAP {
                return Outcome::NoProgress;
            }
            if let Err(e) = tm.updater_tick(now) {
                failed_ticks += 1;
                if first_failure.is_none() {
                    first_failure = Some((e.to_string(), tm.current_period));
                }
            }
            if !*tm.get_start_stop_rx().borrow() {
                break;
            }
            let next = tm
                .next_update_time(now)
                .unwrap_or(now + DRIVER_FALLBACK_STEP);
            now = max(next, now) + TIMER_FLOOR;
        }

        match first_failure {
            Some((reason, period)) => Outcome::Reported {
                reason,
                period,
                failed_ticks,
                total_ticks: iters,
            },
            None => Outcome::Clean,
        }
    }));

    match caught {
        Ok(outcome) => outcome,
        Err(payload) => Outcome::Crashed(panic_reason(&*payload)),
    }
}

/// THE regression test: no degenerate config may crash the tick, and none may hang.
///
/// Run with `-- --nocapture` to see the report of which configs report a failure and
/// what state each is left in — that is the evidence promised in the design spec.
#[test]
fn no_degenerate_config_crashes_the_tick() {
    /// Which FIELDS entries may legitimately make a tick fail. Checked against the mask
    /// itself, not against the rendered label — a substring test on the label happened to
    /// work only because "ot_half_play" contains "half_play", exempted any mask that also
    /// zeroed half_play whatever actually caused its failure, and would have silently
    /// become a no-op if either field were renamed.
    const MAY_FAIL: [usize; 2] = [0 /* half_play */, 3 /* ot_half_play */];

    let mut crashed: Vec<String> = Vec::new();
    let mut stalled: Vec<String> = Vec::new();
    let mut unexpected: Vec<String> = Vec::new();
    let mut never_recovered: Vec<String> = Vec::new();
    let mut reported: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (zlabel, zero) in [
        ("EXACT-0", Duration::ZERO),
        ("1us", Duration::from_micros(1)),
    ] {
        for decided in [false, true] {
            for mask in 0..(1usize << FIELDS.len()) {
                let mut cfg = base_config();
                apply_zeroes(&mut cfg, mask, zero);
                let label = format!(
                    "{zlabel} {} [{}]",
                    if decided { "decided" } else { "tied" },
                    mask_name(mask)
                );
                let outcome = drive(cfg, decided, 90);

                // mask 0 is the untouched base_config: a healthy game. Without this the
                // sweep passes on "nothing crashed", so a regression that made EVERY
                // config fail every tick would still be reported green.
                if mask == 0 {
                    assert_eq!(
                        outcome,
                        Outcome::Clean,
                        "the healthy baseline config must tick cleanly ({label})"
                    );
                }

                match outcome {
                    Outcome::Clean => {}
                    Outcome::Reported {
                        reason,
                        period,
                        failed_ticks,
                        total_ticks,
                    } => {
                        // Pin WHICH parameters may make a tick fail, by mask.
                        if !MAY_FAIL.iter().any(|i| mask & (1 << i) != 0) {
                            unexpected.push(label.clone());
                        }
                        // And pin that a failure is a stumble, not a permanent wedge.
                        // Surviving a fault is only worth anything if the game then
                        // carries on; a config that fails every tick is an app frozen
                        // for the whole run, which is what this branch exists to avoid.
                        if failed_ticks == total_ticks {
                            never_recovered
                                .push(format!("{label}: all {total_ticks} ticks failed"));
                        }
                        reported
                            .entry(format!("{reason} (left in {period:?})"))
                            .or_default()
                            .push(format!("{label} — {failed_ticks}/{total_ticks} ticks"));
                    }
                    Outcome::Crashed(message) => crashed.push(format!("{label}: {message}")),
                    Outcome::NoProgress => stalled.push(label),
                }
            }
        }
    }

    println!("\n===== configs whose tick reported a failure =====");
    if reported.is_empty() {
        println!("  (none)");
    }
    for (reason, cases) in &reported {
        println!("  {} — {} configs", reason, cases.len());
        for case in cases.iter().take(8) {
            println!("      {case}");
        }
        if cases.len() > 8 {
            println!("      ... and {} more", cases.len() - 8);
        }
    }

    assert!(
        crashed.is_empty(),
        "the tick crashed on {} config(s):\n{}",
        crashed.len(),
        crashed.join("\n")
    );
    assert!(
        stalled.is_empty(),
        "the tick never settled on {} config(s):\n{}",
        stalled.len(),
        stalled.join("\n")
    );
    assert!(
        never_recovered.is_empty(),
        "{} config(s) failed EVERY tick — surviving a fault is only worth something if \
         the game then carries on:\n{}",
        never_recovered.len(),
        never_recovered.join("\n")
    );
    assert!(
        unexpected.is_empty(),
        "only zeroing half_play or ot_half_play may make a tick fail, but {} other \
         config(s) did:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
}

/// The two configurations memory records as the ones that actually crashed. Named
/// separately so a regression in either is obvious from the test name alone.
#[test]
fn zero_half_length_survives() {
    let mut cfg = base_config();
    cfg.half_play_duration = Duration::ZERO;
    // Reported, not Crashed and not NoProgress. `!matches!(.., Crashed(_))` alone would
    // also pass on a driver that never settled, which is not survival.
    assert!(matches!(
        drive(cfg, false, 90),
        Outcome::Clean | Outcome::Reported { .. }
    ));
}

#[test]
fn zero_overtime_half_length_survives() {
    let mut cfg = base_config();
    cfg.ot_half_play_duration = Duration::ZERO;
    assert!(matches!(
        drive(cfg, false, 90),
        Outcome::Clean | Outcome::Reported { .. }
    ));
}

/// Kept for future investigation: prints the period-by-period path a single
/// degenerate config takes. Not part of the regression gate.
#[test]
#[ignore = "investigation aid; run explicitly with --ignored"]
fn trace_zero_half_length() {
    let mut cfg = base_config();
    cfg.half_play_duration = Duration::ZERO;
    let mut tm = TournamentManager::new(cfg);
    let base = Instant::now();
    tm.start_play_now(base).unwrap();
    let mut now = base;
    for step in 0..200 {
        println!(
            "step {step} t={:?} period={:?} clock={:?}",
            now.duration_since(base),
            tm.current_period,
            tm.clock_state
        );
        match tm.updater_tick(now) {
            Ok((kind, _)) => println!("  -> {kind:?}"),
            Err(e) => println!("  -> reported: {e}"),
        }
        if !*tm.get_start_stop_rx().borrow() {
            println!("clock latch off");
            break;
        }
        let next = tm.next_update_time(now).unwrap_or(now + TIMER_FLOOR);
        now = max(next, now) + TIMER_FLOOR;
    }
}

/// Measures the one torn-state effect the design identified: when the tick fails at a
/// period change, the period has already advanced and the clock already restarted, but
/// penalties that finished during the period just ended have NOT been cleared.
///
/// This reports what actually happens rather than asserting a particular clean-up
/// outcome — it is the evidence the design spec promised before deciding whether an
/// all-or-nothing tick is needed. Run with `-- --nocapture` to read it.
///
/// Both runs stop the instant the game enters overtime — the culling transition under
/// test. Running any longer is useless: the game ends, the next one starts, and the
/// reset wipes the very evidence being measured.
///
/// The comparison is against an identical game whose overtime half has a real length,
/// so the only difference between the two runs is whether the cull succeeded.
#[test]
fn measure_what_a_failed_cull_leaves_behind() {
    fn run(ot_half: Duration) -> (usize, GamePeriod, bool) {
        let mut cfg = base_config();
        // Long enough halves that a 30-second penalty finishes before overtime, so the
        // cull entering overtime has something real to remove.
        cfg.half_play_duration = Duration::from_secs(20);
        cfg.ot_half_play_duration = ot_half;
        cfg.sudden_death_allowed = false;

        let mut tm = TournamentManager::new(cfg);
        let base = Instant::now();
        tm.start_play_now(base).unwrap();
        tm.start_penalty(
            Color::Black,
            7,
            PenaltyKind::ThirtySecond,
            base,
            Infraction::Unknown,
        )
        .unwrap();

        let deadline = base + Duration::from_secs(120);
        let mut now = base;
        let mut failed = false;
        while now < deadline {
            if tm.updater_tick(now).is_err() {
                failed = true;
            }
            // Stop at the culling transition under test, before the game can end and
            // reset away what we are trying to observe.
            if tm.current_period == GamePeriod::OvertimeFirstHalf {
                break;
            }
            if !*tm.get_start_stop_rx().borrow() {
                break;
            }
            let next = tm
                .next_update_time(now)
                .unwrap_or(now + DRIVER_FALLBACK_STEP);
            now = max(next, now) + TIMER_FLOOR;
        }
        (
            tm.get_penalties()[Color::Black].len(),
            tm.current_period,
            failed,
        )
    }

    let (healthy_left, healthy_period, healthy_failed) = run(Duration::from_secs(4));
    let (degenerate_left, degenerate_period, degenerate_failed) = run(Duration::ZERO);

    println!("\n===== what a failed cull leaves behind, at the moment of failure =====");
    println!(
        "  healthy overtime half:     {healthy_left} penalt(y/ies) left, period {healthy_period:?}, tick failed: {healthy_failed}"
    );
    println!(
        "  zero-length overtime half: {degenerate_left} penalt(y/ies) left, period {degenerate_period:?}, tick failed: {degenerate_failed}"
    );

    // The only things asserted are what matter: neither run may crash, and the
    // degenerate one must be the one that reports a failure.
    // Without this the loop's other two exits — the deadline, or the clock latch going
    // off — let the measurement pass while describing a state nobody meant to measure.
    assert_eq!(
        healthy_period,
        GamePeriod::OvertimeFirstHalf,
        "the healthy run must actually reach the culling transition being measured"
    );
    assert_eq!(
        degenerate_period,
        GamePeriod::OvertimeFirstHalf,
        "the degenerate run must actually reach the culling transition being measured"
    );
    assert!(
        !healthy_failed,
        "a healthy config must not report a tick failure"
    );
    // If the underlying zero-length-period defect is ever fixed in the engine, this
    // assertion fires even though behaviour improved. That is intended: it forces
    // whoever fixes it to revisit this measurement rather than leave it silently
    // asserting a world that no longer exists. Update the expectation; do not delete it.
    assert!(
        degenerate_failed,
        "the zero-length overtime half no longer reports a tick failure — if the engine \
         defect was fixed, update this measurement instead of removing it"
    );
}
