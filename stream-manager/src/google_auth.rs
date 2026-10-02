//! One-time Google sign-in for a desktop app, and keeping the access token fresh afterwards.
//!
//! `connect` opens Google's own sign-in page in the browser; the password is only ever typed
//! there. Google then redirects the browser to a temporary local address on this computer,
//! where we pick up a one-time code and swap it for a long-lived "refresh token". That token is
//! stored next to the config and used to get short-lived access tokens when needed.

use crate::BoxError;
use log::info;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::RandomState,
    fs,
    hash::{BuildHasher, Hasher},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// Lets the app manage the channel's live videos, playlists and live chat.
const SCOPE: &str = "https://www.googleapis.com/auth/youtube";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Deserialize)]
struct ClientSecretFile {
    installed: ClientSecret,
}

#[derive(Debug, Clone, Deserialize)]
struct ClientSecret {
    client_id: String,
    client_secret: String,
    auth_uri: String,
    token_uri: String,
}

impl ClientSecret {
    fn load(path: &Path) -> Result<Self, BoxError> {
        let text = fs::read_to_string(path).map_err(|e| {
            format!(
                "Couldn't read the Google sign-in file at {}: {e}",
                path.display()
            )
        })?;
        let file: ClientSecretFile = serde_json::from_str(&text).map_err(|e| {
            format!(
                "{} doesn't look like a 'Desktop app' OAuth client file: {e}",
                path.display()
            )
        })?;
        Ok(file.installed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredToken {
    refresh_token: String,
    access_token: String,
    /// Unix time (seconds) when `access_token` stops working.
    expires_at: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// An unguessable value so we only accept the sign-in we started ourselves.
fn random_state() -> String {
    let a = RandomState::new().build_hasher().finish();
    let b = RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    if result.is_err() {
        println!("Couldn't open a browser automatically; please open the link above yourself.");
    }
}

async fn post_token_request(
    http: &reqwest::Client,
    token_uri: &str,
    params: &[(&str, &str)],
) -> Result<TokenResponse, BoxError> {
    let response = http.post(token_uri).form(params).send().await?;
    let status = response.status();
    let body = response.text().await?;
    if status.is_success() {
        Ok(serde_json::from_str(&body)?)
    } else {
        let detail = serde_json::from_str::<TokenError>(&body)
            .map(|e| format!("{} {}", e.error, e.error_description.unwrap_or_default()))
            .unwrap_or(body);
        Err(format!("Google refused the token request ({status}): {detail}").into())
    }
}

/// A sign-in that has been started: the browser should open `url`, then `finish` waits for it.
pub struct PendingSignIn {
    pub url: String,
    client: ClientSecret,
    listener: TcpListener,
    redirect_uri: String,
    state: String,
}

/// Starts a sign-in. Google sends the browser back to a temporary address on *this* computer,
/// so the sign-in page must be opened on this computer too.
pub async fn begin_sign_in(client_file: &Path) -> Result<PendingSignIn, BoxError> {
    let client = ClientSecret::load(client_file)?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let state = random_state();

    let mut url = Url::parse(&client.auth_uri)?;
    url.query_pairs_mut()
        .append_pair("client_id", &client.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &state);
    Ok(PendingSignIn {
        url: url.to_string(),
        client,
        listener,
        redirect_uri,
        state,
    })
}

impl PendingSignIn {
    /// Waits (up to 5 minutes) for the browser to come back, then stores the token.
    pub async fn finish(self, token_file: &Path) -> Result<(), BoxError> {
        let code =
            tokio::time::timeout(SIGN_IN_TIMEOUT, wait_for_code(&self.listener, &self.state))
                .await
                .map_err(|_| "Sign-in wasn't finished within 5 minutes; please try again")??;

        let http = reqwest::Client::new();
        let token = post_token_request(
            &http,
            &self.client.token_uri,
            &[
                ("code", code.as_str()),
                ("client_id", &self.client.client_id),
                ("client_secret", &self.client.client_secret),
                ("redirect_uri", &self.redirect_uri),
                ("grant_type", "authorization_code"),
            ],
        )
        .await?;
        let refresh_token = token
            .refresh_token
            .ok_or("Google didn't return a long-lived token; please try again")?;
        save_token(
            token_file,
            &StoredToken {
                refresh_token,
                access_token: token.access_token,
                expires_at: now_secs() + token.expires_in,
            },
        )?;
        info!("Signed in. Token saved to {}", token_file.display());
        Ok(())
    }
}

/// Runs the browser sign-in from the terminal and stores the resulting token in `token_file`.
pub async fn connect(client_file: &Path, token_file: &Path) -> Result<(), BoxError> {
    let pending = begin_sign_in(client_file).await?;
    println!(
        "Opening Google sign-in in your browser. If nothing opens, use this link:

{}
",
        pending.url
    );
    open_browser(&pending.url);
    pending.finish(token_file).await
}

pub fn is_connected(token_file: &Path) -> bool {
    token_file.exists()
}

/// Waits for the browser to come back with `?code=...&state=...` and answers it with a small page.
async fn wait_for_code(listener: &TcpListener, expected_state: &str) -> Result<String, BoxError> {
    loop {
        let (mut socket, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let mut len = 0;
        // Read until the end of the request headers; the code is in the first line.
        while len < buf.len() {
            let n = socket.read(&mut buf[len..]).await?;
            if n == 0 {
                break;
            }
            len += n;
            if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&buf[..len]);
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");
        let url = Url::parse(&format!("http://localhost{path}"))?;
        let param = |name: &str| {
            url.query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.into_owned())
        };

        let (result, message) = match (param("code"), param("state"), param("error")) {
            (_, _, Some(error)) => (
                Some(Err(format!("Sign-in was cancelled or refused: {error}"))),
                "Sign-in was cancelled. You can close this tab and try again.",
            ),
            (Some(code), Some(state), None) if state == expected_state => (
                Some(Ok(code)),
                "Stream Manager is now connected to YouTube. You can close this tab.",
            ),
            (Some(_), _, None) => (
                Some(Err(
                    "Sign-in response didn't match; run `connect` again".to_string()
                )),
                "Something didn't match. Please run connect again.",
            ),
            // Other requests, e.g. the browser asking for a favicon.
            (None, _, None) => (None, ""),
        };
        let response = if result.is_some() {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
                 <html><body style=\"font-family:sans-serif\"><h2>{message}</h2></body></html>"
            )
        } else {
            "HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n".to_string()
        };
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
        if let Some(result) = result {
            return result.map_err(Into::into);
        }
    }
}

fn save_token(path: &Path, token: &StoredToken) -> Result<(), BoxError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string_pretty(token)?)?;
    Ok(())
}

/// Hands out valid access tokens, refreshing them from the stored refresh token as needed.
pub struct GoogleAuth {
    client: ClientSecret,
    token: StoredToken,
    token_file: PathBuf,
    http: reqwest::Client,
}

impl GoogleAuth {
    pub fn load(client_file: &Path, token_file: &Path) -> Result<Self, BoxError> {
        let client = ClientSecret::load(client_file)?;
        let text = fs::read_to_string(token_file)
            .map_err(|_| "Not connected to YouTube yet. Run `stream-manager connect` first.")?;
        let token: StoredToken = serde_json::from_str(&text)?;
        Ok(Self {
            client,
            token,
            token_file: token_file.to_path_buf(),
            http: reqwest::Client::new(),
        })
    }

    pub async fn access_token(&mut self) -> Result<String, BoxError> {
        if now_secs() + 60 >= self.token.expires_at {
            let refreshed = post_token_request(
                &self.http,
                &self.client.token_uri,
                &[
                    ("refresh_token", self.token.refresh_token.as_str()),
                    ("client_id", &self.client.client_id),
                    ("client_secret", &self.client.client_secret),
                    ("grant_type", "refresh_token"),
                ],
            )
            .await
            .map_err(|e| {
                format!("{e}\nThe YouTube connection may have been removed; run `connect` again.")
            })?;
            self.token.access_token = refreshed.access_token;
            self.token.expires_at = now_secs() + refreshed.expires_in;
            if let Some(new_refresh) = refreshed.refresh_token {
                self.token.refresh_token = new_refresh;
            }
            save_token(&self.token_file, &self.token)?;
        }
        Ok(self.token.access_token.clone())
    }
}
