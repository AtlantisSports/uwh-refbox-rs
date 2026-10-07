//! The "prepare" step: creates (or updates) the playlists and scheduled live videos for a day.
//!
//! Everything created is recorded in a state file next to the config, so running it again
//! only does what is still missing or has changed on the portal — never duplicates. The
//! record is saved after every single YouTube call, so an interrupted run picks up where it
//! stopped.

use crate::{
    BoxError,
    config::{Config, CourtConfig, StreamMode},
    portal::{EventPlan, PlannedGame, playlist_title, video_title},
    youtube::{BroadcastSpec, Playlist, StreamInfo, YouTube},
};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use time::{
    Duration, OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description,
};

/// Videos whose portal start time has already passed (e.g. testing with an old event) are
/// scheduled this far in the future instead, because YouTube expects an upcoming start time.
const PAST_START_OFFSET: Duration = Duration::minutes(15);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EventState {
    pub event_slug: String,
    /// Playlist title → playlist id.
    pub playlists: BTreeMap<String, String>,
    /// Game number → its video.
    pub videos: BTreeMap<String, VideoState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoState {
    pub broadcast_id: String,
    pub title: String,
    pub description: String,
    pub scheduled_start: String,
    /// Title of the stream key this video is bound to, once bound.
    pub bound_stream: Option<String>,
    pub in_playlist: bool,
    /// Link to the following game's video, added to the description once that game goes live.
    #[serde(default)]
    pub next_game_link: Option<String>,
}

/// The description as it should be on YouTube: the generated text plus, once known, the link
/// to the next game.
pub fn with_next_link(description: &str, next_game_link: Option<&str>) -> String {
    match next_game_link {
        Some(link) => format!(
            "{description}
Next game: {link}
"
        ),
        None => description.to_string(),
    }
}

pub fn state_path(config_dir: &Path, event_slug: &str) -> PathBuf {
    config_dir.join(format!("state-{event_slug}.json"))
}

pub fn load_state(path: &Path, event_slug: &str) -> Result<EventState, BoxError> {
    if !path.exists() {
        return Ok(EventState {
            event_slug: event_slug.to_string(),
            ..Default::default()
        });
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

pub fn save_state(path: &Path, state: &EventState) -> Result<(), BoxError> {
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

/// Asks the operator to type `yes` before anything is changed on YouTube.
pub fn confirm(question: &str) -> bool {
    print!("{question} Type 'yes' to continue: ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer).is_ok() && answer.trim().eq_ignore_ascii_case("yes")
}

fn normalize(name: &str) -> String {
    name.trim().replace(['–', '—'], "-").to_lowercase()
}

/// Finds a court's stream keys by name: A and B, or only A in one-key mode.
pub fn court_streams<'a>(
    court: &CourtConfig,
    streams: &'a [StreamInfo],
) -> Result<Vec<&'a StreamInfo>, String> {
    let names = court.stream_names();
    let find = |name: &String| {
        streams
            .iter()
            .find(|s| normalize(&s.title) == normalize(name))
            .ok_or_else(|| {
                format!(
                    "No stream key named \"{name}\" for court {}. Create it in YouTube Studio \
                     (Go live → Stream → new stream key) or set stream_a/stream_b in the config.",
                    court.name
                )
            })
    };
    match court.stream_mode {
        StreamMode::TwoKeys => Ok(vec![find(&names[0])?, find(&names[1])?]),
        StreamMode::OneKey => Ok(vec![find(&names[0])?]),
    }
}

/// The stream key the court's game at `position` must be (re)bound to, or `None` when its video
/// is already bound to it. In one-key mode a video bound to B is rebound to A.
fn stream_to_bind<'a>(
    court: &CourtConfig,
    keys: &[&'a StreamInfo],
    position: usize,
    bound: Option<&str>,
) -> Option<&'a StreamInfo> {
    // `keys` comes from `court_streams` for the same court: two keys in two-key mode, where
    // the position is 0 or 1, and one in one-key mode, where it is always 0.
    let wanted = keys[court.stream_for_position(position)];
    (bound != Some(wanted.title.as_str())).then_some(wanted)
}

/// The public portal web address for the event (derived from the API address).
fn portal_event_page(config: &Config) -> String {
    let web = config
        .portal_url
        .trim_end_matches('/')
        .replacen("://api.", "://", 1);
    format!("{web}/events/{}", config.event_slug)
}

fn broadcast_spec(config: &Config, plan: &EventPlan, game: &PlannedGame) -> BroadcastSpec {
    let start_text = game
        .start
        .format(format_description!(
            "[weekday repr:short] [day padding:none] [month repr:short] [year], [hour]:[minute] (UTC[offset_hour sign:mandatory]:[offset_minute])"
        ))
        .unwrap_or_default();
    let mut description = format!(
        "{}\nCourt {} · Game {}\n",
        plan.event_name, game.court, game.number
    );
    if let Some(details) = game.description.as_ref().filter(|d| !d.is_empty()) {
        description.push_str(details);
        description.push('\n');
    }
    description.push_str(&format!(
        "Scheduled start: {start_text}\n\nFull schedule: {}\n",
        portal_event_page(config)
    ));
    let earliest = OffsetDateTime::now_utc() + PAST_START_OFFSET;
    let start = if game.start < earliest {
        earliest
    } else {
        game.start
    };
    BroadcastSpec {
        title: video_title(&plan.event_name, game),
        // YouTube does not allow angle brackets in descriptions either.
        description: description.replace(['<', '>'], ""),
        scheduled_start: start.format(&Rfc3339).unwrap_or_default(),
        privacy: config.privacy.clone(),
    }
}

/// What the prepare step will do, worked out before anything is changed.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Work {
    /// (playlist title, number of videos) for every playlist covered.
    pub playlists: Vec<(String, usize)>,
    pub playlists_to_create: usize,
    pub videos_to_create: usize,
    pub videos_to_update: usize,
    pub binds: usize,
    pub playlist_adds: usize,
    pub units: usize,
    pub privacy: String,
}

impl Work {
    pub fn is_empty(&self) -> bool {
        self.playlists_to_create
            + self.videos_to_create
            + self.videos_to_update
            + self.binds
            + self.playlist_adds
            == 0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Selection {
    pub day: usize,
    /// Courts to include; empty means every court in the config.
    #[serde(default)]
    pub courts: Vec<String>,
    /// Only the first N games of each playlist (for testing).
    #[serde(default)]
    pub limit: Option<usize>,
}

struct Target<'a> {
    playlist_title: String,
    court: &'a CourtConfig,
    games: Vec<&'a PlannedGame>,
}

fn select_targets<'a>(
    config: &'a Config,
    plan: &'a EventPlan,
    selection: &Selection,
) -> Result<Vec<Target<'a>>, BoxError> {
    let mut targets = Vec::new();
    for ((day, court_name), games) in plan.playlists() {
        if day != selection.day
            || (!selection.courts.is_empty() && !selection.courts.contains(&court_name))
        {
            continue;
        }
        let Some(court) = config.courts.iter().find(|c| c.name == court_name) else {
            continue;
        };
        let games = games
            .into_iter()
            .take(selection.limit.unwrap_or(usize::MAX))
            .collect();
        targets.push(Target {
            playlist_title: playlist_title(day, &court_name),
            court,
            games,
        });
    }
    if targets.is_empty() {
        return Err(format!(
            "No games on day {} for the selected court(s).",
            selection.day
        )
        .into());
    }
    Ok(targets)
}

