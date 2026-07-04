# Portal Link Restore Across Restart — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After a relaunch (language change, self-update) or an overnight shutdown within a tournament, the refbox comes back already linked to the same UWH Portal event, court, and game — showing the live countdown to that game's scheduled start — without re-prompting for the token, while a fresh power-on weeks later for a new event starts clean.

**Architecture:** A small versioned JSON file (`portal_link.json`) beside the existing `portal_queue.json` records the live link (event, court, game, mode, last-active timestamp). It is written/refreshed while linked, deleted when unlinked. At startup, if the note is ≤48 h old and its mode uses the current portal, the app re-links and re-establishes the scheduled-start countdown by reusing existing tournament-manager methods.

**Tech Stack:** Rust 2024, `serde` + `serde_json`, `time` crate (`OffsetDateTime`, rfc3339 serde), iced 0.13. Mirrors `refbox/src/portal_manager/queue.rs`.

## Global Constraints

- MSRV Rust 1.85; Edition 2024. Do not use newer std/lang features.
- Clippy `-D warnings` must pass: `cargo clippy -p refbox -- -D warnings` (no `--all-targets`; mirrors CI/`just lint`).
- No `unwrap()`/`expect()` in non-test production code without a "why this cannot panic" comment.
- Changes confined to crate `refbox`. No `uwh-common`, wire-format, wireless-remote, LED-panel, or overlay changes.
- `EventId` and `Mode` are already `Serialize`/`Deserialize`; `GameNumber = String`.
- Approval gates: do NOT create a branch, commit, or open a PR without the human's explicit go-ahead. (The per-task "Commit" steps below are gated on that approval.)
- The design spec is `docs/superpowers/specs/2026-06-22-portal-link-restore-across-restart-design.md`.

---

### Task 1: `portal_link.json` file module (pure I/O + freshness)

A self-contained module mirroring `queue.rs`: the `LinkSessionFile` type, atomic save, tolerant load, corrupt-rename, and the pure freshness check. No dependency on `RefBoxApp`.

**Files:**
- Create: `refbox/src/portal_manager/link_session.rs`
- Modify: `refbox/src/portal_manager/mod.rs` (add the module declaration)
- Test: in `refbox/src/portal_manager/link_session.rs` `#[cfg(test)]`

**Interfaces:**
- Produces:
  - `pub struct LinkSessionFile { pub version: u32, pub event_id: EventId, pub court: Option<String>, pub game_number: Option<GameNumber>, pub mode: crate::config::Mode, #[serde(with = "time::serde::rfc3339")] pub last_active: OffsetDateTime }`
  - `impl LinkSessionFile { pub const CURRENT_VERSION: u32 = 1; }`
  - `pub fn load_or_none(dir: &Path) -> std::io::Result<Option<LinkSessionFile>>`
  - `pub fn save(dir: &Path, note: &LinkSessionFile) -> std::io::Result<()>`
  - `pub fn delete(dir: &Path) -> std::io::Result<()>` (removes the file; missing file is Ok)
  - `pub const FRESHNESS_WINDOW: time::Duration = time::Duration::hours(48);`
  - `pub fn is_fresh(last_active: OffsetDateTime, now: OffsetDateTime, window: time::Duration) -> bool` → `now >= last_active && (now - last_active) <= window`

- [ ] **Step 1: Add the module declaration**

In `refbox/src/portal_manager/mod.rs`, beside the existing `mod queue;` / `mod health;` declarations, add:

```rust
mod link_session;
```

(Keep the same visibility the sibling `queue`/`health` modules use; do not re-export unless a later task needs it — Task 3/5 will reference items as `crate::portal_manager::link_session::*`, so make it `pub(crate) mod link_session;` if `queue`/`health` are also `pub(crate)`, otherwise `mod link_session;` and add `pub(crate)` to the items.)

- [ ] **Step 2: Write the failing tests**

