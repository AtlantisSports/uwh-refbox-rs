//! Puts each game's YouTube link into the "watch" space of the portal's schedule, so viewers can
//! click from the portal straight to the game's video.
//!
//! The portal sets these through `POST /api/admin/update-games-watch-urls`, which needs a portal
//! account with the **admin** role. Portal sign-ins only last minutes, so the account's email and
//! password are kept (in `portal-login.json` next to the config) and used to sign in each time.
//! The password is encrypted with Windows' data protection for the current Windows user (via
//! PowerShell's `ConvertFrom-SecureString`), so only that user on that PC can read it back.

use crate::BoxError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const LOGIN_FILE: &str = "portal-login.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredLogin {
    email: String,
    /// Windows DPAPI-protected password, as produced by `ConvertFrom-SecureString`.
    protected_password: String,
}

pub fn login_path(config_dir: &Path) -> PathBuf {
    config_dir.join(LOGIN_FILE)
}

/// The email of the saved portal sign-in, if there is one.
pub fn saved_email(config_dir: &Path) -> Option<String> {
    let text = fs::read_to_string(login_path(config_dir)).ok()?;
    serde_json::from_str::<StoredLogin>(&text)
        .ok()
        .map(|l| l.email)
}

/// Sets or clears watch links if a portal sign-in is saved, reporting the result through `log`.
/// Never fails: the videos are already made, and a portal problem shouldn't hide that.
pub async fn sync(
    config_dir: &Path,
    portal_url: &str,
    event_slug: &str,
    links: &BTreeMap<String, Option<String>>,
    log: &mut (dyn FnMut(String) + Send),
) {
    if links.is_empty() {
        return;
    }
    if saved_email(config_dir).is_none() {
        log("Portal watch links: skipped (no portal sign-in saved in Settings)".to_string());
        return;
    }
    let count = links.len();
    let clearing = links.values().all(Option::is_none);
    match update_watch_links(config_dir, portal_url, event_slug, links).await {
        Ok(()) if clearing => log(format!("Portal: watch links cleared for {count} game(s)")),
        Ok(()) => log(format!("Portal: watch links set for {count} game(s)")),
        Err(e) => log(format!("⚠ Portal watch links not updated: {e}")),
    }
}

/// Each recorded video's portal watch link, by game number.
pub fn links_for(state: &crate::prepare::EventState) -> BTreeMap<String, Option<String>> {
    state
        .videos
        .iter()
        .map(|(game, video)| {
            (
                game.clone(),
                Some(format!("https://youtu.be/{}", video.broadcast_id)),
            )
        })
        .collect()
}

pub fn forget(config_dir: &Path) {
    let _ = fs::remove_file(login_path(config_dir));
}

/// Checks the sign-in works and has admin rights, then saves it (password encrypted).
pub async fn save(
    config_dir: &Path,
    portal_url: &str,
    email: &str,
    password: &str,
) -> Result<(), BoxError> {
    let token = sign_in(portal_url, email, password).await?;
    check_admin(portal_url, &token).await?;
    let login = StoredLogin {
        email: email.to_string(),
        protected_password: protect(password)?,
    };
    fs::create_dir_all(config_dir)?;
    fs::write(
        login_path(config_dir),
        serde_json::to_string_pretty(&login)?,
    )?;
    Ok(())
}

/// Sets (`Some(link)`) or clears (`None`) the watch link of each game, in one request.
pub async fn update_watch_links(
    config_dir: &Path,
    portal_url: &str,
    event_slug: &str,
    links: &BTreeMap<String, Option<String>>,
) -> Result<(), BoxError> {
    if links.is_empty() {
        return Ok(());
    }
    let text = fs::read_to_string(login_path(config_dir))
        .map_err(|_| "No portal sign-in saved (Settings → Portal sign-in)")?;
    let login: StoredLogin = serde_json::from_str(&text)?;
    let password = unprotect(&login.protected_password)?;
    let token = sign_in(portal_url, &login.email, &password).await?;

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/admin/update-games-watch-urls",
            portal_url.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_vec(&json!({
            "eventSlug": event_slug,
            "watchUrlsByGameNumber": links,
        }))?)
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    Err(match status.as_u16() {
        401 | 403 => "the portal refused: the saved account doesn't have admin rights".to_string(),
        404 => format!("the portal doesn't know the event \"{event_slug}\""),
        _ => format!("the portal answered {status}: {}", short(&body)),
    }
    .into())
}

fn short(text: &str) -> String {
    text.chars().take(300).collect()
}

async fn sign_in(portal_url: &str, email: &str, password: &str) -> Result<String, BoxError> {
    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/authentication",
            portal_url.trim_end_matches('/')
        ))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_vec(
            &json!({ "email": email, "password": password }),
        )?)
        .send()
        .await
        .map_err(|e| format!("couldn't reach the portal: {e}"))?;
    if !response.status().is_success() {
        return Err("the portal didn't accept that email and password".into());
    }
    let body: Value = serde_json::from_str(&response.text().await?)?;
    Ok(body["accessToken"]
        .as_str()
        .ok_or("the portal's sign-in reply had no access token")?
        .to_string())
}

/// Admin-only portal call that changes nothing, used to confirm the account has admin rights.
async fn check_admin(portal_url: &str, token: &str) -> Result<(), BoxError> {
    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/admin/events",
            portal_url.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .send()
        .await?;
    match response.status().as_u16() {
        200 => Ok(()),
        401 | 403 => Err(
            "this portal account doesn't have admin rights, which setting watch links needs".into(),
        ),
        other => Err(format!("the portal answered {other} when checking admin rights").into()),
    }
}

/// Runs a PowerShell script, feeding `input` on standard input (so secrets never appear on a
/// command line), and returns its output unchanged.
fn powershell(script: &str, input: &str) -> Result<String, BoxError> {
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("couldn't run PowerShell to protect the password: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "PowerShell failed: {}",
            short(&String::from_utf8_lossy(&output.stderr))
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Encrypts with Windows DPAPI for the current user.
fn protect(password: &str) -> Result<String, BoxError> {
    if !cfg!(windows) {
        return Err("saving the portal sign-in is only supported on Windows".into());
    }
    let protected = powershell(
        "$p = [Console]::In.ReadToEnd(); \
         ConvertTo-SecureString -String $p -AsPlainText -Force | ConvertFrom-SecureString",
        password,
    )?;
    Ok(protected.trim().to_string())
}

fn unprotect(protected: &str) -> Result<String, BoxError> {
    powershell(
        "$s = ConvertTo-SecureString -String ([Console]::In.ReadToEnd().Trim()); \
         $b = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($s); \
         try { [Console]::Out.Write([Runtime.InteropServices.Marshal]::PtrToStringBSTR($b)) } \
         finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($b) }",
        protected,
    )
    .map_err(|e| {
        format!(
            "couldn't read the saved portal password (saved by another Windows user or PC?): {e}"
        )
        .into()
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_protected_password_comes_back_unchanged_and_is_not_readable() {
        let secret = "correct horse; battery 'staple' $1";
        let protected = protect(secret).unwrap();
        assert!(!protected.contains("horse"));
        assert_eq!(unprotect(&protected).unwrap(), secret);
    }
}
