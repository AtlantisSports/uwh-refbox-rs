//! Carries out a court's switching decisions on vMix and YouTube (ADR 026, §5 and §6).
//!
//! A switch from game A's video to game B's:
//!  1. vMix starts sending on B's stream key (unless A and B share one),
//!  2. once YouTube is receiving it, B's video goes live,
//!  3. a "Game B is live now" message goes into A's chat,
//!  4. A's video ends and vMix stops A's stream key,
//!  5. A's description gets a "Next game" link.
//!
//! If anything before step 2 completes fails, A's video stays live and the caller puts the
//! court on Hold. Problems after that are reported as warnings: B is live either way.
//!
//! In one-key mode vMix sends on stream key A all day: it starts at Start day and stops at End
//! day. Each switch ends the current video first, then puts the next one live on the running
//! stream.

use crate::{
    BoxError,
    app::App,
    config::{CourtConfig, StreamMode, vmix_destination},
    portal::EventPlan,
    prepare::{self, VideoState, with_next_link},
    switcher::{Action, GameNumber},
    vmix,
    youtube::{BroadcastSpec, YouTube},
};
use std::time::Duration;

const STREAM_WAIT: Duration = Duration::from_secs(90);
const LIVE_WAIT: Duration = Duration::from_secs(30);
const POLL_EVERY: Duration = Duration::from_secs(3);

pub enum Outcome {
    Done,
    /// The switch happened, but something around it needs attention.
    Warnings(Vec<String>),
    /// The switch didn't happen. `actually_live` is the game whose video is live now.
    Failed {
        actually_live: Option<GameNumber>,
        error: String,
    },
}

fn normalize(name: &str) -> String {
    name.trim().replace(['–', '—'], "-").to_lowercase()
}

fn video_of(
    state: &prepare::EventState,
    plan: Option<&EventPlan>,
    game: &str,
) -> Result<VideoState, String> {
    if let Some(plan) = plan.filter(|p| p.game(game).is_none()) {
        return Err(format!(
            "The refbox is on Game {game}, which isn't in {}'s schedule.              Set the refbox to this event and court.",
            plan.event_name
        ));
    }
    state.videos.get(game).cloned().ok_or_else(|| {
        format!("Game {game} has no YouTube video yet. Create it on the Prepare tab.")
    })
}

/// Which of the court's two stream keys (0 = A, 1 = B) a video is linked to.
fn stream_index(court: &CourtConfig, game: &str, video: &VideoState) -> Result<usize, String> {
    let bound = video.bound_stream.as_deref().ok_or_else(|| {
        format!("Game {game}'s video isn't linked to a stream key. Run Prepare again.")
    })?;
    court
        .stream_names()
        .iter()
        .position(|n| normalize(n) == normalize(bound))
        .ok_or_else(|| {
            format!(
                "Game {game}'s video is linked to \"{bound}\", which isn't one of Court {}'s stream keys",
                court.name
            )
        })
}

/// In one-key mode, checks that every one of `games` that has a video is linked to stream key
/// A. Videos linked to B would never receive anything, because vMix only sends on A.
fn one_key_ready(
    court: &CourtConfig,
    state: &prepare::EventState,
    games: &[&str],
) -> Result<(), String> {
    if court.stream_mode != StreamMode::OneKey {
        return Ok(());
    }
    let [key_a, _] = court.stream_names();
    let on_other_key = games.iter().any(|game| {
        state
            .videos
            .get(*game)
            .and_then(|v| v.bound_stream.as_deref())
            .is_some_and(|bound| normalize(bound) != normalize(&key_a))
    });
    if on_other_key {
        return Err(
            "Re-run Prepare: some of today's videos use a stream key other than A".to_string(),
        );
    }
    Ok(())
}

/// The court's games on the same day as `game`, from the schedule (just `game` without one).
fn todays_games<'a>(
    plan: Option<&'a EventPlan>,
    court: &CourtConfig,
    game: &'a str,
) -> Vec<&'a str> {
    let Some((plan, day)) = plan.and_then(|p| p.game(game).map(|g| (p, g.day))) else {
        return vec![game];
    };
    plan.games
        .iter()
        .filter(|g| g.court == court.name && g.day == day)
        .map(|g| g.number.as_str())
        .collect()
}

/// vMix destinations to stop at End day. With two keys, both, so nothing is left streaming
/// even if one was started by hand. With one key, only destination 1: destination 2 is left
/// alone, as a venue may use it for something else.
fn destinations_to_stop(court: &CourtConfig) -> &'static [u8] {
    match court.stream_mode {
        StreamMode::TwoKeys => &[1, 2],
        StreamMode::OneKey => &[1],
    }
}