Create `refbox/src/portal_manager/link_session.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use time::macros::datetime;

    fn sample(now: OffsetDateTime) -> LinkSessionFile {
        LinkSessionFile {
            version: LinkSessionFile::CURRENT_VERSION,
            event_id: EventId::from_full("events/2113-A"),
            court: Some("1".to_string()),
            game_number: Some("G27".to_string()),
            mode: crate::config::Mode::Hockey6V6,
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
        let renamed = std::fs::read_dir(tmp.path())
            .unwrap()
            .any(|e| e.unwrap().file_name().to_string_lossy().starts_with("portal_link.corrupt"));
        assert!(renamed, "expected a corrupt backup file");
    }

    #[test]
    fn unknown_version_is_renamed_and_none_returned() {
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
    fn is_fresh_boundaries() {
        let t0 = datetime!(2026-06-22 08:00:00 UTC);
        assert!(is_fresh(t0, t0, FRESHNESS_WINDOW)); // 0h
        assert!(is_fresh(t0, t0 + time::Duration::hours(47), FRESHNESS_WINDOW));
        assert!(is_fresh(t0, t0 + FRESHNESS_WINDOW, FRESHNESS_WINDOW)); // exactly 48h
        assert!(!is_fresh(t0, t0 + time::Duration::hours(48) + time::Duration::seconds(1), FRESHNESS_WINDOW));
        // clock skew: "now" before last_active is treated as not fresh
        assert!(!is_fresh(t0, t0 - time::Duration::hours(1), FRESHNESS_WINDOW));
    }
}
```

> Note: confirm the `EventId` constructor used in tests. Check `uwh-common/src/uwhportal/schedule.rs` for the public constructor (e.g. `EventId::from_full` / `EventId::new`). If the name differs, use the real one; if there is none, construct via `serde_json::from_str("\"events/2113-A\"")` or the field path. This is the only test-only unknown.

- [ ] **Step 3: Run tests, verify they fail**

Run: `cargo test -p refbox portal_manager::link_session -- --nocapture`
Expected: FAIL to compile (`LinkSessionFile` / `load_or_none` / `save` / `delete` / `is_fresh` not found).

- [ ] **Step 4: Write the implementation**

Prepend the production code to `refbox/src/portal_manager/link_session.rs` (above the test module):

```rust
//! On-disk persistence for the active portal link ("link session").
//!
//! Records which event/court/game this machine is linked to plus a
//! "last active" timestamp, so a relaunch (language change, self-update)
//! or a short shutdown can re-establish the link instead of starting
//! dormant. See docs/superpowers/specs/2026-06-22-portal-link-restore-across-restart-design.md.
//!
//! Mirrors the atomic-write + tolerant-load pattern of `queue.rs`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::macros::format_description;

use crate::app::languages; // not needed; remove if unused
use crate::config::Mode;
use uwh_common::uwhportal::schedule::{EventId, GameNumber};

/// How recent the last session must be to auto-restore the link on startup.
pub const FRESHNESS_WINDOW: time::Duration = time::Duration::hours(48);

const FILE_NAME: &str = "portal_link.json";
const TMP_FILE_NAME: &str = "portal_link.json.tmp";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSessionFile {
    pub version: u32,
    pub event_id: EventId,
    pub court: Option<String>,
    pub game_number: Option<GameNumber>,
    pub mode: Mode,
    #[serde(with = "time::serde::rfc3339")]
    pub last_active: OffsetDateTime,
}

impl LinkSessionFile {
    pub const CURRENT_VERSION: u32 = 1;
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
        Ok(note) if note.version == LinkSessionFile::CURRENT_VERSION => Ok(Some(note)),
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
pub fn is_fresh(last_active: OffsetDateTime, now: OffsetDateTime, window: time::Duration) -> bool {
    now >= last_active && (now - last_active) <= window
}
```

> Remove the `use crate::app::languages;` line — it was a stray import in the sketch and is unused. Keep only the imports the code actually references. Verify `GameNumber` is exported from `uwh_common::uwhportal::schedule` (it is: `pub type GameNumber = String;`).

- [ ] **Step 5: Run tests, verify they pass**

Run: `cargo test -p refbox portal_manager::link_session`
Expected: PASS (all tests above green).

- [ ] **Step 6: Lint**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit** (only after human approval to commit)

