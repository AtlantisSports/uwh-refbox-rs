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
//! day. Each switch first checks that B's video can go live on the running stream, posts the
//! chat message, ends A's video, puts B's live and then adds the "Next game" link to A.

use crate::{
    BoxError,
    app::App,
    config::{CourtConfig, StreamMode, vmix_destination},
    portal::EventPlan,
    prepare::{self, VideoState, with_next_link},
    switcher::{Action, GameNumber},
    title_sync::{self, SyncReport},
    vmix,
    youtube::{BroadcastSpec, YouTube},
};
use std::time::Duration;

const STREAM_WAIT: Duration = Duration::from_secs(90);
const LIVE_WAIT: Duration = Duration::from_secs(30);
const POLL_EVERY: Duration = Duration::from_secs(3);
/// In a two-key switch, the longest the portal is given to answer while vMix starts the new
/// stream key. The check never makes the switch wait: once YouTube receives the new stream key,
/// a check that hasn't finished is given up.
const TITLE_CHECK_WAIT: Duration = Duration::from_secs(15);
/// The longest the portal check may hold up a switch where nothing else is waited for (both
/// videos on one stream key), and the longest a two-key switch waits for the check's YouTube
/// update.
const TITLE_CHECK_ON_PATH_WAIT: Duration = Duration::from_secs(5);
/// After vMix starts a stream key that YouTube already showed as receiving, how long "active"
/// may still be left over from the key's last use (YouTube's status lags behind a stop).
const STALE_ACTIVE_WAIT: Duration = Duration::from_secs(30);

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
            "The refbox is on Game {game}, which isn't in {}'s schedule. \
             Set the refbox to this event and court.",
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

