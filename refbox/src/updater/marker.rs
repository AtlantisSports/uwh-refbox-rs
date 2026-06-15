//! Marker files that govern the trial-and-auto-revert boot decision.
//!
//! Two marker files live in the config directory:
//!
//! - `update_trial.marker` — written before rebooting into a new binary;
//!   contains the version being tried (line 1) and the backup version (line 2).
//!   If this file is present on startup it means the previous boot *did not*
//!   clear it cleanly, so we auto-revert.
//! - `update_rolled_back.marker` — written after a successful auto-revert;
//!   presence triggers a one-time "update was rolled back" notice to the
//!   operator. Cleared once the notice has been shown.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::version::Version;

const TRIAL: &str = "update_trial.marker";
const ROLLED_BACK: &str = "update_rolled_back.marker";
const TRIAL_TMP: &str = "update_trial.marker.tmp";
const ROLLED_BACK_TMP: &str = "update_rolled_back.marker.tmp";

/// Decision made by `decide_on_startup` based on marker-file state.
///
/// Precedence (highest first):
/// 1. An uncleared trial marker → `AutoRevert`
/// 2. A rolled-back marker     → `ShowRolledBack`
/// 3. Neither                  → `Normal`
#[derive(Debug)]
pub enum StartupDecision {
    /// No marker files; proceed normally.
    Normal,
    /// A trial marker was not cleared — the previous boot must have crashed.
    /// The updater should revert to the backup binary identified by
    /// `backup_version`.
    AutoRevert { backup_version: Version },
    /// A rolled-back marker is present; show the operator a one-time notice
    /// that the previous update was rolled back.
    ShowRolledBack,
}

fn trial_path(dir: &Path) -> PathBuf {
    dir.join(TRIAL)
}

fn trial_tmp_path(dir: &Path) -> PathBuf {
    dir.join(TRIAL_TMP)
}

fn rolled_back_path(dir: &Path) -> PathBuf {
    dir.join(ROLLED_BACK)
}

fn rolled_back_tmp_path(dir: &Path) -> PathBuf {
    dir.join(ROLLED_BACK_TMP)
}

/// Atomically write the trial marker to `dir`.
///
/// The file contains two lines: the version being tried (line 1) and the
/// backup version to revert to if the trial fails (line 2).
pub fn write_trial(dir: &Path, trying: &Version, backup: &Version) -> io::Result<()> {
    let tmp = trial_tmp_path(dir);
    {
        let mut f = fs::File::create(&tmp)?;
        writeln!(f, "{trying}")?;
        writeln!(f, "{backup}")?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, trial_path(dir))?;
    // fsync the directory so the rename is durable.
    fsync_dir(dir)?;
    Ok(())
}

