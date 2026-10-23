//! Reads live game state from an `overlay-bridge` instance's `GET /game` feed over HTTP,
//! instead of connecting to the refbox directly (see `network.rs`, which this leaves
//! completely untouched).
//!
//! **Liveness rules, matching the contract `overlay-bridge/src/game_feed.rs` documents:**
//! an HTTP request that fails or times out means the *bridge* is unreachable -- nothing is
//! sent, so the renderer freezes on its last known state, the same way `network.rs`
//! already behaves on a lost refbox connection. A successful response's `connected` field
//! is the *only* signal for whether the *refbox* is alive; `connected: false` is treated
//! the same way -- no update is sent, rather than inventing a value. Never inferred from
//! HTTP timing in either direction.
//!
//! **What this reconstructs:** the clock, scores, period, timeout, penalties, the
//! recent-goal callout, and team *names* (`black_team`/`white_team`, already resolved by
//! the bridge -- no portal lookup needed on this side for just the names), plus full rosters
//! from the bridge's `GET /roster` table.
//!
//! **Pictures:** when the feed names a portal event, this also asks that portal directly
//! (through `network.rs`'s never-panicking fetchers) for the game's team logos, player photos,
//! referees, court and start time, and for the event and sponsor logos. Those are merged into
//! the bridge's own names-only game data ([`with_portal_pictures`]): the bridge stays the
//! authority for names and caps, a photo and role tag are used only where the portal's player
//! for that cap has the same name, and a team logo and the portal's coaches and managers
//! (members with no cap number, with their photos and roles, as on the direct Pi mode's roster
//! screen) only where the team names match. With no portal event (a local-CSV roster) nothing is
//! fetched and the names are shown without pictures.

use std::time::Duration;

use log::{debug, error, warn};
use serde::Deserialize;
use uwh_common::{
    bundles::BlackWhiteBundle,
    color::Color,
    game_snapshot::{
        GamePeriod, GameSnapshot, Infraction, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
    },
    uwhportal::schedule::{EventId, GameNumber},
};

use crate::network::{
    self, EventLogos, GameData, MemberRaw, PortalFetchError, StateUpdate, TeamInfoRaw,
};