async fn stream_id(yt: &mut YouTube, title: &str) -> Result<String, BoxError> {
    let streams = yt.list_streams().await?;
    streams
        .into_iter()
        .find(|s| normalize(&s.title) == normalize(title))
        .map(|s| s.id)
        .ok_or_else(|| format!("Stream key \"{title}\" not found on the channel").into())
}

/// Waits until YouTube reports video arriving on the stream key.
async fn wait_for_stream(yt: &mut YouTube, stream_id: &str) -> Result<(), BoxError> {
    let started = tokio::time::Instant::now();
    loop {
        if yt.stream_status(stream_id).await? == "active" {
            return Ok(());
        }
        if started.elapsed() > STREAM_WAIT {
            return Err(
                "YouTube isn't receiving video from vMix on that stream key (waited 90 s). \
                 Check vMix's streaming destination and the internet connection."
                    .into(),
            );
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
}

/// Puts a video live (its stream must already be receiving video).
async fn go_live(
    yt: &mut YouTube,
    game: &str,
    video: &VideoState,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<(), BoxError> {
    let info = yt.broadcast_info(&video.broadcast_id).await?;
    match info.life_cycle.as_str() {
        "live" => return Ok(()),
        "complete" | "revoked" => {
            return Err(format!(
                "Game {game}'s YouTube video has already ended; a finished live video can't be restarted"
            )
            .into());
        }
        _ => {}
    }
    yt.transition(&video.broadcast_id, "live").await?;
    let started = tokio::time::Instant::now();
    while started.elapsed() < LIVE_WAIT {
        tokio::time::sleep(POLL_EVERY).await;
        if yt.broadcast_info(&video.broadcast_id).await?.life_cycle == "live" {
            return Ok(());
        }
    }
    log(format!(
        "Game {game}: YouTube is still starting the video; continuing"
    ));
    Ok(())
}

pub async fn carry_out(
    app: &App,
    court: &CourtConfig,
    plan: Option<&EventPlan>,
    action: &Action,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let state_file = app.state_file();
    let state = match prepare::load_state(&state_file, &app.config().event_slug) {
        Ok(s) => s,
        Err(e) => {
            return Outcome::Failed {
                actually_live: before(action),
                error: format!("Couldn't read the list of videos: {e}"),
            };
        }
    };
    let mut yt = match app.youtube().await {
        Ok(yt) => yt,
        Err(e) => {
            return Outcome::Failed {
                actually_live: before(action),
                error: e.to_string(),
            };
        }
    };
    let outcome = match action {
        Action::GoLive(game) => start(&mut yt, court, plan, &state, game, log).await,
        Action::Switch { from, to } => {
            let ctx = SwitchContext {
                court,
                plan,
                state: &state,
                state_file: &state_file,
            };
            switch(&mut yt, &ctx, from, to, log).await
        }
        Action::End(game) => end(&mut yt, court, plan, &state, game, log).await,
    };
    let units = yt.units_used;
    drop(yt);
    app.record_youtube(None, units);
    outcome
}

/// Which game's video is live if `action` doesn't happen at all.
fn before(action: &Action) -> Option<GameNumber> {
    match action {
        Action::GoLive(_) => None,
        Action::Switch { from, .. } => Some(from.clone()),
        Action::End(game) => Some(game.clone()),
    }
}

async fn start(
    yt: &mut YouTube,
    court: &CourtConfig,
    plan: Option<&EventPlan>,
    state: &prepare::EventState,
    game: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let result: Result<(), BoxError> = async {
        one_key_ready(court, state, &todays_games(plan, court, game))?;
        let video = video_of(state, plan, game)?;
        let index = stream_index(court, game, &video)?;
        let names = court.stream_names();
        let sid = stream_id(yt, &names[index]).await?;
        vmix::start_destination(&court.vmix_address, vmix_destination(index)).await?;
        log(format!(
            "vMix: started destination {}",
            vmix_destination(index)
        ));
        wait_for_stream(yt, &sid).await?;
        go_live(yt, game, &video, log).await?;
        log(format!(
            "Game {game} is LIVE: https://youtu.be/{}",
            video.broadcast_id
        ));
        Ok(())
    }
    .await;
    match result {
        Ok(()) => Outcome::Done,
        Err(e) => Outcome::Failed {
            actually_live: None,
            error: e.to_string(),
        },
    }
}

struct SwitchContext<'a> {
    court: &'a CourtConfig,
    plan: Option<&'a EventPlan>,
    state: &'a prepare::EventState,
    state_file: &'a std::path::Path,
}

async fn switch(
    yt: &mut YouTube,
    ctx: &SwitchContext<'_>,
    from: &str,
    to: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let SwitchContext {
        court,
        plan,
        state,
        state_file,
    } = *ctx;
    let failed = |error: String| Outcome::Failed {
        actually_live: Some(from.to_string()),
        error,
    };
    let to_video = match video_of(state, plan, to) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let to_index = match stream_index(court, to, &to_video) {
        Ok(i) => i,
        Err(e) => return failed(e),
    };
    let from_video = video_of(state, plan, from).ok();
    let from_index = from_video
        .as_ref()
        .and_then(|v| stream_index(court, from, v).ok());
    let names = court.stream_names();
    let mut warnings = Vec::new();

    if court.stream_mode == StreamMode::OneKey || from_index == Some(to_index) {
        // Both videos use the same stream key (one-key mode, or a game was skipped), so they
        // can't overlap: end the old one first, then start the new one on the stream that's
        // already running. vMix is left as it is.
        if let Some(v) = &from_video {
            if let Err(e) = yt.transition(&v.broadcast_id, "complete").await {
                return failed(format!("Couldn't end Game {from}'s video: {e}"));
            }
            log(format!("Game {from}'s video ended"));
        }
        if let Err(e) = go_live(yt, to, &to_video, log).await {
            // The old video has already ended, so nothing is live on this court now.
            return Outcome::Failed {
                actually_live: None,
                error: format!("Game {from} ended but Game {to} couldn't go live: {e}"),
            };
        }
        log(format!(
            "Game {to} is LIVE: https://youtu.be/{}",
            to_video.broadcast_id
        ));
        return Outcome::Done;
    }

    // 1–2: start the new stream key and put the new video live while the old one keeps going.
    let started: Result<(), BoxError> = async {
        let sid = stream_id(yt, &names[to_index]).await?;
        vmix::start_destination(&court.vmix_address, vmix_destination(to_index)).await?;
        log(format!(
            "vMix: started destination {}",
            vmix_destination(to_index)
        ));
        wait_for_stream(yt, &sid).await?;
        go_live(yt, to, &to_video, log).await
    }
    .await;
    if let Err(e) = started {
        // Undo the extra vMix output so the old one carries on alone.
        let _ = vmix::stop_destination(&court.vmix_address, vmix_destination(to_index)).await;
        return failed(format!("Switch to Game {to} didn't happen: {e}"));
    }
    let link = format!("https://youtu.be/{}", to_video.broadcast_id);
    log(format!("Game {to} is LIVE: {link}"));

    let Some(from_video) = from_video else {
        warnings.push(format!(
            "Game {from} had no recorded video, so nothing was ended"
        ));
        return Outcome::Warnings(warnings);
    };

    // 3: tell the old video's chat where the stream went (the chat closes when it ends).
    let teams = plan
        .and_then(|p| p.game(to))
        .map(|g| format!(" ({} vs {})", g.dark, g.light))
        .unwrap_or_default();
    let message = format!("▶ Game {to}{teams} is live now: {link}");
    match yt.broadcast_info(&from_video.broadcast_id).await {
        Ok(info) => match info.live_chat_id {
            Some(chat) => match yt.post_chat_message(&chat, &message).await {
                Ok(()) => log(format!("Chat message posted in Game {from}")),
                Err(e) => warnings.push(format!("Couldn't post the chat message: {e}")),
            },
            None => warnings.push(format!("Game {from}'s video has no live chat")),
        },
        Err(e) => warnings.push(format!("Couldn't post the chat message: {e}")),
    }

    // 4: end the old video and its vMix output.
    match yt.transition(&from_video.broadcast_id, "complete").await {
        Ok(()) => log(format!("Game {from}'s video ended")),
        Err(e) => warnings.push(format!(
            "Game {from}'s video may still be live; end it in YouTube Studio ({e})"
        )),
    }
    if let Some(index) = from_index {
        match vmix::stop_destination(&court.vmix_address, vmix_destination(index)).await {
            Ok(()) => log(format!(
                "vMix: stopped destination {}",
                vmix_destination(index)
            )),
            Err(e) => warnings.push(format!(
                "Couldn't stop vMix destination {}: {e}",
                vmix_destination(index)
            )),
        }
    }

    // 5: "Next game" link in the old video's description, for replay viewers.
    let spec = BroadcastSpec {
        title: from_video.title.clone(),
        description: with_next_link(&from_video.description, Some(&link)),
        scheduled_start: from_video.scheduled_start.clone(),
        privacy: String::new(),
    };
    if from_video.next_game_link.as_deref() != Some(link.as_str()) {
        match yt.update_broadcast(&from_video.broadcast_id, &spec).await {
            Ok(()) => {
                let saved = prepare::load_state(state_file, &state.event_slug).and_then(|mut s| {
                    if let Some(v) = s.videos.get_mut(from) {
                        v.description = spec.description.clone();
                        v.next_game_link = Some(link.clone());
                    }
                    prepare::save_state(state_file, &s)
                });
                if let Err(e) = saved {
                    warnings.push(format!("Couldn't save the video list: {e}"));
                }
            }
            Err(e) => warnings.push(format!("Couldn't add the Next game link: {e}")),
        }
    }

    if warnings.is_empty() {
        Outcome::Done
    } else {
        Outcome::Warnings(warnings)
    }
}

async fn end(
    yt: &mut YouTube,
    court: &CourtConfig,
    plan: Option<&EventPlan>,
    state: &prepare::EventState,
    game: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let mut warnings = Vec::new();
    match video_of(state, plan, game) {
        Ok(video) => match yt.transition(&video.broadcast_id, "complete").await {
            Ok(()) => log(format!("Game {game}'s video ended")),
            Err(e) => warnings.push(format!(
                "Game {game}'s video may still be live; end it in YouTube Studio ({e})"
            )),
        },
        Err(e) => warnings.push(e),
    }
    for &destination in destinations_to_stop(court) {
        match vmix::stop_destination(&court.vmix_address, destination).await {
            Ok(()) => log(format!("vMix: stopped destination {destination}")),
            Err(e) => warnings.push(format!("Couldn't stop vMix destination {destination}: {e}")),
        }
    }
    if warnings.is_empty() {
        Outcome::Done
    } else {
        Outcome::Warnings(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, StreamMode},
        portal::parse_event_plan,
    };

    fn video(bound: Option<&str>) -> VideoState {
        VideoState {
            broadcast_id: "id".to_string(),
            title: String::new(),
            description: String::new(),
            scheduled_start: String::new(),
            bound_stream: bound.map(str::to_string),
            in_playlist: true,
            next_game_link: None,
        }
    }

    fn state(videos: &[(&str, Option<&str>)]) -> prepare::EventState {
        prepare::EventState {
            videos: videos
                .iter()
                .map(|(game, bound)| (game.to_string(), video(*bound)))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn one_key_start_refuses_a_video_on_stream_b() {
        let mut court = Config::default().courts.remove(0);
        let mixed = state(&[("1", Some("Court 1 - A")), ("2", Some("court 1 – b"))]);

        // Two keys: videos on B are expected.
        assert_eq!(one_key_ready(&court, &mixed, &["1", "2"]), Ok(()));

        court.stream_mode = StreamMode::OneKey;
        assert_eq!(
            one_key_ready(&court, &mixed, &["1", "2"]),
            Err("Re-run Prepare: some of today's videos use a stream key other than A".to_string())
        );
        // Only the listed (today's) games are checked.
        assert_eq!(one_key_ready(&court, &mixed, &["1"]), Ok(()));

        let all_a = state(&[("1", Some("Court 1 - A")), ("2", Some("court 1 – a"))]);
        assert_eq!(one_key_ready(&court, &all_a, &["1", "2"]), Ok(()));
    }

    #[test]
    fn one_key_end_stops_only_destination_1() {
        let mut court = Config::default().courts.remove(0);
        assert_eq!(destinations_to_stop(&court), [1, 2]);
        court.stream_mode = StreamMode::OneKey;
        assert_eq!(destinations_to_stop(&court), [1]);
    }

    #[test]
    fn todays_games_are_this_courts_games_on_the_starting_games_day() {
        let game = |number: &str, start: &str, court: &str| {
            format!(
                r#"{{ "number": "{number}", "startsOn": "{start}", "court": "{court}",
                    "dark": {{ "assignment": null }}, "light": {{ "assignment": null }} }}"#
            )
        };
        let plan = parse_event_plan(&format!(
            r#"{{ "event": {{ "name": "Test Cup" }}, "games": [ {}, {}, {}, {} ] }}"#,
            game("1", "2026-08-01T09:00:00+10:00", "1"),
            game("2", "2026-08-01T09:00:00+10:00", "2"),
            game("3", "2026-08-01T10:00:00+10:00", "1"),
            game("4", "2026-08-02T09:00:00+10:00", "1"),
        ))
        .unwrap();
        let court = Config::default().courts.remove(0);
        assert_eq!(todays_games(Some(&plan), &court, "3"), ["1", "3"]);
        assert_eq!(todays_games(None, &court, "3"), ["3"]);
    }
}
