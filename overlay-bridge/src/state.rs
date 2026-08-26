//! Holds the bridge's best current picture of the game, and decides what to show while the
//! refbox is saying nothing at all.
//!
//! The refbox only sends a message when something changes: once a second while its clock is
//! running, and never while the clock is stopped (see spec §5.4, and the phase-0b measurements
//! it cites). So silence from the refbox is ambiguous on its own -- it means either "still
//! running, just between ticks" or "genuinely paused" -- and this module is what tells those two
//! apart, using only the pattern of arrivals it has already seen:
//!
//! - Updates were arriving close together and changing -> the clock was running -> keep counting
//!   locally from the last real value.
//! - Updates were not arriving that way -> the clock was stopped -> hold the last real value
//!   exactly, forever, until a real update says otherwise.
//!
//! No I/O of any kind happens here, including reading the system clock: every function that
//! needs "now" takes it as a parameter, so the logic can be driven and tested by hand.

use std::time::{Duration, Instant};

use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};

/// How close together two real updates must have arrived for the second to count as evidence the
/// clock was running. Chosen in the design spec (§5.4): measured operator interaction (scoring, a
/// penalty entry) stretches the normal one-second tick to 1.5-2.0 seconds, so the threshold sits
/// safely above that -- a shorter one would flag a lost connection every time the referee touches
/// the keypad.
const CONTACT_THRESHOLD: Duration = Duration::from_secs(3);

/// The bridge's live picture of the game: the last real snapshot the refbox sent, plus enough
/// about how it arrived to say what the clock is doing right now, even if the refbox has gone
/// quiet.
#[derive(Debug, Clone)]
pub struct LiveState {
    /// The most recent real snapshot received from the refbox.
    last: GameSnapshot,
    /// When `last` arrived, on the caller's clock.
    last_arrived_at: Instant,
    /// Whether the arrival of `last` -- compared with the real snapshot before it -- is evidence
    /// the clock was running: they arrived under `CONTACT_THRESHOLD` apart and reported different
    /// `secs_in_period` values. `false` until a second real snapshot has been seen, since one
    /// snapshot alone gives no evidence of movement either way, and holding is the conservative
    /// default.
    was_running: bool,
}

impl LiveState {
    /// Starts a live picture from the first real snapshot seen. There is no prior snapshot to
    /// compare arrival timing against, so the clock is presumed stopped until a second snapshot
    /// proves otherwise.
    pub fn new(snapshot: GameSnapshot, at: Instant) -> Self {
        Self {
            last: snapshot,
            last_arrived_at: at,
            was_running: false,
        }
    }

    /// Records a real snapshot from the refbox, arriving at `at`. This always overwrites whatever
    /// the bridge had been showing, including a locally continued estimate that had drifted --
    /// the real value is never blended or smoothed with it.
    pub fn apply(&mut self, snapshot: GameSnapshot, at: Instant) {
        let arrived_within_threshold = at
            .checked_duration_since(self.last_arrived_at)
            .is_some_and(|gap| gap < CONTACT_THRESHOLD);
        let clock_changed = snapshot.secs_in_period != self.last.secs_in_period;

        self.was_running = arrived_within_threshold && clock_changed;
        self.last = snapshot;
        self.last_arrived_at = at;
    }

    /// What the bridge believes the game looks like right now. While the last real snapshot gave
    /// evidence the clock was running, its `secs_in_period` is projected forward (or, for
    /// `GamePeriod::SuddenDeath`, upward -- see that variant's note) by however much time has
    /// passed since it arrived; otherwise the snapshot is returned exactly as received, held at
    /// its last real value.
    pub fn current(&self, now: Instant) -> Display {
        let mut snapshot = self.last.clone();

        if self.was_running {
            let elapsed = now
                .checked_duration_since(self.last_arrived_at)
                .unwrap_or(Duration::ZERO);
            // A whole game lasts nowhere near long enough for this many elapsed seconds to
            // overflow a u32, so the fallback is unreachable in practice; it exists only so a
            // pathological `now` cannot panic.
            let elapsed_secs = u32::try_from(elapsed.as_secs()).unwrap_or(u32::MAX);

            snapshot.secs_in_period = match self.last.current_period {
                // Sudden death has no fixed length, so the refbox counts it up from zero instead
                // of down to zero (see `GamePeriod::time_between` in
                // `uwh-common/src/game_snapshot.rs`, which flips the same way). Continuing it
                // locally has to flip the same way, or the projected clock runs backwards.
                GamePeriod::SuddenDeath => self.last.secs_in_period.saturating_add(elapsed_secs),
                _ => self.last.secs_in_period.saturating_sub(elapsed_secs),
            };
        }

        Display { snapshot }
    }