```bash
git add refbox/src/portal_manager/link_session.rs refbox/src/portal_manager/mod.rs
git commit -m "feat(refbox): add portal_link.json link-session file module"
```

---

### Task 2: `persist_link_session` helper + write/clear wiring

Add a method on `RefBoxApp` that recomputes the note from current state and writes or deletes the file, and call it at every point the link state settles.

**Files:**
- Modify: `refbox/src/app/mod.rs` (new method; calls in apply paths, unlink paths, `handle_game_end`)

**Interfaces:**
- Consumes: `crate::portal_manager::link_session::{LinkSessionFile, save, delete}` (Task 1).
- Produces: `fn persist_link_session(&self)` on `impl RefBoxApp`.

- [ ] **Step 1: Add the helper method**

Add to the main `impl RefBoxApp` block (near `set_current_event_id`, ~`refbox/src/app/mod.rs:822`):

```rust
/// Write or delete `portal_link.json` to reflect the current live link.
/// Linked (portal on + an event selected) → write a note stamped `now`.
/// Not linked → delete any existing note. Errors are logged, never fatal:
/// a failed note write only means a future restart won't auto-relink.
fn persist_link_session(&self) {
    use crate::portal_manager::link_session::{self, LinkSessionFile};
    if self.using_uwhportal {
        if let Some(event_id) = self.current_event_id.clone() {
            // The game the operator is on: the upcoming game between games,
            // otherwise the current game number from the live snapshot.
            let game_number = if self.snapshot.current_period
                == uwh_common::game_snapshot::GamePeriod::BetweenGames
            {
                Some(self.snapshot.next_game_number.clone())
            } else {
                Some(self.snapshot.game_number.clone())
            };
            let note = LinkSessionFile {
                version: LinkSessionFile::CURRENT_VERSION,
                event_id,
                court: self.current_court.clone(),
                game_number,
                mode: self.config.mode,
                last_active: time::OffsetDateTime::now_utc(),
            };
            if let Err(e) = link_session::save(&self.config_dir, &note) {
                log::error!("Failed to write portal_link.json: {e}");
            }
            return;
        }
    }
    if let Err(e) = link_session::delete(&self.config_dir) {
        log::error!("Failed to delete portal_link.json: {e}");
    }
}
```

> Verify the `GamePeriod` import path: `uwh_common::game_snapshot::GamePeriod` (check existing `use` lines at the top of `mod.rs`; the file already imports `GamePeriod` — reuse the in-scope name rather than the full path if it's already imported). Verify `self.snapshot.game_number` / `self.snapshot.next_game_number` field names against `uwh_common/src/game_snapshot.rs:51-52` (`game_number`, `next_game_number`, both `GameNumber`).

- [ ] **Step 2: Call it where the link settles**

Add `self.persist_link_session();` at the end of each of these handlers/methods, after the portal fields have been committed:

1. `apply_app_options` — at the end of the success path (after `self.using_uwhportal = ...` / `self.set_current_event_id(...)` are committed; the method around `refbox/src/app/mod.rs:848`–`1041`). Add the call before the final `return`/end so it runs on the committed state. There are two commit sites in this method (the early branch ~`983-1041` and the later one); add the call so both committed outcomes persist — simplest is to add it once at the natural single exit, or in both branches if they return separately.
2. The unlink confirmation path that calls `set_current_event_id(None)` (~`1158`): add the call after the portal is turned off so the note is deleted.
3. `handle_game_end` (~`780`): add `self.persist_link_session();` before `Task::batch(tasks)` so the advanced game number is captured promptly.

> The helper is idempotent and cheap; calling it more than once on the same state is harmless (it just rewrites/deletes). When in doubt, prefer an extra call over a missed one.

- [ ] **Step 3: Build + lint**

Run: `cargo build -p refbox` then `cargo clippy -p refbox -- -D warnings`
Expected: builds, no warnings.

- [ ] **Step 4: Commit** (after approval)

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): persist portal link session on link/unlink/game-end"
```

---

### Task 3: Startup restore decision + re-link + schedule fetch

At startup, read the note, decide whether to restore (fresh + same portal), and if so re-link and queue the event-list + schedule fetch. Add the one-shot `pending_restore_game` field.

**Files:**
- Modify: `refbox/src/app/mod.rs` (`RefBoxApp` struct field; `RefBoxApp::new`; a pure `decide_restore` helper + its test)

**Interfaces:**
- Consumes: `link_session::{load_or_none, is_fresh, FRESHNESS_WINDOW, LinkSessionFile}` (Task 1); `crate::app::shared_elements::crosses_portal`.
- Produces: `pending_restore_game: Option<GameNumber>` field on `RefBoxApp`; `fn decide_restore(note: &LinkSessionFile, now: OffsetDateTime, current_mode: Mode) -> bool`.

- [ ] **Step 1: Write the failing test for the decision helper**

Add to the `#[cfg(test)]` module of `refbox/src/app/mod.rs` (or the nearest app test module):

