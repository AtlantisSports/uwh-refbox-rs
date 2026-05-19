use log::*;
use thiserror::Error;
use tokio::{
    sync::watch,
    time::{Duration, Instant},
};

use super::snapshot::{BeepTestPeriod, BeepTestSnapshot, TimeSnapshot};
use crate::config::BeepTest as BeepTestConfig;

pub type Result<T> = std::result::Result<T, TournamentManagerError>;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum TournamentManagerError {
    #[error("The `now` value passed is not valid")]
    InvalidNowValue,
    #[error("Already in a {0}")]
    AlreadyStopped(TimeSnapshot),
    #[error("Can't 'start now' when in a play period")]
    AlreadyInPlayPeriod,
}

#[derive(Debug, Clone, PartialEq)]
enum TimeState {
    None,
}

impl TimeState {
    fn as_snapshot(&self) -> TimeSnapshot {
        match self {
            TimeState::None => TimeSnapshot::None,
        }
    }
}

#[derive(Debug)]
pub struct TournamentManager {
    clock_state: ClockState,
    config: BeepTestConfig,
    current_period: BeepTestPeriod,
    time_state: TimeState,
    start_stop_tx: watch::Sender<bool>,
    start_stop_rx: watch::Receiver<bool>,
    count: u8,
    lap_count: u8,
    time_in_next_lap: Duration,
}

impl TournamentManager {
    pub fn new(config: BeepTestConfig) -> Self {
        let (start_stop_tx, start_stop_rx) = watch::channel(false);
        let initial_clock = config
            .levels
            .first()
            .map(|l| l.duration)
            .unwrap_or_default();
        Self {
            time_in_next_lap: config.levels.get(1).map(|l| l.duration).unwrap_or_default(),
            current_period: BeepTestPeriod::Level(0),
            clock_state: ClockState::Stopped {
                clock_time: initial_clock,
            },
            config,
            time_state: TimeState::None,
            start_stop_tx,
            start_stop_rx,
            count: 1,
            lap_count: 0,
        }
    }

    pub fn current_period(&self) -> BeepTestPeriod {
        self.current_period
    }

    pub fn send_clock_running(&self, running: bool) {
        self.start_stop_tx.send(running).unwrap();
    }

    pub fn start_clock(&mut self, now: Instant) {
        let need_to_send = self.start_game_clock(now);

        if need_to_send {
            self.send_clock_running(true);
        }
    }

    pub fn stop_clock(&mut self, now: Instant) -> Result<()> {
        if let ClockState::CountingDown {
            start_time: _,
            time_remaining_at_start: _,
        } = self.clock_state
        {
            let clock_time = if let Some(time) = self.clock_state.clock_time(now) {
                time
            } else {
                Duration::from_nanos(1)
            };

            self.clock_state = ClockState::Stopped { clock_time };
            self.send_clock_running(false);

            Ok(())
        } else {
            self.send_clock_running(true);

            Ok(())
        }
    }

    pub fn get_start_stop_rx(&self) -> watch::Receiver<bool> {
        self.start_stop_rx.clone()
    }

    pub fn game_clock_time(&self, now: Instant) -> Option<Duration> {
        trace!(
            "Getting game clock time with clock state {:?} and now time {now:?}",
            self.clock_state
        );
        self.clock_state.clock_time(now)
    }

    // Returns true if the clock was started, false if it was already running
    fn start_game_clock(&mut self, now: Instant) -> bool {
        if let ClockState::Stopped { clock_time } = self.clock_state {
            info!("{} Starting the game clock", self.status_string(now));
            self.clock_state = ClockState::CountingDown {
                start_time: now,
                time_remaining_at_start: clock_time,
            };

            true
        } else {
            false
        }
    }

    pub fn clock_is_running(&self) -> bool {
        match &self.time_state {
            TimeState::None => self.clock_state.is_running(),
        }
    }

