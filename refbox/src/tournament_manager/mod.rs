use log::*;
use std::{
    cmp::{max, min},
    convert::TryInto,
};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::{
    sync::watch,
    time::{Duration, Instant},
};
use uwh_common::{
    bundles::{BlackWhiteBundle, OptColorBundle},
    color::Color,
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{GamePeriod, GameSnapshot, Infraction, TimeoutSnapshot},
    uwhportal::schedule::{GameNumber, TimingRule},
};

pub mod penalty;
use penalty::*;

pub mod infraction;
use infraction::*;

mod game_stats;
use game_stats::*;

use crate::penalty_editor::IterHelp;

const MAX_TIME_VAL: Duration = Duration::from_secs(MAX_LONG_STRINGABLE_SECS as u64);
const RECENT_GOAL_TIME: Duration = Duration::from_secs(15);

#[derive(Debug)]
pub struct TournamentManager {
    config: GameConfig,
    game_number: GameNumber,
    game_start_time: Instant,
    current_period: GamePeriod,
    clock_state: ClockState,
    timeout_state: Option<TimeoutState>,
    timeouts_used: BlackWhiteBundle<u16>,
    scores: BlackWhiteBundle<u8>,
    penalties: BlackWhiteBundle<Vec<Penalty>>,
    warnings: BlackWhiteBundle<Vec<InfractionDetails>>,
    fouls: OptColorBundle<Vec<InfractionDetails>>,
    has_reset: bool,
    start_stop_tx: watch::Sender<bool>,
    start_stop_rx: watch::Receiver<bool>,
    next_game: Option<NextGameInfo>,
    next_scheduled_start: Option<Instant>,
    reset_game_time: Duration,
    recent_goal: Option<(Color, u8, GamePeriod, Duration)>,
    current_game_stats: GameStats,
    last_game_info: Option<LastGameInfo>,
    time_pause_confirmation: Option<ConfirmPause>,
}

impl TournamentManager {
    pub fn new(config: GameConfig) -> Self {
        let (start_stop_tx, start_stop_rx) = watch::channel(false);
        Self {
            game_number: "0".to_string(),
            game_start_time: Instant::now(),
            current_period: GamePeriod::BetweenGames,
            clock_state: ClockState::Stopped {
                clock_time: config.nominal_break,
            },
            timeout_state: None,
            timeouts_used: Default::default(),
            scores: Default::default(),
            penalties: Default::default(),
            warnings: Default::default(),
            fouls: Default::default(),
            has_reset: true,
            start_stop_tx,
            start_stop_rx,
            next_game: None,
            next_scheduled_start: None,
            reset_game_time: config.nominal_break,
            config,
            recent_goal: None,
            current_game_stats: GameStats::new("0"),
            last_game_info: None,
            time_pause_confirmation: None,
        }
    }

    pub fn clock_is_running(&self) -> bool {
        match &self.timeout_state {
            Some(TimeoutState::Team(_, cs))
            | Some(TimeoutState::Ref(cs))
            | Some(TimeoutState::PenaltyShot(cs))
            | Some(TimeoutState::RugbyPenaltyShot(cs)) => cs.is_running(),
            None => self.clock_state.is_running(),
        }
    }

    pub fn add_score(&mut self, color: Color, player_num: u8, now: Instant) {
        info!(
            "{} Score by {color} player #{player_num}",
            self.status_string(now)
        );
        self.current_game_stats.add_goal(
            self.current_period,
            self.game_clock_time(now),
            color,
            player_num,
            now,
        );
        self.recent_goal = self
            .game_clock_time(now)
            .map(|time| (color, player_num, self.current_period, time));
        let mut scores = self.scores;
        scores[color] += 1;
        self.set_scores(scores, now);
    }

    pub fn get_scores(&self) -> BlackWhiteBundle<u8> {
        self.scores
    }

