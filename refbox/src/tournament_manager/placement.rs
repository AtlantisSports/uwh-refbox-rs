//! Where a given wall-clock moment falls in one court's schedule.
//!
//! Pure arithmetic over data already in memory: no I/O, no clock, nothing to mock.
//!
//! Regulation periods only — first half, half time and second half, or the one period
//! of a single-period game. Overtime and sudden-death lengths depend on the score, and
//! a game nobody played has no score, so a game the catch-up skips or lands inside is
//! measured as regulation.

use std::time::Duration;
use time::OffsetDateTime;
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::GamePeriod,
    uwhportal::schedule::{GameNumber, TimingRule},
};

/// One scheduled game on the selected court, reduced to what placement needs.
///
/// `config` is deliberately not optional. A game whose timing rule cannot be resolved
/// is left out of the list entirely rather than given a default: default period lengths
/// would place the engine confidently at a position that does not exist.
///
/// `timing` is the same rule *before* conversion, kept so the engine can hand a proper
/// `NextGameInfo` to `start_game` — which adopts a game's configuration only from a
/// `TimingRule`. Without it, a catch-up that lands in a break cannot tell the engine
/// what the upcoming game is, and the engine goes on believing the pre-sleep next game
/// is next. `None` only in tests, which never exercise that path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScheduledGame {
    pub(crate) number: GameNumber,
    pub(crate) start_time: OffsetDateTime,
    pub(crate) config: GameConfig,
    pub(crate) timing: Option<TimingRule>,
}

/// Where a moment falls relative to a court's games. Indices are into the slice
/// passed to [`place`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Placement {
    /// Before the first game on the court.
    BeforeFirstGame { until_start: Duration },
    /// Inside a game, at `period` with `time_remaining` left in it.
    InGame {
        index: usize,
        period: GamePeriod,
        time_remaining: Duration,
    },
    /// After one game's regulation end and before the next one's start.
    InBreak {
        next_index: usize,
        until_start: Duration,
    },
    /// Past the end of the last game on the court — or the court has no games at all.
    PastLastGame,
}

/// Whether this calculation models `period` at all.
///
/// It models regulation only — the overtime and sudden-death lengths depend on the
/// score, and [`place`] works from the schedule, which knows nothing about scores.
/// Sudden death has no length at all: it counts up until somebody scores.
///
/// Lives here, beside [`period_at`], so that anyone extending the model has both
/// halves of the decision in front of them. A caller that acts on a placement for a
/// period this returns `false` for is asserting a position the arithmetic cannot
/// actually compute.
pub(crate) fn models_period(period: GamePeriod) -> bool {
    match period {
        GamePeriod::BetweenGames
        | GamePeriod::FirstHalf
        | GamePeriod::HalfTime
        | GamePeriod::SecondHalf => true,
        GamePeriod::PreOvertime
        | GamePeriod::OvertimeFirstHalf
        | GamePeriod::OvertimeHalfTime
        | GamePeriod::OvertimeSecondHalf
        | GamePeriod::PreSuddenDeath
        | GamePeriod::SuddenDeath => false,
    }
}

/// Which period of `config` a moment `into` a game falls in, and how much of it is left.
fn period_at(config: &GameConfig, into: Duration) -> (GamePeriod, Duration) {
    let first = GamePeriod::FirstHalf.duration(config).unwrap_or_default();

    if config.single_half {
        // A single-period game — a playoff or a final — has no half time and no second
        // half, and `GamePeriod::duration` does not know that: it answers for HalfTime
        // and SecondHalf whatever the config holds in those fields. Handing back a
        // period this game does not have would park the engine in a phantom half time,
        // and its expiry would advance to a phantom second half whose end posts a
        // result for a final that finished minutes ago.
        return (GamePeriod::FirstHalf, first.saturating_sub(into));
    }

    let half = GamePeriod::HalfTime.duration(config).unwrap_or_default();
    let second = GamePeriod::SecondHalf.duration(config).unwrap_or_default();

    if into < first {
        (GamePeriod::FirstHalf, first - into)
    } else if into < first + half {
        (GamePeriod::HalfTime, first + half - into)
    } else if into < first + half + second {
        (GamePeriod::SecondHalf, first + half + second - into)
    } else {
        // Only reachable if the caller asks about a moment past the game's end, which
        // `place` never does — it checks the end boundary first.
        (GamePeriod::SecondHalf, Duration::ZERO)
    }
}

