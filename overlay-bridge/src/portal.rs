//! Resolves team names (from the event's public schedule) and player names (from team rosters)
//! against the uwhportal Portal, for display alongside the refbox's own live feed.
//!
//! **No failure here is ever fatal to what's on screen.** The existing `overlay` crate does the
//! opposite: when it fetches a team's roster or an event's logos, a failed request is treated as
//! final for that lookup and never retried (`overlay/src/network.rs`, `TeamInfoRaw::new` and
//! `EventLogos::new`). It doesn't crash, but that one fetch dies forever -- so at a venue with
//! patchy internet, a team's name and roster can simply never appear for an entire game, with
//! nothing on screen explaining why. It only recovers when the game changes and a fresh fetch
//! happens to succeed.
//!
//! [`Directory`] is built the opposite way: [`Directory::refresh_schedule`] and
//! [`Directory::refresh_roster`] are meant to be called repeatedly, on a timer, by whatever owns
//! this `Directory` (a later task wires that loop up). Every successful fetch replaces the
//! relevant cache entry; every failure -- a network error, a non-2xx status, a body that isn't
//! valid JSON, or JSON that isn't shaped the way the portal is expected to shape it -- leaves the
//! existing cache exactly as it was and is reported back as `false`, never as a panic or an
//! `Err` the caller has to unwrap. [`Directory::names_for`] and [`Directory::player_name`] are
//! plain synchronous reads of whatever is cached right now: if nothing has ever been fetched
//! successfully, they return `None`, so the natural fallback is to display cap numbers with no
//! name and let the refbox's own game data stand on its own -- never to error out or to block on
//! the network.
//!
//! No `expect()` or `unwrap()` appears anywhere on a network call, a response body, or a JSON
//! parse in this module -- every one of those is a live panic point in the overlay today.

use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use uwh_common::uwhportal::schedule::{EventId, TeamId};

/// Team names resolved for one game's two colour slots.
///
/// Either side is `None` when the portal has no name to offer for that slot -- either because
/// the schedule has no team assigned there yet (a bracket placeholder waiting on an earlier
/// result), or because nothing has been fetched successfully yet. Callers should fall back to
/// whatever the refbox itself already reports for that slot in this case, rather than treating
/// it as an error: the refbox's own game data does not come from the portal at all, and a portal
/// outage must never affect it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamNames {
    pub dark: Option<String>,
    pub light: Option<String>,
}

/// One game's schedule-derived facts, as read from the public schedule endpoint's matched game
/// object.
///
/// `court` and `startsOn` live on the matched game object itself, never at the schedule
/// response's top level -- reading them from the top level was PR #2474's bug, and this struct
/// exists partly so the parsing that avoids it is exercised directly by a test, independent of
/// whether a later task ever surfaces `court` or `start_time` on the outside.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GameEntry {
    dark_team: Option<TeamId>,
    light_team: Option<TeamId>,
    court: Option<String>,
    start_time: Option<String>,
}

/// The full result of one successful schedule fetch: every game found, keyed by its (string)
/// game number, plus the top-level `teams` map the schedule response carries team display names
/// in. A schedule fetch that succeeds always replaces this wholesale -- there is no partial
/// merge -- since a stale team assignment mixed with fresh ones would be worse than either alone.
#[derive(Debug, Default)]
struct ScheduleCache {
    games: HashMap<String, GameEntry>,
    teams: HashMap<TeamId, String>,
}

/// Caches the portal's team names and player rosters for one event and resolves them for
/// display, without ever letting a portal outage become an error the caller has to handle or a
/// panic that takes the bridge down. See the module doc for the full "never fatal" contract.
pub struct Directory {
    client: Client,
    portal_url: String,
    event_id: EventId,
    schedule: RwLock<ScheduleCache>,
    rosters: RwLock<HashMap<TeamId, HashMap<u8, String>>>,
}