/// The YouTube data both the preview and the real run need.
pub struct Lookups {
    streams: Vec<StreamInfo>,
    existing_playlists: Vec<Playlist>,
}

pub async fn lookups(youtube: &mut YouTube) -> Result<Lookups, BoxError> {
    Ok(Lookups {
        streams: youtube.list_streams().await?,
        existing_playlists: youtube.list_playlists().await?,
    })
}

/// Works out what `run` would do, without changing anything.
pub fn preview(
    config: &Config,
    plan: &EventPlan,
    state: &EventState,
    lookups: &Lookups,
    selection: &Selection,
) -> Result<Work, BoxError> {
    let targets = select_targets(config, plan, selection)?;
    let mut work = Work {
        privacy: config.privacy.clone(),
        ..Default::default()
    };
    for target in &targets {
        let pair = court_streams(target.court, &lookups.streams)?;
        work.playlists
            .push((target.playlist_title.clone(), target.games.len()));
        if !state.playlists.contains_key(&target.playlist_title)
            && !lookups
                .existing_playlists
                .iter()
                .any(|p| p.title == target.playlist_title)
        {
            work.playlists_to_create += 1;
        }
        for (i, game) in target.games.iter().enumerate() {
            let spec = broadcast_spec(config, plan, game);
            match state.videos.get(&game.number) {
                None => {
                    work.videos_to_create += 1;
                    work.binds += 1;
                    work.playlist_adds += 1;
                }
                Some(v) => {
                    let description =
                        with_next_link(&spec.description, v.next_game_link.as_deref());
                    if v.title != spec.title || v.description != description {
                        work.videos_to_update += 1;
                    }
                    if stream_to_bind(target.court, &pair, i, v.bound_stream.as_deref()).is_some() {
                        work.binds += 1;
                    }
                    if !v.in_playlist {
                        work.playlist_adds += 1;
                    }
                }
            }
        }
    }
    work.units = 50
        * (work.playlists_to_create
            + work.videos_to_create
            + work.videos_to_update
            + work.binds
            + work.playlist_adds);
    Ok(work)
}

