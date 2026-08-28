//! The typed game feed served at `GET /game`, for a consumer that reads values rather than the
//! display strings the vMix tables carry (see `tables`' module doc).
//!
//! **This is a published contract with a version on it.** Adding a field is free; removing,
//! renaming, or changing the meaning of one requires bumping [`SCHEMA_VERSION`]. See the design
//! spec `docs/superpowers/specs/2026-08-28-overlay-bridge-renderer-feed-design.md`.
//!
//! **Why this is a separate route rather than extra columns on `/scorebug`.** A vMix table's rows
//! are `BTreeMap`s, so its columns serialize alphabetically; a title left on positional fallback
//! reads whichever column occupies its position. Adding a column to an existing table therefore
//! repoints every positionally-bound title after it, silently and on air. This route cannot do
//! that to them, and it can carry real numbers and nested values, which a flat table of strings
//! cannot.

use serde::Serialize;
use uwh_common::{
    color::Color,
    game_snapshot::{GameSnapshot, PenaltySnapshot, PenaltyTime},
};

use crate::{
    portal::TeamNames,
    state::Display,
    tables::{Rosters, color_code, timeout_label, timeout_seconds},
};

/// The version of the `/game` contract this bridge serves.
///
/// **Bumped only when a field is removed, renamed, or changes meaning. Adding a field never bumps
/// it.** Bumping is expected to stop a consumer, so bumping for a change that could not have
/// broken anything would take a graphic off a live stream for nothing. Note that `period` and
/// `timeout.kind` are `Display` implementations written for people, so renaming a period or
/// timeout label for display reasons *is* a meaning change and does require a bump.
pub const SCHEMA_VERSION: u32 = 1;

/// One goal, exactly as the refbox reported it.
///
/// **No identity is added here, deliberately.** The refbox's `recent_goal` is a single slot with
/// no goal id, so two goals by the same player inside the retention window are byte-identical and
/// indistinguishable. That defect is real and explicitly *not* fixed by this feed; adding a
/// sequence number here would change what a viewer sees, which is out of scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    /// `"BLACK"` or `"WHITE"` -- the same identifiers every vMix table's `team` column uses.
    pub team: String,
    pub player: u8,
}

/// The running timeout, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeout {
    /// The same label `/scorebug`'s `timeout` column carries, from [`timeout_label`].
    pub kind: String,
    /// The timeout's own countdown -- not `secs_in_period`.
    pub secs_remaining: u32,
}

/// One penalty. Mirrors the columns `tables::penalty_row` produces, so both consumers describe a
/// penalty the same way -- with one difference: where the vMix table encodes a dismissal as the
/// literal string `"TD"` with an empty seconds column, this carries a boolean and a null. A typed
/// consumer should not have to recognise `"TD"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Penalty {
    /// `"BLACK"` or `"WHITE"`.
    pub team: String,
    /// The player's cap number, as the refbox reports it.
    pub number: u8,
    /// The player's name from the roster, or `None` when that cap number is not on it -- never a
    /// placeholder, matching `tables`' own contract.
    pub player: Option<String>,
    /// Remaining seconds, or `None` for a total dismissal.
    pub secs_remaining: Option<u32>,
    pub total_dismissal: bool,
    pub infraction: String,
}

/// Everything `GET /game` serves.
///
/// **Every field is always serialized, including as `null`.** No field may gain
/// `skip_serializing_if`: to a consumer an absent key and a null are different things, and only
/// one of them is safe to read blind (the same reasoning `tables::blank_row` already records for
/// the vMix tables). `schema_version` and `connected` are the only fields that are never null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFeed {
    pub schema_version: u32,
    /// Whether the refbox is alive. **The only field that answers that question** -- `recent_goal`,
    /// `timeout` and `next_game_number` are all legitimately null during normal play, so this is
    /// what separates "nothing to report" from "nobody is reporting". Never inferred from timing:
    /// the refbox goes silent for ~25s whenever the clock is stopped.
    pub connected: bool,
    pub period: Option<String>,
    pub secs_in_period: Option<u32>,
    pub black_score: Option<u8>,
    pub white_score: Option<u8>,
    pub black_team: Option<String>,
    pub white_team: Option<String>,
    pub timeout: Option<Timeout>,
    pub game_number: Option<String>,
    pub next_game_number: Option<String>,
    pub is_old_game: Option<bool>,
    pub recent_goal: Option<Goal>,
    pub next_period_len_secs: Option<u32>,
    /// Every penalty on the snapshot -- neither padded nor truncated, unlike `/penalties`. `None`
    /// only when disconnected; an empty list means there are no penalties.
    pub penalties: Option<Vec<Penalty>>,
    /// Paired with `portal_base_url`: both or neither.
    pub event_id: Option<String>,
    /// Paired with `event_id`, and served credential-stripped.
    pub portal_base_url: Option<String>,
}

