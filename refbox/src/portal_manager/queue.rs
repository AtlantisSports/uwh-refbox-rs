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
    /// Whether the portal has already accepted this game's **score**.
    /// `false` = score-pending (the normal queued state: auto-retried,
    /// can go stuck/red). `true` = stats-pending (score is up, only the
    /// stats upload is outstanding) — excluded from the auto-retry loop,
    /// the stuck escalation, and the yellow/red indicator; re-sent only
    /// by a one-shot `RetryStats` command. `#[serde(default)]` so old
    /// `portal_queue.json` files (written before this field existed)
    /// load as score-pending.
    #[serde(default)]
    pub score_sent: bool,
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

/// Atomically write a queue-file envelope to `target`: write to `tmp`,
/// fsync, then rename over the target.
fn write_atomic(target: &Path, tmp: &Path, q: &QueueFile) -> std::io::Result<()> {
    {
        let mut f = fs::File::create(tmp)?;
        serde_json::to_writer(&f, q).map_err(std::io::Error::other)?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(tmp, target)?;
    Ok(())
}

/// Atomically write the queue file to `dir/portal_queue.json`.
pub fn save(dir: &Path, q: &QueueFile) -> std::io::Result<()> {
    write_atomic(&queue_path(dir), &tmp_path(dir), q)
}

// --- Expired-item archive (Bug 2: portal_queue.expired.json) ---
//
// When a queued score passes the 120h expiry limit it is removed from the
// active queue, but copied here first so nothing is ever silently lost. This
// is a behind-the-scenes safety net, not surfaced in the UI.

const ARCHIVE_FILE_NAME: &str = "portal_queue.expired.json";
const ARCHIVE_TMP_FILE_NAME: &str = "portal_queue.expired.json.tmp";

fn archive_path(dir: &Path) -> PathBuf {
    dir.join(ARCHIVE_FILE_NAME)
}

fn archive_tmp_path(dir: &Path) -> PathBuf {
    dir.join(ARCHIVE_TMP_FILE_NAME)
}

/// Load the archive of expired queue items. Missing → empty. Unparseable or
/// unknown-version → logged and treated as empty (the archive is a
/// best-effort safety net, so a corrupt archive must never block a sweep).
pub fn load_archive_or_empty(dir: &Path) -> std::io::Result<QueueFile> {
    let path = archive_path(dir);
    if !path.exists() {
        return Ok(QueueFile::empty());
    }
    let bytes = fs::read(&path)?;
    match serde_json::from_slice::<QueueFile>(&bytes) {
        Ok(q) if q.version == QueueFile::CURRENT_VERSION => Ok(q),
        _ => {
            log::error!("portal_queue.expired.json unreadable; starting a fresh archive");
            Ok(QueueFile::empty())
        }
    }
}

/// Append expired items to the archive, atomically. No-op on empty input.
/// Additive: items archived by earlier sweeps are preserved.
pub fn append_to_archive(dir: &Path, items: &[QueuedItem]) -> std::io::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let mut archive = load_archive_or_empty(dir)?;
    archive.items.extend_from_slice(items);
    write_atomic(&archive_path(dir), &archive_tmp_path(dir), &archive)
}

/// A directory this session successfully read the portal queue from, and is
/// therefore allowed to write back to.
///
/// `open` is the only constructor, and it returns the loaded queue along with
/// the store — so a `QueueStore` cannot exist for a directory we could not
/// read. That is the entire safety property: a session with no readable queue
/// holds no store, and a store is the only route to a queue write, so it
/// cannot destroy a file it never saw. `save` renames over the target, and a
/// rename needs write permission on the *directory* rather than the file, so
/// an unreadable-but-replaceable queue is exactly the case this prevents.
///
/// See `docs/superpowers/specs/2026-08-13-degraded-no-write-target-design.md`.
#[derive(Debug)]
pub(super) struct QueueStore {
    dir: PathBuf,
}

impl QueueStore {
    /// Read `dir`'s queue and, on success, return the write target for it.
    /// A missing file is a successful read of an empty queue (a first run),
    /// and a corrupt one is rotated aside by `load_or_empty` and also
    /// succeeds. Only an I/O or permission failure yields `Err` — and
    /// therefore no write target at all.
    pub(super) fn open(dir: &Path) -> std::io::Result<(Self, QueueFile)> {
        let queue = load_or_empty(dir)?;
        Ok((
            Self {
                dir: dir.to_path_buf(),
            },
            queue,
        ))
    }

    pub(super) fn save(&self, q: &QueueFile) -> std::io::Result<()> {
        save(&self.dir, q)
    }

    pub(super) fn append_to_archive(&self, items: &[QueuedItem]) -> std::io::Result<()> {
        append_to_archive(&self.dir, items)
    }
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
                score_sent: false,
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

        fn one_item_queue(game: &str, black: u8, white: u8) -> QueueFile {
            QueueFile {
                version: 1,
                items: vec![QueuedItem {
                    id: ItemId {
                        event_id: "event".into(),
                        game_number: game.into(),
                    },
                    black_score: black,
                    white_score: white,
                    stats: "{}".into(),
                    queued_at: datetime!(2026-08-13 10:00 UTC),
                    attempts: 0,
                    last_attempt_at: None,
                    force: false,
                    score_sent: false,
                }],
            }
        }

