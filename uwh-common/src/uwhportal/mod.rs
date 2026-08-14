use crate::bundles::BlackWhiteBundle;
use core::{cell::OnceCell, time::Duration};
use log::{debug, info, warn};
use rand::{Rng, SeedableRng};
use reqwest::{
    Client, ClientBuilder, Method, RequestBuilder, StatusCode,
    header::{AUTHORIZATION, HeaderValue},
};
use schedule::{EventId, GameNumber, TeamId, TeamList};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
};

pub mod schedule;

// --- Coin-flip portal types (used by scoresheet generation / coin-flip resolution) ---

#[derive(Debug, Clone, Deserialize)]
pub struct CoinFlipDetails {
    #[serde(rename = "Groups", alias = "groups")]
    pub groups: Vec<GroupCoinFlips>,
    #[serde(rename = "Games", alias = "games")]
    pub games: Vec<CoinFlip>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupCoinFlips {
    #[serde(rename = "Identifier", alias = "identifier")]
    pub identifier: String,
    #[serde(rename = "Name", alias = "name")]
    pub name: String,
    #[serde(rename = "ShortName", alias = "shortName")]
    pub short_name: Option<String>,
    #[serde(rename = "CoinFlips", alias = "coinFlips")]
    pub coin_flips: Vec<CoinFlip>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoinFlip {
    #[serde(rename = "Identifier", alias = "identifier")]
    pub identifier: String,
    #[serde(rename = "TiedTeams", alias = "tiedTeams")]
    pub tied_teams: Vec<CoinFlipTeam>,
    #[serde(rename = "Result", alias = "result")]
    pub result: Option<CoinFlipResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoinFlipTeam {
    #[serde(rename = "TeamId", alias = "teamId")]
    pub team_id: Option<String>,
    #[serde(rename = "PendingAssignmentName", alias = "pendingAssignmentName")]
    pub pending_assignment_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoinFlipResult {
    #[serde(rename = "Kind", alias = "kind")]
    pub kind: String,
    #[serde(rename = "Team", alias = "team")]
    pub team: CoinFlipTeam,
}

/// One member of a team's roster as returned by the portal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RosterPlayer {
    pub number: Option<u8>,
    pub name: String,
    pub is_captain: bool,
    pub is_vice_captain: bool,
}

/// Parse the `roster` array of an `/api/admin/get-event-team` response.
///
/// Display name preference: non-empty `rosterName`, else `username`. Entries with
/// neither a name nor a cap number are skipped. Sorted ascending by cap number,
/// with unnumbered players last (then alphabetical).
fn parse_roster_json(body: &serde_json::Value) -> Vec<RosterPlayer> {
    let mut players = Vec::new();

    if let Some(roster) = body.get("roster").and_then(|v| v.as_array()) {
        for member in roster {
            let number = member
                .get("capNumber")
                .and_then(|v| v.as_u64())
                .map(|n| n as u8);
            let roster_name = member
                .get("rosterName")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let username = member
                .get("username")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let name = roster_name.or(username).unwrap_or("").to_string();

            let mut is_captain = false;
            let mut is_vice_captain = false;
            let mut is_player = false;
            if let Some(roles) = member.get("roles").and_then(|v| v.as_array()) {
                for role in roles {
                    match role.as_str() {
                        Some("Player") => is_player = true,
                        Some("Captain") => is_captain = true,
                        Some("ViceCaptain") => is_vice_captain = true,
                        _ => {}
                    }
                }
            }

            // Only playing members belong on a scoresheet — a team's roster also
            // carries `Manager`, `Coach` and `Official` entries, which would
            // otherwise take up numbered player rows. `Captain`/`ViceCaptain` are
            // modifiers layered on a base role rather than base roles themselves,
            // but they are accepted here as well so that a captain can never be
            // dropped from an official form if the portal omits their `Player` role.
            if !(is_player || is_captain || is_vice_captain) {
                continue;
            }

            if !name.is_empty() || number.is_some() {
                players.push(RosterPlayer {
                    number,
                    name,
                    is_captain,
                    is_vice_captain,
                });
            }
        }
    }

    players.sort_by(|a, b| match (a.number, b.number) {
        (Some(na), Some(nb)) => na.cmp(&nb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });

    players
}

#[derive(Debug, Clone, Serialize)]
pub struct SetCoinFlipModel {
    #[serde(rename = "GroupIdentifier")]
    pub group_identifier: Option<String>,
    #[serde(rename = "CoinFlipIdentifier")]
    pub coin_flip_identifier: String,
    #[serde(rename = "TeamIdOrPendingAssignmentName")]
    pub team_id_or_pending_assignment_name: String,
    #[serde(rename = "Kind")]
    pub kind: String,
}

pub struct UwhPortalClient {
    base_url: String,
    access_token: Option<String>,
    client: Client,
    id: OnceCell<u32>,
}

impl UwhPortalClient {
    pub fn new(
        base_url: &str,
        access_token: Option<&str>,
        require_https: bool,
        timeout: Duration,
    ) -> Result<Self, Box<dyn Error>> {
        let client = ClientBuilder::new()
            .https_only(require_https)
            .timeout(timeout)
            .build()?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self {
            base_url,
            access_token: access_token.map(|s| s.to_string()),
            client,
            id: OnceCell::new(),
        })
    }

    pub fn set_token(&mut self, token: &str) {
        self.access_token = Some(token.to_string());
    }

    pub fn clear_token(&mut self) {
        self.access_token = None;
    }

    pub fn has_token(&self) -> bool {
        self.access_token.is_some()
    }

    pub fn id(&self) -> u32 {
        *self
            .id
            .get_or_init(|| rand::rngs::StdRng::from_os_rng().random_range(1..=999_999))
    }

    /// Will generate a refbox id if it does not already exist.
    pub fn login_to_portal(
        &self,
        event_id: &EventId,
        code: u32,
    ) -> impl std::future::Future<Output = Result<PortalTokenResponse, Box<dyn Error>>> + use<>
    {
        let url = format!(
            "{}/api/events/{}/access-keys/ref-box",
            self.base_url,
            event_id.partial()
        );

        let request = self
            .client
            .request(Method::POST, &url)
            .json(&serde_json::json!({
                "refBoxId": self.id().to_string(),
                "code": code.to_string()
            }));

        async move {
            let response = request.send().await?;

            if response.status() == StatusCode::OK {
                info!("portal login successful");
                let body = response.json::<serde_json::Value>().await?;
                if let Some(token) = body["accessKey"].as_str() {
                    Ok(PortalTokenResponse::Success(token.to_string()))
                } else {
                    Err(Box::new(ApiError::new(
                        "Token not found in response".to_string(),
                    )))?
                }
            } else if response.status() == StatusCode::BAD_REQUEST {
                warn!("portal login failed, response: {:?}", response);
                let body = response.json::<serde_json::Value>().await?;
                if let Some(reason) = body["reason"].as_str() {
                    match reason {
                        "NoPendingLink" => Ok(PortalTokenResponse::NoPendingLink),
                        "InvalidCode" => Ok(PortalTokenResponse::InvalidCode),
                        _ => Err(Box::new(ApiError::new(format!(
                            "Unknown reason: {}",
                            reason
                        ))))?,
                    }
                } else {
                    Err(Box::new(ApiError::new(
                        "Reason not found in response".to_string(),
                    )))?
                }
            } else {
                warn!("portal login failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn login_with_email_and_password(
        &self,
        email: &str,
        password: &str,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn Error>>> + use<> {
        let url = format!("{}/api/authentication", self.base_url,);

        let request = self
            .client
            .request(Method::POST, &url)
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }));

        async move {
            let response = request.send().await?;

            if response.status() == StatusCode::OK {
                info!("portal login successful");
                let body = response.json::<serde_json::Value>().await?;
                if let Some(token) = body["accessToken"].as_str() {
                    Ok(token.to_string())
                } else {
                    Err(Box::new(ApiError::new(
                        "Token not found in response".to_string(),
                    )))?
                }
            } else {
                warn!("portal login failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn verify_token(
        &self,
        event: &EventId,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{}/access-keys/verify",
            self.base_url,
            event.partial()
        );
        let request = authenticated_request(&self.client, Method::GET, &url, &self.access_token);

        async move {
            let response = request.send().await?;

            if response.status() == StatusCode::OK {
                info!("portal token validation successful");
                Ok(())
            } else {
                warn!("portal token validation failed, response: {response:?}");
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn post_game_stats(
        &self,
        event_id: &EventId,
        game_number: &GameNumber,
        stats_json: String,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let url = format!("{}/api/admin/events/stats", self.base_url);

        let request = authenticated_request(&self.client, Method::POST, &url, &self.access_token)
            .query(&[("eventId", event_id.full()), ("gameNumber", game_number)])
            .body(stats_json.clone())
            .header("Content-Type", "application/json")
            .send();

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                info!("portal post game stats successful");
                Ok(())
            } else {
                warn!("portal post game stats failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn post_game_scores(
        &self,
        event_id: &EventId,
        game_number: &GameNumber,
        scores: BlackWhiteBundle<u8>,
        force: bool,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{}/schedule/games/{game_number}/scores",
            self.base_url,
            event_id.partial(),
        );

        let request = authenticated_request(&self.client, Method::POST, &url, &self.access_token)
            .query(&[("force", force)])
            .json(&serde_json::json!({
            "dark": {
                "value": scores.black
            },
            "light": {
                "value": scores.white
            }
            }));

        let client_ = self.client.clone();

        async move {
            let request = request.build()?;
            debug!("Posting game scores to portal: {request:?}");
            debug!(
                "Post body: {:?}",
                std::str::from_utf8(request.body().unwrap().as_bytes().unwrap())
            );
            let response = client_.execute(request).await?;

            if response.status() == StatusCode::OK {
                info!("portal post game scores successful");
                Ok(())
            } else {
                warn!("portal post game scores failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn get_event_schedule_privileged(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<schedule::Schedule, Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{}/schedule/privileged",
            self.base_url,
            event_id.partial()
        );

        let request =
            authenticated_request(&self.client, Method::GET, &url, &self.access_token).send();

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                let body = response.text().await?; // TODO: Can we just call response.json()?
                let schedule: schedule::Schedule = serde_json::from_str(&body)?;
                Ok(schedule)
            } else {
                warn!("portal get event schedule failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    /// Fetch the public `/referees` endpoint for an event and build a map from
    /// portal user ID to a display name.
    ///
    /// Response shape (per portal source, `EventRefereesController.cs`):
    /// ```json
    /// {
    ///   "tournamentReferee": { "user": { "id", "name", "username" }, "rosterName" },
    ///   "referees": {
    ///     "dedicated":         [ { "user": {...}, "rosterName" }, ... ],
    ///     "hybrid":            [ { "user": {...}, "rosterName" }, ... ],
    ///     "timeOrScoreKeeper": [ ... ]
    ///   }
    /// }
    /// ```
    ///
    /// `hybrid` is an array of referee entries; the portal's TypeScript type is
    /// `EventRefereeModelWithPhotos[] | null` (verified via uwh-portal source
    /// `js/@underwater-base/types/EventReferee.ts:155`).
    ///
    /// Display-name preference, per-entry: non-empty `rosterName`, else `user.username`.
    /// `user.name` is intentionally skipped — it is the user's full real name (PII),
    /// and a chosen handle is more appropriate for an operator UI.
    pub fn get_event_referee_name_map_from_referees(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<HashMap<String, String>, Box<dyn Error>>> + use<>
    {
        let url = format!(
            "{}/api/events/{}/referees",
            self.base_url,
            event_id.partial()
        );
        let request = self.client.get(&url).send();

        async move {
            let response = request.await?;
            if response.status() != StatusCode::OK {
                warn!("portal /referees failed, response: {:?}", response);
                let body = response.text().await?;
                return Err(Box::new(ApiError::new(body)).into());
            }
            let body = response.json::<serde_json::Value>().await?;
            let mut map = HashMap::new();

            // Collect every referee-like object into a flat list regardless of category.
            let mut all_items: Vec<&serde_json::Value> = Vec::new();
            if body["tournamentReferee"].is_object() {
                all_items.push(&body["tournamentReferee"]);
            }
            if let Some(cats) = body["referees"].as_object() {
                for (_cat, val) in cats {
                    if let Some(arr) = val.as_array() {
                        all_items.extend(arr.iter());
                    }
                }
            }

            for item in all_items {
                let uid = item["user"]["id"]
                    .as_str()
                    .or_else(|| item["userId"].as_str())
                    .or_else(|| item["id"].as_str());
                let name = item["rosterName"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .or_else(|| item["user"]["username"].as_str());
                if let (Some(uid), Some(name)) = (uid, name) {
                    map.insert(uid.to_string(), name.to_string());
                }
            }
            Ok(map)
        }
    }

    pub fn get_event_teams(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<TeamList, Box<dyn Error>>> + use<> {
        let url = format!("{}/api/events/{}/teams", self.base_url, event_id.partial());

        let request = self.client.get(&url).send();

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                let body = response.json::<serde_json::Value>().await?;
                let teams = body["teams"]
                    .as_array()
                    .ok_or(format!("Invalid response format. Response: {:?}", body))?;
                let mut team_map = BTreeMap::new();
                for team_entry in teams {
                    let team_info = &team_entry["team"];
                    let team_id = team_info["id"]
                        .as_str()
                        .ok_or(format!("Missing team id in response: {:?}", team_info))?;
                    let name = team_info["name"]
                        .as_str()
                        .ok_or(format!("Missing team name in response: {:?}", team_info))?;
                    team_map.insert(TeamId::from_full(team_id)?, name.to_string());
                }
                Ok(team_map)
            } else {
                warn!("portal get event schedule failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn get_event_list(
        &self,
        past: bool,
        schedule_published: bool,
    ) -> impl std::future::Future<Output = Result<Vec<schedule::Event>, Box<dyn Error>>> + use<>
    {
        let url = format!("{}/api/events", self.base_url);

        let filter = if past { "Past" } else { "InProgressOrUpcoming" };
        let schedule_published = if schedule_published { "true" } else { "false" };

        let request = self
            .client
            .get(&url)
            .query(&[
                ("limit", "100"),
                ("filter", filter),
                ("isSchedulePublished", schedule_published),
            ])
            .send();

        #[derive(Debug, Serialize, Deserialize)]
        struct ResponseWrapper {
            #[serde(rename = "totalCount")]
            total_count: u32,
            items: Vec<schedule::Event>,
        }

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                let body = response.text().await?;
                let parsed_response: ResponseWrapper = serde_json::from_str(&body)?;
                Ok(parsed_response.items)
            } else {
                warn!("portal get events list failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn push_event_schedule(
        &self,
        event_slug: &str,
        schedule: &schedule::SendableSchedule,
        force: bool,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let url = format!("{}/api/events/{event_slug}/schedule", self.base_url);

        let mut request =
            authenticated_request(&self.client, Method::POST, &url, &self.access_token)
                .json(schedule);

        if force {
            request = request.query(&[("force", "true")]);
        }

        async move {
            let response = request.send().await?;

            if response.status() == StatusCode::OK {
                info!("portal push event schedule successful");
                Ok(())
            } else {
                warn!(
                    "portal push event schedule failed, response: {:?}",
                    response
                );
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    /// The team map must map from unassigned name to full team id
    pub fn push_team_map(
        &self,
        event_slug: &str,
        team_map: &BTreeMap<&str, &str>,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{event_slug}/schedule/map-teams",
            self.base_url
        );

        let request = authenticated_request(&self.client, Method::POST, &url, &self.access_token)
            .json(&team_map);

        async move {
            let response = request.send().await?;

            if response.status() == StatusCode::OK {
                info!("portal push team map successful");
                Ok(())
            } else {
                warn!("portal push team map failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    // --- Scoresheet generation portal calls (schedule / roster / referees / coin-flip) ---

    /// Public (unauthenticated) event schedule. NOTE: for some events the public
    /// endpoint returns games as a JSON array rather than the object `GameList`
    /// expects; if this fails to parse for real data, use
    /// `get_event_schedule_privileged` instead.
    pub fn get_event_schedule_public(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<schedule::Schedule, Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{}/schedule",
            self.base_url,
            event_id.partial()
        );
        let request = self.client.get(&url).send();
        async move {
            let response = request.await?;
            if response.status() == StatusCode::OK {
                let body = response.text().await?;
                let schedule: schedule::Schedule = serde_json::from_str(&body)?;
                Ok(schedule)
            } else {
                warn!("uwhportal get public event schedule failed, response: {response:?}");
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn get_team_roster(
        &self,
        team_id: &TeamId,
    ) -> impl std::future::Future<Output = Result<Vec<RosterPlayer>, Box<dyn Error>>> + use<> {
        let url = format!("{}/api/admin/get-event-team", self.base_url);
        let team_id_full = team_id.full().to_string();
        let request = self
            .client
            .get(&url)
            .query(&[("teamId", &team_id_full)])
            .send();
        async move {
            let response = request.await?;
            if response.status() == StatusCode::OK {
                let body = response.json::<serde_json::Value>().await?;
                Ok(parse_roster_json(&body))
            } else {
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn get_coin_flips(
        &self,
        event_slug: &str,
    ) -> impl std::future::Future<Output = Result<CoinFlipDetails, Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{event_slug}/schedule/coin-flips",
            self.base_url
        );
        let request =
            authenticated_request(&self.client, Method::GET, &url, &self.access_token).send();
        async move {
            let response = request.await?;
            let status = response.status();
            let body = response.text().await?;
            if status == StatusCode::OK {
                match serde_json::from_str::<CoinFlipDetails>(&body) {
                    Ok(parsed) => Ok(parsed),
                    Err(e) => {
                        debug!("get_coin_flips: failed to decode body: {e}; body: {body}");
                        Err(Box::new(ApiError::new(format!(
                            "error decoding response body: {e}"
                        ))))?
                    }
                }
            } else {
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    /// Returns a map from user_id string to display name, populated from the
    /// authenticated `/participants` endpoint for the event.
    pub fn get_event_referee_name_map(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<HashMap<String, String>, Box<dyn Error>>> + use<>
    {
        let url = format!(
            "{}/api/events/{}/participants",
            self.base_url,
            event_id.partial()
        );
        let request =
            authenticated_request(&self.client, Method::GET, &url, &self.access_token).send();
        async move {
            let response = request.await?;
            if response.status() != StatusCode::OK {
                let body = response.text().await?;
                return Err(Box::new(ApiError::new(body)) as Box<dyn Error>);
            }
            let body = response.json::<serde_json::Value>().await?;
            let mut map = HashMap::new();
            let items = body
                .as_array()
                .cloned()
                .or_else(|| body["participants"].as_array().cloned())
                .or_else(|| body["items"].as_array().cloned())
                .unwrap_or_default();
            // Same nested-user structure as /referees
            for item in &items {
                let uid = item["user"]["id"]
                    .as_str()
                    .or_else(|| item["userId"].as_str())
                    .or_else(|| item["id"].as_str());
                let name = item["rosterName"]
                    .as_str()
                    .or_else(|| item["user"]["name"].as_str())
                    .or_else(|| item["user"]["username"].as_str());
                if let (Some(uid), Some(name)) = (uid, name) {
                    map.insert(uid.to_string(), name.to_string());
                }
            }
            Ok(map)
        }
    }

    /// Returns a map from user_id string to display name for all referees
    /// assigned to the given game (authenticated admin endpoint, AllowAnonymous).
    pub fn get_game_referee_name_map(
        &self,
        event_id: &EventId,
        game_number: &GameNumber,
    ) -> impl std::future::Future<Output = Result<HashMap<String, String>, Box<dyn Error>>> + use<>
    {
        let url = format!("{}/api/admin/events/game-referees", self.base_url);
        let event_id_full = event_id.full().to_string();
        let game_number = game_number.clone();
        let request = authenticated_request(&self.client, Method::GET, &url, &self.access_token)
            .query(&[("eventId", &event_id_full), ("gameNumber", &game_number)])
            .send();
        async move {
            let response = request.await?;
            if response.status() != StatusCode::OK {
                let body = response.text().await?;
                return Err(Box::new(ApiError::new(body)) as Box<dyn Error>);
            }
            let body = response.json::<serde_json::Value>().await?;
            let mut map = HashMap::new();
            // Response: { referees: [ { user: { id, name, username } } ] }
            // name may be null; username is the fallback.
            let items = body["referees"]
                .as_array()
                .cloned()
                .or_else(|| body.as_array().cloned())
                .unwrap_or_default();
            for item in &items {
                let uid = item["user"]["id"]
                    .as_str()
                    .or_else(|| item["userId"].as_str())
                    .or_else(|| item["id"].as_str());
                let name = item["user"]["name"]
                    .as_str()
                    .or_else(|| item["user"]["username"].as_str())
                    .or_else(|| item["rosterName"].as_str());
                if let (Some(uid), Some(name)) = (uid, name) {
                    map.insert(uid.to_string(), name.to_string());
                }
            }
            Ok(map)
        }
    }

    pub fn set_coin_flip_result(
        &self,
        event_slug: &str,
        model: &SetCoinFlipModel,
        force: bool,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let url = format!(
            "{}/api/events/{event_slug}/schedule/coin-flips",
            self.base_url
        );
        let request = authenticated_request(&self.client, Method::POST, &url, &self.access_token)
            .query(&[("force", force)])
            .json(model)
            .send();
        async move {
            let response = request.await?;
            if response.status() == StatusCode::OK {
                Ok(())
            } else {
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }
}

fn authenticated_request(
    client: &Client,
    method: Method,
    url: &str,
    access_token: &Option<String>,
) -> RequestBuilder {
    let mut request = client.request(method, url);
    if let Some(token) = access_token {
        request = request.header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
    }
    request
}

#[derive(Debug)]
struct ApiError {
    message: String,
}

impl ApiError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ApiError {}

/// A character an access key must not contain, because an HTTP header cannot
/// carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsendableAccessKey {
    /// The first character in the key that cannot be sent.
    pub character: char,
}

impl std::fmt::Display for UnsendableAccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the access key contains a character that cannot be sent to the site ({:?})",
            self.character
        )
    }
}

impl Error for UnsendableAccessKey {}

/// Check that `key` can be carried in an `Authorization` header.
///
/// Printable ASCII only. Everything a real access key contains — letters,
/// digits, and the punctuation used by base64 and JWTs — is in this range, and
/// everything outside it is what a header cannot carry: a newline, a tab, a
/// curly quote left by a chat app or a word processor.
///
/// Whitespace around the key is *not* trimmed here; callers that accept a
/// pasted key trim first, so that this reports only characters that are
/// genuinely part of the key.
pub fn check_access_key(key: &str) -> Result<(), UnsendableAccessKey> {
    match key.chars().find(|c| !matches!(c, ' '..='~')) {
        Some(character) => Err(UnsendableAccessKey { character }),
        None => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalTokenResponse {
    Success(String),
    NoPendingLink,
    InvalidCode,
}

#[cfg(test)]
mod coin_flip_tests {
    use super::*;

    #[test]
    fn coin_flip_details_deserializes_camel_case() {
        // The portal sends camelCase (aliases); PascalCase is also accepted.
        let json = r#"{
            "groups": [{
                "identifier": "g1",
                "name": "Group A",
                "shortName": "A",
                "coinFlips": [{
                    "identifier": "cf1",
                    "tiedTeams": [
                        {"teamId": "1-A", "pendingAssignmentName": null},
                        {"teamId": "2-A", "pendingAssignmentName": null}
                    ],
                    "result": {"kind": "White", "team": {"teamId": "1-A", "pendingAssignmentName": null}}
                }]
            }],
            "games": []
        }"#;
        let parsed: CoinFlipDetails = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.groups.len(), 1);
        let g = &parsed.groups[0];
        assert_eq!(g.identifier, "g1");
        assert_eq!(g.coin_flips.len(), 1);
        assert_eq!(g.coin_flips[0].tied_teams.len(), 2);
        assert_eq!(
            g.coin_flips[0].result.as_ref().unwrap().kind.as_str(),
            "White"
        );
    }

    #[test]
    fn set_coin_flip_model_serializes_pascal_case() {
        let model = SetCoinFlipModel {
            group_identifier: Some("g1".to_string()),
            coin_flip_identifier: "cf1".to_string(),
            team_id_or_pending_assignment_name: "1-A".to_string(),
            kind: "White".to_string(),
        };
        let v: serde_json::Value = serde_json::to_value(&model).unwrap();
        assert_eq!(v["GroupIdentifier"], "g1");
        assert_eq!(v["CoinFlipIdentifier"], "cf1");
        assert_eq!(v["TeamIdOrPendingAssignmentName"], "1-A");
        assert_eq!(v["Kind"], "White");
    }
}

#[cfg(test)]
mod roster_tests {
    use super::*;

    #[test]
    fn parses_numbers_names_and_both_captain_roles() {
        let body: serde_json::Value = serde_json::from_str(
            r#"{"roster":[
                {"capNumber":7,"rosterName":"Blake RIVE","roles":["Player","Captain"]},
                {"capNumber":8,"rosterName":"Keith LIN","roles":["Player","ViceCaptain"]},
                {"capNumber":1,"rosterName":"Drake QUIEC","roles":["Player"]}
            ]}"#,
        )
        .unwrap();

        let players = parse_roster_json(&body);

        assert_eq!(players.len(), 3, "all three members should be parsed");
        // sorted ascending by cap number
        assert_eq!(players[0].number, Some(1));
        assert_eq!(players[0].name, "Drake QUIEC");
        assert!(!players[0].is_captain && !players[0].is_vice_captain);

        assert_eq!(players[1].number, Some(7));
        assert!(players[1].is_captain, "Captain role must be surfaced");
        assert!(!players[1].is_vice_captain);

        assert_eq!(players[2].number, Some(8));
        assert!(
            players[2].is_vice_captain,
            "ViceCaptain role must be surfaced, not discarded"
        );
        assert!(!players[2].is_captain);
    }

    #[test]
    fn falls_back_to_username_and_sorts_unnumbered_last() {
        let body: serde_json::Value = serde_json::from_str(
            r#"{"roster":[
                {"rosterName":"  ","username":"zoe99","roles":["Player"]},
                {"capNumber":3,"rosterName":"Taylor COETZEE","roles":["Player"]}
            ]}"#,
        )
        .unwrap();

        let players = parse_roster_json(&body);

        assert_eq!(players.len(), 2);
        assert_eq!(players[0].number, Some(3));
        assert_eq!(
            players[1].name, "zoe99",
            "blank rosterName should fall back to username"
        );
        assert_eq!(players[1].number, None, "unnumbered players sort last");
    }

    #[test]
    fn skips_entries_with_neither_name_nor_number() {
        let body: serde_json::Value =
            serde_json::from_str(r#"{"roster":[{"rosterName":"","roles":["Player"]}]}"#).unwrap();
        assert!(parse_roster_json(&body).is_empty());
    }

    #[test]
    fn keeps_only_playing_members() {
        let body: serde_json::Value = serde_json::from_str(
            r#"{"roster":[
                {"capNumber":1,"rosterName":"Ana PLAYER","roles":["Player"]},
                {"capNumber":2,"rosterName":"Ben MANAGER","roles":["Manager"]},
                {"capNumber":3,"rosterName":"Cal COACH","roles":["Coach"]},
                {"capNumber":4,"rosterName":"Dee OFFICIAL","roles":["Official"]},
                {"capNumber":5,"rosterName":"Eve BOTH","roles":["Player","Manager"]},
                {"capNumber":6,"rosterName":"Fay SKIPPER","roles":["Captain"]},
                {"rosterName":"Gus NOROLE"}
            ]}"#,
        )
        .unwrap();

        let players = parse_roster_json(&body);
        let names: Vec<&str> = players.iter().map(|p| p.name.as_str()).collect();

        assert_eq!(
            names,
            vec!["Ana PLAYER", "Eve BOTH", "Fay SKIPPER"],
            "managers, coaches and officials must not take up player rows, but a \
             player-manager and a captain must be kept"
        );
    }
}

#[cfg(test)]
mod access_key_tests {
    use super::*;

    #[test]
    fn a_curly_quote_is_refused_and_named() {
        // The case this exists for: a key copied through a chat app or a word
        // processor, where a straight quote has been turned into a curly one.
        let err = check_access_key("abc\u{2019}123").unwrap_err();
        assert_eq!(err.character, '\u{2019}');
    }

    #[test]
    fn a_newline_or_tab_is_refused() {
        assert_eq!(check_access_key("abc\n123").unwrap_err().character, '\n');
        assert_eq!(check_access_key("abc\t123").unwrap_err().character, '\t');
    }

    #[test]
    fn a_normal_key_is_accepted() {
        // Letters, digits, and the punctuation base64 and JWTs use.
        let key = "eyJhbGciOiJI.UzI1NiIs-InR5cCI6_IkpXVCJ9~abc+/=";
        assert_eq!(check_access_key(key), Ok(()));
    }
}
