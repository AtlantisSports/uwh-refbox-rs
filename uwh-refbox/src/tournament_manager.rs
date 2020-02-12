use crate::config::Game as GameConfig;
use crate::game_state::GamePeriod;
use std::cmp::max;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TournamentManager {
    config: GameConfig,
    current_game: u16,
    game_start_time: Instant,
    current_period: GamePeriod,
    clock_state: ClockState,
    b_score: u8,
    w_score: u8,
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
            config,
            b_score: 0,
            w_score: 0,
        }
    }

    pub fn clock_is_running(&self) -> bool {
        match self.clock_state {
            ClockState::CountingDown {
                start_time: _,
                time_remaining_at_start: _,
            }
            | ClockState::CountingUp {
                start_time: _,
                time_at_start: _,
            } => true,
            ClockState::Stopped { clock_time: _ } => false,
        }
    }

    pub fn current_period(&self) -> GamePeriod {
        self.current_period
    }

    pub fn current_game(&self) -> u16 {
        self.current_game
    }

    pub fn game_start_time(&self) -> Instant {
        self.game_start_time
    }

    pub fn add_b_score(&mut self, _player_num: u8, now: Instant) {
        self.set_scores(self.b_score + 1, self.w_score, now);
    }

    pub fn add_w_score(&mut self, _player_num: u8, now: Instant) {
        self.set_scores(self.b_score, self.w_score + 1, now);
    }

    pub fn set_scores(&mut self, b_score: u8, w_score: u8, now: Instant) {
        self.b_score = b_score;
        self.w_score = w_score;
        if self.current_period == GamePeriod::SuddenDeath && b_score != w_score {
            self.end_game(now);
        }
    }

    // TODO: Doesn't handle getting behind and catching up correctly
    fn end_game(&mut self, now: Instant) {
        self.current_period = GamePeriod::BetweenGames;

        let scheduled_start = self.game_start_time
            + 2 * Duration::from_secs(self.config.half_play_duration.into())
            + Duration::from_secs(self.config.half_time_duration.into())
            + Duration::from_secs(self.config.nominal_break.into());

        let game_end = match self.clock_state {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => start_time + time_remaining_at_start,
            ClockState::CountingUp {
                start_time: _,
                time_at_start: _,
            }
            | ClockState::Stopped { clock_time: _ } => now,
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

        self.clock_state = ClockState::CountingDown {
            start_time: game_end,
            time_remaining_at_start,
        }
    }

    pub(super) fn update(&mut self, now: Instant) {
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
                        self.current_period = GamePeriod::FirstHalf;
                        self.game_start_time = start_time + time_remaining_at_start;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.half_play_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::FirstHalf => {
                        self.current_period = GamePeriod::HalfTime;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.half_time_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::HalfTime => {
                        self.current_period = GamePeriod::SecondHalf;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.half_play_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::SecondHalf => {
                        if self.b_score != self.w_score
                            || (!self.config.has_overtime && !self.config.sudden_death_allowed)
                        {
                            self.end_game(now);
                        } else if self.config.has_overtime {
                            self.current_period = GamePeriod::PreOvertime;
                            self.clock_state = ClockState::CountingDown {
                                start_time: start_time + time_remaining_at_start,
                                time_remaining_at_start: Duration::from_secs(
                                    self.config.pre_overtime_break.into(),
                                ),
                            }
                        } else {
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
                        self.current_period = GamePeriod::OvertimeFirstHalf;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.ot_half_play_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::OvertimeFirstHalf => {
                        self.current_period = GamePeriod::OvertimeHalfTime;
                        self.clock_state = ClockState::CountingDown {
                            start_time: start_time + time_remaining_at_start,
                            time_remaining_at_start: Duration::from_secs(
                                self.config.ot_half_time_duration.into(),
                            ),
                        }
                    }
                    GamePeriod::OvertimeHalfTime => {
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
                        self.current_period = GamePeriod::SuddenDeath;
                        self.clock_state = ClockState::CountingUp {
                            start_time: start_time + time_remaining_at_start,
                            time_at_start: Duration::from_secs(0),
                        }
                    }
                    GamePeriod::SuddenDeath => {}
                }
            }
        }
    }

    pub fn start_clock(&mut self, now: Instant) {
        if let ClockState::Stopped { clock_time } = self.clock_state {
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

    pub fn stop_clock(&mut self, now: Instant) {
        match self.clock_state {
            ClockState::CountingDown {
                start_time: _,
                time_remaining_at_start: _,
            }
            | ClockState::CountingUp {
                start_time: _,
                time_at_start: _,
            } => {
                self.clock_state = ClockState::Stopped {
                    clock_time: self.clock_time(now).unwrap(),
                }
            }
            ClockState::Stopped { clock_time: _ } => {}
        };
    }

    #[cfg(test)]
    pub(super) fn set_period_and_clock_time(&mut self, period: GamePeriod, clock_time: Duration) {
        if let ClockState::Stopped { clock_time: _ } = self.clock_state {
            self.current_period = period;
            self.clock_state = ClockState::Stopped { clock_time }
        } else {
            panic!("Can't edit period and remaing time while clock is running");
        }
    }

    #[cfg(test)]
    pub(super) fn set_game_start(&mut self, time: Instant) {
        if let ClockState::Stopped { clock_time: _ } = self.clock_state {
            self.game_start_time = time;
        } else {
            panic!("Can't edit game start time while clock is running");
        }
    }

    // Returns `None` if the clock time would be negative, or if `now` is before the start
    // of the current period
    pub fn clock_time(&self, now: Instant) -> Option<Duration> {
        match self.clock_state {
            ClockState::CountingDown {
                start_time,
                time_remaining_at_start,
            } => now
                .checked_duration_since(start_time)
                .and_then(|s| time_remaining_at_start.checked_sub(s)),
            ClockState::CountingUp {
                start_time,
                time_at_start,
            } => now
                .checked_duration_since(start_time)
                .map(|s| s + time_at_start),
            ClockState::Stopped { clock_time } => Some(clock_time),
        }
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
        assert_eq!(tm.clock_time(start), Some(Duration::from_secs(13)));
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.clock_time(start), Some(Duration::from_secs(13)));

        let next_time = start + Duration::from_secs(2);
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(11)));
        tm.stop_clock(next_time);
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(11)));

        let next_time = next_time + Duration::from_secs(3);
        tm.set_period_and_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(18));
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(18)));
        tm.start_clock(next_time);
        assert_eq!(tm.clock_is_running(), true);
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(18)));

        let next_time = next_time + Duration::from_secs(5);
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(23)));
        tm.stop_clock(next_time);
        assert_eq!(tm.clock_is_running(), false);
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(23)));
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

        tm.set_period_and_clock_time(start_period, Duration::from_secs(remaining));
        tm.set_game_start(game_start);
        assert_eq!(tm.clock_is_running(), false);
        tm.start_clock(start);
        assert_eq!(tm.clock_is_running(), true);
        if let Some((b, w)) = score {
            tm.set_scores(b, w, start);
        }
        tm.update(next_time);

        assert_eq!(tm.current_period(), end_period);
        assert_eq!(
            tm.clock_time(next_time),
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

        tm.set_period_and_clock_time(GamePeriod::BetweenGames, Duration::from_secs(1));
        tm.set_game_start(start);
        tm.start_clock(start);
        tm.update(next_time);

        assert_eq!(GamePeriod::FirstHalf, tm.current_period());
        assert_eq!(tm.clock_time(next_time), Some(Duration::from_secs(3)));
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

        tm.set_period_and_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(5));
        tm.set_game_start(game_start);
        tm.start_clock(start);
        tm.set_scores(2, 2, start);
        tm.update(second_time);

        assert_eq!(tm.current_period(), GamePeriod::SuddenDeath);
        assert_eq!(tm.clock_time(second_time), Some(Duration::from_secs(7)));

        let tm_clone = tm.clone();

        tm.set_scores(3, 2, third_time);
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(tm.clock_time(fourth_time), Some(Duration::from_secs(4)));

        let mut tm = tm_clone;
        let tm_clone = tm.clone();

        tm.add_b_score(1, third_time);
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(tm.clock_time(fourth_time), Some(Duration::from_secs(4)));

        let mut tm = tm_clone;

        tm.add_w_score(1, third_time);
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert_eq!(tm.clock_time(fourth_time), Some(Duration::from_secs(4)));
    }
}