/// The vMix destination that carries `game`'s video, to start it again when Stream Manager
/// resumes a live video after a restart. In one-key mode that is always destination 1.
pub fn resume_destination(
    court: &CourtConfig,
    state: &prepare::EventState,
    game: &str,
) -> Result<u8, String> {
    if court.stream_mode == StreamMode::OneKey {
        return Ok(vmix_destination(0));
    }
    let video = state
        .videos
        .get(game)
        .ok_or_else(|| format!("Game {game} has no recorded video"))?;
    Ok(vmix_destination(stream_index(court, game, video)?))
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
    plan.court_games(&court.name, day)
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

/// The two videos of a switch.
#[derive(Debug)]
struct SwitchVideos {
    to_video: VideoState,
    to_index: usize,
    /// The old game's video, if one is recorded.
    from_video: Option<VideoState>,
    /// The old video's stream key, if known.
    from_index: Option<usize>,
    /// Both videos use the same stream key (one-key mode, or a skipped game), so they can't
    /// overlap.
    same_key: bool,
}

/// Finds a switch's videos. The new game's video is looked up through the schedule. The old
/// one is taken straight from the recorded videos, so a live game that the portal has since
/// removed or renumbered still gets ended.
fn switch_videos(
    state: &prepare::EventState,
    plan: Option<&EventPlan>,
    court: &CourtConfig,
    from: &str,
    to: &str,
) -> Result<SwitchVideos, String> {
    let to_video = video_of(state, plan, to)?;
    let to_index = stream_index(court, to, &to_video)?;
    let from_video = state.videos.get(from).cloned();
    let from_index = from_video
        .as_ref()
        .and_then(|v| stream_index(court, from, v).ok());
    Ok(SwitchVideos {
        same_key: court.stream_mode == StreamMode::OneKey || from_index == Some(to_index),
        to_video,
        to_index,
        from_video,
        from_index,
    })
}

/// After a failed two-key switch, the vMix output to stop again: the new stream key's, but only
/// when the old video's key is known and is a different one, so the output carrying the live
/// video is never stopped.
fn output_to_undo(from_index: Option<usize>, to_index: usize) -> Option<usize> {
    from_index
        .filter(|&from| from != to_index)
        .map(|_| to_index)
}

/// Why the next video can't take over the running stream key, if it can't. Checked before the
/// old video is ended, so a problem leaves the old video live.
fn take_over_problem(to: &str, key: &str, life_cycle: &str, stream_status: &str) -> Option<String> {
    if !matches!(life_cycle, "ready" | "testing" | "live") {
        return Some(format!(
            "Game {to}'s YouTube video can't go live (YouTube shows it as \"{life_cycle}\")"
        ));
    }
    if stream_status != "active" {
        return Some(format!(
            "YouTube isn't receiving video on stream key \"{key}\" (it shows \"{stream_status}\"). \
             Check vMix is streaming on that key"
        ));
    }
    None
}

/// Whether a stream status read while waiting shows vMix's video arriving. If the key already
/// showed `active` before vMix was started, that can be left over from its last use, so it
/// only counts once YouTube has shown the key inactive since, or once vMix has been sending
/// for longer than such a leftover lasts.
fn stream_receiving(
    status: &str,
    active_before_start: bool,
    seen_inactive: bool,
    since_start: Duration,
) -> bool {
    status == "active"
        && (!active_before_start || seen_inactive || since_start >= STALE_ACTIVE_WAIT)
}

/// What became of the portal check just before a switch (`None`: it didn't finish in time).
fn title_check_warning(to: &str, result: Option<Result<SyncReport, BoxError>>) -> Option<String> {
    match result {
        Some(Ok(_)) => None,
        Some(Err(e)) => Some(format!(
            "Couldn't check Game {to}'s title against the portal: {e}"
        )),
        None => Some(format!(
            "The portal didn't answer in time, so Game {to}'s title wasn't checked"
        )),
    }
}

/// Runs `main` to the end and `side` alongside it. `side`'s result is returned if it finished
/// first; otherwise it is given up, so it never makes `main` take longer.
async fn alongside<M: Future, S: Future>(main: M, side: S) -> (M::Output, Option<S::Output>) {
    let mut side_result = None;
    tokio::pin!(main, side);
    let main_result = loop {
        tokio::select! {
            result = &mut main => break result,
            result = &mut side, if side_result.is_none() => side_result = Some(result),
        }
    };
    (main_result, side_result)
}

async fn stream_id(yt: &mut YouTube, title: &str) -> Result<String, BoxError> {
    let streams = yt.list_streams().await?;
    streams
        .into_iter()
        .find(|s| normalize(&s.title) == normalize(title))
        .map(|s| s.id)
        .ok_or_else(|| format!("Stream key \"{title}\" not found on the channel").into())
}

/// Starts vMix sending on stream key `index` and waits until YouTube receives it.
///
/// With `may_be_leftover`, an "active" status the key already had before vMix started is not
/// trusted (see [`stream_receiving`]). A switch sets it: the new key may have been stopped by
/// the switch just before. Start day doesn't: there, a key already receiving is usually vMix
/// sending before the day starts, and Start day shouldn't wait for nothing.
async fn start_stream(
    yt: &mut YouTube,
    court: &CourtConfig,
    index: usize,
    may_be_leftover: bool,
    log: &mut (dyn FnMut(String) + Send),
) -> Result<(), BoxError> {
    let names = court.stream_names();
    let sid = stream_id(yt, &names[index]).await?;
    let active_before_start = may_be_leftover && yt.stream_status(&sid).await? == "active";
    vmix::start_destination(&court.vmix_address, vmix_destination(index)).await?;
    log(format!(
        "vMix: started destination {}",
        vmix_destination(index)
    ));
    wait_for_stream(yt, &sid, active_before_start).await
}

/// Waits until YouTube reports video arriving on the stream key (see [`stream_receiving`]).
async fn wait_for_stream(
    yt: &mut YouTube,
    stream_id: &str,
    active_before_start: bool,
) -> Result<(), BoxError> {
    let started = tokio::time::Instant::now();
    let mut seen_inactive = false;
    loop {
        let status = yt.stream_status(stream_id).await?;
        seen_inactive |= status != "active";
        if stream_receiving(
            &status,
            active_before_start,
            seen_inactive,
            started.elapsed(),
        ) {
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

/// Checks, before the old video ends, that the next video can go live on the stream key vMix
/// is already sending on.
async fn check_take_over(
    yt: &mut YouTube,
    court: &CourtConfig,
    to: &str,
    videos: &SwitchVideos,
) -> Result<(), BoxError> {
    let key = &court.stream_names()[videos.to_index];
    let info = yt.broadcast_info(&videos.to_video.broadcast_id).await?;
    let sid = stream_id(yt, key).await?;
    let status = yt.stream_status(&sid).await?;
    match take_over_problem(to, key, &info.life_cycle, &status) {
        Some(problem) => Err(problem.into()),
        None => Ok(()),
    }
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
    match action {
        Action::GoLive(game) => start(&mut yt, court, plan, &state, game, log).await,
        Action::Switch { from, to } => {
            let ctx = SwitchContext {
                app,
                court,
                plan,
                state: &state,
                state_file: &state_file,
            };
            switch(&mut yt, &ctx, from, to, log).await
        }
        Action::End(game) => end(&mut yt, court, &state, game, log).await,
    }
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
        start_stream(yt, court, index, false, log).await?;
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
    app: &'a App,
    court: &'a CourtConfig,
    plan: Option<&'a EventPlan>,
    state: &'a prepare::EventState,
    state_file: &'a std::path::Path,
}

/// Whether the extras (chat message, "Next game" link) run for this switch. They stop when the
/// allowance runs low, so the switches themselves can carry on (ADR 026 §7).
fn extras_allowed(app: &App, court: &CourtConfig, log: &mut (dyn FnMut(String) + Send)) -> bool {
    let extras = app.extras_allowed(&court.name);
    if !extras {
        log("Allowance low: skipped the chat message and Next game link".to_string());
    }
    extras
}

/// Step 3: tells the old video's chat where the stream went (the chat closes when it ends).
/// Returns a warning if the message couldn't be posted.
async fn post_switch_message(
    yt: &mut YouTube,
    plan: Option<&EventPlan>,
    from: &str,
    from_video: &VideoState,
    to: &str,
    link: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Option<String> {
    let teams = plan
        .and_then(|p| p.game(to))
        .map(|g| format!(" ({} vs {})", g.dark, g.light))
        .unwrap_or_default();
    let message = format!("▶ Game {to}{teams} is live now: {link}");
    match yt.broadcast_info(&from_video.broadcast_id).await {
        Ok(info) => match info.live_chat_id {
            Some(chat) => match yt.post_chat_message(&chat, &message).await {
                Ok(()) => {
                    log(format!("Chat message posted in Game {from}"));
                    None
                }
                Err(e) => Some(format!("Couldn't post the chat message: {e}")),
            },
            None => Some(format!("Game {from}'s video has no live chat")),
        },
        Err(e) => Some(format!("Couldn't post the chat message: {e}")),
    }
}

/// Step 5: a "Next game" link in the old video's description, for replay viewers.
async fn add_next_link(
    yt: &mut YouTube,
    ctx: &SwitchContext<'_>,
    from: &str,
    from_video: &VideoState,
    link: &str,
    warnings: &mut Vec<String>,
) {
    if from_video.next_game_link.as_deref() == Some(link) {
        return;
    }
    let spec = BroadcastSpec {
        title: from_video.title.clone(),
        description: with_next_link(&from_video.description, Some(link)),
        scheduled_start: from_video.scheduled_start.clone(),
        privacy: String::new(),
    };
    match yt.update_broadcast(&from_video.broadcast_id, &spec).await {
        Ok(()) => {
            let saved =
                prepare::load_state(ctx.state_file, &ctx.state.event_slug).and_then(|mut s| {
                    if let Some(v) = s.videos.get_mut(from) {
                        v.description = spec.description.clone();
                        v.next_game_link = Some(link.to_string());
                    }
                    prepare::save_state(ctx.state_file, &s)
                });
            if let Err(e) = saved {
                warnings.push(format!("Couldn't save the video list: {e}"));
            }
        }
        Err(e) => warnings.push(format!("Couldn't add the Next game link: {e}")),
    }
}

fn finished(warnings: Vec<String>) -> Outcome {
    if warnings.is_empty() {
        Outcome::Done
    } else {
        Outcome::Warnings(warnings)
    }
}

async fn switch(
    yt: &mut YouTube,
    ctx: &SwitchContext<'_>,
    from: &str,
    to: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let failed = |error: String| Outcome::Failed {
        actually_live: Some(from.to_string()),
        error,
    };
    let videos = match switch_videos(ctx.state, ctx.plan, ctx.court, from, to) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    if videos.same_key {
        same_key_switch(yt, ctx, from, to, &videos, log).await
    } else {
        two_key_switch(yt, ctx, from, to, &videos, log).await
    }
}

/// Both videos use the same stream key (one-key mode, or a game was skipped), so they can't
/// overlap: once the new video is known to be able to go live on the stream that's already
/// running, the old one ends and the new one starts. vMix is left as it is.
async fn same_key_switch(
    yt: &mut YouTube,
    ctx: &SwitchContext<'_>,
    from: &str,
    to: &str,
    videos: &SwitchVideos,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let (app, court) = (ctx.app, ctx.court);
    let mut warnings = Vec::new();

    // The video about to go live gets the portal's latest title, description and start time.
    // Nothing else can be done while waiting for it here, so it gets only a short time.
    let check = title_sync::sync_court_with(app, yt, court, Some(to), log);
    let report = tokio::time::timeout(TITLE_CHECK_ON_PATH_WAIT, check)
        .await
        .ok();
    warnings.extend(title_check_warning(to, report));

    if let Err(e) = check_take_over(yt, court, to, videos).await {
        return Outcome::Failed {
            actually_live: Some(from.to_string()),
            error: format!("Switch to Game {to} didn't happen; Game {from} is still live: {e}"),
        };
    }

    let link = format!("https://youtu.be/{}", videos.to_video.broadcast_id);
    let extras = videos.from_video.is_some() && extras_allowed(app, court, log);
    if let Some(from_video) = &videos.from_video {
        // The new video's link is known in advance, so the message goes in before the old
        // video (and its chat) ends.
        if extras {
            let problem = post_switch_message(yt, ctx.plan, from, from_video, to, &link, log).await;
            warnings.extend(problem);
        }
        if let Err(e) = yt.transition(&from_video.broadcast_id, "complete").await {
            return Outcome::Failed {
                actually_live: Some(from.to_string()),
                error: format!("Couldn't end Game {from}'s video: {e}"),
            };
        }
        log(format!("Game {from}'s video ended"));
    }
    if let Err(e) = go_live(yt, to, &videos.to_video, log).await {
        // The old video has already ended, so nothing is live on this court now.
        return Outcome::Failed {
            actually_live: None,
            error: format!("Game {from} ended but Game {to} couldn't go live: {e}"),
        };
    }
    log(format!("Game {to} is LIVE: {link}"));

    match &videos.from_video {
        Some(from_video) if extras => {
            add_next_link(yt, ctx, from, from_video, &link, &mut warnings).await;
        }
        Some(_) => {}
        None => warnings.push(format!(
            "Game {from} had no recorded video, so nothing was ended"
        )),
    }
    finished(warnings)
}

/// The videos use different stream keys: the new one goes live while the old one keeps going,
/// then the old one ends.
async fn two_key_switch(
    yt: &mut YouTube,
    ctx: &SwitchContext<'_>,
    from: &str,
    to: &str,
    videos: &SwitchVideos,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let (app, court) = (ctx.app, ctx.court);
    let mut warnings = Vec::new();

    // 1: start the new stream key. While YouTube warms up to it, the portal is asked for the
    // new video's latest title, description and start time; the switch never waits for it.
    let (receiving, fetched) = alongside(
        start_stream(yt, court, videos.to_index, true, log),
        tokio::time::timeout(TITLE_CHECK_WAIT, title_sync::begin(app, court, Some(to))),
    )
    .await;
    let report = match receiving {
        Err(e) => Err(e),
        Ok(()) => {
            // The portal answered in time: bring the video in line before it goes live.
            let report = match fetched.and_then(Result::ok) {
                None => None,
                Some(Err(e)) => Some(Err(e)),
                Some(Ok(check)) => {
                    let update = title_sync::complete(app, yt, court, &check, log);
                    tokio::time::timeout(TITLE_CHECK_ON_PATH_WAIT, update)
                        .await
                        .ok()
                }
            };
            // 2: the new video goes live.
            go_live(yt, to, &videos.to_video, log)
                .await
                .map(|()| report)
        }
    };
    let report = match report {
        Ok(report) => report,
        Err(e) => {
            // Undo the extra vMix output so the old one carries on alone.
            if let Some(index) = output_to_undo(videos.from_index, videos.to_index) {
                let _ = vmix::stop_destination(&court.vmix_address, vmix_destination(index)).await;
            }
            return Outcome::Failed {
                actually_live: Some(from.to_string()),
                error: format!("Switch to Game {to} didn't happen: {e}"),
            };
        }
    };
    warnings.extend(title_check_warning(to, report));
    let link = format!("https://youtu.be/{}", videos.to_video.broadcast_id);
    log(format!("Game {to} is LIVE: {link}"));

    let Some(from_video) = &videos.from_video else {
        warnings.push(format!(
            "Game {from} had no recorded video, so nothing was ended"
        ));
        return Outcome::Warnings(warnings);
    };
    let extras = extras_allowed(app, court, log);

    // 3: tell the old video's chat where the stream went (the chat closes when it ends).
    if extras {
        let problem = post_switch_message(yt, ctx.plan, from, from_video, to, &link, log).await;
        warnings.extend(problem);
    }

    // 4: end the old video and its vMix output.
    match yt.transition(&from_video.broadcast_id, "complete").await {
        Ok(()) => log(format!("Game {from}'s video ended")),
        Err(e) => warnings.push(format!(
            "Game {from}'s video may still be live; end it in YouTube Studio ({e})"
        )),
    }
    if let Some(index) = videos.from_index {
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
    if extras {
        add_next_link(yt, ctx, from, from_video, &link, &mut warnings).await;
    }
    finished(warnings)
}

async fn end(
    yt: &mut YouTube,
    court: &CourtConfig,
    state: &prepare::EventState,
    game: &str,
    log: &mut (dyn FnMut(String) + Send),
) -> Outcome {
    let mut warnings = Vec::new();
    // Straight from the recorded videos, so the video still ends if the portal has since
    // removed or renumbered its game.
    match state.videos.get(game) {
        Some(video) => match yt.transition(&video.broadcast_id, "complete").await {
            Ok(()) => log(format!("Game {game}'s video ended")),
            Err(e) => warnings.push(format!(
                "Game {game}'s video may still be live; end it in YouTube Studio ({e})"
            )),
        },
        None => warnings.push(format!(
            "Game {game} had no recorded video, so nothing was ended"
        )),
    }
    for &destination in destinations_to_stop(court) {
        match vmix::stop_destination(&court.vmix_address, destination).await {
            Ok(()) => log(format!("vMix: stopped destination {destination}")),
            Err(e) => warnings.push(format!("Couldn't stop vMix destination {destination}: {e}")),
        }
    }
    finished(warnings)
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
            portal_start: None,
            court: None,
            day: None,
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

    fn plan_of(games: &[&str]) -> EventPlan {
        let games: Vec<String> = games
            .iter()
            .map(|number| {
                format!(
                    r#"{{ "number": "{number}", "startsOn": "2026-08-01T09:00:00+10:00",
                        "court": "1", "dark": {{ "assignment": null }},
                        "light": {{ "assignment": null }} }}"#
                )
            })
            .collect();
        parse_event_plan(&format!(
            r#"{{ "event": {{ "name": "Test Cup" }}, "games": [ {} ] }}"#,
            games.join(", ")
        ))
        .unwrap()
    }

    #[test]
    fn the_old_video_is_found_even_after_the_portal_dropped_its_game() {
        let court = Config::default().courts.remove(0);
        let mut videos = state(&[("1", Some("Court 1 - A")), ("2", Some("Court 1 - B"))]);
        videos.videos.get_mut("1").unwrap().broadcast_id = "live-one".to_string();
        // Game 1, which is live, has been renumbered or deleted on the portal.
        let plan = plan_of(&["2"]);
        let found = switch_videos(&videos, Some(&plan), &court, "1", "2").unwrap();
        assert_eq!(
            found.from_video.map(|v| v.broadcast_id).as_deref(),
            Some("live-one")
        );
        assert_eq!((found.from_index, found.to_index), (Some(0), 1));
        assert!(!found.same_key);

        // The new game still has to be on the portal.
        assert!(switch_videos(&videos, Some(&plan), &court, "2", "1").is_err());
    }

    #[test]
    fn same_key_switches_are_one_key_mode_or_a_shared_key() {
        let mut court = Config::default().courts.remove(0);
        let both_a = state(&[("1", Some("Court 1 - A")), ("3", Some("Court 1 - A"))]);
        assert!(
            switch_videos(&both_a, None, &court, "1", "3")
                .unwrap()
                .same_key
        );
        let a_then_b = state(&[("1", Some("Court 1 - A")), ("2", Some("Court 1 - B"))]);
        assert!(
            !switch_videos(&a_then_b, None, &court, "1", "2")
                .unwrap()
                .same_key
        );
        court.stream_mode = StreamMode::OneKey;
        assert!(
            switch_videos(&both_a, None, &court, "1", "3")
                .unwrap()
                .same_key
        );
    }

    #[test]
    fn a_failed_switch_stops_the_new_output_only_when_the_old_one_is_elsewhere() {
        assert_eq!(output_to_undo(Some(0), 1), Some(1));
        // The old video's key is unknown: the new key might be carrying it, so leave it.
        assert_eq!(output_to_undo(None, 1), None);
        assert_eq!(output_to_undo(Some(1), 1), None);
    }

    #[test]
    fn the_next_video_takes_over_only_when_it_can_go_live_on_a_receiving_key() {
        assert_eq!(
            take_over_problem("2", "Court 1 - A", "ready", "active"),
            None
        );
        assert_eq!(
            take_over_problem("2", "Court 1 - A", "testing", "active"),
            None
        );
        assert_eq!(
            take_over_problem("2", "Court 1 - A", "live", "active"),
            None
        );
        assert_eq!(
            take_over_problem("2", "Court 1 - A", "complete", "active"),
            Some(
                "Game 2's YouTube video can't go live (YouTube shows it as \"complete\")"
                    .to_string()
            )
        );
        assert_eq!(
            take_over_problem("2", "Court 1 - A", "ready", "inactive"),
            Some(
                "YouTube isn't receiving video on stream key \"Court 1 - A\" (it shows \
                 \"inactive\"). Check vMix is streaming on that key"
                    .to_string()
            )
        );
    }

    #[test]
    fn a_leftover_active_status_doesnt_count_as_receiving() {
        let soon = Duration::from_secs(3);
        // The key was idle before vMix started: the first "active" is real.
        assert!(stream_receiving("active", false, false, soon));
        assert!(!stream_receiving("inactive", false, true, soon));
        // The key still showed "active" from its last use: wait for YouTube to catch up.
        assert!(!stream_receiving("active", true, false, soon));
        assert!(stream_receiving("active", true, true, soon));
        assert!(stream_receiving("active", true, false, STALE_ACTIVE_WAIT));
    }

    #[tokio::test]
    async fn the_portal_check_never_holds_up_the_switch() {
        let quick = tokio::time::sleep(Duration::from_millis(20));
        let slow_portal = tokio::time::sleep(Duration::from_secs(30));
        let started = std::time::Instant::now();
        let ((), checked) = alongside(quick, slow_portal).await;
        assert!(checked.is_none());
        assert!(started.elapsed() < Duration::from_secs(5));

        let slower = tokio::time::sleep(Duration::from_millis(50));
        let ((), checked) = alongside(slower, async { 7 }).await;
        assert_eq!(checked, Some(7));
    }

    #[test]
    fn a_missing_title_check_is_reported_as_a_warning() {
        assert_eq!(
            title_check_warning("2", Some(Ok(SyncReport::default()))),
            None
        );
        assert_eq!(
            title_check_warning("2", None).as_deref(),
            Some("The portal didn't answer in time, so Game 2's title wasn't checked")
        );
    }

    #[test]
    fn resuming_restarts_the_live_videos_vmix_destination() {
        let mut court = Config::default().courts.remove(0);
        let videos = state(&[("1", Some("Court 1 - A")), ("2", Some("Court 1 - B"))]);
        assert_eq!(resume_destination(&court, &videos, "1"), Ok(1));
        assert_eq!(resume_destination(&court, &videos, "2"), Ok(2));
        assert!(resume_destination(&court, &videos, "9").is_err());
        court.stream_mode = StreamMode::OneKey;
        assert_eq!(resume_destination(&court, &videos, "9"), Ok(1));
    }

    #[test]
    fn a_game_missing_from_the_schedule_is_explained_in_one_sentence_each() {
        let plan = plan_of(&["2"]);
        let Err(error) = video_of(&state(&[]), Some(&plan), "1") else {
            panic!("Game 1 isn't on the portal");
        };
        assert_eq!(
            error,
            "The refbox is on Game 1, which isn't in Test Cup's schedule. \
             Set the refbox to this event and court."
        );
        assert!(!error.contains("  "));
    }
}
