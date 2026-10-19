//! The rules that decide which live update, if any, this refbox sends to the
//! portal. Pure: no I/O here, only the decision.

pub mod sender;
pub mod stats_sender;
#[cfg(test)]
mod test_portal;

use crate::{
    config::{GameSource, Mode, effective_live_updates},
    tournament_manager::{EnginePenalty, LiveDetails, RunningStats, penalty::PenaltyKind},
};
use std::{num::NonZeroU16, time::Duration};
use uwh_common::{
    color::Color,
    game_snapshot::PenaltyTime,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::{
        live::{
            LiveGameState, LiveGoal, LivePenalty, LivePenaltyTime, LiveScores, LiveTimeout,
            TeamTimeoutClock, TimedPenaltyLength,
        },
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
    /// Every penalty the engine holds, served ones included, in the engine's
    /// order. Compared by [`needs_send_now`] so that a penalty added, edited or
    /// removed sends at once, while one running out (same key) does not.
    /// `None` when the engine read failed: the penalties are unknown, which is
    /// not the same as none.
    pub penalty_key: Option<Vec<PenaltyIdentity>>,
}

/// One in-play stats list ready to send: where it goes and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsFrame {
    pub site: String,
    pub event: EventId,
    /// Always the game in play, never the next one.
    pub game: GameNumber,
    pub list: RunningStats,
}

impl StatsFrame {
    /// Whether `other` is the same list for the same game. Compares the list's key, never its
    /// body, whose `occurredOn` times differ on every build.
    pub fn same_list(&self, other: &StatsFrame) -> bool {
        self.site == other.site
            && self.event == other.event
            && self.game == other.game
            && self.list.key == other.list.key
    }
}

/// What makes a penalty the same penalty from one frame to the next. Its time
/// left is deliberately not part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltyIdentity {
    pub team: Color,
    pub cap: u8,
    pub kind: PenaltyKind,
    pub given: (GamePeriod, Duration),
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
    /// What the snapshot does not carry, read from the engine under the same
    /// lock. `None` when that read failed: the v3 fields then go out null.
    pub details: Option<&'a LiveDetails>,
}

/// The penalty as the portal is sent it, or `None` when it must not be sent: a
/// served penalty (`Seconds(0)`, including a sub-second remainder the snapshot
/// floors to 0) and a kind/time mismatch the engine never produces.
fn live_penalty(p: &EnginePenalty) -> Option<LivePenalty> {
    let length = |kind| match kind {
        PenaltyKind::ThirtySecond => Some(TimedPenaltyLength::ThirtySecond),
        PenaltyKind::OneMinute => Some(TimedPenaltyLength::OneMinute),
        PenaltyKind::TwoMinute => Some(TimedPenaltyLength::TwoMinute),
        PenaltyKind::FourMinute => Some(TimedPenaltyLength::FourMinute),
        PenaltyKind::FiveMinute => Some(TimedPenaltyLength::FiveMinute),
        PenaltyKind::TotalDismissal => None,
    };
    let time = match (p.kind, p.time) {
        (PenaltyKind::TotalDismissal, PenaltyTime::TotalDismissal) => {
            LivePenaltyTime::TotalDismissal
        }
        (kind, PenaltyTime::Seconds(secs)) => LivePenaltyTime::Timed {
            length: length(kind)?,
            seconds_left: NonZeroU16::new(secs)?,
        },
        (_, PenaltyTime::TotalDismissal) => return None,
    };
    Some(LivePenalty {
        team: p.team,
        cap: p.cap,
        time,
    })
}

fn penalty_identity(p: &EnginePenalty) -> PenaltyIdentity {
    PenaltyIdentity {
        team: p.team,
        cap: p.cap,
        kind: p.kind,
        given: p.given,
    }
}

/// The event to send for, or `None` when the v2 send conditions do not hold: Live Updates on, a
/// source and mode that send, and an event. Both [`live_frame`] and [`stats_frame`] start here.
fn send_event<'a>(i: &LiveInputs<'a>) -> Option<&'a EventId> {
    if !effective_live_updates(i.stored_enabled, i.mode, i.source) {
        return None;
    }
    i.event
}