impl Directory {
    /// Builds an empty directory for one event. Nothing is fetched yet -- [`refresh_schedule`]
    /// and [`refresh_roster`] must be called (and, in practice, called again on a timer by
    /// whatever owns this `Directory`) before [`names_for`] or [`player_name`] have anything to
    /// return.
    ///
    /// [`refresh_schedule`]: Directory::refresh_schedule
    /// [`refresh_roster`]: Directory::refresh_roster
    /// [`names_for`]: Directory::names_for
    /// [`player_name`]: Directory::player_name
    pub fn new(client: Client, portal_url: String, event_id: EventId) -> Self {
        Self {
            client,
            portal_url,
            event_id,
            schedule: RwLock::new(ScheduleCache::default()),
            rosters: RwLock::new(HashMap::new()),
        }
    }

    /// Fetches the event's public schedule and, on success, wholesale-replaces the cached team
    /// names and per-game team assignments with it.
    ///
    /// One call resolves both team names for every game in the event: the schedule response's
    /// top-level `teams` map already carries each assigned team's display name, so no second
    /// (per-team) request is needed just to get names -- that second endpoint exists only for
    /// rosters (see [`refresh_roster`]).
    ///
    /// Returns `false` on any failure -- a network error, a non-2xx status, a body that isn't
    /// valid JSON, or JSON missing the `games` array the response is expected to have -- and
    /// leaves the existing cache completely untouched, so a caller retrying this on a timer
    /// degrades to "stale but still serving" rather than ever going blank.
    ///
    /// [`refresh_roster`]: Directory::refresh_roster
    pub async fn refresh_schedule(&self) -> bool {
        let url = format!(
            "{}/api/events/{}/schedule",
            self.portal_url,
            self.event_id.partial()
        );
        let Some(body) = fetch_text(self.client.get(url)).await else {
            return false;
        };
        let Ok(data) = serde_json::from_str::<Value>(&body) else {
            return false;
        };
        let Some(cache) = parse_schedule(&data) else {
            return false;
        };

        *write_lock(&self.schedule) = cache;
        true
    }

    /// Looks up the team names cached for `game_number` by the most recent successful
    /// [`refresh_schedule`]. `None` if `game_number` is not present in that cache -- either
    /// because nothing has ever been fetched successfully, or because the schedule genuinely has
    /// no such game.
    ///
    /// A game whose schedule entry has no team assigned to one or both colours (a bracket
    /// placeholder waiting on an earlier result -- see the module doc) is not an error: it comes
    /// back as `Some(TeamNames { dark: None, .. })` (or `light`), not `None` and not a panic.
    ///
    /// [`refresh_schedule`]: Directory::refresh_schedule
    pub fn names_for(&self, game_number: &str) -> Option<TeamNames> {
        let schedule = read_lock(&self.schedule);
        let entry = schedule.games.get(game_number)?;
        let dark = entry
            .dark_team
            .as_ref()
            .and_then(|id| schedule.teams.get(id))
            .cloned();
        let light = entry
            .light_team
            .as_ref()
            .and_then(|id| schedule.teams.get(id))
            .cloned();
        Some(TeamNames { dark, light })
    }

    /// Fetches `team`'s roster and, on success, replaces the cached roster for that team with
    /// it. Other teams' cached rosters are untouched.
    ///
    /// Returns `false` on any failure -- a network error, a non-2xx status, a body that isn't
    /// valid JSON, or JSON missing the `roster` array the response is expected to have -- and
    /// leaves this team's existing cached roster (if any) completely untouched.
    pub async fn refresh_roster(&self, team: &TeamId) -> bool {
        let request = self
            .client
            .get(format!("{}/api/admin/get-event-team", self.portal_url))
            .query(&[("teamId", team.full())]);
        let Some(body) = fetch_text(request).await else {
            return false;
        };
        let Ok(data) = serde_json::from_str::<Value>(&body) else {
            return false;
        };
        let Some(roster) = parse_roster(&data) else {
            return false;
        };

        write_lock(&self.rosters).insert(team.clone(), roster);
        true
    }

