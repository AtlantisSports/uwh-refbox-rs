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
use std::{collections::BTreeMap, error::Error};

pub mod schedule;

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

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                let body = response.text().await?; // TODO: Can we just call response.json()?
                let schedule: schedule::Schedule = serde_json::from_str(&body)?;
                Ok(schedule)
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

    pub fn get_event_schedule(
        &self,
        event_slug: &str,
    ) -> impl std::future::Future<Output = Result<TeamList, Box<dyn Error>>> + use<> {
        let url = format!("{}/api/events/{event_slug}/schedule", self.base_url);

        let request = self.client.get(&url).send();

        async move {
            let response = request.await?;

            if response.status() == StatusCode::OK {
                let body = response.json::<serde_json::Value>().await?;
                let teams = body["teams"].as_object().ok_or("Invalid response format")?;
                let mut team_map = BTreeMap::new();
                for (team_id, team_info) in teams {
                    if let Some(name) = team_info["name"].as_str() {
                        team_map.insert(TeamId::from_full(team_id)?, name.to_string());
                    }
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
    ) -> impl std::future::Future<Output = Result<Vec<schedule::Event>, Box<dyn Error>>> + use<>
    {
        let url = format!("{}/api/events", self.base_url);

        let filter = if past { "Past" } else { "InProgressOrUpcoming" };

        let request = self
            .client
            .get(&url)
            .query(&[
                ("limit", "100"),
                ("filter", filter),
                ("isSchedulePublished", "true"),
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

#[derive(Debug, Serialize, Deserialize)]
struct PortalToken {
    aud: String,
    entity: Option<String>,
    exp: u64,
    iat: u64,
    iss: String,
    jti: String,
    nbf: u64,
    scope: String,
    sub: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalTokenResponse {
    Success(String),
    NoPendingLink,
    InvalidCode,
}