/// Convert a signed wall-clock difference to a `Duration`, flooring at zero. Every
/// caller below has already established the sign, so the floor is belt-and-braces.
fn positive(delta: time::Duration) -> Duration {
    delta.try_into().unwrap_or_default()
}

/// Where `target` falls in `games`. `games` must be sorted by `start_time` ascending.
pub(crate) fn place(games: &[ScheduledGame], target: OffsetDateTime) -> Placement {
    let first = match games.first() {
        Some(game) => game,
        None => return Placement::PastLastGame,
    };

    if target < first.start_time {
        return Placement::BeforeFirstGame {
            until_start: positive(first.start_time - target),
        };
    }

    for (index, game) in games.iter().enumerate() {
        // `regulation_play` rather than summing the three period lengths here: it is
        // the one place that knows a single-period game is one period long, and a second
        // implementation of it measured such a game as more than twice its real length.
        let end = game.start_time + game.config.regulation_play();

        if target < end {
            let (period, time_remaining) =
                period_at(&game.config, positive(target - game.start_time));
            return Placement::InGame {
                index,
                period,
                time_remaining,
            };
        }

        if let Some(next) = games.get(index + 1) {
            if target < next.start_time {
                return Placement::InBreak {
                    next_index: index + 1,
                    until_start: positive(next.start_time - target),
                };
            }
        }
    }

    Placement::PastLastGame
}

#[cfg(test)]
mod test {
    use super::*;

    fn cfg() -> GameConfig {
        GameConfig {
            half_play_duration: Duration::from_secs(600),
            half_time_duration: Duration::from_secs(180),
            ..Default::default()
        }
    }

