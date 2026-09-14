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
//! the bridge -- no portal lookup needed on this side for just the names). Full team
//! rosters and player photos (the roster page) still come from `network.rs`'s existing
//! portal-fetching code, which this module does not yet call -- see the module-level TODO
//! below.

use std::time::Duration;

use log::{debug, error, warn};
use serde::Deserialize;
use uwh_common::{
    bundles::BlackWhiteBundle,
    color::Color,
    game_snapshot::{
        GamePeriod, GameSnapshot, Infraction, PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
    },
    uwhportal::schedule::EventId,
};

use crate::network::{GameData, MemberRaw, StateUpdate, TeamInfoRaw};

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
/// doesn't. No player photos: a CSV-sourced roster has none, per design, and this doesn't
/// distinguish that from a Portal-sourced one not having resolved one yet -- both are simply
/// absent, the same "absent, not impossible" modelling the roster-from-local-file design settled
/// on.
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

                state_tx
                    .send(StateUpdate::Snapshot(snapshot))
                    .unwrap_or_else(|e| error!("Frontend could not receive snapshot!: {e}"));

                if let Some(game_data) = game_data {
                    state_tx
                        .send(StateUpdate::GameData(game_data))
                        .unwrap_or_else(|e| error!("Frontend could not receive game data!: {e}"));
                }
            }
            None => warn!("Bridge feed had an unrecognised value; discarding this update"),
        }
    }
}