    pub fn start_beep_test_now(&mut self, now: Instant) -> Result<()> {
        self.count = 1;
        if self.time_state != TimeState::None {
            return Err(TournamentManagerError::AlreadyStopped(
                self.time_state.as_snapshot(),
            ));
        }

        // Only allowed to start from Level(0) (the stopped/reset state). Any
        // other Level means the test is already running.
        match self.current_period {
            BeepTestPeriod::Level(0) => {
                self.start_game(now);
            }
            BeepTestPeriod::Level(_) => return Err(TournamentManagerError::AlreadyInPlayPeriod),
        }

        // After start_game, current_period is Level(0) — start the countdown.
        // SAFETY: start_game just set current_period to Level(0), which always
        // has a valid duration when config.levels is non-empty. The cadence engine
        // is only constructed with non-empty configs.
        self.clock_state = ClockState::CountingDown {
            start_time: now,
            time_remaining_at_start: self
                .current_period
                .duration(&self.config)
                .expect("Level(0) must have a duration — config.levels is non-empty"),
        };

        self.send_clock_running(true);

        Ok(())
    }

    pub fn reset_beep_test_now(&mut self, now: Instant) {
        info!("{} Resetting Beep Test", self.status_string(now));

        self.current_period = BeepTestPeriod::Level(0);
        // Level(0) is the warm-up; its duration comes from config.pre.
        let initial_clock = self
            .current_period
            .duration(&self.config)
            .unwrap_or_default();
        self.clock_state = ClockState::Stopped {
            clock_time: initial_clock,
        };
        self.count = 1;
        self.lap_count = 0;
        self.time_in_next_lap = self
            .config
            .levels
            .get(1)
            .map(|l| l.duration)
            .unwrap_or_default();

        self.send_clock_running(false);
    }

    fn status_string(&self, now: Instant) -> String {
        use std::fmt::Write;

        let mut string = String::new();

        if let Some(time) = self.game_clock_time(now).map(|dur| dur.as_secs_f64()) {
            if let Err(e) = write!(
                &mut string,
                "[{:02.0}:{:06.3} ",
                (time / 60.0).floor(),
                time % 60.0
            ) {
                error!("Error with time string: {}", e);
            }
        } else {
            string.push_str("[XX:XX.XXX ");
        }

        string.push_str("BEEP TEST");

        string
    }

    pub fn generate_snapshot(&mut self, now: Instant) -> Option<BeepTestSnapshot> {
        trace!("Generating snapshot");
        let cur_time = self.game_clock_time(now)?;
        trace!("Got current time: {cur_time:?}");
        let secs_in_period = cur_time.as_secs_f32().round() as u32;
        trace!("Got seconds remaining: {secs_in_period}");

        let next_period_len_secs = self
            .current_period
            .next_test_period_dur(&self.config)
            .map_or(0, |dur| dur.as_secs_f32().round() as u32);

        let lap_count = self.lap_count;
        let total_time_in_period = self
            .current_period
            .duration(&self.config)
            .unwrap()
            .as_secs() as u32;
        let time_in_next_period = self.time_in_next_lap.as_secs() as u32;

        Some(BeepTestSnapshot {
            current_period: self.current_period,
            secs_in_period,
            next_period_len_secs,
            lap_count,
            total_time_in_period,
            time_in_next_period,
        })
    }

    fn start_game(&mut self, start_time: Instant) {
        info!("{} Entering beep test", self.status_string(start_time),);
        self.current_period = BeepTestPeriod::Level(0);
        self.lap_count = 0;
    }

    pub fn update(&mut self, now: Instant) -> Result<()> {
        // Case of clock running, with no timeout and not SD
        if let ClockState::CountingDown {
            start_time,
            time_remaining_at_start,
        } = self.clock_state
        {
            let time = now
                .checked_duration_since(start_time)
                .ok_or(TournamentManagerError::InvalidNowValue)?;

            if time >= time_remaining_at_start {
                match self.current_period {
                    BeepTestPeriod::Level(_) => {
                        self.start_next_lap(now);
                    }
                }
            }
        }
        Ok(())
    }