/// The in-play stats list this refbox should be sending now, or `None` when it should send none:
/// only under [`live_frame`]'s send conditions, only while a game is in play (never
/// `BetweenGames`), and only for a real raw `game_number` -- the game in play.
pub fn stats_frame(i: &LiveInputs, stats: Option<&RunningStats>) -> Option<StatsFrame> {
    let event = send_event(i)?;
    let list = stats?;
    let game = &i.snapshot.game_number;
    if i.snapshot.current_period == GamePeriod::BetweenGames || !is_real(game) {
        return None;
    }
    Some(StatsFrame {
        site: i.site.to_string(),
        event: event.clone(),
        game: game.clone(),
        list: list.clone(),
    })
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
    let event = send_event(i)?;
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

    // Between games every v3 field stays null and the key empty, whatever the
    // engine holds: the refbox resets partway through the break.
    let (game, state, heartbeat, penalty_key) =
        if snapshot.current_period != GamePeriod::BetweenGames {
            if !is_real(current) {
                return None;
            }
            let scores = LiveScores {
                black: snapshot.scores.black,
                white: snapshot.scores.white,
            };
            let mut state = state_from_snapshot(Some(scores));
            // Without details (the engine read failed) the v3 fields stay null: a
            // list from an older read would look plausible and be wrong.
            let mut penalty_key = None;
            if let Some(details) = i.details {
                let team_clock = details.team_timeout.map(|t| TeamTimeoutClock {
                    seconds_left: t.seconds_left,
                    running: t.running,
                });
                state.timeout = snapshot
                    .timeout
                    .map(|t| LiveTimeout::from_snapshot(t, team_clock));
                state.last_goal = details
                    .recent_goal
                    .map(|(team, cap, clock_seconds)| LiveGoal {
                        team,
                        cap,
                        clock_seconds,
                    });
                state.penalties = Some(details.penalties.iter().filter_map(live_penalty).collect());
                penalty_key = Some(details.penalties.iter().map(penalty_identity).collect());
            }
            (current.clone(), state, true, penalty_key)
        } else if is_real(next) {
            (next.clone(), state_from_snapshot(None), true, Some(vec![]))
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
            (current.clone(), state, false, Some(vec![]))
        } else {
            return None;
        };

    Some(LiveFrame {
        site: i.site.to_string(),
        event: event.clone(),
        game,
        state,
        heartbeat,
        penalty_key,
    })
}

/// Whether the timeout changed in a way that must be sent at once: a different
/// kind or team, a team clock appearing or disappearing, the team clock
/// starting or stopping, or its seconds differing while the new clock is
/// stopped (an edit). A running team clock ticking is not a change.
fn timeout_changed(last: Option<LiveTimeout>, next: Option<LiveTimeout>) -> bool {
    match (last, next) {
        (Some(LiveTimeout::White(a)), Some(LiveTimeout::White(b)))
        | (Some(LiveTimeout::Black(a)), Some(LiveTimeout::Black(b))) => match (a, b) {
            (None, None) => false,
            (Some(a), Some(b)) => {
                a.running != b.running || (!b.running && a.seconds_left != b.seconds_left)
            }
            _ => true,
        },
        (a, b) => a != b,
    }
}

/// Whether `next` must be sent at once rather than waiting for the next
/// heartbeat. A running clock's ticking seconds alone are not a change; the
/// seconds count only while the clock is stopped, where a difference means
/// the operator edited the clock. The same holds for a team timeout's clock.
///
/// A penalty added, edited or removed changes `penalty_key` and sends at once.
/// `last_goal` and `penalties` are deliberately not compared: a penalty running
/// out and a goal expiring wait for the heartbeat (agreement, "Timing
/// additions").
///
/// A failed engine read (`penalty_key` is `None`) is not a change: the
/// penalties and the team clock are unknown, not removed, so only a different
/// timeout kind or team counts. The first good read after one that was sent
/// sends at once, so the real values replace the nulls.
pub fn needs_send_now(last_sent: Option<&LiveFrame>, next: &LiveFrame) -> bool {
    let Some(last) = last_sent else {
        return true;
    };
    last.site != next.site
        || last.event != next.event
        || last.game != next.game
        || last.state.period != next.state.period
        || last.state.clock_running != next.state.clock_running
        || last.state.scores != next.state.scores
        || (!next.state.clock_running && last.state.clock_seconds != next.state.clock_seconds)
        || match (&last.penalty_key, &next.penalty_key) {
            (Some(a), Some(b)) => a != b || timeout_changed(last.state.timeout, next.state.timeout),
            (_, None) => timeout_changed(
                last.state.timeout.map(without_team_clock),
                next.state.timeout.map(without_team_clock),
            ),
            (None, Some(_)) => true,
        }
}

/// The timeout with any team clock dropped, for comparing kind and team only.
fn without_team_clock(t: LiveTimeout) -> LiveTimeout {
    match t {
        LiveTimeout::White(_) => LiveTimeout::White(None),
        LiveTimeout::Black(_) => LiveTimeout::Black(None),
        other => other,
    }
}

