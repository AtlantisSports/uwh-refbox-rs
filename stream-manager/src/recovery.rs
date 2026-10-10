//! Restart recovery (ADR 026 §9): after a restart, each court asks YouTube which of its
//! prepared videos is live, and carries on from today's. A live video from another day is only
//! reported, never carried on from.

use crate::{BoxError, portal::EventPlan, prepare::EventState, quota, youtube::YouTube};
use std::collections::HashMap;
use time::OffsetDateTime;

/// YouTube takes at most this many video ids in one status check.
const IDS_PER_CHECK: usize = 50;

/// The court's games on every day that have a prepared video, in schedule order: (game, video
/// id, whether the game is today). Every day is asked about so that a video still live from
/// another day is noticed, rather than a second one being put live by Start day.
pub fn court_videos(
    plan: &EventPlan,
    state: &EventState,
    court: &str,
    now: OffsetDateTime,
) -> Vec<(String, String, bool)> {
    let today = quota::todays_day(plan, court, now);
    plan.games
        .iter()
        .filter(|g| g.court == court)
        .filter_map(|g| {
            state.videos.get(&g.number).map(|video| {
                (
                    g.number.clone(),
                    video.broadcast_id.clone(),
                    Some(g.day) == today,
                )
            })
        })
        .collect()
}

/// From the court's games in order with each video's life cycle and whether the game is today:
/// the latest of today's that is live, plus every other one that is also still live (earlier
/// today, or on another day).
pub fn pick_live(games_in_order: &[(&str, &str, bool)]) -> (Option<String>, Vec<String>) {
    let live: Vec<(&str, bool)> = games_in_order
        .iter()
        .filter(|(_, life_cycle, _)| *life_cycle == "live")
        .map(|(game, _, today)| (*game, *today))
        .collect();
    let latest = live
        .iter()
        .rev()
        .find(|(_, today)| *today)
        .map(|(game, _)| game.to_string());
    let others = live
        .iter()
        .map(|(game, _)| game.to_string())
        .filter(|game| latest.as_ref() != Some(game))
        .collect();
    (latest, others)
}

/// Asks YouTube for the life cycle of `videos` (game, video id, today), 50 at a time, and
/// picks the live one with [`pick_live`].
pub async fn find_live(
    youtube: &mut YouTube,
    videos: &[(String, String, bool)],
) -> Result<(Option<String>, Vec<String>), BoxError> {
    let mut life_cycles: HashMap<String, String> = HashMap::new();
    for chunk in videos.chunks(IDS_PER_CHECK) {
        let ids: Vec<&str> = chunk.iter().map(|(_, id, _)| id.as_str()).collect();
        for [id, _title, life_cycle, _privacy, _stream] in youtube.broadcast_statuses(&ids).await? {
            life_cycles.insert(id, life_cycle);
        }
    }
    let in_order: Vec<(&str, &str, bool)> = videos
        .iter()
        .map(|(game, id, today)| {
            (
                game.as_str(),
                life_cycles.get(id).map_or("", String::as_str),
                *today,
            )
        })
        .collect();
    Ok(pick_live(&in_order))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{portal::parse_event_plan, prepare::VideoState};
    use time::macros::datetime;

    #[test]
    fn no_live_video_resumes_nothing() {
        assert_eq!(pick_live(&[]), (None, vec![]));
        assert_eq!(
            pick_live(&[
                ("1", "complete", true),
                ("3", "ready", true),
                ("5", "testing", true)
            ]),
            (None, vec![])
        );
    }

    #[test]
    fn one_live_video_is_resumed() {
        assert_eq!(
            pick_live(&[
                ("12", "complete", true),
                ("14", "live", true),
                ("15", "ready", true)
            ]),
            (Some("14".to_string()), vec![])
        );
    }

    #[test]
    fn with_two_live_videos_the_later_one_is_resumed_and_the_other_reported() {
        assert_eq!(
            pick_live(&[
                ("13", "live", true),
                ("14", "live", true),
                ("15", "ready", true)
            ]),
            (Some("14".to_string()), vec!["13".to_string()])
        );
    }

    #[test]
    fn a_live_video_from_another_day_is_reported_but_never_resumed() {
        // Only another day's video is live: nothing is resumed, and it is reported.
        assert_eq!(
            pick_live(&[
                ("7", "live", false),
                ("12", "ready", true),
                ("13", "ready", true)
            ]),
            (None, vec!["7".to_string()])
        );
        // Today's live video is resumed even when a later day's (e.g. a test event's) is live.
        assert_eq!(
            pick_live(&[
                ("7", "live", false),
                ("12", "live", true),
                ("20", "live", false)
            ]),
            (
                Some("12".to_string()),
                vec!["7".to_string(), "20".to_string()]
            )
        );
    }

    #[test]
    fn court_videos_are_this_courts_prepared_games_on_every_day_in_order() {
        let game = |number: &str, start: &str, court: &str| {
            format!(
                r#"{{ "number": "{number}", "startsOn": "{start}", "court": "{court}",
                    "dark": {{ "assignment": null }}, "light": {{ "assignment": null }} }}"#
            )
        };
        let plan = parse_event_plan(&format!(
            r#"{{ "event": {{ "name": "Test Cup" }}, "games": [ {}, {}, {}, {}, {} ] }}"#,
            game("1", "2026-08-01T09:00:00+10:00", "1"),
            game("2", "2026-08-01T09:00:00+10:00", "2"),
            game("3", "2026-08-01T10:00:00+10:00", "1"),
            game("5", "2026-08-01T11:00:00+10:00", "1"),
            game("7", "2026-08-02T09:00:00+10:00", "1"),
        ))
        .unwrap();
        let video = |id: &str| VideoState {
            broadcast_id: id.to_string(),
            title: String::new(),
            description: String::new(),
            scheduled_start: String::new(),
            bound_stream: None,
            in_playlist: true,
            next_game_link: None,
            portal_start: None,
            court: None,
            day: None,
            thumbnail: None,
        };
        let mut state = EventState::default();
        // Game 3 has no video yet.
        for (number, id) in [("1", "v1"), ("2", "v2"), ("5", "v5"), ("7", "v7")] {
            state.videos.insert(number.to_string(), video(id));
        }
        let rows = |list: &[(&str, &str, bool)]| -> Vec<(String, String, bool)> {
            list.iter()
                .map(|(g, id, today)| (g.to_string(), id.to_string(), *today))
                .collect()
        };
        let day_one = datetime!(2026-08-01 08:00 +10);
        assert_eq!(
            court_videos(&plan, &state, "1", day_one),
            rows(&[("1", "v1", true), ("5", "v5", true), ("7", "v7", false)])
        );
        assert_eq!(
            court_videos(&plan, &state, "2", day_one),
            rows(&[("2", "v2", true)])
        );
        assert_eq!(
            court_videos(&plan, &state, "1", datetime!(2026-08-02 08:00 +10)),
            rows(&[("1", "v1", false), ("5", "v5", false), ("7", "v7", true)])
        );
        // No games today: every video is still asked about, none as today's.
        assert_eq!(
            court_videos(&plan, &state, "1", datetime!(2026-08-05 08:00 +10)),
            rows(&[("1", "v1", false), ("5", "v5", false), ("7", "v7", false)])
        );
        // A court with no prepared videos has nothing to ask about.
        assert_eq!(court_videos(&plan, &state, "3", day_one), vec![]);
    }
}
