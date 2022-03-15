#[cfg(feature = "std")]
use crate::config::Game;
use arrayvec::ArrayVec;
use core::{
    cmp::{Ordering, PartialOrd},
    time::Duration,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GameSnapshotNoHeap {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutSnapshot,
    pub b_score: u8,
    pub w_score: u8,
    pub b_penalties: ArrayVec<PenaltySnapshot, 3>,
    pub w_penalties: ArrayVec<PenaltySnapshot, 3>,
}

#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GameSnapshot {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutSnapshot,
    pub b_score: u8,
    pub w_score: u8,
    pub b_penalties: Vec<PenaltySnapshot>,
    pub w_penalties: Vec<PenaltySnapshot>,
}

#[cfg(feature = "std")]
impl From<GameSnapshot> for GameSnapshotNoHeap {
    fn from(mut snapshot: GameSnapshot) -> Self {
        snapshot.b_penalties.sort_by(|a, b| a.time.cmp(&b.time));
        snapshot.w_penalties.sort_by(|a, b| a.time.cmp(&b.time));
        Self {
            current_period: snapshot.current_period,
            secs_in_period: snapshot.secs_in_period,
            timeout: snapshot.timeout,
            b_score: snapshot.b_score,
            w_score: snapshot.w_score,
            b_penalties: snapshot.b_penalties.into_iter().take(3).collect(),
            w_penalties: snapshot.w_penalties.into_iter().take(3).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PenaltySnapshot {
    pub player_number: u8,
    pub time: PenaltyTime,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum GamePeriod {
    BetweenGames,
    FirstHalf,
    HalfTime,
    SecondHalf,
    PreOvertime,
    OvertimeFirstHalf,
    OvertimeHalfTime,
    OvertimeSecondHalf,
    PreSuddenDeath,
    SuddenDeath,
}

impl GamePeriod {
    #[cfg(feature = "std")]
    pub fn penalties_run(self, config: &Game) -> bool {
        match self {
            Self::BetweenGames
            | Self::HalfTime
            | Self::PreOvertime
            | Self::OvertimeHalfTime
            | Self::PreSuddenDeath => false,
            Self::FirstHalf | Self::SecondHalf => true,
            Self::OvertimeFirstHalf | Self::OvertimeSecondHalf => config.has_overtime,
            Self::SuddenDeath => config.sudden_death_allowed,
        }
    }

    #[cfg(feature = "std")]
    pub fn duration(self, config: &Game) -> Option<Duration> {
        match self {
            Self::BetweenGames | Self::SuddenDeath => None,
            Self::FirstHalf | Self::SecondHalf => {
                Some(Duration::from_secs(config.half_play_duration.into()))
            }
            Self::HalfTime => Some(Duration::from_secs(config.half_time_duration.into())),
            Self::PreOvertime => Some(Duration::from_secs(config.pre_overtime_break.into())),
            Self::OvertimeFirstHalf | Self::OvertimeSecondHalf => {
                Some(Duration::from_secs(config.ot_half_play_duration.into()))
            }
            Self::OvertimeHalfTime => {
                Some(Duration::from_secs(config.ot_half_time_duration.into()))
            }
            Self::PreSuddenDeath => {
                Some(Duration::from_secs(config.pre_sudden_death_duration.into()))
            }
        }
    }

    #[cfg(feature = "std")]
    pub fn time_elapsed_at(self, time: Duration, config: &Game) -> Option<Duration> {
        match self {
            p @ Self::BetweenGames
            | p @ Self::FirstHalf
            | p @ Self::HalfTime
            | p @ Self::SecondHalf
            | p @ Self::PreOvertime
            | p @ Self::OvertimeFirstHalf
            | p @ Self::OvertimeHalfTime
            | p @ Self::OvertimeSecondHalf
            | p @ Self::PreSuddenDeath => p.duration(config).and_then(|d| d.checked_sub(time)),
            Self::SuddenDeath => Some(time),
        }
    }

    pub fn time_between(self, start: Duration, end: Duration) -> Option<Duration> {
        match self {
            Self::BetweenGames
            | Self::FirstHalf
            | Self::HalfTime
            | Self::SecondHalf
            | Self::PreOvertime
            | Self::OvertimeFirstHalf
            | Self::OvertimeHalfTime
            | Self::OvertimeSecondHalf
            | Self::PreSuddenDeath => start.checked_sub(end),
            Self::SuddenDeath => end.checked_sub(start),
        }
    }

    pub fn next_period(self) -> Option<GamePeriod> {
        match self {
            Self::BetweenGames => Some(Self::FirstHalf),
            Self::FirstHalf => Some(Self::HalfTime),
            Self::HalfTime => Some(Self::SecondHalf),
            Self::SecondHalf => Some(Self::PreOvertime),
            Self::PreOvertime => Some(Self::OvertimeFirstHalf),
            Self::OvertimeFirstHalf => Some(Self::OvertimeHalfTime),
            Self::OvertimeHalfTime => Some(Self::OvertimeSecondHalf),
            Self::OvertimeSecondHalf => Some(Self::PreSuddenDeath),
            Self::PreSuddenDeath => Some(Self::SuddenDeath),
            Self::SuddenDeath => None,
        }
    }
}

impl core::fmt::Display for GamePeriod {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            GamePeriod::BetweenGames => write!(f, "Between Games"),
            GamePeriod::FirstHalf => write!(f, "First Half"),
            GamePeriod::HalfTime => write!(f, "Half Time"),
            GamePeriod::SecondHalf => write!(f, "Second Half"),
            GamePeriod::PreOvertime => write!(f, "Pre Overtime"),
            GamePeriod::OvertimeFirstHalf => write!(f, "Overtime First Half"),
            GamePeriod::OvertimeHalfTime => write!(f, "Overtime Half Time"),
            GamePeriod::OvertimeSecondHalf => write!(f, "Overtime Second Half"),
            GamePeriod::PreSuddenDeath => write!(f, "Pre Sudden Death"),
            GamePeriod::SuddenDeath => write!(f, "Sudden Death"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimeoutSnapshot {
    None,
    White(u16),
    Black(u16),
    Ref(u16),
    PenaltyShot(u16),
}

impl core::fmt::Display for TimeoutSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            TimeoutSnapshot::None => write!(f, "No Timeout"),
            TimeoutSnapshot::Black(_) => write!(f, "Black Timeout"),
            TimeoutSnapshot::White(_) => write!(f, "White Timeout"),
            TimeoutSnapshot::Ref(_) => write!(f, "Ref Timeout"),
            TimeoutSnapshot::PenaltyShot(_) => write!(f, "PenaltyShot"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Color {
    Black,
    White,
}

impl core::fmt::Display for Color {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            Self::Black => write!(f, "Black"),
            Self::White => write!(f, "White"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PenaltyTime {
    Seconds(u16),
    TotalDismissal,
}

impl Ord for PenaltyTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            PenaltyTime::TotalDismissal => match other {
                PenaltyTime::TotalDismissal => Ordering::Equal,
                PenaltyTime::Seconds(_) => Ordering::Greater,
            },
            PenaltyTime::Seconds(mine) => match other {
                PenaltyTime::Seconds(theirs) => mine.cmp(theirs),
                PenaltyTime::TotalDismissal => Ordering::Less,
            },
        }
    }
}

impl PartialOrd for PenaltyTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_penalty_time_ord() {
        assert!(PenaltyTime::Seconds(5) > PenaltyTime::Seconds(0));
        assert!(PenaltyTime::Seconds(5) < PenaltyTime::Seconds(9));
        assert!(PenaltyTime::TotalDismissal > PenaltyTime::Seconds(13));
        assert!(PenaltyTime::Seconds(10_000) < PenaltyTime::TotalDismissal);
        assert_eq!(PenaltyTime::Seconds(10), PenaltyTime::Seconds(10));
        assert_eq!(PenaltyTime::TotalDismissal, PenaltyTime::TotalDismissal);
    }

    #[test]
    fn test_period_penalties_run() {
        let all_periods_config = Game {
            has_overtime: true,
            sudden_death_allowed: true,
            ..Default::default()
        };
        let sd_only_config = Game {
            has_overtime: false,
            sudden_death_allowed: true,
            ..Default::default()
        };
        let no_sd_no_ot_config = Game {
            has_overtime: false,
            sudden_death_allowed: false,
            ..Default::default()
        };

        assert_eq!(
            GamePeriod::BetweenGames.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::FirstHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::HalfTime.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::SecondHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::PreOvertime.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::SuddenDeath.penalties_run(&all_periods_config),
            true
        );

        assert_eq!(
            GamePeriod::BetweenGames.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(GamePeriod::FirstHalf.penalties_run(&sd_only_config), true);
        assert_eq!(GamePeriod::HalfTime.penalties_run(&sd_only_config), false);
        assert_eq!(GamePeriod::SecondHalf.penalties_run(&sd_only_config), true);
        assert_eq!(
            GamePeriod::PreOvertime.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(GamePeriod::SuddenDeath.penalties_run(&sd_only_config), true);

        assert_eq!(
            GamePeriod::BetweenGames.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::FirstHalf.penalties_run(&no_sd_no_ot_config),
            true
        );
        assert_eq!(
            GamePeriod::HalfTime.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::SecondHalf.penalties_run(&no_sd_no_ot_config),
            true
        );
        assert_eq!(
            GamePeriod::PreOvertime.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::SuddenDeath.penalties_run(&no_sd_no_ot_config),
            false
        );
    }

    #[test]
    fn test_period_duration() {
        let config = Game {
            half_play_duration: 5,
            half_time_duration: 7,
            pre_overtime_break: 9,
            ot_half_play_duration: 11,
            ot_half_time_duration: 13,
            pre_sudden_death_duration: 15,
            ..Default::default()
        };

        assert_eq!(GamePeriod::BetweenGames.duration(&config), None);
        assert_eq!(
            GamePeriod::FirstHalf.duration(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::HalfTime.duration(&config),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            GamePeriod::SecondHalf.duration(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::PreOvertime.duration(&config),
            Some(Duration::from_secs(9))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.duration(&config),
            Some(Duration::from_secs(11))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.duration(&config),
            Some(Duration::from_secs(13))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.duration(&config),
            Some(Duration::from_secs(11))
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.duration(&config),
            Some(Duration::from_secs(15))
        );
        assert_eq!(GamePeriod::SuddenDeath.duration(&config), None);
    }

    #[test]
    fn test_period_time_elapsed_at() {
        let config = Game {
            half_play_duration: 5,
            half_time_duration: 7,
            pre_overtime_break: 9,
            ot_half_play_duration: 11,
            ot_half_time_duration: 13,
            pre_sudden_death_duration: 15,
            ..Default::default()
        };

        assert_eq!(
            GamePeriod::BetweenGames.time_elapsed_at(Duration::from_secs(5), &config),
            None
        );
        assert_eq!(
            GamePeriod::FirstHalf.time_elapsed_at(Duration::from_secs(3), &config),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            GamePeriod::HalfTime.time_elapsed_at(Duration::from_secs(4), &config),
            Some(Duration::from_secs(3))
        );
        assert_eq!(
            GamePeriod::SecondHalf.time_elapsed_at(Duration::from_secs(3), &config),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            GamePeriod::PreOvertime.time_elapsed_at(Duration::from_secs(4), &config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.time_elapsed_at(Duration::from_secs(7), &config),
            Some(Duration::from_secs(4))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.time_elapsed_at(Duration::from_secs(8), &config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.time_elapsed_at(Duration::from_secs(7), &config),
            Some(Duration::from_secs(4))
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.time_elapsed_at(Duration::from_secs(9), &config),
            Some(Duration::from_secs(6))
        );
        assert_eq!(
            GamePeriod::SuddenDeath.time_elapsed_at(Duration::from_secs(3), &config),
            Some(Duration::from_secs(3))
        );

        assert_eq!(
            GamePeriod::FirstHalf.time_elapsed_at(Duration::from_secs(9), &config),
            None
        );
        assert_eq!(
            GamePeriod::HalfTime.time_elapsed_at(Duration::from_secs(9), &config),
            None
        );
        assert_eq!(
            GamePeriod::SecondHalf.time_elapsed_at(Duration::from_secs(9), &config),
            None
        );
        assert_eq!(
            GamePeriod::PreOvertime.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
    }

    #[test]
    fn test_period_time_between() {
        let mut period = GamePeriod::BetweenGames;
        while period != GamePeriod::SuddenDeath {
            assert_eq!(
                period.time_between(Duration::from_secs(6), Duration::from_secs(2)),
                Some(Duration::from_secs(4))
            );
            assert_eq!(
                period.time_between(Duration::from_secs(6), Duration::from_secs(10)),
                None
            );
            period = period.next_period().unwrap();
        }
        assert_eq!(
            GamePeriod::SuddenDeath.time_between(Duration::from_secs(6), Duration::from_secs(2)),
            None
        );
        assert_eq!(
            GamePeriod::SuddenDeath.time_between(Duration::from_secs(6), Duration::from_secs(10)),
            Some(Duration::from_secs(4))
        );
    }

    #[test]
    fn test_next_period() {
        assert_eq!(
            GamePeriod::BetweenGames.next_period(),
            Some(GamePeriod::FirstHalf)
        );
        assert_eq!(
            GamePeriod::FirstHalf.next_period(),
            Some(GamePeriod::HalfTime)
        );
        assert_eq!(
            GamePeriod::HalfTime.next_period(),
            Some(GamePeriod::SecondHalf)
        );
        assert_eq!(
            GamePeriod::SecondHalf.next_period(),
            Some(GamePeriod::PreOvertime)
        );
        assert_eq!(
            GamePeriod::PreOvertime.next_period(),
            Some(GamePeriod::OvertimeFirstHalf)
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.next_period(),
            Some(GamePeriod::OvertimeHalfTime)
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.next_period(),
            Some(GamePeriod::OvertimeSecondHalf)
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.next_period(),
            Some(GamePeriod::PreSuddenDeath)
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.next_period(),
            Some(GamePeriod::SuddenDeath)
        );
        assert_eq!(GamePeriod::SuddenDeath.next_period(), None);
    }
}
