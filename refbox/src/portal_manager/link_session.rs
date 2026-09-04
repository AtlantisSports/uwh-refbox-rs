//! On-disk persistence for the active portal link ("link session").
//!
//! Records which event/court/game this machine is linked to plus a
//! "last active" timestamp, so a relaunch (language change, self-update)
//! or a short shutdown can re-establish the link instead of starting
//! dormant. See
//! `docs/superpowers/specs/2026-06-22-portal-link-restore-across-restart-design.md`.
//!
//! Mirrors the atomic-write + tolerant-load pattern of `queue.rs`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::macros::format_description;

use crate::config::Mode;
use uwh_common::uwhportal::schedule::{EventId, GameNumber};

/// How recent the last session must be to auto-restore the link on startup,
/// *when the boot clock is trustworthy*. A Raspberry Pi has no battery-backed
/// clock and may boot with a time that has not yet been corrected by the
/// network — `decide_restore` handles that clock-suspect case separately so a
/// wrong boot clock never discards a recent link.
pub const FRESHNESS_WINDOW: time::Duration = time::Duration::hours(120);

const FILE_NAME: &str = "portal_link.json";
const TMP_FILE_NAME: &str = "portal_link.json.tmp";

/// Which site a note belongs to.
///
/// An event id cannot answer this: ids collide between the Portal and a custom
/// site by design, so a note carrying only an event id could be restored against
/// whichever site happened to be configured — handing the operator another
/// server's court and game. That is why a custom session's note used to be
/// written and then ignored at startup.
///
/// `Portal` does not say *which* portal: the existing `mode` field already
/// separates UWH from UWR, and `decide_restore` refuses a note that crosses
/// between them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteSite {
    #[default]
    Portal,
    /// A third-party site, identified by the address it was configured with.
    ///
    /// The whole address, not just the host: a custom address includes the event,
    /// so comparing it asks "the same site *and* the same event?" in one go, and
    /// editing only the event in the URL correctly invalidates the note.
    Custom { address: String },
}

/// The remembered live portal link, persisted next to `portal_queue.json`.
///
/// v2 records a **fact** (which game was last played) rather than a conclusion
/// ("this court is finished"). A fact stays true across restarts and refreshes;
/// a conclusion goes stale, which is what replayed a finished court.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSessionFile {
    pub version: u32,
    pub event_id: EventId,
    pub court: Option<String>,
    /// The game the operator is on right now — in progress, or the confirmed
    /// upcoming game. **Absent whenever nothing is next**, which is what makes an
    /// offline restart show the finished state instead of a phantom game.
    /// Read from v1 notes under its old name.
    #[serde(alias = "game_number")]
    pub current_game: Option<GameNumber>,
    /// The game most recently played to a recorded result on this court: the
    /// anchor the schedule search starts from. Absent when the refbox holds no
    /// history for this court, which always requires an operator pick.
    #[serde(default)]
    pub last_played: Option<GameNumber>,
    /// The anchor's scheduled start. Persisted rather than looked up so the
    /// search still works when the anchor game itself has been removed from the
    /// schedule.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_played_start: Option<OffsetDateTime>,
    pub mode: Mode,
    /// Which site this note was written against.
    ///
    /// Defaults to `Portal`, which is exactly what a note written before this
    /// field existed meant: startup restored those only for the Portal and
    /// ignored them outright for a custom site. Deliberately additive rather
    /// than a version bump — the default reproduces the old behaviour exactly,
    /// and holding the version lets a rolled-back binary still read the note
    /// instead of quarantining it and losing the link mid-tournament.
    #[serde(default)]
    pub site: NoteSite,
    #[serde(with = "time::serde::rfc3339")]
    pub last_active: OffsetDateTime,
}

impl LinkSessionFile {
    pub const CURRENT_VERSION: u32 = 2;
}

fn file_path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

fn tmp_path(dir: &Path) -> PathBuf {
    dir.join(TMP_FILE_NAME)
}

