//! The refbox's own record of each saved colour swap, laid over every schedule it
//! receives so its screens keep showing the swap before the portal has it.

use std::collections::BTreeMap;
use uwh_common::{
    bundles::BlackWhiteBundle,
    uwhportal::schedule::{Game, GameNumber, Schedule, TeamId},
};

/// One saved swap: which team is now dark and which is now light, and whether the
/// portal has confirmed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SwapRecord {
    pub dark: TeamId,
    pub light: TeamId,
    pub delivered: bool,
}

/// Keyed by (event `full()` id, game number). Session memory only.
pub(super) type ColourSwaps = BTreeMap<(String, GameNumber), SwapRecord>;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum SwapFit {
    AlreadyAgrees,
    Swapped,
    NoMatch,
}

/// Swaps dark<->light AND dark_partner<->light_partner when the game has `light` on dark
/// and `dark` on light; leaves the game alone otherwise.
pub(super) fn apply_swap(game: &mut Game, dark: &TeamId, light: &TeamId) -> SwapFit {
    let game_dark = game.dark.assigned();
    let game_light = game.light.assigned();
    if game_dark == Some(dark) && game_light == Some(light) {
        SwapFit::AlreadyAgrees
    } else if game_dark == Some(light) && game_light == Some(dark) {
        std::mem::swap(&mut game.dark, &mut game.light);
        std::mem::swap(&mut game.dark_partner, &mut game.light_partner);
        SwapFit::Swapped
    } else {
        SwapFit::NoMatch
    }
}

/// Applies every record for `event_id` to `schedule`; removes a record that is
/// `delivered` AND `AlreadyAgrees` (the portal now shows it, so stop overriding).
pub(super) fn apply_colour_swaps(schedule: &mut Schedule, event_id: &str, swaps: &mut ColourSwaps) {
    swaps.retain(|(event, number), record| {
        if event != event_id {
            return true;
        }
        let Some(game) = schedule.games.get_mut(number) else {
            return true;
        };
        let fit = apply_swap(game, &record.dark, &record.light);
        !(record.delivered && fit == SwapFit::AlreadyAgrees)
    });
}

/// Some(dark id, light id) only when the game exists and BOTH sides have an assigned team.
pub(super) fn swappable_teams(
    schedule: Option<&Schedule>,
    game: &GameNumber,
) -> Option<BlackWhiteBundle<TeamId>> {
    let game = schedule?.games.get(game)?;
    Some(BlackWhiteBundle {
        black: game.dark.assigned()?.clone(),
        white: game.light.assigned()?.clone(),
    })
}