/// Every penalty on the snapshot, ordered exactly as `tables::penalties` orders it so both
/// consumers agree on which penalty is first (`PenaltyTime` has a deliberate custom `Ord`).
///
/// **Neither padded nor truncated**, unlike `/penalties`, which pads to a fixed row count and
/// keeps only the first ten because a vMix title needs a fixed number of rows to bind to. An
/// array needs neither: an empty list means there are no penalties, and every penalty is served.
fn penalties_of(snapshot: &GameSnapshot, rosters: &Rosters) -> Vec<Penalty> {
    let mut entries: Vec<(Color, &PenaltySnapshot)> = snapshot
        .penalties
        .black
        .iter()
        .map(|penalty| (Color::Black, penalty))
        .chain(
            snapshot
                .penalties
                .white
                .iter()
                .map(|penalty| (Color::White, penalty)),
        )
        .collect();
    entries.sort_by_key(|(_, penalty)| std::cmp::Reverse(penalty.time));

    entries
        .into_iter()
        .map(|(color, penalty)| Penalty {
            team: color_code(color).to_string(),
            number: penalty.player_number,
            player: rosters[color].get(&penalty.player_number).cloned(),
            secs_remaining: match penalty.time {
                PenaltyTime::Seconds(secs) => Some(u32::from(secs)),
                PenaltyTime::TotalDismissal => None,
            },
            total_dismissal: matches!(penalty.time, PenaltyTime::TotalDismissal),
            infraction: penalty.infraction.short_name().to_string(),
        })
        .collect()
}

/// A blanked feed: `connected: false` and every game value `null`.
fn blanked() -> GameFeed {
    GameFeed {
        schema_version: SCHEMA_VERSION,
        connected: false,
        period: None,
        secs_in_period: None,
        black_score: None,
        white_score: None,
        black_team: None,
        white_team: None,
        timeout: None,
        game_number: None,
        next_game_number: None,
        is_old_game: None,
        recent_goal: None,
        next_period_len_secs: None,
        penalties: None,
        event_id: None,
        portal_base_url: None,
    }
}

/// Builds the `/game` payload.
///
/// `names` and `rosters` are resolved by the caller, exactly as they are for `tables`' own
/// builders -- this module never reaches into portal state.
///
/// When `connected` is false the snapshot is ignored entirely and every game value is blanked.
/// That is the same rule `tables::finish_table` applies to every vMix table, so both consumers
/// agree on what a dropped refbox looks like.
pub fn game_feed(
    display: &Display,
    names: Option<&TeamNames>,
    rosters: &Rosters,
    connected: bool,
) -> GameFeed {
    if !connected {
        return blanked();
    }

    let snapshot = &display.snapshot;

    GameFeed {
        schema_version: SCHEMA_VERSION,
        connected: true,
        period: Some(snapshot.current_period.to_string()),
        secs_in_period: Some(snapshot.secs_in_period),
        black_score: Some(snapshot.scores.black),
        white_score: Some(snapshot.scores.white),
        black_team: names.and_then(|n| n.dark.clone()),
        white_team: names.and_then(|n| n.light.clone()),
        timeout: snapshot.timeout.map(|timeout| Timeout {
            kind: timeout_label(timeout).to_string(),
            secs_remaining: u32::from(timeout_seconds(timeout)),
        }),
        game_number: Some(snapshot.game_number().to_string()),
        next_game_number: snapshot.next_game_number().map(ToString::to_string),
        is_old_game: Some(snapshot.is_old_game),
        recent_goal: snapshot.recent_goal.map(|(color, player)| Goal {
            team: color_code(color).to_string(),
            player,
        }),
        next_period_len_secs: snapshot.next_period_len_secs,
        penalties: Some(penalties_of(snapshot, rosters)),
        event_id: None,
        portal_base_url: None,
    }
}

#[cfg(test)]
mod tests {
    use uwh_common::{
        bundles::BlackWhiteBundle,
        color::Color,
        game_snapshot::{
            GamePeriod, GameSnapshot, Infraction, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
        },
    };

    use super::*;

    fn display_with(snapshot: GameSnapshot) -> Display {
        Display { snapshot }
    }

