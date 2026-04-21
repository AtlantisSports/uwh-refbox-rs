//! On-disk persistence for the portal retry queue.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::macros::format_description;

use super::ItemId;

/// Top-level envelope for `portal_queue.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueFile {
    pub version: u32,
    pub items: Vec<QueuedItem>,
}

impl QueueFile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn empty() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            items: Vec::new(),
        }
    }
}

/// Per-game submission record persisted on disk.
///
/// All queued items are implicitly "pending" — i.e. awaiting retry.
/// There is no per-item state enum because the portal client cannot
/// distinguish 409 Conflict, 401 Unauthorised, 5xx or network failure
/// from each other (see the amendment in ADR 011). Stuck-ness is
/// derived from `queued_at` (see `is_item_stuck` in Task 8), and
/// token problems are tracked globally on the `PortalManager` via a
/// separate `verify_token` probe.
///
/// Datetime fields use `time::OffsetDateTime` with serde's
/// RFC 3339 representation (the `time` crate's `serde-human-readable`
/// feature is already enabled workspace-wide).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedItem {
    #[serde(flatten)]
    pub id: ItemId,
    pub black_score: u8,
    pub white_score: u8,
    pub stats: String,
    #[serde(with = "time::serde::rfc3339")]
    pub queued_at: OffsetDateTime,
    pub attempts: u32,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_attempt_at: Option<OffsetDateTime>,
    /// When true, the next submit sends `force=true` so the portal
    /// overwrites any existing server-side value. Set by the operator
    /// via the FORCE THIS GAME RESULT button on the attention action
    /// page (see Task 15).
    pub force: bool,
}

const QUEUE_FILE_NAME: &str = "portal_queue.json";
const TMP_FILE_NAME: &str = "portal_queue.json.tmp";

fn queue_path(dir: &Path) -> PathBuf {
    dir.join(QUEUE_FILE_NAME)
}

fn tmp_path(dir: &Path) -> PathBuf {
    dir.join(TMP_FILE_NAME)
}

/// Load the queue file from `dir`. If missing, return an empty queue. If
/// present but unparseable, rename to `portal_queue.corrupt.<ts>.json`,
/// log an error, and return an empty queue.
pub fn load_or_empty(dir: &Path) -> std::io::Result<QueueFile> {
    let path = queue_path(dir);
    if !path.exists() {
        return Ok(QueueFile::empty());
    }
    let bytes = fs::read(&path)?;
    match serde_json::from_slice::<QueueFile>(&bytes) {
        Ok(q) if q.version == QueueFile::CURRENT_VERSION => Ok(q),
        Ok(q) => {
            log::error!(
                "portal_queue.json has unknown version {}; renaming and starting fresh",
                q.version
            );
            rename_corrupt(&path)?;
            Ok(QueueFile::empty())
        }
        Err(e) => {
            log::error!("portal_queue.json failed to parse ({e}); renaming and starting fresh");
            rename_corrupt(&path)?;
            Ok(QueueFile::empty())
        }
    }
}

fn rename_corrupt(path: &Path) -> std::io::Result<()> {
    // Format: YYYYMMDDTHHMMSSZ, e.g. "20260419T142203Z".
    let fmt = format_description!("[year][month][day]T[hour][minute][second]Z");
    let ts = OffsetDateTime::now_utc()
        .format(&fmt)
        .unwrap_or_else(|_| "unknown-time".to_string());
    let mut new_path = path.to_path_buf();
    new_path.set_file_name(format!("portal_queue.corrupt.{ts}.json"));
    fs::rename(path, &new_path)
}

/// Atomically write the queue file to `dir/portal_queue.json`.
/// Writes to a temp file, fsyncs, then renames over the target.
pub fn save(dir: &Path, q: &QueueFile) -> std::io::Result<()> {
    let tmp = tmp_path(dir);
    {
        let mut f = fs::File::create(&tmp)?;
        serde_json::to_writer(&f, q).map_err(std::io::Error::other)?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, queue_path(dir))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn round_trips_empty_queue() {
        let q = QueueFile::empty();
        let s = serde_json::to_string(&q).unwrap();
        let back: QueueFile = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
        assert_eq!(back.version, 1);
        assert!(back.items.is_empty());
    }

    #[test]
    fn round_trips_queue_with_items() {
        let q = QueueFile {
            version: 1,
            items: vec![QueuedItem {
                id: ItemId {
                    event_id: "2026-spring".into(),
                    game_number: "G27".into(),
                },
                black_score: 3,
                white_score: 2,
                stats: "{\"stub\":true}".into(),
                queued_at: datetime!(2026-04-19 14:22:03 UTC),
                attempts: 2,
                last_attempt_at: Some(datetime!(2026-04-19 14:23:15 UTC)),
                force: false,
            }],
        };
        let s = serde_json::to_string_pretty(&q).unwrap();
        let back: QueueFile = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
    }

    #[cfg(test)]
    mod load_save_tests {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn loads_empty_when_file_missing() {
            let tmp = TempDir::new().unwrap();
            let q = load_or_empty(tmp.path()).unwrap();
            assert_eq!(q, QueueFile::empty());
        }

        #[test]
        fn save_then_load_round_trip() {
            let tmp = TempDir::new().unwrap();
            let q = QueueFile {
                version: 1,
                items: vec![QueuedItem {
                    id: ItemId {
                        event_id: "e1".into(),
                        game_number: "G1".into(),
                    },
                    black_score: 0,
                    white_score: 0,
                    stats: "{}".into(),
                    queued_at: OffsetDateTime::now_utc(),
                    attempts: 0,
                    last_attempt_at: None,
                    force: false,
                }],
            };
            save(tmp.path(), &q).unwrap();
            let back = load_or_empty(tmp.path()).unwrap();
            assert_eq!(back, q);
        }

        #[test]
        fn corrupted_file_is_renamed_and_empty_returned() {
            let tmp = TempDir::new().unwrap();
            let queue_path = tmp.path().join("portal_queue.json");
            std::fs::write(&queue_path, b"this is not json").unwrap();

            let q = load_or_empty(tmp.path()).unwrap();
            assert_eq!(q, QueueFile::empty());

            // Original file should have been renamed.
            assert!(!queue_path.exists());
            let entries: Vec<_> = std::fs::read_dir(tmp.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            assert!(
                entries
                    .iter()
                    .any(|n| n.starts_with("portal_queue.corrupt")),
                "expected a corrupt backup; got {entries:?}"
            );
        }

        #[test]
        fn atomic_write_leaves_no_tmp_file_on_success() {
            let tmp = TempDir::new().unwrap();
            save(tmp.path(), &QueueFile::empty()).unwrap();
            assert!(tmp.path().join("portal_queue.json").exists());
            assert!(!tmp.path().join("portal_queue.json.tmp").exists());
        }
    }
}