/// Creates/updates everything the selection needs, saving the record after every call.
/// Progress lines go to `log`.
pub async fn run(
    config: &Config,
    plan: &EventPlan,
    youtube: &mut YouTube,
    state_file: &Path,
    lookups: &Lookups,
    selection: &Selection,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<(), BoxError> {
    let mut state = load_state(state_file, &config.event_slug)?;
    for target in select_targets(config, plan, selection)? {
        let title = &target.playlist_title;
        let pair = court_streams(target.court, &lookups.streams)?;
        let playlist_id = match state.playlists.get(title) {
            Some(id) => id.clone(),
            None => {
                let id = match lookups
                    .existing_playlists
                    .iter()
                    .find(|p| &p.title == title)
                {
                    Some(p) => {
                        log(format!("Using existing playlist \"{title}\""));
                        p.id.clone()
                    }
                    None => {
                        let description = format!(
                            "{} — {title}\n{}",
                            plan.event_name,
                            portal_event_page(config)
                        );
                        let id = youtube
                            .create_playlist(title, &description, &config.privacy)
                            .await?;
                        log(format!("Created playlist \"{title}\""));
                        id
                    }
                };
                state.playlists.insert(title.clone(), id.clone());
                save_state(state_file, &state)?;
                id
            }
        };

        for (i, game) in target.games.iter().enumerate() {
            let spec = broadcast_spec(config, plan, game);

            let video = match state.videos.get(&game.number).cloned() {
                None => {
                    let id = youtube.insert_broadcast(&spec).await?;
                    log(format!("Created video: {}", spec.title));
                    let v = VideoState {
                        broadcast_id: id,
                        title: spec.title.clone(),
                        description: spec.description.clone(),
                        scheduled_start: spec.scheduled_start.clone(),
                        bound_stream: None,
                        in_playlist: false,
                        next_game_link: None,
                    };
                    state.videos.insert(game.number.clone(), v.clone());
                    save_state(state_file, &state)?;
                    v
                }
                Some(mut v) => {
                    let spec = BroadcastSpec {
                        description: with_next_link(&spec.description, v.next_game_link.as_deref()),
                        ..spec
                    };
                    if v.title != spec.title || v.description != spec.description {
                        youtube.update_broadcast(&v.broadcast_id, &spec).await?;
                        log(format!("Updated video: {}", spec.title));
                        v.title = spec.title.clone();
                        v.description = spec.description.clone();
                        v.scheduled_start = spec.scheduled_start.clone();
                        state.videos.insert(game.number.clone(), v.clone());
                        save_state(state_file, &state)?;
                    }
                    v
                }
            };

            if let Some(stream) =
                stream_to_bind(target.court, &pair, i, video.bound_stream.as_deref())
            {
                youtube
                    .bind_broadcast(&video.broadcast_id, &stream.id)
                    .await?;
                log(format!(
                    "Game {}: linked to stream key \"{}\"",
                    game.number, stream.title
                ));
                if let Some(v) = state.videos.get_mut(&game.number) {
                    v.bound_stream = Some(stream.title.clone());
                }
                save_state(state_file, &state)?;
            }

            if !video.in_playlist {
                youtube
                    .add_to_playlist(&playlist_id, &video.broadcast_id)
                    .await?;
                log(format!("Game {}: added to \"{title}\"", game.number));
                if let Some(v) = state.videos.get_mut(&game.number) {
                    v.in_playlist = true;
                }
                save_state(state_file, &state)?;
            }
        }
    }
    log(format!("Done. Used {} units so far.", youtube.units_used));
    Ok(())
}

/// Terminal version: preview, ask for `yes`, run.
pub async fn run_cli(
    config: &Config,
    plan: &EventPlan,
    youtube: &mut YouTube,
    state_file: &Path,
    selection: &Selection,
) -> Result<(), BoxError> {
    let state = load_state(state_file, &config.event_slug)?;
    let lookups = lookups(youtube).await?;
    let work = preview(config, plan, &state, &lookups, selection)?;
    println!("Event: {}  —  day {}", plan.event_name, selection.day);
    for (title, count) in &work.playlists {
        println!("  Playlist \"{title}\": {count} videos");
    }
    if work.is_empty() {
        println!("Everything is already up to date on YouTube. Nothing to do.");
        return Ok(());
    }
    println!(
        "\nThis will create {} playlist(s) and {} video(s), update {} video(s),\n\
         link {} video(s) to stream keys and add {} to playlists.\n\
         Privacy: {}. Estimated cost: about {} of the 10,000 daily units.",
        work.playlists_to_create,
        work.videos_to_create,
        work.videos_to_update,
        work.binds,
        work.playlist_adds,
        work.privacy,
        work.units + 2,
    );
    if !confirm("Go ahead?") {
        println!("Cancelled. Nothing was changed.");
        return Ok(());
    }
    run(
        config,
        plan,
        youtube,
        state_file,
        &lookups,
        selection,
        &mut |line| info!("{line}"),
    )
    .await
}

/// Deletes every video and playlist this event's record lists (for cleaning up after tests).
pub async fn cleanup(
    youtube: &mut YouTube,
    state_file: &Path,
    event_slug: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<(), BoxError> {
    let mut state = load_state(state_file, event_slug)?;
    while let Some((game, video)) = state.videos.pop_first() {
        youtube.delete_broadcast(&video.broadcast_id).await?;
        log(format!("Deleted video for game {game}"));
        save_state(state_file, &state)?;
    }
    while let Some((title, id)) = state.playlists.pop_first() {
        youtube.delete_playlist(&id).await?;
        log(format!("Deleted playlist \"{title}\""));
        save_state(state_file, &state)?;
    }
    log(format!("Done. Used {} units so far.", youtube.units_used));
    Ok(())
}

/// Terminal version of `cleanup`, with a warning and a `yes` confirmation.
pub async fn cleanup_cli(
    youtube: &mut YouTube,
    state_file: &Path,
    event_slug: &str,
) -> Result<(), BoxError> {
    let state = load_state(state_file, event_slug)?;
    if state.videos.is_empty() && state.playlists.is_empty() {
        println!("Nothing recorded for {event_slug}; nothing to delete.");
        return Ok(());
    }
    println!(
        "This PERMANENTLY deletes {} video(s) and {} playlist(s) created for {event_slug}\n\
         (about {} units). It cannot be undone.",
        state.videos.len(),
        state.playlists.len(),
        50 * (state.videos.len() + state.playlists.len()),
    );
    if !confirm("Delete them?") {
        println!("Cancelled. Nothing was deleted.");
        return Ok(());
    }
    cleanup(youtube, state_file, event_slug, &mut |line| info!("{line}")).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::parse_event_plan;

    fn stream(title: &str) -> StreamInfo {
        StreamInfo {
            id: format!("id-{title}"),
            title: title.to_string(),
            stream_status: "inactive".to_string(),
        }
    }

    #[test]
    fn finds_stream_keys_by_name_ignoring_dash_style_and_case() {
        let court = Config::default().courts.remove(0);
        let streams = [
            stream("court 1 – a"),
            stream("Court 1 — B"),
            stream("Other"),
        ];
        let keys = court_streams(&court, &streams).unwrap();
        assert_eq!(
            (keys[0].id.as_str(), keys[1].id.as_str()),
            ("id-court 1 – a", "id-Court 1 — B")
        );

        let err = court_streams(&court, &streams[..1]).unwrap_err();
        assert!(err.contains("Court 1 - B"));
    }

    #[test]
    fn one_key_mode_needs_only_stream_key_a() {
        let mut court = Config::default().courts.remove(0);
        court.stream_mode = StreamMode::OneKey;
        let streams = [stream("Court 1 - A")];
        let keys = court_streams(&court, &streams).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].title, "Court 1 - A");
    }

    #[test]
    fn rerunning_prepare_in_one_key_mode_rebinds_videos_on_b_to_a() {
        let mut court = Config::default().courts.remove(0);
        let both = [stream("Court 1 - A"), stream("Court 1 - B")];
        let keys: Vec<&StreamInfo> = both.iter().collect();
        // Two keys: game 2 (position 1) belongs on B, so a video already there stays put.
        assert!(stream_to_bind(&court, &keys, 1, Some("Court 1 - B")).is_none());

        court.stream_mode = StreamMode::OneKey;
        let keys = court_streams(&court, &both).unwrap();
        let rebind = stream_to_bind(&court, &keys, 1, Some("Court 1 - B")).unwrap();
        assert_eq!(rebind.title, "Court 1 - A");
        assert!(stream_to_bind(&court, &keys, 1, Some("Court 1 - A")).is_none());
        // A new video is always bound.
        assert_eq!(
            stream_to_bind(&court, &keys, 0, None).map(|s| s.title.as_str()),
            Some("Court 1 - A")
        );
    }

    #[test]
    fn description_links_portal_and_old_start_times_move_to_the_future() {
        let config = Config {
            event_slug: "test-cup".to_string(),
            ..Default::default()
        };
        let plan = parse_event_plan(
            r#"{ "event": { "name": "Test Cup" }, "games": [ {
                "number": "1", "startsOn": "2020-08-01T09:30:00+10:00", "court": "1",
                "description": "A Grade <RR>",
                "dark": { "assignment": null }, "light": { "assignment": null } } ] }"#,
        )
        .unwrap();
        let spec = broadcast_spec(&config, &plan, &plan.games[0]);
        assert_eq!(spec.title, "Test Cup · Court 1 · Game 1 · TBD vs TBD");
        assert!(spec.description.contains("A Grade RR"));
        assert!(
            spec.description
                .contains("Sat 1 Aug 2020, 09:30 (UTC+10:00)")
        );
        assert!(
            spec.description
                .contains("Full schedule: https://dev.uwhportal.com/events/test-cup")
        );
        let start = OffsetDateTime::parse(&spec.scheduled_start, &Rfc3339).unwrap();
        assert!(start > OffsetDateTime::now_utc());
        assert_eq!(spec.privacy, "unlisted");
    }
}
