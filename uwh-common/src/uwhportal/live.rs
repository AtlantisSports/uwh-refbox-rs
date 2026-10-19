//! The live state of a game in progress, as the refbox sends it to the portal.

use crate::{
    color::Color,
    game_snapshot::{GamePeriod, TimeoutSnapshot},
};
use core::num::NonZeroU16;
use serde::{Serialize, Serializer, ser::SerializeStruct};

/// One live update for one game: the body of
/// `PUT /api/events/{eventId}/live-games/{gameNumber}`.
///
/// Serialises as one JSON object with all nine keys always present, in this
/// order: `period`, `clockSeconds`, `clockRunning`, `timeout`,
/// `timeoutSeconds`, `timeoutRunning`, `scores`, `lastGoal`, `penalties`.
/// An absent value is sent as `null`; the portal expects every key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveGameState {
    /// Serialises as the bare variant name, e.g. `"SecondHalf"`.
    pub period: GamePeriod,
    pub clock_seconds: u32,
    pub clock_running: bool,
    /// Serialises as `timeout` (the bare kind name) plus `timeoutSeconds` and
    /// `timeoutRunning`, which come from a team timeout's clock only.
    pub timeout: Option<LiveTimeout>,
    pub scores: Option<LiveScores>,
    pub last_goal: Option<LiveGoal>,
    /// `Some(vec![])` is sent as `[]`, which is not the same as `null`.
    pub penalties: Option<Vec<LivePenalty>>,
}

impl Serialize for LiveGameState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (kind, clock) = match self.timeout {
            None => (None, None),
            Some(LiveTimeout::White(clock)) => (Some("White"), clock),
            Some(LiveTimeout::Black(clock)) => (Some("Black"), clock),
            Some(LiveTimeout::Ref) => (Some("Ref"), None),
            Some(LiveTimeout::PenaltyShot) => (Some("PenaltyShot"), None),
        };
        let mut s = serializer.serialize_struct("LiveGameState", 9)?;
        s.serialize_field("period", &self.period)?;
        s.serialize_field("clockSeconds", &self.clock_seconds)?;
        s.serialize_field("clockRunning", &self.clock_running)?;
        s.serialize_field("timeout", &kind)?;
        s.serialize_field("timeoutSeconds", &clock.map(|c| c.seconds_left))?;
        s.serialize_field("timeoutRunning", &clock.map(|c| c.running))?;
        s.serialize_field("scores", &self.scores)?;
        s.serialize_field("lastGoal", &self.last_goal)?;
        s.serialize_field("penalties", &self.penalties)?;
        s.end()
    }
}

/// Which kind of timeout is running. A team timeout may carry its own clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveTimeout {
    White(Option<TeamTimeoutClock>),
    Black(Option<TeamTimeoutClock>),
    Ref,
    PenaltyShot,
}

impl LiveTimeout {
    /// `team_clock_running` is used only for a White/Black timeout: `Some(r)` gives
    /// `Some(TeamTimeoutClock { seconds_left: <snapshot secs>, running: r })`, `None` gives no clock.
    pub fn from_snapshot(t: TimeoutSnapshot, team_clock_running: Option<bool>) -> Self {
        let clock = |seconds_left| {
            team_clock_running.map(|running| TeamTimeoutClock {
                seconds_left,
                running,
            })
        };
        match t {
            TimeoutSnapshot::White(secs) => Self::White(clock(secs)),
            TimeoutSnapshot::Black(secs) => Self::Black(clock(secs)),
            TimeoutSnapshot::Ref(_) => Self::Ref,
            TimeoutSnapshot::PenaltyShot(_) => Self::PenaltyShot,
        }
    }
}

/// The clock of a team timeout: sent as `timeoutSeconds` and `timeoutRunning`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamTimeoutClock {
    pub seconds_left: u16,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LiveScores {
    pub black: u8,
    pub white: u8,
}