    fn start_next_lap(&mut self, now: Instant) {
        // Was the last level completed (i.e., will next_period wrap to Level(0))?
        // We detect the wrap by comparing next_period to Level(0) after the
        // final lap of the final level. If it wraps, the test is complete:
        // stop the clock and reset to Level(0) ready for a new run.
        let p @ BeepTestPeriod::Level(_) = self.current_period;

        self.lap_count += 1;

        if self.count >= p.count(&self.config).unwrap() {
            self.count = 1;
            let next = self.current_period.next_period(&self.config);
            self.current_period = next;
            info!(
                "{} Entering next period: {}",
                self.status_string(now),
                self.current_period
            );

            // Detect wrap: next_period wraps to Level(0) when all levels are done.
            // In that case, stop the clock — the test is complete.
            if next == BeepTestPeriod::Level(0) {
                // Test complete — reset to the initial stopped state at Level(0).
                self.lap_count = 0;
                self.clock_state = ClockState::Stopped {
                    clock_time: self
                        .current_period
                        .duration(&self.config)
                        .unwrap_or_default(),
                };
                self.send_clock_running(false);
                return;
            }

            if self.current_period.count(&self.config) == Some(1) {
                self.time_in_next_lap = self
                    .current_period
                    .next_test_period_dur(&self.config)
                    .unwrap_or_default();
            }
        } else {
            if self.count == p.count(&self.config).unwrap() - 1 {
                self.time_in_next_lap = self
                    .current_period
                    .next_test_period_dur(&self.config)
                    .unwrap_or_default();
            }
            self.count += 1;
            info!(
                "{} Repeating period: {}",
                self.status_string(now),
                self.current_period
            );
        }

        // Continue counting down for the next lap/period.
        match self.clock_state {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => {
                self.clock_state = ClockState::CountingDown {
                    start_time: start_time + time_remaining_at_start,
                    time_remaining_at_start: self
                        .current_period
                        .duration(&self.config)
                        .unwrap_or_default(),
                };
            }
            ClockState::Stopped { .. } => {
                self.clock_state = ClockState::CountingDown {
                    start_time: now,
                    time_remaining_at_start: self
                        .current_period
                        .duration(&self.config)
                        .unwrap_or_default(),
                };
            }
        }
    }

