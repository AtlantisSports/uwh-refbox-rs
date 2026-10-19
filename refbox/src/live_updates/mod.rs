//! The rules that decide which live update, if any, this refbox sends to the
//! portal. Pure: no I/O here, only the decision.

pub mod sender;

use crate::config::{GameSource, Mode, effective_live_updates};
use uwh_common::{
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::{
        live::{LiveGameState, LiveScores, LiveTimeout},
        schedule::{EventId, GameNumber},
    },
};

/// One live update ready to send: where it goes and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveFrame {
    pub site: String,
    pub event: EventId,
    pub game: GameNumber,
    pub state: LiveGameState,
    /// Whether this frame is re-sent periodically while it stays current.
    /// False for the single frame that marks a finished court.
    pub heartbeat: bool,
}

/// Everything [`live_frame`] needs to decide.
pub struct LiveInputs<'a> {
    pub stored_enabled: bool,
    pub mode: Mode,
    pub source: GameSource,
    pub site: &'a str,
    pub event: Option<&'a EventId>,
    pub snapshot: &'a GameSnapshot,
    /// The game clock alone -- never the timeout clock. See
    /// `TournamentManager::game_clock_running`.
    pub game_clock_running: bool,
}

/// A game number names a real game when it is neither blank nor `"0"`.
fn is_real(number: &str) -> bool {
    !number.is_empty() && number != "0"
}

/// The live update this refbox should be sending now, or `None` when it should
/// send nothing.
///
/// Reads the raw `game_number` / `next_game_number` fields, never
/// `GameSnapshot::game_number()`, which substitutes the next number between
/// games.
pub fn live_frame(i: &LiveInputs) -> Option<LiveFrame> {
    if !effective_live_updates(i.stored_enabled, i.mode, i.source) {
        return None;
    }
    let event = i.event?;
    let snapshot = i.snapshot;
    let current = &snapshot.game_number;
    let next = &snapshot.next_game_number;

    let state_from_snapshot = |scores: Option<LiveScores>| LiveGameState {
        period: snapshot.current_period,
        clock_seconds: snapshot.secs_in_period,
        clock_running: i.game_clock_running,
        timeout: snapshot
            .timeout
            .map(|t| LiveTimeout::from_snapshot(t, None)),
        scores,
        last_goal: None,
        penalties: None,
    };

    let (game, state, heartbeat) = if snapshot.current_period != GamePeriod::BetweenGames {
        if !is_real(current) {
            return None;
        }
        let scores = LiveScores {
            black: snapshot.scores.black,
            white: snapshot.scores.white,
        };
        (current.clone(), state_from_snapshot(Some(scores)), true)
    } else if is_real(next) {
        (next.clone(), state_from_snapshot(None), true)
    } else if is_real(current) {
        let state = LiveGameState {
            period: GamePeriod::BetweenGames,
            clock_seconds: 0,
            clock_running: false,
            timeout: None,
            scores: None,
            last_goal: None,
            penalties: None,
        };
        (current.clone(), state, false)
    } else {
        return None;
    };

    Some(LiveFrame {
        site: i.site.to_string(),
        event: event.clone(),
        game,
        state,
        heartbeat,
    })
}