/// Remove the trial marker atomically. Idempotent — succeeds if already absent.
pub fn clear_trial(dir: &Path) -> io::Result<()> {
    match fs::remove_file(trial_path(dir)) {
        Ok(()) => {
            fsync_dir(dir)?;
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Atomically write the rolled-back marker to `dir`.
///
/// Written after a successful auto-revert so the operator is shown a notice
/// on the next startup.
pub fn write_rolled_back(dir: &Path) -> io::Result<()> {
    let tmp = rolled_back_tmp_path(dir);
    {
        let mut f = fs::File::create(&tmp)?;
        // Content is not significant; presence is what matters.
        f.write_all(b"rolled_back\n")?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, rolled_back_path(dir))?;
    fsync_dir(dir)?;
    Ok(())
}

/// Remove the rolled-back marker atomically. Idempotent — succeeds if already absent.
pub fn clear_rolled_back(dir: &Path) -> io::Result<()> {
    match fs::remove_file(rolled_back_path(dir)) {
        Ok(()) => {
            fsync_dir(dir)?;
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Read marker files and decide what the application should do on startup.
///
/// Precedence: trial marker (auto-revert) > rolled-back marker > normal.
pub fn decide_on_startup(dir: &Path) -> StartupDecision {
    // --- Trial marker check (highest priority) ---
    let trial = trial_path(dir);
    if trial.exists() {
        match read_trial_backup(dir) {
            Some(backup_version) => return StartupDecision::AutoRevert { backup_version },
            None => {
                // malformed trial marker (should never happen — we always write well-formed);
                // treat as no-trial so we don't get stuck in an un-revertable boot loop.
                // The operator will see Normal startup; the trial file should be cleared.
            }
        }
    }

    // --- Rolled-back marker check ---
    if rolled_back_path(dir).exists() {
        return StartupDecision::ShowRolledBack;
    }

    StartupDecision::Normal
}

/// Read the backup version from the trial marker file.
///
/// Returns `None` if the file is missing or the backup line cannot be parsed.
fn read_trial_backup(dir: &Path) -> Option<Version> {
    let contents = fs::read_to_string(trial_path(dir)).ok()?;
    let mut lines = contents.lines();
    let _trying = lines.next()?; // line 1: the version being tried (ignored here)
    let backup_str = lines.next()?; // line 2: the backup version
    Version::parse(backup_str)
}

/// fsync the directory itself so that renames/removals are durable.
///
/// On some filesystems the rename is not persisted until the containing
/// directory's metadata is fsynced. This mirrors the pattern used in
/// `portal_manager/queue.rs`.
fn fsync_dir(dir: &Path) -> io::Result<()> {
    let d = fs::File::open(dir)?;
    d.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn trial_then_healthy_is_normal() {
        let d = tempfile::tempdir().unwrap();
        write_trial(d.path(), &v("0.4.2"), &v("0.4.1")).unwrap();
        clear_trial(d.path()).unwrap();
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
    }

    #[test]
    fn uncleared_trial_triggers_autorevert() {
        let d = tempfile::tempdir().unwrap();
        write_trial(d.path(), &v("0.4.2"), &v("0.4.1")).unwrap();
        match decide_on_startup(d.path()) {
            StartupDecision::AutoRevert { backup_version } => {
                assert_eq!(backup_version, v("0.4.1"))
            }
            other => panic!("expected AutoRevert, got {other:?}"),
        }
    }

    #[test]
    fn rolled_back_marker_shows_message_once() {
        let d = tempfile::tempdir().unwrap();
        write_rolled_back(d.path()).unwrap();
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::ShowRolledBack
        ));
        clear_rolled_back(d.path()).unwrap();
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
    }

    #[test]
    fn clear_trial_is_idempotent() {
        let d = tempfile::tempdir().unwrap();
        // Clearing when absent must not error.
        clear_trial(d.path()).unwrap();
        // Write then double-clear.
        write_trial(d.path(), &v("0.4.2"), &v("0.4.1")).unwrap();
        clear_trial(d.path()).unwrap();
        clear_trial(d.path()).unwrap();
    }

    #[test]
    fn clear_rolled_back_is_idempotent() {
        let d = tempfile::tempdir().unwrap();
        clear_rolled_back(d.path()).unwrap();
        write_rolled_back(d.path()).unwrap();
        clear_rolled_back(d.path()).unwrap();
        clear_rolled_back(d.path()).unwrap();
    }

    #[test]
    fn trial_takes_precedence_over_rolled_back() {
        // Both markers present: trial wins.
        let d = tempfile::tempdir().unwrap();
        write_rolled_back(d.path()).unwrap();
        write_trial(d.path(), &v("0.5.0"), &v("0.4.9")).unwrap();
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::AutoRevert { backup_version } if backup_version == v("0.4.9")
        ));
    }

    #[test]
    fn malformed_trial_treated_as_normal() {
        let d = tempfile::tempdir().unwrap();
        // Write a trial file with no backup version line.
        std::fs::write(d.path().join(TRIAL), b"0.4.2\n").unwrap();
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
    }

    #[test]
    fn empty_dir_is_normal() {
        let d = tempfile::tempdir().unwrap();
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
    }
}