```rust
#[test]
fn decide_restore_fresh_same_portal_true() {
    use crate::portal_manager::link_session::LinkSessionFile;
    let now = time::OffsetDateTime::now_utc();
    let note = LinkSessionFile {
        version: LinkSessionFile::CURRENT_VERSION,
        event_id: uwh_common::uwhportal::schedule::EventId::from_full("events/2113-A"),
        court: Some("1".into()),
        game_number: Some("G1".into()),
        mode: Mode::Hockey6V6,
        last_active: now - time::Duration::hours(20),
    };
    assert!(decide_restore(&note, now, Mode::Hockey6V6));
    // 3v3 shares the UWH portal with 6v6 → still restore
    assert!(decide_restore(&note, now, Mode::Hockey3V3));
}

#[test]
fn decide_restore_stale_false() {
    use crate::portal_manager::link_session::LinkSessionFile;
    let now = time::OffsetDateTime::now_utc();
    let note = LinkSessionFile {
        version: LinkSessionFile::CURRENT_VERSION,
        event_id: uwh_common::uwhportal::schedule::EventId::from_full("events/2113-A"),
        court: None, game_number: None, mode: Mode::Hockey6V6,
        last_active: now - time::Duration::hours(49),
    };
    assert!(!decide_restore(&note, now, Mode::Hockey6V6));
}

#[test]
fn decide_restore_cross_portal_false() {
    use crate::portal_manager::link_session::LinkSessionFile;
    let now = time::OffsetDateTime::now_utc();
    let note = LinkSessionFile {
        version: LinkSessionFile::CURRENT_VERSION,
        event_id: uwh_common::uwhportal::schedule::EventId::from_full("events/2113-A"),
        court: None, game_number: None, mode: Mode::Hockey6V6,
        last_active: now,
    };
    // Rugby uses the UWR portal → must NOT restore a UWH note
    assert!(!decide_restore(&note, now, Mode::Rugby));
}
```

