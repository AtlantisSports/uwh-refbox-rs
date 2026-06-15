use super::{UpdateError, release::ReleaseInfo};

const REPO: &str = "AtlantisSports/uwh-refbox-rs";
const UA: &str = concat!("uwh-refbox-rs/", env!("CARGO_PKG_VERSION"));

/// Query GitHub's latest published release (drafts/pre-releases excluded by the API).
pub async fn check_latest() -> Result<ReleaseInfo, UpdateError> {
    let client = reqwest::ClientBuilder::new()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(UA)
        .build()
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| UpdateError::Network)?;
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(UpdateError::RateLimited);
    }
    if !resp.status().is_success() {
        return Err(UpdateError::Network);
    }
    let body = resp.text().await.map_err(|_| UpdateError::Network)?;
    // Maps non-JSON (e.g. captive-portal HTML page) → NotJson
    ReleaseInfo::from_json(&body)
}

/// Download the bytes at `url` and write them to `dest`.
///
/// Pre-flight: verifies the destination's parent directory is writable by creating and
/// immediately removing a small probe file. On failure, returns `UpdateError::NotWritable`.
///
/// Write-failure mapping:
/// - ENOSPC (errno 28) → `UpdateError::NoSpace`
/// - `PermissionDenied` → `UpdateError::NotWritable`
/// - anything else → `UpdateError::Io`
pub async fn download_to(url: &str, dest: &std::path::Path) -> Result<(), UpdateError> {
    // Pre-flight: check the destination directory is writable.
    let probe = dest.with_extension("probe");
    std::fs::write(&probe, b"").map_err(|_| UpdateError::NotWritable)?;
    // Best-effort removal of the probe; ignore errors (we've already proved writability).
    let _ = std::fs::remove_file(&probe);

    // Fresh client, no auth header — safe across the signed-redirect to the asset CDN.
    let client = reqwest::ClientBuilder::new()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(180))
        .user_agent(UA)
        .build()
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| UpdateError::Network)?;
    if !resp.status().is_success() {
        return Err(UpdateError::Network);
    }
    let bytes = resp.bytes().await.map_err(|_| UpdateError::Network)?;
    std::fs::write(dest, &bytes).map_err(|e| {
        if e.raw_os_error() == Some(28) {
            UpdateError::NoSpace
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            UpdateError::NotWritable
        } else {
            UpdateError::Io(e.to_string())
        }
    })
}

/// Fetch the text body of `url` (used to retrieve checksum files).
pub async fn fetch_text(url: &str) -> Result<String, UpdateError> {
    // Fresh client, no auth header — safe across the signed-redirect to the asset CDN.
    let client = reqwest::ClientBuilder::new()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(UA)
        .build()
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| UpdateError::Network)?;
    if !resp.status().is_success() {
        return Err(UpdateError::Network);
    }
    resp.text().await.map_err(|_| UpdateError::Network)
}
