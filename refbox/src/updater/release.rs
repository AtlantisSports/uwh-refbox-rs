use super::UpdateError;
use super::version::Version;

pub const BIN_ASSET: &str = "refbox-aarch64-linux";
pub const SUM_ASSET: &str = "refbox-aarch64-linux.sha256";

/// Information extracted from a GitHub `/releases/latest` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: Version,
    pub binary_url: String,
    pub checksum_url: String,
}

impl ReleaseInfo {
    /// Parse a GitHub `/releases/latest` JSON body into a `ReleaseInfo`.
    ///
    /// Returns `Err(UpdateError::NotJson)` if the body is not valid JSON (e.g. a captive-portal
    /// HTML page). Returns `Err(UpdateError::BadVersion)` if `tag_name` is absent or
    /// unparseable. Returns `Err(UpdateError::AssetMissing)` if either the binary or checksum
    /// asset is not present in the `assets` array.
    pub fn from_json(body: &str) -> Result<ReleaseInfo, UpdateError> {
        let v: serde_json::Value = serde_json::from_str(body).map_err(|_| UpdateError::NotJson)?;

        let tag = v
            .get("tag_name")
            .and_then(|t| t.as_str())
            .ok_or(UpdateError::BadVersion)?;

        let version = Version::parse(tag).ok_or(UpdateError::BadVersion)?;

        let assets = v
            .get("assets")
            .and_then(|a| a.as_array())
            .ok_or(UpdateError::AssetMissing)?;

        let find_url = |name: &str| -> Option<String> {
            assets.iter().find_map(|a| {
                let asset_name = a.get("name")?.as_str()?;
                if asset_name == name {
                    a.get("browser_download_url")
                        .and_then(|u| u.as_str())
                        .map(str::to_owned)
                } else {
                    None
                }
            })
        };

        let binary_url = find_url(BIN_ASSET).ok_or(UpdateError::AssetMissing)?;
        let checksum_url = find_url(SUM_ASSET).ok_or(UpdateError::AssetMissing)?;

        Ok(ReleaseInfo {
            version,
            binary_url,
            checksum_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"tag_name":"v0.4.3","assets":[
      {"name":"refbox-aarch64-linux","browser_download_url":"https://x/bin"},
      {"name":"refbox-aarch64-linux.sha256","browser_download_url":"https://x/sum"}]}"#;

    #[test]
    fn parses_release() {
        let r = ReleaseInfo::from_json(SAMPLE).unwrap();
        assert_eq!(r.version, Version::parse("0.4.3").unwrap());
        assert_eq!(r.binary_url, "https://x/bin");
        assert_eq!(r.checksum_url, "https://x/sum");
    }

    #[test]
    fn rejects_non_json() {
        assert!(ReleaseInfo::from_json("<html>login</html>").is_err());
    }

    #[test]
    fn rejects_missing_asset() {
        assert!(ReleaseInfo::from_json(r#"{"tag_name":"v0.4.3","assets":[]}"#).is_err());
    }
}