    fn live_snapshot() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 431,
            scores: BlackWhiteBundle { black: 3, white: 2 },
            game_number: "10".to_string(),
            next_game_number: "20".to_string(),
            is_old_game: false,
            recent_goal: Some((Color::Black, 7)),
            next_period_len_secs: Some(180),
            timeout: Some(TimeoutSnapshot::Black(45)),
            ..Default::default()
        }
    }

    #[test]
    fn a_connected_feed_carries_the_snapshot_values() {
        let feed = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            true,
        );

        assert_eq!(feed.schema_version, SCHEMA_VERSION);
        assert!(feed.connected);
        assert_eq!(feed.secs_in_period, Some(431));
        assert_eq!(feed.black_score, Some(3));
        assert_eq!(feed.white_score, Some(2));
        assert_eq!(feed.game_number.as_deref(), Some("10"));
        assert_eq!(feed.next_game_number.as_deref(), Some("20"));
        assert_eq!(feed.is_old_game, Some(false));
        assert_eq!(feed.next_period_len_secs, Some(180));
    }

    /// `period` and `timeout.kind` must be the same strings the vMix tables serve -- one vocabulary
    /// for both consumers. Asserted against `tables`' own helpers rather than hardcoded text, so
    /// the two cannot drift apart silently.
    #[test]
    fn period_and_timeout_use_the_same_words_as_the_vmix_tables() {
        let snapshot = live_snapshot();
        let feed = game_feed(
            &display_with(snapshot.clone()),
            None,
            &Rosters::default(),
            true,
        );

        assert_eq!(
            feed.period.as_deref(),
            Some(snapshot.current_period.to_string().as_str())
        );

        let timeout = feed.timeout.expect("a timeout was set on the snapshot");
        assert_eq!(
            timeout.kind,
            crate::tables::timeout_label(TimeoutSnapshot::Black(45))
        );
        assert_eq!(timeout.secs_remaining, 45);
    }

    #[test]
    fn a_goal_is_carried_as_the_refbox_sent_it() {
        let feed = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            true,
        );
        let goal = feed.recent_goal.expect("a recent goal was set");
        assert_eq!(goal.team, "BLACK");
        assert_eq!(goal.player, 7);
    }

    /// The team names come from the bridge's own portal lookup, which is what stops a consumer
    /// resolving an id against a portal of its own and getting real names for the wrong
    /// tournament. Unresolved serves `null`, never a placeholder.
    #[test]
    fn team_names_come_from_the_bridge_not_from_an_id() {
        let names = TeamNames {
            dark: Some("Dark Team".to_string()),
            light: Some("Light Team".to_string()),
            court: None,
            start_time: None,
        };
        let feed = game_feed(
            &display_with(live_snapshot()),
            Some(&names),
            &Rosters::default(),
            true,
        );
        assert_eq!(feed.black_team.as_deref(), Some("Dark Team"));
        assert_eq!(feed.white_team.as_deref(), Some("Light Team"));

        let unresolved = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            true,
        );
        assert_eq!(
            unresolved.black_team, None,
            "an unresolved name serves null, never a placeholder"
        );
        assert_eq!(unresolved.white_team, None);
    }

    /// Every game value reads `null`, `connected` is `false`, and -- the part a key-count check
    /// would miss -- every key is still present, so a consumer indexing by name finds a null
    /// rather than nothing.
    #[test]
    fn a_disconnected_feed_nulls_every_game_value_and_keeps_every_key() {
        let feed = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            false,
        );

        assert!(!feed.connected);
        assert_eq!(
            feed.schema_version, SCHEMA_VERSION,
            "the version must survive a disconnect -- that is exactly when a consumer needs it"
        );
        assert_eq!(feed.period, None);
        assert_eq!(feed.secs_in_period, None);
        assert_eq!(feed.black_score, None);
        assert_eq!(feed.white_score, None);
        assert_eq!(feed.black_team, None);
        assert_eq!(feed.white_team, None);
        assert_eq!(feed.timeout, None);
        assert_eq!(feed.game_number, None);
        assert_eq!(feed.next_game_number, None);
        assert_eq!(feed.is_old_game, None);
        assert_eq!(feed.recent_goal, None);
        assert_eq!(feed.next_period_len_secs, None);
        assert_eq!(feed.penalties, None);

        let json = serde_json::to_value(&feed).expect("the feed should serialise");
        for key in [
            "schemaVersion",
            "connected",
            "period",
            "secsInPeriod",
            "blackScore",
            "whiteScore",
            "blackTeam",
            "whiteTeam",
            "timeout",
            "gameNumber",
            "nextGameNumber",
            "isOldGame",
            "recentGoal",
            "nextPeriodLenSecs",
            "penalties",
            "eventId",
            "portalBaseUrl",
        ] {
            assert!(
                json.get(key).is_some(),
                "key {key} must be present even when null -- an absent key and a null are \
                 different things to a consumer"
            );
        }
    }

    /// A blanked score must never read as a real one. `0` is the specific danger: it is a
    /// plausible value, and plausible values invented during an outage are what produced the
    /// phantom 0-0 result bug.
    #[test]
    fn a_disconnected_score_is_null_not_zero() {
        let feed = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            false,
        );
        let json = serde_json::to_value(&feed).expect("the feed should serialise");
        assert!(json["blackScore"].is_null());
        assert!(json["whiteScore"].is_null());
        assert!(json["secsInPeriod"].is_null());
    }
    #[test]
    fn penalties_are_neither_padded_nor_truncated() {
        // /penalties pads up to ten rows and takes only the first ten, because a vMix title needs
        // a fixed row count to bind to. An array needs neither, and the renderer is better served
        // by the truth -- the same reasoning /scorebug already applies to its untruncated foul
        // counts.
        let empty = game_feed(
            &display_with(GameSnapshot::default()),
            None,
            &Rosters::default(),
            true,
        );
        assert_eq!(
            empty.penalties.as_deref(),
            Some(&[][..]),
            "no penalties must serve an empty array, not ten blank rows"
        );

        let mut snapshot = GameSnapshot::default();
        snapshot.penalties.black = (1..=12)
            .map(|n| PenaltySnapshot {
                player_number: n,
                time: PenaltyTime::Seconds(u16::from(n) * 10),
                infraction: Infraction::Unknown,
            })
            .collect();
        let many = game_feed(&display_with(snapshot), None, &Rosters::default(), true);
        assert_eq!(
            many.penalties.expect("connected").len(),
            12,
            "all twelve must be served, not the first ten"
        );
    }

    #[test]
    fn a_total_dismissal_is_a_flag_and_a_null_never_td_or_zero() {
        let mut snapshot = GameSnapshot::default();
        snapshot.penalties.white = vec![PenaltySnapshot {
            player_number: 4,
            time: PenaltyTime::TotalDismissal,
            infraction: Infraction::Unknown,
        }];
        let feed = game_feed(&display_with(snapshot), None, &Rosters::default(), true);
        let penalties = feed.penalties.clone().expect("connected");
        let penalty = &penalties[0];

        assert_eq!(penalty.team, "WHITE");
        assert_eq!(penalty.number, 4);
        assert!(penalty.total_dismissal);
        assert_eq!(
            penalty.secs_remaining, None,
            "a dismissal has no countdown -- 0 would read as about to expire"
        );

        let json = serde_json::to_value(&feed).expect("serialise");
        assert!(json["penalties"][0]["secsRemaining"].is_null());
        assert_eq!(json["penalties"][0]["totalDismissal"].as_bool(), Some(true));
        assert_ne!(
            json["penalties"][0]["secsRemaining"].as_str(),
            Some("TD"),
            "the vMix string encoding must not leak into the typed feed"
        );
    }

    #[test]
    fn a_penalty_carries_the_roster_name_when_the_cap_number_is_known() {
        let mut snapshot = GameSnapshot::default();
        snapshot.penalties.black = vec![
            PenaltySnapshot {
                player_number: 7,
                time: PenaltyTime::Seconds(60),
                infraction: Infraction::Unknown,
            },
            PenaltySnapshot {
                player_number: 9,
                time: PenaltyTime::Seconds(30),
                infraction: Infraction::Unknown,
            },
        ];
        let mut rosters = Rosters::default();
        rosters.black.insert(7, "Known Player".to_string());

        let feed = game_feed(&display_with(snapshot), None, &rosters, true);
        let served = feed.penalties.expect("connected");

        let known = served
            .iter()
            .find(|penalty| penalty.number == 7)
            .expect("cap 7 should be served");
        let unknown = served
            .iter()
            .find(|penalty| penalty.number == 9)
            .expect("cap 9 should be served");

        assert_eq!(known.player.as_deref(), Some("Known Player"));
        assert_eq!(
            unknown.player, None,
            "an unknown cap number serves null, never a placeholder"
        );
    }
}