    /// Looks up `cap_number`'s display name on `team`'s roster, as cached by the most recent
    /// successful [`refresh_roster`] for that team.
    ///
    /// `None` if `team`'s roster has never been fetched successfully, or if `cap_number` isn't
    /// on it -- in both cases, the caller's fallback is to display the cap number with no name,
    /// not to treat this as an error.
    ///
    /// [`refresh_roster`]: Directory::refresh_roster
    pub fn player_name(&self, team: &TeamId, cap_number: u8) -> Option<String> {
        read_lock(&self.rosters)
            .get(team)?
            .get(&cap_number)
            .cloned()
    }
}

/// Sends `request` and returns its body as text, or `None` on any failure: the request couldn't
/// be sent, the response status wasn't successful, or the body couldn't be read. Never panics.
async fn fetch_text(request: RequestBuilder) -> Option<String> {
    let response = request.send().await.ok()?;
    let response = response.error_for_status().ok()?;
    response.text().await.ok()
}

/// Parses a public schedule response's `games` array and top-level `teams` map into a
/// [`ScheduleCache`]. Returns `None` if `data` doesn't even have a `games` array -- an
/// unexpected enough shape that it should not overwrite a previously good cache -- but is
/// otherwise as forgiving as possible: a game missing a field it's expected to have simply
/// carries `None` for that field rather than aborting the whole parse, and an unrecognised or
/// malformed team id in `teams` is skipped rather than failing the parse.
fn parse_schedule(data: &Value) -> Option<ScheduleCache> {
    let games_json = data.get("games")?.as_array()?;

    let mut games = HashMap::new();
    for game in games_json {
        let Some(number) = game.get("number").and_then(Value::as_str) else {
            continue;
        };

        let dark_team = game
            .pointer("/dark/assignment/teamId")
            .and_then(Value::as_str)
            .and_then(|id| TeamId::from_full(id).ok());
        let light_team = game
            .pointer("/light/assignment/teamId")
            .and_then(Value::as_str)
            .and_then(|id| TeamId::from_full(id).ok());
        let court = game
            .get("court")
            .and_then(Value::as_str)
            .map(str::to_string);
        let start_time = game
            .get("startsOn")
            .and_then(Value::as_str)
            .map(str::to_string);

        games.insert(
            number.to_string(),
            GameEntry {
                dark_team,
                light_team,
                court,
                start_time,
            },
        );
    }

    let mut teams = HashMap::new();
    if let Some(teams_json) = data.get("teams").and_then(Value::as_object) {
        for (id, team) in teams_json {
            let Ok(team_id) = TeamId::from_full(id) else {
                continue;
            };
            // Trimmed and upper-cased, matching the overlay's own display convention for team
            // names (`overlay/src/network.rs`, `TeamInfoRaw::new`).
            if let Some(name) = team.get("name").and_then(Value::as_str) {
                teams.insert(team_id, name.trim().to_uppercase());
            }
        }
    }

    Some(ScheduleCache { games, teams })
}

/// Parses a team roster response's `roster` array into a cap-number-to-name map. Returns `None`
/// if `data` doesn't even have a `roster` array. A member missing a usable `capNumber` is
/// skipped (there's no cap number to key it by); a member missing `rosterName` displays as
/// `"Player"`, matching the overlay's own convention (`overlay/src/network.rs`,
/// `TeamInfoRaw::new`).
fn parse_roster(data: &Value) -> Option<HashMap<u8, String>> {
    let roster_json = data.get("roster")?.as_array()?;

    let mut roster = HashMap::new();
    for member in roster_json {
        let Some(cap_number) = member
            .get("capNumber")
            .and_then(Value::as_u64)
            .and_then(|n| u8::try_from(n).ok())
        else {
            continue;
        };
        let name = member
            .get("rosterName")
            .and_then(Value::as_str)
            .unwrap_or("Player")
            .trim()
            .to_string();
        roster.insert(cap_number, name);
    }

    Some(roster)
}

