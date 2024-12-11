use derivative::Derivative;
use log::*;
use std::{cmp::Ordering, convert::TryInto};
use thiserror::Error;
use time::Duration as SignedDuration;
use tokio::time::{Duration, Instant};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, Infraction, PenaltySnapshot, PenaltyTime},
};

#[derive(Derivative)]
#[derivative(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyKind {
    ThirtySecond,
    #[derivative(Default)]
    OneMinute,
    TwoMinute,
    FourMinute,
    FiveMinute,
    TotalDismissal,
}

impl PenaltyKind {
    pub(crate) fn as_duration(self) -> Option<Duration> {
        match self {
            Self::ThirtySecond => Some(Duration::from_secs(30)),
            Self::OneMinute => Some(Duration::from_secs(60)),
            Self::TwoMinute => Some(Duration::from_secs(120)),
            Self::FourMinute => Some(Duration::from_secs(240)),
            Self::FiveMinute => Some(Duration::from_secs(300)),
            Self::TotalDismissal => None,
        }
    }

    pub fn fluent(&self) -> &'static str {
        match self {
            Self::ThirtySecond => "thirty-seconds",
            Self::OneMinute => "one-minute",
            Self::TwoMinute => "two-minutes",
            Self::FourMinute => "four-minutes",
            Self::FiveMinute => "five-minutes",
            Self::TotalDismissal => "total-dismissal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Penalty {
    pub(crate) kind: PenaltyKind,
    pub(crate) player_number: u8,
    pub(crate) start_period: GamePeriod,
    pub(crate) start_time: Duration,
    pub(crate) start_instant: Instant,
    pub(crate) infraction: Infraction,
}

impl Penalty {
    pub fn time_elapsed(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> PenaltyResult<SignedDuration> {
        let calc_time_between = |earlier_period: GamePeriod,
                                 earlier_time: Duration,
                                 later_period: GamePeriod,
                                 later_time: Duration| {
            let mut elapsed = if earlier_period.penalties_run(config) {
                earlier_time.try_into()?
            } else {
                SignedDuration::ZERO
            };
            let mut period = earlier_period.next_period().unwrap();
            while period < later_period {
                if period.penalties_run(config) {
                    elapsed += period.duration(config).unwrap();
                }
                period = period.next_period().unwrap();
            }
            if later_period.penalties_run(config) {
                elapsed += later_period
                    .time_elapsed_at(later_time, config)
                    .ok_or(time::error::ConversionRange)?; // Because we know the period must have a duration
            }
            Ok(elapsed)
        };

        match cur_per.cmp(&self.start_period) {
            Ordering::Equal => {
                if cur_per.penalties_run(config) {
                    Ok(cur_per.time_between(self.start_time.try_into()?, cur_time.try_into()?))
                } else {
                    Ok(SignedDuration::ZERO)
                }
            }
            Ordering::Greater => {
                calc_time_between(self.start_period, self.start_time, cur_per, cur_time)
            }
            Ordering::Less => {
                calc_time_between(cur_per, cur_time, self.start_period, self.start_time).map(|d| -d)
            }
        }
    }

    pub fn time_remaining(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> PenaltyResult<SignedDuration> {
        let elapsed = self.time_elapsed(cur_per, cur_time, config);

        if cur_per == GamePeriod::BetweenGames && self.start_period != GamePeriod::BetweenGames {
            // In this case, the game in which the penalty started has completed, and we
            // are counting down to the next game. By definition, any penalties have been
            // served in this situation.
            Ok(SignedDuration::ZERO)
        } else {
            // In all other cases we do the normal calculation and return `None` if the
            // penalty is a TD or an error occurred
            let duration: SignedDuration = self
                .kind
                .as_duration()
                .ok_or(PenaltyError::NoDuration)?
                .try_into()?;

            duration
                .checked_sub(elapsed?)
                .ok_or(PenaltyError::DurationOverflow)
        }
    }

    pub fn is_complete(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> PenaltyResult<bool> {
        match self.kind {
            PenaltyKind::TotalDismissal => Ok(false),
            PenaltyKind::ThirtySecond
            | PenaltyKind::OneMinute
            | PenaltyKind::TwoMinute
            | PenaltyKind::FourMinute
            | PenaltyKind::FiveMinute => self
                .time_remaining(cur_per, cur_time, config)
                .map(|rem| rem <= SignedDuration::ZERO),
        }
    }

    pub fn as_snapshot(
        &self,
        cur_per: GamePeriod,
        cur_time: Duration,
        config: &GameConfig,
    ) -> PenaltyResult<PenaltySnapshot> {
        let time = match self.time_remaining(cur_per, cur_time, config) {
            Ok(dur) => {
                if dur.is_negative() {
                    PenaltyTime::Seconds(0)
                } else {
                    PenaltyTime::Seconds(dur.whole_seconds().try_into()?)
                }
            }
            Err(PenaltyError::NoDuration) => PenaltyTime::TotalDismissal,
            Err(e) => return Err(e),
        };

        Ok(PenaltySnapshot {
            player_number: self.player_number,
            time,
            infraction: self.infraction,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum PenaltyError {
    #[error("A std::time::Duration could not be converted to a time::Duration")]
    ConversionFailed(#[from] time::error::ConversionRange),
    #[error("Duration Overflow")]
    DurationOverflow,
    #[error("A penalty snapshot overflowed the maximum value of a u16")]
    SnapshotOverflow(#[from] core::num::TryFromIntError),
    #[error("A Total Dismissal penalty does not have a duration")]
    NoDuration,
}

pub type PenaltyResult<T> = std::result::Result<T, PenaltyError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PenaltyTimePrintable {
    Pending,
    Served,
    TotalDismissal,
    Remaining(i64),
}

impl PenaltyTimePrintable {
    pub fn fluent(&self) -> String {
        match self {
            Self::Pending => "pending".to_string(),
            Self::Served => "served".to_string(),
            Self::TotalDismissal => "total-dismissal".to_string(),
            Self::Remaining(secs) => {
                format!("{}:{:02}", secs / 60, secs % 60)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::test::initialize;
    use super::*;

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
                Ok(SignedDuration::seconds(2)),
                "Both first half",
            ),
            (
                GamePeriod::OvertimeFirstHalf,
                Duration::from_secs(10),
                GamePeriod::OvertimeFirstHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Ok(SignedDuration::seconds(8)),
                "Both overtime first half",
            ),
            (
                GamePeriod::SuddenDeath,
                Duration::from_secs(10),
                GamePeriod::SuddenDeath,
                Duration::from_secs(55),
                &all_periods_config,
                Ok(SignedDuration::seconds(45)),
                "Both sudden death",
            ),
            (
                GamePeriod::HalfTime,
                Duration::from_secs(4),
                GamePeriod::HalfTime,
                Duration::from_secs(2),
                &all_periods_config,
                Ok(SignedDuration::seconds(0)),
                "Both half time",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SecondHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Ok(SignedDuration::seconds(7)),
                "First half to second half",
            ),
            (
                GamePeriod::BetweenGames,
                Duration::from_secs(4),
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                &all_periods_config,
                Ok(SignedDuration::seconds(3)),
                "Between games to first half",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                &all_periods_config,
                Ok(SignedDuration::seconds(-2)),
                "Both first half, bad timing",
            ),
            (
                GamePeriod::HalfTime,
                Duration::from_secs(2),
                GamePeriod::HalfTime,
                Duration::from_secs(4),
                &all_periods_config,
                Ok(SignedDuration::seconds(0)),
                "Both half time, bad timing",
            ),
            (
                GamePeriod::HalfTime,
                Duration::from_secs(2),
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                &all_periods_config,
                Ok(SignedDuration::seconds(-4)),
                "Half time to first half",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SuddenDeath,
                Duration::from_secs(25),
                &all_periods_config,
                Ok(SignedDuration::seconds(56)),
                "First half to sudden death, all periods",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SuddenDeath,
                Duration::from_secs(25),
                &sd_only_config,
                Ok(SignedDuration::seconds(34)),
                "First half to sudden death, sudden death no overtime",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                GamePeriod::SuddenDeath,
                Duration::from_secs(25),
                &no_sd_no_ot_config,
                Ok(SignedDuration::seconds(9)),
                "First half to sudden death, no sudden death or overtime",
            ),
        ];

        for (start_period, start_time, end_period, end_time, config, result, msg) in test_cases {
            let penalty = Penalty {
                player_number: 0,
                kind: PenaltyKind::OneMinute,
                start_time,
                start_period,
                start_instant: Instant::now(),
                infraction: Infraction::Unknown,
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
                Ok(SignedDuration::seconds(58)),
                "Both first half, 1m",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::TwoMinute,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                Ok(SignedDuration::seconds(118)),
                "Both first half, 2m",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::FiveMinute,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                Ok(SignedDuration::seconds(298)),
                "Both first half, 5m",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(4),
                PenaltyKind::TotalDismissal,
                GamePeriod::FirstHalf,
                Duration::from_secs(2),
                Err(PenaltyError::NoDuration),
                "Both first half, TD",
            ),
            (
                GamePeriod::SuddenDeath,
                Duration::from_secs(5),
                PenaltyKind::OneMinute,
                GamePeriod::SuddenDeath,
                Duration::from_secs(70),
                Ok(SignedDuration::seconds(-5)),
                "Penalty Complete",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(5),
                PenaltyKind::OneMinute,
                GamePeriod::BetweenGames,
                Duration::from_secs(10),
                Ok(SignedDuration::seconds(0)),
                "Game Ended",
            ),
            (
                GamePeriod::FirstHalf,
                Duration::from_secs(5),
                PenaltyKind::TotalDismissal,
                GamePeriod::BetweenGames,
                Duration::from_secs(10),
                Ok(SignedDuration::seconds(0)),
                "Game Ended, TD",
            ),
        ];

        for (start_period, start_time, kind, end_period, end_time, result, msg) in test_cases {
            let penalty = Penalty {
                player_number: 0,
                kind,
                start_time,
                start_period,
                start_instant: Instant::now(),
                infraction: Infraction::Unknown,
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
            start_instant: Instant::now(),
            infraction: Infraction::Unknown,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(60), &config),
            Ok(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(65), &config),
            Ok(true)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(70), &config),
            Ok(true)
        );

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::TwoMinute,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
            start_instant: Instant::now(),
            infraction: Infraction::Unknown,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(120), &config),
            Ok(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(125), &config),
            Ok(true)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(130), &config),
            Ok(true)
        );

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::FiveMinute,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
            start_instant: Instant::now(),
            infraction: Infraction::Unknown,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(300), &config),
            Ok(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(305), &config),
            Ok(true)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(310), &config),
            Ok(true)
        );

        let penalty = Penalty {
            player_number: 0,
            kind: PenaltyKind::TotalDismissal,
            start_time: Duration::from_secs(5),
            start_period: GamePeriod::SuddenDeath,
            start_instant: Instant::now(),
            infraction: Infraction::Unknown,
        };
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(300), &config),
            Ok(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(305), &config),
            Ok(false)
        );
        assert_eq!(
            penalty.is_complete(GamePeriod::SuddenDeath, Duration::from_secs(310), &config),
            Ok(false)
        );
    }
}