    fn t(secs: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + secs)
    }

    /// Two games on one court: game 1 at t=0, game 2 at t=1800.
    /// Each regulation game runs 600 + 180 + 600 = 1380s.
    fn games() -> Vec<ScheduledGame> {
        vec![
            ScheduledGame {
                number: "1".into(),
                start_time: t(0),
                config: cfg(),
                timing: None,
            },
            ScheduledGame {
                number: "2".into(),
                start_time: t(1800),
                config: cfg(),
                timing: None,
            },
        ]
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// A single-period playoff or final: one period, no half time, no second half.
    /// `half_time_duration` still carries a real number, which is exactly the trap —
    /// anything that sums the three period lengths measures this 10-minute game as 23
    /// minutes long.
    fn single_period_cfg() -> GameConfig {
        GameConfig {
            single_half: true,
            ..cfg()
        }
    }

    fn single_period_games() -> Vec<ScheduledGame> {
        vec![
            ScheduledGame {
                number: "F1".into(),
                start_time: t(0),
                config: single_period_cfg(),
                timing: None,
            },
            ScheduledGame {
                number: "F2".into(),
                start_time: t(1800),
                config: single_period_cfg(),
                timing: None,
            },
        ]
    }

    /// The whole point of C1: a single-period game is 600s long, not 1380s. Measured
    /// as 1380s, a wake 700s after kickoff lands inside a final that finished nearly
    /// two minutes ago — and the real next game is never counted down to.
    #[test]
    fn a_single_period_game_ends_after_its_one_period() {
        use Placement::*;

        let cases: &[(i64, Placement, &str)] = &[
            (
                100,
                InGame {
                    index: 0,
                    period: GamePeriod::FirstHalf,
                    time_remaining: secs(500),
                },
                "inside the one period",
            ),
            (
                599,
                InGame {
                    index: 0,
                    period: GamePeriod::FirstHalf,
                    time_remaining: secs(1),
                },
                "the last second of the one period",
            ),
            (
                600,
                InBreak {
                    next_index: 1,
                    until_start: secs(1200),
                },
                "one second past the end is the break, not a phantom half time",
            ),
            (
                700,
                InBreak {
                    next_index: 1,
                    until_start: secs(1100),
                },
                "well past the end is still the break",
            ),
            (
                2399,
                InGame {
                    index: 1,
                    period: GamePeriod::FirstHalf,
                    time_remaining: secs(1),
                },
                "the final second of the second single-period game",
            ),
            (
                2400,
                PastLastGame,
                "past the last single-period game, not still inside it",
            ),
        ];

        for (offset, expected, what) in cases {
            assert_eq!(
                place(&single_period_games(), t(*offset)),
                *expected,
                "{what}"
            );
        }
    }

    /// C2, asserted directly rather than through `place`: even asked about a moment
    /// the fixed `place` can no longer produce, `period_at` must never name a period a
    /// single-period game does not have. Forced into HalfTime the engine plays a
    /// phantom half time, then a phantom second half, and `end_second_half` posts that
    /// result to the portal.
    #[test]
    fn a_single_period_game_has_only_a_first_half() {
        for into in [0u64, 100, 599, 600, 700, 5000] {
            let (period, _) = period_at(&single_period_cfg(), secs(into));
            assert_eq!(
                period,
                GamePeriod::FirstHalf,
                "{into}s into a single-period game must still be the first half"
            );
        }

        assert_eq!(
            period_at(&single_period_cfg(), secs(100)),
            (GamePeriod::FirstHalf, secs(500)),
            "the whole regulation length belongs to the one period"
        );
    }

    #[test]
    fn every_position_in_the_schedule_places_correctly() {
        use GamePeriod::*;
        use Placement::*;

        let cases: &[(i64, Placement, &str)] = &[
            (
                -1200,
                BeforeFirstGame {
                    until_start: secs(1200),
                },
                "before the first game",
            ),
            (
                0,
                InGame {
                    index: 0,
                    period: FirstHalf,
                    time_remaining: secs(600),
                },
                "exactly on kickoff is inside the game, not before it",
            ),
            (
                100,
                InGame {
                    index: 0,
                    period: FirstHalf,
                    time_remaining: secs(500),
                },
                "first half",
            ),
            (
                700,
                InGame {
                    index: 0,
                    period: HalfTime,
                    time_remaining: secs(80),
                },
                "half time",
            ),
            (
                1000,
                InGame {
                    index: 0,
                    period: SecondHalf,
                    time_remaining: secs(380),
                },
                "second half",
            ),
            (
                1500,
                InBreak {
                    next_index: 1,
                    until_start: secs(300),
                },
                "the break between games",
            ),
            (
                2680,
                InGame {
                    index: 1,
                    period: SecondHalf,
                    time_remaining: secs(500),
                },
                "second half of a later game",
            ),
            (99_999, PastLastGame, "past the last game"),
        ];

        for (offset, expected, what) in cases {
            assert_eq!(place(&games(), t(*offset)), *expected, "{what}");
        }
    }

    #[test]
    fn the_model_covers_exactly_the_periods_place_can_compute() {
        // If someone adds a period to `period_at`, this is the other half of the
        // change. Overtime and sudden death depend on the score, which the
        // schedule does not know.
        for period in [
            GamePeriod::BetweenGames,
            GamePeriod::FirstHalf,
            GamePeriod::HalfTime,
            GamePeriod::SecondHalf,
        ] {
            assert!(models_period(period), "{period:?} should be modelled");
        }
        for period in [
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::OvertimeSecondHalf,
            GamePeriod::PreSuddenDeath,
            GamePeriod::SuddenDeath,
        ] {
            assert!(!models_period(period), "{period:?} must NOT be modelled");
        }
    }

    #[test]
    fn an_empty_schedule_is_past_the_last_game() {
        assert_eq!(place(&[], t(0)), Placement::PastLastGame);
    }
}
