//! Holds the bridge's live picture of the game: the last real snapshot the refbox sent, exactly
//! as it sent it, plus when it arrived.
//!
//! **This module used to do much more.** Earlier drafts kept the game clock counting locally
//! through a dropout -- comparing how closely two real snapshots' arrival times tracked each
//! other to infer whether the clock was running, then projecting `secs_in_period` forward (or,
//! for `GamePeriod::SuddenDeath`, upward) from the last real value. That is gone (spec §4.6,
//! reversed 2026-08-26, by Eric): the bridge now shows only what the refbox actually sent, or
//! nothing at all -- never a guess, however plausible.
//!
//! **Whether the connection itself is alive -- the only question that decides whether a served
//! table shows real values or blanks them -- is answered entirely by [`crate::feed::Connection`],
//! not by anything here.** The refbox goes completely silent whenever the clock is stopped (25
//! seconds observed, see `feed`'s module doc), so silence from this module's point of view is
//! never evidence of anything, and this module makes no attempt to interpret it.

use std::time::Instant;

use uwh_common::game_snapshot::GameSnapshot;

/// The bridge's live picture of the game: the last real snapshot the refbox sent, and when it
/// arrived. Nothing here is inferred, projected, or held-with-a-caveat -- [`LiveState::current`]
/// always returns exactly the last real snapshot, verbatim.
#[derive(Debug, Clone)]
pub struct LiveState {
    /// The most recent real snapshot received from the refbox.
    last: GameSnapshot,
    /// When `last` arrived, on the caller's clock. Nothing in this module reads this -- it is
    /// kept so a caller (the operator status page, Task 7's job) can report how long ago the
    /// refbox last actually spoke, which is a fact worth showing on its own but is never used to
    /// decide whether the connection is alive. That question belongs entirely to
    /// [`crate::feed::Connection`]; see the module doc.
    last_arrived_at: Instant,
}

impl LiveState {
    /// Starts a live picture from the first real snapshot seen -- or, at startup before any real
    /// snapshot has arrived, from a synthetic default (see `server::AppState::new`, which seeds
    /// this with `GameSnapshot::default()` so every route is servable immediately).
    pub fn new(snapshot: GameSnapshot, at: Instant) -> Self {
        Self {
            last: snapshot,
            last_arrived_at: at,
        }
    }

    /// Records a real snapshot from the refbox, arriving at `at`. Always overwrites whatever was
    /// held before outright -- there is nothing to blend, smooth, or compare it against, because
    /// this struct never holds anything but the refbox's own last word.
    pub fn apply(&mut self, snapshot: GameSnapshot, at: Instant) {
        self.last = snapshot;
        self.last_arrived_at = at;
    }

    /// The last real snapshot the refbox sent, exactly as it sent it. No projection, no
    /// inference: if the refbox has said nothing since this arrived, this is still exactly
    /// right, because the game it describes is stopped (see the module doc) -- not stale.
    pub fn current(&self) -> Display {
        Display {
            snapshot: self.last.clone(),
        }
    }

    /// When the last real snapshot arrived, on the caller's clock. See the `last_arrived_at`
    /// field doc -- purely informational, never used by this module (or by anything decided
    /// here) to judge whether the refbox is still reachable.
    pub fn last_arrived_at(&self) -> Instant {
        self.last_arrived_at
    }
}

/// The live picture of the game the bridge currently believes is correct: the last real snapshot
/// from the refbox, completely unmodified. Every field is exactly as the refbox last reported
/// it -- including `secs_in_period`, which used to be corrected for elapsed time here and no
/// longer is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub snapshot: GameSnapshot,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uwh_common::game_snapshot::GamePeriod;

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

    #[test]
    fn current_returns_the_last_real_snapshot_verbatim_no_matter_how_long_since_it_arrived() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let state = LiveState::new(base.clone(), t0);

        // No further real snapshot ever arrives here. `current` takes no notion of "now" at all
        // any more -- there is nothing it could use one for -- so there is no elapsed time to
        // vary in this test; the point is that the *only* thing `current` can possibly report is
        // exactly `base`, unconditionally.
        assert_eq!(state.current().snapshot, base);
    }

    #[test]
    fn apply_overwrites_the_previous_snapshot_and_arrival_time_outright() {
        let t0 = Instant::now();
        let base = first_half_snapshot();
        let mut state = LiveState::new(base, t0);

        let t1 = t0 + Duration::from_secs(5);
        let next = GameSnapshot {
            secs_in_period: 42,
            ..first_half_snapshot()
        };
        state.apply(next.clone(), t1);

        assert_eq!(
            state.current().snapshot,
            next,
            "apply must replace the held snapshot outright, not blend with what came before"
        );
        assert_eq!(state.last_arrived_at(), t1);
    }

    #[test]
    fn sudden_death_relays_verbatim_like_any_other_period() {
        // The refbox counts sudden death up rather than down (see `GamePeriod::time_between`),
        // which used to matter here because the old projection logic had to flip direction for
        // it. It no longer matters at all: this module now relays whatever `secs_in_period` the
        // refbox sent, for every period alike, without ever computing with it. A build that
        // still special-cased sudden death (or any other period) would only be able to disagree
        // with this by touching the value in the first place -- which relaying verbatim never
        // does.
        let sudden_death = GameSnapshot {
            current_period: GamePeriod::SuddenDeath,
            secs_in_period: 187,
            ..first_half_snapshot()
        };
        let state = LiveState::new(sudden_death.clone(), Instant::now());

        assert_eq!(state.current().snapshot, sudden_death);
    }
}
