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

use crate::{
    BoxError,
    app::App,
    config::{CourtConfig, vmix_destination},
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

    if from_index == Some(to_index) {
        // Both videos use the same stream key (e.g. a game was skipped), so they can't overlap:
        // end the old one first, then start the new one on the stream that's already running.
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
    // Stop both of the court's destinations, so nothing is left streaming at the end of the
    // day even if one was started by hand in vMix.
    for index in 0..2 {
        let destination = vmix_destination(index);
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