/// Load the note. Missing → `None`. Present but unparseable or of an
/// unknown version → rename to `portal_link.corrupt.<ts>.json`, log, and
/// return `None`. Never blocks startup.
pub fn load_or_none(dir: &Path) -> std::io::Result<Option<LinkSessionFile>> {
    let path = file_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    match serde_json::from_slice::<LinkSessionFile>(&bytes) {
        // v1 notes migrate in place: `game_number` reads as `current_game` via
        // the serde alias, and the anchor fields default to absent. Forcing a
        // re-pick on upgrade day, mid-tournament, would be worse than the
        // one-day gap in history that this leaves.
        Ok(note) if note.version <= LinkSessionFile::CURRENT_VERSION => Ok(Some(note)),
        Ok(note) => {
            log::error!(
                "portal_link.json has unknown version {}; renaming and ignoring",
                note.version
            );
            rename_corrupt(&path)?;
            Ok(None)
        }
        Err(e) => {
            log::error!("portal_link.json failed to parse ({e}); renaming and ignoring");
            rename_corrupt(&path)?;
            Ok(None)
        }
    }
}

fn rename_corrupt(path: &Path) -> std::io::Result<()> {
    // Format: YYYYMMDDTHHMMSSZ, e.g. "20260622T142203Z".
    let fmt = format_description!("[year][month][day]T[hour][minute][second]Z");
    let ts = OffsetDateTime::now_utc()
        .format(&fmt)
        .unwrap_or_else(|_| "unknown-time".to_string());
    let mut new_path = path.to_path_buf();
    new_path.set_file_name(format!("portal_link.corrupt.{ts}.json"));
    fs::rename(path, &new_path)
}

/// Atomically write the note: temp file → fsync → rename over target.
pub fn save(dir: &Path, note: &LinkSessionFile) -> std::io::Result<()> {
    let tmp = tmp_path(dir);
    {
        let mut f = fs::File::create(&tmp)?;
        serde_json::to_writer(&f, note).map_err(std::io::Error::other)?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, file_path(dir))?;
    Ok(())
}

