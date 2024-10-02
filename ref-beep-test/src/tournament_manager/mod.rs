use log::*;
use thiserror::Error;
use time::UtcOffset;
use tokio::{
    sync::watch,
    time::{Duration, Instant},
};

use super::config::BeepTest as BeepTestConfig;
use super::snapshot::{BeepTestPeriod, BeepTestSnapshot, TimeSnapshot};

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
    timezone: UtcOffset,
    time_state: TimeState,
    start_stop_tx: watch::Sender<bool>,
    start_stop_rx: watch::Receiver<bool>,
    count: u8,
}

impl TournamentManager {
    pub fn new(config: BeepTestConfig) -> Self {
        let (start_stop_tx, start_stop_rx) = watch::channel(false);
        Self {
            current_period: BeepTestPeriod::Pre,
            clock_state: ClockState::Stopped {
                clock_time: Duration::from_nanos(1),
            },
            config,
            timezone: UtcOffset::UTC,
            time_state: TimeState::None,
            start_stop_tx,
            start_stop_rx,
            count: 1,
        }
    }

    pub fn current_period(&self) -> BeepTestPeriod {
        self.current_period
    }

    pub fn set_timezone(&mut self, timezone: UtcOffset) {
        self.timezone = timezone;
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

        match self.current_period {
            BeepTestPeriod::Level1
            | BeepTestPeriod::Level2
            | BeepTestPeriod::Level3
            | BeepTestPeriod::Level4
            | BeepTestPeriod::Level5
            | BeepTestPeriod::Level6
            | BeepTestPeriod::Level7
            | BeepTestPeriod::Level8
            | BeepTestPeriod::Level9
            | BeepTestPeriod::Level10 => return Err(TournamentManagerError::AlreadyInPlayPeriod),
            BeepTestPeriod::Pre | BeepTestPeriod::Level0 => {
                self.start_game(now);
            }
        }

        self.clock_state = match self.current_period {
            p @ BeepTestPeriod::Level0
            | p @ BeepTestPeriod::Level1
            | p @ BeepTestPeriod::Level2
            | p @ BeepTestPeriod::Level3
            | p @ BeepTestPeriod::Level4
            | p @ BeepTestPeriod::Level5
            | p @ BeepTestPeriod::Level6
            | p @ BeepTestPeriod::Level7
            | p @ BeepTestPeriod::Level8
            | p @ BeepTestPeriod::Level9
            | p @ BeepTestPeriod::Level10 => ClockState::CountingDown {
                start_time: now,
                time_remaining_at_start: p.duration(&self.config).unwrap(),
            },
            p @ BeepTestPeriod::Pre => ClockState::Stopped {
                clock_time: p.duration(&self.config).unwrap(),
            },
        };

        println!("Setting clock state to: {:?}", self.clock_state);
        println!("Sending clock running status: true");

        self.send_clock_running(true);

        println!("Period: {:?}", self.current_period);

        Ok(())
    }

    pub fn reset_beep_test_now(&mut self, now: Instant) {
        info!("{} Resetting Beep Test", self.status_string(now));

        self.current_period = BeepTestPeriod::Pre;
        self.clock_state = ClockState::Stopped {
            clock_time: Duration::from_nanos(1),
        };
        self.count = 1;

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

        Some(BeepTestSnapshot {
            current_period: self.current_period,
            secs_in_period,
            next_period_len_secs,
        })
    }

    fn start_game(&mut self, start_time: Instant) {
        info!("{} Entering beep test", self.status_string(start_time),);
        self.current_period = BeepTestPeriod::Level0;
    }

    pub(super) fn update(&mut self, now: Instant) -> Result<()> {
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
                    BeepTestPeriod::Pre => {
                        self.start_game(start_time + time_remaining_at_start);
                    }
                    BeepTestPeriod::Level0
                    | BeepTestPeriod::Level1
                    | BeepTestPeriod::Level2
                    | BeepTestPeriod::Level3
                    | BeepTestPeriod::Level4
                    | BeepTestPeriod::Level5
                    | BeepTestPeriod::Level6
                    | BeepTestPeriod::Level7
                    | BeepTestPeriod::Level8
                    | BeepTestPeriod::Level9
                    | BeepTestPeriod::Level10 => {
                        self.start_next_period(now);
                    }
                }
            }
        }
        Ok(())
    }

    fn start_next_period(&mut self, now: Instant) {
        match self.current_period {
            BeepTestPeriod::Pre => {
                info!("{} Entering next period", self.status_string(now));
                self.current_period = BeepTestPeriod::Level0;
            }
            BeepTestPeriod::Level0 => {
                info!("{} Entering next period", self.status_string(now));
                self.current_period = BeepTestPeriod::Level1;
            }
            BeepTestPeriod::Level1 => {
                if self.count == self.config.count_3 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level2;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level1;
                }
            }
            BeepTestPeriod::Level2 => {
                if self.count == self.config.count_3 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level3;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level2;
                }
            }
            BeepTestPeriod::Level3 => {
                if self.count == self.config.count_3 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level4;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level3;
                }
            }
            BeepTestPeriod::Level4 => {
                if self.count == self.config.count_4 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level5;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level4;
                }
            }
            BeepTestPeriod::Level5 => {
                if self.count == self.config.count_4 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level6;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level5;
                }
            }
            BeepTestPeriod::Level6 => {
                if self.count == self.config.count_4 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level7;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level6;
                }
            }
            BeepTestPeriod::Level7 => {
                if self.count == self.config.count_4 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level8;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level7;
                }
            }
            BeepTestPeriod::Level8 => {
                if self.count == self.config.count_4 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level9;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level8;
                }
            }
            BeepTestPeriod::Level9 => {
                if self.count == self.config.count_5 {
                    self.count = 1;
                    info!("{} Entering next period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level10;
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level9;
                }
            }
            BeepTestPeriod::Level10 => {
                if self.count == self.config.count_3 {
                    self.count = 1;
                    info!("{} Ended and resetting", self.status_string(now));
                    self.current_period = BeepTestPeriod::Pre;
                    info!("{:?} Period", self.current_period);
                } else {
                    self.count += 1;
                    info!("{} Repeating period", self.status_string(now));
                    self.current_period = BeepTestPeriod::Level10;
                    info!("{:?} Period", self.current_period);
                }
            }
        }
        if self.current_period != BeepTestPeriod::Pre {
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
                            .unwrap(),
                    };
                }
                ClockState::Stopped { .. } => {
                    self.clock_state = ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: self
                            .current_period
                            .duration(&self.config)
                            .unwrap(),
                    }
                }
            }
        } else {
            self.clock_state = ClockState::Stopped {
                clock_time: self.current_period.duration(&self.config).unwrap(),
            };
            self.send_clock_running(false);
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