        #[test]
        fn store_open_hands_back_the_queue_it_loaded() {
            let tmp = TempDir::new().unwrap();
            let q = one_item_queue("G1", 3, 2);
            save(tmp.path(), &q).unwrap();

            let (_store, loaded) = QueueStore::open(tmp.path()).unwrap();
            assert_eq!(
                loaded, q,
                "open must return the queue it read, not an empty one"
            );
        }

        #[test]
        fn store_open_on_a_missing_file_is_an_empty_queue() {
            let tmp = TempDir::new().unwrap();
            let (_store, loaded) = QueueStore::open(tmp.path()).unwrap();
            assert!(loaded.items.is_empty());
        }

        #[test]
        fn store_writes_where_it_read() {
            let tmp = TempDir::new().unwrap();
            let (store, _) = QueueStore::open(tmp.path()).unwrap();
            let q = one_item_queue("G2", 1, 0);

            store.save(&q).unwrap();
            assert_eq!(
                load_or_empty(tmp.path()).unwrap(),
                q,
                "a store must write back to the directory it was opened on"
            );
        }

        #[cfg(unix)]
        #[test]
        fn no_store_exists_for_a_queue_that_cannot_be_read() {
            // THE safety property: a directory whose queue file we cannot read
            // yields no write target at all, so nothing can overwrite it.
            use std::os::unix::fs::PermissionsExt;
            let tmp = TempDir::new().unwrap();
            save(tmp.path(), &QueueFile::empty()).unwrap();
            let path = queue_path(tmp.path());
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

            let result = QueueStore::open(tmp.path());

            // Restore before asserting so a failure cannot leave an unreadable
            // file behind for the rest of the suite.
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(
                result.is_err(),
                "an unreadable queue file must not produce a write target"
            );
        }

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
                    score_sent: false,
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

    #[test]
    fn score_sent_round_trips_true() {
        let item = QueuedItem {
            id: ItemId {
                event_id: "e1".into(),
                game_number: "G1".into(),
            },
            black_score: 1,
            white_score: 0,
            stats: "{}".into(),
            queued_at: datetime!(2026-06-25 12:00:00 UTC),
            attempts: 0,
            last_attempt_at: None,
            force: false,
            score_sent: true,
        };
        let s = serde_json::to_string(&item).unwrap();
        let back: QueuedItem = serde_json::from_str(&s).unwrap();
        assert!(back.score_sent);
        assert_eq!(item, back);
    }

    #[test]
    fn missing_score_sent_field_defaults_to_false() {
        // Simulate an old portal_queue.json written before this field existed.
        let item = QueuedItem {
            id: ItemId {
                event_id: "e1".into(),
                game_number: "G1".into(),
            },
            black_score: 0,
            white_score: 0,
            stats: "{}".into(),
            queued_at: datetime!(2026-06-25 12:00:00 UTC),
            attempts: 0,
            last_attempt_at: None,
            force: false,
            score_sent: true,
        };
        let mut v = serde_json::to_value(&item).unwrap();
        v.as_object_mut().unwrap().remove("score_sent");
        let back: QueuedItem = serde_json::from_value(v).unwrap();
        assert!(
            !back.score_sent,
            "an item with no score_sent field must load as score-pending (false)"
        );
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_item(game: &str) -> QueuedItem {
        QueuedItem {
            id: ItemId {
                event_id: "e1".into(),
                game_number: game.into(),
            },
            black_score: 1,
            white_score: 0,
            stats: "{}".into(),
            queued_at: OffsetDateTime::now_utc(),
            attempts: 0,
            last_attempt_at: None,
            force: false,
            score_sent: false,
        }
    }

    #[test]
    fn load_archive_missing_is_empty() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(
            load_archive_or_empty(tmp.path()).unwrap(),
            QueueFile::empty()
        );
    }

    #[test]
    fn append_then_load_round_trips() {
        let tmp = TempDir::new().unwrap();
        let item = sample_item("G1");
        append_to_archive(tmp.path(), std::slice::from_ref(&item)).unwrap();
        let back = load_archive_or_empty(tmp.path()).unwrap();
        assert_eq!(back.items, vec![item]);
    }

    #[test]
    fn append_accumulates_across_calls() {
        let tmp = TempDir::new().unwrap();
        let a = sample_item("G1");
        let b = sample_item("G2");
        append_to_archive(tmp.path(), std::slice::from_ref(&a)).unwrap();
        append_to_archive(tmp.path(), std::slice::from_ref(&b)).unwrap();
        let back = load_archive_or_empty(tmp.path()).unwrap();
        assert_eq!(back.items, vec![a, b]);
    }

    #[test]
    fn append_empty_writes_no_file() {
        let tmp = TempDir::new().unwrap();
        append_to_archive(tmp.path(), &[]).unwrap();
        assert!(!tmp.path().join("portal_queue.expired.json").exists());
    }
}