/// The most recent goal: who scored it and where the game clock stood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveGoal {
    pub team: Color,
    pub cap: u8,
    pub clock_seconds: u32,
}

/// One penalty being served. Serialises as
/// `{"team","cap","kind","secondsLeft"}`; a total dismissal sends
/// `"kind":"TotalDismissal"` with `"secondsLeft":null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivePenalty {
    pub team: Color,
    pub cap: u8,
    pub time: LivePenaltyTime,
}

impl Serialize for LivePenalty {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("LivePenalty", 4)?;
        s.serialize_field("team", &self.team)?;
        s.serialize_field("cap", &self.cap)?;
        match self.time {
            LivePenaltyTime::Timed {
                length,
                seconds_left,
            } => {
                s.serialize_field("kind", &length)?;
                s.serialize_field("secondsLeft", &seconds_left.get())?;
            }
            LivePenaltyTime::TotalDismissal => {
                s.serialize_field("kind", "TotalDismissal")?;
                s.serialize_field("secondsLeft", &None::<u16>)?;
            }
        }
        s.end()
    }
}

/// How much of a penalty is left. A timed penalty's `seconds_left` is never
/// zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivePenaltyTime {
    Timed {
        length: TimedPenaltyLength,
        seconds_left: NonZeroU16,
    },
    TotalDismissal,
}