    pub fn next_update_time(&self, now: Instant) -> Option<Instant> {
        let time = self.clock_state.clock_time(now)?;
        let sub_secs = time.subsec_nanos();
        let secs = time.as_secs();
        let next = if sub_secs <= 499_999_999 {
            if secs < 1 {
                sub_secs
            } else {
                sub_secs + 500_000_001
            }
        } else {
            sub_secs - 499_999_999
        };
        Some(now + Duration::from_nanos(next as u64))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ClockState {
    Stopped {
        clock_time: Duration,
    },
    CountingDown {
        start_time: Instant,
        time_remaining_at_start: Duration,
    },
}

impl std::default::Default for ClockState {
    fn default() -> Self {
        ClockState::Stopped {
            clock_time: Duration::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BeepTest as BeepTestConfig, Level};

    /// A minimal two-level config used by most tests.
    /// Level(0) is the 1-second warm-up (`pre`). Level(1) count=2 duration=10 s.
    /// Level(2) count=2 duration=8 s.
    fn test_config() -> BeepTestConfig {
        BeepTestConfig {
            pre: Duration::from_secs(1),
            levels: vec![
                Level {
                    count: 2,
                    duration: Duration::from_secs(10),
                },
                Level {
                    count: 2,
                    duration: Duration::from_secs(8),
                },
            ],
        }
    }

    /// A tiny single-level config used for end-of-test traversal.
    /// Level(0) is the 1-second warm-up; Level(1) count=1 duration=1 s.
    /// After Level(1) the engine wraps back to Level(0) and stops.
    fn tiny_config() -> BeepTestConfig {
        BeepTestConfig {
            pre: Duration::from_secs(1),
            levels: vec![Level {
                count: 1,
                duration: Duration::from_secs(1),
            }],
        }
    }

    // Test 1 — a freshly constructed engine is not running.
    #[test]
    fn starts_stopped() {
        let tm = TournamentManager::new(test_config());
        assert!(!tm.clock_is_running());
        // New engine starts at Level(0), which is the first real level.
        assert_eq!(tm.current_period(), BeepTestPeriod::Level(0));
    }

    // Test 2 — calling start_clock marks the engine as running.
    #[test]
    fn start_clock_marks_running() {
        let mut tm = TournamentManager::new(test_config());
        let now = Instant::now();
        tm.start_clock(now);
        assert!(tm.clock_is_running());
    }

    // Test 3 — calling stop_clock after start_clock marks the engine as stopped.
    #[test]
    fn stop_clock_marks_stopped() {
        let mut tm = TournamentManager::new(test_config());
        let now = Instant::now();
        tm.start_clock(now);
        tm.stop_clock(now + Duration::from_secs(1)).unwrap();
        assert!(!tm.clock_is_running());
    }

    // Test 4 — start_beep_test_now starts at Level(0) (the warm-up) with the clock running.
    //
    // `start_beep_test_now` is the intended entry point for beginning the beep test.
    // It sets current_period to Level(0) (the warm-up shown as "Level 0" on the display)
    // and starts the countdown using `config.pre` as the warm-up duration.
    #[test]
    fn start_beep_test_starts_at_level_0() {
        let mut tm = TournamentManager::new(test_config());
        let now = Instant::now();
        tm.start_beep_test_now(now).unwrap();
        assert_eq!(tm.current_period(), BeepTestPeriod::Level(0));
        assert!(tm.clock_is_running());
    }

    // Test 5 — driving the engine through all levels via update() eventually ends the test.
    //
    // With tiny_config (pre=1 s, one level count=1 duration=1 s): start_beep_test_now
    // begins Level(0) (the warm-up). After 1 s the warm-up completes and the engine
    // advances to Level(1). After another 1 s the single Level(1) lap completes;
    // next_period wraps to Level(0), the engine detects the wrap, stops the clock, and
    // resets lap_count to 0. The engine does NOT stay in a "Finished" state — it
    // resets to Level(0) stopped, ready for the next start_beep_test_now call.
    #[test]
    fn full_run_ends_stopped() {
        let mut tm = TournamentManager::new(tiny_config());
        let t0 = Instant::now();

        // Enter the test: stopped → Level(0) (warm-up), clock counting down for 1 s.
        tm.start_beep_test_now(t0).unwrap();
        assert_eq!(tm.current_period(), BeepTestPeriod::Level(0));
        assert!(tm.clock_is_running());

        // Advance past Level(0) (warm-up). Engine advances to Level(1).
        tm.update(t0 + Duration::from_secs(2)).unwrap();
        assert_eq!(tm.current_period(), BeepTestPeriod::Level(1));
        assert!(tm.clock_is_running());

        // Advance past Level(1)'s single lap. next_period wraps to Level(0),
        // the engine detects the wrap, stops the clock, and resets to idle.
        tm.update(t0 + Duration::from_secs(4)).unwrap();
        assert_eq!(tm.current_period(), BeepTestPeriod::Level(0));
        assert!(!tm.clock_is_running());
    }
}

impl ClockState {
    fn is_running(&self) -> bool {
        match self {
            ClockState::CountingDown { .. } => true,
            ClockState::Stopped { .. } => false,
        }
    }

    /// Returns `None` if the clock time would be negative, or if `now` is before the start
    /// of the clock
    fn clock_time(&self, now: Instant) -> Option<Duration> {
        match self {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => now
                .checked_duration_since(*start_time)
                .and_then(|s| time_remaining_at_start.checked_sub(s)),

            ClockState::Stopped { clock_time } => Some(*clock_time),
        }
    }
}