/// Remove the note. A missing file is treated as success.
pub fn delete(dir: &Path) -> std::io::Result<()> {
    match fs::remove_file(file_path(dir)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// True iff `now` is within `window` of `last_active` (and not before it).
/// A `now` earlier than `last_active` (clock moved backwards) is not fresh.
pub fn is_fresh(last_active: OffsetDateTime, now: OffsetDateTime, window: time::Duration) -> bool {
    now >= last_active && (now - last_active) <= window
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use time::macros::datetime;

    fn sample(now: OffsetDateTime) -> LinkSessionFile {
        LinkSessionFile {
            version: LinkSessionFile::CURRENT_VERSION,
            event_id: EventId::from_full("events/2113-A").unwrap(),
            court: Some("1".to_string()),
            current_game: Some("G27".to_string()),
            last_played: None,
            last_played_start: None,
            mode: Mode::Hockey6V6,
            site: NoteSite::Portal,
            last_active: now,
        }
    }

    #[test]
    fn round_trips_via_serde() {
        let note = sample(datetime!(2026-06-22 14:22:03 UTC));
        let s = serde_json::to_string(&note).unwrap();
        let back: LinkSessionFile = serde_json::from_str(&s).unwrap();
        assert_eq!(note, back);
    }

    #[test]
    fn load_none_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(load_or_none(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn save_then_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let note = sample(OffsetDateTime::now_utc());
        save(tmp.path(), &note).unwrap();
        assert_eq!(load_or_none(tmp.path()).unwrap(), Some(note));
    }

    #[test]
    fn delete_removes_file_and_is_ok_when_missing() {
        let tmp = TempDir::new().unwrap();
        save(tmp.path(), &sample(OffsetDateTime::now_utc())).unwrap();
        delete(tmp.path()).unwrap();
        assert!(load_or_none(tmp.path()).unwrap().is_none());
        // second delete on an already-absent file is still Ok
        delete(tmp.path()).unwrap();
    }

    #[test]
    fn corrupt_file_is_renamed_and_none_returned() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("portal_link.json");
        std::fs::write(&path, b"not json").unwrap();
        assert!(load_or_none(tmp.path()).unwrap().is_none());
        assert!(!path.exists());
        let renamed = std::fs::read_dir(tmp.path()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("portal_link.corrupt")
        });
        assert!(renamed, "expected a corrupt backup file");
    }

    #[test]
    fn v1_note_migrates_game_number_to_current_game() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("portal_link.json");
        // Exactly the v1 shape: `game_number`, no `last_played`.
        let v1 = r#"{"version":1,"event_id":"events/2113-A","court":"1",
                     "game_number":"G27","mode":"Hockey6V6",
                     "last_active":"2026-08-17T09:00:00Z"}"#;
        std::fs::write(&path, v1).unwrap();
        let note = load_or_none(tmp.path())
            .unwrap()
            .expect("v1 note must still load");
        assert_eq!(note.current_game, Some("G27".to_string()));
        assert_eq!(note.last_played, None);
        assert_eq!(note.last_played_start, None);
        assert!(path.exists(), "a v1 note must not be quarantined");
    }

    #[test]
    fn v1_finished_encoding_migrates_to_no_history() {
        // v1 recorded a finished court as "court, but no game number". Under v2 that
        // is not a conclusion any more: no current game and no anchor, which the
        // decision function answers with NeedsPick — never a replay.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("portal_link.json");
        let v1 = r#"{"version":1,"event_id":"events/2113-A","court":"1",
                     "game_number":null,"mode":"Hockey6V6",
                     "last_active":"2026-08-17T09:00:00Z"}"#;
        std::fs::write(&path, v1).unwrap();
        let note = load_or_none(tmp.path()).unwrap().unwrap();
        assert_eq!(note.current_game, None);
        assert_eq!(note.last_played, None);
    }

    #[test]
    fn v2_round_trips_the_anchor() {
        let tmp = TempDir::new().unwrap();
        let note = LinkSessionFile {
            version: LinkSessionFile::CURRENT_VERSION,
            event_id: EventId::from_full("events/2113-A").unwrap(),
            court: Some("1".to_string()),
            current_game: None,
            last_played: Some("G27".to_string()),
            last_played_start: Some(datetime!(2026-08-17 14:00:00 UTC)),
            mode: Mode::Hockey6V6,
            site: NoteSite::Portal,
            last_active: datetime!(2026-08-17 14:22:03 UTC),
        };
        save(tmp.path(), &note).unwrap();
        assert_eq!(load_or_none(tmp.path()).unwrap(), Some(note));
    }

    #[test]
    fn a_future_version_is_still_quarantined() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("portal_link.json");
        let mut note = sample(OffsetDateTime::now_utc());
        note.version = 999;
        std::fs::write(&path, serde_json::to_string(&note).unwrap()).unwrap();
        assert!(load_or_none(tmp.path()).unwrap().is_none());
        assert!(!path.exists());
    }

    #[test]
    fn atomic_write_leaves_no_tmp_file() {
        let tmp = TempDir::new().unwrap();
        save(tmp.path(), &sample(OffsetDateTime::now_utc())).unwrap();
        assert!(tmp.path().join("portal_link.json").exists());
        assert!(!tmp.path().join("portal_link.json.tmp").exists());
    }

    #[test]
    fn a_note_written_before_the_site_field_reads_as_portal() {
        // The migration that matters: every note already on disk has no `site` key,
        // and every one of those was a Portal note as far as startup was concerned.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("portal_link.json");
        let v2 = r#"{"version":2,"event_id":"events/2113-A","court":"1",
                     "current_game":"G27","mode":"Hockey6V6",
                     "last_active":"2026-08-17T09:00:00Z"}"#;
        std::fs::write(&path, v2).unwrap();
        let note = load_or_none(tmp.path())
            .unwrap()
            .expect("a note without a site must still load");
        assert_eq!(note.site, NoteSite::Portal);
        assert!(path.exists(), "it must not be quarantined");
    }

    #[test]
    fn a_custom_site_note_round_trips_its_address() {
        let tmp = TempDir::new().unwrap();
        let note = LinkSessionFile {
            site: NoteSite::Custom {
                address: "http://scoreboard.local:8099/api/events/1234-A".to_string(),
            },
            ..sample(OffsetDateTime::now_utc())
        };
        save(tmp.path(), &note).unwrap();
        assert_eq!(load_or_none(tmp.path()).unwrap(), Some(note));
    }

    #[test]
    fn is_fresh_boundaries() {
        let t0 = datetime!(2026-06-22 08:00:00 UTC);
        assert!(is_fresh(t0, t0, FRESHNESS_WINDOW)); // 0h
        assert!(is_fresh(
            t0,
            t0 + FRESHNESS_WINDOW - time::Duration::hours(1),
            FRESHNESS_WINDOW
        )); // just inside the window
        assert!(is_fresh(t0, t0 + FRESHNESS_WINDOW, FRESHNESS_WINDOW)); // exactly the window
        assert!(!is_fresh(
            t0,
            t0 + FRESHNESS_WINDOW + time::Duration::seconds(1),
            FRESHNESS_WINDOW
        )); // just past the window
        // clock skew: "now" before last_active is treated as not fresh
        assert!(!is_fresh(
            t0,
            t0 - time::Duration::hours(1),
            FRESHNESS_WINDOW
        ));
    }
}
