//! Who may use the control page: the devices allowed to reach it, the signed-in sessions, the
//! Stream Deck button key, and the one-at-a-time line for PIN sign-ins.

use crate::config::Config;
use std::{
    collections::VecDeque,
    net::IpAddr,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Mutex as AsyncMutex;

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

/// At most this many sign-in attempts wait in line at once; any more are turned away.
pub const MAX_WAITING_SIGN_INS: usize = 20;

/// Random bytes in the Stream Deck button key (64 hex characters).
const BUTTON_KEY_BYTES: usize = 32;
/// Random bytes in a sign-in (session) token.
const SESSION_TOKEN_BYTES: usize = 32;

/// `bytes` bytes from the operating system's random source, as lowercase hex.
pub fn random_hex(bytes: usize) -> Result<String, getrandom::Error> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer)?;
    Ok(buffer.iter().map(|b| format!("{b:02x}")).collect())
}

/// A new Stream Deck button key.
pub fn new_button_key() -> Result<String, getrandom::Error> {
    random_hex(BUTTON_KEY_BYTES)
}

/// A new sign-in token for the session cookie.
pub fn new_session_token() -> Result<String, getrandom::Error> {
    random_hex(SESSION_TOKEN_BYTES)
}

/// Whether `given` is `secret`, taking the same time wherever they differ. An empty secret
/// (not created yet) never matches.
pub fn secret_matches(given: &str, secret: &str) -> bool {
    let (given, secret) = (given.as_bytes(), secret.as_bytes());
    if secret.is_empty() || given.len() != secret.len() {
        return false;
    }
    given
        .iter()
        .zip(secret)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

/// Which devices may reach the control page, as read when Stream Manager started: changes to
/// these settings apply after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Devices {
    /// Off: the page listens on this mini PC only. On: on the network, for the listed devices.
    pub allow_others: bool,
    pub allowed: Vec<IpAddr>,
}

impl Devices {
    pub fn from_config(config: &Config) -> Self {
        Self {
            allow_others: config.allow_other_devices,
            allowed: config.allowed_devices.clone(),
        }
    }

    /// Whether a request from `ip` is answered: always from this mini PC itself, and from a
    /// listed device while other devices are allowed.
    pub fn allows(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        ip.is_loopback()
            || (self.allow_others && self.allowed.iter().any(|a| a.to_canonical() == ip))
    }
}

/// The sign-in attempts waiting, so a flood of them can't pile up without limit.
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
    /// Takes a place in the line, unless [`MAX_WAITING_SIGN_INS`] are already waiting.
    pub fn join(&self) -> Option<Place<'_>> {
        self.waiting
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |waiting| {
                (waiting < MAX_WAITING_SIGN_INS).then_some(waiting + 1)
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

/// How a sign-in attempt was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignIn {
    Right,
    Wrong,
    /// [`MAX_WAITING_SIGN_INS`] were already waiting: turned away without looking at the PIN.
    TooMany,
}

/// Every PIN sign-in, right or wrong, from any device, goes through this one line, one at a
/// time. The PIN is looked at only when an attempt reaches the front, and after a wrong PIN the
/// next attempt waits (see [`PinFailures`]), so sending many guesses at once is no faster than
/// sending them one by one.
#[derive(Debug, Default)]
pub struct SignInLine {
    line: WaitingLine,
    /// Held by the attempt at the front of the line. Tokio's mutex is fair, so attempts reach
    /// the front in the order they arrived.
    front: AsyncMutex<()>,
    failures: Mutex<PinFailures>,
}

impl SignInLine {
    /// Waits for this attempt's turn, then asks `is_right` whether its PIN is right.
    pub async fn attempt(&self, is_right: impl FnOnce() -> bool) -> SignIn {
        let Some(_place) = self.line.join() else {
            return SignIn::TooMany;
        };
        let _front = self.front.lock().await;
        // The lock is let go before waiting.
        let ready = self.failures().ready_at();
        if let Some(ready) = ready {
            tokio::time::sleep_until(ready.into()).await;
        }
        if is_right() {
            SignIn::Right
        } else {
            self.failures().record(Instant::now());
            SignIn::Wrong
        }
    }