/// The portal's v3 validation rules (agreement, "Portal review of the field
/// table"), checked against the SERIALISED body exactly as the portal reads it.
/// Returns one label per broken rule; empty means the portal would accept it.
/// The portal rejects the WHOLE send on any one of these, live score included.
#[cfg(test)]
pub(crate) fn portal_rule_violations(body: &serde_json::Value) -> Vec<&'static str> {
    use serde_json::Value;

    let mut found = Vec::new();
    let is_null = |v: Option<&Value>| v.is_none_or(Value::is_null);
    let team_ok = |v: Option<&Value>| matches!(v.and_then(Value::as_str), Some("Black" | "White"));
    let cap_ok = |v: Option<&Value>| v.and_then(Value::as_u64).is_some_and(|c| c <= 255);

    let between = body.get("period").and_then(Value::as_str) == Some("BetweenGames");

    // lastGoal: null, or {team, cap, clockSeconds}.
    let goal = body.get("lastGoal");
    if !is_null(goal) {
        if between {
            found.push("rule 5: lastGoal not null in BetweenGames");
        }
        match goal.and_then(Value::as_object) {
            None => found.push("lastGoal not an object"),
            Some(g) => {
                if !team_ok(g.get("team")) {
                    found.push("rule 1: team not Black/White");
                }
                if !cap_ok(g.get("cap")) {
                    found.push("rule 2: cap not 0-255");
                }
                if g.get("clockSeconds").and_then(Value::as_u64).is_none() {
                    found.push("lastGoal.clockSeconds not a whole number");
                }
            }
        }
    }

    // penalties: null, or an array of {team, cap, kind, secondsLeft}.
    let penalties = body.get("penalties");
    if !is_null(penalties) {
        if between {
            found.push("rule 5: penalties not null in BetweenGames");
        }
        match penalties.and_then(Value::as_array) {
            None => found.push("penalties not an array"),
            Some(list) => {
                for p in list {
                    if !team_ok(p.get("team")) {
                        found.push("rule 1: team not Black/White");
                    }
                    if !cap_ok(p.get("cap")) {
                        found.push("rule 2: cap not 0-255");
                    }
                    let kind = p.get("kind").and_then(Value::as_str);
                    let timed = matches!(
                        kind,
                        Some(
                            "ThirtySecond"
                                | "OneMinute"
                                | "TwoMinute"
                                | "FourMinute"
                                | "FiveMinute"
                        )
                    );
                    let dismissal = kind == Some("TotalDismissal");
                    if !timed && !dismissal {
                        found.push("rule 3: kind not one of the six");
                    }
                    let secs = p.get("secondsLeft");
                    if dismissal && !is_null(secs) {
                        found.push("rule 4: secondsLeft not null for TotalDismissal");
                    }
                    if !dismissal {
                        if is_null(secs) {
                            found.push("rule 4: secondsLeft null for a timed penalty");
                        } else if secs.and_then(Value::as_u64).is_none_or(|s| s == 0) {
                            found.push("rule 4: secondsLeft not > 0");
                        }
                    }
                }
            }
        }
    }

    // timeoutSeconds: >= 0, and only with a White/Black timeout.
    let timeout_secs = body.get("timeoutSeconds");
    if !is_null(timeout_secs) {
        if timeout_secs.and_then(Value::as_u64).is_none() {
            found.push("rule 6: timeoutSeconds not >= 0");
        }
        let team_timeout = matches!(
            body.get("timeout").and_then(Value::as_str),
            Some("White" | "Black")
        );
        if !team_timeout {
            found.push("rule 6: timeoutSeconds without a White/Black timeout");
        }
    }

    // timeoutRunning: non-null (a boolean) if and only if timeoutSeconds is non-null.
    let timeout_running = body.get("timeoutRunning");
    if is_null(timeout_running) != is_null(timeout_secs) {
        found.push("rule 7: timeoutRunning non-null iff timeoutSeconds non-null");
    } else if !is_null(timeout_running) && !timeout_running.is_some_and(Value::is_boolean) {
        found.push("rule 7: timeoutRunning not a boolean");
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tournament_manager::TeamTimeout;
    use uwh_common::{bundles::BlackWhiteBundle, game_snapshot::TimeoutSnapshot};

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
            details: None,
        }
    }

    fn frame(game: &str, state: LiveGameState) -> LiveFrame {
        LiveFrame {
            site: SITE.to_string(),
            event: event(),
            game: game.to_string(),
            state,
            heartbeat: true,
            penalty_key: Some(vec![]),
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

    /// The agreement's own v3 example, as the portal would receive it.
    fn agreement_example() -> serde_json::Value {
        serde_json::json!({
            "period": "SecondHalf", "clockSeconds": 462, "clockRunning": false,
            "timeout": "White", "timeoutSeconds": 41, "timeoutRunning": true,
            "scores": { "black": 1, "white": 3 },
            "lastGoal": { "team": "White", "cap": 7, "clockSeconds": 470 },
            "penalties": [
                { "team": "Black", "cap": 4, "kind": "TwoMinute", "secondsLeft": 83 },
                { "team": "White", "cap": 11, "kind": "TotalDismissal", "secondsLeft": null }
            ]
        })
    }

    /// Every rule must be seen failing on a body that breaks it, and on nothing
    /// else: a check never seen failing is not a check.
    #[test]
    fn validator_catches_each_rule() {
        use serde_json::{Value, json};

        assert_eq!(
            portal_rule_violations(&agreement_example()),
            Vec::<&str>::new(),
            "the agreement example is valid"
        );

        type Mutation = fn(&mut Value);
        let cases: [(&str, Mutation, &str); 13] = [
            (
                "penalty team Red",
                |b| b["penalties"][0]["team"] = json!("Red"),
                "rule 1: team not Black/White",
            ),
            (
                "goal team Red",
                |b| b["lastGoal"]["team"] = json!("Red"),
                "rule 1: team not Black/White",
            ),
            (
                "penalty cap 300",
                |b| b["penalties"][0]["cap"] = json!(300),
                "rule 2: cap not 0-255",
            ),
            (
                "goal cap 300",
                |b| b["lastGoal"]["cap"] = json!(300),
                "rule 2: cap not 0-255",
            ),
            (
                "kind TenMinute",
                |b| b["penalties"][0]["kind"] = json!("TenMinute"),
                "rule 3: kind not one of the six",
            ),
            (
                "secondsLeft 0",
                |b| b["penalties"][0]["secondsLeft"] = json!(0),
                "rule 4: secondsLeft not > 0",
            ),
            (
                "TD with seconds",
                |b| b["penalties"][1]["secondsLeft"] = json!(60),
                "rule 4: secondsLeft not null for TotalDismissal",
            ),
            (
                "timed penalty with null",
                |b| b["penalties"][0]["secondsLeft"] = Value::Null,
                "rule 4: secondsLeft null for a timed penalty",
            ),
            (
                "penalties in BetweenGames",
                |b| {
                    b["period"] = json!("BetweenGames");
                    b["lastGoal"] = Value::Null;
                },
                "rule 5: penalties not null in BetweenGames",
            ),
            (
                "lastGoal in BetweenGames",
                |b| {
                    b["period"] = json!("BetweenGames");
                    b["penalties"] = Value::Null;
                },
                "rule 5: lastGoal not null in BetweenGames",
            ),
            (
                "timeoutSeconds with Ref",
                |b| b["timeout"] = json!("Ref"),
                "rule 6: timeoutSeconds without a White/Black timeout",
            ),
            (
                "timeoutSeconds negative",
                |b| b["timeoutSeconds"] = json!(-1),
                "rule 6: timeoutSeconds not >= 0",
            ),
            (
                "timeoutRunning without seconds",
                |b| b["timeoutSeconds"] = Value::Null,
                "rule 7: timeoutRunning non-null iff timeoutSeconds non-null",
            ),
        ];
        for (what, mutate, expected) in cases {
            let mut body = agreement_example();
            mutate(&mut body);
            assert_eq!(portal_rule_violations(&body), vec![expected], "{what}");
        }

        // Shape checks beyond the seven rules: each seen failing too.
        let shapes: [(&str, Mutation, &str); 4] = [
            (
                "lastGoal a string",
                |b| b["lastGoal"] = json!("goal"),
                "lastGoal not an object",
            ),
            (
                "lastGoal clock negative",
                |b| b["lastGoal"]["clockSeconds"] = json!(-1),
                "lastGoal.clockSeconds not a whole number",
            ),
            (
                "penalties an object",
                |b| b["penalties"] = json!({}),
                "penalties not an array",
            ),
            (
                "timeoutRunning a string",
                |b| b["timeoutRunning"] = json!("yes"),
                "rule 7: timeoutRunning not a boolean",
            ),
        ];
        for (what, mutate, expected) in shapes {
            let mut body = agreement_example();
            mutate(&mut body);
            assert_eq!(portal_rule_violations(&body), vec![expected], "{what}");
        }

        // Rule 7 the other way round: seconds without the running flag.
        let mut body = agreement_example();
        body["timeoutRunning"] = Value::Null;
        assert_eq!(
            portal_rule_violations(&body),
            vec!["rule 7: timeoutRunning non-null iff timeoutSeconds non-null"],
            "timeoutSeconds without timeoutRunning"
        );
        // Rule 6 allows 0: a team timeout stopped at 0 and waiting.
        let mut body = agreement_example();
        body["timeoutSeconds"] = json!(0);
        body["timeoutRunning"] = json!(false);
        assert_eq!(
            portal_rule_violations(&body),
            Vec::<&str>::new(),
            "0 left, stopped"
        );
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

    fn list(key: &str, body: &str) -> RunningStats {
        RunningStats {
            body: body.to_string(),
            key: key.to_string(),
        }
    }

    #[test]
    fn stats_frame_none_when_off_manual_custom_rugby_beeptest_or_no_event() {
        let snap = snapshot(GamePeriod::FirstHalf, "5", "6", 300);
        let ev = event();
        let stats = list("[]", "[]");
        let base = || inputs(&snap, &ev);
        assert!(
            stats_frame(&base(), Some(&stats)).is_some(),
            "baseline sends"
        );
        assert_eq!(stats_frame(&base(), None), None, "no list");

        let cases = [
            (
                LiveInputs {
                    stored_enabled: false,
                    ..base()
                },
                "setting off",
            ),
            (
                LiveInputs {
                    source: GameSource::Manual,
                    ..base()
                },
                "manual source",
            ),
            (
                LiveInputs {
                    source: GameSource::Custom,
                    ..base()
                },
                "custom site",
            ),
            (
                LiveInputs {
                    mode: Mode::Rugby,
                    ..base()
                },
                "rugby",
            ),
            (
                LiveInputs {
                    mode: Mode::BeepTest,
                    ..base()
                },
                "beep test",
            ),
            (
                LiveInputs {
                    event: None,
                    ..base()
                },
                "no event",
            ),
        ];
        for (i, why) in cases {
            assert_eq!(stats_frame(&i, Some(&stats)), None, "{why}");
        }
    }

    #[test]
    fn stats_frame_none_between_games() {
        let ev = event();
        let stats = list("[]", "[]");
        // A real current and next game both: the break still sends no list.
        let snap = snapshot(GamePeriod::BetweenGames, "5", "6", 120);
        assert_eq!(stats_frame(&inputs(&snap, &ev), Some(&stats)), None);
        let snap = snapshot(GamePeriod::BetweenGames, "5", "", 120);
        assert_eq!(stats_frame(&inputs(&snap, &ev), Some(&stats)), None);
    }

    #[test]
    fn stats_frame_none_for_unreal_game_number() {
        let ev = event();
        let stats = list("[]", "[]");
        for current in ["", "0"] {
            // A real next number must not stand in for the game in play.
            let snap = snapshot(GamePeriod::FirstHalf, current, "6", 300);
            assert_eq!(
                stats_frame(&inputs(&snap, &ev), Some(&stats)),
                None,
                "game number {current:?}"
            );
        }
    }

    #[test]
    fn stats_frame_names_current_game() {
        let ev = event();
        let stats = list("[{\"k\":1}]", "[{\"k\":1,\"t\":\"x\"}]");
        for period in [
            GamePeriod::FirstHalf,
            GamePeriod::HalfTime,
            GamePeriod::SecondHalf,
            GamePeriod::SuddenDeath,
        ] {
            let snap = snapshot(period, "5", "6", 300);
            assert_eq!(
                stats_frame(&inputs(&snap, &ev), Some(&stats)),
                Some(StatsFrame {
                    site: SITE.to_string(),
                    event: event(),
                    game: "5".to_string(),
                    list: stats.clone(),
                }),
                "{period:?}"
            );
        }
    }

    #[test]
    fn same_list_ignores_body() {
        let base = StatsFrame {
            site: SITE.to_string(),
            event: event(),
            game: "5".to_string(),
            list: list("K", "body one"),
        };
        let other_body = StatsFrame {
            list: list("K", "body two"),
            ..base.clone()
        };
        assert!(base.same_list(&other_body), "same key, different body");

        let different = [
            StatsFrame {
                list: list("K2", "body one"),
                ..base.clone()
            },
            StatsFrame {
                game: "6".to_string(),
                ..base.clone()
            },
            StatsFrame {
                event: EventId::from_partial("1890-C"),
                ..base.clone()
            },
            StatsFrame {
                site: "https://other.example".to_string(),
                ..base.clone()
            },
        ];
        for d in &different {
            assert!(!base.same_list(d), "{d:?}");
        }
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
                penalty_key: None,
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
    // ---- v3 fields -------------------------------------------------------

    fn pen(
        team: Color,
        cap: u8,
        kind: PenaltyKind,
        time: PenaltyTime,
        given: u64,
    ) -> EnginePenalty {
        EnginePenalty {
            team,
            cap,
            kind,
            time,
            given: (GamePeriod::FirstHalf, Duration::from_secs(given)),
        }
    }

    fn nz(secs: u16) -> NonZeroU16 {
        NonZeroU16::new(secs).unwrap()
    }

    /// Built by hand, not with `penalty_identity`, so a fault there shows.
    fn identity(p: &EnginePenalty) -> PenaltyIdentity {
        PenaltyIdentity {
            team: p.team,
            cap: p.cap,
            kind: p.kind,
            given: p.given,
        }
    }

    fn in_play_with(details: Option<&LiveDetails>, snap: &GameSnapshot) -> LiveFrame {
        let ev = event();
        live_frame(&LiveInputs {
            details,
            ..inputs(snap, &ev)
        })
        .unwrap()
    }

    #[test]
    fn in_play_frame_carries_v3_fields() {
        let penalties = vec![
            pen(
                Color::Black,
                4,
                PenaltyKind::TwoMinute,
                PenaltyTime::Seconds(83),
                100,
            ),
            pen(
                Color::Black,
                9,
                PenaltyKind::OneMinute,
                PenaltyTime::Seconds(0),
                200,
            ),
            pen(
                Color::White,
                11,
                PenaltyKind::TotalDismissal,
                PenaltyTime::TotalDismissal,
                300,
            ),
        ];
        let details = LiveDetails {
            recent_goal: Some((Color::White, 7, 470)),
            penalties: penalties.clone(),
            team_timeout: None,
        };
        let snap = snapshot(GamePeriod::SecondHalf, "5", "6", 462);
        let got = in_play_with(Some(&details), &snap);
        assert_eq!(
            got.state.last_goal,
            Some(LiveGoal {
                team: Color::White,
                cap: 7,
                clock_seconds: 470,
            })
        );
        assert_eq!(
            got.state.penalties,
            Some(vec![
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
            ])
        );
        // The served one is not sent but is still part of the key.
        assert_eq!(
            got.penalty_key,
            Some(penalties.iter().map(identity).collect::<Vec<_>>())
        );
    }

    /// `as_snapshot` floors a sub-second remainder to `Seconds(0)`: it must be
    /// dropped, never sent as `secondsLeft: 0` (Review Focus 1).
    #[test]
    fn penalty_under_a_second_is_not_sent() {
        let details = LiveDetails {
            recent_goal: None,
            penalties: vec![
                pen(
                    Color::Black,
                    4,
                    PenaltyKind::ThirtySecond,
                    PenaltyTime::Seconds(0),
                    100,
                ),
                pen(
                    Color::White,
                    5,
                    PenaltyKind::ThirtySecond,
                    PenaltyTime::Seconds(1),
                    101,
                ),
            ],
            team_timeout: None,
        };
        let snap = snapshot(GamePeriod::FirstHalf, "5", "6", 300);
        let got = in_play_with(Some(&details), &snap);
        assert_eq!(
            got.state.penalties,
            Some(vec![LivePenalty {
                team: Color::White,
                cap: 5,
                time: LivePenaltyTime::Timed {
                    length: TimedPenaltyLength::ThirtySecond,
                    seconds_left: nz(1),
                },
            }])
        );
        assert_eq!(got.penalty_key.map(|k| k.len()), Some(2));
    }

    /// The LED panel's limit of 3 per team must not leak in (Review Focus 2).
    #[test]
    fn more_than_three_penalties_all_sent() {
        let details = LiveDetails {
            recent_goal: None,
            penalties: (1..=5)
                .map(|cap| {
                    pen(
                        Color::Black,
                        cap,
                        PenaltyKind::OneMinute,
                        PenaltyTime::Seconds(50),
                        cap.into(),
                    )
                })
                .collect(),
            team_timeout: None,
        };
        let snap = snapshot(GamePeriod::FirstHalf, "5", "6", 300);
        let got = in_play_with(Some(&details), &snap);
        let sent = got.state.penalties.unwrap();
        assert_eq!(sent.len(), 5);
        assert_eq!(
            sent.iter().map(|p| p.cap).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn no_active_penalties_is_an_empty_list() {
        let details = LiveDetails {
            recent_goal: None,
            penalties: vec![],
            team_timeout: None,
        };
        let snap = snapshot(GamePeriod::FirstHalf, "5", "6", 300);
        let got = in_play_with(Some(&details), &snap);
        assert_eq!(got.state.penalties, Some(vec![]));
        assert_eq!(got.state.last_goal, None);
        let body = serde_json::to_string(&got.state).unwrap();
        assert!(body.contains(r#""penalties":[]"#), "{body}");
    }

    /// The refbox resets partway through the break, so nothing it holds then
    /// is sent: rule 5.
    #[test]
    fn break_frames_null_goal_and_penalties() {
        let details = LiveDetails {
            recent_goal: Some((Color::Black, 3, 10)),
            penalties: vec![
                pen(
                    Color::Black,
                    4,
                    PenaltyKind::TwoMinute,
                    PenaltyTime::Seconds(83),
                    100,
                ),
                pen(
                    Color::White,
                    11,
                    PenaltyKind::TotalDismissal,
                    PenaltyTime::TotalDismissal,
                    300,
                ),
            ],
            team_timeout: Some(TeamTimeout {
                running: true,
                seconds_left: 41,
            }),
        };
        let ev = event();
        for (current, next, heartbeat) in [("5", "6", true), ("12", "", false)] {
            let snap = snapshot(GamePeriod::BetweenGames, current, next, 120);
            let got = live_frame(&LiveInputs {
                details: Some(&details),
                ..inputs(&snap, &ev)
            })
            .unwrap();
            assert_eq!(got.heartbeat, heartbeat, "{current}/{next}");
            assert_eq!(got.state.last_goal, None, "{current}/{next}");
            assert_eq!(got.state.penalties, None, "{current}/{next}");
            assert_eq!(got.penalty_key, Some(vec![]), "{current}/{next}");
        }
    }

    /// The engine read failed, so v2 falls back to the app's last snapshot. A
    /// penalty list from an older read would look plausible and be wrong, so
    /// the v3 fields go out null (Review Focus 4).
    #[test]
    fn failed_engine_read_sends_v3_fields_null() {
        let snap = GameSnapshot {
            timeout: Some(TimeoutSnapshot::White(41)),
            ..snapshot(GamePeriod::SecondHalf, "5", "6", 462)
        };
        let got = in_play_with(None, &snap);
        assert_eq!(got.state.last_goal, None);
        assert_eq!(got.state.penalties, None);
        assert_eq!(got.state.timeout, Some(LiveTimeout::White(None)));
        assert_eq!(got.penalty_key, None);
        let body = serde_json::to_value(&got.state).unwrap();
        assert_eq!(portal_rule_violations(&body), Vec::<&str>::new());
    }

    #[test]
    fn team_timeout_clock_comes_from_details() {
        let details = LiveDetails {
            recent_goal: None,
            penalties: vec![],
            team_timeout: Some(TeamTimeout {
                running: false,
                seconds_left: 0,
            }),
        };
        // The snapshot's past-zero stand-in must not reach the wire.
        let snap = GameSnapshot {
            timeout: Some(TimeoutSnapshot::Black(u16::MAX)),
            ..snapshot(GamePeriod::SecondHalf, "5", "6", 462)
        };
        let got = in_play_with(Some(&details), &snap);
        assert_eq!(
            got.state.timeout,
            Some(LiveTimeout::Black(Some(TeamTimeoutClock {
                seconds_left: 0,
                running: false,
            })))
        );
    }

    /// Every frame `live_frame` can build passes every portal rule, as the
    /// portal reads the serialised body.
    #[test]
    fn every_frame_passes_the_portal_rules() {
        use PenaltyKind::*;
        let periods = [
            GamePeriod::BetweenGames,
            GamePeriod::FirstHalf,
            GamePeriod::HalfTime,
            GamePeriod::SecondHalf,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::OvertimeSecondHalf,
            GamePeriod::PreSuddenDeath,
            GamePeriod::SuddenDeath,
        ];
        let timeouts = [
            (None, None),
            (
                Some(TimeoutSnapshot::White(41)),
                Some(TeamTimeout {
                    running: true,
                    seconds_left: 41,
                }),
            ),
            (
                Some(TimeoutSnapshot::Black(0)),
                Some(TeamTimeout {
                    running: false,
                    seconds_left: 0,
                }),
            ),
            // Past zero: the snapshot holds `u16::MAX`, the engine read says 0.
            (
                Some(TimeoutSnapshot::White(u16::MAX)),
                Some(TeamTimeout {
                    running: true,
                    seconds_left: 0,
                }),
            ),
            (Some(TimeoutSnapshot::Ref(12)), None),
            (Some(TimeoutSnapshot::PenaltyShot(5)), None),
        ];
        let mut every_kind = Vec::new();
        let mut given = 0;
        for kind in [ThirtySecond, OneMinute, TwoMinute, FourMinute, FiveMinute] {
            for secs in [0, 1, 83] {
                given += 1;
                every_kind.push(pen(
                    Color::Black,
                    255,
                    kind,
                    PenaltyTime::Seconds(secs),
                    given,
                ));
            }
        }
        every_kind.push(pen(
            Color::White,
            0,
            TotalDismissal,
            PenaltyTime::TotalDismissal,
            99,
        ));
        // Mismatches the engine never produces: must be dropped, not sent.
        every_kind.push(pen(
            Color::White,
            1,
            TotalDismissal,
            PenaltyTime::Seconds(83),
            98,
        ));
        every_kind.push(pen(
            Color::White,
            2,
            TwoMinute,
            PenaltyTime::TotalDismissal,
            97,
        ));

        let ev = event();
        let mut checked = 0;
        for period in periods {
            for (current, next) in [("5", "6"), ("12", "")] {
                for (timeout, running) in timeouts {
                    let full = LiveDetails {
                        recent_goal: Some((Color::White, 255, 470)),
                        penalties: every_kind.clone(),
                        team_timeout: running,
                    };
                    let empty = LiveDetails {
                        recent_goal: None,
                        penalties: vec![],
                        team_timeout: running,
                    };
                    for details in [None, Some(&empty), Some(&full)] {
                        let snap = GameSnapshot {
                            timeout,
                            ..snapshot(period, current, next, 300)
                        };
                        let Some(frame) = live_frame(&LiveInputs {
                            details,
                            ..inputs(&snap, &ev)
                        }) else {
                            continue;
                        };
                        let body = serde_json::to_value(&frame.state).unwrap();
                        assert_eq!(
                            portal_rule_violations(&body),
                            Vec::<&str>::new(),
                            "{period:?} {current}/{next} {timeout:?} {details:?}: {body}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert_eq!(checked, 10 * 2 * 6 * 3, "every situation built a frame");
    }

    // ---- v3 send-at-once rules -------------------------------------------

    fn keyed(key: Vec<EnginePenalty>, state: LiveGameState) -> LiveFrame {
        LiveFrame {
            penalty_key: Some(key.iter().map(identity).collect()),
            ..frame("5", state)
        }
    }

    fn two_minute(team: Color) -> EnginePenalty {
        pen(
            team,
            4,
            PenaltyKind::TwoMinute,
            PenaltyTime::Seconds(83),
            100,
        )
    }

    #[test]
    fn penalty_added_is_a_change() {
        let last = keyed(vec![], running_state(300));
        let next = keyed(vec![two_minute(Color::Black)], running_state(299));
        assert!(needs_send_now(Some(&last), &next));
    }

    #[test]
    fn penalty_edited_is_a_change() {
        let last = keyed(vec![two_minute(Color::Black)], running_state(300));
        let edited = EnginePenalty {
            kind: PenaltyKind::FourMinute,
            ..two_minute(Color::Black)
        };
        let next = keyed(vec![edited], running_state(299));
        assert!(needs_send_now(Some(&last), &next));
    }

    /// Review Focus 5: an edit that moves a penalty to the other team.
    #[test]
    fn penalty_moved_team_is_a_change() {
        let last = keyed(vec![two_minute(Color::Black)], running_state(300));
        let next = keyed(vec![two_minute(Color::White)], running_state(299));
        assert!(needs_send_now(Some(&last), &next));
    }

    #[test]
    fn penalty_removed_is_a_change() {
        let same = two_minute(Color::Black);
        let last = keyed(vec![same.clone(), same.clone()], running_state(300));
        let next = keyed(vec![same], running_state(299));
        assert!(needs_send_now(Some(&last), &next));
    }

    /// The portal ticks a penalty down itself; it running out waits for the
    /// heartbeat. Its key stays until a period start culls it.
    #[test]
    fn penalty_running_out_is_not_a_change() {
        let p = two_minute(Color::Black);
        let sent = LivePenalty {
            team: Color::Black,
            cap: 4,
            time: LivePenaltyTime::Timed {
                length: TimedPenaltyLength::TwoMinute,
                seconds_left: nz(1),
            },
        };
        let last = keyed(
            vec![p.clone()],
            LiveGameState {
                penalties: Some(vec![sent]),
                ..running_state(300)
            },
        );
        let next = keyed(
            vec![EnginePenalty {
                time: PenaltyTime::Seconds(0),
                ..p
            }],
            LiveGameState {
                penalties: Some(vec![]),
                ..running_state(299)
            },
        );
        assert!(!needs_send_now(Some(&last), &next));
    }

    #[test]
    fn goal_expiring_is_not_a_change() {
        let last = frame(
            "5",
            LiveGameState {
                last_goal: Some(LiveGoal {
                    team: Color::White,
                    cap: 7,
                    clock_seconds: 315,
                }),
                ..running_state(300)
            },
        );
        let next = frame("5", running_state(299));
        assert!(!needs_send_now(Some(&last), &next));
    }

    fn team_timeout(seconds_left: u16, running: bool) -> LiveGameState {
        LiveGameState {
            clock_running: false,
            timeout: Some(LiveTimeout::White(Some(TeamTimeoutClock {
                seconds_left,
                running,
            }))),
            ..running_state(300)
        }
    }

    #[test]
    fn team_timeout_ticking_is_not_a_change() {
        let last = frame("5", team_timeout(40, true));
        let next = frame("5", team_timeout(39, true));
        assert!(!needs_send_now(Some(&last), &next));
    }

    #[test]
    fn team_timeout_stop_is_a_change() {
        let last = frame("5", team_timeout(40, true));
        assert!(needs_send_now(
            Some(&last),
            &frame("5", team_timeout(40, false))
        ));
        // And starting again.
        let stopped = frame("5", team_timeout(40, false));
        assert!(needs_send_now(
            Some(&stopped),
            &frame("5", team_timeout(40, true))
        ));
    }

    #[test]
    fn team_timeout_seconds_edit_while_stopped_is_a_change() {
        let last = frame("5", team_timeout(40, false));
        assert!(!needs_send_now(
            Some(&last),
            &frame("5", team_timeout(40, false))
        ));
        assert!(needs_send_now(
            Some(&last),
            &frame("5", team_timeout(30, false))
        ));
    }

    #[test]
    fn team_timeout_clock_appearing_or_disappearing_is_a_change() {
        let without = frame(
            "5",
            LiveGameState {
                clock_running: false,
                timeout: Some(LiveTimeout::White(None)),
                ..running_state(300)
            },
        );
        let with = frame("5", team_timeout(40, true));
        assert!(needs_send_now(Some(&without), &with));
        assert!(needs_send_now(Some(&with), &without));
        // A different team is a change even with the same clock.
        let black = frame(
            "5",
            LiveGameState {
                timeout: Some(LiveTimeout::Black(Some(TeamTimeoutClock {
                    seconds_left: 40,
                    running: true,
                }))),
                ..team_timeout(40, true)
            },
        );
        assert!(needs_send_now(Some(&with), &black));
    }

    // ---- a failed engine read is unknown, not removed ----------------------

    fn unknown(state: LiveGameState) -> LiveFrame {
        LiveFrame {
            penalty_key: None,
            ..frame("5", state)
        }
    }

    /// The clock passes 0:00 a moment before the period changes; a read in
    /// that gap fails. The penalties are unknown, not gone, so nothing is
    /// sent at once.
    #[test]
    fn failed_read_does_not_drop_penalties_at_once() {
        let last = keyed(vec![two_minute(Color::Black)], running_state(1));
        assert!(!needs_send_now(Some(&last), &unknown(running_state(0))));
    }

    #[test]
    fn failed_read_does_not_drop_the_team_clock_at_once() {
        let last = frame("5", team_timeout(40, true));
        let next = unknown(LiveGameState {
            timeout: Some(LiveTimeout::White(None)),
            ..team_timeout(40, true)
        });
        assert!(!needs_send_now(Some(&last), &next));
    }

    /// Unknown penalties do not hide a change that was read: the timeout's
    /// team, the score and the period still send at once.
    #[test]
    fn failed_read_still_sends_other_changes() {
        let last = frame("5", team_timeout(40, true));
        let other_team = unknown(LiveGameState {
            timeout: Some(LiveTimeout::Black(None)),
            ..team_timeout(40, true)
        });
        assert!(needs_send_now(Some(&last), &other_team));
        let scored = unknown(LiveGameState {
            scores: Some(LiveScores { black: 9, white: 9 }),
            ..running_state(299)
        });
        assert!(needs_send_now(
            Some(&keyed(vec![], running_state(300))),
            &scored
        ));
    }

    /// If a frame from a failed read did go out (its nulls on the portal), the
    /// next good read sends at once to put the real values back.
    #[test]
    fn good_read_after_a_sent_failed_one_sends_at_once() {
        let last = unknown(running_state(300));
        let next = keyed(vec![two_minute(Color::Black)], running_state(299));
        assert!(needs_send_now(Some(&last), &next));
        assert!(needs_send_now(
            Some(&last),
            &keyed(vec![], running_state(299))
        ));
    }
}