/// Reads `lock`, recovering the guard even if a previous holder panicked while holding it rather
/// than propagating the poison as a panic of its own. Nothing in this module panics while
/// holding either lock -- both critical sections are a handful of infallible map operations --
/// so poisoning should never actually happen; this exists so that if it somehow did, a directory
/// lookup would keep serving its last good cache instead of taking the whole bridge down with
/// it, in keeping with this module's "never fatal" contract.
fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// The write-side counterpart of [`read_lock`]; see its doc for why poison is recovered rather
/// than propagated.
fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    /// A public schedule response trimmed from the real dev portal (event `1889-B`): two games,
    /// one with both teams assigned and one with `teamId: null` on both sides (a bracket
    /// placeholder), plus the `teams` entries they reference.
    const SCHEDULE_FIXTURE: &str = include_str!("../tests/fixtures/schedule-response.json");

    /// A real 12-member roster response. Cap numbers and every field name are exactly as the
    /// portal returned them; the player names are replaced and photo URLs nulled, since the
    /// originals are real people and this file is committed.
    const ROSTER_FIXTURE: &str = include_str!("../tests/fixtures/team-roster-response.json");

    fn team_2529b() -> TeamId {
        TeamId::from_full("teams/2529-B").expect("fixture id should be valid")
    }

    fn team_2530b() -> TeamId {
        TeamId::from_full("teams/2530-B").expect("fixture id should be valid")
    }

    fn schedule_fixture_json() -> Value {
        serde_json::from_str(SCHEDULE_FIXTURE).expect("fixture should be valid JSON")
    }

    fn roster_fixture_json() -> Value {
        serde_json::from_str(ROSTER_FIXTURE).expect("fixture should be valid JSON")
    }

    fn directory_at(portal_url: String) -> Directory {
        Directory::new(Client::new(), portal_url, EventId::from_partial("1889-B"))
    }

    // ---- pure parsing: no network involved ----

    #[test]
    fn schedule_fixture_yields_team_ids_court_and_start_time_for_game_1() {
        let cache = parse_schedule(&schedule_fixture_json()).expect("fixture should parse");
        let game = cache.games.get("1").expect("game 1 should be present");

        assert_eq!(game.dark_team, Some(team_2529b()));
        assert_eq!(game.light_team, Some(team_2530b()));
        assert_eq!(game.court.as_deref(), Some("1"));
        assert_eq!(
            game.start_time.as_deref(),
            Some("2026-08-01T09:30:00+10:00")
        );
    }

    #[test]
    fn schedule_fixture_team_names_come_from_the_top_level_teams_map() {
        let cache = parse_schedule(&schedule_fixture_json()).expect("fixture should parse");

        // Deliberately NOT "Sydney Kings A" / "Brisbane Barracudas" -- those are the
        // `pendingAssignmentName` values on the game's own assignment, a different field the
        // brief says is not where display names come from.
        assert_eq!(
            cache.teams.get(&team_2529b()).map(String::as_str),
            Some("SYDNEY KINGS A")
        );
        assert_eq!(
            cache.teams.get(&team_2530b()).map(String::as_str),
            Some("BRISBANE A")
        );
    }

    #[test]
    fn unassigned_slot_yields_no_team_id_and_does_not_error() {
        let cache = parse_schedule(&schedule_fixture_json()).expect("fixture should parse");
        let game = cache.games.get("51").expect("game 51 should be present");

        assert_eq!(game.dark_team, None);
        assert_eq!(game.light_team, None);
        // Still has its other fields -- a null teamId doesn't take the rest of the game with it.
        assert_eq!(game.court.as_deref(), Some("1"));
    }

    #[test]
    fn names_for_resolves_both_teams_from_a_cached_schedule() {
        let directory = directory_at("http://portal.invalid".to_string());
        *write_lock(&directory.schedule) =
            parse_schedule(&schedule_fixture_json()).expect("fixture should parse");

        let names = directory.names_for("1").expect("game 1 should be present");
        assert_eq!(names.dark.as_deref(), Some("SYDNEY KINGS A"));
        assert_eq!(names.light.as_deref(), Some("BRISBANE A"));
    }

    #[test]
    fn names_for_an_unassigned_slot_is_some_with_no_names_not_an_error() {
        let directory = directory_at("http://portal.invalid".to_string());
        *write_lock(&directory.schedule) =
            parse_schedule(&schedule_fixture_json()).expect("fixture should parse");

        let names = directory
            .names_for("51")
            .expect("game 51 should be present");
        assert_eq!(names, TeamNames::default());
    }

    #[test]
    fn names_for_an_unknown_game_number_is_none() {
        let directory = directory_at("http://portal.invalid".to_string());
        *write_lock(&directory.schedule) =
            parse_schedule(&schedule_fixture_json()).expect("fixture should parse");

        assert_eq!(directory.names_for("does-not-exist"), None);
    }

    #[test]
    fn roster_fixture_maps_cap_numbers_to_names() {
        let roster = parse_roster(&roster_fixture_json()).expect("fixture should parse");

        assert_eq!(roster.len(), 12);
        assert_eq!(roster.get(&2).map(String::as_str), Some("A. Fisher"));
        assert_eq!(roster.get(&27).map(String::as_str), Some("L. Haugen"));
        assert_eq!(roster.get(&99), None);
    }

    /// Valid JSON, but not the shape either parser expects -- neither has the array it looks
    /// for, so both should decline to produce a cache rather than panic on a missing key.
    fn empty_object() -> Value {
        serde_json::from_str("{}").expect("empty object is valid JSON")
    }

    #[test]
    fn malformed_schedule_json_is_handled_without_panicking() {
        assert!(parse_schedule(&empty_object()).is_none());
    }

    #[test]
    fn malformed_roster_json_is_handled_without_panicking() {
        assert!(parse_roster(&empty_object()).is_none());
    }

    // ---- network: proves the "never fatal" contract against a real (fake) HTTP server ----

    /// Accepts exactly one connection on `listener`, drains its request, and writes back
    /// `status_line` with `body` as the response, then closes the connection. Mirrors the
    /// "act as the other side of the socket" pattern `feed.rs`'s own tests use, just speaking
    /// enough raw HTTP/1.1 for `reqwest` to parse a response out of it.
    async fn serve_one(listener: TcpListener, status_line: &str, body: &str) {
        let (mut socket, _) = listener.accept().await.expect("accept a connection");

        // Drain the request so the client isn't left waiting on a full-duplex write; the
        // request's content doesn't matter to any of these tests, only that a request landed.
        let mut request = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let read = socket.read(&mut chunk).await.unwrap_or(0);
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write the mock response");
        let _ = socket.shutdown().await;
    }

    #[tokio::test]
    async fn a_successful_schedule_fetch_populates_names_for() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let directory = directory_at(format!("http://{addr}"));

        let server = tokio::spawn(serve_one(listener, "200 OK", SCHEDULE_FIXTURE));
        let ok = directory.refresh_schedule().await;
        server.await.expect("mock server task");

        assert!(ok, "fetch against the fixture body should succeed");
        let names = directory.names_for("1").expect("game 1 should be present");
        assert_eq!(names.dark.as_deref(), Some("SYDNEY KINGS A"));
        assert_eq!(names.light.as_deref(), Some("BRISBANE A"));
    }

    #[tokio::test]
    async fn a_successful_roster_fetch_populates_player_name() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let directory = directory_at(format!("http://{addr}"));
        let team = team_2529b();

        let server = tokio::spawn(serve_one(listener, "200 OK", ROSTER_FIXTURE));
        let ok = directory.refresh_roster(&team).await;
        server.await.expect("mock server task");

        assert!(ok, "fetch against the fixture body should succeed");
        assert_eq!(
            directory.player_name(&team, 2).as_deref(),
            Some("A. Fisher")
        );
    }

    #[tokio::test]
    async fn a_failing_roster_request_leaves_a_previously_cached_roster_intact() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let directory = directory_at(format!("http://{addr}"));
        let team = team_2529b();

        // First: a good response, seeds the cache.
        let server = tokio::spawn(serve_one(listener, "200 OK", ROSTER_FIXTURE));
        let first = directory.refresh_roster(&team).await;
        server.await.expect("mock server task");
        assert!(first, "seeding fetch should succeed");
        assert_eq!(
            directory.player_name(&team, 2).as_deref(),
            Some("A. Fisher")
        );

        // The listener from the first fetch has now gone out of scope, so nothing is listening
        // at `addr` any more -- the second fetch is refused at the TCP level, the same way
        // `feed.rs`'s own tests simulate a genuinely unreachable peer.
        let second = directory.refresh_roster(&team).await;
        assert!(!second, "fetch against a closed port should fail");

        // The cache must still hold what the FIRST fetch cached -- not be cleared, and not hold
        // anything from the failed second attempt (there was nothing to hold).
        assert_eq!(
            directory.player_name(&team, 2).as_deref(),
            Some("A. Fisher")
        );
    }

    #[tokio::test]
    async fn a_failing_request_with_no_cache_yields_no_names_not_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let addr = listener.local_addr().expect("local_addr");
        drop(listener); // nothing is listening at `addr` any more; every connect is refused

        let directory = directory_at(format!("http://{addr}"));
        let team = team_2529b();

        let schedule_ok = directory.refresh_schedule().await;
        assert!(!schedule_ok);
        assert_eq!(directory.names_for("1"), None);

        let roster_ok = directory.refresh_roster(&team).await;
        assert!(!roster_ok);
        assert_eq!(directory.player_name(&team, 2), None);
    }

    #[tokio::test]
    async fn a_non_json_response_body_does_not_panic_and_is_not_cached() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let directory = directory_at(format!("http://{addr}"));

        let server = tokio::spawn(serve_one(listener, "200 OK", "this is not json at all"));
        let ok = directory.refresh_schedule().await;
        server.await.expect("mock server task");

        assert!(!ok, "a non-JSON body must not be treated as a success");
        assert_eq!(directory.names_for("1"), None);
    }

    #[tokio::test]
    async fn a_server_error_status_is_treated_as_a_failure_not_a_success() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let directory = directory_at(format!("http://{addr}"));

        let server = tokio::spawn(serve_one(
            listener,
            "500 Internal Server Error",
            SCHEDULE_FIXTURE,
        ));
        let ok = directory.refresh_schedule().await;
        server.await.expect("mock server task");

        assert!(
            !ok,
            "a 500 status must not be treated as a success, even with a valid body"
        );
        assert_eq!(directory.names_for("1"), None);
    }

    /// Sanity check that the timeout-free tests above aren't just hanging forever and being
    /// killed by the test harness -- if `fetch_text` ever regressed to blocking indefinitely
    /// instead of returning `None` on a connection refusal, this bounds how long that could go
    /// unnoticed.
    #[tokio::test]
    async fn a_refused_connection_does_not_hang() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let addr = listener.local_addr().expect("local_addr");
        drop(listener);

        let directory = directory_at(format!("http://{addr}"));
        let result = tokio::time::timeout(Duration::from_secs(5), directory.refresh_schedule())
            .await
            .expect("refresh_schedule should not hang on a refused connection");
        assert!(!result);
    }
}
