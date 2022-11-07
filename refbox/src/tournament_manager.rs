use derivative::Derivative;
use log::*;
use std::{
    cmp::{max, min, Ordering},
    convert::TryInto,
    ops::{Index, IndexMut},
};
use thiserror::Error;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};
use tokio::{
    sync::watch,
    time::{Duration, Instant},
};
use uwh_common::{
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{
        Color, GamePeriod, GameSnapshot, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
    },
    uwhscores::TimingRules,
};

const MAX_TIME_VAL: Duration = Duration::from_secs(MAX_LONG_STRINGABLE_SECS as u64);
const RECENT_GOAL_TIME: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct TournamentManager {
    config: GameConfig,
    game_number: u32,
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
    has_reset: bool,
    start_stop_tx: watch::Sender<bool>,
    start_stop_rx: watch::Receiver<bool>,
    next_game: Option<NextGameInfo>,
    next_scheduled_start: Option<Instant>,
    reset_game_time: Duration,
    timezone: UtcOffset,
    recent_goal: Option<(Color, u8, GamePeriod, Duration)>,
}

impl TournamentManager {
    pub fn new(config: GameConfig) -> Self {
        let (start_stop_tx, start_stop_rx) = watch::channel(false);
        Self {
            game_number: 0,
            game_start_time: Instant::now(),
            current_period: GamePeriod::BetweenGames,
            clock_state: ClockState::Stopped {
                clock_time: config.nominal_break,
            },
            timeout_state: TimeoutState::None,
            w_timeouts_used: 0,
            b_timeouts_used: 0,
            b_score: 0,
            w_score: 0,
            b_penalties: vec![],
            w_penalties: vec![],
            has_reset: true,
            start_stop_tx,
            start_stop_rx,
            next_game: None,
            next_scheduled_start: None,
            reset_game_time: config.nominal_break,
            config,
            timezone: UtcOffset::UTC,
            recent_goal: None,
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

    pub fn add_b_score(&mut self, player_num: u8, now: Instant) {
        info!(
            "{} Score by Black player #{player_num}",
            self.status_string(now)
        );
        self.recent_goal = self
            .game_clock_time(now)
            .map(|time| (Color::Black, player_num, self.current_period, time));
        self.set_scores(self.b_score + 1, self.w_score, now);
    }

    pub fn add_w_score(&mut self, player_num: u8, now: Instant) {
        info!(
            "{} Score by White player #{player_num}",
            self.status_string(now)
        );
        self.recent_goal = self
            .game_clock_time(now)
            .map(|time| (Color::White, player_num, self.current_period, time));
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
        info!(
            "{} Scores set to B({b_score}) W({w_score})",
            self.status_string(now)
        );
        if self.current_period == GamePeriod::SuddenDeath && b_score != w_score {
            self.end_game(now);
        }
    }

    pub fn set_timezone(&mut self, timezone: UtcOffset) {
        self.timezone = timezone;
    }

    pub fn current_period(&self) -> GamePeriod {
        self.current_period
    }

    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    pub fn next_game_info(&self) -> &Option<NextGameInfo> {
        &self.next_game
    }

    /// The config can only be modified between games
    pub fn set_config(&mut self, config: GameConfig) -> Result<()> {
        if self.current_period != GamePeriod::BetweenGames {
            return Err(TournamentManagerError::GameInProgress);
        }
        self.config = config;
        Ok(())
    }

    pub fn clear_scheduled_game_start(&mut self) {
        self.next_scheduled_start = None;
    }

    pub fn game_number(&self) -> u32 {
        self.game_number
    }

    pub fn next_game_number(&self) -> u32 {
        if self.current_period == GamePeriod::BetweenGames {
            if let Some(ref info) = self.next_game {
                return info.number;
            }
        }
        self.game_number + 1
    }

    pub fn set_game_number(&mut self, number: u32) {
        info!("Game Number set to {number}");
        self.game_number = number;
    }

    pub fn set_next_game(&mut self, info: NextGameInfo) {
        info!("Next Game Info set to {info:?}");
        self.next_game = Some(info);
    }

    pub fn reset_game(&mut self, now: Instant) {
        info!("{} Resetting Game", self.status_string(now));
        let was_running = self.clock_is_running();

        self.current_period = GamePeriod::BetweenGames;
        self.clock_state = ClockState::Stopped {
            clock_time: self.config.minimum_break,
        };
        self.timeout_state = TimeoutState::None;
        self.reset();

        if was_running {
            self.start_game_clock(now);
        }
    }

    fn reset(&mut self) {
        self.b_score = 0;
        self.w_score = 0;
        self.b_penalties.clear();
        self.w_penalties.clear();
        self.has_reset = true;
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
                        Err(TournamentManagerError::TooManyTeamTimeouts(Color::White))
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
                        Err(TournamentManagerError::TooManyTeamTimeouts(Color::Black))
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
                Err(TournamentManagerError::TooManyTeamTimeouts(Color::White))
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
                Err(TournamentManagerError::TooManyTeamTimeouts(Color::Black))
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
                info!("{} Starting a white timeout", self.status_string(now));
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::White(ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: self.config.team_timeout_duration,
                    });
                } else {
                    self.timeout_state = TimeoutState::White(ClockState::Stopped {
                        clock_time: self.config.team_timeout_duration,
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
                info!("{} Starting a black timeout", self.status_string(now));
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::Black(ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: self.config.team_timeout_duration,
                    });
                } else {
                    self.timeout_state = TimeoutState::Black(ClockState::Stopped {
                        clock_time: self.config.team_timeout_duration,
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
                info!("{} Starting a ref timeout", self.status_string(now));
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::Ref(ClockState::CountingUp {
                        start_time: now,
                        time_at_start: Duration::ZERO,
                    });
                } else {
                    self.timeout_state = TimeoutState::Ref(ClockState::Stopped {
                        clock_time: Duration::ZERO,
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
                info!("{} Starting a penalty shot", self.status_string(now));
                if self.clock_is_running() {
                    self.stop_game_clock(now)?;
                    self.timeout_state = TimeoutState::PenaltyShot(ClockState::CountingUp {
                        start_time: now,
                        time_at_start: Duration::ZERO,
                    });
                } else {
                    self.timeout_state = TimeoutState::PenaltyShot(ClockState::Stopped {
                        clock_time: Duration::ZERO,
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
                info!("{} Ending team timeout", self.status_string(now));
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
                let timeout_time = match cs.clone() {
                    ClockState::Stopped { clock_time } => {
                        self.timeout_state = TimeoutState::None;
                        Some(clock_time)
                    }
                    ClockState::CountingUp {
                        start_time,
                        time_at_start,
                    } => {
                        self.start_game_clock(now);
                        self.timeout_state = TimeoutState::None;
                        now.checked_duration_since(start_time)
                            .map(|d| d + time_at_start)
                    }
                    ClockState::CountingDown { .. } => panic!("Invalid timeout state"),
                };

                if let Some(dur) = timeout_time {
                    info!(
                        "{} Ending ref timeout or penalty shot. The timeout duration was {:?}",
                        self.status_string(now),
                        dur
                    );
                } else {
                    info!(
                        "{} Ending ref timeout or penalty shot",
                        self.status_string(now)
                    );
                }

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
        info!(
            "{} Starting a {kind:?} penalty for {color} player #{player_number}",
            self.status_string(now)
        );
        let start_time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;

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
        let vec = match color {
            Color::Black => &mut self.b_penalties,
            Color::White => &mut self.w_penalties,
        };

        if vec.len() < index + 1 {
            return Err(TournamentManagerError::InvalidIndex(color, index));
        }
        let pen = vec.remove(index);
        info!(
            "{} Deleting {color} player #{}'s {:?} penalty",
            self.status_string(Instant::now()),
            pen.player_number,
            pen.kind
        );

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
        let status_str = self.status_string(Instant::now());
        let penalty = match old_color {
            Color::Black => self.b_penalties.get_mut(index),
            Color::White => self.w_penalties.get_mut(index),
        }
        .ok_or(TournamentManagerError::InvalidIndex(old_color, index))?;
        info!(
            "{status_str} Editing {old_color} player #{}'s {:?} penalty: \
            it is now {new_color} player #{new_player_number}'s {new_kind:?} penalty",
            penalty.player_number, penalty.kind
        );

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

    pub fn limit_pen_list_len(&mut self, color: Color, limit: usize, now: Instant) -> Result<()> {
        let time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;
        let period = self.current_period;

        let list = match color {
            Color::Black => &mut self.b_penalties,
            Color::White => &mut self.w_penalties,
        };

        while list.len() > limit {
            let mut index = None;
            'inner: for (i, pen) in list.iter().enumerate() {
                if pen
                    .is_complete(period, time, &self.config)
                    .ok_or(TournamentManagerError::InvalidNowValue)?
                {
                    index = Some(i);
                    break 'inner;
                }
            }

            if let Some(i) = index {
                list.remove(i);
            } else {
                return Err(TournamentManagerError::TooManyPenalties(limit));
            }
        }
        Ok(())
    }

    fn cull_penalties(&mut self, now: Instant) -> Result<()> {
        let time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;
        let period = self.current_period;

        info!("{} Culling penalties", self.status_string(now));

        for vec in [&mut self.b_penalties, &mut self.w_penalties].into_iter() {
            let keep = vec
                .iter()
                .map(|pen| pen.is_complete(period, time, &self.config).map(|k| !k))
                .collect::<Option<Vec<_>>>()
                .ok_or(TournamentManagerError::InvalidNowValue)?;
            let mut i = 0;
            vec.retain(|_| {
                let k = keep[i];
                i += 1;
                k
            });
        }

        Ok(())
    }

    fn calc_time_to_next_game(&self, now: Instant, from_time: Instant) -> Duration {
        info!("Next game info is: {:?}", self.next_game);
        let scheduled_start =
            if let Some(start_time) = self.next_game.as_ref().and_then(|info| info.start_time) {
                let cur_time = OffsetDateTime::now_utc().to_offset(self.timezone);
                info!("Current time is: {cur_time}");

                let start_time = start_time.assume_offset(self.timezone);
                info!("Start time is: {start_time}");

                let time_to_game = start_time - cur_time;
                info!("Calculated time to next game: {time_to_game}");

                match time_to_game.try_into() {
                    Ok(dur) => Instant::now() + dur,
                    Err(e) => {
                        error!("Failed to calculate time to next game start: {e}");
                        now
                    }
                }
            } else {
                self.next_scheduled_start
                    .unwrap_or(now + self.config.nominal_break)
            };

        let time_remaining_at_start =
            if let Some(time_until_start) = scheduled_start.checked_duration_since(from_time) {
                max(time_until_start, self.config.minimum_break)
            } else {
                self.config.minimum_break
            };

        // Make sure the value isn't too big
        min(time_remaining_at_start, MAX_TIME_VAL)
    }

    pub fn apply_next_game_start(&mut self, now: Instant) -> Result<()> {
        if self.current_period != GamePeriod::BetweenGames {
            return Err(TournamentManagerError::GameInProgress);
        }

        let next_game_info = if let Some(info) = self.next_game.as_ref() {
            info
        } else {
            return Err(TournamentManagerError::NoNextGameInfo);
        };

        if let Some(ref timing) = next_game_info.timing {
            self.config = timing.clone().into();
        }

        let time_remaining_at_start = self.calc_time_to_next_game(now, now);

        info!(
            "{} Setting between games time based on uwhscores info: {time_remaining_at_start:?}",
            self.status_string(now),
        );

        self.clock_state = ClockState::CountingDown {
            start_time: now,
            time_remaining_at_start,
        };

        Ok(())
    }

    fn end_game(&mut self, now: Instant) {
        let was_running = self.clock_is_running();

        self.current_period = GamePeriod::BetweenGames;

        info!(
            "{} Ending game {}. Score is B({}), W({})",
            self.status_string(now),
            self.game_number,
            self.b_score,
            self.w_score
        );

        let game_end = match self.clock_state {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => start_time + time_remaining_at_start,
            ClockState::CountingUp { .. } | ClockState::Stopped { .. } => now,
        };

        let time_remaining_at_start = self.calc_time_to_next_game(now, game_end);

        info!(
            "{} Entering between games, time to next game is {time_remaining_at_start:?}",
            self.status_string(now),
        );

        self.clock_state = ClockState::CountingDown {
            start_time: game_end,
            time_remaining_at_start,
        };

        if !was_running {
            self.send_clock_running(true);
        }

        self.reset_game_time =
            time_remaining_at_start.saturating_sub(self.config.post_game_duration);
        info!(
            "{} Will reset game at {:?}",
            self.status_string(now),
            self.reset_game_time
        )
    }

    fn start_game(&mut self, start_time: Instant) {
        if !self.has_reset {
            info!("Resetting game");
            self.reset();
        }

        self.game_number = self.next_game_number();

        if let Some(timing) = self.next_game.take().and_then(|info| info.timing) {
            self.config = timing.into();
        }

        info!(
            "{} Entering first half of game {}",
            self.status_string(start_time),
            self.game_number
        );
        self.current_period = GamePeriod::FirstHalf;
        self.game_start_time = start_time;
        self.b_timeouts_used = 0;
        self.w_timeouts_used = 0;
        self.has_reset = false;

        let sched_start = self.next_scheduled_start.unwrap_or(start_time);
        self.next_scheduled_start = Some(
            sched_start
                + 2 * self.config.half_play_duration
                + self.config.half_time_duration
                + self.config.nominal_break,
        );
    }

    pub fn would_end_game(&self, now: Instant) -> Result<bool> {
        if let ClockState::CountingDown {
            start_time,
            time_remaining_at_start,
        } = self.clock_state
        {
            let time = now
                .checked_duration_since(start_time)
                .ok_or(TournamentManagerError::InvalidNowValue)?;

            Ok(time >= time_remaining_at_start
                && ((self.current_period == GamePeriod::SecondHalf
                    && (self.b_score != self.w_score
                        || (!self.config.overtime_allowed && !self.config.sudden_death_allowed)))
                    || (self.current_period == GamePeriod::OvertimeSecondHalf
                        && (self.b_score != self.w_score || !self.config.sudden_death_allowed))))
        } else {
            Ok(false)
        }
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

            if !self.has_reset
                && self.current_period == GamePeriod::BetweenGames
                && self.game_clock_time(now).unwrap_or(Duration::ZERO) <= self.reset_game_time
            {
                info!("{} Resetting game", self.status_string(now));
                self.reset();
            };

            if time >= time_remaining_at_start {
                let mut need_cull = false;
                match self.current_period {
                    GamePeriod::BetweenGames => {
                        self.start_game(start_time + time_remaining_at_start);
                    }
                    GamePeriod::FirstHalf => {
                        info!("{} Entering half time", self.status_string(now));
                        self.current_period = GamePeriod::HalfTime;
                    }
                    GamePeriod::HalfTime => {
                        info!("{} Entering second half", self.status_string(now));
                        self.current_period = GamePeriod::SecondHalf;
                        self.w_timeouts_used = 0;
                        self.b_timeouts_used = 0;
                        need_cull = true;
                    }
                    GamePeriod::SecondHalf => {
                        if self.b_score != self.w_score
                            || (!self.config.overtime_allowed && !self.config.sudden_death_allowed)
                        {
                            self.end_game(now);
                        } else if self.config.overtime_allowed {
                            info!(
                                "{} Entering pre-overtime. Score is B({}), W({})",
                                self.status_string(now),
                                self.b_score,
                                self.w_score
                            );
                            self.current_period = GamePeriod::PreOvertime;
                        } else {
                            info!(
                                "{} Entering pre-sudden death. Score is B({}), W({})",
                                self.status_string(now),
                                self.b_score,
                                self.w_score
                            );
                            self.current_period = GamePeriod::PreSuddenDeath;
                        }
                    }
                    GamePeriod::PreOvertime => {
                        info!("{} Entering overtime first half", self.status_string(now));
                        self.current_period = GamePeriod::OvertimeFirstHalf;
                        need_cull = true;
                    }
                    GamePeriod::OvertimeFirstHalf => {
                        info!("{} Entering overtime half time", self.status_string(now));
                        self.current_period = GamePeriod::OvertimeHalfTime;
                    }
                    GamePeriod::OvertimeHalfTime => {
                        info!("{} Entering ovetime second half", self.status_string(now));
                        self.current_period = GamePeriod::OvertimeSecondHalf;
                        need_cull = true;
                    }
                    GamePeriod::OvertimeSecondHalf => {
                        if self.b_score != self.w_score || !self.config.sudden_death_allowed {
                            self.end_game(now);
                        } else {
                            info!(
                                "{} Entering pre-sudden death. Score is B({}), W({})",
                                self.status_string(now),
                                self.b_score,
                                self.w_score
                            );
                            self.current_period = GamePeriod::PreSuddenDeath;
                        }
                    }
                    GamePeriod::PreSuddenDeath => {
                        info!("{} Entering sudden death", self.status_string(now));
                        self.current_period = GamePeriod::SuddenDeath;
                        need_cull = true;
                    }
                    GamePeriod::SuddenDeath => {
                        error!(
                            "{} Impossible state: in sudden death with clock counting down",
                            self.status_string(now)
                        )
                    }
                }
                if self.current_period != GamePeriod::BetweenGames {
                    self.clock_state = if self.current_period != GamePeriod::SuddenDeath {
                        ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: self
                                .current_period
                                .duration(&self.config)
                                .unwrap(),
                        }
                    } else {
                        ClockState::CountingUp {
                            start_time: start_time + time_remaining_at_start,
                            time_at_start: Duration::ZERO,
                        }
                    };
                    if need_cull {
                        self.cull_penalties(now)?;
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
                                info!("{} Ending team timeout", self.status_string(now));
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

    pub fn get_start_stop_rx(&self) -> watch::Receiver<bool> {
        self.start_stop_rx.clone()
    }

    // Returns true if the clock was started, false if it was already running
    fn start_game_clock(&mut self, now: Instant) -> bool {
        if let ClockState::Stopped { clock_time } = self.clock_state {
            info!("{} Starting the game clock", self.status_string(now));
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
                info!("{} Stopping the game clock", self.status_string(now));
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
        self.start_stop_tx.send(running).unwrap();
    }

    pub fn start_clock(&mut self, now: Instant) {
        let mut need_to_send = false;
        let status_str = self.status_string(now);
        match &mut self.timeout_state {
            TimeoutState::None => need_to_send = self.start_game_clock(now),
            TimeoutState::Black(ref mut cs) | TimeoutState::White(ref mut cs) => {
                if let ClockState::Stopped { clock_time } = cs {
                    info!("{status_str} Starting the timeout clock");
                    *cs = ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: *clock_time,
                    };
                    need_to_send = true;
                }
            }
            TimeoutState::Ref(ref mut cs) | TimeoutState::PenaltyShot(ref mut cs) => {
                if let ClockState::Stopped { clock_time } = cs {
                    info!("{status_str} Starting the timeout clock");
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
        let status_str = self.status_string(now);
        match &mut self.timeout_state {
            TimeoutState::None => need_to_send = self.stop_game_clock(now)?,
            TimeoutState::Black(ref mut cs) | TimeoutState::White(ref mut cs) => {
                if let ClockState::CountingDown { .. } = cs {
                    info!("{status_str} Stopping the timeout clock");
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
                    info!("{status_str} Stopping the timeout clock");
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

    pub fn halt_clock(&mut self, now: Instant) -> Result<()> {
        if self.timeout_state != TimeoutState::None {
            return Err(TournamentManagerError::AlreadyInTimeout(
                self.timeout_state.as_snapshot(now),
            ));
        }

        if let ClockState::CountingDown {
            start_time,
            time_remaining_at_start,
        } = self.clock_state
        {
            let clock_time = if let Some(time) = self.clock_state.clock_time(now) {
                info!("{} Halting the game clock", self.status_string(now));
                time
            } else {
                let lost_time = now
                    .checked_duration_since(start_time)
                    .ok_or(TournamentManagerError::InvalidNowValue)?
                    .checked_sub(time_remaining_at_start)
                    .unwrap(); // Guaranteed not to panic beacuse `self.clock_state.clock_time(now)` was `None`
                info!(
                    "{} Halting the game clock, lost time: {lost_time:?}",
                    self.status_string(now)
                );

                Duration::from_nanos(1)
            };

            self.clock_state = ClockState::Stopped { clock_time };
            self.send_clock_running(false);

            Ok(())
        } else {
            Err(TournamentManagerError::InvalidState)
        }
    }

    pub fn start_play_now(&mut self, now: Instant) -> Result<()> {
        if self.timeout_state != TimeoutState::None {
            return Err(TournamentManagerError::AlreadyInTimeout(
                self.timeout_state.as_snapshot(Instant::now()),
            ));
        }

        let was_running = self.clock_is_running();

        let mut need_cull = false;
        match self.current_period {
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath => return Err(TournamentManagerError::AlreadyInPlayPeriod),
            GamePeriod::BetweenGames => {
                self.start_game(now);
            }
            GamePeriod::HalfTime => {
                info!("{} Entering second half", self.status_string(now));
                self.current_period = GamePeriod::SecondHalf;
                self.w_timeouts_used = 0;
                self.b_timeouts_used = 0;
                need_cull = true;
            }
            GamePeriod::PreOvertime => {
                info!("{} Entering overtime first half", self.status_string(now));
                self.current_period = GamePeriod::OvertimeFirstHalf;
                need_cull = true;
            }
            GamePeriod::OvertimeHalfTime => {
                info!("{} Entering ovetime second half", self.status_string(now));
                self.current_period = GamePeriod::OvertimeSecondHalf;
                need_cull = true;
            }
            GamePeriod::PreSuddenDeath => {
                info!("{} Entering sudden death", self.status_string(now));
                self.current_period = GamePeriod::SuddenDeath;
                need_cull = true;
            }
        }
        self.clock_state = match self.current_period {
            p @ GamePeriod::FirstHalf
            | p @ GamePeriod::SecondHalf
            | p @ GamePeriod::OvertimeFirstHalf
            | p @ GamePeriod::OvertimeSecondHalf => ClockState::CountingDown {
                start_time: now,
                time_remaining_at_start: p.duration(&self.config).unwrap(),
            },
            GamePeriod::SuddenDeath => ClockState::CountingUp {
                start_time: now,
                time_at_start: Duration::ZERO,
            },
            _ => unreachable!(),
        };
        if need_cull {
            self.cull_penalties(now)?;
        }

        info!(
            "{} {} manually started by refs",
            self.status_string(now),
            self.current_period
        );

        if !was_running {
            self.send_clock_running(true);
        }

        Ok(())
    }

    pub fn set_game_clock_time(&mut self, clock_time: Duration) -> Result<()> {
        if !self.clock_is_running() {
            let time = clock_time.as_secs_f64();
            info!(
                "Setting Game clock to {:02.0}:{:06.3} ",
                (time / 60.0).floor(),
                time % 60.0
            );
            self.clock_state = ClockState::Stopped { clock_time };
            Ok(())
        } else {
            Err(TournamentManagerError::ClockIsRunning)
        }
    }

    pub fn set_timeout_clock_time(&mut self, clock_time: Duration) -> Result<()> {
        if !self.clock_is_running() {
            let time = clock_time.as_secs_f64();
            info!(
                "Setting Timeout clock to {:02.0}:{:06.3} ",
                (time / 60.0).floor(),
                time % 60.0
            );
            let new_cs = ClockState::Stopped { clock_time };
            match self.timeout_state {
                TimeoutState::Black(ref mut cs)
                | TimeoutState::White(ref mut cs)
                | TimeoutState::Ref(ref mut cs)
                | TimeoutState::PenaltyShot(ref mut cs) => *cs = new_cs,
                TimeoutState::None => {
                    return Err(TournamentManagerError::NotInTimeout);
                }
            };
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
            self.next_scheduled_start = Some(
                time + 2 * self.config.half_play_duration
                    + self.config.half_time_duration
                    + self.config.nominal_break,
            );
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

    pub(crate) fn get_penalties(&self) -> BlackWhiteBundle<Vec<Penalty>> {
        BlackWhiteBundle {
            black: self.b_penalties.clone(),
            white: self.w_penalties.clone(),
        }
    }

    pub(crate) fn printable_penalty_time(&self, pen: &Penalty, now: Instant) -> Option<String> {
        let cur_time = self.game_clock_time(now)?;
        if pen.is_complete(self.current_period, cur_time, &self.config)? {
            return Some("Served".to_string());
        }
        if let Some(time) = pen.time_remaining(self.current_period, cur_time, &self.config) {
            let time = time.as_secs();
            Some(format!("{}:{:02}", time / 60, time % 60))
        } else {
            Some("DSMS".to_string())
        }
    }

    /// Returns `None` if the clock time would be negative, or if `now` is before the start
    /// of the current period
    pub fn game_clock_time(&self, now: Instant) -> Option<Duration> {
        trace!(
            "Getting game clock time with clock state {:?} and now time {now:?}",
            self.clock_state
        );
        self.clock_state.clock_time(now)
    }

    /// Returns `None` if there is no timeout, if the clock time would be negative, or if `now` is
    /// before the start of the current timeout
    pub fn timeout_clock_time(&self, now: Instant) -> Option<Duration> {
        match self.timeout_state {
            TimeoutState::None => None,
            TimeoutState::Black(ref cs)
            | TimeoutState::White(ref cs)
            | TimeoutState::Ref(ref cs)
            | TimeoutState::PenaltyShot(ref cs) => cs.clock_time(now),
        }
    }

    pub fn generate_snapshot(&mut self, now: Instant) -> Option<GameSnapshot> {
        trace!("Generating snapshot");
        let cur_time = self.game_clock_time(now)?;
        trace!("Got current time: {cur_time:?}");
        let secs_in_period = cur_time.as_secs().try_into().ok()?;
        trace!("Got seconds remaining: {secs_in_period}");

        let b_penalties = self
            .b_penalties
            .iter()
            .map(|pen| pen.as_snapshot(self.current_period, cur_time, &self.config))
            .collect::<Option<Vec<_>>>()?;
        trace!("Got black penalties");
        let w_penalties = self
            .w_penalties
            .iter()
            .map(|pen| pen.as_snapshot(self.current_period, cur_time, &self.config))
            .collect::<Option<Vec<_>>>()?;
        trace!("Got white penalties");

        if let Some((_, _, goal_per, goal_time)) = self.recent_goal {
            if (goal_per != self.current_period)
                | (goal_time.saturating_sub(cur_time) > RECENT_GOAL_TIME)
            {
                self.recent_goal = None;
            }
        }

        let next_period_len_secs = self
            .current_period
            .next_period_dur(&self.config)
            .map(|dur| dur.as_secs().try_into().unwrap_or(0));

        Some(GameSnapshot {
            current_period: self.current_period,
            secs_in_period,
            timeout: self.timeout_state.as_snapshot(now),
            b_score: self.b_score,
            w_score: self.w_score,
            b_penalties,
            w_penalties,
            is_old_game: !self.has_reset,
            game_number: self.game_number(),
            next_game_number: self.next_game_number(),
            tournament_id: 0,
            recent_goal: self.recent_goal.map(|(c, n, _, _)| (c, n)),
            next_period_len_secs,
        })
    }

    pub fn next_update_time(&self, now: Instant) -> Option<Instant> {
        match (&self.timeout_state, self.current_period) {
            // cases where the clock is counting up
            (TimeoutState::Ref(cs), _) | (TimeoutState::PenaltyShot(cs), _) => cs
                .clock_time(now)
                .map(|ct| now + Duration::from_nanos(1_000_000_000 - ct.subsec_nanos() as u64)),
            (TimeoutState::None, GamePeriod::SuddenDeath) => self
                .clock_state
                .clock_time(now)
                .map(|ct| now + Duration::from_nanos(1_000_000_000 - ct.subsec_nanos() as u64)),
            // cases where the clock is counting down
            (TimeoutState::Black(cs), _) | (TimeoutState::White(cs), _) => cs
                .clock_time(now)
                .map(|ct| now + Duration::from_nanos(ct.subsec_nanos() as u64)),
            (TimeoutState::None, _) => self
                .clock_state
                .clock_time(now)
                .map(|ct| now + Duration::from_nanos(ct.subsec_nanos() as u64)),
        }
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

        string.push_str(match self.current_period {
            GamePeriod::BetweenGames => "BTWNGMS]",
            GamePeriod::FirstHalf => "FRSTHLF]",
            GamePeriod::HalfTime => "HLFTIME]",
            GamePeriod::SecondHalf => "SCNDHLF]",
            GamePeriod::PreOvertime => "PREOVTM]",
            GamePeriod::OvertimeFirstHalf => "OTFRSTH]",
            GamePeriod::OvertimeHalfTime => "OTHLFTM]",
            GamePeriod::OvertimeSecondHalf => "OTSCNDH]",
            GamePeriod::PreSuddenDeath => "PRESDND]",
            GamePeriod::SuddenDeath => "SUDNDTH]",
        });

        string
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

#[derive(Derivative)]
#[derivative(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyKind {
    #[derivative(Default)]
    OneMinute,
    TwoMinute,
    FiveMinute,
    TotalDismissal,
}

impl PenaltyKind {
    pub(crate) fn as_duration(self) -> Option<Duration> {
        match self {
            Self::OneMinute => Some(Duration::from_secs(60)),
            Self::TwoMinute => Some(Duration::from_secs(120)),
            Self::FiveMinute => Some(Duration::from_secs(300)),
            Self::TotalDismissal => None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Penalty {
    pub(crate) kind: PenaltyKind,
    pub(crate) player_number: u8,
    pub(crate) start_period: GamePeriod,
    pub(crate) start_time: Duration,
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
                        .map(|_| Duration::ZERO)
                }
            }
            Ordering::Greater => {
                let mut elapsed = if self.start_period.penalties_run(config) {
                    self.start_time
                } else {
                    Duration::ZERO
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
        let elapsed = self.time_elapsed(cur_per, cur_time, config);
        let total = self.kind.as_duration();

        if cur_per == GamePeriod::BetweenGames && self.start_period != GamePeriod::BetweenGames {
            // In this case, the game in which the penalty started has completed, and we
            // are counting down to the next game. By definition, any penalties have been
            // served in this situation.
            Some(Duration::ZERO)
        } else {
            // In all other cases we do the normal calculation and return `None` if the
            // penalty is a TD or an error occurred
            Some(total?.checked_sub(elapsed?).unwrap_or(Duration::ZERO))
        }
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
                .map(|rem| rem == Duration::ZERO),
        }
    }

    fn as_snapshot(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> Option<PenaltySnapshot> {
        let time = self.time_remaining(cur_per, cur_time, config).map_or_else(
            || {
                if self.kind == PenaltyKind::TotalDismissal {
                    Some(PenaltyTime::TotalDismissal)
                } else {
                    None
                }
            },
            |dur| Some(PenaltyTime::Seconds(dur.as_secs().try_into().unwrap())),
        )?;
        Some(PenaltySnapshot {
            player_number: self.player_number,
            time,
        })
    }
}

#[derive(Derivative)]
#[derivative(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlackWhiteBundle<T> {
    pub black: T,
    pub white: T,
}

impl<T> Index<Color> for BlackWhiteBundle<T> {
    type Output = T;

    fn index(&self, color: Color) -> &Self::Output {
        match color {
            Color::Black => &self.black,
            Color::White => &self.white,
        }
    }
}

impl<T> IndexMut<Color> for BlackWhiteBundle<T> {
    fn index_mut(&mut self, color: Color) -> &mut Self::Output {
        match color {
            Color::Black => &mut self.black,
            Color::White => &mut self.white,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NextGameInfo {
    pub number: u32,
    pub timing: Option<TimingRules>,
    pub start_time: Option<PrimitiveDateTime>,
}

#[derive(Debug, PartialEq, Error)]
pub enum TournamentManagerError {
    #[error("Can't edit clock time while clock is running")]
    ClockIsRunning,
    #[error("Can't start a {0} during {1}")]
    WrongGamePeriod(TimeoutSnapshot, GamePeriod),
    #[error("The {0} team has no more timeouts to use")]
    TooManyTeamTimeouts(Color),
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
    #[error("`update()` needs to be called before this action can be performed")]
    NeedsUpdate,
    #[error("The `now` value passed is not valid")]
    InvalidNowValue,
    #[error("Can't 'start now' when in a play period")]
    AlreadyInPlayPeriod,
    #[error("Action impossible unless in BetweenGames period")]
    GameInProgress,
    #[error("Too many active penalties, can't limit list to {0} values")]
    TooManyPenalties(usize),
    #[error("No {0} penalty exists at the index {1}")]
    InvalidIndex(Color, usize),
    #[error("Can't halt game from the current state")]
    InvalidState,
    #[error("Next Game Info is needed to perform this action")]
    NoNextGameInfo,
}

pub type Result<T> = std::result::Result<T, TournamentManagerError>;

#[cfg(test)]
mod test {
    use super::TournamentManagerError as TMErr;
    use super::*;
    use std::convert::TryInto;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn initialize() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    // TODO: test correct sending of time start/stop signals

    #[test]
    fn test_clock_start_stop() {
        initialize();
        let config = GameConfig {
            nominal_break: Duration::from_secs(13),
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
        initialize();
        let config = GameConfig {
            nominal_break: Duration::from_secs(13),
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
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(stop), Some(Duration::from_secs(3)));

        tm.set_timeout_state(TimeoutState::White(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(stop), Some(Duration::from_secs(3)));

        tm.set_timeout_state(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(stop), Some(Duration::from_secs(7)));

        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        }));

        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(start), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(start), Some(Duration::from_secs(5)));
        tm.stop_clock(stop).unwrap();
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(stop), Some(Duration::from_secs(18)));
        assert_eq!(tm.timeout_clock_time(stop), Some(Duration::from_secs(7)));
    }

    #[test]
    fn test_between_game_timing() {
        initialize();
        // Total time between starts of games is nominally 32s
        let config = GameConfig {
            half_play_duration: Duration::from_secs(10),
            half_time_duration: Duration::from_secs(3),
            nominal_break: Duration::from_secs(9),
            minimum_break: Duration::from_secs(2),
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let mut now = Instant::now();
        tm.start_clock(now);
        assert_eq!(tm.next_scheduled_start, None);
        tm.start_play_now(now).unwrap();
        assert_eq!(tm.next_scheduled_start, Some(now + Duration::from_secs(32)));

        now += Duration::from_secs(1);
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(1));
        tm.start_clock(now);

        now += Duration::from_secs(2);
        tm.update(now).unwrap();
        // Check that when a game runs short, the between games is lengthened to compensate
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(29)));

        now += Duration::from_secs(30);
        tm.update(now).unwrap();
        assert_eq!(tm.next_scheduled_start, Some(now + Duration::from_secs(31)));

        tm.stop_clock(now).unwrap();
        now += Duration::from_secs(35);
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(1));
        tm.start_clock(now);

        now += Duration::from_secs(2);
        tm.update(now).unwrap();
        // Check that when a game runs long, the between games is shortened to match, down to the
        // minimum break length
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(1)));

        now += Duration::from_secs(10);
        tm.update(now).unwrap();
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(1));
        tm.start_clock(now);

        now += Duration::from_secs(2);
        tm.update(now).unwrap();
        // Check that after falling behind the system tries to catch up
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(14)));
    }

    #[test]
    fn test_reset() {
        initialize();
        let config = GameConfig {
            post_game_duration: Duration::from_secs(4),
            minimum_break: Duration::from_secs(3),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let mut now = Instant::now();

        let b_pen = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 12,
            start_period: GamePeriod::SecondHalf,
            start_time: Duration::from_secs(234),
        };
        let w_pen = Penalty {
            kind: PenaltyKind::TotalDismissal,
            player_number: 3,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(413),
        };

        // Test the internal automatic reset during the BetweenGame Period
        assert_eq!(tm.has_reset, true);
        tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, Duration::from_secs(1));
        tm.start_clock(now);
        now += Duration::from_secs(2);
        tm.update(now).unwrap();
        assert_eq!(tm.has_reset, false);

        tm.b_score = 2;
        tm.w_score = 3;
        tm.b_penalties.push(b_pen.clone());
        tm.w_penalties.push(w_pen.clone());
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(2));
        tm.next_scheduled_start = Some(now + Duration::from_secs(12));
        tm.start_clock(now);

        assert_eq!(tm.b_score, 2);
        assert_eq!(tm.w_score, 3);
        assert_eq!(tm.b_penalties, vec![b_pen.clone()]);
        assert_eq!(tm.w_penalties, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);

        now += Duration::from_secs(1);
        tm.update(now).unwrap();

        assert_eq!(tm.b_score, 2);
        assert_eq!(tm.w_score, 3);
        assert_eq!(tm.b_penalties, vec![b_pen.clone()]);
        assert_eq!(tm.w_penalties, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);

        now += Duration::from_secs(2);
        tm.update(now).unwrap();

        assert_eq!(tm.b_score, 2);
        assert_eq!(tm.w_score, 3);
        assert_eq!(tm.b_penalties, vec![b_pen.clone()]);
        assert_eq!(tm.w_penalties, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);
        // 10s between games, 4s before reset
        assert_eq!(tm.reset_game_time, Duration::from_secs(6));

        now += Duration::from_secs(1);
        tm.update(now).unwrap();

        assert_eq!(tm.b_score, 2);
        assert_eq!(tm.w_score, 3);
        assert_eq!(tm.b_penalties, vec![b_pen.clone()]);
        assert_eq!(tm.w_penalties, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);

        now += Duration::from_secs(5);
        tm.update(now).unwrap();

        assert_eq!(tm.b_score, 0);
        assert_eq!(tm.w_score, 0);
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(tm.w_penalties, vec![]);
        assert_eq!(tm.has_reset, true);

        // Test manual reset by the user
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(5));
        tm.b_score = 2;
        tm.w_score = 3;
        tm.b_penalties.push(b_pen.clone());
        tm.w_penalties.push(w_pen.clone());
        tm.has_reset = false;

        tm.reset_game(now);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(3)));
        assert_eq!(tm.b_score, 0);
        assert_eq!(tm.w_score, 0);
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(tm.w_penalties, vec![]);
        assert_eq!(tm.has_reset, true);

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(5));
        tm.b_score = 2;
        tm.w_score = 3;
        tm.b_penalties.push(b_pen);
        tm.w_penalties.push(w_pen);
        tm.has_reset = false;
        tm.start_clock(now);

        now += Duration::from_secs(1);
        tm.update(now).unwrap();

        tm.reset_game(now);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(3)));
        assert_eq!(tm.b_score, 0);
        assert_eq!(tm.w_score, 0);
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(tm.w_penalties, vec![]);
        assert_eq!(tm.has_reset, true);
    }

    #[test]
    fn test_change_config() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let other_config = GameConfig {
            half_play_duration: Duration::from_secs(40),
            ..config
        };
        let mut tm = TournamentManager::new(config);

        tm.current_period = GamePeriod::FirstHalf;
        assert_eq!(
            tm.set_config(other_config.clone()),
            Err(TMErr::GameInProgress)
        );

        tm.current_period = GamePeriod::BetweenGames;
        assert_eq!(tm.set_config(other_config), Ok(()));
    }

    #[test]
    fn test_can_start_timeouts() {
        initialize();
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
        tm.b_timeouts_used = 1;
        tm.w_timeouts_used = 1;
        assert_eq!(
            tm.can_start_b_timeout(),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::Black))
        );
        assert_eq!(
            tm.can_start_w_timeout(),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::White))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
    }

    #[test]
    fn test_start_timeouts() {
        initialize();
        let config = GameConfig {
            team_timeouts_per_half: 1,
            team_timeout_duration: Duration::from_secs(10),
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
            tm.timeout_state,
            TimeoutState::Black(ClockState::Stopped {
                clock_time: Duration::from_secs(10)
            })
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_w_timeout(start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::White(ClockState::Stopped {
                clock_time: Duration::from_secs(10)
            })
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_ref_timeout(start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::Ref(ClockState::Stopped {
                clock_time: Duration::from_secs(0)
            })
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_penalty_shot(start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::PenaltyShot(ClockState::Stopped {
                clock_time: Duration::from_secs(0)
            })
        );

        // Test starting timeouts with clock running, and test team timeouts ending
        tm.b_timeouts_used = 0;
        tm.w_timeouts_used = 0;
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_b_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::Black(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(7)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            TimeoutState::Black(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.timeout_clock_time(t_o_end), Some(Duration::from_secs(0)));
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        tm.update(after_t_o).unwrap();
        assert_eq!(tm.timeout_state, TimeoutState::None);
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(Duration::from_secs(26)));
        assert_eq!(
            tm.start_b_timeout(t_o_start),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::Black))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_w_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::White(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(7)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            TimeoutState::White(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(10)
            })
        );
        assert_eq!(tm.timeout_clock_time(t_o_end), Some(Duration::from_secs(0)));
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        tm.update(after_t_o).unwrap();
        assert_eq!(tm.timeout_state, TimeoutState::None);
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(Duration::from_secs(26)));
        assert_eq!(
            tm.start_w_timeout(t_o_start),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::White))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_ref_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::Ref(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            TimeoutState::Ref(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(
            tm.timeout_clock_time(t_o_end),
            Some(Duration::from_secs(10))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::None);
        tm.start_clock(start);
        assert_eq!(tm.start_penalty_shot(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            })
        );
        assert_eq!(
            tm.timeout_clock_time(t_o_end),
            Some(Duration::from_secs(10))
        );
    }

    #[test]
    fn test_end_timeouts() {
        initialize();
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
        assert_eq!(tm.timeout_state, TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::White(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::Ref(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, TimeoutState::None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: two_secs,
        }));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, TimeoutState::None);
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
        assert_eq!(tm.timeout_state, TimeoutState::None);
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
        assert_eq!(tm.timeout_state, TimeoutState::None);
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
        assert_eq!(tm.timeout_state, TimeoutState::None);
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
        assert_eq!(tm.timeout_state, TimeoutState::None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);
    }

    #[test]
    fn test_can_switch_timeouts() {
        initialize();
        let config = GameConfig {
            team_timeouts_per_half: 1,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let ten_secs = Duration::from_secs(10);

        tm.b_timeouts_used = 1;
        tm.w_timeouts_used = 1;

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(TimeoutState::Black(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(
            tm.can_switch_to_w_timeout(),
            Err(TMErr::TooManyTeamTimeouts(Color::White))
        );
        tm.set_timeout_state(TimeoutState::White(ClockState::CountingDown {
            start_time: start,
            time_remaining_at_start: ten_secs,
        }));
        assert_eq!(
            tm.can_switch_to_b_timeout(),
            Err(TMErr::TooManyTeamTimeouts(Color::Black))
        );

        tm.b_timeouts_used = 0;
        tm.w_timeouts_used = 0;

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
        initialize();
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
            tm.timeout_state,
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
            tm.timeout_state,
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
            tm.timeout_state,
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
            tm.timeout_state,
            TimeoutState::Ref(ClockState::CountingUp {
                start_time: start,
                time_at_start: ten_secs,
            })
        );
    }

    #[test]
    fn test_start_play_now() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            overtime_allowed: true,
            ot_half_play_duration: Duration::from_secs(300),
            sudden_death_allowed: true,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let mut now = Instant::now();

        let fifteen_secs = Duration::from_secs(15);

        let to_b = TimeoutSnapshot::Black(0);
        let to_w = TimeoutSnapshot::White(0);
        let to_r = TimeoutSnapshot::Ref(0);
        let to_ps = TimeoutSnapshot::PenaltyShot(0);

        tm.set_timeout_state(TimeoutState::Black(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_b)));

        tm.set_timeout_state(TimeoutState::White(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_w)));

        tm.set_timeout_state(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_r)));