/// Whether `next` must be sent at once rather than waiting for the next
/// heartbeat. A running clock's ticking seconds alone are not a change; the
/// seconds count only while the clock is stopped, where a difference means
/// the operator edited the clock.
pub fn needs_send_now(last_sent: Option<&LiveFrame>, next: &LiveFrame) -> bool {
    let Some(last) = last_sent else {
        return true;
    };
    last.site != next.site
        || last.event != next.event
        || last.game != next.game
        || last.state.period != next.state.period
        || last.state.timeout != next.state.timeout
        || last.state.clock_running != next.state.clock_running
        || last.state.scores != next.state.scores
        || (!next.state.clock_running && last.state.clock_seconds != next.state.clock_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uwh_common::{
        bundles::BlackWhiteBundle, game_snapshot::TimeoutSnapshot, uwhportal::live::LiveTimeout,
    };

    const SITE: &str = "https://uwhportal.example";

    fn event() -> EventId {
        EventId::from_partial("1889-B")
    }

    fn snapshot(period: GamePeriod, current: &str, next: &str, secs: u32) -> GameSnapshot {
        GameSnapshot {
            current_period: period,
            secs_in_period: secs,
            scores: BlackWhiteBundle { black: 2, white: 1 },
            game_number: current.to_string(),
            next_game_number: next.to_string(),
            ..Default::default()
        }
    }

    fn inputs<'a>(snapshot: &'a GameSnapshot, event: &'a EventId) -> LiveInputs<'a> {
        LiveInputs {
            stored_enabled: true,
            mode: Mode::Hockey6V6,
            source: GameSource::Portal,
            site: SITE,
            event: Some(event),
            snapshot,
            game_clock_running: true,
        }
    }

    fn frame(game: &str, state: LiveGameState) -> LiveFrame {
        LiveFrame {
            site: SITE.to_string(),
            event: event(),
            game: game.to_string(),
            state,
            heartbeat: true,
        }
    }

    fn running_state(secs: u32) -> LiveGameState {
        LiveGameState {
            period: GamePeriod::FirstHalf,
            clock_seconds: secs,
            clock_running: true,
            timeout: None,
            scores: Some(LiveScores { black: 0, white: 0 }),
            last_goal: None,
            penalties: None,
        }
    }

    #[test]
    fn no_frame_when_off_manual_custom_rugby_beeptest_or_no_event() {
        let snap = snapshot(GamePeriod::FirstHalf, "5", "6", 300);
        let ev = event();
        assert!(live_frame(&inputs(&snap, &ev)).is_some(), "baseline sends");

        let off = LiveInputs {
            stored_enabled: false,
            ..inputs(&snap, &ev)
        };
        assert_eq!(live_frame(&off), None, "setting off");

        let manual = LiveInputs {
            source: GameSource::Manual,
            ..inputs(&snap, &ev)
        };
        assert_eq!(live_frame(&manual), None, "manual source");

        let custom = LiveInputs {
            source: GameSource::Custom,
            ..inputs(&snap, &ev)
        };
        assert_eq!(live_frame(&custom), None, "custom site");

        let rugby = LiveInputs {
            mode: Mode::Rugby,
            ..inputs(&snap, &ev)
        };
        assert_eq!(live_frame(&rugby), None, "rugby");

        let beep = LiveInputs {
            mode: Mode::BeepTest,
            ..inputs(&snap, &ev)
        };
        assert_eq!(live_frame(&beep), None, "beep test");

        let no_event = LiveInputs {
            event: None,
            ..inputs(&snap, &ev)
        };
        assert_eq!(live_frame(&no_event), None, "no event");
    }

    #[test]
    fn in_play_names_current_game_with_scores() {
        let snap = snapshot(GamePeriod::SecondHalf, "5", "6", 462);
        let ev = event();
        let got = live_frame(&inputs(&snap, &ev)).unwrap();
        assert_eq!(
            got,
            LiveFrame {
                site: SITE.to_string(),
                event: event(),
                game: "5".to_string(),
                state: LiveGameState {
                    period: GamePeriod::SecondHalf,
                    clock_seconds: 462,
                    clock_running: true,
                    timeout: None,
                    scores: Some(LiveScores { black: 2, white: 1 }),
                    last_goal: None,
                    penalties: None,
                },
                heartbeat: true,
            }
        );
    }

    #[test]
    fn sudden_death_seconds_pass_through() {
        let snap = snapshot(GamePeriod::SuddenDeath, "5", "6", 75);
        let ev = event();
        let got = live_frame(&inputs(&snap, &ev)).unwrap();
        assert_eq!(got.state.period, GamePeriod::SuddenDeath);
        assert_eq!(got.state.clock_seconds, 75);
    }

    #[test]
    fn break_names_next_game_without_scores() {
        let snap = snapshot(GamePeriod::BetweenGames, "5", "6", 120);
        let ev = event();
        let got = live_frame(&inputs(&snap, &ev)).unwrap();
        assert_eq!(got.game, "6");
        assert_eq!(got.state.period, GamePeriod::BetweenGames);
        assert_eq!(got.state.clock_seconds, 120);
        assert_eq!(got.state.scores, None);
        assert!(got.heartbeat);
    }

    #[test]
    fn break_after_reset_with_current_zero_names_next_game() {
        let snap = snapshot(GamePeriod::BetweenGames, "0", "1", 120);
        let ev = event();
        let got = live_frame(&inputs(&snap, &ev)).unwrap();
        assert_eq!(got.game, "1");
        assert!(got.heartbeat);
    }

    #[test]
    fn finished_court_sends_one_forced_frame() {
        let snap = snapshot(GamePeriod::BetweenGames, "12", "", 40);
        let ev = event();
        let got = live_frame(&inputs(&snap, &ev)).unwrap();
        assert_eq!(got.game, "12");
        assert_eq!(
            got.state,
            LiveGameState {
                period: GamePeriod::BetweenGames,
                clock_seconds: 0,
                clock_running: false,
                timeout: None,
                scores: None,
                last_goal: None,
                penalties: None,
            }
        );
        assert!(!got.heartbeat);
    }

    #[test]
    fn nothing_real_means_no_frame() {
        let ev = event();
        for (current, next) in [("0", ""), ("", ""), ("0", "0"), ("", "0")] {
            let snap = snapshot(GamePeriod::BetweenGames, current, next, 40);
            assert_eq!(
                live_frame(&inputs(&snap, &ev)),
                None,
                "between games, current {current:?}, next {next:?}"
            );
        }
        for current in ["0", ""] {
            let snap = snapshot(GamePeriod::FirstHalf, current, "6", 40);
            assert_eq!(
                live_frame(&inputs(&snap, &ev)),
                None,
                "in play, current {current:?}"
            );
        }
    }

    #[test]
    fn clock_running_comes_from_input_not_timeout() {
        let snap = GameSnapshot {
            timeout: Some(TimeoutSnapshot::Ref(5)),
            ..snapshot(GamePeriod::FirstHalf, "5", "6", 300)
        };
        let ev = event();
        let i = LiveInputs {
            game_clock_running: false,
            ..inputs(&snap, &ev)
        };
        let got = live_frame(&i).unwrap();
        assert!(!got.state.clock_running);
        assert_eq!(got.state.timeout, Some(LiveTimeout::Ref));
    }

    #[test]
    fn ticking_clock_is_not_a_change() {
        let last = frame("5", running_state(300));
        let next = frame("5", running_state(299));
        assert!(!needs_send_now(Some(&last), &next));
        assert!(needs_send_now(None, &next), "nothing sent yet");
    }

    #[test]
    fn clock_edit_while_stopped_is_a_change() {
        let stopped = |secs| LiveGameState {
            clock_running: false,
            ..running_state(secs)
        };
        let last = frame("5", stopped(300));
        assert!(!needs_send_now(Some(&last), &frame("5", stopped(300))));
        assert!(needs_send_now(Some(&last), &frame("5", stopped(240))));
    }

    #[test]
    fn score_change_is_a_change() {
        let last = frame("5", running_state(300));
        let next = frame(
            "5",
            LiveGameState {
                scores: Some(LiveScores { black: 1, white: 0 }),
                ..running_state(299)
            },
        );
        assert!(needs_send_now(Some(&last), &next));
    }

    #[test]
    fn timeout_start_is_a_change() {
        let last = frame("5", running_state(300));
        let next = frame(
            "5",
            LiveGameState {
                clock_running: false,
                timeout: Some(LiveTimeout::White(None)),
                ..running_state(300)
            },
        );
        assert!(needs_send_now(Some(&last), &next));
        // The timeout alone, with the running flag unchanged, still counts.
        let next_same_run = frame(
            "5",
            LiveGameState {
                timeout: Some(LiveTimeout::Ref),
                ..running_state(299)
            },
        );
        assert!(needs_send_now(Some(&last), &next_same_run));
    }

    #[test]
    fn only_period_change_is_a_change() {
        let last = frame("5", running_state(300));
        let next = frame(
            "5",
            LiveGameState {
                period: GamePeriod::SecondHalf,
                ..running_state(300)
            },
        );
        assert!(needs_send_now(Some(&last), &next));
    }

    /// The operator pressing STOP: nothing else changes, not even the seconds.
    #[test]
    fn only_clock_running_change_is_a_change() {
        let last = frame("5", running_state(300));
        let next = frame(
            "5",
            LiveGameState {
                clock_running: false,
                ..running_state(300)
            },
        );
        assert!(needs_send_now(Some(&last), &next));
    }

    #[test]
    fn game_number_change_is_a_change() {
        let last = frame("5", running_state(300));
        let next = frame("6", running_state(299));
        assert!(needs_send_now(Some(&last), &next));
    }

    #[test]
    fn site_change_is_a_change() {
        let last = frame("5", running_state(300));
        let next = LiveFrame {
            site: "https://other.example".to_string(),
            ..frame("5", running_state(299))
        };
        assert!(needs_send_now(Some(&last), &next));
        let other_event = LiveFrame {
            event: EventId::from_partial("2000-A"),
            ..frame("5", running_state(299))
        };
        assert!(needs_send_now(Some(&last), &other_event));
    }
}