> Adjust `EventId::from_full` to the real constructor (see Task 1 note). Confirm `crosses_portal(Mode::Hockey6V6, Mode::Rugby) == true` and `crosses_portal(Mode::Hockey6V6, Mode::Hockey3V3) == false` by reading `shared_elements::crosses_portal`; if its semantics differ, adjust the assertions and the helper accordingly.

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p refbox decide_restore`
Expected: FAIL (`decide_restore` not found).

- [ ] **Step 3: Implement the decision helper**

Add as a free function in `refbox/src/app/mod.rs` (near other free helpers like `font_family_id`):

```rust
/// Decide whether a remembered link note should be restored at startup:
/// it must be fresh (within the freshness window) and belong to the same
/// portal as the current mode (a UWH note is never restored into UWR).
fn decide_restore(
    note: &crate::portal_manager::link_session::LinkSessionFile,
    now: time::OffsetDateTime,
    current_mode: Mode,
) -> bool {
    use crate::app::shared_elements::crosses_portal;
    use crate::portal_manager::link_session::{is_fresh, FRESHNESS_WINDOW};
    is_fresh(note.last_active, now, FRESHNESS_WINDOW) && !crosses_portal(note.mode, current_mode)
}
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p refbox decide_restore`
Expected: PASS.

- [ ] **Step 5: Add the `pending_restore_game` field**

In the `RefBoxApp` struct (near `current_event_id`, ~`refbox/src/app/mod.rs:169`):

```rust
/// One-shot: the game number to re-select once the schedule arrives,
/// set during a startup link restore and cleared on first use. `None`
/// in normal operation.
pending_restore_game: Option<GameNumber>,
```

Initialise it to `None` in the struct literal in `RefBoxApp::new` (the literal around `:1536`–`1580`, beside `current_event_id: None,`):

```rust
pending_restore_game: None,
```

- [ ] **Step 6: Restore at startup**

In `RefBoxApp::new`, after the `new` struct literal is built (after `:1580`) and **before** the `startup_tasks` are assembled (`:1585`), insert the restore block. It mutates `new` and queues a schedule fetch:

```rust
// Restore a recent portal link so a relaunch (language change, self-update)
// or short shutdown comes back recognized instead of dormant. A stale or
// cross-portal note is ignored (and a stale one deleted) so a fresh power-on
// weeks later for a new event starts clean. See ADR 011/017 amendment.
let mut restore_schedule_for: Option<EventId> = None;
match crate::portal_manager::link_session::load_or_none(&new.config_dir) {
    Ok(Some(note)) => {
        if decide_restore(&note, time::OffsetDateTime::now_utc(), new.config.mode) {
            info!("Restoring portal link to {} (court {:?}, game {:?})",
                note.event_id.full(), note.court, note.game_number);
            new.using_uwhportal = true;
            new.current_court = note.court.clone();
            new.pending_restore_game = note.game_number.clone();
            new.set_current_event_id(Some(note.event_id.clone()));
            restore_schedule_for = Some(note.event_id);
        } else {
            info!("Portal link note present but stale/cross-portal; starting dormant");
            let _ = crate::portal_manager::link_session::delete(&new.config_dir);
        }
    }
    Ok(None) => {}
    Err(e) => log::error!("Failed to read portal_link.json: {e}"),
}
```

Then, where startup tasks are pushed (`:1590`), the existing `if new.using_uwhportal { startup_tasks.push(new.request_event_list()); }` now fires because restore set the flag. Immediately after that line, also fetch the schedule:

```rust
if let Some(event_id) = restore_schedule_for {
    startup_tasks.push(new.request_schedule(event_id));
}
```

> `set_current_event_id` takes `&mut self` and `new` is `let mut new` already (it is constructed mutably). `request_event_list`/`request_schedule` take `&self` and return `Task<Message>`. Confirm `EventId` is in scope at this point (it is used elsewhere in `mod.rs`).

- [ ] **Step 7: Build, test, lint**

Run: `cargo build -p refbox && cargo test -p refbox decide_restore && cargo clippy -p refbox -- -D warnings`
Expected: builds, tests pass, no warnings.

- [ ] **Step 8: Commit** (after approval)

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): restore recent portal link at startup"
```

---

### Task 4: Re-select the game and start the scheduled countdown

When the schedule arrives for the restored event, consume `pending_restore_game`, set it as the next game, and start the live countdown to its scheduled start by reusing `apply_next_game_start`.

**Files:**
- Modify: `refbox/src/app/mod.rs` (`Message::RecvSchedule` handler, ~`:3583`)

**Interfaces:**
- Consumes: `pending_restore_game` (Task 3); `tm.set_next_game`, `tm.apply_next_game_start`, `tm.generate_snapshot`, `self.apply_snapshot`, `schedule.get_game_and_timing` (all existing).

- [ ] **Step 1: Add the restore branch in `RecvSchedule`**

Inside `Message::RecvSchedule`, after `self.schedule = Some(schedule);` is set for the current event (the block guarded by `if *id == event_id` and `self.edited_settings.is_none()`, around `:3628`–`:3656`), add a restore branch that runs when `pending_restore_game` is `Some`. Place it so it takes precedence over the existing "use `tm.next_game_number()`" default for this one arrival:

