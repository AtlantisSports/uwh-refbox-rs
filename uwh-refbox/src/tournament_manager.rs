use crate::config::Game as GameConfig;
use crate::game_snapshot::{GamePeriod, GameSnapshot, TimeoutSnapshot};
use log::*;
use std::{
    cmp::max,
    convert::TryInto,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct TournamentManager {
    config: GameConfig,
    current_game: u16,
    game_start_time: Instant,
    current_period: GamePeriod,
    clock_state: ClockState,
    timeout_state: TimeoutState,
    w_timeouts_used: u16,
    b_timeouts_used: u16,
    b_score: u8,
    w_score: u8,
    start_stop_senders: Vec<Sender<bool>>,
}

impl TournamentManager {
    pub fn new(config: GameConfig) -> Self {
        Self {
            current_game: 0,
            game_start_time: Instant::now(),
            current_period: GamePeriod::BetweenGames,
            clock_state: ClockState::Stopped {
                clock_time: Duration::from_secs(config.nominal_break.into()),
            },
            timeout_state: TimeoutState::None,
            w_timeouts_used: 0,
            b_timeouts_used: 0,
            config,
            b_score: 0,
            w_score: 0,
            start_stop_senders: vec![],
        }
    }

    pub fn clock_is_running(&self) -> bool {
        match &self.timeout_state {
            TimeoutState::Black(cs)
            | TimeoutState::White(cs)
            | TimeoutState::Ref(cs)
            | TimeoutState::PenaltyShot(cs) => cs.is_running(),
            TimeoutState::None => self.clock_state.is_running(),
        }
    }

    pub fn current_period(&self) -> GamePeriod {
        self.current_period
    }

    #[cfg(test)]
    pub fn current_game(&self) -> u16 {
        self.current_game
    }

    #[cfg(test)]
    pub fn game_start_time(&self) -> Instant {
        self.game_start_time
    }

    pub fn add_b_score(&mut self, _player_num: u8, now: Instant) {
        self.set_scores(self.b_score + 1, self.w_score, now);
    }

    pub fn add_w_score(&mut self, _player_num: u8, now: Instant) {
        self.set_scores(self.b_score, self.w_score + 1, now);
    }

    pub fn get_b_score(&self) -> u8 {
        self.b_score
    }

    pub fn get_w_score(&self) -> u8 {
        self.w_score
    }

    pub fn set_scores(&mut self, b_score: u8, w_score: u8, now: Instant) {
        self.b_score = b_score;
        self.w_score = w_score;
        if self.current_period == GamePeriod::SuddenDeath && b_score != w_score {
            self.end_game(now);
        }
    }

    /// Returns `Ok` if timeout can be started, otherwise returns `Err` describing why not
    pub fn can_start_w_timeout(&self) -> Result<()> {
        if let ts @ TimeoutState::White(_) = &self.timeout_state {
            Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ))
        } else {
            match self.current_period {
                GamePeriod::FirstHalf | GamePeriod::SecondHalf => {
                    if self.w_timeouts_used < self.config.team_timeouts_per_half {
                        Ok(())
                    } else {
                        Err(TournamentManagerError::TooManyTeamTimeouts("white"))
                    }
                }
                _ => Err(TournamentManagerError::WrongGamePeriod(
                    TimeoutSnapshot::White(0),
                    self.current_period,
                )),
            }
        }
    }

    /// Returns `Ok` if timeout can be started, otherwise returns `Err` describing why not
    pub fn can_start_b_timeout(&self) -> Result<()> {
        if let ts @ TimeoutState::Black(_) = &self.timeout_state {
            Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ))
        } else {
            match self.current_period {
                GamePeriod::FirstHalf | GamePeriod::SecondHalf => {
                    if self.b_timeouts_used < self.config.team_timeouts_per_half {
                        Ok(())
                    } else {
                        Err(TournamentManagerError::TooManyTeamTimeouts("black"))
                    }
                }
                _ => Err(TournamentManagerError::WrongGamePeriod(
                    TimeoutSnapshot::Black(0),
                    self.current_period,
                )),
            }
        }
    }

    /// Returns `Ok` if timeout can be started, otherwise returns `Err` describing why not
    pub fn can_start_ref_timeout(&self) -> Result<()> {
        if let ts @ TimeoutState::Ref(_) = &self.timeout_state {
            Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ))
        } else {
            Ok(())
        }
    }

    /// Returns `Ok` if penalty shot can be started, otherwise returns `Err` describing why not
    pub fn can_start_penalty_shot(&self) -> Result<()> {
        if let ts @ TimeoutState::PenaltyShot(_) = &self.timeout_state {
            return Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ));
        };
        match self.current_period {
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath => Ok(()),
            gp @ _ => Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(0),
                gp,
            )),
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_w_timeout(&self) -> Result<()> {
        if let TimeoutState::Black(_) = &self.timeout_state {
            Err(TournamentManagerError::NotInBlackTimeout)
        } else {
            Ok(())
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_b_timeout(&self) -> Result<()> {
        if let TimeoutState::White(_) = &self.timeout_state {
            Err(TournamentManagerError::NotInWhiteTimeout)
        } else {
            Ok(())
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_ref_timeout(&self) -> Result<()> {
        if let TimeoutState::PenaltyShot(_) = &self.timeout_state {
            Err(TournamentManagerError::NotInPenaltyShot)
        } else {
            Ok(())
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_penalty_shot(&self) -> Result<()> {
        if let TimeoutState::Ref(_) = &self.timeout_state {
            Err(TournamentManagerError::NotInRefTimeout)
        } else {
            Ok(())
        }
    }

    pub fn start_w_timeout(&mut self, now: Instant) -> Result<()> {
        match self.can_start_w_timeout() {
            Ok(()) => {
                info!("Starting a white timeout");
                self.stop_game_clock(now)?;
                self.timeout_state = TimeoutState::White(ClockState::CountingDown {
                    start_time: now,
                    time_remaining_at_start: Duration::from_secs(
                        self.config.team_timeout_duration.into(),
                    ),
                });
                self.w_timeouts_used += 1;
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn start_b_timeout(&mut self, now: Instant) -> Result<()> {
        match self.can_start_b_timeout() {
            Ok(()) => {
                info!("Starting a black timeout");
                self.stop_game_clock(now)?;
                self.timeout_state = TimeoutState::Black(ClockState::CountingDown {
                    start_time: now,
                    time_remaining_at_start: Duration::from_secs(
                        self.config.team_timeout_duration.into(),
                    ),
                });
                self.b_timeouts_used += 1;
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn start_ref_timeout(&mut self, now: Instant) -> Result<()> {
        match self.can_start_ref_timeout() {
            Ok(()) => {
                info!("Starting a ref timeout");
                self.stop_game_clock(now)?;
                self.timeout_state = TimeoutState::Ref(ClockState::CountingUp {
                    start_time: now,
                    time_at_start: Duration::from_secs(0),
                });
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn start_penalty_shot(&mut self, now: Instant) -> Result<()> {
        match self.can_start_penalty_shot() {
            Ok(()) => {
                info!("Starting a penalty shot");
                self.stop_game_clock(now)?;
                self.timeout_state = TimeoutState::PenaltyShot(ClockState::CountingUp {
                    start_time: now,
                    time_at_start: Duration::from_secs(0),
                });
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn switch_to_w_timeout(&mut self) -> Result<()> {
        match self.can_switch_to_w_timeout() {
            Ok(()) => {
                info!("Switching to a white timeout");
                if let TimeoutState::Black(cs) = &self.timeout_state {
                    self.timeout_state = TimeoutState::White(cs.clone());
                }
                self.w_timeouts_used += 1;
                self.b_timeouts_used -= 1;
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn switch_to_b_timeout(&mut self) -> Result<()> {
        match self.can_switch_to_b_timeout() {
            Ok(()) => {
                info!("Switching to a black timeout");
                if let TimeoutState::White(cs) = &self.timeout_state {
                    self.timeout_state = TimeoutState::Black(cs.clone());
                }
                self.b_timeouts_used += 1;
                self.w_timeouts_used -= 1;
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn switch_to_ref_timeout(&mut self) -> Result<()> {
        match self.can_switch_to_ref_timeout() {
            Ok(()) => {
                info!("Switching to a ref timeout");
                if let TimeoutState::PenaltyShot(cs) = &self.timeout_state {
                    self.timeout_state = TimeoutState::Ref(cs.clone());
                }
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn switch_to_penalty_shot(&mut self) -> Result<()> {
        match self.can_switch_to_penalty_shot() {
            Ok(()) => {
                info!("Switching to a penalty shot");
                if let TimeoutState::Ref(cs) = &self.timeout_state {
                    self.timeout_state = TimeoutState::PenaltyShot(cs.clone());
                }
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    // TODO: Doesn't handle getting behind and catching up correctly
    fn end_game(&mut self, now: Instant) {
        self.current_period = GamePeriod::BetweenGames;

        info!(
            "Ending game {}. Score is B({}), W({})",
            self.current_game, self.b_score, self.w_score
        );

        let scheduled_start = self.game_start_time
            + 2 * Duration::from_secs(self.config.half_play_duration.into())
            + Duration::from_secs(self.config.half_time_duration.into())
            + Duration::from_secs(self.config.nominal_break.into());

        let game_end = match self.clock_state {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => start_time + time_remaining_at_start,
            ClockState::CountingUp { .. } | ClockState::Stopped { .. } => now,
        };

        let time_remaining_at_start =
            if let Some(time_until_start) = scheduled_start.checked_duration_since(game_end) {
                max(
                    time_until_start,
                    Duration::from_secs(self.config.minimum_break.into()),
                )
            } else {
                Duration::from_secs(self.config.minimum_break.into())
            };

        info!(
            "Entering between games, time to next game is {} seconds",
            time_remaining_at_start.as_secs()
        );

        self.clock_state = ClockState::CountingDown {
            start_time: game_end,
            time_remaining_at_start,
        }
    }

    pub(super) fn update(&mut self, now: Instant) {
        // Case of clock running, with no timeout and not SD
        if let ClockState::CountingDown {
            start_time,
            time_remaining_at_start,
        } = self.clock_state
        {
            if let Some(time_before_reset) = time_remaining_at_start
                .checked_sub(Duration::from_secs(self.config.pre_game_duration.into()))
            {
                if self.current_period == GamePeriod::BetweenGames
                    && now.duration_since(start_time) > time_before_reset
                {
                    self.set_scores(0, 0, now);
                }
            }

            if now.duration_since(start_time) >= time_remaining_at_start {
                match self.current_period {
                    GamePeriod::BetweenGames => {
                        self.current_game += 1;
                        info!("Entering first half of game {}", self.current_game);
                        self.current_period = GamePeriod::FirstHalf;
                        self.game_start_time = start_time + time_remaining_at_start;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.half_play_duration.into(),
                            ),
                        };
                        self.w_timeouts_used = 0;
                        self.b_timeouts_used = 0;
                    }
                    GamePeriod::FirstHalf => {
                        info!("Entering half time");
                        self.current_period = GamePeriod::HalfTime;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.half_time_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::HalfTime => {
                        info!("Entering half time");
                        self.current_period = GamePeriod::SecondHalf;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.half_play_duration.into(),
                            ),
                        };
                        self.w_timeouts_used = 0;
                        self.b_timeouts_used = 0;
                    }
                    GamePeriod::SecondHalf => {
                        if self.b_score != self.w_score
                            || (!self.config.has_overtime && !self.config.sudden_death_allowed)
                        {
                            self.end_game(now);
                        } else if self.config.has_overtime {
                            info!(
                                "Entering pre-overtime. Score is B({}), W({})",
                                self.b_score, self.w_score
                            );
                            self.current_period = GamePeriod::PreOvertime;
                            self.clock_state = ClockState::CountingDown {
                                start_time: start_time + time_remaining_at_start,
                                time_remaining_at_start: Duration::from_secs(
                                    self.config.pre_overtime_break.into(),
                                ),
                            }
                        } else {
                            info!(
                                "Entering pre-sudden death. Score is B({}), W({})",
                                self.b_score, self.w_score
                            );
                            self.current_period = GamePeriod::PreSuddenDeath;
                            self.clock_state = ClockState::CountingDown {
                                start_time: start_time + time_remaining_at_start,
                                time_remaining_at_start: Duration::from_secs(
                                    self.config.pre_sudden_death_duration.into(),
                                ),
                            }
                        }
                    }
                    GamePeriod::PreOvertime => {
                        info!("Entering overtime first half");
                        self.current_period = GamePeriod::OvertimeFirstHalf;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.ot_half_play_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::OvertimeFirstHalf => {
                        info!("Entering overtime half time");
                        self.current_period = GamePeriod::OvertimeHalfTime;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.ot_half_time_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::OvertimeHalfTime => {
                        info!("Entering ovetime second half");
                        self.current_period = GamePeriod::OvertimeSecondHalf;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.ot_half_play_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::OvertimeSecondHalf => {
                        if self.b_score != self.w_score || !self.config.sudden_death_allowed {
                            self.end_game(now);
                        } else {
                            info!(
                                "Entering pre-sudden death. Score is B({}), W({})",
                                self.b_score, self.w_score
                            );
                            self.current_period = GamePeriod::PreSuddenDeath;
                            self.clock_state = ClockState::CountingDown {
                                start_time: start_time + time_remaining_at_start,
                                time_remaining_at_start: Duration::from_secs(
                                    self.config.pre_sudden_death_duration.into(),
                                ),
                            }
                        }
                    }
                    GamePeriod::PreSuddenDeath => {
                        info!("Entering sudden death");
                        self.current_period = GamePeriod::SuddenDeath;
                        self.clock_state = ClockState::CountingUp {
                            start_time: start_time + time_remaining_at_start,
                            time_at_start: Duration::from_secs(0),
                        }
                    }
                    GamePeriod::SuddenDeath => {}
                }
            }
        } else {
            // We are either in a timeout, sudden death, or stopped clock. Sudden death and
            // stopped clock don't need anything done
            match &self.timeout_state {
                TimeoutState::Black(cs) | TimeoutState::White(cs) => match cs {
                    ClockState::CountingDown {
                        start_time,
                        time_remaining_at_start,
                    } => {
                        if now.duration_since(*start_time) >= *time_remaining_at_start {
                            if let ClockState::Stopped { clock_time } = self.clock_state {
                                self.clock_state = ClockState::CountingDown {
                                    start_time: *start_time + *time_remaining_at_start,
                                    time_remaining_at_start: clock_time,
                                }
                            } else {
                                panic!("Cannot end team timeout because game clock isn't stopped");
                            }
                        }
                    }
                    ClockState::CountingUp { .. } | ClockState::Stopped { .. } => {}
                },
                TimeoutState::Ref(_) | TimeoutState::PenaltyShot(_) | TimeoutState::None => {}
            };
        }
    }

    pub fn add_start_stop_sender(&mut self, sender: Sender<bool>) {
        self.start_stop_senders.push(sender);
    }

    fn start_game_clock(&mut self, now: Instant) {
        if let ClockState::Stopped { clock_time } = self.clock_state {
            info!("Starting the game clock");
            self.send_clock_running(true);
            match self.current_period {
                GamePeriod::SuddenDeath => {
                    self.clock_state = ClockState::CountingUp {
                        start_time: now,
                        time_at_start: clock_time,
                    };
                }
                _ => {
                    self.clock_state = ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: clock_time,
                    };
                }
            }
        }
    }

    fn stop_game_clock(&mut self, now: Instant) -> Result<()> {
        match self.clock_state {
            ClockState::CountingDown { .. } | ClockState::CountingUp { .. } => {
                info!("Stopping the game clock");
                self.send_clock_running(false);
                self.clock_state = ClockState::Stopped {
                    clock_time: self
                        .clock_state
                        .clock_time(now)
                        .ok_or(TournamentManagerError::NeedsUpdate)?,
                };
                Ok(())
            }
            ClockState::Stopped { .. } => Ok(()),
        }
    }

    fn send_clock_running(&self, running: bool) {
        for sender in &self.start_stop_senders {
            sender.send(running).unwrap();
        }
    }

    pub fn start_clock(&mut self, now: Instant) {
        let mut need_to_send = false;
        match &mut self.timeout_state {
            TimeoutState::None => self.start_game_clock(now),
            TimeoutState::Black(ref mut cs) | TimeoutState::White(ref mut cs) => {
                if let ClockState::Stopped { clock_time } = cs {
                    info!("Starting the timeout clock");
                    *cs = ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: *clock_time,
                    };
                    need_to_send = true;
                }
            }
            TimeoutState::Ref(ref mut cs) | TimeoutState::PenaltyShot(ref mut cs) => {
                if let ClockState::Stopped { clock_time } = cs {
                    info!("Starting the timeout clock");
                    *cs = ClockState::CountingUp {
                        start_time: now,
                        time_at_start: *clock_time,
                    };
                    need_to_send = true;
                }
            }
        };
        if need_to_send {
            self.send_clock_running(true);
        }
    }

    pub fn stop_clock(&mut self, now: Instant) -> Result<()> {
        let mut need_to_send = false;
        match &mut self.timeout_state {
            TimeoutState::None => self.stop_game_clock(now)?,
            TimeoutState::Black(ref mut cs) | TimeoutState::White(ref mut cs) => {
                if let ClockState::CountingDown { .. } = cs {
                    info!("Stopping the timeout clock");
                    *cs = ClockState::Stopped {
                        clock_time: cs
                            .clock_time(now)
                            .ok_or(TournamentManagerError::NeedsUpdate)?,
                    };
                    need_to_send = true;
                }
            }
            TimeoutState::Ref(ref mut cs) | TimeoutState::PenaltyShot(ref mut cs) => {
                if let ClockState::CountingUp { .. } = cs {
                    info!("Starting the timeout clock");
                    *cs = ClockState::Stopped {
                        clock_time: cs
                            .clock_time(now)
                            .ok_or(TournamentManagerError::NeedsUpdate)?,
                    };
                    need_to_send = true;
                }
            }
        };
        if need_to_send {
            self.send_clock_running(false);
        }
        Ok(())
    }

    pub fn set_game_clock_time(&mut self, clock_time: Duration) -> Result<()> {
        if !self.clock_is_running() {
            self.clock_state = ClockState::Stopped { clock_time };
            Ok(())
        } else {
            Err(TournamentManagerError::ClockIsRunning)
        }
    }

    #[cfg(test)]
    pub(super) fn set_period_and_game_clock_time(
        &mut self,
        period: GamePeriod,
        clock_time: Duration,
    ) {
        if let ClockState::Stopped { .. } = self.clock_state {
            self.current_period = period;
            self.clock_state = ClockState::Stopped { clock_time }
        } else {
            panic!("Can't edit period and remaing time while clock is running");
        }
    }

    #[cfg(test)]
    pub(super) fn set_game_start(&mut self, time: Instant) {
        if let ClockState::Stopped { .. } = self.clock_state {
            self.game_start_time = time;
        } else {
            panic!("Can't edit game start time while clock is running");
        }
    }

    /// Returns `None` if the clock time would be negative, or if `now` is before the start
    /// of the current period
    pub fn game_clock_time(&self, now: Instant) -> Option<Duration> {
        self.clock_state.clock_time(now)
    }

    /// Returns `None` if the clock time would be negative, if `now` is before the start
    /// of the timeout, or if there is no timeout
    pub fn timeout_time(&self, now: Instant) -> Option<Duration> {
        match &self.timeout_state {
            TimeoutState::None => None,
            TimeoutState::Black(cs)
            | TimeoutState::White(cs)
            | TimeoutState::Ref(cs)
            | TimeoutState::PenaltyShot(cs) => cs.clock_time(now),
        }
    }

    pub fn generate_snapshot(&self, now: Instant) -> Option<GameSnapshot> {
        self.game_clock_time(now)
            .and_then(|clock_time| clock_time.as_secs().try_into().ok())
            .map(|secs_in_period| GameSnapshot {
                current_period: self.current_period,
                secs_in_period,
                timeout: if self.clock_is_running() {
                    TimeoutSnapshot::None
                } else {
                    TimeoutSnapshot::Ref(0)
                },
                b_score: self.b_score,
                w_score: self.w_score,
                penalties: vec![],
            })
    }
}

#[derive(Debug, Clone)]
enum ClockState {
    Stopped {
        clock_time: Duration,
    },
    CountingDown {
        start_time: Instant,
        time_remaining_at_start: Duration,
    },
    CountingUp {
        start_time: Instant,
        time_at_start: Duration,
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
            ClockState::CountingDown { .. } | ClockState::CountingUp { .. } => true,
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
            ClockState::CountingUp {
                start_time,
                time_at_start,
            } => now
                .checked_duration_since(*start_time)
                .map(|s| s + *time_at_start),
            ClockState::Stopped { clock_time } => Some(*clock_time),
        }
    }

    fn as_secs_u16(&self, now: Instant) -> u16 {
        self.clock_time(now)
            .unwrap_or(Duration::from_secs(std::u16::MAX.into()))
            .as_secs()
            .try_into()
            .unwrap()
    }
}

#[derive(Debug, Clone)]
enum TimeoutState {
    None,
    Black(ClockState),
    White(ClockState),
    Ref(ClockState),
    PenaltyShot(ClockState),
}

impl TimeoutState {
    fn as_snapshot(&self, now: Instant) -> TimeoutSnapshot {
        match self {
            TimeoutState::None => TimeoutSnapshot::None,
            TimeoutState::Black(cs) => TimeoutSnapshot::Black(cs.as_secs_u16(now)),
            TimeoutState::White(cs) => TimeoutSnapshot::White(cs.as_secs_u16(now)),
            TimeoutState::Ref(cs) => TimeoutSnapshot::Ref(cs.as_secs_u16(now)),
            TimeoutState::PenaltyShot(cs) => TimeoutSnapshot::PenaltyShot(cs.as_secs_u16(now)),
        }
    }
}

#[derive(Debug, Error)]
pub enum TournamentManagerError {
    #[error("Can't edit clock time while clock is running")]
    ClockIsRunning,
    #[error("Can't start a {0} during {1}")]
    WrongGamePeriod(TimeoutSnapshot, GamePeriod),
    #[error("The {0} team has no more timeouts to use")]
    TooManyTeamTimeouts(&'static str),
    #[error("Already in a {0}")]
    AlreadyInTimeout(TimeoutSnapshot),
    #[error("Can only switch to Penalty Shot from Ref Timeout")]
    NotInRefTimeout,
    #[error("Can only switch to Ref Timeout from Penalty Shot")]
    NotInPenaltyShot,
    #[error("Can only switch to White Timeout from Black Timeout")]
    NotInBlackTimeout,
    #[error("Can only switch to Black Timeout from White Timeout")]
    NotInWhiteTimeout,
    #[error("update() needs to be called before this action can be performed")]
    NeedsUpdate,
}

pub type Result<T> = std::result::Result<T, TournamentManagerError>;

#[cfg(test)]
mod test {
    use super::*;
    use std::convert::TryInto;

    // TODO: test score clearing

    #[test]
    fn test_clock_start_stop() {
        let config = GameConfig {
            nominal_break: 13,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(13)));
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(13)));

        let next_time = start + Duration::from_secs(2);
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(11)));
        tm.stop_game_clock(next_time).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(11)));

        let next_time = next_time + Duration::from_secs(3);
        tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(18));
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(18)));
        tm.start_game_clock(next_time);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(18)));

        let next_time = next_time + Duration::from_secs(5);
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(23)));
        tm.stop_game_clock(next_time).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(23)));
    }

    struct TransitionTestSetup {
        config: GameConfig,
        game_start_offset: i64,
        start_period: GamePeriod,
        remaining: u64,
        score: Option<(u8, u8)>,
        time_delay: u64,
        end_period: GamePeriod,
        end_clock_time: u64,
    }

    fn test_transition(setup: TransitionTestSetup) {
        let TransitionTestSetup {
            config,
            game_start_offset,
            start_period,
            remaining,
            score,
            time_delay,
            end_period,
            end_clock_time,
        } = setup;

        let start = Instant::now();
        let next_time = start + Duration::from_secs(time_delay);
        let game_start = if game_start_offset < 0 {
            start - Duration::from_secs((-game_start_offset).try_into().unwrap())
        } else {
            start + Duration::from_secs(game_start_offset.try_into().unwrap())
        };

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(start_period, Duration::from_secs(remaining));
        tm.set_game_start(game_start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        if let Some((b, w)) = score {
            tm.set_scores(b, w, start);
        }
        tm.update(next_time);

        assert_eq!(tm.current_period(), end_period);
        assert_eq!(
            tm.game_clock_time(next_time),
            Some(Duration::from_secs(end_clock_time)),
        );
    }

    #[test]
    fn test_transition_bg_to_fh() {
        let config = GameConfig {
            half_play_duration: 3,
            ..Default::default()
        };

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, Duration::from_secs(1));
        tm.set_game_start(start);
        tm.start_game_clock(start);
        tm.update(next_time);

        assert_eq!(GamePeriod::FirstHalf, tm.current_period());
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(3)));
        assert_eq!(tm.current_game(), 1);
        assert_eq!(tm.game_start_time(), next_time);
    }

    #[test]
    fn test_transition_bg_to_fh_delayed() {
        let config = GameConfig {
            half_play_duration: 3,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::BetweenGames,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::FirstHalf,
            end_clock_time: 2,
        });
    }

    #[test]
    fn test_transition_fh_to_ht() {
        let config = GameConfig {
            half_time_duration: 5,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::FirstHalf,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::HalfTime,
            end_clock_time: 4,
        });
    }

    #[test]
    fn test_transition_ht_to_sh() {
        let config = GameConfig {
            half_play_duration: 6,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::HalfTime,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::SecondHalf,
            end_clock_time: 5,
        });
    }

    #[test]
    fn test_transition_sh_to_pot() {
        let config = GameConfig {
            has_overtime: true,
            pre_overtime_break: 7,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::SecondHalf,
            remaining: 1,
            score: Some((1, 1)),
            time_delay: 2,
            end_period: GamePeriod::PreOvertime,
            end_clock_time: 6,
        });
    }

    #[test]
    fn test_transition_sh_to_psd() {
        let config = GameConfig {
            has_overtime: false,
            sudden_death_allowed: true,
            pre_sudden_death_duration: 8,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::SecondHalf,
            remaining: 1,
            score: Some((1, 1)),
            time_delay: 2,
            end_period: GamePeriod::PreSuddenDeath,
            end_clock_time: 7,
        });
    }

    #[test]
    fn test_transition_sh_to_bg_tied_no_ot_no_sd() {
        let config = GameConfig {
            has_overtime: false,
            sudden_death_allowed: false,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 5,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 5 = 25 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -20,
            start_period: GamePeriod::SecondHalf,
            remaining: 1,
            score: Some((1, 1)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 3,
        });
    }

    #[test]
    fn test_transition_sh_to_bg_tied_no_ot_no_sd_use_min_break() {
        let config = GameConfig {
            has_overtime: false,
            sudden_death_allowed: false,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 7,
            minimum_break: 5,
            ..Default::default()
        };
        // 2*9 + 2 + 7 = 27 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -30,
            start_period: GamePeriod::SecondHalf,
            remaining: 1,
            score: Some((1, 1)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 4,
        });
    }

    #[test]
    fn test_transition_sh_to_bg_not_tied_no_ot_no_sd() {
        let config = GameConfig {
            has_overtime: false,
            sudden_death_allowed: false,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 6,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 6 = 26 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -20,
            start_period: GamePeriod::SecondHalf,
            remaining: 1,
            score: Some((2, 4)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 4,
        });
    }

    #[test]
    fn test_transition_sh_to_bg_not_tied_with_ot() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: true,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 8,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 8 = 28 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -20,
            start_period: GamePeriod::SecondHalf,
            remaining: 1,
            score: Some((3, 2)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 6,
        });
    }

    #[test]
    fn test_transition_pot_to_otfh() {
        let config = GameConfig {
            has_overtime: true,
            ot_half_play_duration: 4,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::PreOvertime,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::OvertimeFirstHalf,
            end_clock_time: 3,
        });
    }

    #[test]
    fn test_transition_otfh_to_otht() {
        let config = GameConfig {
            has_overtime: true,
            ot_half_time_duration: 5,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::OvertimeFirstHalf,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::OvertimeHalfTime,
            end_clock_time: 4,
        });
    }

    #[test]
    fn test_transition_otht_to_otsh() {
        let config = GameConfig {
            has_overtime: true,
            ot_half_play_duration: 7,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::OvertimeHalfTime,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::OvertimeSecondHalf,
            end_clock_time: 6,
        });
    }

    #[test]
    fn test_transition_otsh_to_psd() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: 9,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::OvertimeSecondHalf,
            remaining: 1,
            score: Some((1, 1)),
            time_delay: 2,
            end_period: GamePeriod::PreSuddenDeath,
            end_clock_time: 8,
        });
    }

    #[test]
    fn test_transition_otsh_to_bg_tied_no_sd() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: false,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 8,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 8 = 28 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -20,
            start_period: GamePeriod::OvertimeSecondHalf,
            remaining: 1,
            score: Some((1, 1)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 6,
        });
    }

    #[test]
    fn test_transition_otsh_to_bg_not_tied_no_sd() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: false,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 8,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 8 = 28 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -18,
            start_period: GamePeriod::OvertimeSecondHalf,
            remaining: 1,
            score: Some((10, 1)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 8,
        });
    }

    #[test]
    fn test_transition_otsh_to_bg_not_tied_with_sd() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: true,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 8,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 8 = 28 sec from game start to game start
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: -21,
            start_period: GamePeriod::OvertimeSecondHalf,
            remaining: 1,
            score: Some((11, 9)),
            time_delay: 2,
            end_period: GamePeriod::BetweenGames,
            end_clock_time: 5,
        });
    }

    #[test]
    fn test_transition_psd_to_sd() {
        let config = GameConfig {
            sudden_death_allowed: true,
            ..Default::default()
        };
        test_transition(TransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::PreSuddenDeath,
            remaining: 1,
            score: None,
            time_delay: 2,
            end_period: GamePeriod::SuddenDeath,
            end_clock_time: 1,
        });
    }

    #[test]
    fn test_end_sd() {
        let config = GameConfig {
            sudden_death_allowed: true,
            half_play_duration: 9,
            half_time_duration: 2,
            nominal_break: 8,
            minimum_break: 1,
            ..Default::default()
        };
        // 2*9 + 2 + 8 = 28 sec from game start to game start

        let start = Instant::now();
        let game_start = start - Duration::from_secs(17);
        let second_time = start + Duration::from_secs(2);
        let third_time = second_time + Duration::from_secs(2);
        let fourth_time = third_time + Duration::from_secs(3);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(5));
        tm.set_game_start(game_start);
        tm.start_game_clock(start);
        tm.set_scores(2, 2, start);
        tm.update(second_time);

        assert_eq!(tm.current_period(), GamePeriod::SuddenDeath);
        assert_eq!(
            tm.game_clock_time(second_time),
            Some(Duration::from_secs(7))
        );

        let tm_clone = tm.clone();

        tm.set_scores(3, 2, third_time);
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );

        let mut tm = tm_clone;
        let tm_clone = tm.clone();

        tm.add_b_score(1, third_time);
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );

        let mut tm = tm_clone;

        tm.add_w_score(1, third_time);
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );
    }
}
