//! Marker files that govern the trial-and-auto-revert boot decision.
//!
//! Two marker files live in the config directory:
//!
//! - `update_trial.marker` — written before rebooting into a new binary;
//!   contains three lines: the version being tried (line 1), the backup version
//!   (line 2), and a phase token (line 3: `pending` or `booted`).
//!
//!   Phase semantics:
//!   - `pending` — freshly written by the installer; the trial binary has not
//!     yet booted. On the first boot `decide_on_startup` arms the marker to
//!     `booted` and returns `Normal`, giving the new binary a chance to run.
//!   - `booted` — the trial binary has booted at least once but never cleared
//!     the marker (i.e. never reached a healthy state). On any subsequent boot
//!     `decide_on_startup` returns `AutoRevert`.
//!
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

const PHASE_PENDING: &str = "pending";
const PHASE_BOOTED: &str = "booted";

/// Decision made by `decide_on_startup` based on marker-file state.
///
/// Precedence (highest first):
/// 1. An armed (`booted`) trial marker → `AutoRevert`
/// 2. A rolled-back marker             → `ShowRolledBack`
/// 3. Neither                          → `Normal`
#[derive(Debug)]
pub enum StartupDecision {
    /// No marker files; proceed normally.
    Normal,
    /// A trial marker was booted once but never cleared — the previous boot
    /// must have crashed or failed to reach a healthy state.
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

/// Atomically write the trial marker with the given phase token.
///
/// The file contains three lines: the version being tried (line 1), the
/// backup version to revert to if the trial fails (line 2), and the phase
/// token (line 3: `pending` or `booted`).
fn write_trial_marker(
    dir: &Path,
    trying: &Version,
    backup: &Version,
    phase: &str,
) -> io::Result<()> {
    let tmp = trial_tmp_path(dir);
    {
        let mut f = fs::File::create(&tmp)?;
        writeln!(f, "{trying}")?;
        writeln!(f, "{backup}")?;
        writeln!(f, "{phase}")?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, trial_path(dir))?;
    // fsync the directory so the rename is durable.
    fsync_dir(dir)?;
    Ok(())
}

/// Atomically write the trial marker to `dir` in the `pending` phase.
///
/// Called by the installer before rebooting into the new binary.
/// The file contains three lines: the version being tried (line 1), the
/// backup version to revert to if the trial fails (line 2), and the phase
/// token `pending` (line 3).
pub fn write_trial(dir: &Path, trying: &Version, backup: &Version) -> io::Result<()> {
    write_trial_marker(dir, trying, backup, PHASE_PENDING)
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

/// Read all three fields from the trial marker file.
///
/// Returns `None` if the file is missing or either version line cannot be
/// parsed. The phase string is returned as-is; an absent or unrecognised
/// phase is treated by the caller as `pending` (backward-compat).
fn read_trial(dir: &Path) -> Option<(Version, Version, String)> {
    let contents = fs::read_to_string(trial_path(dir)).ok()?;
    let mut lines = contents.lines();
    let trying_str = lines.next()?;
    let backup_str = lines.next()?;
    let phase = lines.next().unwrap_or(PHASE_PENDING).to_owned();
    let trying = Version::parse(trying_str)?;
    let backup = Version::parse(backup_str)?;
    Some((trying, backup, phase))
}

/// Read the two versions recorded in the trial marker, if one is present.
///
/// Returns `(trying, backup)` — the version being trialled and the version it
/// would revert to. Used to log a successful self-update once the freshly
/// installed binary reaches a healthy state. Returns `None` when no marker is
/// present or it is malformed.
pub fn trial_versions(dir: &Path) -> Option<(Version, Version)> {
    read_trial(dir).map(|(trying, backup, _phase)| (trying, backup))
}

/// Read marker files and decide what the application should do on startup.
///
/// Two-phase trial logic:
/// - First boot of a `pending` trial: arm the marker to `booted`, return
///   `Normal`. The new binary gets to run; if it reaches a healthy state, it
///   clears the marker. If it crashes or restarts before clearing, the next
///   call will see `booted` and trigger `AutoRevert`.
/// - Any boot with a `booted` trial: the trial binary has run before but
///   never cleared the marker — return `AutoRevert`.
///
/// Precedence: armed trial > rolled-back marker > normal.
pub fn decide_on_startup(dir: &Path) -> StartupDecision {
    // --- Trial marker check (highest priority) ---
    if trial_path(dir).exists() {
        match read_trial(dir) {
            Some((trying, backup, phase)) => {
                if phase == PHASE_BOOTED {
                    // Second (or later) boot with an uncleared trial marker:
                    // the trial binary failed to reach a healthy state.
                    return StartupDecision::AutoRevert {
                        backup_version: backup,
                    };
                }
                // First boot of the trial binary (`pending`, or any unrecognised
                // phase for backward-compat). Arm the marker and give the new
                // binary a grace pass. Best-effort: if the re-write fails, we
                // still allow the boot rather than blocking it.
                let _ = write_trial_marker(dir, &trying, &backup, PHASE_BOOTED);
                return StartupDecision::Normal;
            }
            None => {
                // Malformed trial marker (should never happen — we always write
                // well-formed); treat as no-trial so we don't get stuck in an
                // un-revertable boot loop. The operator will see Normal startup.
            }
        }
    }

    // --- Rolled-back marker check ---
    if rolled_back_path(dir).exists() {
        return StartupDecision::ShowRolledBack;
    }

    StartupDecision::Normal
}

/// fsync the directory itself so that renames/removals are durable.
///
/// On some filesystems a rename is not persisted until the containing
/// directory's metadata is fsynced.
#[cfg(unix)]
fn fsync_dir(dir: &Path) -> io::Result<()> {
    let d = fs::File::open(dir)?;
    d.sync_all()?;
    Ok(())
}

/// Windows cannot open a directory as a file handle for fsync (it returns
/// "Access is denied"), and directory fsync is not the durability mechanism
/// there, so this is a no-op. (Same approach taken by tempfile/atomicwrites.)
#[cfg(not(unix))]
fn fsync_dir(_dir: &Path) -> io::Result<()> {
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
        // Install writes trial, new binary clears it after reaching healthy state.
        let d = tempfile::tempdir().unwrap();
        write_trial(d.path(), &v("0.4.2"), &v("0.4.1")).unwrap();
        // First boot: arms marker, returns Normal.
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
        // Binary reaches healthy state and clears the marker.
        clear_trial(d.path()).unwrap();
        // Next startup: no marker, Normal.
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
    }

    #[test]
    fn fresh_trial_first_boot_normal_then_autorevert_on_second() {
        let d = tempfile::tempdir().unwrap();
        write_trial(d.path(), &v("0.4.3"), &v("0.4.2")).unwrap();
        // First boot: grace pass — new binary gets to run.
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
        // Second boot, still uncleared -> auto-revert.
        match decide_on_startup(d.path()) {
            StartupDecision::AutoRevert { backup_version } => {
                assert_eq!(backup_version, v("0.4.2"))
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
        // Both markers present: an armed (booted) trial wins over rolled-back.
        let d = tempfile::tempdir().unwrap();
        write_rolled_back(d.path()).unwrap();
        write_trial(d.path(), &v("0.5.0"), &v("0.4.9")).unwrap();
        // First decide arms the trial marker.
        assert!(matches!(
            decide_on_startup(d.path()),
            StartupDecision::Normal
        ));
        // Second decide: armed trial takes precedence over rolled-back.
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

    #[test]
    fn trial_versions_reads_both_versions_when_present() {
        let d = tempfile::tempdir().unwrap();
        // No marker -> None.
        assert_eq!(trial_versions(d.path()), None);
        // After a trial is written, both versions are recoverable.
        write_trial(d.path(), &v("0.4.4"), &v("0.4.3")).unwrap();
        assert_eq!(trial_versions(d.path()), Some((v("0.4.4"), v("0.4.3"))));
    }
}