```rust
if let Some(restore_num) = self.pending_restore_game.take() {
    let mut tm = self.tm.lock().unwrap();
    if tm.current_period() == GamePeriod::BetweenGames {
        if let (Some(game), Some(timing)) = self
            .schedule
            .as_ref()
            .unwrap()
            .get_game_and_timing(&restore_num)
        {
            info!("Restoring upcoming game {restore_num} from schedule");
            tm.set_next_game(NextGameInfo {
                number: game.number.clone(),
                timing: Some(timing.clone()),
                start_time: Some(game.start_time),
            });
            let now = Instant::now();
            // why this cannot panic: we just confirmed BetweenGames and set
            // next_game, the two preconditions apply_next_game_start checks.
            tm.apply_next_game_start(now).unwrap();
            self.config.game = tm.config().clone();
            let snapshot = tm.generate_snapshot(now).unwrap();
            drop(tm);
            return self.apply_snapshot(snapshot);
        } else {
            warn!("Restore game {restore_num} not found in schedule; staying on default");
        }
    }
}
```

> This mirrors the existing `set_next_game(NextGameInfo { number, timing, start_time })` construction a few lines above (`:3643`). The difference is it looks up `restore_num` (the remembered game) instead of `tm.next_game_number()`, and additionally calls `apply_next_game_start` to start the live countdown. The `return self.apply_snapshot(...)` ends the handler for the restore arrival; ensure it does not bypass essential later work in the handler — if the handler has trailing side effects needed in all cases, instead bind the snapshot, fall through, and apply it at the existing handler exit. Confirm `apply_snapshot` returns `Task<Message>` and matches the handler's return type.

- [ ] **Step 2: Build + lint**

Run: `cargo build -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: builds, no warnings.

- [ ] **Step 3: Commit** (after approval)

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): re-select game and start scheduled countdown on restore"
```

---

### Task 5: Refresh the timestamp on the portal heartbeat

Keep `last_active` current while the app runs linked, so the freshness window measures the gap since the machine was last in use — not since the link was first made.

**Files:**
- Modify: `refbox/src/app/mod.rs` (`Message::PortalEvent` handler, ~`:2361`)

**Interfaces:**
- Consumes: `persist_link_session` (Task 2).

- [ ] **Step 1: Refresh on the heartbeat**

In the `Message::PortalEvent(ev)` handler, after the `match ev { ... }` block and before `Task::none()` (~`:2380`), add:

```rust
// The background portal task fires verify_token on its cadence
// (~5 min when healthy). Use that heartbeat to refresh the link note's
// last-active timestamp so the 48h restore window tracks real usage.
if self.using_uwhportal && self.current_event_id.is_some() {
    self.persist_link_session();
}
```

- [ ] **Step 2: Build + lint**

Run: `cargo build -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: builds, no warnings.

- [ ] **Step 3: Commit** (after approval)

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): refresh portal link timestamp on heartbeat"
```

---

### Task 6: Full check + manual acceptance walkthrough

**Files:** none (verification only).

- [ ] **Step 1: Run the full suite**

Run: `just check`
Expected: fmt, lint, tests, audit all green.

- [ ] **Step 2: Rebuild the binary before any walkthrough**

Run: `cargo build -p refbox`
(Walkthrough must run `target/debug/refbox`, not a clippy/test binary.)

- [ ] **Step 3: Manual acceptance (operator-verifiable)**

Run the app (`WAYLAND_DISPLAY= cargo run -p refbox`, with `dangerouslyDisableSandbox`) and confirm, against a dev portal event (`UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com`):

1. Link to an event, select a court and a game, then switch language **Korean ↔ English**. After the relaunch: still linked to the same event + court + game; the clock shows the live countdown to the scheduled start; **no token prompt**.
2. Switch **English ↔ Spanish**: no relaunch, nothing changes.
3. Link, then quit and relaunch the program (cold) the **same day**: it reconnects.
4. Hand-edit `~/.config/refbox/portal_link.json` `last_active` to **>48 h** ago, relaunch: starts clean and dormant (no event, portal off).
5. Switch **Mode Hockey → Rugby** (cross-portal restart): the UWH link is **not** restored.
6. With an expired token: restore shows the normal re-link prompt.

