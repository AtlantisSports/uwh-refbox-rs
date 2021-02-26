use crate::config::Game as GameConfig;
use crate::game_snapshot::{
    Color, GamePeriod, GameSnapshot, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
};
use log::*;
use std::{
    cmp::{max, Ordering},
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
    b_timeouts_used: u16,
    w_timeouts_used: u16,
    b_score: u8,
    w_score: u8,
    b_penalties: Vec<Penalty>,
    w_penalties: Vec<Penalty>,
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
            b_penalties: vec![],
            w_penalties: vec![],
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

    #[cfg(test)]
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
            gp => Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(0),
                gp,
            )),
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_w_timeout(&self) -> Result<()> {
        if let TimeoutState::Black(_) = &self.timeout_state {
            if self.w_timeouts_used < self.config.team_timeouts_per_half {
                Ok(())
            } else {
                Err(TournamentManagerError::TooManyTeamTimeouts("white"))
            }
        } else {
            Err(TournamentManagerError::NotInBlackTimeout)
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_b_timeout(&self) -> Result<()> {
        if let TimeoutState::White(_) = &self.timeout_state {
            if self.b_timeouts_used < self.config.team_timeouts_per_half {
                Ok(())
            } else {
                Err(TournamentManagerError::TooManyTeamTimeouts("black"))
            }
        } else {
            Err(TournamentManagerError::NotInWhiteTimeout)
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_ref_timeout(&self) -> Result<()> {
        if let TimeoutState::PenaltyShot(_) = &self.timeout_state {
            Ok(())
        } else {
            Err(TournamentManagerError::NotInPenaltyShot)
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_penalty_shot(&self) -> Result<()> {
        match self.current_period {
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath => {
                if let TimeoutState::Ref(_) = &self.timeout_state {
                    Ok(())
                } else {
                    Err(TournamentManagerError::NotInRefTimeout)
                }
            }
            gp => Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(0),
                gp,
            )),
        }
    }

    pub fn start_w_timeout(&mut self, now: Instant) -> Result<()> {
        match self.can_start_w_timeout() {
            Ok(()) => {
                info!("Starting a white timeout");
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::White(ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: Duration::from_secs(
                            self.config.team_timeout_duration.into(),
                        ),
                    });
                } else {
                    self.timeout_state = TimeoutState::White(ClockState::Stopped {
                        clock_time: Duration::from_secs(self.config.team_timeout_duration.into()),
                    });
                }
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
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::Black(ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: Duration::from_secs(
                            self.config.team_timeout_duration.into(),
                        ),
                    });
                } else {
                    self.timeout_state = TimeoutState::Black(ClockState::Stopped {
                        clock_time: Duration::from_secs(self.config.team_timeout_duration.into()),
                    });
                }
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
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::Ref(ClockState::CountingUp {
                        start_time: now,
                        time_at_start: Duration::from_secs(0),
                    });
                } else {
                    self.timeout_state = TimeoutState::Ref(ClockState::Stopped {
                        clock_time: Duration::from_secs(0),
                    });
                }
                Ok(())
            }
            e @ Err(_) => e,
        }
    }

    pub fn start_penalty_shot(&mut self, now: Instant) -> Result<()> {
        match self.can_start_penalty_shot() {
            Ok(()) => {
                info!("Starting a penalty shot");
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::PenaltyShot(ClockState::CountingUp {
                        start_time: now,
                        time_at_start: Duration::from_secs(0),
                    });
                } else {
                    self.timeout_state = TimeoutState::PenaltyShot(ClockState::Stopped {
                        clock_time: Duration::from_secs(0),
                    });
                }
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
                self.b_timeouts_used = self.b_timeouts_used.saturating_sub(1);
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
                self.w_timeouts_used = self.w_timeouts_used.saturating_sub(1);
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

    pub fn end_timeout(&mut self, now: Instant) -> Result<()> {
        match &self.timeout_state {
            TimeoutState::None => Err(TournamentManagerError::NotInTimeout),
            TimeoutState::Black(cs) | TimeoutState::White(cs) => {
                info!("Ending team timeout");
                match cs {
                    ClockState::Stopped { .. } => self.timeout_state = TimeoutState::None,
                    ClockState::CountingDown { .. } => {
                        self.start_game_clock(now);
                        self.timeout_state = TimeoutState::None;
                    }
                    ClockState::CountingUp { .. } => panic!("Invalid timeout state"),
                };
                Ok(())
            }
            TimeoutState::Ref(cs) | TimeoutState::PenaltyShot(cs) => {
                info!("Ending ref timeout or penalty shot");
                match cs {
                    ClockState::Stopped { .. } => self.timeout_state = TimeoutState::None,
                    ClockState::CountingUp { .. } => {
                        self.start_game_clock(now);
                        self.timeout_state = TimeoutState::None;
                    }
                    ClockState::CountingDown { .. } => panic!("Invalid timeout state"),
                };
                Ok(())
            }
        }
    }

    pub fn start_penalty(
        &mut self,
        color: Color,
        player_number: u8,
        kind: PenaltyKind,
        now: Instant,
    ) -> Result<()> {
        let start_time = if let Some(t) = self.game_clock_time(now) {
            t
        } else {
            return Err(TournamentManagerError::InvalidNowValue);
        };
        let penalty = Penalty {
            start_time,
            start_period: self.current_period,
            player_number,
            kind,
        };
        match color {
            Color::Black => self.b_penalties.push(penalty),
            Color::White => self.w_penalties.push(penalty),
        };
        Ok(())
    }

    pub fn delete_penalty(&mut self, color: Color, index: usize) -> Result<()> {
        match color {
            Color::Black => {
                if self.b_penalties.len() < index + 1 {
                    return Err(TournamentManagerError::InvalidIndex(color, index));
                }
                self.b_penalties.remove(index);
            }
            Color::White => {
                if self.w_penalties.len() < index + 1 {
                    return Err(TournamentManagerError::InvalidIndex(color, index));
                }
                self.w_penalties.remove(index);
            }
        }
        Ok(())
    }

    pub fn edit_penalty(
        &mut self,
        old_color: Color,
        index: usize,
        new_color: Color,
        new_player_number: u8,
        new_kind: PenaltyKind,
    ) -> Result<()> {
        let penalty = match old_color {
            Color::Black => self.b_penalties.get_mut(index),
            Color::White => self.w_penalties.get_mut(index),
        }
        .ok_or(TournamentManagerError::InvalidIndex(old_color, index))?;

        penalty.player_number = new_player_number;
        penalty.kind = new_kind;
        if old_color != new_color {
            match old_color {
                Color::Black => self.w_penalties.push(self.b_penalties.remove(index)),
                Color::White => self.b_penalties.push(self.w_penalties.remove(index)),
            };
        }
        Ok(())
    }

    fn cull_penalties(&mut self, now: Instant) -> Result<()> {
        let time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;

        let keep: Vec<_> = self
            .b_penalties
            .iter()
            .map(|pen| pen.is_complete(self.current_period, time, &self.config))
            .collect::<Option<Vec<_>>>()
            .ok_or(TournamentManagerError::InvalidNowValue)?
            .iter()
            .map(|k| !k)
            .collect();
        let mut i = 0;
        self.b_penalties.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });

        let keep: Vec<_> = self
            .w_penalties
            .iter()
            .map(|pen| pen.is_complete(self.current_period, time, &self.config))
            .collect::<Option<Vec<_>>>()
            .ok_or(TournamentManagerError::InvalidNowValue)?
            .iter()
            .map(|k| !k)
            .collect();
        let mut i = 0;
        self.w_penalties.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });

        Ok(())
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

    pub(super) fn update(&mut self, now: Instant) -> Result<()> {
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

            //TODO: Panics if `now` is invalid
            if now
                .checked_duration_since(start_time)
                .ok_or(TournamentManagerError::InvalidNowValue)?
                >= time_remaining_at_start
            {
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
                        self.b_timeouts_used = 0;
                        self.w_timeouts_used = 0;
                        self.b_penalties = vec![];
                        self.w_penalties = vec![];
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
                        self.cull_penalties(now)?;
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
                        };
                        self.cull_penalties(now)?;
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
                        };
                        self.cull_penalties(now)?;
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
                        };
                        self.cull_penalties(now)?;
                    }
                    GamePeriod::SuddenDeath => {
                        error!("Impossible state: in sudden death with clock counting down")
                    }
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
                            self.timeout_state = TimeoutState::None;
                        }
                    }
                    ClockState::CountingUp { .. } | ClockState::Stopped { .. } => {}
                },
                TimeoutState::Ref(_) | TimeoutState::PenaltyShot(_) | TimeoutState::None => {}
            };
        };

        Ok(())
    }

    pub fn add_start_stop_sender(&mut self, sender: Sender<bool>) {
        self.start_stop_senders.push(sender);
    }

    // Returns true if the clock was started, false if it was already running
    fn start_game_clock(&mut self, now: Instant) -> bool {
        if let ClockState::Stopped { clock_time } = self.clock_state {
            info!("Starting the game clock");
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
            true
        } else {
            false
        }
    }

    // Returns true if the clock was stopped, false if it was already stopped
    fn stop_game_clock(&mut self, now: Instant) -> Result<bool> {
        match self.clock_state {
            ClockState::CountingDown { .. } | ClockState::CountingUp { .. } => {
                info!("Stopping the game clock");
                self.clock_state = ClockState::Stopped {
                    clock_time: self
                        .clock_state
                        .clock_time(now)
                        .ok_or(TournamentManagerError::NeedsUpdate)?,
                };
                Ok(true)
            }
            ClockState::Stopped { .. } => Ok(false),
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
            TimeoutState::None => need_to_send = self.start_game_clock(now),
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
            TimeoutState::None => need_to_send = self.stop_game_clock(now)?,
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

    #[cfg(test)]
    fn set_timeout_state(&mut self, state: TimeoutState) {
        if let ClockState::Stopped { .. } = self.clock_state {
            self.timeout_state = state;
        } else {
            panic!("Can't edit timeout state while clock is running");
        }
    }

    #[cfg(test)]
    fn get_timeout_state(&self) -> TimeoutState {
        self.timeout_state.clone()
    }

    #[cfg(test)]
    pub(super) fn set_timeouts_used(&mut self, b: u16, w: u16) {
        self.b_timeouts_used = b;
        self.w_timeouts_used = w;
    }

    /// Returns `None` if the clock time would be negative, or if `now` is before the start
    /// of the current period
    pub fn game_clock_time(&self, now: Instant) -> Option<Duration> {
        self.clock_state.clock_time(now)
    }

    /// Returns `None` if the clock time would be negative, if `now` is before the start
    /// of the timeout, or if there is no timeout
    #[cfg(test)]
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
        let cur_time = self.game_clock_time(now)?;
        let secs_in_period = cur_time.as_secs().try_into().ok()?;

        let b_penalties = self
            .b_penalties
            .iter()
            .map(|pen| pen.as_snapshot(self.current_period, cur_time, &self.config))
            .collect::<Option<Vec<_>>>()?;
        let w_penalties = self
            .w_penalties
            .iter()
            .map(|pen| pen.as_snapshot(self.current_period, cur_time, &self.config))
            .collect::<Option<Vec<_>>>()?;

        Some(GameSnapshot {
            current_period: self.current_period,
            secs_in_period,
            timeout: self.timeout_state.as_snapshot(now),
            b_score: self.b_score,
            w_score: self.w_score,
            b_penalties,
            w_penalties,
        })
    }

    pub fn nanos_to_update(&self, now: Instant) -> Option<u32> {
        match (&self.timeout_state, self.current_period) {
            // cases where the clock is counting up
            (TimeoutState::Ref(cs), _) | (TimeoutState::PenaltyShot(cs), _) => cs
                .clock_time(now)
                .map(|ct| 1_000_000_000 - ct.subsec_nanos()),
            (TimeoutState::None, GamePeriod::SuddenDeath) => self
                .clock_state
                .clock_time(now)
                .map(|ct| 1_000_000_000 - ct.subsec_nanos()),
            // cases where the clock is counting down
            (TimeoutState::Black(cs), _) | (TimeoutState::White(cs), _) => {
                cs.clock_time(now).map(|ct| ct.subsec_nanos())
            }
            (TimeoutState::None, _) => self.clock_state.clock_time(now).map(|ct| ct.subsec_nanos()),
        }
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
            .unwrap_or_else(|| Duration::from_secs(std::u16::MAX.into()))
            .as_secs()
            .try_into()
            .unwrap()
    }
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyKind {
    OneMinute,
    TwoMinute,
    FiveMinute,
    TotalDismissal,
}

impl PenaltyKind {
    fn as_duration(self) -> Option<Duration> {
        match self {
            Self::OneMinute => Some(Duration::from_secs(60)),
            Self::TwoMinute => Some(Duration::from_secs(120)),
            Self::FiveMinute => Some(Duration::from_secs(300)),
            Self::TotalDismissal => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Penalty {
    kind: PenaltyKind,
    player_number: u8,
    start_period: GamePeriod,
    start_time: Duration,
}

impl Penalty {
    fn time_elapsed(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> Option<Duration> {
        match cur_per.cmp(&self.start_period) {
            Ordering::Equal => {
                if cur_per.penalties_run(config) {
                    cur_per.time_between(self.start_time, cur_time)
                } else {
                    // Capture a None if the timing is impossible, but no penalty time can have elapsed
                    cur_per
                        .time_between(self.start_time, cur_time)
                        .map(|_| Duration::from_secs(0))
                }
            }
            Ordering::Greater => {
                let mut elapsed = if self.start_period.penalties_run(config) {
                    self.start_time
                } else {
                    Duration::from_secs(0)
                };
                let mut period = self.start_period.next_period()?;
                while period < cur_per {
                    if period.penalties_run(config) {
                        elapsed += period.duration(config)?;
                    }
                    period = period.next_period()?;
                }
                if cur_per.penalties_run(config) {
                    elapsed += cur_per.time_elapsed_at(cur_time, config)?;
                }
                Some(elapsed)
            }
            Ordering::Less => None,
        }
    }

    fn time_remaining(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> Option<Duration> {
        let elapsed = self.time_elapsed(cur_per, cur_time, config)?;
        let total = self.kind.as_duration()?;
        Some(total.checked_sub(elapsed).unwrap_or(Duration::from_secs(0)))
    }

    fn is_complete(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> Option<bool> {
        match self.kind {
            PenaltyKind::TotalDismissal => Some(false),
            PenaltyKind::OneMinute | PenaltyKind::TwoMinute | PenaltyKind::FiveMinute => self
                .time_remaining(cur_per, cur_time, config)
                .map(|rem| rem == Duration::from_secs(0)),
        }
    }

    fn as_snapshot(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> Option<PenaltySnapshot> {
        let time = match self.kind {
            PenaltyKind::OneMinute | PenaltyKind::TwoMinute | PenaltyKind::FiveMinute => {
                PenaltyTime::Seconds(
                    self.time_remaining(cur_per, cur_time, config)?
                        .as_secs()
                        .try_into()
                        .unwrap(),
                )
            }
            PenaltyKind::TotalDismissal => PenaltyTime::TotalDismissal,
        };
        Some(PenaltySnapshot {
            player_number: self.player_number,
            time,
        })
    }
}

#[derive(Debug, PartialEq, Error)]
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
    #[error("Need to be in a timeout to end it")]
    NotInTimeout,
    #[error("update() needs to be called before this action can be performed")]
    NeedsUpdate,
    #[error("The `now` value passed is not valid")]
    InvalidNowValue,
    #[error("No {0} penalty exists at the index {1}")]
    InvalidIndex(Color, usize),
}

pub type Result<T> = std::result::Result<T, TournamentManagerError>;

#[cfg(test)]
mod test {
    use super::TournamentManagerError as TMErr;
    use super::*;
    use std::convert::TryInto;

    // TODO: test score clearing
    // TODO: test correct sending of time start/stop signals

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

    #[test]
    fn test_clock_start_stop_with_timeouts() {
        let config = GameConfig {
            nominal_break: 13,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let stop = start + Duration::from_secs(2);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(18));
        tm.set_timeout_state(TimeoutState::Black(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(stop), Some(Duration::from_secs(3)));

        tm.set_timeout_state(TimeoutState::White(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(stop), Some(Duration::from_secs(3)));

        tm.set_timeout_state(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(stop), Some(Duration::from_secs(7)));

        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_time(stop), Some(Duration::from_secs(7)));
    }

    #[test]
    fn test_can_start_timeouts() {
        let config = GameConfig {
            team_timeouts_per_half: 1,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let to_b = TimeoutSnapshot::Black(0);
        let to_w = TimeoutSnapshot::White(0);
        let to_r = TimeoutSnapshot::Ref(0);
        let to_ps = TimeoutSnapshot::PenaltyShot(0);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        assert_eq!(tm.can_start_b_timeout(), Ok(()));
        assert_eq!(tm.can_start_w_timeout(), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(10));
        assert_eq!(tm.can_start_b_timeout(), Ok(()));
        assert_eq!(tm.can_start_w_timeout(), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        let otfh = GamePeriod::OvertimeFirstHalf;
        tm.set_period_and_game_clock_time(otfh, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TMErr::WrongGamePeriod(to_b, otfh))
        );
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TMErr::WrongGamePeriod(to_w, otfh))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        let otsh = GamePeriod::OvertimeSecondHalf;
        tm.set_period_and_game_clock_time(otsh, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TMErr::WrongGamePeriod(to_b, otsh))
        );
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TournamentManagerError::WrongGamePeriod(to_w, otsh))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        let otsd = GamePeriod::SuddenDeath;
        tm.set_period_and_game_clock_time(otsd, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TournamentManagerError::WrongGamePeriod(to_b, otsd))
        );
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TournamentManagerError::WrongGamePeriod(to_w, otsd))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        let ht = GamePeriod::HalfTime;
        tm.set_period_and_game_clock_time(ht, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TournamentManagerError::WrongGamePeriod(to_b, ht))
        );
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TournamentManagerError::WrongGamePeriod(to_w, ht))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(
            tm.can_start_penalty_shot(),
            Err(TournamentManagerError::WrongGamePeriod(to_ps, ht))
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.set_timeout_state(TimeoutState::Black(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TournamentManagerError::AlreadyInTimeout(to_b))
        );
        assert_eq!(tm.can_start_w_timeout(), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        tm.set_timeout_state(TimeoutState::White(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.can_start_b_timeout(), Ok(()));
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TournamentManagerError::AlreadyInTimeout(to_w))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        tm.set_timeout_state(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.can_start_b_timeout(), Ok(()));
        assert_eq!(tm.can_start_w_timeout(), Ok(()));
        assert_eq!(
            tm.can_start_ref_timeout(),
            Err(TournamentManagerError::AlreadyInTimeout(to_r))
        );
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));

        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.can_start_b_timeout(), Ok(()));
        assert_eq!(tm.can_start_w_timeout(), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(
            tm.can_start_penalty_shot(),
            Err(TournamentManagerError::AlreadyInTimeout(to_ps))
        );

        tm.set_timeout_state(TimeoutState::None);
        tm.set_timeouts_used(1, 1);
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TournamentManagerError::TooManyTeamTimeouts("black"))
        );
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TournamentManagerError::TooManyTeamTimeouts("white"))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
    }

    #[test]
    fn test_start_timeouts() {
        let config = GameConfig {
            team_timeouts_per_half: 1,
            team_timeout_duration: 10,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let t_o_start = start + Duration::from_secs(2);
        let mid_t_o = t_o_start + Duration::from_secs(3);
        let t_o_end = t_o_start + Duration::from_secs(10);
        let after_t_o = t_o_end + Duration::from_secs(2);

        // Test starting timeouts with the clock stopped
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_b_timeout(start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Black(ClockState::Stopped {
                clock_time: Duration::from_secs(10)
            })
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_w_timeout(start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::White(ClockState::Stopped {
                clock_time: Duration::from_secs(10)
            })
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_ref_timeout(start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Ref(ClockState::Stopped {
                clock_time: Duration::from_secs(0)
            })
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_penalty_shot(start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::PenaltyShot(ClockState::Stopped {
                clock_time: Duration::from_secs(0)
            })
        );

        // Test starting timeouts with clock running, and test team timeouts ending
        tm.set_timeouts_used(0, 0);
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_b_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Black(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_time(mid_t_o), Some(Duration::from_secs(7)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Black(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.timeout_time(t_o_end), Some(Duration::from_secs(0)));
        assert_eq!(tm.timeout_time(after_t_o), None);
        tm.update(after_t_o).unwrap();
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.timeout_time(after_t_o), None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(Duration::from_secs(26)));
        assert_eq!(
            tm.start_b_timeout(t_o_start),
            Err(TournamentManagerError::TooManyTeamTimeouts("black"))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_w_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::White(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_time(mid_t_o), Some(Duration::from_secs(7)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::White(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.timeout_time(t_o_end), Some(Duration::from_secs(0)));
        assert_eq!(tm.timeout_time(after_t_o), None);
        tm.update(after_t_o).unwrap();
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.timeout_time(after_t_o), None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(Duration::from_secs(26)));
        assert_eq!(
            tm.start_w_timeout(t_o_start),
            Err(TournamentManagerError::TooManyTeamTimeouts("white"))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_ref_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Ref(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_time(mid_t_o), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Ref(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(tm.timeout_time(t_o_end), Some(Duration::from_secs(10)));

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_penalty_shot(t_o_start), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_time(mid_t_o), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(tm.timeout_time(t_o_end), Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_end_timeouts() {
        let config = GameConfig::default();
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let t_o_start = start - Duration::from_secs(2);
        let t_o_end = start + Duration::from_secs(5);
        let after_t_o = t_o_end + Duration::from_secs(10);

        let two_secs = Duration::from_secs(2);
        let ten_secs = Duration::from_secs(10);
        let twenty_secs = Duration::from_secs(20);
        let thirty_secs = Duration::from_secs(30);

        // Test ending timeouts with the clock stopped
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        assert_eq!(tm.end_timeout(t_o_end), Err(TMErr::NotInTimeout));
        tm.set_timeout_state(TimeoutState::Black(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::White(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::Ref(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        // Test ending timeouts with the clock running
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::Black(ClockState::CountingDown {
            start_time: t_o_start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::White(ClockState::CountingDown {
            start_time: t_o_start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::Ref(ClockState::CountingUp {
            start_time: t_o_start,
            time_at_start: ten_secs,
        }));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::CountingUp {
            start_time: t_o_start,
            time_at_start: ten_secs,
        }));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.get_timeout_state(), TimeoutState::None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);
    }

    #[test]
    fn test_can_switch_timeouts() {
        let config = GameConfig {
            team_timeouts_per_half: 1,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let ten_secs = Duration::from_secs(10);

        tm.set_timeouts_used(1, 1);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::Black(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(
            tm.can_switch_to_w_timeout(),
            Err(TMErr::TooManyTeamTimeouts("white"))
        );
        tm.set_timeout_state(TimeoutState::White(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(
            tm.can_switch_to_b_timeout(),
            Err(TMErr::TooManyTeamTimeouts("black"))
        );

        tm.set_timeouts_used(0, 0);

        tm.set_timeout_state(TimeoutState::Black(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(tm.can_switch_to_b_timeout(), Err(TMErr::NotInWhiteTimeout));
        assert_eq!(tm.can_switch_to_w_timeout(), Ok(()));
        assert_eq!(tm.can_switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));

        tm.set_timeout_state(TimeoutState::White(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(tm.can_switch_to_b_timeout(), Ok(()));
        assert_eq!(tm.can_switch_to_w_timeout(), Err(TMErr::NotInBlackTimeout));
        assert_eq!(tm.can_switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));

        tm.set_timeout_state(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        }));
        assert_eq!(tm.can_switch_to_b_timeout(), Err(TMErr::NotInWhiteTimeout));
        assert_eq!(tm.can_switch_to_w_timeout(), Err(TMErr::NotInBlackTimeout));
        assert_eq!(tm.can_switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.can_switch_to_penalty_shot(), Ok(()));

        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        }));
        assert_eq!(tm.can_switch_to_b_timeout(), Err(TMErr::NotInWhiteTimeout));
        assert_eq!(tm.can_switch_to_w_timeout(), Err(TMErr::NotInBlackTimeout));
        assert_eq!(tm.can_switch_to_ref_timeout(), Ok(()));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));

        tm.set_period_and_game_clock_time(GamePeriod::HalfTime, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        }));
        assert_eq!(
            tm.can_switch_to_penalty_shot(),
            Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(0),
                GamePeriod::HalfTime
            ))
        );
    }

    #[test]
    fn test_switch_timeouts() {
        let config = GameConfig {
            team_timeouts_per_half: 1,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let ten_secs = Duration::from_secs(10);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::Black(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(tm.switch_to_b_timeout(), Err(TMErr::NotInWhiteTimeout));
        assert_eq!(tm.switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(tm.switch_to_w_timeout(), Ok(()));

        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::White(ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            })
        );
        assert_eq!(tm.switch_to_w_timeout(), Err(TMErr::NotInBlackTimeout));
        assert_eq!(tm.switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(tm.switch_to_b_timeout(), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Black(ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            })
        );

        tm.set_timeout_state(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        }));
        assert_eq!(tm.switch_to_b_timeout(), Err(TMErr::NotInWhiteTimeout));
        assert_eq!(tm.switch_to_w_timeout(), Err(TMErr::NotInBlackTimeout));
        assert_eq!(tm.switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.switch_to_penalty_shot(), Ok(()));

        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: start,
                time_at_start: ten_secs,
            })
        );
        assert_eq!(tm.switch_to_b_timeout(), Err(TMErr::NotInWhiteTimeout));
        assert_eq!(tm.switch_to_w_timeout(), Err(TMErr::NotInBlackTimeout));
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(tm.switch_to_ref_timeout(), Ok(()));
        assert_eq!(
            tm.get_timeout_state(),
            TimeoutState::Ref(ClockState::CountingUp {
                start_time: start,
                time_at_start: ten_secs,
            })
        );
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
        tm.update(next_time).unwrap();

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
        tm.update(next_time).unwrap();

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
        tm.update(second_time).unwrap();

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

    #[test]
    fn test_penalty_time_elapsed() {
        let all_periods_config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: true,
            half_play_duration: 5,
            half_time_duration: 7,
            pre_overtime_break: 9,
            ot_half_play_duration: 11,
            ot_half_time_duration: 13,
            pre_sudden_death_duration: 15,
            ..Default::default()
        };
        let sd_only_config = GameConfig {
            has_overtime: false,
            sudden_death_allowed: true,
            ..all_periods_config.clone()
        };
        let no_sd_no_ot_config = GameConfig {
            has_overtime: false,
            sudden_death_allowed: false,
            ..all_periods_config.clone()
        };

        // (start_period, start_time, end_period, end_time, config, result, msg)
        let test_cases = vec![
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Some(Duration::from_secs(2)),
                "Both first half",
            ),
            (
                GamePeriod::OvertimeFirstHalf,
                Duration::from_secs(10),
                GamePeriod::OvertimeFirstHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Some(Duration::from_secs(8)),
                "Both overtime first half",
            ),
            (
                GamePeriod::SuddenDeath,
                Duration::from_secs(10),
                GamePeriod::SuddenDeath,
                Duration::from_secs(55),
                &all_periods_config,
                Some(Duration::from_secs(45)),
                "Both sudden death",
            ),
            (
                GamePeriod::HalfTime,
                Duration::from_secs(4),
                GamePeriod::HalfTime,
                Duration::from_secs(2),
                &all_periods_config,
                Some(Duration::from_secs(0)),
                "Both half time",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SecondHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Some(Duration::from_secs(7)),
                "First half to second half",
            ),
            (
                GamePeriod::BetweenGames,
                Duration::from_secs(4),
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Some(Duration::from_secs(3)),
                "Between games to first half",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                &all_periods_config,
                None,
                "Both first half, bad timing",
            ),
            (
                GamePeriod::HalfTime,
                Duration::from_secs(2),
                GamePeriod::HalfTime,
                Duration::from_secs(4),
                &all_periods_config,
                None,
                "Both half time, bad timing",
            ),
            (
                GamePeriod::HalfTime,
                Duration::from_secs(2),
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                &all_periods_config,
                None,
                "Half time to first half",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SuddenDeath,
                Duration::from_secs(25),
                &all_periods_config,
                Some(Duration::from_secs(56)),
                "First half to sudden death, all periods",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SuddenDeath,
                Duration::from_secs(25),
                &sd_only_config,
                Some(Duration::from_secs(34)),
                "First half to sudden death, sudden death no overtime",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SuddenDeath,
                Duration::from_secs(25),
                &no_sd_no_ot_config,
                Some(Duration::from_secs(9)),
                "First half to sudden death, no sudden death or overtime",
            ),
        ];

        for (start_period, start_time, end_period, end_time, config, result, msg) in test_cases {
            let penalty = Penalty {
                player_number: 0,
                kind: PenaltyKind::OneMinute,
                start_time,
                start_period,
            };
            assert_eq!(
                penalty.time_elapsed(end_period, end_time, config),
                result,
                "{}",
                msg
            );
        }
    }

    #[test]
    fn test_penalty_time_remaining() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: true,
            half_play_duration: 5,
            half_time_duration: 7,
            pre_overtime_break: 9,
            ot_half_play_duration: 11,
            ot_half_time_duration: 13,
            pre_sudden_death_duration: 15,
            ..Default::default()
        };

        // (start_period, start_time, kind, end_period, end_time, result, msg)
        let test_cases = vec![
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::OneMinute,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                Some(Duration::from_secs(58)),
                "Both first half, 1m",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::TwoMinute,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                Some(Duration::from_secs(118)),
                "Both first half, 2m",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::FiveMinute,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                Some(Duration::from_secs(298)),
                "Both first half, 5m",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::TotalDismissal,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                None,
                "Both first half, TD",
            ),
            (
                GamePeriod::SuddenDeath,
                Duration::from_secs(5),
                PenaltyKind::OneMinute,
                GamePeriod::SuddenDeath,
                Duration::from_secs(70),
                Some(Duration::from_secs(0)),
                "Penalty Complete",
            ),
        ];

        for (start_period, start_time, kind, end_period, end_time, result, msg) in test_cases {
            let penalty = Penalty {
                player_number: 0,
                kind,
                start_time,
                start_period,
            };
            assert_eq!(
                penalty.time_remaining(end_period, end_time, &config),
                result,
                "{}",
                msg
            );
        }
    }

    #[test]
    fn test_penalty_is_complete() {
        let config = GameConfig {
            has_overtime: true,
            sudden_death_allowed: true,
            half_play_duration: 5,
            half_time_duration: 7,
            pre_overtime_break: 9,
            ot_half_play_duration: 11,
            ot_half_time_duration: 13,
            pre_sudden_death_duration: 15,
            ..Default::default()
        };

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::OneMinute,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(60), &config),
            Some(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(65), &config),
            Some(true)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(70), &config),
            Some(true)
        );

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::TwoMinute,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(120), &config),
            Some(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(125), &config),
            Some(true)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(130), &config),
            Some(true)
        );

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::FiveMinute,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(300), &config),
            Some(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(305), &config),
            Some(true)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(310), &config),
            Some(true)
        );

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::TotalDismissal,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(300), &config),
            Some(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(305), &config),
            Some(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(310), &config),
            Some(false)
        );
    }
}
