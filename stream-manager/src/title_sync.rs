//! Keeps a court's upcoming YouTube videos in step with the portal during the day (ADR 026 §2):
//! titles (placeholder teams resolved), descriptions and start times. Games removed from the
//! portal are reported, never deleted.

use crate::{
    BoxError,
    app::App,
    config::{Config, CourtConfig},
    portal::{self, EventPlan, PlannedGame},
    prepare::{self, EventState},
    quota,
    youtube::{YouTube, YouTubeAccess},
};
use time::OffsetDateTime;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncReport {
    /// Videos changed on YouTube.
    pub updated: usize,
    /// Games with a video that are no longer on the portal (the videos are kept).
    pub removed: Vec<String>,
}

/// The schedule day to check: that of `only_game`, else of the live game, else today's.
fn sync_day(
    plan: &EventPlan,
    court: &str,
    live: Option<&str>,
    only_game: Option<&str>,
    now: OffsetDateTime,
) -> Option<usize> {
    only_game
        .or(live)
        .and_then(|game| plan.game(game))
        .filter(|game| game.court == court)
        .map(|game| game.day)
        .or_else(|| quota::todays_day(plan, court, now))
}

/// The games to check: just `only_game` (if it's on this court), or else the court's games on
/// `day` that come after the live one, or all of them with nothing live.
pub fn games_to_sync<'a>(
    plan: &'a EventPlan,
    court: &str,
    day: usize,
    live: Option<&str>,
    only_game: Option<&str>,
) -> Vec<&'a PlannedGame> {
    if let Some(only) = only_game {
        return plan
            .game(only)
            .filter(|game| game.court == court)
            .into_iter()
            .collect();
    }
    let todays: Vec<&PlannedGame> = plan.court_games(court, day).collect();
    match live.and_then(|live| todays.iter().position(|g| g.number == live)) {
        Some(position) => todays[position + 1..].to_vec(),
        None => todays,
    }
}

/// Games recorded in the state for `court` on `day` that are no longer on the portal. A record
/// that doesn't say its court and day (written before they were recorded) is skipped.
pub fn removed_games(state: &EventState, plan: &EventPlan, court: &str, day: usize) -> Vec<String> {
    state
        .videos
        .iter()
        .filter(|(number, video)| {
            video.court.as_deref() == Some(court)
                && video.day == Some(day)
                && plan.game(number).is_none()
        })
        .map(|(number, _)| number.clone())
        .collect()
}

/// What one check works with: the settings, the fresh schedule and the games to check.
pub struct Check {
    config: Config,
    plan: EventPlan,
    day: Option<usize>,
    games: Vec<String>,
}

/// Fetches the schedule, replaces the app's cached copy, and picks the games to check. Needs no
/// YouTube connection, so a switch can run it while it waits for YouTube.
pub async fn begin(
    app: &App,
    court: &CourtConfig,
    only_game: Option<&str>,
) -> Result<Check, BoxError> {
    let config = app.config();
    let plan = portal::fetch_event_plan(&config.portal_url, &config.event_slug).await?;
    app.set_plan(&config.portal_url, &config.event_slug, plan.clone());
    let live = app.live_game(&court.name);
    let day = sync_day(
        &plan,
        &court.name,
        live.as_deref(),
        only_game,
        OffsetDateTime::now_utc(),
    );
    let games = day
        .map(|day| {
            games_to_sync(&plan, &court.name, day, live.as_deref(), only_game)
                .into_iter()
                .map(|g| g.number.clone())
                .collect()
        })
        .unwrap_or_default();
    Ok(Check {
        config,
        plan,
        day,
        games,
    })
}