        tm.set_timeout_state(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        }));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_ps)));

        tm.set_timeout_state(TimeoutState::None);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(tm.start_play_now(now), Ok(()));
        assert_eq!(tm.current_period, GamePeriod::FirstHalf);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(900)));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInPlayPeriod));

        now += Duration::from_secs(10);
        tm.stop_game_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::HalfTime, fifteen_secs);
        assert_eq!(tm.start_play_now(now), Ok(()));
        assert_eq!(tm.current_period, GamePeriod::SecondHalf);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(900)));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInPlayPeriod));

        now += Duration::from_secs(10);
        tm.stop_game_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::PreOvertime, fifteen_secs);
        assert_eq!(tm.start_play_now(now), Ok(()));
        assert_eq!(tm.current_period, GamePeriod::OvertimeFirstHalf);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(300)));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInPlayPeriod));

        now += Duration::from_secs(10);
        tm.stop_game_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::OvertimeHalfTime, fifteen_secs);
        assert_eq!(tm.start_play_now(now), Ok(()));
        assert_eq!(tm.current_period, GamePeriod::OvertimeSecondHalf);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(300)));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInPlayPeriod));

        now += Duration::from_secs(10);
        tm.stop_game_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::PreSuddenDeath, fifteen_secs);
        assert_eq!(tm.start_play_now(now), Ok(()));
        assert_eq!(tm.current_period, GamePeriod::SuddenDeath);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(0)));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInPlayPeriod));
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

        assert_eq!(tm.current_period, end_period);
        assert_eq!(
            tm.game_clock_time(next_time),
            Some(Duration::from_secs(end_clock_time)),
        );
    }

    #[test]
    fn test_transition_bg_to_fh() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(3),
            ..Default::default()
        };

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, Duration::from_secs(1));
        tm.set_game_start(start);
        tm.start_game_clock(start);
        tm.update(next_time).unwrap();

        assert_eq!(GamePeriod::FirstHalf, tm.current_period);
        assert_eq!(tm.game_clock_time(next_time), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_number(), 1);
        assert_eq!(tm.game_start_time, next_time);
    }

    #[test]
    fn test_transition_bg_to_fh_delayed() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(3),
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
        initialize();
        let config = GameConfig {
            half_time_duration: Duration::from_secs(5),
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
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(6),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            pre_overtime_break: Duration::from_secs(7),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(8),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(5),
            minimum_break: Duration::from_secs(1),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(7),
            minimum_break: Duration::from_secs(5),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(6),
            minimum_break: Duration::from_secs(1),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(1),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            ot_half_play_duration: Duration::from_secs(4),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            ot_half_time_duration: Duration::from_secs(5),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            ot_half_play_duration: Duration::from_secs(7),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(9),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: false,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(1),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: false,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(1),
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(1),
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
        initialize();
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
        initialize();
        let config = GameConfig {
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            nominal_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(1),
            ..Default::default()
        };
        // 2*9 + 2 + 8 = 28 sec from game start to game start

        let start = Instant::now();
        let game_start = start - Duration::from_secs(17);
        let second_time = start + Duration::from_secs(2);
        let third_time = second_time + Duration::from_secs(2);
        let fourth_time = third_time + Duration::from_secs(3);

        let mut tm = TournamentManager::new(config);

        let setup_tm = |tm: &mut TournamentManager| {
            tm.stop_game_clock(fourth_time).unwrap();
            tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(5));
            tm.set_game_start(game_start);
            tm.start_game_clock(start);
            tm.set_scores(2, 2, start);
            tm.update(second_time).unwrap()
        };

        setup_tm(&mut tm);

        assert_eq!(tm.current_period, GamePeriod::SuddenDeath);
        assert_eq!(
            tm.game_clock_time(second_time),
            Some(Duration::from_secs(7))
        );

        setup_tm(&mut tm);

        tm.set_scores(3, 2, third_time);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );

        setup_tm(&mut tm);

        tm.add_b_score(1, third_time);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );

        setup_tm(&mut tm);

        tm.add_w_score(1, third_time);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );
    }

    #[test]
    fn test_penalty_time_elapsed() {
        initialize();
        let all_periods_config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(5),
            half_time_duration: Duration::from_secs(7),
            pre_overtime_break: Duration::from_secs(9),
            ot_half_play_duration: Duration::from_secs(11),
            ot_half_time_duration: Duration::from_secs(13),
            pre_sudden_death_duration: Duration::from_secs(15),
            ..Default::default()
        };
        let sd_only_config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: true,
            ..all_periods_config
        };
        let no_sd_no_ot_config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..all_periods_config
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(5),
            half_time_duration: Duration::from_secs(7),
            pre_overtime_break: Duration::from_secs(9),
            ot_half_play_duration: Duration::from_secs(11),
            ot_half_time_duration: Duration::from_secs(13),
            pre_sudden_death_duration: Duration::from_secs(15),
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
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(5),
                PenaltyKind::OneMinute,
                GamePeriod::BetweenGames,
                Duration::from_secs(10),
                Some(Duration::ZERO),
                "Game Ended",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(5),
                PenaltyKind::TotalDismissal,
                GamePeriod::BetweenGames,
                Duration::from_secs(10),
                Some(Duration::ZERO),
                "Game Ended, TD",
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
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(5),
            half_time_duration: Duration::from_secs(7),
            pre_overtime_break: Duration::from_secs(9),
            ot_half_play_duration: Duration::from_secs(11),
            ot_half_time_duration: Duration::from_secs(13),
            pre_sudden_death_duration: Duration::from_secs(15),
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

    #[test]
    fn test_start_penalty() {
        initialize();
        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(Default::default());

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 2,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }]
        );
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::Black, 3, PenaltyKind::TwoMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::Black, 4, PenaltyKind::FiveMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::Black, 5, PenaltyKind::TotalDismissal, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 6, PenaltyKind::OneMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 7, PenaltyKind::TwoMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 8, PenaltyKind::FiveMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 9, PenaltyKind::TotalDismissal, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![
                Penalty {
                    kind: PenaltyKind::OneMinute,
                    player_number: 2,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(24)
                },
                Penalty {
                    kind: PenaltyKind::TwoMinute,
                    player_number: 3,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
                Penalty {
                    kind: PenaltyKind::FiveMinute,
                    player_number: 4,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
                Penalty {
                    kind: PenaltyKind::TotalDismissal,
                    player_number: 5,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
            ]
        );
        assert_eq!(
            tm.w_penalties,
            vec![
                Penalty {
                    kind: PenaltyKind::OneMinute,
                    player_number: 6,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
                Penalty {
                    kind: PenaltyKind::TwoMinute,
                    player_number: 7,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
                Penalty {
                    kind: PenaltyKind::FiveMinute,
                    player_number: 8,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
                Penalty {
                    kind: PenaltyKind::TotalDismissal,
                    player_number: 9,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22)
                },
            ]
        );
    }

    #[test]
    fn test_delete_penalty() {
        initialize();
        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(Default::default());

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 2,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        assert_eq!(
            tm.delete_penalty(Color::Black, 1,),
            Err(TournamentManagerError::InvalidIndex(Color::Black, 1))
        );
        assert_eq!(
            tm.delete_penalty(Color::White, 0,),
            Err(TournamentManagerError::InvalidIndex(Color::White, 0))
        );
        assert_eq!(
            tm.delete_penalty(Color::White, 1,),
            Err(TournamentManagerError::InvalidIndex(Color::White, 1))
        );
        tm.delete_penalty(Color::Black, 0).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::White, 3, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(
            tm.w_penalties,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 3,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(21)
            }],
        );

        assert_eq!(
            tm.delete_penalty(Color::White, 1,),
            Err(TournamentManagerError::InvalidIndex(Color::White, 1))
        );
        assert_eq!(
            tm.delete_penalty(Color::Black, 0),
            Err(TournamentManagerError::InvalidIndex(Color::Black, 0))
        );
        assert_eq!(
            tm.delete_penalty(Color::Black, 1),
            Err(TournamentManagerError::InvalidIndex(Color::Black, 1))
        );
        tm.delete_penalty(Color::White, 0).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(tm.w_penalties, vec![]);
    }

    #[test]
    fn test_edit_penalty() {
        initialize();
        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(Default::default());

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 2,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        assert_eq!(
            tm.edit_penalty(Color::Black, 1, Color::Black, 2, PenaltyKind::TwoMinute),
            Err(TournamentManagerError::InvalidIndex(Color::Black, 1))
        );
        assert_eq!(
            tm.edit_penalty(Color::White, 0, Color::Black, 2, PenaltyKind::TwoMinute),
            Err(TournamentManagerError::InvalidIndex(Color::White, 0))
        );
        assert_eq!(
            tm.edit_penalty(Color::White, 1, Color::Black, 2, PenaltyKind::TwoMinute),
            Err(TournamentManagerError::InvalidIndex(Color::White, 1))
        );
        tm.edit_penalty(Color::Black, 0, Color::Black, 3, PenaltyKind::TwoMinute)
            .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![Penalty {
                kind: PenaltyKind::TwoMinute,
                player_number: 3,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(Color::Black, 0, Color::Black, 4, PenaltyKind::FiveMinute)
            .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![Penalty {
                kind: PenaltyKind::FiveMinute,
                player_number: 4,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::Black,
            0,
            Color::Black,
            5,
            PenaltyKind::TotalDismissal,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.b_penalties,
            vec![Penalty {
                kind: PenaltyKind::TotalDismissal,
                player_number: 5,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );
        assert_eq!(tm.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::Black,
            0,
            Color::White,
            6,
            PenaltyKind::TotalDismissal,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(
            tm.w_penalties,
            vec![Penalty {
                kind: PenaltyKind::TotalDismissal,
                player_number: 6,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );

        let next_time = next_time + Duration::from_secs(1);
        assert_eq!(
            tm.edit_penalty(Color::White, 1, Color::White, 2, PenaltyKind::TwoMinute),
            Err(TournamentManagerError::InvalidIndex(Color::White, 1))
        );
        assert_eq!(
            tm.edit_penalty(Color::Black, 0, Color::Black, 2, PenaltyKind::TwoMinute),
            Err(TournamentManagerError::InvalidIndex(Color::Black, 0))
        );
        assert_eq!(
            tm.edit_penalty(Color::Black, 1, Color::Black, 2, PenaltyKind::TwoMinute),
            Err(TournamentManagerError::InvalidIndex(Color::Black, 1))
        );
        tm.edit_penalty(Color::White, 0, Color::White, 7, PenaltyKind::FiveMinute)
            .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(
            tm.w_penalties,
            vec![Penalty {
                kind: PenaltyKind::FiveMinute,
                player_number: 7,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(Color::White, 0, Color::White, 8, PenaltyKind::TwoMinute)
            .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(
            tm.w_penalties,
            vec![Penalty {
                kind: PenaltyKind::TwoMinute,
                player_number: 8,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(Color::White, 0, Color::White, 10, PenaltyKind::OneMinute)
            .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.b_penalties, vec![]);
        assert_eq!(
            tm.w_penalties,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 10,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24)
            }],
        );
    }

    #[test]
    fn test_snapshot_penalty() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            ..Default::default()
        };

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![PenaltySnapshot {
                player_number: 2,
                time: PenaltyTime::Seconds(59)
            }]
        );
        assert_eq!(snapshot.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::White, 3, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![PenaltySnapshot {
                player_number: 2,
                time: PenaltyTime::Seconds(57)
            }]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![PenaltySnapshot {
                player_number: 3,
                time: PenaltyTime::Seconds(59)
            }]
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::Black, 4, PenaltyKind::TwoMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 5, PenaltyKind::TwoMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(55)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(119)
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(57)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(119)
                },
            ]
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::Black, 6, PenaltyKind::FiveMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 7, PenaltyKind::FiveMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(53)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(117)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::Seconds(299)
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(55)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(117)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::Seconds(299)
                },
            ]
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::Black, 8, PenaltyKind::TotalDismissal, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 9, PenaltyKind::TotalDismissal, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(51)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(115)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::Seconds(297)
                },
                PenaltySnapshot {
                    player_number: 8,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(53)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(115)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::Seconds(297)
                },
                PenaltySnapshot {
                    player_number: 9,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );

        // Check 5 seconds after Half Time has started (there were 15s remaining in first half)
        let next_time = next_time + Duration::from_secs(20);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(36)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(100)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::Seconds(282)
                },
                PenaltySnapshot {
                    player_number: 8,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(38)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(100)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::Seconds(282)
                },
                PenaltySnapshot {
                    player_number: 9,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );

        // Check 10 seconds after Second Half has started (there were 175s remaining in Half Time)
        let next_time = next_time + Duration::from_secs(185);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(26)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(90)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::Seconds(272)
                },
                PenaltySnapshot {
                    player_number: 8,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(28)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(90)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::Seconds(272)
                },
                PenaltySnapshot {
                    player_number: 9,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );

        // Check after the first two penalties have finished
        let next_time = next_time + Duration::from_secs(30);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(60)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::Seconds(242)
                },
                PenaltySnapshot {
                    player_number: 8,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(60)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::Seconds(242)
                },
                PenaltySnapshot {
                    player_number: 9,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );

        // Check after all the penalties have finished
        let next_time = next_time + Duration::from_secs(250);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 8,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 9,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
    }

    #[test]
    fn test_snapshot_penalty_new_game() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(config);

        tm.b_score = 1;
        tm.w_score = 5;
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![PenaltySnapshot {
                player_number: 2,
                time: PenaltyTime::Seconds(59)
            }]
        );
        assert_eq!(snapshot.w_penalties, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(Color::White, 3, PenaltyKind::TwoMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::Black, 5, PenaltyKind::TotalDismissal, next_time)
            .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(57)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::TotalDismissal,
                }
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![PenaltySnapshot {
                player_number: 3,
                time: PenaltyTime::Seconds(119)
            }]
        );

        // Check after the game has ended
        let next_time = next_time + Duration::from_secs(30);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(snapshot.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(0)
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![PenaltySnapshot {
                player_number: 3,
                time: PenaltyTime::Seconds(0)
            },]
        );
    }

    #[test]
    fn test_cull_penalties() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            ..Default::default()
        };

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(71));
        tm.start_game_clock(start);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 3, PenaltyKind::OneMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::Black, 4, PenaltyKind::TwoMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 5, PenaltyKind::TwoMinute, next_time)
            .unwrap();
        tm.start_penalty(Color::Black, 6, PenaltyKind::TotalDismissal, next_time)
            .unwrap();
        tm.start_penalty(Color::White, 7, PenaltyKind::TotalDismissal, next_time)
            .unwrap();

        // Check before culling
        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(59)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(119)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(59)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(119)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );

        // Check during half time (pre-culling)
        let next_time = next_time + Duration::from_secs(75);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(50)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(0)
                },
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(50)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );

        // Check 6s after half time (post-culling)
        let next_time = next_time + Duration::from_secs(180);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.b_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 4,
                    time: PenaltyTime::Seconds(44)
                },
                PenaltySnapshot {
                    player_number: 6,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
        assert_eq!(
            snapshot.w_penalties,
            vec![
                PenaltySnapshot {
                    player_number: 5,
                    time: PenaltyTime::Seconds(44)
                },
                PenaltySnapshot {
                    player_number: 7,
                    time: PenaltyTime::TotalDismissal
                },
            ]
        );
    }

    #[test]
    fn test_limit_penalty_list_length() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut tm = TournamentManager::new(config);

        let mut now = Instant::now();

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(900));
        tm.start_game_clock(now);

        let pen_start = now + Duration::from_secs(30);
        tm.start_penalty(Color::Black, 2, PenaltyKind::OneMinute, pen_start)
            .unwrap();
        tm.start_penalty(Color::White, 3, PenaltyKind::OneMinute, pen_start)
            .unwrap();
        tm.start_penalty(Color::Black, 4, PenaltyKind::TwoMinute, pen_start)
            .unwrap();
        tm.start_penalty(Color::White, 5, PenaltyKind::TwoMinute, pen_start)
            .unwrap();
        tm.start_penalty(Color::Black, 6, PenaltyKind::TotalDismissal, pen_start)
            .unwrap();
        tm.start_penalty(Color::White, 7, PenaltyKind::TotalDismissal, pen_start)
            .unwrap();

        // Check while all penalties are still running, too many
        now += Duration::from_secs(60);
        assert_eq!(
            tm.limit_pen_list_len(Color::Black, 2, now),
            Err(TMErr::TooManyPenalties(2))
        );
        assert_eq!(
            tm.limit_pen_list_len(Color::White, 2, now),
            Err(TMErr::TooManyPenalties(2))
        );
        assert_eq!(tm.b_penalties.len(), 3);
        assert_eq!(tm.w_penalties.len(), 3);

        // Check while two penalties are still running per color
        now += Duration::from_secs(60);
        assert_eq!(tm.limit_pen_list_len(Color::Black, 2, now), Ok(()));
        assert_eq!(tm.limit_pen_list_len(Color::White, 2, now), Ok(()));
        assert_eq!(tm.b_penalties.len(), 2);
        assert_eq!(tm.w_penalties.len(), 2);

        // Check while one penalty is still running per color
        now += Duration::from_secs(60);
        assert_eq!(tm.limit_pen_list_len(Color::Black, 2, now), Ok(()));
        assert_eq!(tm.limit_pen_list_len(Color::White, 2, now), Ok(()));
        assert_eq!(tm.b_penalties.len(), 2);
        assert_eq!(tm.w_penalties.len(), 2);
    }

    #[test]
    fn test_would_end_game() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start_time = Instant::now();
        let next_time = start_time + Duration::from_secs(1);

        tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, Duration::from_nanos(10));
        tm.start_clock(start_time);
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::FirstHalf;
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::HalfTime;
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::SecondHalf;
        assert_eq!(Ok(true), tm.would_end_game(next_time));

        tm.set_scores(3, 4, start_time);
        assert_eq!(Ok(true), tm.would_end_game(next_time));

        tm.config.sudden_death_allowed = true;
        assert_eq!(Ok(true), tm.would_end_game(next_time));

        tm.config.overtime_allowed = true;
        assert_eq!(Ok(true), tm.would_end_game(next_time));

        tm.set_scores(4, 4, start_time);
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::PreOvertime;
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::OvertimeFirstHalf;
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::OvertimeHalfTime;
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.current_period = GamePeriod::OvertimeSecondHalf;
        assert_eq!(Ok(false), tm.would_end_game(next_time));

        tm.config.sudden_death_allowed = false;
        assert_eq!(Ok(true), tm.would_end_game(next_time));

        tm.set_scores(4, 5, start_time);
        tm.config.sudden_death_allowed = true;
        assert_eq!(Ok(true), tm.would_end_game(next_time));

        tm.clock_state = ClockState::CountingUp {
            start_time,
            time_at_start: Duration::ZERO,
        };
        tm.set_scores(4, 5, start_time);
        assert_eq!(Ok(false), tm.would_end_game(next_time));
    }

    #[test]
    fn test_halt_game() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start_time = Instant::now();
        let next_time = start_time + Duration::from_secs(1);

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_millis(500));
        tm.start_clock(start_time);
        tm.halt_clock(next_time).unwrap();
        assert_eq!(
            ClockState::Stopped {
                clock_time: Duration::from_nanos(1)
            },
            tm.clock_state
        );

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_millis(500));
        tm.timeout_state = TimeoutState::Ref(ClockState::CountingUp {
            start_time,
            time_at_start: Duration::ZERO,
        });
        assert_eq!(
            Err(TMErr::AlreadyInTimeout(TimeoutSnapshot::Ref(1))),
            tm.halt_clock(next_time)
        );

        tm.timeout_state = TimeoutState::None;
        assert_eq!(Err(TMErr::InvalidState), tm.halt_clock(next_time));
    }
}
