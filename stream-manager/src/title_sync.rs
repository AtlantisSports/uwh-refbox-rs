//! Keeps a court's upcoming YouTube videos in step with the portal during the day (ADR 026 §2):
//! titles (placeholder teams resolved), descriptions and start times, and on the 10-minute
//! check their thumbnails too. Games removed from the portal are reported, never deleted.

use crate::{
    BoxError,
    app::App,
    config::{Config, CourtConfig},
    portal::{self, EventPlan, PlannedGame},
    prepare::{self, EventState},
    quota,
    thumbnail::MissingGlyph,
    youtube::{self, THUMBNAIL_COST, YouTube, YouTubeAccess},
};
use std::{collections::HashSet, path::Path};
use time::{Date, OffsetDateTime};
use tokio::time::Instant;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncReport {
    /// Videos changed on YouTube.
    pub updated: usize,
    /// Games with a video that are no longer on the portal (the videos are kept).
    pub removed: Vec<String>,
}

/// What the 10-minute checks remember about thumbnails from one check to the next. Kept by the
/// app in memory, so a restart starts afresh.
#[derive(Debug, Default)]
pub struct ThumbnailMemory {
    /// The allowance day (Google's, in Pacific time) on which YouTube refused an upload.
    refused_on: Option<Date>,
    /// Courts already told their thumbnails are waiting to save the allowance for switches,
    /// until the allowance allows them again.
    told_saving: HashSet<String>,
    /// An event name the font can't draw, which is on every thumbnail.
    undrawable_event: Option<String>,
}

impl ThumbnailMemory {
    /// YouTube refused an upload at `now`: the checks upload no more thumbnails that day.
    pub fn note_refusal(&mut self, now: OffsetDateTime) {
        self.refused_on = Some(quota::pacific_date(now));
    }

    /// Whether YouTube refused an upload earlier on `now`'s allowance day.
    pub fn refused_today(&self, now: OffsetDateTime) -> bool {
        self.refused_on == Some(quota::pacific_date(now))
    }

    /// Notes whether the allowance lets `court` upload a thumbnail now. True when it doesn't
    /// and the court hasn't been told so since it last did, which is when to say so.
    pub fn saving_news(&mut self, court: &str, allowed: bool) -> bool {
        if allowed {
            self.told_saving.remove(court);
            false
        } else {
            self.told_saving.insert(court.to_string())
        }
    }

    /// The font can't draw `event_name`: no thumbnails until the event's name changes.
    pub fn note_undrawable_event(&mut self, event_name: &str) {
        self.undrawable_event = Some(event_name.to_string());
    }