/// Updates the checked games' videos where the portal has changed. Returns how many changed.
/// The court's lock is taken one game at a time, so a switch can go ahead in between.
async fn update(
    app: &App,
    youtube: &mut YouTubeAccess<'_>,
    check: &Check,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<usize, BoxError> {
    let state_file = app.state_file()?;
    let mut updated = 0;
    for game in check.games.iter().filter_map(|g| check.plan.game(g)) {
        let mut yt = youtube.step(&game.court).await?;
        // Read afresh: a switch may have changed the record while the court's lock was free.
        let mut state = prepare::load_state(&state_file, &check.config.event_slug)?;
        let changed = prepare::sync_video(
            &mut yt,
            &check.config,
            &check.plan,
            game,
            &mut state,
            &state_file,
            log,
        )
        .await?;
        if changed {
            updated += 1;
        }
    }
    Ok(updated)
}

/// Lists the court's games gone from the portal, and notes the ones not reported yet today.
fn finish(
    app: &App,
    court: &CourtConfig,
    check: &Check,
    updated: usize,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<SyncReport, BoxError> {
    let removed = match check.day {
        Some(day) => {
            let state = prepare::load_state(&app.state_file()?, &check.config.event_slug)?;
            removed_games(&state, &check.plan, &court.name, day)
        }
        None => Vec::new(),
    };
    for game in app.newly_removed(&court.name, &removed) {
        log(format!(
            "⚠ Game {game} is no longer on the portal; its video was kept"
        ));
    }
    Ok(SyncReport { updated, removed })
}

/// Checks the court's games today that haven't been live yet (or only `only_game`) against the
/// portal and updates their videos where it changed.
pub async fn sync_court(
    app: &App,
    court: &CourtConfig,
    only_game: Option<&str>,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<SyncReport, BoxError> {
    let check = begin(app, court, only_game).await?;
    let updated = if check.games.is_empty() {
        0
    } else {
        update(app, &mut YouTubeAccess::shared(app), &check, log).await?
    };
    finish(app, court, &check, updated, log)
}

/// [`sync_court`] for a caller that already holds the court's lock (a switch).
pub async fn sync_court_with(
    app: &App,
    yt: &mut YouTube,
    court: &CourtConfig,
    only_game: Option<&str>,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<SyncReport, BoxError> {
    let check = begin(app, court, only_game).await?;
    complete(app, yt, court, &check, log).await
}

/// The rest of [`sync_court_with`] after [`begin`]: updates the videos where the portal has
/// changed and reports the removed games.
pub async fn complete(
    app: &App,
    yt: &mut YouTube,
    court: &CourtConfig,
    check: &Check,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<SyncReport, BoxError> {
    let updated = update(app, &mut YouTubeAccess::Held(yt), check, log).await?;
    finish(app, court, check, updated, log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{portal::parse_event_plan, prepare::VideoState};
    use time::macros::datetime;

    fn plan(games: &[(&str, &str, &str)]) -> EventPlan {
        let games: Vec<String> = games
            .iter()
            .map(|(number, start, court)| {
                format!(
                    r#"{{ "number": "{number}", "startsOn": "{start}", "court": "{court}",
                        "dark": {{ "assignment": null }}, "light": {{ "assignment": null }} }}"#
                )
            })
            .collect();
        parse_event_plan(&format!(
            r#"{{ "event": {{ "name": "Test Cup" }}, "games": [ {} ] }}"#,
            games.join(", ")
        ))
        .unwrap()
    }

    fn sample() -> EventPlan {
        plan(&[
            ("20", "2026-08-01T09:00:00+10:00", "1"),
            ("21", "2026-08-01T09:00:00+10:00", "2"),
            ("22", "2026-08-01T10:00:00+10:00", "1"),
            ("24", "2026-08-01T11:00:00+10:00", "1"),
            ("30", "2026-08-02T09:00:00+10:00", "1"),
        ])
    }

    fn numbers(games: Vec<&PlannedGame>) -> Vec<&str> {
        games.into_iter().map(|g| g.number.as_str()).collect()
    }

    #[test]
    fn only_games_after_the_live_one_are_checked() {
        let plan = sample();
        assert_eq!(
            numbers(games_to_sync(&plan, "1", 1, Some("22"), None)),
            ["24"]
        );
        // Nothing live: all of today's games on the court.
        assert_eq!(
            numbers(games_to_sync(&plan, "1", 1, None, None)),
            ["20", "22", "24"]
        );
        // Just before a switch, only the game about to go live.
        assert_eq!(
            numbers(games_to_sync(&plan, "1", 1, Some("20"), Some("22"))),
            ["22"]
        );
        // A game on another court is never checked for this one.
        assert!(games_to_sync(&plan, "1", 1, None, Some("21")).is_empty());
    }

    #[test]
    fn the_day_checked_is_the_live_games_else_todays() {
        let plan = sample();
        let day_one = datetime!(2026-08-01 08:00 +10);
        let day_two = datetime!(2026-08-02 08:00 +10);
        assert_eq!(sync_day(&plan, "1", None, None, day_one), Some(1));
        assert_eq!(sync_day(&plan, "1", None, None, day_two), Some(2));
        // Still on day 1's last game after midnight.
        assert_eq!(sync_day(&plan, "1", Some("24"), None, day_two), Some(1));
        assert_eq!(
            sync_day(&plan, "1", Some("20"), Some("30"), day_one),
            Some(2)
        );
        assert_eq!(
            sync_day(&plan, "1", None, None, datetime!(2026-08-05 08:00 +10)),
            None
        );
    }

    fn video(number: &str, court_and_day: Option<(&str, usize)>) -> VideoState {
        VideoState {
            broadcast_id: format!("v{number}"),
            title: format!("Test Cup · Court 1 · Game {number} · TBD vs TBD"),
            description: String::new(),
            scheduled_start: "2026-08-01T12:00:00+10:00".to_string(),
            bound_stream: None,
            in_playlist: true,
            next_game_link: None,
            portal_start: Some("2026-08-01T12:00:00+10:00".to_string()),
            court: court_and_day.map(|(court, _)| court.to_string()),
            day: court_and_day.map(|(_, day)| day),
        }
    }

    #[test]
    fn a_game_gone_from_the_portal_is_listed_as_removed() {
        let mut state = EventState::default();
        for (number, court_and_day) in [
            ("20", Some(("1", 1))),
            ("22", Some(("1", 1))),
            // Gone from the portal: game 23 on court 1, day 1.
            ("23", Some(("1", 1))),
            // Also gone, but on court 2, and on day 2.
            ("25", Some(("2", 1))),
            ("31", Some(("1", 2))),
            // Gone, but an older record that doesn't say its court or day: skipped.
            ("26", None),
        ] {
            state
                .videos
                .insert(number.to_string(), video(number, court_and_day));
        }
        let plan = sample();
        assert_eq!(removed_games(&state, &plan, "1", 1), ["23"]);
        assert_eq!(removed_games(&state, &plan, "2", 1), ["25"]);
        assert_eq!(removed_games(&state, &plan, "1", 2), ["31"]);
    }
}