    fn failures(&self) -> std::sync::MutexGuard<'_, PinFailures> {
        self.failures.lock().unwrap_or_else(|e| e.into_inner())
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

/// Wrong PINs in a row at the sign-in, from any device.
#[derive(Debug, Default)]
pub struct PinFailures {
    count: u32,
    last: Option<Instant>,
}

impl PinFailures {
    /// Counts a wrong PIN at `now` and returns how long the next attempt waits: 1 s, then
    /// doubling, up to 30 s. A run of wrong PINs is forgotten after 15 minutes without one.
    /// A right PIN doesn't reset the count.
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

    /// When the next attempt may be looked at, after the last wrong PIN (if any).
    pub fn ready_at(&self) -> Option<Instant> {
        self.last.map(|last| last + failure_wait(self.count))
    }
}

/// The wait after the `count`th wrong PIN in a row.
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
    fn at_most_twenty_sign_ins_wait_at_once() {
        let line = WaitingLine::default();
        let mut places: Vec<Place<'_>> = (0..MAX_WAITING_SIGN_INS)
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

    #[test]
    fn the_next_attempt_waits_after_a_wrong_pin() {
        let start = Instant::now();
        let mut failures = PinFailures::default();
        assert_eq!(failures.ready_at(), None);
        failures.record(start);
        assert_eq!(failures.ready_at(), Some(start + SECOND));
        failures.record(start + SECOND * 5);
        assert_eq!(failures.ready_at(), Some(start + SECOND * 7));
    }

    #[test]
    fn device_check_allows_this_pc_and_listed_devices_only() {
        let listed: IpAddr = "192.168.1.50".parse().unwrap();
        let other: IpAddr = "192.168.1.51".parse().unwrap();
        let mut devices = Devices {
            allow_others: true,
            allowed: vec![listed],
        };
        for local in ["127.0.0.1", "::1", "::ffff:127.0.0.1"] {
            assert!(devices.allows(local.parse().unwrap()), "{local}");
        }
        assert!(devices.allows(listed));
        assert!(devices.allows("::ffff:192.168.1.50".parse().unwrap()));
        assert!(!devices.allows(other));
        // With other devices off, even a listed one is refused.
        devices.allow_others = false;
        assert!(!devices.allows(listed));
        assert!(devices.allows("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn secrets_match_only_exactly_and_never_when_empty() {
        let key = new_button_key().unwrap();
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(key, new_button_key().unwrap());
        assert!(secret_matches(&key, &key));
        let mut wrong = key.clone();
        wrong.replace_range(63.., if key.ends_with('0') { "1" } else { "0" });
        assert!(!secret_matches(&wrong, &key));
        assert!(!secret_matches(&key[..63], &key));
        assert!(!secret_matches("", &key));
        assert!(!secret_matches("", ""));
    }

    #[tokio::test]
    async fn a_pin_is_looked_at_only_at_the_front_of_the_line() {
        let line = SignInLine::default();
        // Someone else is at the front.
        let front = line.front.lock().await;
        let looked = std::sync::atomic::AtomicBool::new(false);
        let attempt = line.attempt(|| {
            looked.store(true, Ordering::SeqCst);
            true
        });
        tokio::pin!(attempt);
        // The attempt waits; its PIN isn't looked at yet.
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut attempt)
                .await
                .is_err()
        );
        assert!(!looked.load(Ordering::SeqCst));
        drop(front);
        assert_eq!(attempt.await, SignIn::Right);
        assert!(looked.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn a_full_line_turns_attempts_away_without_looking_or_counting() {
        let line = SignInLine::default();
        let places: Vec<Place<'_>> = (0..MAX_WAITING_SIGN_INS)
            .map(|_| line.line.join().expect("room in the line"))
            .collect();
        let outcome = line
            .attempt(|| panic!("the PIN must not be looked at"))
            .await;
        assert_eq!(outcome, SignIn::TooMany);
        assert_eq!(line.failures().ready_at(), None, "not counted");
        drop(places);
    }

    #[tokio::test]
    async fn after_a_wrong_pin_the_next_attempt_waits_and_a_right_one_keeps_the_count() {
        let line = SignInLine::default();
        let started = Instant::now();
        assert_eq!(line.attempt(|| false).await, SignIn::Wrong);
        // The wrong PIN itself is answered at once.
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(line.attempt(|| true).await, SignIn::Right);
        // The next attempt waited about 1 s.
        assert!(started.elapsed() >= SECOND);
        // The right PIN didn't reset the count: the next wrong PIN is the second in a row.
        let mut failures = line.failures();
        let last = failures.last.unwrap();
        assert_eq!(failures.record(last), SECOND * 2);
    }
}