    pub fn set_scores(&mut self, scores: BlackWhiteBundle<u8>, now: Instant) {
        self.scores = scores;
        info!("{} Scores set to {scores}", self.status_string(now));

        if self.current_period == GamePeriod::SuddenDeath
            && scores.black != scores.white
            && !self.in_score_confirm_pause()
        {
            self.end_game(now);
        }
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

    pub(crate) fn last_game_info(&self) -> Option<&LastGameInfo> {
        self.last_game_info.as_ref()
    }

    pub fn clear_scheduled_game_start(&mut self) {
        self.next_scheduled_start = None;
    }

    pub fn game_number(&self) -> GameNumber {
        self.game_number.clone()
    }

    pub fn next_game_number(&self) -> GameNumber {
        if let Some(ref info) = self.next_game {
            return info.number.clone();
        }

        match self.game_number.parse::<u32>() {
            Ok(num) => (num + 1).to_string(),
            Err(_) => {
                error!(
                    "Failed to parse game_number '{}'. Defaulting to '1' for next game number",
                    self.game_number
                );
                "1".to_string()
            }
        }
    }

    pub fn set_game_number<S: ToString>(&mut self, number: S) {
        self.game_number = number.to_string();
        info!("Game Number set to {}", self.game_number);
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
        self.timeout_state = None;
        self.reset();

        if was_running {
            self.start_game_clock(now);
        }
    }

    fn reset(&mut self) {
        self.scores = Default::default();
        self.penalties.iter_mut().for_each(|(_, p)| p.clear());
        self.warnings.iter_mut().for_each(|(_, w)| w.clear());
        self.fouls.iter_mut().for_each(|(_, f)| f.clear());
        self.current_game_stats = GameStats::new(self.next_game_number());
        self.has_reset = true;
    }

    /// Returns `Ok` if timeout can be started, otherwise returns `Err` describing why not
    pub fn can_start_team_timeout(&self, color: Color) -> Result<()> {
        if let Some(ts @ TimeoutState::Team(timeout_color, _)) = &self.timeout_state {
            if *timeout_color == color {
                return Err(TournamentManagerError::AlreadyInTimeout(
                    ts.as_snapshot(Instant::now()),
                ));
            }
        };
        match self.current_period {
            GamePeriod::FirstHalf | GamePeriod::SecondHalf => {
                if self.timeouts_used[color] < self.config.num_team_timeouts_allowed {
                    Ok(())
                } else {
                    Err(TournamentManagerError::TooManyTeamTimeouts(color))
                }
            }
            _ => Err(TournamentManagerError::WrongGamePeriod(
                match color {
                    Color::White => TimeoutSnapshot::White(0),
                    Color::Black => TimeoutSnapshot::Black(0),
                },
                self.current_period,
            )),
        }
    }

    /// Returns `Ok` if timeout can be started, otherwise returns `Err` describing why not
    pub fn can_start_ref_timeout(&self) -> Result<()> {
        if let Some(ts @ TimeoutState::Ref(_)) = &self.timeout_state {
            Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ))
        } else {
            Ok(())
        }
    }

    /// Returns `Ok` if penalty shot can be started, otherwise returns `Err` describing why not
    pub fn can_start_penalty_shot(&self) -> Result<()> {
        if let Some(ts @ TimeoutState::PenaltyShot(_)) = &self.timeout_state {
            return Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ));
        } else if let Some(ts @ TimeoutState::RugbyPenaltyShot(_)) = &self.timeout_state {
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

    /// Returns `Ok` if penalty shot can be started, otherwise returns `Err` describing why not
    pub fn can_start_rugby_penalty_shot(&self) -> Result<()> {
        if let Some(ts @ TimeoutState::RugbyPenaltyShot(_)) = &self.timeout_state {
            return Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
            ));
        } else if let Some(ts @ TimeoutState::PenaltyShot(_)) = &self.timeout_state {
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
                TimeoutSnapshot::PenaltyShot(self.config.penalty_shot_duration.as_secs() as u16),
                gp,
            )),
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_team_timeout(&self, color: Color) -> Result<()> {
        if let Some(TimeoutState::Team(timeout_color, _)) = &self.timeout_state {
            if color != *timeout_color {
                if self.timeouts_used[color] < self.config.num_team_timeouts_allowed {
                    Ok(())
                } else {
                    Err(TournamentManagerError::TooManyTeamTimeouts(color))
                }
            } else {
                Err(TournamentManagerError::NotInTeamTimeout(color))
            }
        } else {
            Err(TournamentManagerError::NotInTeamTimeout(color))
        }
    }

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_ref_timeout(&self) -> Result<()> {
        if let Some(TimeoutState::PenaltyShot(_)) = &self.timeout_state {
            Ok(())
        } else if let Some(TimeoutState::RugbyPenaltyShot(_)) = &self.timeout_state {
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
                if let Some(TimeoutState::Ref(_)) = &self.timeout_state {
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

    /// Returns `Ok` if timeout type can be switched, otherwise returns `Err` describing why not
    pub fn can_switch_to_rugby_penalty_shot(&self) -> Result<()> {
        match self.current_period {
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath => {
                if let Some(TimeoutState::Ref(_)) = &self.timeout_state {
                    Ok(())
                } else {
                    Err(TournamentManagerError::NotInRefTimeout)
                }
            }
            gp => Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(self.config.penalty_shot_duration.as_secs() as u16),
                gp,
            )),
        }
    }

    pub fn start_team_timeout(&mut self, color: Color, now: Instant) -> Result<()> {
        self.can_start_team_timeout(color)?;
        info!("{} Starting a {color} timeout", self.status_string(now));
        let cs = if self.clock_is_running() {
            self.stop_game_clock(now)?;
            ClockState::CountingDown {
                start_time: now,
                time_remaining_at_start: self.config.team_timeout_duration,
            }
        } else {
            ClockState::Stopped {
                clock_time: self.config.team_timeout_duration,
            }
        };
        self.timeout_state = Some(TimeoutState::Team(color, cs));
        self.timeouts_used[color] += 1;
        Ok(())
    }

    pub fn start_ref_timeout(&mut self, now: Instant) -> Result<()> {
        self.can_start_ref_timeout()?;
        info!("{} Starting a ref timeout", self.status_string(now));
        if self.clock_is_running() {
            self.stop_game_clock(now)?;
            self.timeout_state = Some(TimeoutState::Ref(ClockState::CountingUp {
                start_time: now,
                time_at_start: Duration::ZERO,
            }));
        } else {
            self.timeout_state = Some(TimeoutState::Ref(ClockState::Stopped {
                clock_time: Duration::ZERO,
            }));
        }
        Ok(())
    }

    pub fn start_penalty_shot(&mut self, now: Instant) -> Result<()> {
        self.can_start_penalty_shot()?;
        info!("{} Starting a penalty shot", self.status_string(now));
        if self.clock_is_running() {
            self.stop_game_clock(now)?;
            self.timeout_state = Some(TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: now,
                time_at_start: Duration::ZERO,
            }));
        } else {
            self.timeout_state = Some(TimeoutState::PenaltyShot(ClockState::Stopped {
                clock_time: Duration::ZERO,
            }));
        }
        Ok(())
    }

    pub fn start_rugby_penalty_shot(&mut self, now: Instant) -> Result<()> {
        self.can_start_rugby_penalty_shot()?;
        info!("{} Starting a rugby penalty shot", self.status_string(now));
        if self.clock_is_running() {
            self.timeout_state = Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                start_time: now,
                time_remaining_at_start: self.config.penalty_shot_duration,
            }));
        } else {
            self.timeout_state = Some(TimeoutState::RugbyPenaltyShot(ClockState::Stopped {
                clock_time: self.config.penalty_shot_duration,
            }));
        }
        Ok(())
    }

    pub fn switch_to_team_timeout(&mut self, new_color: Color) -> Result<()> {
        self.can_switch_to_team_timeout(new_color)?;
        info!("Switching to a {new_color} timeout");
        if let Some(TimeoutState::Team(color, _)) = &mut self.timeout_state {
            *color = new_color;
        }
        self.timeouts_used[new_color] += 1;
        self.timeouts_used[new_color.other()] =
            self.timeouts_used[new_color.other()].saturating_sub(1);
        Ok(())
    }

    pub fn switch_to_ref_timeout(&mut self, now: Instant) -> Result<()> {
        self.can_switch_to_ref_timeout()?;
        info!("Switching to a ref timeout");
        if let Some(TimeoutState::PenaltyShot(cs)) = &self.timeout_state {
            self.timeout_state = Some(TimeoutState::Ref(cs.clone()));
        } else if let Some(TimeoutState::RugbyPenaltyShot(_)) = &self.timeout_state {
            self.timeout_state = Some(TimeoutState::Ref(ClockState::CountingUp {
                start_time: now,
                time_at_start: Duration::ZERO,
            }));
        }
        Ok(())
    }

    pub fn switch_to_penalty_shot(&mut self) -> Result<()> {
        self.can_switch_to_penalty_shot()?;
        info!("Switching to a penalty shot");
        if let Some(TimeoutState::Ref(cs)) = &self.timeout_state {
            self.timeout_state = Some(TimeoutState::PenaltyShot(cs.clone()));
        }
        Ok(())
    }

    pub fn switch_to_rugby_penalty_shot(&mut self, now: Instant) -> Result<()> {
        self.can_switch_to_rugby_penalty_shot()?;
        info!("Switching to a rugby penalty shot");
        if let Some(TimeoutState::Ref(cs)) = &self.timeout_state {
            let new_cs = match cs {
                ClockState::Stopped { .. } => ClockState::Stopped {
                    clock_time: self.config.penalty_shot_duration,
                },
                ClockState::CountingUp { .. } => ClockState::CountingDown {
                    start_time: now,
                    time_remaining_at_start: self.config.penalty_shot_duration,
                },
                ClockState::CountingDown { .. } => unreachable!(),
            };

            self.timeout_state = Some(TimeoutState::RugbyPenaltyShot(new_cs));
        }
        Ok(())
    }

    pub fn timeout_end_would_end_game(&self, now: Instant) -> Result<bool> {
        if self.could_end_game(now)? {
            return Ok(true);
        } else if let Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown { .. })) =
            self.timeout_state
        {
            if let ClockState::Stopped { clock_time } = self.clock_state {
                return Ok(clock_time.is_zero()
                    && ((self.current_period == GamePeriod::SecondHalf
                        && (self.scores.are_not_equal()
                            || (!self.config.overtime_allowed
                                && !self.config.sudden_death_allowed)))
                        || (self.current_period == GamePeriod::OvertimeSecondHalf
                            && (self.scores.are_not_equal()
                                || !self.config.sudden_death_allowed))));
            } else if let ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } = self.clock_state
            {
                return self.check_time_remaining(now, start_time, time_remaining_at_start);
            }
        };
        Ok(false)
    }

    pub fn end_timeout(&mut self, now: Instant) -> Result<()> {
        match &self.timeout_state {
            None => Err(TournamentManagerError::NotInTimeout),
            Some(TimeoutState::Team(color, cs)) => {
                info!("{} Ending {color} team timeout", self.status_string(now));
                match cs {
                    ClockState::Stopped { .. } => self.timeout_state = None,
                    ClockState::CountingDown { .. } => {
                        self.start_game_clock(now);
                        self.timeout_state = None;
                    }
                    ClockState::CountingUp { .. } => {
                        error!("Invalid timeout state");
                        return Err(TournamentManagerError::InvalidState);
                    }
                };

                Ok(())
            }
            Some(TimeoutState::Ref(cs)) | Some(TimeoutState::PenaltyShot(cs)) => {
                let timeout_time = match cs.clone() {
                    ClockState::Stopped { clock_time } => {
                        self.timeout_state = None;
                        Some(clock_time)
                    }
                    ClockState::CountingUp {
                        start_time,
                        time_at_start,
                    } => {
                        self.start_game_clock(now);
                        self.timeout_state = None;
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
            Some(TimeoutState::RugbyPenaltyShot(cs)) => {
                info!("{} Ending rugby penalty shot", self.status_string(now));
                match cs {
                    ClockState::CountingDown {
                        start_time,
                        time_remaining_at_start,
                    } => self.handle_rugby_pen_shot_end(now, *start_time, *time_remaining_at_start),
                    ClockState::Stopped { .. } => {
                        self.timeout_state = None;
                        Ok(())
                    }
                    ClockState::CountingUp { .. } => unreachable!(),
                }
            }
        }
    }

    pub fn start_penalty(
        &mut self,
        color: Color,
        player_number: u8,
        kind: PenaltyKind,
        now: Instant,
        infraction: Infraction,
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
            start_instant: now,
            infraction,
        };
        self.penalties[color].push(penalty);
        Ok(())
    }

    pub fn delete_penalty(&mut self, color: Color, index: usize) -> Result<()> {
        if self.penalties[color].len() < index + 1 {
            return Err(TournamentManagerError::InvalidPenIndex(color, index));
        }
        let pen = self.penalties[color].remove(index);
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
        new_infraction: Infraction,
    ) -> Result<()> {
        let status_str = self.status_string(Instant::now());
        let penalty = self.penalties[old_color]
            .get_mut(index)
            .ok_or(TournamentManagerError::InvalidPenIndex(old_color, index))?;
        info!(
            "{status_str} Editing {old_color} player #{}'s {:?} penalty: \
            it is now {new_color} player #{new_player_number}'s {new_kind:?} penalty",
            penalty.player_number, penalty.kind
        );

        penalty.player_number = new_player_number;
        penalty.kind = new_kind;
        penalty.infraction = new_infraction;
        if old_color != new_color {
            let penalty = self.penalties[old_color].remove(index);
            self.penalties[new_color].push(penalty);
        }
        Ok(())
    }

    pub fn limit_pen_list_len(&mut self, color: Color, limit: usize, now: Instant) -> Result<()> {
        let time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;
        let period = self.current_period;

        while self.penalties[color].len() > limit {
            let mut index = None;
            'inner: for (i, pen) in self.penalties[color].iter().enumerate() {
                if pen.is_complete(period, time, &self.config)? {
                    index = Some(i);
                    break 'inner;
                }
            }

            if let Some(i) = index {
                let removed = self.penalties[color].remove(i);
                self.current_game_stats.add_penalty(&removed, color);
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

        for color in [Color::Black, Color::White] {
            let keep = self.penalties[color]
                .iter()
                .map(|pen| pen.is_complete(period, time, &self.config).map(|k| !k))
                .collect::<PenaltyResult<Vec<_>>>()?;
            let mut i = 0;
            self.penalties[color].retain(|pen| {
                let k = keep[i];
                i += 1;
                if !k {
                    self.current_game_stats.add_penalty(pen, color);
                }
                k
            });
        }

        Ok(())
    }

    pub(crate) fn get_penalties(&self) -> &BlackWhiteBundle<Vec<Penalty>> {
        &self.penalties
    }

    pub fn add_warning(
        &mut self,
        color: Color,
        player_number: Option<u8>,
        infraction: Infraction,
        now: Instant,
    ) -> Result<()> {
        info!(
            "{} Adding {color} {} warning for {}",
            self.status_string(now),
            print_p_num_warn(player_number),
            infraction.short_name()
        );
        let start_time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;

        let warning = InfractionDetails {
            player_number,
            start_period: self.current_period,
            start_time,
            start_instant: now,
            infraction,
        };
        self.warnings[color].push(warning);
        Ok(())
    }

    pub fn add_foul(
        &mut self,
        color: Option<Color>,
        player_number: Option<u8>,
        infraction: Infraction,
        now: Instant,
    ) -> Result<()> {
        info!(
            "{} Adding {}{} foul for {}",
            self.status_string(now),
            print_color(color),
            print_p_num_foul(player_number),
            infraction.short_name()
        );
        let start_time = self
            .game_clock_time(now)
            .ok_or(TournamentManagerError::InvalidNowValue)?;

        let foul = InfractionDetails {
            player_number,
            start_period: self.current_period,
            start_time,
            start_instant: now,
            infraction,
        };
        self.fouls[color].push(foul);
        Ok(())
    }

    pub fn get_warnings(&self) -> &BlackWhiteBundle<Vec<InfractionDetails>> {
        &self.warnings
    }

    pub fn get_fouls(&self) -> &OptColorBundle<Vec<InfractionDetails>> {
        &self.fouls
    }

    pub fn edit_warning(
        &mut self,
        old_color: Color,
        index: usize,
        new_color: Color,
        new_player_number: Option<u8>,
        new_infraction: Infraction,
    ) -> Result<()> {
        let status_str = self.status_string(Instant::now());
        let warning = self.warnings[old_color]
            .get_mut(index)
            .ok_or(TournamentManagerError::InvalidWarnIndex(old_color, index))?;
        info!(
            "{status_str} Editing {old_color} {} warning for {}: \
            it is now {new_color} {} warning for {}",
            print_p_num_warn(warning.player_number),
            warning.infraction.short_name(),
            print_p_num_warn(new_player_number),
            new_infraction.short_name()
        );

        warning.player_number = new_player_number;
        warning.infraction = new_infraction;

        if old_color != new_color {
            let warning = self.warnings[old_color].remove(index);
            self.warnings[new_color].push(warning);
        }

        Ok(())
    }

    pub fn edit_foul(
        &mut self,
        old_color: Option<Color>,
        index: usize,
        new_color: Option<Color>,
        new_player_number: Option<u8>,
        new_infraction: Infraction,
    ) -> Result<()> {
        let status_str = self.status_string(Instant::now());
        let foul = self.fouls[old_color]
            .get_mut(index)
            .ok_or(TournamentManagerError::InvalidFoulIndex(old_color, index))?;
        info!(
            "{status_str} Editing {}{} foul for {}: \
            it is now {}{} foul for {}",
            print_color(old_color),
            print_p_num_foul(foul.player_number),
            foul.infraction.short_name(),
            print_color(new_color),
            print_p_num_foul(new_player_number),
            new_infraction.short_name()
        );

        foul.player_number = new_player_number;
        foul.infraction = new_infraction;

        if old_color != new_color {
            let foul = self.fouls[old_color].remove(index);
            self.fouls[new_color].push(foul);
        }

        Ok(())
    }

    pub fn delete_warning(&mut self, color: Color, index: usize) -> Result<()> {
        if self.warnings[color].len() < index + 1 {
            return Err(TournamentManagerError::InvalidWarnIndex(color, index));
        }
        let warning = self.warnings[color].remove(index);
        info!(
            "{} Deleting {color} {} warning for {}",
            self.status_string(Instant::now()),
            print_p_num_warn(warning.player_number),
            warning.infraction.short_name()
        );

        Ok(())
    }

    pub fn delete_foul(&mut self, color: Option<Color>, index: usize) -> Result<()> {
        if self.fouls[color].len() < index + 1 {
            return Err(TournamentManagerError::InvalidFoulIndex(color, index));
        }
        let foul = self.fouls[color].remove(index);
        info!(
            "{} Deleting {}{} foul for {}",
            self.status_string(Instant::now()),
            print_color(color),
            print_p_num_foul(foul.player_number),
            foul.infraction.short_name()
        );

        Ok(())
    }

    fn calc_time_to_next_game(&self, now: Instant, from_time: Instant) -> Duration {
        info!("Next game info is: {:?}", self.next_game);
        let scheduled_start =
            if let Some(start_time) = self.next_game.as_ref().and_then(|info| info.start_time) {
                let cur_time = OffsetDateTime::now_utc();
                info!("Current time is: {cur_time}");
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
            "{} Setting between games time based on uwhportal info: {time_remaining_at_start:?}",
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
            "{} Ending game {}. Score is {}",
            self.status_string(now),
            self.game_number,
            self.scores,
        );

        for color in [Color::Black, Color::White] {
            for penalty in self.penalties[color].iter() {
                self.current_game_stats.add_penalty(penalty, color);
            }
        }

        self.current_game_stats.add_end_time(now);
        self.last_game_info = Some(LastGameInfo {
            scores: self.scores,
            stats: self.current_game_stats.clone(),
        });

        let game_end = if let Some(pause_conf) = &self.time_pause_confirmation {
            pause_conf.pause_began
        } else {
            match self.clock_state {
                ClockState::CountingDown {
                    start_time,
                    time_remaining_at_start,
                } => start_time + time_remaining_at_start,
                ClockState::CountingUp { .. } | ClockState::Stopped { .. } => now,
            }
        };

        let time_remaining_at_start = if let Some(pause_conf) = &self.time_pause_confirmation {
            self.calc_time_to_next_game(now, pause_conf.pause_began)
        } else {
            self.calc_time_to_next_game(now, game_end)
        };

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
        self.current_game_stats.add_start_time(start_time);
        self.current_period = GamePeriod::FirstHalf;
        self.game_start_time = start_time;
        self.timeouts_used.black = 0;
        self.timeouts_used.white = 0;
        self.has_reset = false;

        let sched_start = self.next_scheduled_start.unwrap_or(start_time);
        self.next_scheduled_start = Some(
            sched_start
                + 2 * self.config.half_play_duration
                + self.config.half_time_duration
                + self.config.nominal_break,
        );
    }

    pub fn could_end_game(&self, now: Instant) -> Result<bool> {
        if self.time_pause_confirmation.is_some() {
            Ok(false)
        } else {
            if let Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            })) = self.timeout_state
            {
                if !self.check_time_remaining(now, start_time, time_remaining_at_start)? {
                    return Ok(false);
                } else if let ClockState::Stopped { clock_time } = self.clock_state {
                    if clock_time.is_zero() {
                        return Ok(true);
                    }
                }
            };

            if let ClockState::CountingUp { .. } = self.clock_state {
                if (self.current_period == GamePeriod::SuddenDeath) & (self.scores.are_not_equal())
                {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }

            if let ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } = self.clock_state
            {
                self.check_time_remaining(now, start_time, time_remaining_at_start)
            } else {
                if let Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                    start_time,
                    time_remaining_at_start,
                })) = self.timeout_state
                {
                    if !self.check_time_remaining(now, start_time, time_remaining_at_start)? {
                        return Ok(false);
                    } else if let ClockState::Stopped { clock_time } = self.clock_state {
                        if clock_time.is_zero() {
                            return Ok(true);
                        }
                    }
                };

                if let ClockState::CountingDown {
                    start_time,
                    time_remaining_at_start,
                } = self.clock_state
                {
                    self.check_time_remaining(now, start_time, time_remaining_at_start)
                } else {
                    Ok(false)
                }
            }
        }
    }

    fn check_time_remaining(
        &self,
        now: Instant,
        start_time: Instant,
        time_remaining_at_start: Duration,
    ) -> Result<bool> {
        let time = now
            .checked_duration_since(start_time)
            .ok_or(TournamentManagerError::InvalidNowValue)?;

        Ok(time >= time_remaining_at_start
            && (self.current_period == GamePeriod::SecondHalf
                || (self.current_period == GamePeriod::OvertimeSecondHalf)))
    }

    pub fn pause_has_ended(&self, now: Instant) -> bool {
        if let Some(ref pause_conf) = self.time_pause_confirmation {
            let elapsed = now.duration_since(pause_conf.pause_began);
            elapsed >= pause_conf.duration_of_pause
        } else {
            false
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

            // Check if there is a penalty shot that is not finished
            let unfinished_penalty_shot =
                if let Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                    start_time,
                    time_remaining_at_start,
                })) = self.timeout_state
                {
                    let elapsed = now
                        .checked_duration_since(start_time)
                        .ok_or(TournamentManagerError::InvalidNowValue)?;
                    if elapsed < time_remaining_at_start {
                        true
                    } else {
                        self.handle_rugby_pen_shot_end(now, start_time, time_remaining_at_start)?;
                        false
                    }
                } else {
                    false
                };

            if time >= time_remaining_at_start {
                let mut need_cull = false;
                let mut leave_game_clock_running = true;
                match (self.current_period, unfinished_penalty_shot) {
                    (GamePeriod::BetweenGames, _) => {
                        self.start_game(start_time + time_remaining_at_start);
                    }
                    (GamePeriod::FirstHalf, false) => {
                        self.end_first_half(now);
                    }
                    (GamePeriod::FirstHalf, true) => {
                        info!(
                            "{} Extending First Half for unfinished penalty shot",
                            self.status_string(now)
                        );
                        leave_game_clock_running = false;
                    }
                    (GamePeriod::HalfTime, _) => {
                        info!("{} Entering second half", self.status_string(now));
                        self.current_period = GamePeriod::SecondHalf;
                        if self.config.timeouts_counted_per_half {
                            self.timeouts_used.white = 0;
                            self.timeouts_used.black = 0;
                        }
                        need_cull = true;
                    }
                    (GamePeriod::SecondHalf, false) => {
                        self.end_second_half(now);
                    }
                    (GamePeriod::SecondHalf, true) => {
                        info!(
                            "{} Extending Second Half for unfinished penalty shot",
                            self.status_string(now)
                        );
                        leave_game_clock_running = false;
                    }
                    (GamePeriod::PreOvertime, _) => {
                        info!("{} Entering overtime first half", self.status_string(now));
                        self.current_period = GamePeriod::OvertimeFirstHalf;
                        need_cull = true;
                    }
                    (GamePeriod::OvertimeFirstHalf, false) => {
                        self.end_overtime_first_half(now);
                    }
                    (GamePeriod::OvertimeFirstHalf, true) => {
                        info!(
                            "{} Extending Overtime First Half for unfinished penalty shot",
                            self.status_string(now)
                        );
                        leave_game_clock_running = false;
                    }
                    (GamePeriod::OvertimeHalfTime, _) => {
                        info!("{} Entering ovetime second half", self.status_string(now));
                        self.current_period = GamePeriod::OvertimeSecondHalf;
                        need_cull = true;
                    }
                    (GamePeriod::OvertimeSecondHalf, false) => {
                        self.end_overtime_second_half(now);
                    }
                    (GamePeriod::OvertimeSecondHalf, true) => {
                        info!(
                            "{} Extending Overtime Second Half for unfinished penalty shot",
                            self.status_string(now)
                        );
                        leave_game_clock_running = false;
                    }
                    (GamePeriod::PreSuddenDeath, _) => {
                        info!("{} Entering sudden death", self.status_string(now));
                        self.current_period = GamePeriod::SuddenDeath;
                        need_cull = true;
                    }
                    (GamePeriod::SuddenDeath, _) => {
                        error!(
                            "{} Impossible state: in sudden death with clock counting down",
                            self.status_string(now)
                        )
                    }
                }
                if leave_game_clock_running {
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
                } else {
                    self.clock_state = ClockState::Stopped {
                        clock_time: Duration::ZERO,
                    };
                }
            }
        } else if let ClockState::CountingUp { .. } = self.clock_state {
            // In sudden death, check if in socre confirmation pause
            if self.time_pause_confirmation.is_some() & self.pause_has_ended(now) {
                self.end_confirm_pause(now)?;
            }
        } else {
            // We are either in a timeout, sudden death, or stopped clock. Sudden death and
            // stopped clock don't need anything done
            match &self.timeout_state {
                Some(TimeoutState::Team(color, cs)) => match cs {
                    ClockState::CountingDown {
                        start_time,
                        time_remaining_at_start,
                    } => {
                        if now.duration_since(*start_time) >= *time_remaining_at_start {
                            if let ClockState::Stopped { clock_time } = self.clock_state {
                                info!("{} Ending {color} team timeout", self.status_string(now));
                                self.clock_state = ClockState::CountingDown {
                                    start_time: *start_time + *time_remaining_at_start,
                                    time_remaining_at_start: clock_time,
                                }
                            } else {
                                panic!(
                                    "Cannot end {color} team timeout because game clock isn't stopped"
                                );
                            }
                            self.timeout_state = None;
                        }
                    }
                    ClockState::CountingUp { .. } | ClockState::Stopped { .. } => {}
                },
                Some(TimeoutState::RugbyPenaltyShot(cs)) => match cs {
                    ClockState::CountingDown {
                        start_time,
                        time_remaining_at_start,
                    } => {
                        if now.duration_since(*start_time) >= *time_remaining_at_start {
                            self.handle_rugby_pen_shot_end(
                                now,
                                *start_time,
                                *time_remaining_at_start,
                            )?;
                        }
                    }
                    ClockState::CountingUp { .. } | ClockState::Stopped { .. } => (),
                },
                Some(TimeoutState::Ref(_)) | Some(TimeoutState::PenaltyShot(_)) | None => {}
            };
        };

        Ok(())
    }

    fn end_first_half(&mut self, now: Instant) {
        if self.config.single_half {
            if self.scores.are_not_equal()
                || (!self.config.overtime_allowed && !self.config.sudden_death_allowed)
            {
                self.end_game(now);
            } else if self.config.overtime_allowed {
                info!(
                    "{} Entering pre-overtime. Score is {}",
                    self.status_string(now),
                    self.scores
                );
                self.current_period = GamePeriod::PreOvertime;
            } else {
                info!(
                    "{} Entering pre-sudden death. Score is {}",
                    self.status_string(now),
                    self.scores
                );
                self.current_period = GamePeriod::PreSuddenDeath;
            }
        } else {
            info!("{} Entering half time", self.status_string(now));
            self.current_period = GamePeriod::HalfTime;
        }
    }

    fn end_second_half(&mut self, now: Instant) {
        if self.scores.are_not_equal()
            || (!self.config.overtime_allowed && !self.config.sudden_death_allowed)
        {
            self.end_game(now);
        } else if self.config.overtime_allowed {
            info!(
                "{} Entering pre-overtime. Score is {}",
                self.status_string(now),
                self.scores
            );
            self.current_period = GamePeriod::PreOvertime;
        } else {
            info!(
                "{} Entering pre-sudden death. Score is {}",
                self.status_string(now),
                self.scores
            );
            self.current_period = GamePeriod::PreSuddenDeath;
        }
    }

    fn end_overtime_first_half(&mut self, now: Instant) {
        info!("{} Entering overtime half time", self.status_string(now));
        self.current_period = GamePeriod::OvertimeHalfTime;
    }

    fn end_overtime_second_half(&mut self, now: Instant) {
        if self.scores.are_not_equal() || !self.config.sudden_death_allowed {
            self.end_game(now);
        } else {
            info!(
                "{} Entering pre-sudden death. Score is {}",
                self.status_string(now),
                self.scores
            );
            self.current_period = GamePeriod::PreSuddenDeath;
        }
    }

    fn handle_rugby_pen_shot_end(
        &mut self,
        now: Instant,
        start_time: Instant,
        time_remaining_at_start: Duration,
    ) -> Result<()> {
        info!(
            "{} Handling end of rugby penalty shot",
            self.status_string(now)
        );
        if let ClockState::Stopped { clock_time } = self.clock_state {
            if clock_time == Duration::ZERO {
                match self.current_period {
                    GamePeriod::FirstHalf => {
                        self.end_first_half(now);
                    }
                    GamePeriod::SecondHalf => {
                        self.end_second_half(now);
                    }
                    GamePeriod::OvertimeFirstHalf => {
                        self.end_overtime_first_half(now);
                    }
                    GamePeriod::OvertimeSecondHalf => {
                        self.end_overtime_second_half(now);
                    }
                    GamePeriod::SuddenDeath => {
                        error!(
                            "{} Penalty shot ended during sudden death with clock stopped",
                            self.status_string(now)
                        );
                        return Err(TournamentManagerError::InvalidState);
                    }
                    GamePeriod::BetweenGames
                    | GamePeriod::HalfTime
                    | GamePeriod::PreOvertime
                    | GamePeriod::OvertimeHalfTime
                    | GamePeriod::PreSuddenDeath => {
                        error!(
                            "{} Impossible state: penalty shot ended during non-play period",
                            self.status_string(now)
                        );
                        return Err(TournamentManagerError::InvalidState);
                    }
                }
            }
            self.clock_state = ClockState::CountingDown {
                start_time: min(now, start_time + time_remaining_at_start),
                time_remaining_at_start: self.current_period.duration(&self.config).unwrap(),
            }
        }
        self.timeout_state = None;

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
            None => need_to_send = self.start_game_clock(now),
            Some(TimeoutState::Team(_, cs)) => {
                if let ClockState::Stopped { clock_time } = cs {
                    info!("{status_str} Starting the timeout clock");
                    *cs = ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: *clock_time,
                    };
                    need_to_send = true;
                }
            }
            Some(TimeoutState::RugbyPenaltyShot(cs)) => {
                if let ClockState::Stopped { clock_time } = cs {
                    info!("{status_str} Starting the penalty shot clock");
                    *cs = ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: *clock_time,
                    };
                    if !self.start_game_clock(now) {
                        warn!(
                            "{status_str} Starting the penalty shot clock, but the game clock was already running"
                        )
                    }
                    need_to_send = true;
                }
            }
            Some(TimeoutState::Ref(cs)) | Some(TimeoutState::PenaltyShot(cs)) => {
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
            None => need_to_send = self.stop_game_clock(now)?,
            Some(TimeoutState::Team(_, cs)) => {
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
            Some(TimeoutState::RugbyPenaltyShot(cs)) => {
                if let ClockState::CountingDown { .. } = cs {
                    info!("{status_str} Stopping the timeout clock");
                    *cs = ClockState::Stopped {
                        clock_time: cs
                            .clock_time(now)
                            .ok_or(TournamentManagerError::NeedsUpdate)?,
                    };
                    if !self.stop_game_clock(now)? {
                        warn!(
                            "{status_str} Stopping the penalty shot clock, but the game clock was not running"
                        );
                    }
                    need_to_send = true;
                }
            }
            Some(TimeoutState::Ref(cs)) | Some(TimeoutState::PenaltyShot(cs)) => {
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

    pub fn halt_clock(&mut self, now: Instant, mut end_timeout: bool) -> Result<()> {
        if end_timeout {
            self.timeout_state = None;
        }

        match self.timeout_state {
            None => {}
            Some(TimeoutState::RugbyPenaltyShot(_)) => {
                end_timeout = true;
                self.timeout_state = None;
            }
            Some(ref ts @ TimeoutState::Team(_, _))
            | Some(ref ts @ TimeoutState::Ref(_))
            | Some(ref ts @ TimeoutState::PenaltyShot(_)) => {
                return Err(TournamentManagerError::AlreadyInTimeout(
                    ts.as_snapshot(now),
                ));
            }
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
        } else if end_timeout {
            if let ClockState::Stopped { ref mut clock_time } = self.clock_state {
                *clock_time = Duration::from_nanos(1);
                self.send_clock_running(false);
                Ok(())
            } else {
                Err(TournamentManagerError::InvalidState)
            }
        } else {
            Err(TournamentManagerError::InvalidState)
        }
    }

    pub fn start_play_now(&mut self, now: Instant) -> Result<()> {
        if let Some(ref ts) = self.timeout_state {
            return Err(TournamentManagerError::AlreadyInTimeout(
                ts.as_snapshot(Instant::now()),
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
                if self.config.timeouts_counted_per_half {
                    self.timeouts_used.white = 0;
                    self.timeouts_used.black = 0;
                }
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
                "Setting game clock to {:02.0}:{:06.3}",
                (time / 60.0).floor(),
                time % 60.0
            );

            for pen in self
                .penalties
                .black
                .iter_mut()
                .chain(self.penalties.white.iter_mut())
            {
                if (pen.kind != PenaltyKind::TotalDismissal)
                    && (pen.time_remaining(self.current_period, clock_time, &self.config)?
                        > pen.kind.as_duration().unwrap())
                {
                    pen.start_period = self.current_period;
                    pen.start_time = clock_time;
                }
            }

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
                Some(TimeoutState::Team(_, ref mut cs))
                | Some(TimeoutState::Ref(ref mut cs))
                | Some(TimeoutState::PenaltyShot(ref mut cs))
                | Some(TimeoutState::RugbyPenaltyShot(ref mut cs)) => *cs = new_cs,
                None => {
                    return Err(TournamentManagerError::NotInTimeout);
                }
            };
            Ok(())
        } else {
            Err(TournamentManagerError::ClockIsRunning)
        }
    }

    pub fn pause_for_confirm(&mut self, now: Instant) -> Result<()> {
        if self.timeout_state.is_some() {
            return Err(TournamentManagerError::PausingDuringTimeout);
        }
        if !self.clock_state.is_running() {
            return Err(TournamentManagerError::ClockStopped);
        }
        info!("Pausing for Confirmation");
        let pause_inst = match self.clock_state {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => min(start_time + time_remaining_at_start, now),
            ClockState::CountingUp { .. } => now,
            ClockState::Stopped { .. } => unreachable!(),
        };

        let dur_pause = match self.current_period {
            GamePeriod::SecondHalf => {
                if self.config.overtime_allowed {
                    min(self.config.pre_overtime_break, self.config.minimum_break) / 2
                } else if self.config.sudden_death_allowed {
                    min(
                        self.config.pre_sudden_death_duration,
                        self.config.minimum_break,
                    ) / 2
                } else {
                    self.config.minimum_break / 2
                }
            }
            GamePeriod::OvertimeSecondHalf => {
                if self.config.sudden_death_allowed {
                    min(
                        self.config.pre_sudden_death_duration,
                        self.config.minimum_break,
                    ) / 2
                } else {
                    self.config.minimum_break / 2
                }
            }
            GamePeriod::SuddenDeath => self.config.minimum_break / 2,
            _ => {
                unreachable!()
            }
        };

        info!("Current Period: {}", self.current_period);

        let clock_at_pause = match self.current_period {
            GamePeriod::SuddenDeath => self.clock_state.clock_time(now).unwrap(),
            GamePeriod::SecondHalf | GamePeriod::OvertimeSecondHalf => Duration::from_millis(10),
            GamePeriod::BetweenGames
            | GamePeriod::FirstHalf
            | GamePeriod::HalfTime
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::PreSuddenDeath => unreachable!(),
        };

        self.clock_state = ClockState::Stopped {
            clock_time: Duration::from_millis(10),
        };

        self.time_pause_confirmation = Some(ConfirmPause {
            pause_began: pause_inst,
            duration_of_pause: dur_pause,
            clock_time: clock_at_pause,
        });

        Ok(())
    }

    /// Scores must be accurately set before calling this
    pub fn end_confirm_pause(&mut self, now: Instant) -> Result<()> {
        info!(
            "Ending Pause, Pause Duration: {:?}",
            self.time_pause_confirmation
        );
        if let Some(confirm_pause) = &self.time_pause_confirmation {
            let scores = self.scores;
            self.current_period = match self.current_period {
                GamePeriod::SecondHalf => {
                    if scores.are_not_equal() {
                        GamePeriod::BetweenGames
                    } else if self.config.overtime_allowed {
                        GamePeriod::PreOvertime
                    } else if self.config.sudden_death_allowed {
                        GamePeriod::PreSuddenDeath
                    } else {
                        GamePeriod::BetweenGames
                    }
                }
                GamePeriod::OvertimeSecondHalf => {
                    if scores.are_not_equal() {
                        GamePeriod::BetweenGames
                    } else if self.config.sudden_death_allowed {
                        GamePeriod::PreSuddenDeath
                    } else {
                        GamePeriod::BetweenGames
                    }
                }
                GamePeriod::SuddenDeath => {
                    if !scores.are_not_equal() {
                        GamePeriod::SuddenDeath
                    } else {
                        GamePeriod::BetweenGames
                    }
                }
                _ => {
                    unreachable!()
                }
            };

            info!("Current Period: {}", self.current_period);

            let pause_duration = now.duration_since(confirm_pause.pause_began);
            let time_into_sd = confirm_pause.clock_time;

            let next_period_remaining_duration = if self.current_period == GamePeriod::BetweenGames
            {
                self.calc_time_to_next_game(now, confirm_pause.pause_began)
                    .saturating_sub(pause_duration)
            } else {
                self.current_period
                    .duration(&self.config)
                    .map(|d| d.saturating_sub(pause_duration))
                    .unwrap_or(Duration::ZERO)
            };

            if self.current_period == GamePeriod::BetweenGames {
                self.end_game(now)
            } else {
                self.send_clock_running(true);
            }

            info!(
                "Next period remaining duration: {:?}",
                next_period_remaining_duration
            );

            self.clock_state = match self.current_period {
                GamePeriod::PreOvertime | GamePeriod::PreSuddenDeath | GamePeriod::BetweenGames => {
                    ClockState::CountingDown {
                        start_time: now,
                        time_remaining_at_start: next_period_remaining_duration,
                    }
                }
                GamePeriod::SuddenDeath => ClockState::CountingUp {
                    start_time: (now),
                    time_at_start: (time_into_sd),
                },
                _ => unreachable!(),
            };

            self.time_pause_confirmation = None;

            return Ok(());
        }
        Err(TournamentManagerError::NotPaused)
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
            panic!("Can't edit period and remaining time while clock is running");
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
    fn set_timeout_state(&mut self, state: Option<TimeoutState>) {
        if let ClockState::Stopped { .. } = self.clock_state {
            self.timeout_state = state;
        } else {
            panic!("Can't edit timeout state while clock is running");
        }
    }

    pub(crate) fn printable_penalty_time(
        &self,
        pen: &Penalty,
        now: Instant,
    ) -> Option<PenaltyTimePrintable> {
        let cur_time = self.game_clock_time(now)?;
        if pen
            .is_complete(self.current_period, cur_time, &self.config)
            .ok()?
        {
            return Some(PenaltyTimePrintable::Served);
        }
        if let Ok(time) = pen.time_remaining(self.current_period, cur_time, &self.config) {
            let time = time.whole_seconds();
            Some(PenaltyTimePrintable::Remaining(time))
        } else {
            Some(PenaltyTimePrintable::TotalDismissal)
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
            None => None,
            Some(TimeoutState::Team(_, ref cs))
            | Some(TimeoutState::Ref(ref cs))
            | Some(TimeoutState::PenaltyShot(ref cs))
            | Some(TimeoutState::RugbyPenaltyShot(ref cs)) => cs.clock_time(now),
        }
    }

    pub fn in_score_confirm_pause(&self) -> bool {
        self.time_pause_confirmation.is_some()
    }

    pub fn generate_snapshot(&mut self, now: Instant) -> Option<GameSnapshot> {
        trace!("Generating snapshot");
        let cur_time = self.game_clock_time(now)?;
        trace!("Got current time: {cur_time:?}");
        let secs_in_period = cur_time.as_secs().try_into().ok()?;

        trace!("Got seconds remaining: {secs_in_period}");

        let penalties = self
            .penalties
            .iter()
            .map(|(c, pens)| {
                (
                    c,
                    pens.iter()
                        .map(|p| p.as_snapshot(self.current_period, cur_time, &self.config))
                        .collect::<PenaltyResult<Vec<_>>>()
                        .ok(),
                )
            })
            .collect::<BlackWhiteBundle<_>>()
            .complete()?;
        trace!("Got penalties");

        let warnings = self
            .warnings
            .iter()
            .map(|(c, warns)| (c, warns.iter().map(|war| war.as_snapshot()).collect()))
            .collect();
        trace!("Got warnings");

        let fouls = self
            .fouls
            .iter()
            .map(|(c, fouls)| (c, fouls.iter().map(|foul| foul.as_snapshot()).collect()))
            .collect();
        trace!("Got fouls");

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

        let conf_pause_time = self.time_pause_confirmation.as_ref().map(|p| {
            p.duration_of_pause
                .saturating_sub(now.duration_since(p.pause_began))
                .as_secs()
                .try_into()
                .unwrap_or(0)
        });

        Some(GameSnapshot {
            current_period: self.current_period,
            secs_in_period,
            timeout: self.timeout_state.as_ref().map(|t| t.as_snapshot(now)),
            scores: self.scores,
            penalties,
            warnings,
            fouls,
            is_old_game: !self.has_reset,
            game_number: self.game_number(),
            next_game_number: self.next_game_number(),
            event_id: None,
            recent_goal: self.recent_goal.map(|(c, n, _, _)| (c, n)),
            next_period_len_secs,
            conf_pause_time,
        })
    }

    pub fn next_update_time(&self, now: Instant) -> Option<Instant> {
        let now_plus_subsec = |d: Duration| now + Duration::from_nanos(d.subsec_nanos() as u64);
        let now_plus_invert_subsec =
            |d: Duration| now + Duration::from_nanos(1_000_000_000 - d.subsec_nanos() as u64);

        if let Some(ref pause_conf) = self.time_pause_confirmation {
            return now
                .checked_duration_since(pause_conf.pause_began)
                .and_then(|d| pause_conf.duration_of_pause.checked_sub(d))
                .map(now_plus_subsec);
        };

        match (&self.timeout_state, self.current_period) {
            // cases where the clock is counting up
            (Some(TimeoutState::Ref(cs)), _) | (Some(TimeoutState::PenaltyShot(cs)), _) => {
                cs.clock_time(now).map(now_plus_invert_subsec)
            }
            (None, GamePeriod::SuddenDeath) => {
                self.clock_state.clock_time(now).map(now_plus_invert_subsec)
            }
            // cases where the clock is counting down
            (Some(TimeoutState::Team(_, cs)), _) => cs.clock_time(now).map(now_plus_subsec),
            (Some(TimeoutState::RugbyPenaltyShot(cs)), period) => {
                let time_to_pen_update = cs.clock_time(now).map(now_plus_subsec);
                let time_to_period_update = self.clock_state.clock_time(now).map(|ct| {
                    if period == GamePeriod::SuddenDeath {
                        now_plus_invert_subsec(ct)
                    } else {
                        now_plus_subsec(ct)
                    }
                });
                if cs.is_running() && !self.clock_state.is_running() {
                    time_to_pen_update
                } else {
                    match (time_to_period_update, time_to_pen_update) {
                        (Some(period), Some(pen)) => Some(std::cmp::min(period, pen)),
                        _ => time_to_period_update.or(time_to_pen_update),
                    }
                }
            }
            (None, _) => self.clock_state.clock_time(now).map(now_plus_subsec),
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
            .unwrap_or_else(|| Duration::from_secs(u16::MAX.into()))
            .as_secs()
            .try_into()
            .unwrap()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TimeoutState {
    Team(Color, ClockState),
    Ref(ClockState),
    PenaltyShot(ClockState),
    RugbyPenaltyShot(ClockState),
}

impl TimeoutState {
    fn as_snapshot(&self, now: Instant) -> TimeoutSnapshot {
        match self {
            TimeoutState::Team(Color::Black, cs) => TimeoutSnapshot::Black(cs.as_secs_u16(now)),
            TimeoutState::Team(Color::White, cs) => TimeoutSnapshot::White(cs.as_secs_u16(now)),
            TimeoutState::Ref(cs) => TimeoutSnapshot::Ref(cs.as_secs_u16(now)),
            TimeoutState::PenaltyShot(cs) | TimeoutState::RugbyPenaltyShot(cs) => {
                TimeoutSnapshot::PenaltyShot(cs.as_secs_u16(now))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextGameInfo {
    pub number: GameNumber,
    pub timing: Option<TimingRule>,
    pub start_time: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LastGameInfo {
    pub scores: BlackWhiteBundle<u8>,
    pub stats: GameStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmPause {
    pub pause_began: Instant,
    pub duration_of_pause: Duration,
    pub clock_time: Duration,
}

#[derive(Debug, PartialEq, Eq, Error)]
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
    #[error("Can only switch to {0} Timeout from another team Timeout")]
    NotInTeamTimeout(Color),
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
    InvalidPenIndex(Color, usize),
    #[error("No {0} warning exists at the index {1}")]
    InvalidWarnIndex(Color, usize),
    #[error("No {0:?} penalty exists at the index {1}")]
    InvalidFoulIndex(Option<Color>, usize),
    #[error("Can't halt game from the current state")]
    InvalidState,
    #[error("Next Game Info is needed to perform this action")]
    NoNextGameInfo,
    #[error("Penalty error: {0}")]
    PenaltyError(#[from] PenaltyError),
    #[error("Time not paused")]
    NotPaused,
    #[error("Cannot pause during timeout")]
    PausingDuringTimeout,
    #[error("The clock is already stopped")]
    ClockStopped,
}

pub type Result<T> = std::result::Result<T, TournamentManagerError>;

fn print_color(color: Option<Color>) -> &'static str {
    match color {
        Some(Color::Black) => "Black",
        Some(Color::White) => "White",
        None => "Equal",
    }
}

fn print_p_num_warn(num: Option<u8>) -> String {
    if let Some(n) = num {
        format!("player #{n}'s")
    } else {
        "team's".to_string()
    }
}

fn print_p_num_foul(num: Option<u8>) -> String {
    if let Some(n) = num {
        format!(" player #{n}'s")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod test {
    use super::TournamentManagerError as TMErr;
    use super::*;
    use std::convert::TryInto;
    use std::sync::Once;
    use uwh_common::game_snapshot::{PenaltySnapshot, PenaltyTime};

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
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::Stopped {
                clock_time: Duration::from_secs(5),
            },
        )));

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

        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::Stopped {
                clock_time: Duration::from_secs(5),
            },
        )));

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

        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        })));

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

        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        })));

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
            start_instant: now,
            infraction: Infraction::Unknown,
        };
        let w_pen = Penalty {
            kind: PenaltyKind::TotalDismissal,
            player_number: 3,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(413),
            start_instant: now,
            infraction: Infraction::Unknown,
        };

        // Test the internal automatic reset during the BetweenGame Period
        assert_eq!(tm.has_reset, true);
        tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, Duration::from_secs(1));
        tm.start_clock(now);
        now += Duration::from_secs(2);
        tm.update(now).unwrap();
        assert_eq!(tm.has_reset, false);

        tm.scores.black = 2;
        tm.scores.white = 3;
        tm.penalties.black.push(b_pen.clone());
        tm.penalties.white.push(w_pen.clone());
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(2));
        tm.next_scheduled_start = Some(now + Duration::from_secs(12));
        tm.start_clock(now);

        assert_eq!(tm.scores.black, 2);
        assert_eq!(tm.scores.white, 3);
        assert_eq!(tm.penalties.black, vec![b_pen.clone()]);
        assert_eq!(tm.penalties.white, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);

        now += Duration::from_secs(1);
        tm.update(now).unwrap();

        assert_eq!(tm.scores.black, 2);
        assert_eq!(tm.scores.white, 3);
        assert_eq!(tm.penalties.black, vec![b_pen.clone()]);
        assert_eq!(tm.penalties.white, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);

        now += Duration::from_secs(2);
        tm.update(now).unwrap();

        assert_eq!(tm.scores.black, 2);
        assert_eq!(tm.scores.white, 3);
        assert_eq!(tm.penalties.black, vec![b_pen.clone()]);
        assert_eq!(tm.penalties.white, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);
        // 10s between games, 4s before reset
        assert_eq!(tm.reset_game_time, Duration::from_secs(6));

        now += Duration::from_secs(1);
        tm.update(now).unwrap();

        assert_eq!(tm.scores.black, 2);
        assert_eq!(tm.scores.white, 3);
        assert_eq!(tm.penalties.black, vec![b_pen.clone()]);
        assert_eq!(tm.penalties.white, vec![w_pen.clone()]);
        assert_eq!(tm.has_reset, false);

        now += Duration::from_secs(5);
        tm.update(now).unwrap();

        assert_eq!(tm.scores.black, 0);
        assert_eq!(tm.scores.white, 0);
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(tm.penalties.white, vec![]);
        assert_eq!(tm.has_reset, true);

        // Test manual reset by the user
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(5));
        tm.scores.black = 2;
        tm.scores.white = 3;
        tm.penalties.black.push(b_pen.clone());
        tm.penalties.white.push(w_pen.clone());
        tm.has_reset = false;

        tm.reset_game(now);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(3)));
        assert_eq!(tm.scores.black, 0);
        assert_eq!(tm.scores.white, 0);
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(tm.penalties.white, vec![]);
        assert_eq!(tm.has_reset, true);

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(5));
        tm.scores.black = 2;
        tm.scores.white = 3;
        tm.penalties.black.push(b_pen);
        tm.penalties.white.push(w_pen);
        tm.has_reset = false;
        tm.start_clock(now);

        now += Duration::from_secs(1);
        tm.update(now).unwrap();

        tm.reset_game(now);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(3)));
        assert_eq!(tm.scores.black, 0);
        assert_eq!(tm.scores.white, 0);
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(tm.penalties.white, vec![]);
        assert_eq!(tm.has_reset, true);
    }

    #[test]
    fn test_next_update_time() {
        initialize();
        let config = GameConfig {
            penalty_shot_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let now = Instant::now();

        // Case 1: Time pause confirmation is active
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::ZERO);
        tm.start_clock(now);
        tm.time_pause_confirmation = Some(ConfirmPause {
            pause_began: now,
            duration_of_pause: Duration::from_secs(5),
            clock_time: Duration::ZERO,
        });
        assert_eq!(tm.next_update_time(now), Some(now));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_0000)),
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_secs(2))
        );

        // Case 2: TimeoutState::Ref with clock counting up
        tm.time_pause_confirmation = None;
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::ZERO,
        })));
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now + Duration::from_secs(1)));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_secs(2))
        );

        // Case 3: TimeoutState::PenaltyShot with clock counting up
        tm.stop_clock(now).unwrap();
        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::ZERO,
        })));
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now + Duration::from_secs(1)));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_secs(2))
        );

        // Case 4: GamePeriod::SuddenDeath with clock counting up
        tm.set_timeout_state(None);
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::ZERO);
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now + Duration::from_secs(1)));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_secs(2))
        );

        // Case 5: TimeoutState::Team with clock counting down
        tm.stop_clock(now).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(10));
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::Stopped {
                clock_time: Duration::from_secs(5),
            },
        )));
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_secs(2))
        );

        // Case 6: TimeoutState::RugbyPenaltyShot with clock counting down, Rugby penalty shot is cause of update time
        tm.stop_clock(now).unwrap();
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_millis(5_500),
        })));
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_millis(500))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_millis(1_500))
        );

        // Case 7: TimeoutState::RugbyPenaltyShot with clock counting down, Game clock is cause of update time
        tm.stop_clock(now).unwrap();
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(5),
        })));
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_millis(10_500));
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_millis(500))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_millis(1_500))
        );

        // Case 8: ClockState::CountingDown with no timeout
        tm.stop_clock(now).unwrap();
        tm.set_timeout_state(None);
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.start_clock(now);
        assert_eq!(tm.next_update_time(now), Some(now));
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(8_000)),
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            tm.next_update_time(now + Duration::from_nanos(1_000_008_000)),
            Some(now + Duration::from_secs(2))
        );

        // Case 9: ClockState::Stopped
        tm.stop_clock(now).unwrap();
        assert_eq!(tm.next_update_time(now), Some(now));
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
            num_team_timeouts_allowed: 1,
            penalty_shot_duration: Duration::from_secs(45),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let to_b = TimeoutSnapshot::Black(0);
        let to_w = TimeoutSnapshot::White(0);
        let to_r = TimeoutSnapshot::Ref(0);
        let to_ps = TimeoutSnapshot::PenaltyShot(0);
        let to_rps = TimeoutSnapshot::PenaltyShot(45);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(tm.can_start_team_timeout(Color::White), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(10));
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(tm.can_start_team_timeout(Color::White), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        let otfh = GamePeriod::OvertimeFirstHalf;
        tm.set_period_and_game_clock_time(otfh, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TMErr::WrongGamePeriod(to_b, otfh))
        );
        assert_eq!(
            tm.can_start_team_timeout(Color::White),
            Err(TMErr::WrongGamePeriod(to_w, otfh))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        let otsh = GamePeriod::OvertimeSecondHalf;
        tm.set_period_and_game_clock_time(otsh, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TMErr::WrongGamePeriod(to_b, otsh))
        );
        assert_eq!(
            tm.can_start_team_timeout(Color::White),
            Err(TournamentManagerError::WrongGamePeriod(to_w, otsh))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        let otsd = GamePeriod::SuddenDeath;
        tm.set_period_and_game_clock_time(otsd, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TournamentManagerError::WrongGamePeriod(to_b, otsd))
        );
        assert_eq!(
            tm.can_start_team_timeout(Color::White),
            Err(TournamentManagerError::WrongGamePeriod(to_w, otsd))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        let ht = GamePeriod::HalfTime;
        tm.set_period_and_game_clock_time(ht, Duration::from_secs(10));
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TournamentManagerError::WrongGamePeriod(to_b, ht))
        );
        assert_eq!(
            tm.can_start_team_timeout(Color::White),
            Err(TournamentManagerError::WrongGamePeriod(to_w, ht))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(
            tm.can_start_penalty_shot(),
            Err(TournamentManagerError::WrongGamePeriod(to_ps, ht))
        );
        assert_eq!(
            tm.can_start_rugby_penalty_shot(),
            Err(TournamentManagerError::WrongGamePeriod(to_rps, ht))
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::Stopped {
                clock_time: Duration::from_secs(0),
            },
        )));
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TournamentManagerError::AlreadyInTimeout(to_b))
        );
        assert_eq!(tm.can_start_team_timeout(Color::White), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::Stopped {
                clock_time: Duration::from_secs(0),
            },
        )));
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(
            tm.can_start_team_timeout(Color::White),
            Err(TournamentManagerError::AlreadyInTimeout(to_w))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        })));
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(tm.can_start_team_timeout(Color::White), Ok(()));
        assert_eq!(
            tm.can_start_ref_timeout(),
            Err(TournamentManagerError::AlreadyInTimeout(to_r))
        );
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));

        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        })));
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(tm.can_start_team_timeout(Color::White), Ok(()));
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(
            tm.can_start_penalty_shot(),
            Err(TournamentManagerError::AlreadyInTimeout(to_ps))
        );
        assert_eq!(
            tm.can_start_rugby_penalty_shot(),
            Err(TournamentManagerError::AlreadyInTimeout(to_ps))
        );

        tm.set_timeout_state(None);
        tm.timeouts_used.black = 1;
        tm.timeouts_used.white = 1;
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::Black))
        );
        assert_eq!(
            tm.can_start_team_timeout(Color::White),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::White))
        );
        assert_eq!(tm.can_start_ref_timeout(), Ok(()));
        assert_eq!(tm.can_start_penalty_shot(), Ok(()));
        assert_eq!(tm.can_start_rugby_penalty_shot(), Ok(()));
    }

    #[test]
    fn test_start_timeouts() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 1,
            team_timeout_duration: Duration::from_secs(10),
            penalty_shot_duration: Duration::from_secs(25),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let t_o_start = start + Duration::from_secs(2);
        let mid_t_o = t_o_start + Duration::from_secs(3);
        let t_o_end = t_o_start + Duration::from_secs(10);
        let r_ps_end = t_o_start + Duration::from_secs(25);
        let after_t_o = t_o_end + Duration::from_secs(2);
        let after_r_ps = r_ps_end + Duration::from_secs(2);

        // Test starting timeouts with the clock stopped
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_team_timeout(Color::Black, start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::Black,
                ClockState::Stopped {
                    clock_time: Duration::from_secs(10)
                }
            ))
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_team_timeout(Color::White, start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::White,
                ClockState::Stopped {
                    clock_time: Duration::from_secs(10)
                }
            ))
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_ref_timeout(start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Ref(ClockState::Stopped {
                clock_time: Duration::from_secs(0)
            }))
        );

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_penalty_shot(start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::PenaltyShot(ClockState::Stopped {
                clock_time: Duration::from_secs(0)
            }))
        );

        tm.end_timeout(start).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        assert_eq!(tm.start_rugby_penalty_shot(start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::RugbyPenaltyShot(ClockState::Stopped {
                clock_time: Duration::from_secs(25)
            }))
        );

        // Test starting timeouts with clock running, and test team timeouts ending
        tm.timeouts_used.black = 0;
        tm.timeouts_used.white = 0;
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(None);
        tm.start_clock(start);
        assert_eq!(tm.start_team_timeout(Color::Black, t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::Black,
                ClockState::CountingDown {
                    start_time: t_o_start,
                    time_remaining_at_start: Duration::from_secs(10)
                }
            ))
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(7)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::Black,
                ClockState::CountingDown {
                    start_time: t_o_start,
                    time_remaining_at_start: Duration::from_secs(10)
                }
            ))
        );
        assert_eq!(tm.timeout_clock_time(t_o_end), Some(Duration::from_secs(0)));
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        tm.update(after_t_o).unwrap();
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(Duration::from_secs(26)));
        assert_eq!(
            tm.start_team_timeout(Color::Black, t_o_start),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::Black))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(None);
        tm.start_clock(start);
        assert_eq!(tm.start_team_timeout(Color::White, t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::White,
                ClockState::CountingDown {
                    start_time: t_o_start,
                    time_remaining_at_start: Duration::from_secs(10)
                }
            ))
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(7)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::White,
                ClockState::CountingDown {
                    start_time: t_o_start,
                    time_remaining_at_start: Duration::from_secs(10)
                }
            ))
        );
        assert_eq!(tm.timeout_clock_time(t_o_end), Some(Duration::from_secs(0)));
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        tm.update(after_t_o).unwrap();
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.timeout_clock_time(after_t_o), None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(Duration::from_secs(26)));
        assert_eq!(
            tm.start_team_timeout(Color::White, t_o_start),
            Err(TournamentManagerError::TooManyTeamTimeouts(Color::White))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(None);
        tm.start_clock(start);
        assert_eq!(tm.start_ref_timeout(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Ref(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            }))
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Ref(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            }))
        );
        assert_eq!(
            tm.timeout_clock_time(t_o_end),
            Some(Duration::from_secs(10))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(None);
        tm.start_clock(start);
        assert_eq!(tm.start_penalty_shot(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            }))
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(tm.timeout_clock_time(mid_t_o), Some(Duration::from_secs(3)));
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(28)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: t_o_start,
                time_at_start: Duration::from_secs(0)
            }))
        );
        assert_eq!(
            tm.timeout_clock_time(t_o_end),
            Some(Duration::from_secs(10))
        );

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(None);
        tm.start_clock(start);
        assert_eq!(tm.start_rugby_penalty_shot(t_o_start), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(25)
            }))
        );
        assert_eq!(tm.game_clock_time(t_o_start), Some(Duration::from_secs(28)));
        assert_eq!(
            tm.timeout_clock_time(mid_t_o),
            Some(Duration::from_secs(22))
        );
        assert_eq!(tm.game_clock_time(mid_t_o), Some(Duration::from_secs(25)));
        tm.update(mid_t_o).unwrap();
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: Duration::from_secs(25)
            }))
        );
        assert_eq!(
            tm.timeout_clock_time(r_ps_end),
            Some(Duration::from_secs(0))
        );
        assert_eq!(tm.timeout_clock_time(after_r_ps), None);
        tm.update(after_r_ps).unwrap();
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.timeout_clock_time(after_r_ps), None);
        assert_eq!(tm.game_clock_time(after_r_ps), Some(Duration::from_secs(1)));
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
        let fifteen_secs = Duration::from_secs(15);
        let twenty_secs = Duration::from_secs(20);
        let thirty_secs = Duration::from_secs(30);

        // Test ending timeouts with the clock stopped
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        assert_eq!(tm.end_timeout(t_o_end), Err(TMErr::NotInTimeout));
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::Stopped {
                clock_time: two_secs,
            },
        )));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::Stopped {
                clock_time: two_secs,
            },
        )));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::Stopped {
            clock_time: two_secs,
        })));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: two_secs,
        })));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(ClockState::Stopped {
            clock_time: two_secs,
        })));
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(t_o_end), Some(thirty_secs));
        assert_eq!(tm.clock_is_running(), false);

        // Test ending timeouts with the clock running
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::CountingDown {
                start_time: t_o_start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::CountingUp {
            start_time: t_o_start,
            time_at_start: ten_secs,
        })));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::CountingUp {
            start_time: t_o_start,
            time_at_start: ten_secs,
        })));
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(twenty_secs));
        assert_eq!(tm.clock_is_running(), true);

        tm.stop_clock(after_t_o).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, thirty_secs);
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(ClockState::Stopped {
            clock_time: ten_secs,
        })));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.end_timeout(t_o_end), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.game_clock_time(after_t_o), Some(fifteen_secs));
        assert_eq!(tm.clock_is_running(), true);
    }

    #[test]
    fn test_can_switch_timeouts() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 1,
            penalty_shot_duration: Duration::from_secs(45),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let ten_secs = Duration::from_secs(10);

        tm.timeouts_used.black = 1;
        tm.timeouts_used.white = 1;

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::White),
            Err(TMErr::TooManyTeamTimeouts(Color::White))
        );
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::Black),
            Err(TMErr::TooManyTeamTimeouts(Color::Black))
        );

        tm.timeouts_used.black = 0;
        tm.timeouts_used.white = 0;

        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(tm.can_switch_to_team_timeout(Color::White), Ok(()));
        assert_eq!(tm.can_switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.can_switch_to_rugby_penalty_shot(),
            Err(TMErr::NotInRefTimeout)
        );

        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(tm.can_switch_to_team_timeout(Color::Black), Ok(()));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(tm.can_switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.can_switch_to_rugby_penalty_shot(),
            Err(TMErr::NotInRefTimeout)
        );

        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        })));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(tm.can_switch_to_ref_timeout(), Err(TMErr::NotInPenaltyShot));
        assert_eq!(tm.can_switch_to_penalty_shot(), Ok(()));
        assert_eq!(tm.can_switch_to_rugby_penalty_shot(), Ok(()));

        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        })));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(tm.can_switch_to_ref_timeout(), Ok(()));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.can_switch_to_rugby_penalty_shot(),
            Err(TMErr::NotInRefTimeout)
        );

        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.can_switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(tm.can_switch_to_ref_timeout(), Ok(()));
        assert_eq!(tm.can_switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.can_switch_to_rugby_penalty_shot(),
            Err(TMErr::NotInRefTimeout)
        );

        tm.set_period_and_game_clock_time(GamePeriod::HalfTime, Duration::from_secs(30));
        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        })));
        assert_eq!(
            tm.can_switch_to_penalty_shot(),
            Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(0),
                GamePeriod::HalfTime
            ))
        );
        assert_eq!(
            tm.can_switch_to_rugby_penalty_shot(),
            Err(TournamentManagerError::WrongGamePeriod(
                TimeoutSnapshot::PenaltyShot(45),
                GamePeriod::HalfTime
            ))
        );
    }

    #[test]
    fn test_switch_timeouts() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 1,
            penalty_shot_duration: Duration::from_secs(25),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let later = start + Duration::from_secs(2);
        let ten_secs = Duration::from_secs(10);
        let twenty_five_seconds = Duration::from_secs(25);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(30));
        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::CountingDown {
                start_time: start,
                time_remaining_at_start: ten_secs,
            },
        )));
        assert_eq!(
            tm.switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.switch_to_ref_timeout(later),
            Err(TMErr::NotInPenaltyShot)
        );
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.switch_to_rugby_penalty_shot(later),
            Err(TMErr::NotInRefTimeout)
        );
        assert_eq!(tm.switch_to_team_timeout(Color::White), Ok(()));

        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::White,
                ClockState::CountingDown {
                    start_time: start,
                    time_remaining_at_start: ten_secs,
                }
            ))
        );
        assert_eq!(
            tm.switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(
            tm.switch_to_ref_timeout(later),
            Err(TMErr::NotInPenaltyShot)
        );
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.switch_to_rugby_penalty_shot(later),
            Err(TMErr::NotInRefTimeout)
        );
        assert_eq!(tm.switch_to_team_timeout(Color::Black), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Team(
                Color::Black,
                ClockState::CountingDown {
                    start_time: start,
                    time_remaining_at_start: ten_secs,
                }
            ))
        );

        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        })));
        assert_eq!(
            tm.switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(
            tm.switch_to_ref_timeout(later),
            Err(TMErr::NotInPenaltyShot)
        );
        assert_eq!(tm.switch_to_penalty_shot(), Ok(()));

        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::PenaltyShot(ClockState::CountingUp {
                start_time: start,
                time_at_start: ten_secs,
            }))
        );
        assert_eq!(
            tm.switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.switch_to_rugby_penalty_shot(later),
            Err(TMErr::NotInRefTimeout)
        );
        assert_eq!(tm.switch_to_ref_timeout(later), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Ref(ClockState::CountingUp {
                start_time: start,
                time_at_start: ten_secs,
            }))
        );

        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::CountingUp {
            start_time: start,
            time_at_start: ten_secs,
        })));
        assert_eq!(tm.switch_to_rugby_penalty_shot(later), Ok(()));

        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
                start_time: later,
                time_remaining_at_start: twenty_five_seconds,
            }))
        );
        assert_eq!(
            tm.switch_to_team_timeout(Color::Black),
            Err(TMErr::NotInTeamTimeout(Color::Black))
        );
        assert_eq!(
            tm.switch_to_team_timeout(Color::White),
            Err(TMErr::NotInTeamTimeout(Color::White))
        );
        assert_eq!(tm.switch_to_penalty_shot(), Err(TMErr::NotInRefTimeout));
        assert_eq!(
            tm.switch_to_rugby_penalty_shot(later),
            Err(TMErr::NotInRefTimeout)
        );
        assert_eq!(tm.switch_to_ref_timeout(later), Ok(()));
        assert_eq!(
            tm.timeout_state,
            Some(TimeoutState::Ref(ClockState::CountingUp {
                start_time: later,
                time_at_start: Duration::ZERO,
            }))
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

        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::Black,
            ClockState::Stopped {
                clock_time: Duration::from_secs(0),
            },
        )));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_b)));

        tm.set_timeout_state(Some(TimeoutState::Team(
            Color::White,
            ClockState::Stopped {
                clock_time: Duration::from_secs(0),
            },
        )));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_w)));

        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        })));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_r)));

        tm.set_timeout_state(Some(TimeoutState::PenaltyShot(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        })));
        assert_eq!(tm.start_play_now(now), Err(TMErr::AlreadyInTimeout(to_ps)));

        tm.set_timeout_state(None);
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
        score: Option<BlackWhiteBundle<u8>>,
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
        if let Some(scores) = score {
            tm.set_scores(scores, start);
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
        assert_eq!(tm.game_number(), "1");
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
            score: Some(BlackWhiteBundle { black: 1, white: 1 }),
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
            score: Some(BlackWhiteBundle { black: 1, white: 1 }),
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
            score: Some(BlackWhiteBundle { black: 1, white: 1 }),
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
            score: Some(BlackWhiteBundle { black: 1, white: 1 }),
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
            score: Some(BlackWhiteBundle { black: 2, white: 4 }),
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
            score: Some(BlackWhiteBundle { black: 3, white: 2 }),
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
            score: Some(BlackWhiteBundle { black: 1, white: 1 }),
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
            score: Some(BlackWhiteBundle { black: 1, white: 1 }),
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
            score: Some(BlackWhiteBundle {
                black: 10,
                white: 1,
            }),
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
            score: Some(BlackWhiteBundle {
                black: 11,
                white: 9,
            }),
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
            tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);
            tm.update(second_time).unwrap()
        };

        setup_tm(&mut tm);

        assert_eq!(tm.current_period, GamePeriod::SuddenDeath);
        assert_eq!(
            tm.game_clock_time(second_time),
            Some(Duration::from_secs(7))
        );

        setup_tm(&mut tm);

        tm.set_scores(BlackWhiteBundle { black: 3, white: 2 }, third_time);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );

        setup_tm(&mut tm);

        tm.add_score(Color::Black, 1, third_time);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );

        setup_tm(&mut tm);

        tm.add_score(Color::White, 1, third_time);
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            tm.game_clock_time(fourth_time),
            Some(Duration::from_secs(4))
        );
    }

    // Test setup with rugby penalties that are incomplete when the period ends
    struct PenaltyTransitionTestSetup {
        config: GameConfig,
        game_start_offset: i64,
        start_period: GamePeriod,
        remaining: u64,
        start_penalty_after: u64,
        first_check: u64,
        second_check: u64,
        end_period: GamePeriod,
        end_clock_time: u64,
    }

    fn test_penalty_transition(setup: PenaltyTransitionTestSetup) {
        let PenaltyTransitionTestSetup {
            config,
            game_start_offset,
            start_period,
            remaining,
            start_penalty_after,
            first_check,
            second_check,
            end_period,
            end_clock_time,
        } = setup;

        let start = Instant::now();
        let start_penalty_at = start + Duration::from_secs(start_penalty_after);
        let first_time = start + Duration::from_secs(first_check);
        let second_time = start + Duration::from_secs(second_check);
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
        tm.start_rugby_penalty_shot(start_penalty_at).unwrap();
        tm.update(first_time).unwrap();

        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.current_period, start_period);
        assert_eq!(tm.game_clock_time(first_time), Some(Duration::ZERO));

        tm.update(second_time).unwrap();

        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.current_period, end_period);
        assert_eq!(
            tm.game_clock_time(second_time),
            Some(Duration::from_secs(end_clock_time)),
        );
    }

    #[test]
    fn test_transition_fh_to_ht_rugby_pen() {
        initialize();
        let config = GameConfig {
            half_time_duration: Duration::from_secs(5),
            penalty_shot_duration: Duration::from_secs(3),
            ..Default::default()
        };
        test_penalty_transition(PenaltyTransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::FirstHalf,
            remaining: 3,
            start_penalty_after: 2,
            first_check: 4,
            second_check: 7,
            end_period: GamePeriod::HalfTime,
            end_clock_time: 3,
        });
    }

    #[test]
    fn test_transition_sh_to_pot_rugby_pen() {
        initialize();
        let config = GameConfig {
            penalty_shot_duration: Duration::from_secs(10),
            overtime_allowed: true,
            pre_overtime_break: Duration::from_secs(5),
            ..Default::default()
        };
        test_penalty_transition(PenaltyTransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::SecondHalf,
            remaining: 4,
            start_penalty_after: 2,
            first_check: 5,
            second_check: 14,
            end_period: GamePeriod::PreOvertime,
            end_clock_time: 3,
        });
    }

    #[test]
    fn test_transition_ot_fh_to_ot_ht_rugby_pen() {
        initialize();
        let config = GameConfig {
            penalty_shot_duration: Duration::from_secs(13),
            overtime_allowed: true,
            ot_half_time_duration: Duration::from_secs(7),
            ..Default::default()
        };
        test_penalty_transition(PenaltyTransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::OvertimeFirstHalf,
            remaining: 5,
            start_penalty_after: 1,
            first_check: 7,
            second_check: 15,
            end_period: GamePeriod::OvertimeHalfTime,
            end_clock_time: 6,
        });
    }

    #[test]
    fn test_transition_ot_sh_to_psd_rugby_pen() {
        initialize();
        let config = GameConfig {
            penalty_shot_duration: Duration::from_secs(9),
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(14),
            ..Default::default()
        };
        test_penalty_transition(PenaltyTransitionTestSetup {
            config,
            game_start_offset: 0,
            start_period: GamePeriod::OvertimeSecondHalf,
            remaining: 17,
            start_penalty_after: 15,
            first_check: 18,
            second_check: 27,
            end_period: GamePeriod::PreSuddenDeath,
            end_clock_time: 11,
        });
    }

    #[test]
    fn test_start_penalty() {
        initialize();
        let start = Instant::now();
        let first_time = start + Duration::from_secs(1);
        let time = first_time;

        let mut tm = TournamentManager::new(Default::default());

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            first_time,
            Infraction::StickInfringement,
        )
        .unwrap();

        let next_time = time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 2,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: first_time,
                infraction: Infraction::StickInfringement,
            }]
        );
        assert_eq!(tm.penalties.white, vec![]);

        let time = next_time + Duration::from_secs(1);
        let next_time = time + Duration::from_secs(1);
        tm.start_penalty(
            Color::Black,
            3,
            PenaltyKind::TwoMinute,
            time,
            Infraction::DelayOfGame,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            4,
            PenaltyKind::FiveMinute,
            time,
            Infraction::FalseStart,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            5,
            PenaltyKind::TotalDismissal,
            time,
            Infraction::FreeArm,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            6,
            PenaltyKind::OneMinute,
            time,
            Infraction::GrabbingTheBarrier,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            7,
            PenaltyKind::TwoMinute,
            time,
            Infraction::IllegalAdvancement,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            8,
            PenaltyKind::FiveMinute,
            time,
            Infraction::IllegalSubstitution,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            9,
            PenaltyKind::TotalDismissal,
            time,
            Infraction::IllegallyStoppingThePuck,
        )
        .unwrap();

        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![
                Penalty {
                    kind: PenaltyKind::OneMinute,
                    player_number: 2,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(24),
                    start_instant: first_time,
                    infraction: Infraction::StickInfringement,
                },
                Penalty {
                    kind: PenaltyKind::TwoMinute,
                    player_number: 3,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::DelayOfGame,
                },
                Penalty {
                    kind: PenaltyKind::FiveMinute,
                    player_number: 4,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::FalseStart,
                },
                Penalty {
                    kind: PenaltyKind::TotalDismissal,
                    player_number: 5,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::FreeArm,
                },
            ]
        );
        assert_eq!(
            tm.penalties.white,
            vec![
                Penalty {
                    kind: PenaltyKind::OneMinute,
                    player_number: 6,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::GrabbingTheBarrier,
                },
                Penalty {
                    kind: PenaltyKind::TwoMinute,
                    player_number: 7,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::IllegalAdvancement,
                },
                Penalty {
                    kind: PenaltyKind::FiveMinute,
                    player_number: 8,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::IllegalSubstitution,
                },
                Penalty {
                    kind: PenaltyKind::TotalDismissal,
                    player_number: 9,
                    start_period: GamePeriod::FirstHalf,
                    start_time: Duration::from_secs(22),
                    start_instant: time,
                    infraction: Infraction::IllegallyStoppingThePuck,
                },
            ]
        );
    }

    #[test]
    fn test_delete_penalty() {
        initialize();
        let start = Instant::now();
        let time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(Default::default());

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            time,
            Infraction::StickInfringement,
        )
        .unwrap();

        let next_time = time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 2,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: time,
                infraction: Infraction::StickInfringement,
            }],
        );
        assert_eq!(tm.penalties.white, vec![]);

        let time = next_time;
        let next_time = time + Duration::from_secs(1);
        assert_eq!(
            tm.delete_penalty(Color::Black, 1,),
            Err(TournamentManagerError::InvalidPenIndex(Color::Black, 1))
        );
        assert_eq!(
            tm.delete_penalty(Color::White, 0,),
            Err(TournamentManagerError::InvalidPenIndex(Color::White, 0))
        );
        assert_eq!(
            tm.delete_penalty(Color::White, 1,),
            Err(TournamentManagerError::InvalidPenIndex(Color::White, 1))
        );
        tm.delete_penalty(Color::Black, 0).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(tm.penalties.white, vec![]);

        let time = next_time + Duration::from_secs(1);
        let next_time = time + Duration::from_secs(1);
        tm.start_penalty(
            Color::White,
            3,
            PenaltyKind::OneMinute,
            time,
            Infraction::Obstruction,
        )
        .unwrap();

        tm.update(next_time).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(
            tm.penalties.white,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 3,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(21),
                start_instant: time,
                infraction: Infraction::Obstruction,
            }],
        );

        assert_eq!(
            tm.delete_penalty(Color::White, 1,),
            Err(TournamentManagerError::InvalidPenIndex(Color::White, 1))
        );
        assert_eq!(
            tm.delete_penalty(Color::Black, 0),
            Err(TournamentManagerError::InvalidPenIndex(Color::Black, 0))
        );
        assert_eq!(
            tm.delete_penalty(Color::Black, 1),
            Err(TournamentManagerError::InvalidPenIndex(Color::Black, 1))
        );
        tm.delete_penalty(Color::White, 0).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(tm.penalties.white, vec![]);
    }

    #[test]
    fn test_edit_penalty() {
        initialize();
        let start = Instant::now();
        let pen_start_time = start + Duration::from_secs(1);

        let mut tm = TournamentManager::new(Default::default());

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            pen_start_time,
            Infraction::OutOfBounds,
        )
        .unwrap();

        let next_time = pen_start_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 2,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::OutOfBounds,
            }],
        );
        assert_eq!(tm.penalties.white, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        assert_eq!(
            tm.edit_penalty(
                Color::Black,
                1,
                Color::Black,
                2,
                PenaltyKind::TwoMinute,
                Infraction::IllegalAdvancement
            ),
            Err(TournamentManagerError::InvalidPenIndex(Color::Black, 1))
        );
        assert_eq!(
            tm.edit_penalty(
                Color::White,
                0,
                Color::Black,
                2,
                PenaltyKind::TwoMinute,
                Infraction::IllegalAdvancement
            ),
            Err(TournamentManagerError::InvalidPenIndex(Color::White, 0))
        );
        assert_eq!(
            tm.edit_penalty(
                Color::White,
                1,
                Color::Black,
                2,
                PenaltyKind::TwoMinute,
                Infraction::IllegalAdvancement
            ),
            Err(TournamentManagerError::InvalidPenIndex(Color::White, 1))
        );
        tm.edit_penalty(
            Color::Black,
            0,
            Color::Black,
            3,
            PenaltyKind::TwoMinute,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![Penalty {
                kind: PenaltyKind::TwoMinute,
                player_number: 3,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
            }],
        );
        assert_eq!(tm.penalties.white, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::Black,
            0,
            Color::Black,
            4,
            PenaltyKind::FiveMinute,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![Penalty {
                kind: PenaltyKind::FiveMinute,
                player_number: 4,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
            }],
        );
        assert_eq!(tm.penalties.white, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::Black,
            0,
            Color::Black,
            5,
            PenaltyKind::TotalDismissal,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(
            tm.penalties.black,
            vec![Penalty {
                kind: PenaltyKind::TotalDismissal,
                player_number: 5,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
            }],
        );
        assert_eq!(tm.penalties.white, vec![]);

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::Black,
            0,
            Color::White,
            6,
            PenaltyKind::TotalDismissal,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(
            tm.penalties.white,
            vec![Penalty {
                kind: PenaltyKind::TotalDismissal,
                player_number: 6,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
            }],
        );

        let next_time = next_time + Duration::from_secs(1);
        assert_eq!(
            tm.edit_penalty(
                Color::White,
                1,
                Color::White,
                2,
                PenaltyKind::TwoMinute,
                Infraction::Unknown
            ),
            Err(TournamentManagerError::InvalidPenIndex(Color::White, 1))
        );
        assert_eq!(
            tm.edit_penalty(
                Color::Black,
                0,
                Color::Black,
                2,
                PenaltyKind::TwoMinute,
                Infraction::Unknown
            ),
            Err(TournamentManagerError::InvalidPenIndex(Color::Black, 0))
        );
        assert_eq!(
            tm.edit_penalty(
                Color::Black,
                1,
                Color::Black,
                2,
                PenaltyKind::TwoMinute,
                Infraction::Unknown
            ),
            Err(TournamentManagerError::InvalidPenIndex(Color::Black, 1))
        );
        tm.edit_penalty(
            Color::White,
            0,
            Color::White,
            7,
            PenaltyKind::FiveMinute,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(
            tm.penalties.white,
            vec![Penalty {
                kind: PenaltyKind::FiveMinute,
                player_number: 7,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
            }],
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::White,
            0,
            Color::White,
            8,
            PenaltyKind::TwoMinute,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(
            tm.penalties.white,
            vec![Penalty {
                kind: PenaltyKind::TwoMinute,
                player_number: 8,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
            }],
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.edit_penalty(
            Color::White,
            0,
            Color::White,
            10,
            PenaltyKind::OneMinute,
            Infraction::Unknown,
        )
        .unwrap();
        tm.update(next_time).unwrap();
        assert_eq!(tm.penalties.black, vec![]);
        assert_eq!(
            tm.penalties.white,
            vec![Penalty {
                kind: PenaltyKind::OneMinute,
                player_number: 10,
                start_period: GamePeriod::FirstHalf,
                start_time: Duration::from_secs(24),
                start_instant: pen_start_time,
                infraction: Infraction::Unknown,
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
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            next_time,
            Infraction::Unknown,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(59),
                    infraction: Infraction::Unknown,
                }],
                white: vec![]
            }
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(
            Color::White,
            3,
            PenaltyKind::OneMinute,
            next_time,
            Infraction::UnsportsmanlikeConduct,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(57),
                    infraction: Infraction::Unknown,
                }],
                white: vec![PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(59),
                    infraction: Infraction::UnsportsmanlikeConduct,
                }]
            }
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(
            Color::Black,
            4,
            PenaltyKind::TwoMinute,
            next_time,
            Infraction::DelayOfGame,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            5,
            PenaltyKind::TwoMinute,
            next_time,
            Infraction::FalseStart,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(55),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(119),
                        infraction: Infraction::DelayOfGame,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(57),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(119),
                        infraction: Infraction::FalseStart,
                    },
                ]
            }
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(
            Color::Black,
            6,
            PenaltyKind::FiveMinute,
            next_time,
            Infraction::IllegalAdvancement,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            7,
            PenaltyKind::FiveMinute,
            next_time,
            Infraction::IllegallyStoppingThePuck,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(53),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(117),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::Seconds(299),
                        infraction: Infraction::IllegalAdvancement,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(55),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(117),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(299),
                        infraction: Infraction::IllegallyStoppingThePuck,
                    },
                ]
            }
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(
            Color::Black,
            8,
            PenaltyKind::TotalDismissal,
            next_time,
            Infraction::IllegalSubstitution,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            9,
            PenaltyKind::TotalDismissal,
            next_time,
            Infraction::Obstruction,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(51),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(115),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::Seconds(297),
                        infraction: Infraction::IllegalAdvancement,
                    },
                    PenaltySnapshot {
                        player_number: 8,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalSubstitution,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(53),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(115),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(297),
                        infraction: Infraction::IllegallyStoppingThePuck,
                    },
                    PenaltySnapshot {
                        player_number: 9,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::Obstruction,
                    },
                ]
            }
        );

        // Check 5 seconds after Half Time has started (there were 15s remaining in first half)
        let next_time = next_time + Duration::from_secs(20);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(36),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(100),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::Seconds(282),
                        infraction: Infraction::IllegalAdvancement,
                    },
                    PenaltySnapshot {
                        player_number: 8,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalSubstitution,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(38),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(100),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(282),
                        infraction: Infraction::IllegallyStoppingThePuck,
                    },
                    PenaltySnapshot {
                        player_number: 9,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::Obstruction,
                    },
                ]
            }
        );

        // Check 10 seconds after Second Half has started (there were 175s remaining in Half Time)
        let next_time = next_time + Duration::from_secs(185);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(26),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(90),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::Seconds(272),
                        infraction: Infraction::IllegalAdvancement,
                    },
                    PenaltySnapshot {
                        player_number: 8,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalSubstitution,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(28),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(90),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(272),
                        infraction: Infraction::IllegallyStoppingThePuck,
                    },
                    PenaltySnapshot {
                        player_number: 9,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::Obstruction,
                    },
                ]
            }
        );

        // Check after the first two penalties have finished
        let next_time = next_time + Duration::from_secs(30);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(60),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::Seconds(242),
                        infraction: Infraction::IllegalAdvancement,
                    },
                    PenaltySnapshot {
                        player_number: 8,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalSubstitution,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(60),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(242),
                        infraction: Infraction::IllegallyStoppingThePuck,
                    },
                    PenaltySnapshot {
                        player_number: 9,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::Obstruction,
                    },
                ]
            }
        );

        // Check after all the penalties have finished
        let next_time = next_time + Duration::from_secs(250);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::IllegalAdvancement,
                    },
                    PenaltySnapshot {
                        player_number: 8,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalSubstitution,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::UnsportsmanlikeConduct,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::IllegallyStoppingThePuck,
                    },
                    PenaltySnapshot {
                        player_number: 9,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::Obstruction,
                    },
                ]
            }
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

        tm.scores.black = 1;
        tm.scores.white = 5;
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(25));
        tm.start_game_clock(start);
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            next_time,
            Infraction::Unknown,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(59),
                    infraction: Infraction::Unknown,
                }],
                white: vec![]
            }
        );

        let next_time = next_time + Duration::from_secs(1);
        tm.start_penalty(
            Color::White,
            3,
            PenaltyKind::TwoMinute,
            next_time,
            Infraction::DelayOfGame,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            5,
            PenaltyKind::TotalDismissal,
            next_time,
            Infraction::FalseStart,
        )
        .unwrap();

        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(57),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::FalseStart,
                    }
                ],
                white: vec![PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(119),
                    infraction: Infraction::DelayOfGame,
                }]
            }
        );

        // Check after the game has ended
        let next_time = next_time + Duration::from_secs(30);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(snapshot.current_period, GamePeriod::BetweenGames);
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::FalseStart,
                    },
                ],
                white: vec![PenaltySnapshot {
                    player_number: 3,
                    time: PenaltyTime::Seconds(0),
                    infraction: Infraction::DelayOfGame,
                },]
            }
        );
    }

    #[test]
    fn test_snapshot_penalty_before_penalty_start() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            ..Default::default()
        };

        let tm_start = Instant::now();
        let earlier_time = tm_start + Duration::from_secs(1);
        let pen_start = tm_start + Duration::from_secs(2);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(25));
        tm.start_game_clock(tm_start);
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            pen_start,
            Infraction::Unknown,
        )
        .unwrap();

        let snapshot = tm.generate_snapshot(earlier_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(61),
                    infraction: Infraction::Unknown,
                }],
                white: vec![]
            }
        );
    }

    #[test]
    fn test_time_edit_limits_penalty_duration() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            ..Default::default()
        };

        let tm_start = Instant::now();
        let pen_start = tm_start + Duration::from_secs(20);
        let clock_stop = tm_start + Duration::from_secs(30);
        let clock_restart = tm_start + Duration::from_secs(40);
        let check_time = tm_start + Duration::from_secs(50);

        let start_clock_time = Duration::from_secs(180);
        let edited_clock_time = Duration::from_secs(240);

        let mut tm = TournamentManager::new(config);

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, start_clock_time);
        tm.start_game_clock(tm_start);
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            pen_start,
            Infraction::Unknown,
        )
        .unwrap();

        tm.stop_clock(clock_stop).unwrap(); // At this point the game clock reads 150s, the penalty has 50s left
        tm.set_game_clock_time(edited_clock_time).unwrap();
        tm.start_game_clock(clock_restart);

        // The penalty should have 50s left, without the limiting it would have 130s left
        let snapshot = tm.generate_snapshot(check_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![PenaltySnapshot {
                    player_number: 2,
                    time: PenaltyTime::Seconds(50),
                    infraction: Infraction::Unknown,
                }],
                white: vec![]
            }
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
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            next_time,
            Infraction::Unknown,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            3,
            PenaltyKind::OneMinute,
            next_time,
            Infraction::DelayOfGame,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            4,
            PenaltyKind::TwoMinute,
            next_time,
            Infraction::FalseStart,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            5,
            PenaltyKind::TwoMinute,
            next_time,
            Infraction::FreeArm,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            6,
            PenaltyKind::TotalDismissal,
            next_time,
            Infraction::GrabbingTheBarrier,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            7,
            PenaltyKind::TotalDismissal,
            next_time,
            Infraction::IllegalAdvancement,
        )
        .unwrap();

        // Check before culling
        let next_time = next_time + Duration::from_secs(1);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(59),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(119),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::GrabbingTheBarrier,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(59),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(119),
                        infraction: Infraction::FreeArm,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalAdvancement,
                    },
                ]
            }
        );

        // Check during half time (pre-culling)
        let next_time = next_time + Duration::from_secs(75);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(50),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::GrabbingTheBarrier,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 3,
                        time: PenaltyTime::Seconds(0),
                        infraction: Infraction::DelayOfGame,
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(50),
                        infraction: Infraction::FreeArm,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalAdvancement,
                    },
                ]
            }
        );

        // Check 6s after half time (post-culling)
        let next_time = next_time + Duration::from_secs(180);
        tm.update(next_time).unwrap();
        let snapshot = tm.generate_snapshot(next_time).unwrap();
        assert_eq!(
            snapshot.penalties,
            BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(44),
                        infraction: Infraction::FalseStart,
                    },
                    PenaltySnapshot {
                        player_number: 6,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::GrabbingTheBarrier,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(44),
                        infraction: Infraction::FreeArm,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::IllegalAdvancement,
                    },
                ]
            }
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
        tm.start_penalty(
            Color::Black,
            2,
            PenaltyKind::OneMinute,
            pen_start,
            Infraction::Unknown,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            3,
            PenaltyKind::OneMinute,
            pen_start,
            Infraction::DelayOfGame,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            4,
            PenaltyKind::TwoMinute,
            pen_start,
            Infraction::FalseStart,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            5,
            PenaltyKind::TwoMinute,
            pen_start,
            Infraction::FreeArm,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            6,
            PenaltyKind::TotalDismissal,
            pen_start,
            Infraction::GrabbingTheBarrier,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            7,
            PenaltyKind::TotalDismissal,
            pen_start,
            Infraction::IllegalAdvancement,
        )
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
        assert_eq!(tm.penalties.black.len(), 3);
        assert_eq!(tm.penalties.white.len(), 3);

        // Check while two penalties are still running per color
        now += Duration::from_secs(60);
        assert_eq!(tm.limit_pen_list_len(Color::Black, 2, now), Ok(()));
        assert_eq!(tm.limit_pen_list_len(Color::White, 2, now), Ok(()));
        assert_eq!(tm.penalties.black.len(), 2);
        assert_eq!(tm.penalties.white.len(), 2);

        // Check while one penalty is still running per color
        now += Duration::from_secs(60);
        assert_eq!(tm.limit_pen_list_len(Color::Black, 2, now), Ok(()));
        assert_eq!(tm.limit_pen_list_len(Color::White, 2, now), Ok(()));
        assert_eq!(tm.penalties.black.len(), 2);
        assert_eq!(tm.penalties.white.len(), 2);
    }

    #[test]
    fn test_could_end_game() {
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
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::FirstHalf;
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::HalfTime;
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::SecondHalf;
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.set_scores(BlackWhiteBundle { black: 3, white: 4 }, start_time);
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.config.sudden_death_allowed = true;
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.config.overtime_allowed = true;
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.set_scores(BlackWhiteBundle { black: 4, white: 4 }, start_time);
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::PreOvertime;
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::OvertimeFirstHalf;
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::OvertimeHalfTime;
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::OvertimeSecondHalf;
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.config.sudden_death_allowed = false;
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.set_scores(BlackWhiteBundle { black: 4, white: 5 }, start_time);
        tm.config.sudden_death_allowed = true;
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.stop_clock(start_time).unwrap();
        tm.config.overtime_allowed = false;
        tm.config.sudden_death_allowed = false;
        tm.current_period = GamePeriod::SecondHalf;
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_secs(20),
            },
        )));
        tm.clock_state = ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_nanos(10),
        };
        assert_eq!(Ok(false), tm.could_end_game(next_time));
        tm.clock_state = ClockState::Stopped {
            clock_time: Duration::ZERO,
        };
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.current_period = GamePeriod::FirstHalf;
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_secs(20),
            },
        )));
        tm.clock_state = ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_nanos(10),
        };
        assert_eq!(Ok(false), tm.could_end_game(next_time));
        tm.clock_state = ClockState::Stopped {
            clock_time: Duration::ZERO,
        };
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.stop_clock(start_time).unwrap();
        tm.current_period = GamePeriod::SecondHalf;
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_nanos(10),
            },
        )));
        tm.clock_state = ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_secs(20),
        };
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.stop_clock(start_time).unwrap();
        tm.set_game_clock_time(Duration::ZERO).unwrap();
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_nanos(10),
            },
        )));
        assert_eq!(Ok(true), tm.could_end_game(next_time));

        tm.clock_state = ClockState::CountingUp {
            start_time,
            time_at_start: Duration::ZERO,
        };
        tm.set_scores(BlackWhiteBundle { black: 4, white: 5 }, start_time);
        assert_eq!(Ok(false), tm.could_end_game(next_time));
    }

    #[test]
    fn test_timeout_end_would_end_game() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start_time = Instant::now();
        let next_time = start_time + Duration::from_secs(1);

        tm.stop_clock(start_time).unwrap();
        tm.current_period = GamePeriod::SecondHalf;
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_secs(20),
            },
        )));
        tm.clock_state = ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_nanos(10),
        };
        assert_eq!(Ok(true), tm.timeout_end_would_end_game(next_time));
        tm.clock_state = ClockState::Stopped {
            clock_time: Duration::ZERO,
        };
        assert_eq!(Ok(true), tm.timeout_end_would_end_game(next_time));

        tm.current_period = GamePeriod::FirstHalf;
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_secs(20),
            },
        )));
        tm.clock_state = ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_nanos(10),
        };
        assert_eq!(Ok(false), tm.timeout_end_would_end_game(next_time));
        tm.clock_state = ClockState::Stopped {
            clock_time: Duration::ZERO,
        };
        assert_eq!(Ok(false), tm.timeout_end_would_end_game(next_time));

        tm.stop_clock(start_time).unwrap();
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_nanos(10),
            },
        )));
        tm.clock_state = ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_secs(20),
        };
        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.stop_clock(start_time).unwrap();
        tm.current_period = GamePeriod::SecondHalf;
        tm.set_game_clock_time(Duration::ZERO).unwrap();
        tm.set_timeout_state(Some(TimeoutState::RugbyPenaltyShot(
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start: Duration::from_nanos(10),
            },
        )));
        assert_eq!(Ok(true), tm.could_end_game(next_time));
    }

    #[test]
    fn test_halt_clock() {
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
        tm.halt_clock(next_time, false).unwrap();
        assert_eq!(
            ClockState::Stopped {
                clock_time: Duration::from_nanos(1)
            },
            tm.clock_state
        );

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_millis(500));
        tm.timeout_state = Some(TimeoutState::Ref(ClockState::CountingUp {
            start_time,
            time_at_start: Duration::ZERO,
        }));
        assert_eq!(
            Err(TMErr::AlreadyInTimeout(TimeoutSnapshot::Ref(1))),
            tm.halt_clock(next_time, false)
        );

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_millis(500));
        tm.timeout_state = Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_secs(20),
        }));
        tm.halt_clock(next_time, false).unwrap();
        assert_eq!(
            ClockState::Stopped {
                clock_time: Duration::from_nanos(1)
            },
            tm.clock_state
        );

        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::ZERO);
        tm.timeout_state = Some(TimeoutState::RugbyPenaltyShot(ClockState::CountingDown {
            start_time,
            time_remaining_at_start: Duration::from_secs(20),
        }));
        tm.halt_clock(next_time, true).unwrap();
        assert_eq!(
            ClockState::Stopped {
                clock_time: Duration::from_nanos(1)
            },
            tm.clock_state
        );

        tm.timeout_state = None;
        assert_eq!(Err(TMErr::InvalidState), tm.halt_clock(next_time, false));
    }

    #[test]
    fn test_pause_score_confirm_with_ot_and_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_1_end = game_end + Duration::from_secs(4);

        // Coming out of second half, with overtime and sd, pause_duration should be 4
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(4));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(3);
        let after_end = game_end + Duration::from_secs(5);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_1_end).unwrap();

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());
    }

    #[test]
    fn test_pause_score_confirm_with_only_sd_score_changed_to_tie() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_2_end = game_end + Duration::from_secs(6);

        // No overtime, only sd allowed, pause_duration should be 6
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(6));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(5);
        let after_end = game_end + Duration::from_secs(7);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        tm.end_confirm_pause(pause_2_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::PreSuddenDeath);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_pause_score_confirm_no_ot_and_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_3_end = game_end + Duration::from_secs(10);

        // No overtime or sd, pause_duration should be 10
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(10));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(9);
        let after_end = game_end + Duration::from_secs(11);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_3_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_pause_score_confirm_with_ot_and_sd_btwn_games_shortest() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(4),
            post_game_duration: Duration::from_secs(4),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_4_end = game_end + Duration::from_secs(2);

        // With OT and SD, between games is shortest period, pause_duration should be 2
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(2));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(1);
        let after_end = game_end + Duration::from_secs(3);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_4_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_pause_score_confirm_from_ot_with_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            ot_half_play_duration: Duration::from_secs(60),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_ot_end = start + Duration::from_secs(29);
        let ot_end = start + Duration::from_secs(30);
        let pause_5_end = ot_end + Duration::from_secs(6);

        // Coming out of OT, pause should be 6
        tm.set_period_and_game_clock_time(GamePeriod::OvertimeSecondHalf, Duration::from_secs(30));
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_ot_end));
        assert_eq!(Ok(true), tm.could_end_game(ot_end));

        tm.pause_for_confirm(ot_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(6));
        assert_eq!(confirm.pause_began, ot_end);

        let before_end = ot_end + Duration::from_secs(5);
        let after_end = ot_end + Duration::from_secs(7);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_5_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_pause_score_confirm_from_ot_no_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: false,
            pre_overtime_break: Duration::from_secs(8),
            ot_half_play_duration: Duration::from_secs(60),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_ot_end = start + Duration::from_secs(29);
        let ot_end = start + Duration::from_secs(30);
        let pause_6_end = ot_end + Duration::from_secs(10);

        // Coming out of OT, pause should be 10
        tm.set_period_and_game_clock_time(GamePeriod::OvertimeSecondHalf, Duration::from_secs(30));
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_ot_end));
        assert_eq!(Ok(true), tm.could_end_game(ot_end));

        tm.pause_for_confirm(ot_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(10));
        assert_eq!(confirm.pause_began, ot_end);

        let before_end = ot_end + Duration::from_secs(9);
        let after_end = ot_end + Duration::from_secs(11);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_6_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_pause_score_confirm_from_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_overtime_break: Duration::from_secs(8),
            ot_half_play_duration: Duration::from_secs(60),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);
        let pause_7_end = next_time + Duration::from_secs(10);

        // Coming out of SD, pause should be 10
        tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::from_micros(10));
        tm.set_scores(BlackWhiteBundle { black: 1, white: 1 }, start);
        tm.start_game_clock(start);
        assert!(tm.clock_is_running());

        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.pause_for_confirm(next_time).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(10));
        assert_eq!(confirm.pause_began, next_time);

        let before_end = next_time + Duration::from_secs(9);
        let after_end = next_time + Duration::from_secs(11);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.set_scores(BlackWhiteBundle { black: 2, white: 1 }, start);

        tm.end_confirm_pause(pause_7_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_tied_pause_score_confirm_with_ot_and_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_8_end = game_end + Duration::from_secs(4);

        // Coming out of second half, with overtime and sd, pause_duration should be 4
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(4));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(3);
        let after_end = game_end + Duration::from_secs(5);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_8_end).unwrap();

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);

        assert_eq!(tm.current_period, GamePeriod::PreOvertime);
        assert!(tm.clock_is_running());
    }

    #[test]
    fn test_tied_pause_score_confirm_with_only_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_9_end = game_end + Duration::from_secs(6);

        // No overtime, only sd allowed, pause_duration should be 6
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(6));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(5);
        let after_end = game_end + Duration::from_secs(7);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_9_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::PreSuddenDeath);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_tied_pause_score_confirm_no_ot_and_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: false,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_10_end = game_end + Duration::from_secs(10);

        // No overtime or sd, pause_duration should be 10
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(10));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(9);
        let after_end = game_end + Duration::from_secs(11);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_10_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    // End of OT with SD
    #[test]
    fn test_tied_pause_score_confirm_from_ot_to_sd() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let pause_11_end = game_end + Duration::from_secs(6);

        // Overtime into SD, pause_duration should be 6
        tm.set_period_and_game_clock_time(GamePeriod::OvertimeSecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(6));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(5);
        let after_end = game_end + Duration::from_secs(7);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.end_confirm_pause(pause_11_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::PreSuddenDeath);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_tied_pause_score_confirm_from_game_to_ot_changed_to_not_tied() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let mid_pause = game_end + Duration::from_secs(2);

        // Game into Overtime, pause_duration should be 4
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(4));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(3);
        let after_end = game_end + Duration::from_secs(5);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.set_scores(BlackWhiteBundle { black: 2, white: 3 }, mid_pause);

        assert_eq!(tm.current_period, GamePeriod::SecondHalf);

        tm.end_confirm_pause(mid_pause).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_tied_pause_score_confirm_from_ot_to_sd_changed_to_not_tied() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_sudden_death_duration: Duration::from_secs(12),
            pre_overtime_break: Duration::from_secs(8),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let pre_game_end = start + Duration::from_secs(29);
        let game_end = start + Duration::from_secs(30);
        let mid_pause = game_end + Duration::from_secs(3);
        let pause_13_end = game_end + Duration::from_secs(6);

        // Overtime into SD, pause_duration should be 6
        tm.set_period_and_game_clock_time(GamePeriod::OvertimeSecondHalf, Duration::from_secs(30));
        tm.set_game_start(start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        tm.set_scores(BlackWhiteBundle { black: 2, white: 2 }, start);

        assert_eq!(Ok(false), tm.could_end_game(pre_game_end));
        assert_eq!(Ok(true), tm.could_end_game(game_end));

        tm.pause_for_confirm(game_end).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(6));
        assert_eq!(confirm.pause_began, game_end);

        let before_end = game_end + Duration::from_secs(5);
        let after_end = game_end + Duration::from_secs(7);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.set_scores(BlackWhiteBundle { black: 2, white: 3 }, mid_pause);

        tm.end_confirm_pause(pause_13_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }

    #[test]
    fn test_pause_score_confirm_from_sd_changed_back_to_tie_in_pause() {
        initialize();
        let config = GameConfig {
            overtime_allowed: true,
            sudden_death_allowed: true,
            pre_overtime_break: Duration::from_secs(8),
            ot_half_play_duration: Duration::from_secs(60),
            minimum_break: Duration::from_secs(20),
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        let start = Instant::now();
        let next_time = start + Duration::from_secs(1);
        let mid_pause = next_time + Duration::from_secs(5);
        let pause_14_end = next_time + Duration::from_secs(10);

        // Coming out of SD, pause should be 10
        tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(30));
        tm.set_scores(BlackWhiteBundle { black: 1, white: 1 }, start);
        tm.start_game_clock(start);
        assert_eq!(tm.clock_is_running(), true);

        assert_eq!(Ok(false), tm.could_end_game(next_time));

        tm.pause_for_confirm(next_time).unwrap();

        assert!(tm.in_score_confirm_pause());

        let confirm = tm.time_pause_confirmation.as_ref().unwrap();
        assert_eq!(confirm.duration_of_pause, Duration::from_secs(10));
        assert_eq!(confirm.pause_began, next_time);

        let before_end = next_time + Duration::from_secs(9);
        let after_end = next_time + Duration::from_secs(11);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        tm.set_scores(BlackWhiteBundle { black: 1, white: 1 }, mid_pause);

        tm.end_confirm_pause(pause_14_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::SuddenDeath);
        let clock_time = Duration::from_secs(31);
        assert_eq!(tm.game_clock_time(pause_14_end), Some(clock_time));
        assert!(tm.clock_is_running());

        assert!(!tm.in_score_confirm_pause());
        assert_eq!(tm.time_pause_confirmation, None);
    }
}
