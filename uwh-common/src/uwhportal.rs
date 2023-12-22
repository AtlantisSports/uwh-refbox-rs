use core::time::Duration;
use log::{info, warn};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::{Client, ClientBuilder, Method, RequestBuilder, StatusCode};
use std::error::Error;

pub struct UwhPortalClient {
    base_url: String,
    access_token: Option<String>,
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
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            access_token: access_token.map(|s| s.to_string()),
            client,
        })
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

        let request = authenticated_request(&self.client, Method::POST, &url, &self.access_token)
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
