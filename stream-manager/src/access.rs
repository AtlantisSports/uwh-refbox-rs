//! Who may use the control page: the signed-in sessions, and the slowdown after wrong PINs.

use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

/// A sign-in lasts this long; then the PIN is asked for again.
pub const SESSION_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
/// At most this many sign-ins are kept; a new one pushes out the oldest.
pub const MAX_SESSIONS: usize = 50;

/// The wait after the first wrong PIN. It doubles with every wrong PIN after that.
const FIRST_FAILURE_WAIT: Duration = Duration::from_secs(1);
/// The longest wait after a wrong PIN.
const LONGEST_FAILURE_WAIT: Duration = Duration::from_secs(30);
/// Wrong PINs are forgotten once there has been none for this long.
const FAILURES_FORGOTTEN_AFTER: Duration = Duration::from_secs(15 * 60);

/// At most this many wrong PINs wait for their answer at once; any more are answered at once.
pub const MAX_WAITING_WRONG_PINS: usize = 20;

/// The wrong PINs waiting for their answer, so a flood of them can't pile up without limit.
#[derive(Debug, Default)]
pub struct WaitingLine {
    waiting: AtomicUsize,
}

/// A place in the [`WaitingLine`], given up when dropped (answered, or the request went away).
#[derive(Debug)]
pub struct Place<'a> {
    line: &'a WaitingLine,
}

impl WaitingLine {
    /// Takes a place in the line, unless [`MAX_WAITING_WRONG_PINS`] are already waiting.
    pub fn join(&self) -> Option<Place<'_>> {
        self.waiting
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |waiting| {
                (waiting < MAX_WAITING_WRONG_PINS).then_some(waiting + 1)
            })
            .ok()
            .map(|_| Place { line: self })
    }
}

impl Drop for Place<'_> {
    fn drop(&mut self) {
        self.line.waiting.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The signed-in sessions (cookie tokens), oldest first.
#[derive(Debug, Default)]
pub struct Sessions {
    sessions: VecDeque<(String, Instant)>,
}

impl Sessions {
    /// Adds a sign-in made at `now`, dropping expired ones and, past [`MAX_SESSIONS`], the oldest.
    pub fn add(&mut self, token: String, now: Instant) {
        self.drop_expired(now);
        while self.sessions.len() >= MAX_SESSIONS {
            self.sessions.pop_front();
        }
        self.sessions.push_back((token, now));
    }

    /// Whether `token` is a sign-in that hasn't expired at `now`.
    pub fn has(&self, token: &str, now: Instant) -> bool {
        self.sessions
            .iter()
            .any(|(t, signed_in)| t == token && !expired(*signed_in, now))
    }

    pub fn remove(&mut self, token: &str) {
        self.sessions.retain(|(t, _)| t != token);
    }

    fn drop_expired(&mut self, now: Instant) {
        self.sessions
            .retain(|(_, signed_in)| !expired(*signed_in, now));
    }
}

fn expired(signed_in: Instant, now: Instant) -> bool {
    now.saturating_duration_since(signed_in) >= SESSION_LIFETIME
}

/// Wrong PINs in a row, from any device and by any route (sign-in, `?pin=`, `x-pin`).
#[derive(Debug, Default)]
pub struct PinFailures {
    count: u32,
    last: Option<Instant>,
}

impl PinFailures {
    /// Counts a wrong PIN at `now` and returns how long to wait before answering it: 1 s, then
    /// doubling, up to 30 s. A run of wrong PINs is forgotten after 15 minutes without one.
    /// A correct PIN never comes here, so it is never slowed down, and it doesn't reset the
    /// count either.
    pub fn record(&mut self, now: Instant) -> Duration {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) >= FAILURES_FORGOTTEN_AFTER)
        {
            self.count = 0;
        }
        self.count = self.count.saturating_add(1);
        self.last = Some(now);
        failure_wait(self.count)
    }
}

/// The wait for the `count`th wrong PIN in a row.
fn failure_wait(count: u32) -> Duration {
    let doublings = count.saturating_sub(1).min(16);
    FIRST_FAILURE_WAIT
        .saturating_mul(1 << doublings)
        .min(LONGEST_FAILURE_WAIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn wrong_pins_wait_longer_each_time_up_to_thirty_seconds() {
        let start = Instant::now();
        let mut failures = PinFailures::default();
        let waits: Vec<u64> = (0..8)
            .map(|i| failures.record(start + SECOND * i).as_secs())
            .collect();
        assert_eq!(waits, [1, 2, 4, 8, 16, 30, 30, 30]);
        // Many more stay at 30 s.
        for i in 8..200 {
            assert_eq!(failures.record(start + SECOND * i), LONGEST_FAILURE_WAIT);
        }
    }

    #[test]
    fn wrong_pins_are_forgotten_after_fifteen_quiet_minutes() {
        let start = Instant::now();
        let mut failures = PinFailures::default();
        failures.record(start);
        failures.record(start + SECOND);
        assert_eq!(failures.record(start + SECOND * 2).as_secs(), 4);
        // Just under 15 minutes after the last one: still counted.
        let later = start + SECOND * 2 + FAILURES_FORGOTTEN_AFTER - SECOND;
        assert_eq!(failures.record(later).as_secs(), 8);
        // 15 minutes after the last one: back to the first wait.
        assert_eq!(
            failures.record(later + FAILURES_FORGOTTEN_AFTER).as_secs(),
            1
        );
    }

    #[test]
    fn at_most_twenty_wrong_pins_wait_at_once() {
        let line = WaitingLine::default();
        let mut places: Vec<Place<'_>> = (0..MAX_WAITING_WRONG_PINS)
            .map(|_| line.join().expect("room in the line"))
            .collect();
        assert!(line.join().is_none());
        // One answered (or gone): one more may wait.
        drop(places.pop());
        let again = line.join();
        assert!(again.is_some());
        assert!(line.join().is_none());
        drop(places);
    }

    #[test]
    fn a_sign_in_expires_after_24_hours() {
        let start = Instant::now();
        let mut sessions = Sessions::default();
        sessions.add("a".into(), start);
        assert!(sessions.has("a", start + SESSION_LIFETIME - SECOND));
        assert!(!sessions.has("a", start + SESSION_LIFETIME));
        assert!(!sessions.has("b", start));
        sessions.remove("a");
        assert!(!sessions.has("a", start));
    }

    #[test]
    fn at_most_fifty_sign_ins_are_kept_dropping_the_oldest() {
        let start = Instant::now();
        let mut sessions = Sessions::default();
        for i in 0..=MAX_SESSIONS {
            sessions.add(format!("t{i}"), start + SECOND * i as u32);
        }
        let now = start + SECOND * 100;
        assert!(!sessions.has("t0", now));
        assert!(sessions.has("t1", now));
        assert!(sessions.has(&format!("t{MAX_SESSIONS}"), now));
        assert_eq!(sessions.sessions.len(), MAX_SESSIONS);
    }

    #[test]
    fn expired_sign_ins_make_room_before_the_oldest_live_one_is_dropped() {
        let start = Instant::now();
        let mut sessions = Sessions::default();
        sessions.add("old".into(), start);
        for i in 1..MAX_SESSIONS {
            sessions.add(format!("t{i}"), start + SESSION_LIFETIME / 2);
        }
        // "old" has expired by now, so adding one more keeps every other sign-in.
        let now = start + SESSION_LIFETIME;
        sessions.add("new".into(), now);
        assert!(sessions.has("t1", now) && sessions.has("new", now));
        assert_eq!(sessions.sessions.len(), MAX_SESSIONS);
    }
}
