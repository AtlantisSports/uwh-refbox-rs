use core::time::Duration;
use log::{info, warn};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::{Client, ClientBuilder, Method, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub struct UwhPortalClient {
    base_url: String,
    username: Option<String>,
    password: Option<String>,
    access_token: Arc<Mutex<Option<String>>>,
    client: Client,
}

impl UwhPortalClient {
    pub fn new(
        base_url: &str,
        username: Option<&str>,
        password: Option<&str>,
        require_https: bool,
        timeout: Duration,
    ) -> Result<Self, Box<dyn Error>> {
        let client = ClientBuilder::new()
            .https_only(require_https)
            .timeout(timeout)
            .build()?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
            access_token: Arc::new(Mutex::new(None)),
            client,
        })
    }

    pub fn log_in(&self) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> {
        let url = format!("{}/api/authentication", self.base_url);
        let body = LoginRequest {
            email: self.username.clone().unwrap_or_default(),
            password: self.password.clone().unwrap_or_default(),
        };
        let access_token = self.access_token.clone();
        let request = self.client.post(url).json(&body).send();

        async move {
            let response = request.await?.error_for_status()?;
            let login_response: LoginResponse = response.json().await?;
            *access_token.lock().unwrap() = Some(login_response.access_token);
            Ok(())
        }
    }

    pub fn post_game_stats(
        &self,
        tid: u32,
        gid: u32,
        stats_json: String,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> {
        let url = format!(
            "{}/api/admin/events/stats?legacyEventId={}&gameNumber={}",
            self.base_url, tid, gid
        );

        let access_token = self.access_token.clone();
        let client = self.client.clone();
        let log_in_request = self.log_in();

        async move {
            let response = authenticated_request(&client, Method::POST, &url, &access_token)
                .body(stats_json.clone())
                .header("Content-Type", "application/json")
                .send()
                .await?;

            if response.status() == StatusCode::UNAUTHORIZED {
                info!("uwhportal access token invalid, logging in again");
                log_in_request.await?; // Retry the login request

                let retry_response =
                    authenticated_request(&client, Method::POST, &url, &access_token)
                        .body(stats_json)
                        .header("Content-Type", "application/json")
                        .send()
                        .await?;

                if retry_response.status() == StatusCode::OK {
                    Ok(())
                } else {
                    warn!("login retry failed, response: {:?}", retry_response);
                    let body = retry_response.text().await?;
                    Err(Box::new(ApiError::new(body)))?
                }
            } else if response.status() == StatusCode::OK {
                info!("uwhportal post game stats successful");
                Ok(())
            } else {
                warn!("uwhportal post game stats failed, response: {:?}", response);
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
    access_token: &Arc<Mutex<Option<String>>>,
) -> RequestBuilder {
    let mut request = client.request(method, url);
    let access_token = access_token.lock().unwrap().clone();
    if let Some(token) = access_token {
        request = request.header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
    }
    request
}

#[derive(Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
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
