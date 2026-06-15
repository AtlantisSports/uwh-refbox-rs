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
    })?;
    // The downloaded artifact must be executable for the smoke test and swap.
    set_executable(dest).map_err(|e| UpdateError::Io(e.to_string()))?;
    Ok(())
}

/// Mark a freshly-downloaded file executable. On Unix this sets mode 0o755 so
/// the binary can be smoke-tested and exec'd after the swap; on other platforms
/// the executable bit is not used, so this is a no-op.
#[cfg(unix)]
fn set_executable(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn set_executable(_path: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn set_executable_sets_exec_bit() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        std::fs::write(&p, b"x").unwrap();
        // default is typically 0o644 — no exec bit
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o111,
            0
        );
        set_executable(&p).unwrap();
        assert_ne!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o111,
            0
        );
    }
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