/// The length a timed penalty was given. Serialises as the variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TimedPenaltyLength {
    ThirtySecond,
    OneMinute,
    TwoMinute,
    FourMinute,
    FiveMinute,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(secs: u16) -> NonZeroU16 {
        NonZeroU16::new(secs).unwrap()
    }

    fn body(state: &LiveGameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    fn base(timeout: Option<LiveTimeout>) -> LiveGameState {
        LiveGameState {
            period: GamePeriod::FirstHalf,
            clock_seconds: 300,
            clock_running: false,
            timeout,
            scores: None,
            last_goal: None,
            penalties: None,
        }
    }

    #[test]
    fn agreement_example_body_is_exact() {
        let state = LiveGameState {
            period: GamePeriod::SecondHalf,
            clock_seconds: 462,
            clock_running: false,
            timeout: Some(LiveTimeout::White(Some(TeamTimeoutClock {
                seconds_left: 41,
                running: true,
            }))),
            scores: Some(LiveScores { black: 1, white: 3 }),
            last_goal: Some(LiveGoal {
                team: Color::White,
                cap: 7,
                clock_seconds: 470,
            }),
            penalties: Some(vec![
                LivePenalty {
                    team: Color::Black,
                    cap: 4,
                    time: LivePenaltyTime::Timed {
                        length: TimedPenaltyLength::TwoMinute,
                        seconds_left: nz(83),
                    },
                },
                LivePenalty {
                    team: Color::White,
                    cap: 11,
                    time: LivePenaltyTime::TotalDismissal,
                },
            ]),
        };
        assert_eq!(
            body(&state),
            r#"{"period":"SecondHalf","clockSeconds":462,"clockRunning":false,"timeout":"White","timeoutSeconds":41,"timeoutRunning":true,"scores":{"black":1,"white":3},"lastGoal":{"team":"White","cap":7,"clockSeconds":470},"penalties":[{"team":"Black","cap":4,"kind":"TwoMinute","secondsLeft":83},{"team":"White","cap":11,"kind":"TotalDismissal","secondsLeft":null}]}"#
        );
    }

    #[test]
    fn in_play_body_is_exact() {
        let state = LiveGameState {
            period: GamePeriod::SecondHalf,
            clock_seconds: 462,
            clock_running: true,
            timeout: None,
            scores: Some(LiveScores { black: 1, white: 3 }),
            last_goal: None,
            penalties: None,
        };
        assert_eq!(
            body(&state),
            r#"{"period":"SecondHalf","clockSeconds":462,"clockRunning":true,"timeout":null,"timeoutSeconds":null,"timeoutRunning":null,"scores":{"black":1,"white":3},"lastGoal":null,"penalties":null}"#
        );
    }

    #[test]
    fn between_games_body_serialises_nulls() {
        let state = LiveGameState {
            period: GamePeriod::BetweenGames,
            clock_seconds: 120,
            clock_running: true,
            timeout: Some(LiveTimeout::Ref),
            scores: None,
            last_goal: None,
            penalties: None,
        };
        let body = body(&state);
        assert!(body.contains(r#""timeout":"Ref""#), "{body}");
        assert!(body.contains(r#""scores":null"#), "{body}");
    }

    #[test]
    fn each_penalty_kind_serialises_its_name() {
        let timed = |length| LivePenaltyTime::Timed {
            length,
            seconds_left: nz(5),
        };
        let cases = [
            (timed(TimedPenaltyLength::ThirtySecond), "ThirtySecond"),
            (timed(TimedPenaltyLength::OneMinute), "OneMinute"),
            (timed(TimedPenaltyLength::TwoMinute), "TwoMinute"),
            (timed(TimedPenaltyLength::FourMinute), "FourMinute"),
            (timed(TimedPenaltyLength::FiveMinute), "FiveMinute"),
            (LivePenaltyTime::TotalDismissal, "TotalDismissal"),
        ];
        for (time, name) in cases {
            let is_td = time == LivePenaltyTime::TotalDismissal;
            let penalty = LivePenalty {
                team: Color::Black,
                cap: 2,
                time,
            };
            let left = if is_td { "null" } else { "5" };
            assert_eq!(
                serde_json::to_string(&penalty).unwrap(),
                format!(r#"{{"team":"Black","cap":2,"kind":"{name}","secondsLeft":{left}}}"#)
            );
        }
    }

    #[test]
    fn ref_and_penalty_shot_carry_no_timeout_seconds() {
        for timeout in [None, Some(LiveTimeout::Ref), Some(LiveTimeout::PenaltyShot)] {
            let body = body(&base(timeout));
            assert!(body.contains(r#""timeoutSeconds":null"#), "{body}");
            assert!(body.contains(r#""timeoutRunning":null"#), "{body}");
        }
    }

    #[test]
    fn team_timeout_without_clock_sends_both_null() {
        for timeout in [LiveTimeout::White(None), LiveTimeout::Black(None)] {
            let body = body(&base(Some(timeout)));
            assert!(body.contains(r#""timeoutSeconds":null"#), "{body}");
            assert!(body.contains(r#""timeoutRunning":null"#), "{body}");
        }
    }

    #[test]
    fn team_timeout_at_zero_stopped_is_sent() {
        let body = body(&base(Some(LiveTimeout::Black(Some(TeamTimeoutClock {
            seconds_left: 0,
            running: false,
        })))));
        assert!(
            body.contains(r#""timeout":"Black","timeoutSeconds":0,"timeoutRunning":false"#),
            "{body}"
        );
    }

    #[test]
    fn empty_penalty_list_serialises_as_array() {
        let state = LiveGameState {
            penalties: Some(vec![]),
            ..base(None)
        };
        let body = body(&state);
        assert!(body.contains(r#""penalties":[]"#), "{body}");
    }

    #[test]
    fn from_snapshot_fills_team_clock_only() {
        assert_eq!(
            LiveTimeout::from_snapshot(TimeoutSnapshot::White(10), Some(true)),
            LiveTimeout::White(Some(TeamTimeoutClock {
                seconds_left: 10,
                running: true,
            }))
        );
        assert_eq!(
            LiveTimeout::from_snapshot(TimeoutSnapshot::Black(20), None),
            LiveTimeout::Black(None)
        );
        assert_eq!(
            LiveTimeout::from_snapshot(TimeoutSnapshot::Ref(5), Some(true)),
            LiveTimeout::Ref
        );
        assert_eq!(
            LiveTimeout::from_snapshot(TimeoutSnapshot::PenaltyShot(37), None),
            LiveTimeout::PenaltyShot
        );
    }
}