/// Marks the record delivered only when it still describes the send that was
/// delivered (`sent`, black = dark, white = light). The operator may have swapped
/// again since: an older send's delivery report must not mark the newer record
/// delivered, or the record would retire while the portal still holds the old colours.
pub(super) fn mark_delivered(
    swaps: &mut ColourSwaps,
    event_id: &str,
    game: &GameNumber,
    sent: &BlackWhiteBundle<TeamId>,
) {
    if let Some(record) = swaps.get_mut(&(event_id.to_string(), game.clone())) {
        if record.dark == sent.black && record.light == sent.white {
            record.delivered = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;
    use uwh_common::uwhportal::schedule::{EventId, ScheduledTeam};

    fn team(n: &str) -> TeamId {
        TeamId::from_partial(n)
    }

    fn st(n: &str) -> ScheduledTeam {
        ScheduledTeam::new_team_id(team(n))
    }

    fn game(number: &str, dark: ScheduledTeam, light: ScheduledTeam) -> Game {
        Game {
            number: number.to_string(),
            dark,
            light,
            dark_partner: None,
            light_partner: None,
            start_time: OffsetDateTime::UNIX_EPOCH,
            court: "Court 1".to_string(),
            timing_rule: "rule".to_string(),
            referee_assignments: None,
            description: None,
        }
    }

    fn schedule(event: &str, games: Vec<Game>) -> Schedule {
        Schedule {
            event_id: EventId::from_full(event).unwrap(),
            games: games.into_iter().map(|g| (g.number.clone(), g)).collect(),
            non_game_entries: Vec::new(),
            groups: Vec::new(),
            timing_rules: Vec::new(),
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }

    fn record(dark: &str, light: &str, delivered: bool) -> SwapRecord {
        SwapRecord {
            dark: team(dark),
            light: team(light),
            delivered,
        }
    }

    const X: &str = "events/xxx";
    const Y: &str = "events/yyy";

    #[test]
    fn apply_swap_moves_both_sides_and_partners() {
        let mut g = game("12", st("aaa"), st("bbb"));
        g.dark_partner = Some(st("ppp"));
        g.light_partner = Some(st("qqq"));
        assert_eq!(
            apply_swap(&mut g, &team("bbb"), &team("aaa")),
            SwapFit::Swapped
        );
        assert_eq!(g.dark, st("bbb"));
        assert_eq!(g.light, st("aaa"));
        assert_eq!(g.dark_partner, Some(st("qqq")));
        assert_eq!(g.light_partner, Some(st("ppp")));
    }

    #[test]
    fn apply_swap_is_a_no_op_when_the_portal_already_agrees() {
        let mut g = game("12", st("bbb"), st("aaa"));
        let before = g.clone();
        assert_eq!(
            apply_swap(&mut g, &team("bbb"), &team("aaa")),
            SwapFit::AlreadyAgrees
        );
        assert_eq!(g, before);
    }

    #[test]
    fn apply_swap_never_invents_a_team() {
        let mut g = game("12", st("aaa"), st("ccc"));
        let before = g.clone();
        assert_eq!(
            apply_swap(&mut g, &team("bbb"), &team("aaa")),
            SwapFit::NoMatch
        );
        assert_eq!(g, before);
    }

    #[test]
    fn undelivered_swap_survives_a_fresh_schedule() {
        let mut swaps = ColourSwaps::new();
        swaps.insert(
            (X.to_string(), "12".to_string()),
            record("bbb", "aaa", false),
        );
        let mut s = schedule(X, vec![game("12", st("aaa"), st("bbb"))]);
        apply_colour_swaps(&mut s, X, &mut swaps);
        assert_eq!(s.games["12"].dark, st("bbb"));
        assert_eq!(swaps.len(), 1);
        // A schedule that already agrees still keeps the undelivered record.
        let mut s = schedule(X, vec![game("12", st("bbb"), st("aaa"))]);
        apply_colour_swaps(&mut s, X, &mut swaps);
        assert_eq!(s.games["12"].dark, st("bbb"));
        assert_eq!(swaps.len(), 1);
    }

    #[test]
    fn delivered_swap_retires_once_the_portal_agrees() {
        let mut swaps = ColourSwaps::new();
        swaps.insert(
            (X.to_string(), "12".to_string()),
            record("bbb", "aaa", true),
        );
        // An old download still in flight: swapped, record kept.
        let mut s = schedule(X, vec![game("12", st("aaa"), st("bbb"))]);
        apply_colour_swaps(&mut s, X, &mut swaps);
        assert_eq!(s.games["12"].dark, st("bbb"));
        assert_eq!(swaps.len(), 1);
        // The portal now agrees: record removed.
        let mut s = schedule(X, vec![game("12", st("bbb"), st("aaa"))]);
        apply_colour_swaps(&mut s, X, &mut swaps);
        assert!(swaps.is_empty());
    }

    #[test]
    fn records_apply_only_to_their_own_event() {
        let mut swaps = ColourSwaps::new();
        swaps.insert(
            (X.to_string(), "12".to_string()),
            record("bbb", "aaa", false),
        );
        let mut s = schedule(Y, vec![game("12", st("aaa"), st("bbb"))]);
        apply_colour_swaps(&mut s, Y, &mut swaps);
        assert_eq!(s.games["12"].dark, st("aaa"));
        assert_eq!(swaps.len(), 1);
    }

    #[test]
    fn mark_delivered_flags_only_the_named_record() {
        let mut swaps = ColourSwaps::new();
        swaps.insert(
            (X.to_string(), "12".to_string()),
            record("bbb", "aaa", false),
        );
        swaps.insert(
            (X.to_string(), "13".to_string()),
            record("bbb", "aaa", false),
        );
        let sent = BlackWhiteBundle {
            black: team("bbb"),
            white: team("aaa"),
        };
        mark_delivered(&mut swaps, X, &"12".to_string(), &sent);
        assert!(swaps[&(X.to_string(), "12".to_string())].delivered);
        assert!(!swaps[&(X.to_string(), "13".to_string())].delivered);
    }

    #[test]
    fn stale_delivery_report_leaves_the_newer_record_undelivered() {
        let mut swaps = ColourSwaps::new();
        let key = (X.to_string(), "12".to_string());
        swaps.insert(key.clone(), record("aaa", "bbb", false));
        let stale = BlackWhiteBundle {
            black: team("bbb"),
            white: team("aaa"),
        };
        mark_delivered(&mut swaps, X, &"12".to_string(), &stale);
        assert!(!swaps[&key].delivered);
        let current = BlackWhiteBundle {
            black: team("aaa"),
            white: team("bbb"),
        };
        mark_delivered(&mut swaps, X, &"12".to_string(), &current);
        assert!(swaps[&key].delivered);
    }

    #[test]
    fn swappable_teams_needs_two_real_teams() {
        let s = schedule(
            X,
            vec![
                game("12", st("aaa"), st("bbb")),
                game("13", st("aaa"), ScheduledTeam::new_winner_of("12")),
            ],
        );
        let both = swappable_teams(Some(&s), &"12".to_string()).unwrap();
        assert_eq!(both.black, team("aaa"));
        assert_eq!(both.white, team("bbb"));
        assert!(swappable_teams(Some(&s), &"13".to_string()).is_none());
        assert!(swappable_teams(Some(&s), &"99".to_string()).is_none());
        assert!(swappable_teams(None, &"12".to_string()).is_none());
    }
}