    /// Whether the refbox is still in contact, judged purely by how long it has been since its
    /// last real snapshot arrived. This is a separate question from whether the clock was
    /// running: a long silence during a genuine stoppage (half time, an injury) is expected and
    /// still reports `Stale` here -- the design accepts that as a known limitation (spec §5.4,
    /// §9.4), closed instead by the connection-level keepalive elsewhere in the bridge, not by
    /// this heuristic.
    pub fn contact(&self, now: Instant) -> Contact {
        let elapsed = now
            .checked_duration_since(self.last_arrived_at)
            .unwrap_or(Duration::ZERO);

        if elapsed < CONTACT_THRESHOLD {
            Contact::Live
        } else {
            Contact::Stale { since: elapsed }
        }
    }
}

/// The live picture of the game the bridge currently believes is correct: the last real snapshot
/// from the refbox, with `secs_in_period` corrected for any time that has passed since then while
/// the clock was known to be running. Every other field is exactly as the refbox last reported
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub snapshot: GameSnapshot,
}

/// Whether the refbox is currently reachable, judged by how long it has been silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contact {
    /// A real snapshot arrived within `CONTACT_THRESHOLD`.
    Live,
    /// No real snapshot has arrived for `since`, at least `CONTACT_THRESHOLD`.
    Stale { since: Duration },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real snapshots captured from a live refbox on 2026-08-26 (see feed.rs's tests for the
    /// full description of this fixture). Used here so every test builds on a genuinely
    /// game-shaped snapshot rather than an invented one.
    const FIXTURE: &str = include_str!("../tests/fixtures/feed-capture.jsonl");

    /// Fetch one line of the fixture by its 0-based position, already parsed.
    fn fixture_snapshot(n: usize) -> GameSnapshot {
        let line = FIXTURE
            .lines()
            .nth(n)
            .unwrap_or_else(|| panic!("fixture is missing line {n}"));
        serde_json::from_str(line).unwrap_or_else(|e| panic!("fixture line {n} should parse: {e}"))
    }

    /// `fixture_snapshot(1)`: `FirstHalf`, `secs_in_period: 90`.
    fn first_half_snapshot() -> GameSnapshot {
        fixture_snapshot(1)
    }

    /// A `SuddenDeath` snapshot, adapted from a real captured one so every field besides the
    /// period and clock stays realistic. No sudden-death state was captured in the fixture (the
    /// game it came from never reached one), so this is the one snapshot in this file that isn't
    /// pulled straight from a fixture line.
    fn sudden_death_snapshot(secs_in_period: u32) -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::SuddenDeath,
            secs_in_period,
            ..first_half_snapshot()
        }
    }

    #[test]
    fn ticks_arriving_each_second_then_silence_keep_counting_down_in_real_time() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        assert_eq!(base.secs_in_period, 90);

        let mut state = LiveState::new(base.clone(), t0);
        // A second real tick, one second later, with the clock down by one -- exactly the
        // steady-state pattern the refbox produces while running.
        let second_tick = GameSnapshot {
            secs_in_period: 89,
            ..base
        };
        let t1 = t0 + Duration::from_secs(1);
        state.apply(second_tick, t1);

        // Now silence: no further real snapshots. Five seconds after the last real tick, the
        // locally continued clock should have kept counting down in step with real time.
        let display = state.current(t1 + Duration::from_secs(5));
        assert_eq!(display.snapshot.current_period, GamePeriod::FirstHalf);
        assert_eq!(display.snapshot.secs_in_period, 84);
    }

    #[test]
    fn no_ticks_at_all_then_silence_holds_the_last_value_indefinitely() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let state = LiveState::new(base.clone(), t0);

        // Only one real snapshot has ever been seen, so there's no evidence the clock was
        // running -- it must hold, no matter how much later `current` is asked.
        for later in [
            Duration::from_secs(1),
            Duration::from_secs(30),
            Duration::from_secs(3600),
            Duration::from_secs(1_000_000),
        ] {
            let display = state.current(t0 + later);
            assert_eq!(
                display.snapshot.secs_in_period, base.secs_in_period,
                "held clock should not move after {later:?} of silence"
            );
        }
    }

    #[test]
    fn sudden_death_counts_up_not_down() {
        let t0 = Instant::now();
        let first = sudden_death_snapshot(0);
        let mut state = LiveState::new(first, t0);

        let t1 = t0 + Duration::from_secs(1);
        state.apply(sudden_death_snapshot(1), t1);

        // Six seconds of silence after a real value of 1. A countdown-always implementation
        // would saturate this at 0 (1 - 6, clamped); the correct, counts-up behaviour is 7.
        let display = state.current(t1 + Duration::from_secs(6));
        assert_eq!(display.snapshot.current_period, GamePeriod::SuddenDeath);
        assert_eq!(
            display.snapshot.secs_in_period, 7,
            "sudden death must count UP through a dropout, not down"
        );
    }

    #[test]
    fn a_two_second_gap_during_normal_running_is_not_stale() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let mut state = LiveState::new(base.clone(), t0);
        let t1 = t0 + Duration::from_secs(1);
        state.apply(
            GameSnapshot {
                secs_in_period: base.secs_in_period - 1,
                ..base
            },
            t1,
        );

        // A gap the size real operator interaction produces (spec §5.4: 1.5-2.0s), measured from
        // the last real arrival.
        let contact = state.contact(t1 + Duration::from_millis(2000));
        assert_eq!(contact, Contact::Live);
    }

    #[test]
    fn a_four_second_gap_while_running_is_reported_stale() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let mut state = LiveState::new(base.clone(), t0);
        let t1 = t0 + Duration::from_secs(1);
        state.apply(
            GameSnapshot {
                secs_in_period: base.secs_in_period - 1,
                ..base
            },
            t1,
        );

        let gap = Duration::from_secs(4);
        let contact = state.contact(t1 + gap);
        assert_eq!(contact, Contact::Stale { since: gap });
    }

    #[test]
    fn a_real_snapshot_overwrites_a_drifted_local_estimate() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let mut state = LiveState::new(base.clone(), t0);
        let t1 = t0 + Duration::from_secs(1);
        state.apply(
            GameSnapshot {
                secs_in_period: base.secs_in_period - 1, // 89
                ..base.clone()
            },
            t1,
        );

        // Let a long silence drift the local estimate well away from anything the refbox could
        // plausibly have sent next -- just documents that drift genuinely happened before the
        // real snapshot below overwrites it. `current` is a pure query, so asking it here has no
        // effect on the state that the rest of the test exercises.
        let drifted = state.current(t1 + Duration::from_secs(500));
        assert_eq!(
            drifted.snapshot.secs_in_period, 0,
            "drift should have bottomed out at 0"
        );

        // A real snapshot arrives, close enough behind the last one (2.5s, under the 3s
        // threshold) that the clock is still known to be running -- but reporting a value (65)
        // that has nothing to do with either the 89 it continues from or the drifted 0 above, as
        // if an operator had edited the clock. It must win outright, not blend with either.
        let t2 = t1 + Duration::from_millis(2500);
        state.apply(
            GameSnapshot {
                secs_in_period: 65,
                ..base
            },
            t2,
        );
        assert_eq!(state.current(t2).snapshot.secs_in_period, 65);

        // And it must be the new anchor for further local continuation, not just a one-off value
        // that gets read once and forgotten. Five more silent seconds should count down from 65,
        // not from a stale trajectory still rooted at 89 (which would read 82 here, not 60).
        let display = state.current(t2 + Duration::from_secs(5));
        assert_eq!(display.snapshot.secs_in_period, 60);
    }

    #[test]
    fn the_clock_never_counts_below_zero() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let almost_over = GameSnapshot {
            secs_in_period: 4,
            ..base.clone()
        };
        let mut state = LiveState::new(almost_over, t0);
        let t1 = t0 + Duration::from_secs(1);
        state.apply(
            GameSnapshot {
                secs_in_period: 3,
                ..base
            },
            t1,
        );

        // Ten seconds of silence, far more than the 3 seconds remaining when the last real
        // snapshot arrived.
        let display = state.current(t1 + Duration::from_secs(10));
        assert_eq!(display.snapshot.secs_in_period, 0);
    }
}