- [ ] **Step 4: Verify boot-at-7:42 countdown (the user's scenario)**

With a fresh note for a game whose `start_time` is ~18 min in the future, relaunch and confirm the "time to next game" reads ~18:00 counting down (and that booting within 4 min of, or after, the start shows the 4-minute minimum break instead).

---

## Self-Review

**Spec coverage:**
- Link note file (contents, atomic write, corrupt-rename) → Task 1. ✓
- Lifecycle (write on link, refresh on heartbeat, clear on unlink) → Task 2 (write/clear) + Task 5 (refresh). ✓
- Startup restore (48h + mode gate) → Task 3 (`decide_restore` + `new()` wiring). ✓
- Re-select game + scheduled countdown (4-min floor via existing logic) → Task 4. ✓
- Edge cases (stale → clean+delete, cross-portal, corrupt/missing, Latin-only no-relaunch, token expired) → Tasks 1/3 + acceptance Task 6. ✓
- Acceptance criteria + tests → Task 1/3 unit tests + Task 6 walkthrough. ✓
- ADR note → recorded in spec; restate in PR body at merge time. ✓

**Placeholder scan:** No "TBD/TODO". The three "confirm X" notes (EventId constructor name, GamePeriod import path, `crosses_portal` semantics) are verification instructions with a concrete fallback, not unfilled gaps — the engineer reads the named file and uses the real symbol.

**Type consistency:** `LinkSessionFile` fields and `load_or_none`/`save`/`delete`/`is_fresh`/`FRESHNESS_WINDOW` are used with the same names/types across Tasks 1→5. `pending_restore_game: Option<GameNumber>` defined in Task 3, consumed in Task 4. `decide_restore(&LinkSessionFile, OffsetDateTime, Mode) -> bool` defined and called consistently. `GameNumber = String`.

## Deviations

Executed on branch `feat/refbox/portal-link-restore` (off master `fde592e9`). Commits:
`4f08eaaa` T1, `55eca68b` T2, `d3edbe5c` T3, `67b2ab23` T4, `14fb9a08` T5, `6473fe34` review-fix.

1. **persist_link_session call sites (Task 2).** Hooked at the single `ApplyConfigPage`
   commit (right after `persist_config`, mod.rs ~2548) + the `PortalEvent` heartbeat
   (Task 5) — NOT inside each `apply_*` method, and NOT in `handle_game_end`. Reason:
   `apply_snapshot` updates `self.snapshot` *after* calling `handle_game_end`, so persisting
   there would capture the just-ended game; and `apply_snapshot` runs every clock tick, so
   persisting in it would cause excessive I/O. The ~5-min heartbeat re-derives the current
   game from the up-to-date snapshot, covering game advances. Cross-portal unlink (the two
   `set_current_event_id(None)` restart paths) relies on the startup mode-gate ignoring a
   cross-portal note rather than an explicit delete. Code-reviewer confirmed this is sound.

2. **Clippy gate timing.** `refbox` is a bin crate, so the Task-1 `pub` functions read as
   dead code until Task 3 referenced them. Ran the `clippy -D warnings` gate after Task 3/4
   (when all items are used) rather than per the plan's Task-1 step. Final `just check` green.

3. **Startup schedule fetch — race fix (code review, commit `6473fe34`).** The plan (Task 3)
   pushed `request_event_list()` and `request_schedule()` into the same startup batch. But
   `RecvSchedule` only re-selects the game when the event is already in `self.events`, so a
   schedule response winning the race would silently leave the restore dormant. Fixed by
   adding a one-shot `pending_restore_schedule: Option<EventId>` field set during restore and
   consumed in `RecvEventList` (after `self.events` is populated) to fire the schedule fetch.

4. **Minor refinements:** `EventId::from_full` returns `Result` → `.unwrap()` in tests;
   `GameNumber = String` (stored directly, no string-form workaround); `decide_restore` tests
   live in a new `#[cfg(test)] mod restore_tests` (mod.rs had no test module before).

**Verification status:** `just check` green (fmt, lint, 345 tests incl. 11 new, audit). Code
reviewed (1 Important issue found + fixed; minors are intentional/match `queue.rs`). End-to-end
restore behavior (Task 6 Step 3–4) needs an operator-driven walkthrough against a portal-linked
session — pending.
