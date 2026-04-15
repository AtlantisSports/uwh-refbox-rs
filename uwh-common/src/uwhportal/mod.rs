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
                info!("uwhportal login successful");
                let body = response.json::<serde_json::Value>().await?;
                if let Some(token) = body["accessKey"].as_str() {
                    Ok(PortalTokenResponse::Success(token.to_string()))
                } else {
                    Err(Box::new(ApiError::new(
                        "Token not found in response".to_string(),
                    )))?
                }
            } else if response.status() == StatusCode::BAD_REQUEST {
                warn!("uwhportal login failed, response: {:?}", response);
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
                warn!("uwhportal login failed, response: {:?}", response);
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
                "emailOrUsername": email,
                "password": password
            }));

        async move {
            let response = request.send().await?;

            if response.status() == StatusCode::OK {
                info!("uwhportal login successful");
                let body = response.json::<serde_json::Value>().await?;
                if let Some(token) = body["accessToken"].as_str() {
                    Ok(token.to_string())
                } else {
                    Err(Box::new(ApiError::new(
                        "Token not found in response".to_string(),
                    )))?
                }
            } else {
                warn!("uwhportal login failed, response: {:?}", response);
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
                info!("uwhportal token validation successful");
                Ok(())
            } else {
                warn!("uwhportal token validation failed, response: {response:?}");
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
                info!("uwhportal post game stats successful");
                Ok(())
            } else {
                warn!("uwhportal post game stats failed, response: {:?}", response);
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
            debug!("Posting game scores to uwhportal: {request:?}");
            debug!(
                "Post body: {:?}",
                std::str::from_utf8(request.body().unwrap().as_bytes().unwrap())
            );
            let response = client_.execute(request).await?;

            if response.status() == StatusCode::OK {
                info!("uwhportal post game scores successful");
                Ok(())
            } else {
                warn!(
                    "uwhportal post game scores failed, response: {:?}",
                    response
                );
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

        let public_url = format!(
            "{}/api/events/{}/schedule",
            self.base_url,
            event_id.partial()
        );
        let client = self.client.clone();

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                let body = response.text().await?;
                let mut schedule: schedule::Schedule = serde_json::from_str(&body)?;

                // refereesByGameNumber is not present on the privileged endpoint;
                // fetch it from the public endpoint and merge in.
                if let Ok(pub_resp) = client.get(&public_url).send().await {
                    if pub_resp.status() == StatusCode::OK {
                        if let Ok(pub_body) = pub_resp.text().await {
                            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&pub_body) {
                                if let Some(refs) = raw.get("refereesByGameNumber") {
                                    if let Ok(parsed) =
                                        serde_json::from_value::<schedule::RefereesByGameNumber>(
                                            refs.clone(),
                                        )
                                    {
                                        schedule.referees_by_game_number = Some(parsed);
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(schedule)
            } else {
                warn!("uwhportal get event schedule failed, response: {response:?}");
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
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
                warn!(
                    "uwhportal get event schedule failed, response: {:?}",
                    response
                );
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
                warn!("uwhportal get events list failed, response: {:?}", response);
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
                info!("uwhportal push event schedule successful");
                Ok(())
            } else {
                warn!(
                    "uwhportal push event schedule failed, response: {:?}",
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
                info!("uwhportal push team map successful");
                Ok(())
            } else {
                warn!("uwhportal push team map failed, response: {:?}", response);
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    /// Attempts to parse the public schedule endpoint as a `Schedule`.
    ///
    /// Note: the public endpoint returns games as a JSON array, but `GameList`
    /// (`IndexMap<String, Game>`) requires an object keyed by game number.
    /// Deserialization will fail for real portal data. Use
    /// `get_event_schedule_privileged` with an access token for full schedule
    /// data, or `get_event_schedule_public_raw` to access the raw JSON.
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

    pub fn get_event_schedule_public_raw(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn Error>>> + use<> {
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
                Ok(body)
            } else {
                warn!("uwhportal get public event schedule failed, response: {response:?}");
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn get_team_roster(
        &self,
        team_id: &TeamId,
    ) -> impl std::future::Future<
        Output = Result<(Vec<(Option<u8>, String)>, Option<String>), Box<dyn Error>>,
    > + use<> {
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
                let mut players = Vec::new();
                let mut captain_name: Option<String> = None;

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

                        if let Some(roles) = member.get("roles").and_then(|v| v.as_array()) {
                            if roles.iter().any(|r| r.as_str() == Some("Captain")) {
                                captain_name = Some(name.clone());
                            }
                        }

                        if !name.is_empty() || number.is_some() {
                            players.push((number, name));
                        }
                    }
                }
                players.sort_by(|a, b| match (a.0, b.0) {
                    (Some(na), Some(nb)) => na.cmp(&nb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.1.cmp(&b.1),
                });
                Ok((players, captain_name))
            } else {
                let body = response.text().await?;
                Err(Box::new(ApiError::new(body)))?
            }
        }
    }

    pub fn get_event_schedule_privileged_raw(
        &self,
        event_id: &EventId,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn Error>>> + use<> {
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
                let body = response.text().await?;
                Ok(body)
            } else {
                warn!("uwhportal get privileged event schedule failed, response: {response:?}");
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

    /// Returns a map from user_id string to display name, populated from the public
    /// `/referees` endpoint for the event (no authentication required).
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
                let body = response.text().await?;
                return Err(Box::new(ApiError::new(body)) as Box<dyn Error>);
            }
            let body = response.json::<serde_json::Value>().await?;
            let mut map = HashMap::new();
            // Response: { tournamentReferee: {...}, referees: { dedicated: [...],
            //   hybrid: {...}, timeOrScoreKeeper: [...] } }
            // Collect all referee objects into a flat list regardless of category.
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
            // user ID at item["user"]["id"]; name: non-empty rosterName preferred,
            // then user.name, then user.username.
            for item in all_items {
                let uid = item["user"]["id"]
                    .as_str()
                    .or_else(|| item["userId"].as_str())
                    .or_else(|| item["id"].as_str());
                let name = item["rosterName"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .or_else(|| item["user"]["name"].as_str())
                    .or_else(|| item["user"]["username"].as_str());
                if let (Some(uid), Some(name)) = (uid, name) {
                    map.insert(uid.to_string(), name.to_string());
                }
            }
            Ok(map)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalTokenResponse {
    Success(String),
    NoPendingLink,
    InvalidCode,
}
