use core::time::Duration;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use log::{debug, info, warn};
use reqwest::header::{AUTHORIZATION, HeaderValue};
use reqwest::{Client, ClientBuilder, Method, RequestBuilder, StatusCode};
use schedule::{EventId, GameNumber, TeamId, TeamList};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use crate::bundles::BlackWhiteBundle;

pub mod schedule;

pub struct UwhPortalClient {
    base_url: String,
    access_token: Option<String>,
    token_validity: Arc<Mutex<TokenValidity>>,
    event: Option<String>,
    client: Client,
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

        let mut ret = Self {
            base_url,
            access_token: access_token.map(|s| s.to_string()),
            token_validity: Arc::new(Mutex::new(TokenValidity::Unknown)),
            event: None,
            client,
        };

        ret.check_token();

        Ok(ret)
    }

    /// Returns the current token validity state and the event name if
    /// for which the token is valid. If the event name is None, the token
    /// is valid for all events.
    pub fn token_validity(&self) -> (TokenValidity, Option<String>) {
        (*self.token_validity.lock().unwrap(), self.event.clone())
    }

    pub fn set_token(&mut self, token: &str) {
        self.access_token = Some(token.to_string());

        self.check_token();
    }

    fn check_token(&mut self) {
        if let Some(token) = &self.access_token {
            let mut issuer = self.base_url.clone();
            if issuer.starts_with("https://api.") {
                issuer = issuer.replacen("https://api.", "https://", 1);
            } else if issuer.starts_with("http://api.") {
                issuer = issuer.replacen("http://api.", "http://", 1);
            }

            let mut val = Validation::new(Algorithm::RS256);
            val.set_required_spec_claims(&["exp", "iss"]);
            val.set_audience(&["API"]);
            val.set_issuer(&[issuer]);
            val.reject_tokens_expiring_in_less_than = 60;
            val.validate_exp = true;
            val.validate_nbf = true;
            val.insecure_disable_signature_validation();

            // Garbage, but we need a key to compile
            let decoder = DecodingKey::from_secret(b"secret");
            let ret = decode::<PortalToken>(token, &decoder, &val);
            match ret {
                Ok(t) => {
                    self.event = t.claims.entity.map(|s| s.replacen("events/", "", 1));
                    *self.token_validity.lock().unwrap() = TokenValidity::LocallyChecked;
                }
                Err(e) => {
                    warn!("uwhportal token validation failed: {e:?}");
                    *self.token_validity.lock().unwrap() = TokenValidity::Invalid;
                }
            }
        } else {
            *self.token_validity.lock().unwrap() = TokenValidity::Invalid;
        }
    }

    /// Calling this with any token validity other than `LocallyChecked` is a no-op.
    pub fn verify_token(
        &self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + use<> {
        let request = self.event.as_ref().map(|e| {
            let url = format!("{}/api/admin/events/{e}/access-keys/verify", self.base_url);
            authenticated_request(&self.client, Method::GET, &url, &self.access_token)
        });

        let token_validity = self.token_validity.clone();
        async move {
            if *token_validity.lock().unwrap() != TokenValidity::LocallyChecked {
                info!("uwhportal token validation skipped");
                return Ok(());
            }

            let response = if let Some(request) = request {
                request.send().await?
            } else {
                return Ok(());
            };

            if response.status() == StatusCode::OK {
                info!("uwhportal token validation successful");
                *token_validity.lock().unwrap() = TokenValidity::Valid;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenValidity {
    Invalid,
    LocallyChecked,
    Valid,
    Unknown,
}
