//! The live state of a game in progress, as the refbox sends it to the portal.

use crate::game_snapshot::{GamePeriod, TimeoutSnapshot};
use serde::Serialize;

/// One live update for one game: the body of
/// `PUT /api/events/{eventId}/live-games/{gameNumber}`.
///
/// `timeout` and `scores` serialise as `null` when absent; the portal expects
/// both keys to be present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveGameState {
    /// Serialises as the bare variant name, e.g. `"SecondHalf"`.
    pub period: GamePeriod,
    pub clock_seconds: u32,
    pub clock_running: bool,
    pub timeout: Option<LiveTimeout>,
    pub scores: Option<LiveScores>,
}

/// Which kind of timeout is running, without its remaining time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LiveTimeout {
    White,
    Black,
    Ref,
    PenaltyShot,
}

impl From<TimeoutSnapshot> for LiveTimeout {
    fn from(timeout: TimeoutSnapshot) -> Self {
        match timeout {
            TimeoutSnapshot::White(_) => Self::White,
            TimeoutSnapshot::Black(_) => Self::Black,
            TimeoutSnapshot::Ref(_) => Self::Ref,
            TimeoutSnapshot::PenaltyShot(_) => Self::PenaltyShot,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LiveScores {
    pub black: u8,
    pub white: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_play_body_is_exact() {
        let state = LiveGameState {
            period: GamePeriod::SecondHalf,
            clock_seconds: 462,
            clock_running: true,
            timeout: None,
            scores: Some(LiveScores { black: 1, white: 3 }),
        };
        assert_eq!(
            serde_json::to_string(&state).unwrap(),
            r#"{"period":"SecondHalf","clockSeconds":462,"clockRunning":true,"timeout":null,"scores":{"black":1,"white":3}}"#
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
        };
        let body = serde_json::to_string(&state).unwrap();
        assert!(body.contains(r#""timeout":"Ref""#), "{body}");
        assert!(body.contains(r#""scores":null"#), "{body}");
    }

    #[test]
    fn timeout_kind_drops_seconds() {
        assert_eq!(
            LiveTimeout::from(TimeoutSnapshot::White(10)),
            LiveTimeout::White
        );
        assert_eq!(
            LiveTimeout::from(TimeoutSnapshot::Black(20)),
            LiveTimeout::Black
        );
        assert_eq!(LiveTimeout::from(TimeoutSnapshot::Ref(0)), LiveTimeout::Ref);
        assert_eq!(
            LiveTimeout::from(TimeoutSnapshot::PenaltyShot(37)),
            LiveTimeout::PenaltyShot
        );
    }
}