    /// Whether `event_name` is one the font couldn't draw.
    pub fn undrawable_event(&self, event_name: &str) -> bool {
        self.undrawable_event.as_deref() == Some(event_name)
    }
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

/// Updates the checked games' videos where the portal has changed. Returns how many changed
/// (thumbnails not counted). The court's lock is taken one game at a time, so a switch can go
/// ahead in between. With `thumbnails_until`, once every title is checked, the thumbnails whose
/// content changed are drawn and uploaded again until then; never on a switch's own check,
/// which only has a few seconds.
async fn update(
    app: &App,
    youtube: &mut YouTubeAccess<'_>,
    check: &Check,
    thumbnails_until: Option<Instant>,
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
    if let Some(until) = thumbnails_until {
        // Whatever is left when the time is up waits for the next check.
        if Instant::now() < until {
            let thumbnails = update_thumbnails(app, check, &state_file, log);
            let _ = tokio::time::timeout_at(until, thumbnails).await;
        }
    }
    Ok(updated)
}

/// Asks whether the allowance lets `court` upload a thumbnail, and says it doesn't once until
/// it does again.
fn thumbnails_allowed(
    app: &App,
    court: &str,
    allowed: bool,
    log: &mut (dyn FnMut(String) + Send),
) -> bool {
    let news = app.thumbnail_memory().saving_news(court, allowed);
    if news {
        log("Thumbnails not uploaded — saving the allowance for switches".to_string());
    }
    allowed
}

/// The 10-minute check's thumbnails: draws and uploads again those of the checked games whose
/// content changed. Done without the court's lock, so neither a slow picture download nor an
/// upload ever holds up a switch. Problems are logged, never returned: the titles matter more
/// than their thumbnails. One stops the thumbnails for the rest of this check, and a refusal
/// from YouTube for the rest of Google's day.
async fn update_thumbnails(
    app: &App,
    check: &Check,
    state_file: &Path,
    log: &mut (dyn FnMut(String) + Send),
) {
    {
        let memory = app.thumbnail_memory();
        if memory.refused_today(OffsetDateTime::now_utc())
            || memory.undrawable_event(&check.plan.event_name)
        {
            return;
        }
    }
    let mut yt = match app.youtube() {
        Ok(yt) => yt,
        Err(e) => {
            log(format!("Thumbnails not checked — {e}"));
            return;
        }
    };
    let mut pictures = prepare::Thumbnails::default();
    for game in check.games.iter().filter_map(|g| check.plan.game(g)) {
        let key = match prepare::load_state(state_file, &check.config.event_slug) {
            Ok(state) => match prepare::thumbnail_to_redraw(&state, &check.plan, game) {
                Some(key) => key,
                None => continue,
            },
            Err(e) => {
                log(format!("Game {}: thumbnail not checked — {e}", game.number));
                continue;
            }
        };
        // Asked before drawing, so nothing is drawn or downloaded while the switches need the
        // allowance.
        let allowed = app.can_spend_extra(&game.court, THUMBNAIL_COST);
        if !thumbnails_allowed(app, &game.court, allowed, log) {
            return;
        }
        let redrawn = match prepare::redraw_thumbnail(
            &mut pictures,
            &check.plan,
            game,
            key.clone(),
            log,
        )
        .await
        {
            Ok(redrawn) => redrawn,
            Err(e) => {
                let (line, stop) = prepare::thumbnail_failed(&game.number, e.as_ref());
                log(line);
                match e.downcast_ref::<MissingGlyph>() {
                    Some(missing) if missing.in_event_name => app
                        .thumbnail_memory()
                        .note_undrawable_event(&check.plan.event_name),
                    Some(_) => {
                        let recorded =
                            prepare::record_undrawable(state_file, &check.config, game, key);
                        if let Err(e) = recorded {
                            log(format!(
                                "Game {}: the skip wasn't recorded, so it will be tried again — {e}",
                                game.number
                            ));
                        }
                    }
                    None => {}
                }
                if stop {
                    return;
                }
                continue;
            }
        };
        // Asked again just before the upload: the switches always come first.
        let extras_ok = || app.can_spend_extra(&game.court, THUMBNAIL_COST);
        let uploaded = prepare::upload_redrawn(
            &mut yt,
            &check.config,
            game,
            redrawn,
            state_file,
            &extras_ok,
            log,
        )
        .await;
        match uploaded {
            Ok(prepare::Upload::NotAllowed) => {
                thumbnails_allowed(app, &game.court, false, log);
                return;
            }
            Ok(_) => {}
            Err(e) if youtube::is_refusal(e.as_ref()) => {
                app.thumbnail_memory()
                    .note_refusal(OffsetDateTime::now_utc());
                log(format!(
                    "Thumbnails: YouTube refused an upload ({e}), so the 10-minute check \
                     uploads no more today. Prepare still tries."
                ));
                return;
            }
            Err(e) => {
                log(format!("Game {}: thumbnail not updated — {e}", game.number));
                return;
            }
        }
    }
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
/// portal and updates their videos where it changed, then their thumbnails until
/// `thumbnails_until`.
pub async fn sync_court(
    app: &App,
    court: &CourtConfig,
    only_game: Option<&str>,
    thumbnails_until: Instant,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<SyncReport, BoxError> {
    let check = begin(app, court, only_game).await?;
    let updated = if check.games.is_empty() {
        0
    } else {
        let mut youtube = YouTubeAccess::shared(app);
        update(app, &mut youtube, &check, Some(thumbnails_until), log).await?
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
    let updated = update(app, &mut YouTubeAccess::Held(yt), check, None, log).await?;
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

    #[test]
    fn a_refused_upload_stops_the_checks_thumbnails_until_googles_next_day() {
        let mut memory = ThumbnailMemory::default();
        // 10:00 in Sydney on 1 Aug is 17:00 on 31 Jul in Pacific time.
        let refused = datetime!(2026-08-01 10:00 +10);
        assert!(!memory.refused_today(refused));
        memory.note_refusal(refused);
        assert!(memory.refused_today(refused));
        // Google's day turns at 17:00 in Sydney.
        assert!(memory.refused_today(datetime!(2026-08-01 16:59 +10)));
        assert!(!memory.refused_today(datetime!(2026-08-01 17:00 +10)));
    }

    #[test]
    fn saving_the_allowance_is_said_once_per_court_until_it_allows_again() {
        let mut memory = ThumbnailMemory::default();
        assert!(!memory.saving_news("1", true));
        assert!(memory.saving_news("1", false));
        assert!(!memory.saving_news("1", false));
        // Another court is told on its own.
        assert!(memory.saving_news("2", false));
        // Allowed again, then not: told again.
        assert!(!memory.saving_news("1", true));
        assert!(memory.saving_news("1", false));
        assert!(!memory.saving_news("2", false));
    }

    #[test]
    fn an_event_name_the_font_cant_draw_is_tried_again_once_it_changes() {
        let mut memory = ThumbnailMemory::default();
        assert!(!memory.undrawable_event("東京 Cup"));
        memory.note_undrawable_event("東京 Cup");
        assert!(memory.undrawable_event("東京 Cup"));
        assert!(!memory.undrawable_event("Tokyo Cup"));
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
            thumbnail: None,
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
