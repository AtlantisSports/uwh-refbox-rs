pub mod release;
pub mod swap;
pub mod verify;
pub mod version;

/// Errors that can occur during the self-update process.
///
/// Each variant maps to a distinct UI error state shown to the operator;
/// see the UI mapping in the updater task (Task 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    /// A network request failed (connection error, timeout, etc.).
    Network,
    /// GitHub responded with HTTP 429 (rate-limited).
    RateLimited,
    /// The server response was not valid JSON (e.g. a captive-portal HTML page).
    NotJson,
    /// A required release asset was not found in the GitHub response.
    AssetMissing,
    /// The `tag_name` field was absent or could not be parsed as a version number.
    BadVersion,
    /// The downloaded file's checksum did not match the expected value.
    Checksum,
    /// The filesystem does not have enough free space for the update.
    NoSpace,
    /// The binary path is not writable by the current process.
    NotWritable,
    /// An I/O error occurred; the inner string contains a human-readable description.
    Io(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Network => write!(f, "network error"),
            UpdateError::RateLimited => write!(f, "rate limited by GitHub"),
            UpdateError::NotJson => write!(f, "server response was not JSON"),
            UpdateError::AssetMissing => write!(f, "required release asset not found"),
            UpdateError::BadVersion => write!(f, "could not parse release version"),
            UpdateError::Checksum => write!(f, "checksum mismatch"),
            UpdateError::NoSpace => write!(f, "not enough disk space"),
            UpdateError::NotWritable => write!(f, "binary path is not writable"),
            UpdateError::Io(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}