/// The `/game` schema version this client understands. Bumped only when a field is
/// removed, renamed, or changes meaning (`overlay-bridge/src/game_feed.rs`'s own rule) --
/// so a mismatch here means this client's understanding of the feed is stale, not that a
/// field was merely added.
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Goal {
    team: String,
    player: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Timeout {
    kind: String,
    secs_remaining: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Penalty {
    team: String,
    number: u8,
    player: Option<String>,
    secs_remaining: Option<u32>,
    total_dismissal: bool,
    #[allow(dead_code)] // ditto -- see `PenaltySnapshot::infraction` note in `build_snapshot`
    infraction: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameFeed {
    schema_version: u32,
    connected: bool,
    period: Option<String>,
    secs_in_period: Option<u32>,
    black_score: Option<u8>,
    white_score: Option<u8>,
    black_team: Option<String>,
    white_team: Option<String>,
    timeout: Option<Timeout>,
    game_number: Option<String>,
    next_game_number: Option<String>,
    is_old_game: Option<bool>,
    recent_goal: Option<Goal>,
    next_period_len_secs: Option<u32>,
    penalties: Option<Vec<Penalty>>,
    event_id: Option<String>,
    portal_base_url: Option<String>,
}

/// The reverse of `GamePeriod`'s `Display` impl (`uwh-common/src/game_snapshot.rs`).
/// `overlay-bridge` deliberately serves these human labels rather than a machine name, and
/// its own contract says renaming one is a `SCHEMA_VERSION`-bumping change -- so an
/// unrecognised string here means the version check above should already have caught it,
/// not that this list is out of date.
fn parse_period(s: &str) -> Option<GamePeriod> {
    Some(match s {
        "Between Games" => GamePeriod::BetweenGames,
        "First Half" => GamePeriod::FirstHalf,
        "Half Time" => GamePeriod::HalfTime,
        "Second Half" => GamePeriod::SecondHalf,
        "Pre Overtime" => GamePeriod::PreOvertime,
        "Overtime First Half" => GamePeriod::OvertimeFirstHalf,
        "Overtime Half Time" => GamePeriod::OvertimeHalfTime,
        "Overtime Second Half" => GamePeriod::OvertimeSecondHalf,
        "Pre Sudden Death" => GamePeriod::PreSuddenDeath,
        "Sudden Death" => GamePeriod::SuddenDeath,
        _ => return None,
    })
}

fn parse_color(s: &str) -> Option<Color> {
    match s {
        "BLACK" => Some(Color::Black),
        "WHITE" => Some(Color::White),
        _ => None,
    }
}

/// The reverse of `overlay-bridge/src/tables.rs`'s `timeout_label` -- deliberately not
/// `TimeoutSnapshot`'s own `Display` impl, which uses different text ("PenaltyShot" with no
/// space) and was never meant to be viewer- or wire-facing.
fn parse_timeout(t: &Timeout) -> Option<TimeoutSnapshot> {
    let secs = t.secs_remaining as u16;
    match t.kind.as_str() {
        "Black Timeout" => Some(TimeoutSnapshot::Black(secs)),
        "White Timeout" => Some(TimeoutSnapshot::White(secs)),
        "Ref Timeout" => Some(TimeoutSnapshot::Ref(secs)),
        "Penalty Shot" => Some(TimeoutSnapshot::PenaltyShot(secs)),
        _ => None,
    }
}

/// Rebuilds a `GameSnapshot` from the feed so the existing rendering code (`pages/`,
/// `flag.rs`) needs no changes at all -- it already knows how to draw a `GameSnapshot`,
/// regardless of whether one arrived from the refbox directly or was reconstructed here.
fn build_snapshot(feed: &GameFeed) -> Option<GameSnapshot> {
    let current_period = parse_period(feed.period.as_deref()?)?;
    let is_old_game = feed.is_old_game.unwrap_or(false);

    // `.game_number()` (used everywhere in the renderer instead of the raw fields) reads
    // `next_game_number` while `BetweenGames && !is_old_game`, and `game_number` otherwise.
    // The bridge already applied that same resolution into `feed.game_number` -- so the
    // raw fields here are arranged to make our own `.game_number()` call reproduce exactly
    // what the bridge already resolved, in either branch.
    let resolved_current = feed.game_number.clone().unwrap_or_default();
    let (game_number, next_game_number) =
        if current_period == GamePeriod::BetweenGames && !is_old_game {
            (
                feed.next_game_number
                    .clone()
                    .unwrap_or_else(|| resolved_current.clone()),
                resolved_current,
            )
        } else {
            (
                resolved_current,
                feed.next_game_number.clone().unwrap_or_default(),
            )
        };

    // Both or neither, matching the bridge's own pairing rule -- an id without its portal
    // address cannot be resolved safely, since ids are not unique across portal environments.
    let (event_id, portal_base_url) = match (&feed.event_id, &feed.portal_base_url) {
        (Some(id), Some(url)) => match EventId::from_full(id) {
            Ok(id) => (Some(id), Some(url.clone())),
            Err(e) => {
                warn!("Bridge sent an unparseable event id {id:?}: {e}");
                (None, None)
            }
        },
        _ => (None, None),
    };

    let recent_goal = feed
        .recent_goal
        .as_ref()
        .and_then(|g| Some((parse_color(&g.team)?, g.player)));

    let timeout = feed.timeout.as_ref().and_then(parse_timeout);

    let mut penalties: BlackWhiteBundle<Vec<PenaltySnapshot>> = BlackWhiteBundle::default();
    for p in feed.penalties.iter().flatten() {
        let Some(color) = parse_color(&p.team) else {
            continue;
        };
        let time = if p.total_dismissal {
            PenaltyTime::TotalDismissal
        } else {
            PenaltyTime::Seconds(p.secs_remaining.unwrap_or(0) as u16)
        };
        penalties[color].push(PenaltySnapshot {
            player_number: p.number,
            time,
            // The renderer only ever draws a penalty's player number and remaining time
            // (`flag.rs`'s `synchronize_penalties`/`draw`) -- never `infraction` -- so a
            // filler value here changes nothing on screen. Reconstructing the real
            // `Infraction` would need a full reverse lookup of `Infraction::short_name`
            // for no observable benefit.
            infraction: Infraction::Unknown,
        });
    }

    Some(GameSnapshot {
        current_period,
        secs_in_period: feed.secs_in_period.unwrap_or(0),
        timeout,
        scores: BlackWhiteBundle {
            black: feed.black_score.unwrap_or(0),
            white: feed.white_score.unwrap_or(0),
        },
        penalties,
        // Confirmed unused anywhere in this renderer (no `.warnings`/`.fouls` access exists
        // in `pages/` or `flag.rs`), so there is nothing to reconstruct these from.
        warnings: BlackWhiteBundle::default(),
        fouls: Default::default(),
        is_old_game,
        game_number,
        next_game_number,
        event_id,
        portal_base_url,
        recent_goal,
        next_period_len_secs: feed.next_period_len_secs,
        // Confirmed unused anywhere in this renderer.
        conf_pause_time: None,
    })
}

/// A `GameData` carrying the team names the feed already resolved, and full team rosters when
/// `full_roster` has them (fetched separately -- see [`fetch_full_roster`]), falling back to
/// per-player names for whoever currently has a penalty ([`members_with_known_names`]) when it
/// doesn't. Names only, with no pictures, referees, court or start time: in portal mode those are
/// merged in afterwards from a separate portal fetch ([`with_portal_pictures`]); a CSV-sourced
/// roster has none, per design.
///
/// `snapshot` must be the `GameSnapshot` just built from this same `feed` by
/// [`build_snapshot`]: `State::update_state`'s `GameData` handling matches against the
/// game number (and, when there is a portal id, that too) already in the renderer's state, so
/// this borrows both from the snapshot we're about to send instead of recomputing them, to
/// guarantee they agree. **`event_id` is carried through as-is, `None` included** -- with no
/// portal at all, `event_id` is always `None`, and `update_state` treats that as its own valid
/// match rule (game number alone), not as "reject this update".
///
/// Returns `None` only when either team name is missing -- unlike before, the absence of a
/// portal id is no longer a reason to give up.
fn build_game_data(
    feed: &GameFeed,
    snapshot: &GameSnapshot,
    full_roster: Option<&(Vec<MemberRaw>, Vec<MemberRaw>)>,
) -> Option<GameData> {
    let (black_members, white_members) = match full_roster {
        Some((black, white)) if !black.is_empty() || !white.is_empty() => {
            (black.clone(), white.clone())
        }
        _ => (
            members_with_known_names(feed, "BLACK"),
            members_with_known_names(feed, "WHITE"),
        ),
    };

    Some(GameData {
        pool: String::new(),
        start_time: String::new(),
        referees: Vec::new(),
        black: TeamInfoRaw {
            team_name: feed.black_team.clone()?,
            members: black_members,
            ..Default::default()
        },
        white: TeamInfoRaw {
            team_name: feed.white_team.clone()?,
            members: white_members,
            ..Default::default()
        },
        game_number: snapshot.game_number().clone(),
        event_id: snapshot.event_id.clone(),
    })
}

/// Every player from `feed`'s current penalties whose name the bridge has already resolved, for
/// `team_code` (`"BLACK"`/`"WHITE"`). Not a full roster -- just enough for `flag.rs`'s existing
/// name lookup (which searches this same `members` list by cap number) to find a name for
/// whoever is currently flagged for a penalty. Used only as a fallback when the bridge's
/// `/roster` endpoint (see [`fetch_full_roster`]) hasn't resolved anything yet or is
/// unreachable -- a cap number with no resolved name is simply omitted, matching `flag.rs`'s own
/// fallback of showing the number alone.
fn members_with_known_names(feed: &GameFeed, team_code: &str) -> Vec<MemberRaw> {
    feed.penalties
        .iter()
        .flatten()
        .filter(|p| p.team == team_code)
        .filter_map(|p| {
            Some(MemberRaw {
                name: p.player.clone()?,
                role: None,
                number: Some(p.number),
                picture: None,
                geared_picture: None,
            })
        })
        .collect()
}

/// One row of the bridge's `GET /roster` table (`overlay-bridge/src/tables.rs`'s `roster`
/// function): every player on both teams for the game currently on screen, unlike this crate's
/// own `/game` feed, which only ever carries the handful of players with a current penalty. Like
/// every vMix table, values are plain strings even where the underlying type isn't -- `number`
/// needs parsing back to a cap number.
#[derive(Debug, Clone, Deserialize)]
struct RosterRow {
    team: String,
    number: String,
    player: String,
}

/// Fetches the bridge's full roster for both teams. `None` on any failure (network error, bad
/// status, unparseable body) -- the caller falls back to [`members_with_known_names`], the same
/// "never invent a value, degrade to what's already known" rule the rest of this module follows.
/// `Some` with possibly-empty lists means the fetch succeeded but nothing is resolved yet (no
/// game selected, or an empty roster file) -- also falls back, since an empty roster is no more
/// useful than none at all.
async fn fetch_full_roster(
    client: &reqwest::Client,
    roster_url: &str,
) -> Option<(Vec<MemberRaw>, Vec<MemberRaw>)> {
    let text = client
        .get(roster_url)
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let rows: Vec<RosterRow> = serde_json::from_str(&text).ok()?;

    let member_from = |row: &RosterRow| MemberRaw {
        name: row.player.clone(),
        role: None,
        number: row.number.parse().ok(),
        picture: None,
        geared_picture: None,
    };
    let black: Vec<MemberRaw> = rows
        .iter()
        .filter(|r| r.team == "BLACK")
        .map(member_from)
        .collect();
    let white: Vec<MemberRaw> = rows
        .iter()
        .filter(|r| r.team == "WHITE")
        .map(member_from)
        .collect();
    Some((black, white))
}

/// Which portal game the pictures belong to: the portal address, the event and the game number
/// on screen. A change in any of them means the held portal data is for a different game.
#[derive(Debug, Clone, PartialEq)]
struct PortalTarget {
    portal_url: String,
    event_id: EventId,
    game_number: GameNumber,
}

/// The portal game `snapshot` is showing, or `None` when the feed names no portal event (the
/// bridge sends the event id and the portal address both or neither).
fn portal_target(snapshot: &GameSnapshot) -> Option<PortalTarget> {
    Some(PortalTarget {
        portal_url: snapshot
            .portal_base_url
            .as_deref()?
            .trim_end_matches('/')
            .to_string(),
        event_id: snapshot.event_id.clone()?,
        game_number: snapshot.game_number().clone(),
    })
}

/// Whether two player names are the same person's: equal ignoring case and surrounding spaces.
fn names_match(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// `bridge` with the portal's referees, court and start time; and, only when the portal team of
/// that colour has the same name, that team's logo and each player's photos and role tag from the
/// portal player with the same cap number -- but only when that player's name matches too, so a
/// cap number reused by a different player never shows the wrong face. When the team names
/// differ, no logo, photo or role tag is copied for that team. When the team names match,
/// the portal's coaches and managers (members with no cap number) are appended after the bridge's
/// players, with their names, roles and photos as the portal has them; a different team's staff
/// are never added. Team names, player names and caps, the game
/// number and the event id all stay the bridge's, and a portal player with a cap number the
/// bridge lacks is not added.
fn with_portal_pictures(mut bridge: GameData, portal: &GameData) -> GameData {
    fn merge_team(team: &mut TeamInfoRaw, portal: &TeamInfoRaw) {
        let same_team = names_match(&team.team_name, &portal.team_name);
        team.flag = if same_team { portal.flag.clone() } else { None };
        for member in &mut team.members {
            let found = if same_team {
                member.number.and_then(|number| {
                    portal
                        .members
                        .iter()
                        .find(|p| p.number == Some(number) && names_match(&p.name, &member.name))
                })
            } else {
                None
            };
            if let Some(found) = found {
                member.role = found.role.clone();
            }
            member.picture = found.and_then(|p| p.picture.clone());
            member.geared_picture = found.and_then(|p| p.geared_picture.clone());
        }
        if same_team {
            team.members.extend(
                portal
                    .members
                    .iter()
                    .filter(|p| p.number.is_none())
                    .cloned(),
            );
        }
    }

    merge_team(&mut bridge.black, &portal.black);
    merge_team(&mut bridge.white, &portal.white);
    bridge.referees = portal.referees.clone();
    bridge.pool = portal.pool.clone();
    bridge.start_time = portal.start_time.clone();
    bridge
}

/// Whether the renderer will reset its teams to placeholders on receiving `snapshot`, mirroring
/// the `Snapshot` rule in `main.rs`'s `State::update_state`: a new event or a new game number.
/// `previous` is the event id and game number of the last snapshot sent, `None` before the first.
/// After a reset the game data must be sent again even if it equals what was last sent.
fn renderer_resets_teams(
    previous: Option<&(Option<EventId>, GameNumber)>,
    snapshot: &GameSnapshot,
) -> bool {
    match previous {
        Some((event_id, game_number)) => {
            *event_id != snapshot.event_id || game_number != snapshot.game_number()
        }
        None => true,
    }
}

/// Whether a finished portal fetch is still for the game on screen. One that finished after the
/// game, event or portal changed is discarded.
fn result_is_for_current_target(current: Option<&PortalTarget>, result_for: &PortalTarget) -> bool {
    current == Some(result_for)
}

/// Whether game data must be sent again: only when the bridge's names-only data or the held
/// portal data changed since the last send. Every `GameData` makes the renderer rebuild all its
/// team and player pictures, so resending an unchanged one each tick would waste that work.
fn needs_send(
    last_sent: Option<&(GameData, u64)>,
    bridge_data: &GameData,
    portal_generation: u64,
) -> bool {
    match last_sent {
        Some((data, generation)) => data != bridge_data || *generation != portal_generation,
        None => true,
    }
}

/// How long to wait before asking an unreachable portal again.
const PORTAL_RETRY_INTERVAL: Duration = Duration::from_secs(5);

/// The longest one portal fetch may take before it is abandoned and treated as unreachable. A
/// portal that accepts the connection and then sends nothing would otherwise leave the fetch
/// waiting forever, so its pictures would never arrive and it would never be retried.
const PORTAL_FETCH_TIMEOUT: Duration = Duration::from_secs(60);

/// Fetches one game's portal data until it arrives, then sends it on `result_tx` tagged with the
/// game it is for. Gives up, with a warning, when the portal says there is no such game or
/// refuses the request; retries every [`PORTAL_RETRY_INTERVAL`] while it is unreachable.
async fn fetch_portal_game_data(
    target: PortalTarget,
    result_tx: tokio::sync::mpsc::UnboundedSender<(PortalTarget, GameData)>,
) {
    loop {
        let result = tokio::time::timeout(
            PORTAL_FETCH_TIMEOUT,
            network::try_fetch_game_data(&target.portal_url, &target.event_id, &target.game_number),
        )
        .await
        .unwrap_or(Err(PortalFetchError::Unreachable));
        match result {
            Ok(data) => {
                // The receiver lives as long as the polling loop, which outlives this task.
                let _ = result_tx.send((target, data));
                return;
            }
            Err(PortalFetchError::NotInSchedule) => {
                warn!(
                    "Game {} is not in the portal schedule for event {}; showing it without pictures",
                    target.game_number, target.event_id
                );
                return;
            }
            Err(PortalFetchError::Refused) => {
                warn!(
                    "The portal refused the request for game {} of event {}; showing it without pictures",
                    target.game_number, target.event_id
                );
                return;
            }
            Err(PortalFetchError::Unreachable) => {
                warn!(
                    "Could not get game {} of event {} from the portal; retrying in {}s",
                    target.game_number,
                    target.event_id,
                    PORTAL_RETRY_INTERVAL.as_secs()
                );
                tokio::time::sleep(PORTAL_RETRY_INTERVAL).await;
            }
        }
    }
}

/// Fetches an event's logos until they arrive, then sends them straight to the renderer, which
/// applies them only while that event is still on screen. Gives up, with a warning, when the
/// portal refuses; retries every [`PORTAL_RETRY_INTERVAL`] while it is unreachable.
async fn fetch_event_logos(
    portal_url: String,
    event_id: EventId,
    state_tx: crossbeam_channel::Sender<StateUpdate>,
) {
    loop {
        let result = tokio::time::timeout(
            PORTAL_FETCH_TIMEOUT,
            EventLogos::try_new(&portal_url, event_id.clone()),
        )
        .await
        .unwrap_or(Err(PortalFetchError::Unreachable));
        match result {
            Ok(logos) => {
                state_tx
                    .send(StateUpdate::EventLogos(event_id, logos))
                    .unwrap_or_else(|e| error!("Frontend could not receive event logos!: {e}"));
                return;
            }
            Err(PortalFetchError::Unreachable) => {
                warn!(
                    "Could not get the logos for event {event_id} from the portal; retrying in {}s",
                    PORTAL_RETRY_INTERVAL.as_secs()
                );
                tokio::time::sleep(PORTAL_RETRY_INTERVAL).await;
            }
            Err(e @ (PortalFetchError::Refused | PortalFetchError::NotInSchedule)) => {
                warn!("Giving up on the logos for event {event_id}: {e:?}");
                return;
            }
        }
    }
}

#[tokio::main]
pub async fn networking_thread(
    state_tx: crossbeam_channel::Sender<StateUpdate>,
    config: crate::AppConfig,
) {
    let client = reqwest::Client::new();
    let bridge_url = config.bridge_url.trim_end_matches('/');
    let url = format!("{bridge_url}/game");
    let roster_url = format!("{bridge_url}/roster");
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    let mut schema_mismatch_warned = false;

    network::init_http_client();
    let (result_tx, mut result_rx) =
        tokio::sync::mpsc::unbounded_channel::<(PortalTarget, GameData)>();
    let mut current_target: Option<PortalTarget> = None;
    let mut game_data_task: Option<tokio::task::JoinHandle<()>> = None;
    // The portal's data for `current_target`, once fetched.
    let mut portal_data: Option<GameData> = None;
    // Bumped whenever `portal_data` changes, so `needs_send` can tell without comparing images.
    let mut portal_generation: u64 = 0;
    let mut logos_key: Option<(String, EventId)> = None;
    let mut logos_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut last_sent: Option<(GameData, u64)> = None;
    // The event id and game number of the last snapshot sent, for `renderer_resets_teams`.
    let mut last_snapshot_sent: Option<(Option<EventId>, GameNumber)> = None;

    log::info!("Polling overlay-bridge for game state at {url}");

    loop {
        interval.tick().await;

        let text = match client.get(&url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    warn!("Could not read overlay-bridge response body: {e}");
                    continue;
                }
            },
            Err(e) => {
                // The bridge itself is unreachable -- send nothing, so the renderer
                // freezes on its last known state (see module doc).
                warn!("Could not reach overlay-bridge at {url}: {e}");
                continue;
            }
        };

        let feed = match serde_json::from_str::<GameFeed>(&text) {
            Ok(feed) => feed,
            Err(e) => {
                warn!("Bridge sent an unparseable /game response: {e}");
                continue;
            }
        };

        if feed.schema_version != SUPPORTED_SCHEMA_VERSION {
            if !schema_mismatch_warned {
                error!(
                    "overlay-bridge is serving /game schema version {}, but this renderer \
                         only understands version {SUPPORTED_SCHEMA_VERSION} -- refusing to \
                         render its data until this is resolved.",
                    feed.schema_version
                );
                schema_mismatch_warned = true;
            }
            continue;
        }
        schema_mismatch_warned = false;

        if !feed.connected {
            // The refbox is not alive, per the ONLY field that says so. Send nothing,
            // rather than inventing a value -- same rule as a lost bridge connection.
            debug!("Bridge reports the refbox is not connected");
            continue;
        }

        match build_snapshot(&feed) {
            Some(snapshot) => {
                debug!("Got snapshot from overlay-bridge!");

                let target = portal_target(&snapshot);
                if target != current_target {
                    if let Some(task) = game_data_task.take() {
                        task.abort();
                    }
                    portal_data = None;
                    portal_generation += 1;
                    if let Some(target) = target.clone() {
                        game_data_task = Some(tokio::spawn(fetch_portal_game_data(
                            target,
                            result_tx.clone(),
                        )));
                    }
                    current_target = target;
                }

                while let Ok((result_for, data)) = result_rx.try_recv() {
                    if result_is_for_current_target(current_target.as_ref(), &result_for) {
                        portal_data = Some(data);
                        portal_generation += 1;
                    }
                }

                // A second request, not part of the /game feed itself: the pre-game reveal
                // needs every player on both rosters, not just whoever currently has a
                // penalty. Fetched every tick, same as /game -- rosters change rarely, but
                // there's no cheap way to know when, and this is a small response.
                let full_roster = fetch_full_roster(&client, &roster_url).await;

                // The snapshot must be sent -- and therefore applied by the renderer --
                // before the game data that depends on it: `State::update_state`'s
                // `GameData` handling only accepts an update whose `event_id` and
                // `game_number` match what's *already* in `self.snapshot`, so sending
                // this first on the same in-order channel is what makes that match land.
                let game_data = build_game_data(&feed, &snapshot, full_roster.as_ref());

                if renderer_resets_teams(last_snapshot_sent.as_ref(), &snapshot) {
                    last_sent = None;
                }
                last_snapshot_sent =
                    Some((snapshot.event_id.clone(), snapshot.game_number().clone()));

                state_tx
                    .send(StateUpdate::Snapshot(snapshot))
                    .unwrap_or_else(|e| error!("Frontend could not receive snapshot!: {e}"));

                // Only after this tick's snapshot is sent: the logos task sends straight to the
                // renderer, which drops logos for any event but its current snapshot's. The
                // channel is in order, so starting the task here means its logos can never
                // overtake the snapshot that introduces their event.
                let key = current_target
                    .as_ref()
                    .map(|t| (t.portal_url.clone(), t.event_id.clone()));
                if key != logos_key {
                    if let Some(task) = logos_task.take() {
                        task.abort();
                    }
                    if let Some((portal_url, event_id)) = key.clone() {
                        logos_task = Some(tokio::spawn(fetch_event_logos(
                            portal_url,
                            event_id,
                            state_tx.clone(),
                        )));
                    }
                    logos_key = key;
                }

                if let Some(bridge_data) = game_data {
                    if needs_send(last_sent.as_ref(), &bridge_data, portal_generation) {
                        let to_send = match &portal_data {
                            Some(portal) => with_portal_pictures(bridge_data.clone(), portal),
                            None => bridge_data.clone(),
                        };
                        state_tx
                            .send(StateUpdate::GameData(to_send))
                            .unwrap_or_else(|e| {
                                error!("Frontend could not receive game data!: {e}")
                            });
                        last_sent = Some((bridge_data, portal_generation));
                    }
                }
            }
            None => warn!("Bridge feed had an unrecognised value; discarding this update"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::Image;

    fn img(v: u8) -> Image {
        (1, 1, vec![v, v, v, 255])
    }

    fn member(name: &str, number: u8) -> MemberRaw {
        MemberRaw {
            name: name.to_string(),
            role: None,
            number: Some(number),
            picture: None,
            geared_picture: None,
        }
    }

    fn game(black: TeamInfoRaw, white: TeamInfoRaw) -> GameData {
        GameData {
            pool: String::new(),
            start_time: String::new(),
            referees: Vec::new(),
            black,
            white,
            game_number: "12".to_string(),
            event_id: None,
        }
    }

    fn team(name: &str, members: Vec<MemberRaw>) -> TeamInfoRaw {
        TeamInfoRaw {
            team_name: name.to_string(),
            members,
            flag: None,
        }
    }

    fn target(game_number: &str) -> PortalTarget {
        PortalTarget {
            portal_url: "https://portal.example".to_string(),
            event_id: EventId::from_full("events/1889-B").unwrap(),
            game_number: game_number.to_string(),
        }
    }

    #[test]
    fn a_photo_is_copied_when_cap_and_name_match() {
        let bridge = game(team("B", vec![member("Ann", 3)]), team("W", vec![]));
        let mut portal_ann = member("Ann", 3);
        portal_ann.picture = Some(img(1));
        portal_ann.geared_picture = Some(img(2));
        let portal = game(team("B", vec![portal_ann]), team("W", vec![]));

        let merged = with_portal_pictures(bridge, &portal);
        assert!(merged.black.members[0].picture == Some(img(1)));
        assert!(merged.black.members[0].geared_picture == Some(img(2)));
    }

    #[test]
    fn no_photo_when_the_portal_name_for_that_cap_differs() {
        let bridge = game(team("B", vec![member("Ann", 3)]), team("W", vec![]));
        let mut portal_bea = member("Bea", 3);
        portal_bea.picture = Some(img(1));
        let portal = game(team("B", vec![portal_bea]), team("W", vec![]));

        let merged = with_portal_pictures(bridge, &portal);
        assert!(merged.black.members[0].picture.is_none());
        assert!(merged.black.members[0].geared_picture.is_none());
        assert_eq!(merged.black.members[0].name, "Ann");
    }

    #[test]
    fn no_photo_or_role_when_the_team_names_differ() {
        let bridge = game(
            team("BRIDGE NAME", vec![member("Ann", 3)]),
            team("W", vec![]),
        );
        let mut portal_ann = member("Ann", 3);
        portal_ann.picture = Some(img(1));
        portal_ann.geared_picture = Some(img(2));
        portal_ann.role = Some("Captain".to_string());
        let portal = game(team("PORTAL NAME", vec![portal_ann]), team("W", vec![]));

        let merged = with_portal_pictures(bridge, &portal);
        assert!(merged.black.members[0].picture.is_none());
        assert!(merged.black.members[0].geared_picture.is_none());
        assert!(merged.black.members[0].role.is_none());
    }

    #[test]
    fn names_match_ignores_case_and_surrounding_spaces() {
        assert!(names_match(" ann ", "ANN"));
        assert!(!names_match("Ann", "Anna"));
    }

    #[test]
    fn team_logo_comes_from_the_portal_when_team_names_match() {
        let bridge = game(team("TEAM A", vec![]), team("W", vec![]));
        let mut portal_black = team("Team A ", vec![]);
        portal_black.flag = Some(img(9));
        let portal = game(portal_black, team("W", vec![]));

        let merged = with_portal_pictures(bridge, &portal);
        assert_eq!(merged.black.team_name, "TEAM A");
        assert!(merged.black.flag == Some(img(9)));
    }

    #[test]
    fn no_team_logo_when_the_team_names_differ() {
        let bridge = game(team("BRIDGE NAME", vec![]), team("W", vec![]));
        let mut portal_black = team("PORTAL NAME", vec![]);
        portal_black.flag = Some(img(9));
        let mut portal = game(portal_black, team("W", vec![]));
        portal.referees = vec![member("Ref", 0)];
        portal.pool = "COURT: 1".to_string();
        portal.start_time = "START: 10:00".to_string();

        let merged = with_portal_pictures(bridge, &portal);
        assert_eq!(merged.black.team_name, "BRIDGE NAME");
        assert!(merged.black.flag.is_none());
        assert!(merged.referees == vec![member("Ref", 0)]);
        assert_eq!(merged.pool, "COURT: 1");
        assert_eq!(merged.start_time, "START: 10:00");
    }

    #[test]
    fn a_matched_player_gets_the_portal_role() {
        let mut portal_ann = member("Ann", 3);
        portal_ann.role = Some("Captain".to_string());
        let portal = game(team("B", vec![portal_ann]), team("W", vec![]));

        let bridge = game(team("B", vec![member("Ann", 3)]), team("W", vec![]));
        let merged = with_portal_pictures(bridge, &portal);
        assert_eq!(merged.black.members[0].role.as_deref(), Some("Captain"));

        let bridge = game(team("B", vec![member("Bea", 3)]), team("W", vec![]));
        let merged = with_portal_pictures(bridge, &portal);
        assert!(merged.black.members[0].role.is_none());
    }

    #[test]
    fn portal_staff_with_no_cap_are_added_with_their_photos_and_role() {
        let coach = MemberRaw {
            name: "Coach Bob".to_string(),
            role: Some("Coach".to_string()),
            number: None,
            picture: Some(img(5)),
            geared_picture: None,
        };
        let bridge = game(team("B", vec![member("Ann", 3)]), team("W", vec![]));
        let portal = game(team("B", vec![member("Ann", 3), coach]), team("W", vec![]));

        let merged = with_portal_pictures(bridge, &portal);
        assert_eq!(merged.black.members.len(), 2);
        let added = &merged.black.members[1];
        assert_eq!(added.name, "Coach Bob");
        assert_eq!(added.role.as_deref(), Some("Coach"));
        assert!(added.picture == Some(img(5)));
        assert!(added.number.is_none());
    }

    #[test]
    fn no_portal_staff_when_the_team_names_differ() {
        let coach = MemberRaw {
            name: "Coach Bob".to_string(),
            role: Some("Coach".to_string()),
            number: None,
            picture: Some(img(5)),
            geared_picture: None,
        };
        let bridge = game(
            team("BRIDGE NAME", vec![member("Ann", 3)]),
            team("W", vec![]),
        );
        let portal = game(team("PORTAL NAME", vec![coach]), team("W", vec![]));

        let merged = with_portal_pictures(bridge.clone(), &portal);
        assert!(merged.black.members == bridge.black.members);
    }

    #[test]
    fn a_game_number_flip_back_resends_game_data() {
        let e = Some(EventId::from_full("events/1889-B").unwrap());
        let snap = |game_number: &str| GameSnapshot {
            event_id: e.clone(),
            game_number: game_number.to_string(),
            is_old_game: true,
            ..Default::default()
        };
        assert!(renderer_resets_teams(
            Some(&(e.clone(), "5".to_string())),
            &snap("6")
        ));
        assert!(renderer_resets_teams(
            Some(&(e.clone(), "6".to_string())),
            &snap("5")
        ));
        assert!(!renderer_resets_teams(
            Some(&(e.clone(), "5".to_string())),
            &snap("5")
        ));
    }

    #[test]
    fn a_new_event_resends_game_data() {
        let e1 = Some(EventId::from_full("events/1889-B").unwrap());
        let e2 = Some(EventId::from_full("events/2001-C").unwrap());
        let snapshot = GameSnapshot {
            event_id: e2,
            game_number: "5".to_string(),
            is_old_game: true,
            ..Default::default()
        };
        assert!(renderer_resets_teams(
            Some(&(e1, "5".to_string())),
            &snapshot
        ));
        assert!(renderer_resets_teams(None, &snapshot));
    }

    #[test]
    fn portal_only_players_are_not_added() {
        let bridge = game(team("B", vec![member("Ann", 3)]), team("W", vec![]));
        let portal = game(
            team("B", vec![member("Ann", 3), member("Cat", 7)]),
            team("W", vec![]),
        );

        let merged = with_portal_pictures(bridge, &portal);
        assert_eq!(merged.black.members.len(), 1);
    }

    #[test]
    fn portal_target_needs_both_event_and_address_and_trims_the_slash() {
        let event_id = EventId::from_full("events/1889-B").unwrap();
        let both = GameSnapshot {
            event_id: Some(event_id.clone()),
            portal_base_url: Some("https://portal.example/".to_string()),
            game_number: "12".to_string(),
            is_old_game: true,
            ..Default::default()
        };
        let t = portal_target(&both).unwrap();
        assert_eq!(t.portal_url, "https://portal.example");
        assert_eq!(t.event_id, event_id);
        assert_eq!(&t.game_number, both.game_number());

        let no_event = GameSnapshot {
            event_id: None,
            ..both.clone()
        };
        assert!(portal_target(&no_event).is_none());
        let no_address = GameSnapshot {
            portal_base_url: None,
            ..both
        };
        assert!(portal_target(&no_address).is_none());
    }

    #[test]
    fn a_result_for_a_previous_game_is_discarded() {
        let game_1 = target("1");
        let game_2 = target("2");
        assert!(!result_is_for_current_target(Some(&game_2), &game_1));
        assert!(result_is_for_current_target(Some(&game_1), &game_1));
        assert!(!result_is_for_current_target(None, &game_1));
    }

    #[test]
    fn needs_send_only_when_names_or_portal_data_change() {
        let data = game(team("B", vec![member("Ann", 3)]), team("W", vec![]));
        let other = game(team("B", vec![member("Bea", 3)]), team("W", vec![]));
        let last = (data.clone(), 1);
        assert!(needs_send(None, &data, 1));
        assert!(!needs_send(Some(&last), &data, 1));
        assert!(needs_send(Some(&last), &data, 2));
        assert!(needs_send(Some(&last), &other, 1));
    }
}
