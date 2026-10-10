//! Restart recovery (ADR 026 §9): after a restart, each court asks YouTube which of today's
//! prepared videos is live, and carries on from there.

use crate::{BoxError, portal::EventPlan, prepare::EventState, quota, youtube::YouTube};
use std::collections::HashMap;
use time::OffsetDateTime;

/// YouTube takes at most this many video ids in one status check.
const IDS_PER_CHECK: usize = 50;

/// The court's games today that have a prepared video, in schedule order: (game, video id).
pub fn todays_videos(
    plan: &EventPlan,
    state: &EventState,
    court: &str,
    now: OffsetDateTime,
) -> Vec<(String, String)> {
    let Some(day) = quota::todays_day(plan, court, now) else {
        return Vec::new();
    };
    plan.court_games(court, day)
        .filter_map(|g| {
            state
                .videos
                .get(&g.number)
                .map(|video| (g.number.clone(), video.broadcast_id.clone()))
        })
        .collect()
}

/// From the court's games in order with each video's life cycle, the latest one that is live,
/// plus any earlier ones that are also still live.
pub fn pick_live(games_in_order: &[(&str, &str)]) -> (Option<String>, Vec<String>) {
    let mut live: Vec<String> = games_in_order
        .iter()
        .filter(|(_, life_cycle)| *life_cycle == "live")
        .map(|(game, _)| game.to_string())
        .collect();
    let latest = live.pop();
    (latest, live)
}

/// Asks YouTube for the life cycle of `videos` (game, video id), 50 at a time, and picks the
/// live one with [`pick_live`].
pub async fn find_live(
    youtube: &mut YouTube,
    videos: &[(String, String)],
) -> Result<(Option<String>, Vec<String>), BoxError> {
    let mut life_cycles: HashMap<String, String> = HashMap::new();
    for chunk in videos.chunks(IDS_PER_CHECK) {
        let ids: Vec<&str> = chunk.iter().map(|(_, id)| id.as_str()).collect();
        for [id, _title, life_cycle, _privacy, _stream] in youtube.broadcast_statuses(&ids).await? {
            life_cycles.insert(id, life_cycle);
        }
    }
    let in_order: Vec<(&str, &str)> = videos
        .iter()
        .map(|(game, id)| {
            (
                game.as_str(),
                life_cycles.get(id).map_or("", String::as_str),
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
            pick_live(&[("1", "complete"), ("3", "ready"), ("5", "testing")]),
            (None, vec![])
        );
    }

    #[test]
    fn one_live_video_is_resumed() {
        assert_eq!(
            pick_live(&[("12", "complete"), ("14", "live"), ("15", "ready")]),
            (Some("14".to_string()), vec![])
        );
    }

    #[test]
    fn with_two_live_videos_the_later_one_is_resumed_and_the_other_reported() {
        assert_eq!(
            pick_live(&[("13", "live"), ("14", "live"), ("15", "ready")]),
            (Some("14".to_string()), vec!["13".to_string()])
        );
    }

    #[test]
    fn todays_videos_are_this_courts_prepared_games_today_in_order() {
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
        let pairs = |list: &[(&str, &str)]| -> Vec<(String, String)> {
            list.iter()
                .map(|(g, id)| (g.to_string(), id.to_string()))
                .collect()
        };
        let day_one = datetime!(2026-08-01 08:00 +10);
        assert_eq!(
            todays_videos(&plan, &state, "1", day_one),
            pairs(&[("1", "v1"), ("5", "v5")])
        );
        assert_eq!(
            todays_videos(&plan, &state, "2", day_one),
            pairs(&[("2", "v2")])
        );
        assert_eq!(
            todays_videos(&plan, &state, "1", datetime!(2026-08-02 08:00 +10)),
            pairs(&[("7", "v7")])
        );
        // No games today.
        assert_eq!(
            todays_videos(&plan, &state, "1", datetime!(2026-08-05 08:00 +10)),
            vec![]
        );
    }
}
