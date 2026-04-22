# Portal Health Indicator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface UWH Portal submission health to the tournament operator so silent score-submission failures become visible, and persist failed submissions for automatic retry across restarts.

**Architecture:** A new `refbox/src/portal_manager/` module owns (1) a persisted on-disk JSON retry queue, (2) a background async task driving health checks and retries, and (3) a synchronous indicator-state getter consumed by the existing time-banner helper. A new clickable health tile on the left end of the time banner opens a Detail Page mirroring the Select Event layout; tapping red/yellow rows opens per-item action pages for stuck submissions and expired-token recovery.

> **Note on the conflict-handling design:** This plan reflects the refined design recorded in the [2026-04-21 amendment to ADR 011](../../decisions/011-portal-health-indicator.md#2026-04-21--conflict-handling-refined-after-api-verification). Because `uwh-common`'s portal client collapses all submission failure modes (409, 401, 500, network) into a single generic `Result<(), Box<dyn Error>>` error — and returns no portal-side score values on a 409 — the plan does not distinguish conflict from other failures. All non-success outcomes become a `Pending` queue item that auto-retries. Items that have been continuously retrying for 30 minutes escalate from yellow to red and surface a single "attention" action page offering `FORCE THIS GAME RESULT` and `DISCARD THIS SUBMISSION`. Token-expired (401) is still surfaced separately, because it is detected via a dedicated `verify_token` health probe rather than by introspecting submission errors.

**Reference documents:**
- Spec: `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`
- ADR: `docs/decisions/011-portal-health-indicator.md`

**Tech Stack:** Rust 2024 (MSRV 1.85), iced 0.13, tokio async runtime, reqwest HTTP (via `uwh-common`), serde_json, `time = "0.3"` (workspace-standard datetime crate; `chrono` is deliberately avoided — the refbox crate already uses `time`), Fluent `.ftl` translations.

**Branch:** Implementation happens on a new branch `feat/refbox/portal-health-indicator` cut from `master`. This plan document lives on `docs/workspace/backlog-adrs` alongside the spec.

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Create | `refbox/resources/UWH_Portal_Compact_Logo.png` | Logo asset (copied from user's Downloads) |
| Create | `refbox/src/portal_manager/mod.rs` | `PortalManager` struct, public API, subscription plumbing |
| Create | `refbox/src/portal_manager/queue.rs` | `QueueFile`, `QueuedItem`, load/save/atomic-write |
| Create | `refbox/src/portal_manager/health.rs` | Background health-check task, retry loop, 30-minute stuck threshold |
| Modify | `refbox/src/main.rs` | Register `mod portal_manager;` |
| Modify | `refbox/src/app/mod.rs` | Add `portal_manager` field; new `AppState` variants; reroute `handle_game_end()` |
| Modify | `refbox/src/app/message.rs` | Add new `Message` variants |
| Modify | `refbox/src/app/view_builders/shared_elements.rs` | Extend `make_game_time_button` with `portal_indicator` parameter |
| Modify | `refbox/src/app/view_builders/configuration.rs` (8 call sites) | Pass new parameter to `make_game_time_button` |
| Modify | `refbox/src/app/view_builders/list_selector.rs` | Pass new parameter to `make_game_time_button` |
| Modify | `refbox/src/app/view_builders/fouls.rs` | Pass new parameter to `make_game_time_button` |
| Modify | `refbox/src/app/view_builders/time_edit.rs` | Pass new parameter to `make_game_time_button` |
| Modify | `refbox/src/app/view_builders/confirmation.rs` (2 call sites + advisory banner) | Pass new parameter; add advisory banner |
| Modify | `refbox/src/app/view_builders/warnings.rs` | Pass new parameter |
| Modify | `refbox/src/app/view_builders/warnings_fouls_summary.rs` | Pass new parameter |
| Modify | `refbox/src/app/view_builders/score_edit.rs` | Pass new parameter |
| Modify | `refbox/src/app/view_builders/game_info.rs` | Pass new parameter |
| Modify | `refbox/src/app/view_builders/penalties.rs` | Pass new parameter |
| Modify | `refbox/src/app/view_builders/keypad_pages/mod.rs` | Pass new parameter |
| Modify | `refbox/src/app/view_builders/main_view.rs` | Pass new parameter |
| Create | `refbox/src/app/view_builders/portal_detail.rs` | Detail page view builder |
| Create | `refbox/src/app/view_builders/portal_attention_action.rs` | Stuck-item action page (FORCE / DISCARD / BACK) |
| Create | `refbox/src/app/view_builders/portal_token_expired_action.rs` | Token-expired action page |
| Modify | `refbox/src/app/view_builders/mod.rs` | `mod portal_detail; pub(super) use portal_detail::*;` (etc.) |
| Modify | `refbox/translations/en/refbox.ftl` and 13 others | New translation keys |
| Modify | `refbox/Cargo.toml` | No new deps expected (verify) |

---

## Execution Order

1. **Tasks 1–2** — Prerequisites (branch + asset + module scaffolding). Compiles after each.
2. **Tasks 3–10** — Backend (types → queue → health task → public API → 30-min escalation). Each task is fully unit-testable in isolation.
3. **Tasks 11–13** — Health tile on the time banner. UI appears but has no interactions beyond tap.
4. **Tasks 14–16** — Detail page + two action pages (attention, token-expired). UI now fully navigable.
5. **Task 17** — End-game advisory banner.
6. **Task 18** — Reroute `handle_game_end()` through `PortalManager`. Feature is now wired end-to-end.
7. **Tasks 19–20** — Config overrides for dev-portal testing.
8. **Task 21** — Translation keys added across all languages.
9. **Task 22** — Final `just check` + manual verification against `dev.uwhportal.com`.

---

### Task 1: Create the implementation branch and copy the logo asset

**Files:**
- Create: `refbox/resources/UWH_Portal_Compact_Logo.png`

- [ ] **Step 1: Cut the implementation branch from master**

```bash
git fetch origin master
git checkout -b feat/refbox/portal-health-indicator origin/master
```

Expected: on branch `feat/refbox/portal-health-indicator`, working tree clean.

- [ ] **Step 2: Copy the logo into `refbox/resources/`**

```bash
cp "/mnt/c/Users/Eric/Downloads/UWH Portal Compact Logo.png" \
   "refbox/resources/UWH_Portal_Compact_Logo.png"
```

- [ ] **Step 3: Verify the file was copied and is a valid PNG**

```bash
file refbox/resources/UWH_Portal_Compact_Logo.png
```

Expected: output like `PNG image data, ...`. If it fails, the source path is wrong — ask the user.

- [ ] **Step 4: Commit the asset alone**

```bash
git add refbox/resources/UWH_Portal_Compact_Logo.png
git commit -m "$(cat <<'EOF'
feat(refbox): add UWH Portal compact logo asset

Logo used by the new portal health indicator tile on the time
banner. Size 100 px wide target when rendered.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Create `portal_manager` module scaffolding with public types

**Files:**
- Create: `refbox/src/portal_manager/mod.rs`
- Create: `refbox/src/portal_manager/queue.rs`
- Create: `refbox/src/portal_manager/health.rs`
- Modify: `refbox/src/main.rs`

- [ ] **Step 1: Register the module in `main.rs`**

In `refbox/src/main.rs`, add `mod portal_manager;` alongside the other top-level module declarations (near `mod sound_controller;` and `mod tournament_manager;`):

```rust
mod app;
mod app_icon;
mod penalty_editor;
mod portal_manager;  // <-- new
mod sim_app;
mod sound_controller;
mod tournament_manager;

mod config;
```

- [ ] **Step 2: Create the three submodule files as empty stubs**

```bash
mkdir -p refbox/src/portal_manager
touch refbox/src/portal_manager/mod.rs
touch refbox/src/portal_manager/queue.rs
touch refbox/src/portal_manager/health.rs
```

- [ ] **Step 3: Fill `refbox/src/portal_manager/mod.rs` with the core public types and submodule declarations**

```rust
//! Portal Manager — tracks UWH Portal submission health, retries failures
//! from an on-disk queue, and surfaces problems to the operator.
//!
//! See `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`
//! and `docs/decisions/011-portal-health-indicator.md`.

// Scaffolding: types are defined up front and progressively wired up in Tasks
// 3–14 of the portal health indicator plan. This attribute is removed in
// Task 22 once all types have live callers.
#![allow(dead_code)]

pub mod health;
pub mod queue;

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Overall health state of the portal connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Last exchange succeeded; queue is empty; token is valid.
    Green,
    /// A check is in flight or the last call was slow-but-successful.
    Yellow,
    /// At least one item needs attention.
    Red,
}

/// Overlay icon currently showing on top of the status dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayState {
    /// Plain dot, no overlay.
    None,
    /// Green checkmark shown until `deadline` (instant).
    /// The view layer compares against `Instant::now()` to decide visibility.
    RecentSuccess,
    /// Red exclamation mark (persists while any item needs attention).
    AttentionNeeded,
}

/// Combined state consumed by the time-banner helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalIndicatorState {
    pub health: HealthState,
    pub overlay: OverlayState,
}

impl Default for PortalIndicatorState {
    fn default() -> Self {
        Self {
            health: HealthState::Green,
            overlay: OverlayState::None,
        }
    }
}

/// Unique identifier for a queued item (event_id + game_number).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId {
    pub event_id: String,
    pub game_number: String,
}

/// Event emitted by the portal manager's background task for the iced
/// Subscription to convert into a `Message`.
#[derive(Debug, Clone)]
pub enum PortalEvent {
    HealthChanged(HealthState),
    OverlayChanged(OverlayState),
    ItemAdded(ItemId),
    ItemResolved(ItemId),
    ItemUpdated(ItemId),
}

/// Stub for the manager struct — fleshed out in later tasks.
pub struct PortalManager {
    // Fields populated in Task 6.
    indicator_state: PortalIndicatorState,
    _recent_successes_instant: Option<Instant>, // silence unused-field warning
}

impl PortalManager {
    pub fn indicator_state(&self) -> PortalIndicatorState {
        self.indicator_state
    }
}
```

- [ ] **Step 4: Fill `refbox/src/portal_manager/queue.rs` with an empty module header for now**

```rust
//! On-disk persistence for the portal retry queue.
//! See `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`.

// Types and load/save functions defined in Task 3.
```

- [ ] **Step 5: Fill `refbox/src/portal_manager/health.rs` with an empty module header**

```rust
//! Background health-check task and retry loop.
//! See `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`.

// Implementation in Tasks 7–10.
```

- [ ] **Step 6: Verify the workspace compiles**

```bash
just check
```

Expected: all green. Because `refbox` is a binary crate, `pub` types with no in-crate caller still trigger `dead_code` warnings, and CI runs with `-D warnings`. The module-level `#![allow(dead_code)]` at the top of `mod.rs` silences them collectively while the types are being wired up in later tasks; the comment next to the attribute is the "explicit discussion and justification" required by `.claude/rules/rust.md`. Task 22 removes the attribute once all types have live callers.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/main.rs refbox/src/portal_manager/
git commit -m "$(cat <<'EOF'
feat(refbox): scaffold portal_manager module with public types

Introduces the portal_manager module with its three submodules
(mod.rs, queue.rs, health.rs) and the public types consumed by the
UI layer: HealthState, OverlayState, PortalIndicatorState, ItemId,
PortalEvent.

Implementation of each submodule follows in later commits.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Queue file types and serde round-trip

**Files:**
- Modify: `refbox/src/portal_manager/queue.rs`

- [ ] **Step 1: Define the queue file types**

Replace the contents of `refbox/src/portal_manager/queue.rs` with:

```rust
//! On-disk persistence for the portal retry queue.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

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
}
```

- [ ] **Step 2: Run the tests to verify they pass**

```bash
cargo test -p refbox portal_manager::queue
```

Expected: three tests pass.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/portal_manager/queue.rs
git commit -m "$(cat <<'EOF'
feat(refbox): add queue file types with serde round-trip

Defines QueueFile envelope (version + items) and QueuedItem
per-record shape. There is no per-item state enum because the
portal client collapses all failure modes into a single generic
error; stuck-ness is derived from queued_at and token problems
are tracked globally (see ADR 011 amendment). Tests cover
empty-queue round-trip and populated-queue round-trip.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Queue file atomic load and save

**Files:**
- Modify: `refbox/src/portal_manager/queue.rs`

- [ ] **Step 1: Add tests for load/save**

Append the following to the `tests` module in `refbox/src/portal_manager/queue.rs`:

```rust
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
            entries.iter().any(|n| n.starts_with("portal_queue.corrupt")),
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
```

- [ ] **Step 2: Add the `tempfile` dev-dependency**

Edit `refbox/Cargo.toml` (under `[dev-dependencies]`). If `tempfile` is not already present:

```toml
[dev-dependencies]
# ... existing ...
tempfile = "3"
```

- [ ] **Step 3: Run the tests to verify they fail with "function not found"**

```bash
cargo test -p refbox portal_manager::queue::load_save_tests
```

Expected: compile error — `load_or_empty` and `save` do not exist yet.

- [ ] **Step 4: Implement `load_or_empty` and `save`**

Add to `refbox/src/portal_manager/queue.rs` (above the `tests` module):

```rust
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use time::macros::format_description;

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
    let fmt = format_description!(
        "[year][month][day]T[hour][minute][second]Z"
    );
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
        serde_json::to_writer(&f, q).map_err(|e| std::io::Error::other(e))?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, queue_path(dir))?;
    Ok(())
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test -p refbox portal_manager::queue
```

Expected: all tests pass (three original + four load/save tests).

- [ ] **Step 6: Commit**

```bash
git add refbox/Cargo.toml refbox/src/portal_manager/queue.rs
git commit -m "$(cat <<'EOF'
feat(refbox): add atomic load/save for portal queue file

load_or_empty loads portal_queue.json if present and parses cleanly,
returns empty QueueFile otherwise. A corrupted or unknown-version
file is renamed to portal_queue.corrupt.<timestamp>.json and logged
at error level, allowing the app to start fresh.

save writes via temp-file + rename for crash-safe atomic updates.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: `PortalManager` state and indicator-state computation

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs`

- [ ] **Step 1: Add tests for state computation**

The indicator computation must differentiate between "queued but still
retrying silently" (Yellow) and "stuck long enough that the operator
needs to step in" (Red). Stuck-ness is defined as **30 minutes since
`queued_at`** (see ADR 011 amendment). The token-problem flag is
separate and always contributes Red.

Append to `refbox/src/portal_manager/mod.rs` (at the bottom):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal_manager::queue::{QueueFile, QueuedItem};
    use time::{Duration as TimeDuration, OffsetDateTime};

    fn mk_young_item() -> QueuedItem {
        QueuedItem {
            id: ItemId {
                event_id: "e".into(),
                game_number: "G1".into(),
            },
            black_score: 0,
            white_score: 0,
            stats: "{}".into(),
            queued_at: OffsetDateTime::now_utc(),
            attempts: 0,
            last_attempt_at: None,
            force: false,
        }
    }

    fn mk_stuck_item() -> QueuedItem {
        let mut it = mk_young_item();
        it.queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(31);
        it
    }

    #[test]
    fn empty_queue_and_no_problems_is_green() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        assert_eq!(m.indicator_state().health, HealthState::Green);
        assert_eq!(m.indicator_state().overlay, OverlayState::None);
    }

    #[test]
    fn young_pending_item_is_yellow_no_overlay() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_young_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
        assert_eq!(m.indicator_state().overlay, OverlayState::None);
    }

    #[test]
    fn stuck_item_is_red_with_attention_overlay() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_stuck_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert_eq!(m.indicator_state().overlay, OverlayState::AttentionNeeded);
    }

    #[test]
    fn token_known_problem_is_red_with_attention_overlay() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, true);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert_eq!(m.indicator_state().overlay, OverlayState::AttentionNeeded);
    }

    #[test]
    fn health_check_in_flight_is_yellow() {
        let m = PortalManager::new_for_test(QueueFile::empty(), true, false);
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
    }

    #[test]
    fn recent_success_overlay_is_suppressed_by_stuck_item() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_stuck_item()],
        };
        let mut m = PortalManager::new_for_test(q, false, false);
        m.mark_recent_success();
        assert_eq!(m.indicator_state().overlay, OverlayState::AttentionNeeded);
    }

    #[test]
    fn recent_success_shows_when_queue_empty() {
        let mut m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        m.mark_recent_success();
        assert_eq!(m.indicator_state().overlay, OverlayState::RecentSuccess);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p refbox portal_manager::tests
```

Expected: compile error — `new_for_test`, `mark_recent_success`, and the 30-minute stuck-item check don't exist yet.

- [ ] **Step 3: Replace the stub `PortalManager` in `mod.rs` with the state-computing struct**

Replace the existing `PortalManager` definition (and its `impl`) in `refbox/src/portal_manager/mod.rs` with:

```rust
use std::time::{Duration, Instant};

use time::{Duration as TimeDuration, OffsetDateTime};

use crate::portal_manager::queue::{QueueFile, QueuedItem};

/// How long the green-checkmark overlay remains visible after a successful submit.
pub const RECENT_SUCCESS_DURATION: Duration = Duration::from_secs(10);

/// How long an item may sit in the queue before it escalates from
/// Yellow (silently retrying) to Red (operator needs to decide).
/// See ADR 011 amendment (2026-04-21).
pub const STUCK_THRESHOLD: TimeDuration = TimeDuration::minutes(30);

pub fn is_item_stuck(item: &QueuedItem, now: OffsetDateTime) -> bool {
    (now - item.queued_at) >= STUCK_THRESHOLD
}

pub struct PortalManager {
    queue: QueueFile,
    check_in_flight: bool,
    /// Set true when the last `verify_token` probe failed. Cleared on
    /// the next successful probe or by `token_refreshed()` after the
    /// operator re-logs-in.
    token_known_problem: bool,
    recent_success_until: Option<Instant>,
    indicator_state: PortalIndicatorState,
}

impl PortalManager {
    /// Test-only constructor. The production constructor is introduced in Task 6.
    #[cfg(test)]
    pub(crate) fn new_for_test(
        queue: QueueFile,
        check_in_flight: bool,
        token_known_problem: bool,
    ) -> Self {
        let mut m = Self {
            queue,
            check_in_flight,
            token_known_problem,
            recent_success_until: None,
            indicator_state: PortalIndicatorState::default(),
        };
        m.recompute_indicator();
        m
    }

    pub fn indicator_state(&self) -> PortalIndicatorState {
        self.indicator_state
    }

    pub fn mark_recent_success(&mut self) {
        self.recent_success_until = Some(Instant::now() + RECENT_SUCCESS_DURATION);
        self.recompute_indicator();
    }

    fn has_stuck_items(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        self.queue.items.iter().any(|it| is_item_stuck(it, now))
    }

    fn has_any_queue_items(&self) -> bool {
        !self.queue.items.is_empty()
    }

    fn needs_attention(&self) -> bool {
        self.token_known_problem || self.has_stuck_items()
    }

    fn recompute_indicator(&mut self) {
        let health = if self.needs_attention() {
            HealthState::Red
        } else if self.check_in_flight || self.has_any_queue_items() {
            HealthState::Yellow
        } else {
            HealthState::Green
        };

        let overlay = if self.needs_attention() {
            OverlayState::AttentionNeeded
        } else if self
            .recent_success_until
            .map(|t| Instant::now() < t)
            .unwrap_or(false)
        {
            OverlayState::RecentSuccess
        } else {
            OverlayState::None
        };

        self.indicator_state = PortalIndicatorState { health, overlay };
    }
}
```

Remove the unused `_recent_successes_instant` field and the stub constructor from Task 2.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p refbox portal_manager
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "$(cat <<'EOF'
feat(refbox): compute portal indicator state from queue + flags

PortalManager owns the QueueFile, a check_in_flight flag, a
token_known_problem flag, and an optional recent_success_until
deadline. The indicator_state is recomputed from those inputs:

- Red + AttentionNeeded: token problem OR any item queued for ≥ 30
  minutes (the "stuck" threshold from the ADR 011 amendment).
- Yellow: a queued item is still within the 30-minute window
  (retrying silently in the background) OR a verify_token probe
  is in flight.
- Green: nothing pending, nothing stuck, no token problem.

The recent-success overlay is suppressed whenever an attention
item is present (the "mutex" rule from the spec).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Public API method stubs and the `PortalEvent` mpsc channel

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs`

- [ ] **Step 1: Introduce the production constructor and method stubs**

Add (below the `#[cfg(test)]` constructor in `refbox/src/portal_manager/mod.rs`):

```rust
use tokio::sync::mpsc;

use crate::portal_manager::queue::{self, QueuedItem};

/// Channel buffer for `PortalEvent`s from the background task to the UI subscription.
const EVENT_CHANNEL_BUFFER: usize = 64;

impl PortalManager {
    /// Construct a new PortalManager. Loads the queue from `config_dir`
    /// (starting fresh if the file is missing or corrupted) and prepares
    /// the mpsc channel that the background task will send events on.
    ///
    /// Returns (manager, event_receiver). The receiver is fed into an iced
    /// Subscription by the app.
    pub fn new(config_dir: &std::path::Path) -> std::io::Result<(Self, mpsc::Receiver<PortalEvent>)> {
        let queue = queue::load_or_empty(config_dir)?;
        let (_tx, rx) = mpsc::channel(EVENT_CHANNEL_BUFFER);
        let mut m = Self {
            queue,
            check_in_flight: false,
            token_known_problem: false,
            recent_success_until: None,
            indicator_state: PortalIndicatorState::default(),
            _event_tx: _tx,
            config_dir: config_dir.to_path_buf(),
        };
        m.recompute_indicator();
        Ok((m, rx))
    }

    /// Enqueue a game-end submission and trigger an immediate attempt.
    /// Writes to disk *before* attempting the submit so a crash between
    /// write and send does not lose the score.
    pub fn enqueue_game_end(
        &mut self,
        event_id: String,
        game_number: String,
        black_score: u8,
        white_score: u8,
        stats: String,
    ) -> std::io::Result<()> {
        let item = QueuedItem {
            id: ItemId {
                event_id,
                game_number,
            },
            black_score,
            white_score,
            stats,
            queued_at: OffsetDateTime::now_utc(),
            attempts: 0,
            last_attempt_at: None,
            force: false,
        };
        self.queue.items.push(item);
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        Ok(())
    }

    /// Operator tapped FORCE THIS GAME RESULT on the attention action page.
    /// Sets the item's `force` flag so the next submit sends `force=true`,
    /// resets the attempt counter and the last-attempt timestamp so the
    /// background task retries immediately, and resets `queued_at` to
    /// "now" so the item is no longer considered stuck (the operator
    /// has restarted its 30-minute clock by making a decision).
    pub fn force_submit(&mut self, id: &ItemId) -> std::io::Result<()> {
        if let Some(item) = self.find_mut(id) {
            item.force = true;
            item.attempts = 0;
            item.last_attempt_at = None;
            item.queued_at = OffsetDateTime::now_utc();
            queue::save(&self.config_dir, &self.queue)?;
        }
        self.recompute_indicator();
        Ok(())
    }

    /// Operator tapped DISCARD THIS SUBMISSION on the attention action page.
    /// Removes the item from the queue without submitting. Whatever the
    /// portal currently has for that game stands.
    pub fn discard(&mut self, id: &ItemId) -> std::io::Result<()> {
        self.queue.items.retain(|it| it.id != *id);
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        Ok(())
    }

    /// Called after a successful portal re-login. Clears the global
    /// token-problem flag and resets every queued item's attempt
    /// counter and last-attempt timestamp so the background task
    /// retries them immediately on its next tick.
    pub fn token_refreshed(&mut self) -> std::io::Result<()> {
        self.token_known_problem = false;
        for item in &mut self.queue.items {
            item.attempts = 0;
            item.last_attempt_at = None;
        }
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        Ok(())
    }

    /// Force an immediate health check (fires verify_token out-of-band).
    /// Implementation wired in Task 8.
    pub fn verify_now(&mut self) {
        // TODO(Task 8): send a request to the background task to verify now.
    }

    fn find_mut(&mut self, id: &ItemId) -> Option<&mut QueuedItem> {
        self.queue.items.iter_mut().find(|it| it.id == *id)
    }
}
```

Add the two new fields to the `PortalManager` struct (note the
`token_known_problem` field was already added in Task 5):

```rust
pub struct PortalManager {
    queue: QueueFile,
    check_in_flight: bool,
    token_known_problem: bool,
    recent_success_until: Option<Instant>,
    indicator_state: PortalIndicatorState,
    _event_tx: mpsc::Sender<PortalEvent>,   // <-- new
    config_dir: std::path::PathBuf,          // <-- new
}
```

Update `new_for_test` to also populate these fields (use a dropped sender and a temp dir path):

```rust
#[cfg(test)]
pub(crate) fn new_for_test(
    queue: QueueFile,
    check_in_flight: bool,
    token_known_problem: bool,
) -> Self {
    let (tx, _rx) = mpsc::channel(1);
    let mut m = Self {
        queue,
        check_in_flight,
        token_known_problem,
        recent_success_until: None,
        indicator_state: PortalIndicatorState::default(),
        _event_tx: tx,
        config_dir: std::env::temp_dir(),
    };
    m.recompute_indicator();
    m
}
```

- [ ] **Step 2: Add tests for the public API**

Append to the `#[cfg(test)] mod tests` block in `refbox/src/portal_manager/mod.rs`:

```rust
#[test]
fn enqueue_game_end_appends_item_and_turns_yellow() {
    // Use a temp dir so the save succeeds.
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path()).unwrap();

    assert_eq!(m.indicator_state().health, HealthState::Green);

    m.enqueue_game_end(
        "event".into(),
        "G1".into(),
        3,
        2,
        "{}".into(),
    )
    .unwrap();

    // Fresh item: Yellow (retrying silently), not Red.
    assert_eq!(m.indicator_state().health, HealthState::Yellow);
    assert_eq!(m.queue.items.len(), 1);
}

#[test]
fn discard_removes_item_and_returns_to_green() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path()).unwrap();
    m.enqueue_game_end("event".into(), "G1".into(), 0, 0, "{}".into())
        .unwrap();

    let id = m.queue.items[0].id.clone();
    m.discard(&id).unwrap();

    assert_eq!(m.indicator_state().health, HealthState::Green);
    assert!(m.queue.items.is_empty());
}

#[test]
fn force_submit_flags_force_and_resets_attempt_counters() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path()).unwrap();
    m.enqueue_game_end("event".into(), "G1".into(), 0, 0, "{}".into())
        .unwrap();
    let id = m.queue.items[0].id.clone();

    // Pretend the item has been retrying for a while.
    m.queue.items[0].attempts = 7;
    m.queue.items[0].last_attempt_at = Some(OffsetDateTime::now_utc());

    m.force_submit(&id).unwrap();

    assert!(m.queue.items[0].force);
    assert_eq!(m.queue.items[0].attempts, 0);
    assert!(m.queue.items[0].last_attempt_at.is_none());
}

#[test]
fn token_refreshed_clears_flag_and_resets_queue_items() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path()).unwrap();
    m.enqueue_game_end("event".into(), "G1".into(), 0, 0, "{}".into())
        .unwrap();
    m.token_known_problem = true;
    m.queue.items[0].attempts = 4;
    m.queue.items[0].last_attempt_at = Some(OffsetDateTime::now_utc());

    m.token_refreshed().unwrap();

    assert!(!m.token_known_problem);
    assert_eq!(m.queue.items[0].attempts, 0);
    assert!(m.queue.items[0].last_attempt_at.is_none());
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p refbox portal_manager
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "$(cat <<'EOF'
feat(refbox): add PortalManager public API and event channel

Introduces PortalManager::new() (loads queue from disk) and the
public methods: enqueue_game_end, force_submit, discard,
token_refreshed, verify_now (stub). All mutations save to disk
before updating indicator state, so a crash after a push but
before the next event cycle does not lose data.

No conflict-specific resolve method — per the ADR 011 amendment,
conflicts are indistinguishable from other failures so the operator
resolves all stuck items via force_submit or discard.

The PortalEvent mpsc channel is plumbed but not yet driven — the
background task that consumes the receiver lands in Task 7.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Background health-check task with verify_token

**Files:**
- Modify: `refbox/src/portal_manager/health.rs`
- Modify: `refbox/src/portal_manager/mod.rs`

This task introduces the async task that polls `verify_token` on the 5-min / 15-sec cadence and the retry loop for pending items. For testability, the implementation is split so the decision logic is synchronous and unit-testable, while the I/O and timer are thin wrappers.

- [ ] **Step 1: Define the decision logic types in `health.rs`**

Replace the contents of `refbox/src/portal_manager/health.rs` with:

```rust
//! Background health-check task and retry loop.
//!
//! The decision logic (when to fire a health check, when to retry an item,
//! what cadence to use) is encapsulated in `HealthDecisionState` and is
//! unit-testable without any async runtime or I/O.

use std::time::{Duration, Instant};

use super::HealthState;

pub const GREEN_CADENCE: Duration = Duration::from_secs(5 * 60);
pub const DEGRADED_CADENCE: Duration = Duration::from_secs(15);

/// What the background loop should do on its next wakeup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextAction {
    /// Wait until `at` and then re-evaluate.
    SleepUntil(Instant),
    /// Fire a verify_token health check.
    HealthCheck,
    /// Retry a queued item (by index into the queue's `items` vec).
    RetryItem(usize),
}

#[derive(Debug, Clone)]
pub struct HealthDecisionState {
    pub last_successful_interaction: Option<Instant>,
    pub current_health: HealthState,
}

impl HealthDecisionState {
    pub fn next_cadence(&self) -> Duration {
        match self.current_health {
            HealthState::Green => GREEN_CADENCE,
            HealthState::Yellow | HealthState::Red => DEGRADED_CADENCE,
        }
    }

    /// Has the cadence elapsed since the last successful interaction?
    pub fn is_health_check_due(&self, now: Instant) -> bool {
        match self.last_successful_interaction {
            None => true, // First ever check — fire immediately.
            Some(last) => now.saturating_duration_since(last) >= self.next_cadence(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_cadence_is_five_minutes() {
        let s = HealthDecisionState {
            last_successful_interaction: Some(Instant::now()),
            current_health: HealthState::Green,
        };
        assert_eq!(s.next_cadence(), GREEN_CADENCE);
    }

    #[test]
    fn yellow_and_red_cadence_is_fifteen_seconds() {
        for h in [HealthState::Yellow, HealthState::Red] {
            let s = HealthDecisionState {
                last_successful_interaction: Some(Instant::now()),
                current_health: h,
            };
            assert_eq!(s.next_cadence(), DEGRADED_CADENCE);
        }
    }

    #[test]
    fn first_ever_check_is_due_immediately() {
        let s = HealthDecisionState {
            last_successful_interaction: None,
            current_health: HealthState::Green,
        };
        assert!(s.is_health_check_due(Instant::now()));
    }

    #[test]
    fn green_check_not_due_if_cadence_not_elapsed() {
        let now = Instant::now();
        let s = HealthDecisionState {
            last_successful_interaction: Some(now),
            current_health: HealthState::Green,
        };
        assert!(!s.is_health_check_due(now + Duration::from_secs(60)));
    }

    #[test]
    fn green_check_due_after_cadence() {
        let now = Instant::now();
        let s = HealthDecisionState {
            last_successful_interaction: Some(now),
            current_health: HealthState::Green,
        };
        assert!(s.is_health_check_due(now + GREEN_CADENCE));
    }

    #[test]
    fn degraded_check_due_after_fifteen_seconds() {
        let now = Instant::now();
        let s = HealthDecisionState {
            last_successful_interaction: Some(now),
            current_health: HealthState::Red,
        };
        assert!(s.is_health_check_due(now + Duration::from_secs(15)));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p refbox portal_manager::health
```

Expected: six tests pass.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/portal_manager/health.rs
git commit -m "$(cat <<'EOF'
feat(refbox): add health-check cadence decision logic

HealthDecisionState encapsulates the 5-min-green / 15-sec-degraded
cadence rule and exposes pure functions for unit testing. The async
task that consumes this state and actually calls verify_token lives
in a separate commit so the timing can be exercised without a Tokio
runtime.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Retry-loop decision logic and integration into `PortalManager`

**Files:**
- Modify: `refbox/src/portal_manager/health.rs`
- Modify: `refbox/src/portal_manager/mod.rs`

- [ ] **Step 1: Add retry decision tests to `health.rs`**

The retry rule is: an item is eligible to auto-retry if it is **not
stuck** (i.e. less than 30 minutes have passed since it was queued)
**and** the 15-second cadence has elapsed since the last attempt.
There is no attempt cap — retries continue at the 15-second cadence
for the full 30-minute window. Once the item is stuck, auto-retry
stops and the operator must decide (Force / Discard).

Append to the `tests` module in `refbox/src/portal_manager/health.rs`:

```rust
use super::super::queue::QueuedItem;
use super::super::ItemId;
use time::{Duration as TimeDuration, OffsetDateTime};

fn mk_queue_item(attempts: u32) -> QueuedItem {
    QueuedItem {
        id: ItemId {
            event_id: "e".into(),
            game_number: "G1".into(),
        },
        black_score: 0,
        white_score: 0,
        stats: "{}".into(),
        queued_at: OffsetDateTime::now_utc(),
        attempts,
        last_attempt_at: None,
        force: false,
    }
}

#[test]
fn young_item_with_zero_attempts_is_eligible_for_retry() {
    let item = mk_queue_item(0);
    assert!(is_item_retry_eligible(&item, OffsetDateTime::now_utc()));
}

#[test]
fn stuck_item_is_not_eligible_for_auto_retry() {
    let mut item = mk_queue_item(4);
    // Queued 31 minutes ago — past the 30-minute stuck threshold.
    item.queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(31);
    assert!(!is_item_retry_eligible(&item, OffsetDateTime::now_utc()));
}

#[test]
fn item_with_recent_attempt_respects_cadence() {
    let mut item = mk_queue_item(1);
    let now = OffsetDateTime::now_utc();
    item.last_attempt_at = Some(now);
    // 5 seconds later — still within the 15s cadence.
    assert!(!is_item_retry_eligible(&item, now + TimeDuration::seconds(5)));
    // 15 seconds later — eligible again.
    assert!(is_item_retry_eligible(&item, now + TimeDuration::seconds(15)));
}
```

- [ ] **Step 2: Implement `is_item_retry_eligible`**

Add to `refbox/src/portal_manager/health.rs` above the `tests` module:

```rust
use time::{Duration as TimeDuration, OffsetDateTime};

use super::is_item_stuck;
use super::queue::QueuedItem;

pub fn is_item_retry_eligible(item: &QueuedItem, now: OffsetDateTime) -> bool {
    if is_item_stuck(item, now) {
        return false; // Stuck items wait for operator action.
    }
    match item.last_attempt_at {
        None => true,
        Some(last) => (now - last) >= TimeDuration::seconds(15),
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p refbox portal_manager::health
```

Expected: six original + three new tests all pass.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/portal_manager/health.rs
git commit -m "$(cat <<'EOF'
feat(refbox): add retry-eligibility rules for queued items

is_item_retry_eligible returns true when the item is not stuck
(queued less than 30 minutes ago) and 15 seconds have elapsed
since the last attempt. Stuck items are not auto-retry-eligible;
they wait for the operator to choose Force or Discard on the
attention action page.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Async background task driving health checks and retries

**Files:**
- Modify: `refbox/src/portal_manager/health.rs`
- Modify: `refbox/src/portal_manager/mod.rs`

This task adds the Tokio background task that owns a `PortalClient`, fires `verify_token` on cadence, and fires `post_game_scores` / `post_game_stats` for eligible items. Because the task communicates with the `PortalManager` via an mpsc channel of commands (and sends events the other way), it is testable by driving commands into a stand-in.

- [ ] **Step 1: Add the command enum and task loop to `health.rs`**

Append to `refbox/src/portal_manager/health.rs`:

```rust
use tokio::sync::mpsc;

use super::PortalEvent;

/// Commands from the UI side of PortalManager to the background task.
#[derive(Debug, Clone)]
pub enum PortalCommand {
    /// Trigger an immediate verify_token check.
    VerifyNow,
    /// An item was enqueued; attempt immediate submit.
    ItemEnqueued(super::ItemId),
    /// Retry requested for a specific item (resets cadence).
    RetryItem(super::ItemId),
    /// Shut down the background task.
    Shutdown,
}

pub struct BackgroundTaskHandle {
    pub command_tx: mpsc::Sender<PortalCommand>,
    pub event_rx: mpsc::Receiver<PortalEvent>,
}

/// Spawn the background portal task. The caller holds `command_tx` to
/// drive it and `event_rx` to consume state updates.
///
/// The actual HTTP calls are abstracted behind `PortalTaskIo` so the
/// task can be unit-tested with an in-process fake.
pub fn spawn(io: impl PortalTaskIo + Send + 'static) -> BackgroundTaskHandle {
    let (command_tx, command_rx) = mpsc::channel(64);
    let (event_tx, event_rx) = mpsc::channel(64);
    tokio::spawn(run_task(io, command_rx, event_tx));
    BackgroundTaskHandle {
        command_tx,
        event_rx,
    }
}

/// Abstraction over the portal HTTP calls. Production impl wraps
/// `uwh_common::uwhportal::UwhPortalClient`; tests supply a fake.
#[async_trait::async_trait]
pub trait PortalTaskIo {
    async fn verify_token(&self) -> Result<(), PortalCallError>;
    async fn post_scores(
        &self,
        item: &super::queue::QueuedItem,
    ) -> Result<(), PortalCallError>;
    async fn post_stats(
        &self,
        item: &super::queue::QueuedItem,
    ) -> Result<(), PortalCallError>;
}

#[derive(Debug)]
pub enum PortalCallError {
    /// The portal call did not succeed. We cannot distinguish a conflict
    /// (409), token-expiry (401), or network/5xx failure from the current
    /// uwh-common API — all non-success outcomes collapse to this one
    /// variant. See ADR 011 amendment (2026-04-21) for the rationale.
    Failed(String),
}

async fn run_task(
    io: impl PortalTaskIo,
    mut command_rx: mpsc::Receiver<PortalCommand>,
    event_tx: mpsc::Sender<PortalEvent>,
) {
    let _ = (&io, &mut command_rx, &event_tx);
    // The full task loop is implemented in the next step; this stub
    // exits immediately to satisfy the compiler during scaffolding.
}
```

- [ ] **Step 2: Add the `async-trait` dev dependency if not present**

Check `refbox/Cargo.toml`:

```bash
grep -n async-trait refbox/Cargo.toml || echo "needs adding"
```

If missing, add under `[dependencies]`:

```toml
async-trait = "0.1"
```

- [ ] **Step 3: Run `just check` — expect it to compile but do nothing at runtime**

```bash
just check
```

Expected: all green. No behavioural change yet.

- [ ] **Step 4: Implement the task loop**

Replace the `run_task` stub in `refbox/src/portal_manager/health.rs` with:

```rust
async fn run_task(
    io: impl PortalTaskIo,
    mut command_rx: mpsc::Receiver<PortalCommand>,
    event_tx: mpsc::Sender<PortalEvent>,
) {
    use tokio::time::{sleep, Duration as TokioDuration};

    let mut last_success: Option<Instant> = None;
    let mut current_health = HealthState::Green;

    loop {
        // Determine sleep duration: the smaller of (a) the next cadence
        // boundary, (b) 2 seconds to allow frequent polling.
        let sleep_for = TokioDuration::from_millis(2_000);

        tokio::select! {
            _ = sleep(sleep_for) => {
                // Periodic wakeup: fire a health check if cadence is due.
                let state = HealthDecisionState {
                    last_successful_interaction: last_success,
                    current_health,
                };
                if state.is_health_check_due(Instant::now()) {
                    let _ = event_tx
                        .send(PortalEvent::HealthChanged(HealthState::Yellow))
                        .await;
                    match io.verify_token().await {
                        Ok(()) => {
                            last_success = Some(Instant::now());
                            current_health = HealthState::Green;
                            let _ = event_tx
                                .send(PortalEvent::HealthChanged(HealthState::Green))
                                .await;
                        }
                        Err(_) => {
                            current_health = HealthState::Red;
                            let _ = event_tx
                                .send(PortalEvent::HealthChanged(HealthState::Red))
                                .await;
                        }
                    }
                }
            }
            cmd = command_rx.recv() => {
                match cmd {
                    Some(PortalCommand::Shutdown) | None => break,
                    Some(PortalCommand::VerifyNow) => {
                        // Force last_success to expire so next tick fires a check.
                        last_success = None;
                    }
                    Some(PortalCommand::ItemEnqueued(_)) | Some(PortalCommand::RetryItem(_)) => {
                        // Retry logic detailed in the full-integration task
                        // (Task 10). For now these commands trigger a
                        // health-check-due condition so the next tick does work.
                        last_success = None;
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 5: Write an integration test driving the task with a fake IO**

Append to the `tests` module in `refbox/src/portal_manager/health.rs`:

```rust
use std::sync::{Arc, Mutex};

struct FakeIo {
    verify_results: Mutex<Vec<Result<(), PortalCallError>>>,
    verify_count: Arc<std::sync::atomic::AtomicU32>,
}

#[async_trait::async_trait]
impl PortalTaskIo for FakeIo {
    async fn verify_token(&self) -> Result<(), PortalCallError> {
        self.verify_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut v = self.verify_results.lock().unwrap();
        if v.is_empty() {
            Ok(())
        } else {
            v.remove(0)
        }
    }
    async fn post_scores(&self, _: &super::queue::QueuedItem) -> Result<(), PortalCallError> {
        Ok(())
    }
    async fn post_stats(&self, _: &super::queue::QueuedItem) -> Result<(), PortalCallError> {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn verify_now_triggers_immediate_health_check() {
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let io = FakeIo {
        verify_results: Mutex::new(vec![Ok(())]),
        verify_count: count.clone(),
    };
    let handle = spawn(io);

    handle
        .command_tx
        .send(PortalCommand::VerifyNow)
        .await
        .unwrap();

    // Advance the paused clock past the 2-second poll interval so the
    // task wakes and fires a health check.
    tokio::time::advance(Duration::from_secs(3)).await;
    // Yield to let the task drain the channel.
    tokio::task::yield_now().await;

    assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 1);

    handle
        .command_tx
        .send(PortalCommand::Shutdown)
        .await
        .unwrap();
}
```

- [ ] **Step 6: Run the test**

```bash
cargo test -p refbox portal_manager::health::tests::verify_now_triggers_immediate_health_check
```

Expected: pass. If the test is flaky because of timing, inspect and widen the timing tolerance rather than disabling.

- [ ] **Step 7: Commit**

```bash
git add refbox/Cargo.toml refbox/src/portal_manager/health.rs
git commit -m "$(cat <<'EOF'
feat(refbox): spawn background portal task with PortalTaskIo trait

Introduces the async task loop that drives verify_token calls on
the 5-min / 15-sec cadence and surfaces HealthState changes via
PortalEvent. The HTTP surface is abstracted behind PortalTaskIo so
the task is unit-testable with an in-process fake.

Post-score and post-stats calls are stubbed here and wired into the
retry loop in the next task. The VerifyNow command from the UI
(from the future verify_now() method) forces an immediate cadence
expiration.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Retry loop and item-state integration

**Files:**
- Modify: `refbox/src/portal_manager/health.rs`
- Modify: `refbox/src/portal_manager/mod.rs`

- [ ] **Step 1: Expand the task loop to drain pending items**

This task connects the retry eligibility logic from Task 8 with the async loop from Task 9. The loop maintains a snapshot of the queue it receives via a new command `PortalCommand::QueueUpdated(QueueFile)` and attempts eligible items in turn.

**Test-coverage discipline:** any task that adds new behaviour to the async loop must include tests that exercise the new path. The implementer for this task should add async tests (using `#[tokio::test(flavor = "current_thread", start_paused = true)]`) that cover, at minimum: (a) a successful retry emits `ItemResolved` and drives `post_scores` then `post_stats`, (b) a `post_scores` failure emits `ItemUpdated` and skips `post_stats`, and (c) a `QueueUpdated` command replaces the snapshot before the next tick.

Replace `run_task` with the expanded version that also attempts queue items:

```rust
async fn run_task(
    io: impl PortalTaskIo,
    mut command_rx: mpsc::Receiver<PortalCommand>,
    event_tx: mpsc::Sender<PortalEvent>,
) {
    use tokio::time::{sleep, Duration as TokioDuration};
    use super::queue::QueueFile;

    let mut last_success: Option<Instant> = None;
    let mut current_health = HealthState::Green;
    let mut queue_snapshot = QueueFile::empty();

    loop {
        let sleep_for = TokioDuration::from_millis(2_000);

        tokio::select! {
            _ = sleep(sleep_for) => {
                // Tick: try each eligible queued item, then health-check if due.
                let now = time::OffsetDateTime::now_utc();
                for item in &queue_snapshot.items {
                    if is_item_retry_eligible(item, now)
                        && attempt_item(&io, item, &event_tx).await
                    {
                        last_success = Some(Instant::now());
                    }
                }

                let state = HealthDecisionState {
                    last_successful_interaction: last_success,
                    current_health,
                };
                if state.is_health_check_due(Instant::now()) {
                    let _ = event_tx
                        .send(PortalEvent::HealthChanged(HealthState::Yellow))
                        .await;
                    match io.verify_token().await {
                        Ok(()) => {
                            last_success = Some(Instant::now());
                            current_health = HealthState::Green;
                            let _ = event_tx
                                .send(PortalEvent::HealthChanged(HealthState::Green))
                                .await;
                        }
                        Err(_) => {
                            current_health = HealthState::Red;
                            let _ = event_tx
                                .send(PortalEvent::HealthChanged(HealthState::Red))
                                .await;
                        }
                    }
                }
            }
            cmd = command_rx.recv() => {
                match cmd {
                    Some(PortalCommand::Shutdown) | None => break,
                    Some(PortalCommand::VerifyNow) => {
                        last_success = None;
                    }
                    Some(PortalCommand::ItemEnqueued(_)) | Some(PortalCommand::RetryItem(_)) => {
                        last_success = None;
                    }
                    Some(PortalCommand::QueueUpdated(new_queue)) => {
                        queue_snapshot = new_queue;
                    }
                }
            }
        }
    }
}

/// Returns `true` iff both portal calls succeeded. The caller uses this
/// to decide whether to stamp `last_success`; advancing `last_success`
/// on a failed attempt would suppress the cadence-driven `verify_token`
/// health check and leave the indicator stuck at Green during a silent
/// portal outage — exactly the failure mode this feature is meant to
/// surface.
async fn attempt_item(
    io: &impl PortalTaskIo,
    item: &super::queue::QueuedItem,
    event_tx: &mpsc::Sender<PortalEvent>,
) -> bool {
    // The portal API collapses all non-success outcomes (409 conflict,
    // 401 token expired, 5xx, network) into a single error, so we treat
    // any `Err` identically: leave the item on the queue for the next
    // retry tick, and emit ItemUpdated so the UI can refresh the row.
    let score_result = io.post_scores(item).await;
    if score_result.is_err() {
        let _ = event_tx
            .send(PortalEvent::ItemUpdated(item.id.clone()))
            .await;
        return false;
    }
    match io.post_stats(item).await {
        Ok(()) => {
            let _ = event_tx
                .send(PortalEvent::ItemResolved(item.id.clone()))
                .await;
            true
        }
        Err(_) => {
            let _ = event_tx
                .send(PortalEvent::ItemUpdated(item.id.clone()))
                .await;
            false
        }
    }
}
```

Add `QueueUpdated(super::queue::QueueFile)` to the `PortalCommand` enum.

- [ ] **Step 2: Wire `PortalManager` to push queue updates to the task**

In `refbox/src/portal_manager/mod.rs`, replace the unused `_event_tx` field and the dummy `new()` with a real spawn that drives the background task.

For this implementation, the manager holds a `command_tx: mpsc::Sender<health::PortalCommand>` and after every mutation calls `self.push_queue_snapshot()`:

```rust
fn push_queue_snapshot(&self) {
    let tx = self.command_tx.clone();
    let snap = self.queue.clone();
    tokio::spawn(async move {
        let _ = tx.send(health::PortalCommand::QueueUpdated(snap)).await;
    });
}
```

Call `self.push_queue_snapshot()` at the end of each of: `enqueue_game_end`, `force_submit`, `discard`, `token_refreshed`.

The production `new()` becomes:

```rust
pub fn new(
    config_dir: &std::path::Path,
    io: impl health::PortalTaskIo + Send + 'static,
) -> std::io::Result<(Self, mpsc::Receiver<PortalEvent>)> {
    let queue = queue::load_or_empty(config_dir)?;
    let handle = health::spawn(io);
    let command_tx = handle.command_tx.clone();

    let mut m = Self {
        queue,
        check_in_flight: false,
        recent_success_until: None,
        indicator_state: PortalIndicatorState::default(),
        token_known_problem: false,
        command_tx,
        config_dir: config_dir.to_path_buf(),
    };
    m.recompute_indicator();
    m.push_queue_snapshot();
    Ok((m, handle.event_rx))
}
```

Replace the `_event_tx` field with `command_tx: mpsc::Sender<health::PortalCommand>`.

Adjust `new_for_test` accordingly:

```rust
#[cfg(test)]
pub(crate) fn new_for_test(
    queue: QueueFile,
    check_in_flight: bool,
    token_known_problem: bool,
) -> Self {
    let (tx, _rx) = mpsc::channel(1);
    let mut m = Self {
        queue,
        check_in_flight,
        recent_success_until: None,
        indicator_state: PortalIndicatorState::default(),
        token_known_problem,
        command_tx: tx,
        config_dir: std::env::temp_dir(),
    };
    m.recompute_indicator();
    m
}
```

- [ ] **Step 3: Update the earlier tests that called `PortalManager::new(tmp.path())`**

The new signature is `new(&Path, impl PortalTaskIo)`. For the existing tests, provide a do-nothing fake:

```rust
#[cfg(test)]
struct NullIo;

#[cfg(test)]
#[async_trait::async_trait]
impl health::PortalTaskIo for NullIo {
    async fn verify_token(&self) -> Result<(), health::PortalCallError> {
        Ok(())
    }
    async fn post_scores(&self, _: &queue::QueuedItem) -> Result<(), health::PortalCallError> {
        Ok(())
    }
    async fn post_stats(&self, _: &queue::QueuedItem) -> Result<(), health::PortalCallError> {
        Ok(())
    }
}
```

And update existing tests to wrap the constructor in a Tokio runtime:

```rust
#[tokio::test]
async fn enqueue_game_end_appends_item_and_turns_yellow() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
    // ... rest unchanged
}
```

(Apply the same `#[tokio::test]` change to `discard_removes_item_and_returns_to_green`,
`force_submit_flags_force_and_resets_attempt_counters`, and
`token_refreshed_clears_flag_and_resets_queue_items`.)

- [ ] **Step 4: Run `just check`**

```bash
just check
```

Expected: all green. Any clippy lints must be fixed before commit.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/
git commit -m "$(cat <<'EOF'
feat(refbox): connect retry loop to queue snapshots

Background task now drains retry-eligible queued items on every
tick, abstracted behind PortalTaskIo. PortalManager pushes a
QueueUpdated command to the task after every queue mutation so
the task's snapshot stays fresh.

PortalEvent::ItemResolved is emitted on a full success (scores +
stats). Any failure (collapsed by uwh-common into a single Err)
emits ItemUpdated; the item stays on the queue for the next tick
or — once it crosses the 30-minute stuck threshold — for operator
action.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: Extend `make_game_time_button` with `portal_indicator` parameter

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs`
- Modify: 12 other view-builder files (all call sites)

- [ ] **Step 1: Add the `portal_indicator` parameter to the function signature**

In `refbox/src/app/view_builders/shared_elements.rs`, update the `make_game_time_button` signature:

```rust
use crate::portal_manager::PortalIndicatorState;

pub(super) fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    tall: bool,
    editing_time: bool,
    mode: Mode,
    clock_running: bool,
    portal_indicator: PortalIndicatorState,   // <-- new
) -> Row<'a, Message> {
    // ... existing body unchanged for now — rendering of the tile lands in Task 12.
    let _ = portal_indicator;  // silence unused for this commit
    // ... existing body
}
```

- [ ] **Step 2: Update every call site**

Every call to `make_game_time_button(...)` must pass a `portal_indicator` argument.

The view-builder files are free functions that receive a `ViewData<'a, 'b>` struct (defined in `refbox/src/app/view_data.rs`), not methods on `self`. The cleanest way to thread the indicator state is to add a field to `ViewData` and populate it once per frame in `view()`:

```rust
// refbox/src/app/view_data.rs
pub(super) struct ViewData<'a, 'b> {
    // ... existing fields ...
    pub(super) portal_indicator: PortalIndicatorState,
}
```

Populate it once in `view()` in `refbox/src/app/mod.rs`:

```rust
let data = ViewData {
    // ... existing fields ...
    portal_indicator: self.portal_manager.indicator_state(),
};
```

Then each of the 12 view-builder files destructures `portal_indicator` out of `data` and passes it as the last argument to `make_game_time_button`:

```rust
let ViewData { snapshot, mode, clock_running, portal_indicator, .. } = data;
// ...
make_game_time_button(
    snapshot,
    /* tall: */ true,
    /* editing_time: */ false,
    mode,
    clock_running,
    portal_indicator,   // <-- new argument
)
```

Use this uniform pattern everywhere to keep the diff consistent and searchable.

Do this for every call in every of these files (paths relative to `refbox/src`):

- `app/view_builders/configuration.rs` (8 calls)
- `app/view_builders/list_selector.rs` (1 call)
- `app/view_builders/fouls.rs` (1 call)
- `app/view_builders/time_edit.rs` (1 call)
- `app/view_builders/confirmation.rs` (2 calls)
- `app/view_builders/warnings.rs` (1 call)
- `app/view_builders/warnings_fouls_summary.rs` (1 call)
- `app/view_builders/score_edit.rs` (1 call)
- `app/view_builders/game_info.rs` (1 call)
- `app/view_builders/penalties.rs` (1 call)
- `app/view_builders/keypad_pages/mod.rs` (1 call)
- `app/view_builders/main_view.rs` (1 call)

Use this exact pattern everywhere to keep the diff consistent and searchable.

- [ ] **Step 3: Add the `portal_manager` field to the `App` struct**

This requires reading `refbox/src/app/mod.rs` and finding the `RefBoxApp` struct. Add:

```rust
pub struct RefBoxApp {
    // ... existing fields ...
    portal_manager: PortalManager,
}
```

Initialise it in the `new()` or constructor path. The production `PortalTaskIo` impl that wraps `UwhPortalClient` is in Task 12 — for this task, use a temporary `NullIo` adapter (move it out of `#[cfg(test)]` for now; it'll be replaced in Task 12).

- [ ] **Step 4: Run `just check`**

```bash
just check
```

Expected: compiles, all tests pass, no clippy warnings.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): thread PortalIndicatorState into make_game_time_button

Extends the shared time-banner helper with a portal_indicator
parameter and updates every call site (20 calls across 12 files)
to pass the current state from self.portal_manager.indicator_state().

The state is not yet rendered — the tile widget itself lands in the
next commit. This commit just threads the plumbing.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: Render the health tile widget

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs`
- Modify: `refbox/src/app/theme/mod.rs` (if new constants needed)
- Modify: `refbox/src/portal_manager/mod.rs` (production `UwhPortalClient`-backed IO impl)

- [ ] **Step 1: Write a helper that builds the tile widget**

In `refbox/src/app/view_builders/shared_elements.rs`, add a new function above `make_game_time_button`:

```rust
use crate::portal_manager::{HealthState, OverlayState, PortalIndicatorState};
use iced::{
    widget::{container, image, row as iced_row, column as iced_col, Space},
    Alignment, ContentFit, Length,
};

const HEALTH_TILE_SIZE: f32 = 130.0;
const LOGO_WIDTH: f32 = 100.0;
const DOT_SIZE: f32 = 50.0;

fn make_health_tile<'a>(state: PortalIndicatorState) -> iced::Element<'a, Message> {
    let logo = image("refbox/resources/UWH_Portal_Compact_Logo.png")
        .width(Length::Fixed(LOGO_WIDTH))
        .content_fit(ContentFit::Contain);

    let dot_color = match state.health {
        HealthState::Green => super::super::theme::GREEN,
        HealthState::Yellow => super::super::theme::YELLOW,
        HealthState::Red => super::super::theme::RED,
    };

    let dot = container(Space::new(Length::Fixed(DOT_SIZE), Length::Fixed(DOT_SIZE)))
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(dot_color)),
            border: iced::Border {
                radius: iced::border::Radius::from(DOT_SIZE / 2.0),
                ..Default::default()
            },
            ..Default::default()
        });

    let content = iced_col![logo, Space::new(Length::Fill, Length::Fixed(8.0)), dot]
        .align_x(Alignment::Center)
        .spacing(0);

    let tile = container(content)
        .width(Length::Fixed(HEALTH_TILE_SIZE))
        .height(Length::Fixed(HEALTH_TILE_SIZE))
        .padding(8.0)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(super::super::theme::LIGHT_GRAY)),
            border: iced::Border {
                radius: super::super::theme::BORDER_RADIUS,
                ..Default::default()
            },
            ..Default::default()
        });

    // Wrap in a button so the tile is a single tap target.
    iced::widget::button(tile)
        .padding(0)
        .on_press(Message::OpenPortalDetailPage)
        .into()
}
```

(The overlay icons — green checkmark and red exclamation — are added in a later step of this task.)

- [ ] **Step 2: Prepend the tile to the banner row**

In `make_game_time_button`, at the end of the function where `time_row` is constructed, prepend the tile:

```rust
let mut time_row = row![make_health_tile(portal_indicator), time_button]
    .spacing(SPACING)
    .align_y(Alignment::Center);
```

Remove the `let _ = portal_indicator;` line introduced in Task 11.

- [ ] **Step 3: Raise the banner's minimum height to fit the 130 px tile**

Where the banner height is computed in `make_game_time_button`, update:

```rust
let button_height = if tall {
    Length::Fixed(HEALTH_TILE_SIZE + PADDING + SMALL_PLUS_TEXT)
} else {
    Length::Fixed(HEALTH_TILE_SIZE)
};
```

- [ ] **Step 4: Implement the overlay icons**

Add SVG assets for the checkmark and exclamation mark (use existing SVGs if there's an equivalent, otherwise create simple new ones at `refbox/resources/check_circle.svg` and `refbox/resources/exclamation.svg`).

Inside `make_health_tile`, wrap the dot in a stack so an overlay icon can sit on top when active:

```rust
let dot_with_overlay: iced::Element<'a, Message> = match state.overlay {
    OverlayState::None => dot.into(),
    OverlayState::RecentSuccess => {
        let check = iced::widget::svg(iced::widget::svg::Handle::from_path(
            "refbox/resources/check_circle.svg",
        ))
        .width(Length::Fixed(DOT_SIZE))
        .height(Length::Fixed(DOT_SIZE));
        iced::widget::stack![dot, check].into()
    }
    OverlayState::AttentionNeeded => {
        let bang = iced::widget::svg(iced::widget::svg::Handle::from_path(
            "refbox/resources/exclamation.svg",
        ))
        .width(Length::Fixed(DOT_SIZE))
        .height(Length::Fixed(DOT_SIZE));
        iced::widget::stack![dot, bang].into()
    }
};
```

Use `dot_with_overlay` in the `iced_col!` instead of `dot`.

- [ ] **Step 5: Add the production `UwhPortalClient`-backed IO impl**

In `refbox/src/portal_manager/mod.rs`, add:

```rust
/// Shared handle to the `UwhPortalClient`. The UI thread and the
/// background retry task both hold clones so token mutations
/// (set_token / clear_token) on the UI thread are immediately visible
/// to the task without a restart. `std::sync::Mutex` (not tokio's) is
/// safe here because `UwhPortalClient`'s request methods return
/// `impl Future + use<>`: the guard builds the request, is dropped,
/// and the network round-trip is awaited without any lock held.
pub type SharedUwhPortalClient =
    std::sync::Arc<std::sync::Mutex<uwh_common::uwhportal::UwhPortalClient>>;

/// Currently-selected event id. `Option` because at app-startup time
/// no event is chosen yet; `Arc<Mutex<_>>` so the UI thread can update
/// it when the operator picks or switches an event and the background
/// task picks up the change on its next tick. A write helper on
/// `RefBoxApp` mirrors `current_event_id` into this field — see
/// `set_current_event_id` in Step 6.
pub type SelectedEventId =
    std::sync::Arc<std::sync::Mutex<Option<uwh_common::uwhportal::schedule::EventId>>>;

pub struct UwhPortalIo {
    client: SharedUwhPortalClient,
    event_id: SelectedEventId,
}

impl UwhPortalIo {
    pub fn new(client: SharedUwhPortalClient, event_id: SelectedEventId) -> Self {
        Self { client, event_id }
    }
}

/// Snapshot the currently-selected event id for use across an `.await`
/// boundary. Drops the guard before returning, so the caller can hold
/// the `Option<EventId>` safely. A poisoned mutex yields the last value
/// (see the same pattern used in the writer helper in Step 6).
fn snapshot_event_id(shared: &SelectedEventId) -> Option<uwh_common::uwhportal::schedule::EventId> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// `QueuedItem.event_id` is a `String` (items entered the queue via
/// `enqueue_game_end`, which accepts any string). Defensively map
/// parse failures to a portal call error instead of panicking.
fn parse_event_id(
    raw: &str,
) -> Result<uwh_common::uwhportal::schedule::EventId, health::PortalCallError> {
    uwh_common::uwhportal::schedule::EventId::from_full(raw).map_err(|e| {
        health::PortalCallError::Failed(format!("invalid queued event_id {raw:?}: {e}"))
    })
}

#[async_trait::async_trait]
impl health::PortalTaskIo for UwhPortalIo {
    async fn verify_token(&self) -> Result<(), health::PortalCallError> {
        let event_id = match snapshot_event_id(&self.event_id) {
            Some(id) => id,
            // No event selected yet — reporting success keeps the
            // indicator green; reporting failure would flash red on a
            // freshly-started refbox before the operator has picked a
            // tournament. `post_scores` / `post_stats` use the queued
            // item's own event id and are unaffected.
            None => return Ok(()),
        };
        // Lock, build the request, drop the guard, then await.
        let fut = self
            .client
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .verify_token(&event_id);
        fut.await.map_err(classify_error)
    }

    async fn post_scores(
        &self,
        item: &queue::QueuedItem,
    ) -> Result<(), health::PortalCallError> {
        let event_id = parse_event_id(&item.event_id)?;
        let fut = self
            .client
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .post_game_scores(&event_id, &item.game_number, item.scores.clone(), item.force);
        fut.await.map_err(classify_error)
    }

    async fn post_stats(
        &self,
        item: &queue::QueuedItem,
    ) -> Result<(), health::PortalCallError> {
        let event_id = parse_event_id(&item.event_id)?;
        let fut = self
            .client
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .post_game_stats(&event_id, &item.game_number, item.stats.clone());
        fut.await.map_err(classify_error)
    }
}

/// Collapse any portal-client error (409, 401, 5xx, network) into
/// `PortalCallError::Failed`. See ADR 011 amendment (2026-04-21).
/// Generic over `Display` so callers don't have to re-box the error.
fn classify_error<E: std::fmt::Display>(e: E) -> health::PortalCallError {
    health::PortalCallError::Failed(e.to_string())
}
```

We do not inspect HTTP status codes because they are not surfaced by the client — every failure collapses to `PortalCallError::Failed`.

- [ ] **Step 6: Wire `UwhPortalIo` into the app and add the writer helper**

In `refbox/src/app/mod.rs`:

1. Add a `portal_event_id: SelectedEventId` field on `RefBoxApp`, initialised to `Arc::new(Mutex::new(None))` alongside `current_event_id: None` so the two start in sync.
2. Convert the existing `Option<UwhPortalClient>` app field to `Option<SharedUwhPortalClient>` (wrap in `Arc<Mutex<_>>`). Every existing caller that reads the client now takes the lock, builds the request, drops the guard, and awaits — mirroring the pattern inside `UwhPortalIo`. Each `.unwrap()` on the mutex gets a `// why this cannot panic:` comment.
3. Replace the temporary `NullIo` from Task 11 with `UwhPortalIo::new(client.clone(), portal_event_id.clone())` when the client is successfully constructed. If client construction fails, retain `NullIo` as a deliberate fallback (the queue can still accept and persist items; no portal calls are made until the client is re-established). The startup-I/O-failure path from Task 11 (degraded `PortalManager`) is unchanged.
4. Add the writer helper:

```rust
/// Update `current_event_id` and mirror the new value into the
/// `portal_event_id` shared handle so the background portal-health
/// task sees it on its next tick. This is the only place that writes
/// `current_event_id` after construction.
fn set_current_event_id(&mut self, new: Option<EventId>) {
    self.current_event_id = new.clone();
    // why this cannot panic: the guarded data is a plain `Option`
    // and no writer panics while holding the guard; a poisoned
    // mutex just returns the previous value, which we overwrite.
    *self
        .portal_event_id
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = new;
}
```

5. Route every write to `self.current_event_id` through `self.set_current_event_id(...)`. At the time of writing, the only such write is inside `apply_settings_change`. If a future task adds persisted-event-id startup seeding, that path must also call the helper.

- [ ] **Step 7: Run `just check`**

```bash
just check
```

Expected: all green.

- [ ] **Step 8: Run the app visually**

```bash
cargo run -p refbox
```

Expected: time banner now shows the UWH Portal logo + green dot tile at the left end on every page. The tile is tappable (tap opens a blank detail page — Task 13 adds the content).

- [ ] **Step 9: Commit**

```bash
git add refbox/src/ refbox/resources/
git commit -m "$(cat <<'EOF'
feat(refbox): render portal health tile on time banner

Adds the 130x130 clickable tile to the left end of the time banner
showing the UWH Portal logo above a coloured status dot. Green /
yellow / red base colours reflect current HealthState; the overlay
icon (checkmark on success, exclamation when attention needed)
stacks on top of the dot.

Also wires in the production UwhPortalIo that backs the background
task with real HTTP calls via uwh_common::uwhportal::UwhPortalClient.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

#### Post-commit notes — Task 12 (added 2026-04-22)

Task 12 shipped in **two commits** on `feat/refbox/portal-health-indicator`:

- `bb4f0b0` — the primary implementation (Steps 1–9 above).
- `68503bb` — a review-driven follow-up that wired the missing `current_event_id` → `portal_event_id` writer, renamed the type alias `SharedEventId` → `SelectedEventId`, and folded in two readability nits. Without this follow-up, the tile's `verify_token` leg was non-functional: the shared field stayed permanently `None`, so the health check short-circuited as `Ok(())` and the tile could not detect an unreachable portal. See the review record in this plan's sibling session memory.

**Pre-authorized deviations from the Task 12 spec** (decided before dispatch; reflected in the Step 5/6 rewrites above):

1. **`Message::OpenPortalDetailPage` placeholder** — added to `refbox/src/app/message.rs` during Task 12 with a `Task::none()` stub handler in `update()` tagged `// TODO(Task 13): route to detail page.`. Task 13 owns the real routing and view builder. Keeping the variant in Task 12 lets the tile compile against its real `on_press` target instead of a dummy.
2. **Client wrapped as `Option<SharedUwhPortalClient>` (`Arc<Mutex<UwhPortalClient>>`)** — the UI thread mutates the client's auth token at runtime (set_token / clear_token from Settings), so the background task must see those mutations without a restart. `std::sync::Mutex` rather than tokio's because `UwhPortalClient`'s methods return `impl Future + use<>`: the guard is dropped before any `.await`, so the sync mutex is safe and lets synchronous reactor paths (`id()`, `has_token()`, `set_token()`) stay sync. Every call site's `.unwrap()` has a `// why this cannot panic:` comment.

**Post-hoc deviations introduced during implementation** (reflected in the Step 5/6 rewrites above; recorded here so the history is complete):

1. **`SelectedEventId` shape** — the original plan was internally contradictory: the Step 5 struct showed `event_id: EventId` (always present) but other plan text treated `current_event_id` as `Option<EventId>` (because no event is selected at startup). The implementer resolved in favour of `Option` and used `Arc<Mutex<_>>` to allow runtime switching. Renamed from `SharedEventId` to `SelectedEventId` in the follow-up commit — the former described the Rust plumbing; the latter describes what the value actually is.
2. **`parse_event_id` defensive helper** — `QueuedItem.event_id` is a `String`, so the production IO impl cannot assume it parses. Maps parse failures to `PortalCallError::Failed` rather than panicking.
3. **`classify_error` made generic over `Display`** — lets call sites pass the error by value without re-boxing. Behaviour unchanged.
4. **`set_current_event_id` writer helper** — added in the follow-up commit. The mirror invariant (shared mutex tracks `current_event_id`) is documented in the field's doc comment but not type-enforced; any future write site that bypasses the helper will silently desync the two fields. A future refactor could introduce a newtype wrapper to enforce this structurally.
5. **`NullIo` retained as a production fallback** — Task 11's "move `NullIo` out of `#[cfg(test)]`" expectation stands: when `UwhPortalClient` construction fails (only possible on a bad https-only config), the app falls back to `NullIo` so the queue can still accept and persist items. The indicator still reflects queue-stuck state. When `enqueue_game_end` is later wired to game-end, items will silently "resolve" under `NullIo` — worth a comment at that call site in the task that lands that wiring.

**Not done by Task 12, required by Task 13:**

- No startup path currently seeds `current_event_id` from persisted config, so the writer helper alone is sufficient *today*. If a future task adds persistence of the last-used event id, that path must also call `set_current_event_id`.
- The UI-layer periodic tick that drives time-based transitions (30-min stuck yellow→red, 10-sec green-overlay expiry) is a Task 13 responsibility — see Task 13's Subscription work. That tick must NOT share state with the game clock, penalty clocks, or the background task's `POLL_INTERVAL`; it is a pure UI-layer timer.

---

### Task 13: Detail page — AppState, Message, empty view builder

**Files:**
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs`
- Create: `refbox/src/app/view_builders/portal_detail.rs`
- Modify: `refbox/src/app/view_builders/mod.rs`

- [ ] **Step 1: Add Message variants**

In `refbox/src/app/message.rs`, add:

```rust
use crate::portal_manager::{ItemId, PortalEvent};

// ... inside the Message enum:
    OpenPortalDetailPage,
    PortalEvent(PortalEvent),
    PortalRowTapped(ItemId),
    PortalForceSubmit(ItemId),
    PortalDiscardTapped(ItemId),
    PortalGoToLogin,
    ClosePortalDetailPage,
```

- [ ] **Step 2: Add AppState variants**

In `refbox/src/app/mod.rs`, add to the `AppState` enum:

```rust
    PortalDetailPage,
    PortalAttentionAction { item_id: ItemId, discard_armed: bool },
    PortalTokenExpiredAction,
```

- [ ] **Step 3: Handle `Message::OpenPortalDetailPage`**

In the `update()` match, add:

```rust
Message::OpenPortalDetailPage => {
    self.app_state = AppState::PortalDetailPage;
    Task::none()
}
Message::ClosePortalDetailPage => {
    self.app_state = AppState::MainPage;
    Task::none()
}
```

- [ ] **Step 4: Create the stub view builder**

Create `refbox/src/app/view_builders/portal_detail.rs` with:

```rust
use iced::{
    widget::{column, container, row, text, Space},
    Alignment, Length,
};

use crate::app::message::Message;
use crate::app::theme::*;

pub(super) fn build_portal_detail_page<'a>(
    // App context fields needed to render will be added here as the page
    // is fleshed out in Task 14.
) -> iced::Element<'a, Message> {
    let title = text("PORTAL — (detail page)").size(MEDIUM_TEXT);
    let back = iced::widget::button(text("BACK").size(SMALL_PLUS_TEXT))
        .on_press(Message::ClosePortalDetailPage)
        .padding(PADDING)
        .style(red_button);

    let list_area = container(title)
        .width(Length::FillPortion(4))
        .height(Length::Fill)
        .padding(PADDING);

    let side = column![Space::new(Length::Fill, Length::Fill), back]
        .align_x(Alignment::Center)
        .width(Length::FillPortion(1));

    row![list_area, side]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}
```

- [ ] **Step 5: Wire the view builder**

In `refbox/src/app/view_builders/mod.rs`, add:

```rust
mod portal_detail;
pub(super) use portal_detail::build_portal_detail_page;
```

In `refbox/src/app/mod.rs` `view()` method, add a match arm:

```rust
AppState::PortalDetailPage => {
    let time_banner = make_game_time_button(
        &self.snapshot, /* tall */ false, false, self.mode,
        self.clock_running, self.portal_manager.indicator_state(),
    );
    column![time_banner, build_portal_detail_page()]
        .spacing(SPACING)
        .into()
}
```

- [ ] **Step 6: Run `just check` and visually verify**

```bash
just check
cargo run -p refbox
```

Tap the health tile. The detail page appears with a BACK button in the side column. Tap BACK — main page returns.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): add portal detail page scaffolding

Adds AppState::PortalDetailPage, Message::OpenPortalDetailPage /
ClosePortalDetailPage, and a minimal view builder that shows a
title + BACK button. Tapping the health tile now opens the detail
page; tapping BACK returns to the main page.

Row rendering and row ordering land in the next commit.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 14: Detail page — row rendering and ordering

**Files:**
- Modify: `refbox/src/app/view_builders/portal_detail.rs`
- Modify: `refbox/src/portal_manager/mod.rs` (expose row data to the view)

- [ ] **Step 1: Expose a `detail_rows()` method on `PortalManager`**

In `refbox/src/portal_manager/mod.rs`:

```rust
#[derive(Debug, Clone)]
pub enum DetailRow {
    /// Shown at the top when `token_known_problem` is true. Tapping
    /// drives the operator through the portal re-login flow.
    TokenExpired,
    /// A queued item that has crossed the 30-minute stuck threshold.
    /// Tapping opens the attention action page (FORCE / DISCARD).
    Stuck {
        id: ItemId,
        game_number: String,
        attempts: u32,
    },
    /// A queued item that is still in the auto-retry window
    /// (< 30 min since it was queued). Informational. Tapping forces
    /// an immediate retry attempt.
    Pending {
        id: ItemId,
        game_number: String,
        attempts: u32,
        retry_in_secs: Option<u32>,
        stats_only: bool,
    },
    RecentSuccess {
        id: ItemId,
        game_number: String,
        submitted_mins_ago: u32,
    },
}

impl PortalManager {
    pub fn detail_rows(&self) -> Vec<DetailRow> {
        let mut out: Vec<DetailRow> = Vec::new();

        // Token expired banner goes first if the background task has
        // flagged a token problem.
        if self.token_known_problem {
            out.push(DetailRow::TokenExpired);
        }

        // Queue items: stuck rows (oldest first) then pending rows
        // (oldest first). The API does not surface a dedicated
        // "conflict" outcome — any item stuck past the 30-minute
        // threshold gets the red Stuck row.
        let now = time::OffsetDateTime::now_utc();
        let mut items: Vec<_> = self.queue.items.iter().collect();
        items.sort_by_key(|it| it.queued_at);
        for it in &items {
            if is_item_stuck(it, now) {
                out.push(DetailRow::Stuck {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                    attempts: it.attempts,
                });
            }
        }
        for it in &items {
            if !is_item_stuck(it, now) {
                out.push(DetailRow::Pending {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                    attempts: it.attempts,
                    retry_in_secs: None, // computed at render time from last_attempt_at
                    stats_only: false,
                });
            }
        }
        // Recent successes (newest first, capped at 5) — in-memory only;
        // populated from the recent-success buffer (new field on PortalManager).
        // ... see Step 2.
        out
    }
}
```

- [ ] **Step 2: Add an in-memory recent-success buffer**

Add to the `PortalManager` struct:

```rust
recent_successes: std::collections::VecDeque<RecentSuccess>,
```

Define:

```rust
#[derive(Debug, Clone)]
struct RecentSuccess {
    id: ItemId,
    game_number: String,
    submitted_at: Instant,
}
```

Initialise to an empty `VecDeque` in both `new()` and `new_for_test()`. When an item is resolved successfully (from a `PortalEvent::ItemResolved` delivered via subscription), push to the front; pop from the back when size exceeds 5.

Since the resolution happens in the app's `update()` handler (reacting to `Message::PortalEvent`), add a public method `PortalManager::on_item_resolved(&mut self, id: ItemId)` that updates the queue (removes the item) AND pushes to `recent_successes`.

Augment `detail_rows()` to append the recent-success rows at the end:

```rust
let now = Instant::now();
for rs in &self.recent_successes {
    let mins = now.saturating_duration_since(rs.submitted_at).as_secs() / 60;
    out.push(DetailRow::RecentSuccess {
        id: rs.id.clone(),
        game_number: rs.game_number.clone(),
        submitted_mins_ago: mins as u32,
    });
}
```

- [ ] **Step 3: Render rows in the view builder**

Flesh out `build_portal_detail_page` in `refbox/src/app/view_builders/portal_detail.rs`:

```rust
use crate::portal_manager::{DetailRow, PortalManager};

pub(super) fn build_portal_detail_page<'a>(
    portal_manager: &'a PortalManager,
) -> iced::Element<'a, Message> {
    let state = portal_manager.indicator_state();
    let summary_text = match state.health {
        HealthState::Green => "PORTAL — CONNECTED · All clear",
        HealthState::Yellow => "PORTAL — CHECKING…",
        HealthState::Red => "PORTAL — ISSUES",
    };
    let title_row = row![
        text(summary_text).size(SMALL_PLUS_TEXT)
    ]
    .spacing(SPACING);

    let mut rows_col = column![].spacing(SPACING);
    for row_data in portal_manager.detail_rows() {
        rows_col = rows_col.push(render_row(row_data));
    }

    let list_area = container(column![title_row, rows_col].spacing(SPACING))
        .width(Length::FillPortion(4))
        .height(Length::Fill)
        .padding(PADDING)
        .style(light_gray_container);

    // ... BACK button and side column as before
}

fn render_row<'a>(r: DetailRow) -> iced::Element<'a, Message> {
    match r {
        DetailRow::TokenExpired => iced::widget::button(
            text("Portal login expired — tap to re-login").size(SMALL_PLUS_TEXT),
        )
        .on_press(Message::PortalGoToLogin)
        .style(red_button)
        .padding(PADDING)
        .into(),
        DetailRow::Stuck { id, game_number, attempts } => {
            iced::widget::button(
                text(format!(
                    "G{} · Needs attention · {} attempts",
                    game_number, attempts,
                ))
                .size(SMALL_PLUS_TEXT),
            )
            .on_press(Message::PortalRowTapped(id))
            .style(red_button)
            .padding(PADDING)
            .into()
        }
        DetailRow::Pending { id, game_number, attempts, retry_in_secs, stats_only } => {
            let label = match (retry_in_secs, stats_only) {
                (Some(secs), false) => format!(
                    "G{} · Pending · {} attempts · retry in 0:{:02}",
                    game_number, attempts, secs,
                ),
                (None, _) => format!(
                    "G{} · Pending · {} attempts · tap to retry",
                    game_number, attempts,
                ),
                (Some(secs), true) => format!(
                    "G{} · Pending · stats only · retry in 0:{:02}",
                    game_number, secs,
                ),
            };
            iced::widget::button(text(label).size(SMALL_PLUS_TEXT))
                .on_press(Message::PortalRowTapped(id))
                .style(yellow_button)
                .padding(PADDING)
                .into()
        }
        DetailRow::RecentSuccess { game_number, submitted_mins_ago, .. } => {
            iced::widget::button(
                text(format!("G{} · Submitted {} min ago", game_number, submitted_mins_ago))
                    .size(SMALL_PLUS_TEXT),
            )
            .style(green_button)
            .padding(PADDING)
            .into()
        }
    }
}
```

- [ ] **Step 4: Update the caller to pass `&self.portal_manager`**

In `refbox/src/app/mod.rs` `view()`, replace `build_portal_detail_page()` with
`build_portal_detail_page(&self.portal_manager)`.

- [ ] **Step 5: Write tests for row ordering in `portal_manager/mod.rs`**

Append to the `tests` module:

```rust
#[tokio::test]
async fn detail_rows_orders_token_then_stuck_then_pending_oldest_first() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

    // One stuck item: queued 40 min ago.
    m.enqueue_game_end("e".into(), "G3".into(), 3, 2, "{}".into()).unwrap();
    m.queue.items[0].queued_at =
        time::OffsetDateTime::now_utc() - time::Duration::minutes(40);

    // Two young pendings: queued 5 min apart but both recent.
    m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into()).unwrap();
    m.queue.items[1].queued_at =
        time::OffsetDateTime::now_utc() - time::Duration::minutes(5);
    m.enqueue_game_end("e".into(), "G2".into(), 0, 0, "{}".into()).unwrap();
    m.queue.items[2].queued_at =
        time::OffsetDateTime::now_utc() - time::Duration::minutes(2);

    // Flag the token as having a known problem to exercise the
    // TokenExpired row.
    m.token_known_problem = true;

    let rows = m.detail_rows();
    assert!(matches!(rows[0], DetailRow::TokenExpired));
    assert!(matches!(rows[1], DetailRow::Stuck { ref game_number, .. } if game_number == "G3"));
    assert!(matches!(rows[2], DetailRow::Pending { ref game_number, .. } if game_number == "G1"));
    assert!(matches!(rows[3], DetailRow::Pending { ref game_number, .. } if game_number == "G2"));
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p refbox portal_manager
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): render detail-page rows with correct ordering

PortalManager::detail_rows() returns the rows in the spec-mandated
order: token-expired row first (if the background task has flagged
a token problem), then stuck items (>30 min since queued, oldest
first), then young pending items (oldest first), then recent
successes (newest first, cap 5).

The view builder renders each row as a coloured button: red for
token-expired and stuck, yellow for young pending, green
(non-tappable) for recent successes. Tapping a red stuck row
routes to the attention action page; tapping a yellow pending
row forces an immediate retry attempt. Both cases emit
Message::PortalRowTapped(id); the app's update() branches on
whether the item is stuck.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 15: Attention action page (stuck items) and row-tap dispatch

**Files:**
- Create: `refbox/src/app/view_builders/portal_attention_action.rs`
- Modify: `refbox/src/app/view_builders/mod.rs`
- Modify: `refbox/src/app/mod.rs`
- Modify: `refbox/src/portal_manager/mod.rs` (public `find` helper)

This page appears when the operator taps a red **Stuck** row on the detail page.
Because the portal API collapses all failure causes (409 conflict, 401 token expiry,
5xx, network) into a single Err with no diagnostic detail — see the ADR 011
amendment dated 2026-04-21 — we offer the operator only two actions:

- **FORCE THIS GAME RESULT** — resubmit with `force=true` so the portal accepts
  the refbox's value even if a conflicting record already exists.
- **DISCARD THIS SUBMISSION** — remove the queued item (two-tap confirmation to
  prevent accidental data loss).

- [ ] **Step 1: Create the view builder**

Create `refbox/src/app/view_builders/portal_attention_action.rs`:

```rust
use iced::{widget::{button, column, text, Space}, Length};
use crate::app::message::Message;
use crate::app::theme::*;
use crate::portal_manager::ItemId;

pub(super) fn build_portal_attention_action<'a>(
    id: ItemId,
    game_number: String,
    black_score: u8,
    white_score: u8,
    attempts: u32,
    discard_armed: bool,
) -> iced::Element<'a, Message> {
    let title = text(format!("Game {} · Needs attention", game_number)).size(MEDIUM_TEXT);
    let info = text(format!(
        "This result has not been accepted by the UWH Portal after {} attempts.\n\
         Refbox value: {}-{}",
        attempts, black_score, white_score,
    ));

    let force = button(text("FORCE THIS GAME RESULT"))
        .on_press(Message::PortalForceSubmit(id.clone()))
        .style(green_button)
        .padding(PADDING)
        .width(Length::Fill);

    let (discard_label, discard_style) = if discard_armed {
        ("TAP AGAIN TO CONFIRM DISCARD", yellow_button as _)
    } else {
        ("DISCARD THIS SUBMISSION", red_button as _)
    };
    let discard = button(text(discard_label))
        .on_press(Message::PortalDiscardTapped(id.clone()))
        .style(discard_style)
        .padding(PADDING)
        .width(Length::Fill);

    let back = button(text("BACK"))
        .on_press(Message::ClosePortalDetailPage)
        .style(gray_button)
        .padding(PADDING)
        .width(Length::Fill);

    column![
        title,
        info,
        force,
        discard,
        Space::new(Length::Fill, Length::Fill),
        back,
    ]
    .spacing(SPACING)
    .padding(PADDING)
    .into()
}
```

- [ ] **Step 2: Register the view builder**

In `refbox/src/app/view_builders/mod.rs`:

```rust
mod portal_attention_action;
pub(super) use portal_attention_action::build_portal_attention_action;
```

- [ ] **Step 3: Add a public `find` helper on `PortalManager`**

In `refbox/src/portal_manager/mod.rs`:

```rust
pub fn find(&self, id: &ItemId) -> Option<&QueuedItem> {
    self.queue.items.iter().find(|it| it.id == *id)
}

pub fn is_stuck(&self, id: &ItemId) -> bool {
    match self.find(id) {
        Some(item) => health::is_item_stuck(item, time::OffsetDateTime::now_utc()),
        None => false,
    }
}
```

- [ ] **Step 4: Handle row-tap dispatch and the FORCE / DISCARD messages**

In `refbox/src/app/mod.rs` `update()`:

```rust
Message::PortalRowTapped(id) => {
    // Route based on whether the item is stuck (>30 min) or young.
    // Young items just retry immediately; stuck items open the
    // attention action page for operator decision.
    if self.portal_manager.is_stuck(&id) {
        self.app_state = AppState::PortalAttentionAction {
            item_id: id,
            discard_armed: false,
        };
    } else {
        // Young pending row tapped — force an immediate retry.
        // The background task will pick it up on its next tick once
        // `force_immediate_retry` clears last_attempt_at.
        if let Err(e) = self.portal_manager.force_immediate_retry(&id) {
            log::error!("force_immediate_retry failed: {e}");
        }
    }
    Task::none()
}
Message::PortalForceSubmit(id) => {
    if let Err(e) = self.portal_manager.force_submit(&id) {
        log::error!("force_submit failed: {e}");
    }
    self.app_state = AppState::PortalDetailPage;
    Task::none()
}
Message::PortalDiscardTapped(id) => {
    if let AppState::PortalAttentionAction { item_id, discard_armed } = &self.app_state {
        if *item_id == id {
            if *discard_armed {
                if let Err(e) = self.portal_manager.discard(&id) {
                    log::error!("discard failed: {e}");
                }
                self.app_state = AppState::PortalDetailPage;
            } else {
                self.app_state = AppState::PortalAttentionAction {
                    item_id: id,
                    discard_armed: true,
                };
            }
        }
    }
    Task::none()
}
```

Add a helper on `PortalManager` that is used by the "young row tapped → retry now"
path:

```rust
pub fn force_immediate_retry(&mut self, id: &ItemId) -> std::io::Result<()> {
    if let Some(item) = self.find_mut(id) {
        item.last_attempt_at = None;
        queue::save(&self.config_dir, &self.queue)?;
    }
    // No indicator recompute needed — the row stays young-pending.
    Ok(())
}
```

- [ ] **Step 5: Render in `view()`**

```rust
AppState::PortalAttentionAction { item_id, discard_armed } => {
    let item = self.portal_manager.find(item_id).cloned();
    let banner = make_game_time_button(...);
    let body = if let Some(item) = item {
        build_portal_attention_action(
            item_id.clone(),
            item.id.game_number.clone(),
            item.black_score,
            item.white_score,
            item.attempts,
            *discard_armed,
        )
    } else {
        text("Item no longer in queue").into()
    };
    column![banner, body].spacing(SPACING).into()
}
```

Add the variant to `AppState`:

```rust
PortalAttentionAction {
    item_id: crate::portal_manager::ItemId,
    discard_armed: bool,
},
```

- [ ] **Step 6: Run `just check` and visually verify**

```bash
just check
cargo run -p refbox
```

Seed a stuck item via a dev override (e.g. manually set `queued_at` to 40 minutes
ago in the on-disk queue.json), restart, then tap the red row on the detail page.
Confirm the attention action page shows FORCE THIS GAME RESULT / DISCARD THIS
SUBMISSION / BACK, that the two-tap DISCARD flow works, and that FORCE removes
the item from the queue on the next retry tick.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): add portal attention action page for stuck items

Tapping a red stuck row on the detail page now opens a scoped page
with FORCE THIS GAME RESULT and DISCARD THIS SUBMISSION (two-tap
confirmation) plus BACK. The page is the operator's only escalation
path since the portal API collapses all failure causes into a
single Err — see ADR 011 amendment 2026-04-21.

Tapping a young (yellow) pending row instead forces an immediate
retry via PortalManager::force_immediate_retry, which clears
last_attempt_at so the next background tick attempts the item
without waiting out the 15-second retry window.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 16: Token-expired action page with re-login flow

**Files:**
- Create: `refbox/src/app/view_builders/portal_token_expired_action.rs`
- Modify: `refbox/src/app/view_builders/mod.rs`
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Create the view builder**

Create `refbox/src/app/view_builders/portal_token_expired_action.rs`:

```rust
use iced::{widget::{button, column, text, Space}, Length};
use crate::app::message::Message;
use crate::app::theme::*;

pub(super) fn build_portal_token_expired_action<'a>() -> iced::Element<'a, Message> {
    let title = text("Portal login expired").size(MEDIUM_TEXT);
    let body = text(
        "The UWH Portal login has expired. Queued scores cannot be sent until \
         you log in again. Tap GO TO LOGIN to re-authenticate.",
    );

    let login = button(text("GO TO LOGIN"))
        .on_press(Message::PortalGoToLogin)
        .style(blue_button)
        .padding(PADDING)
        .width(Length::Fill);
    let back = button(text("BACK"))
        .on_press(Message::ClosePortalDetailPage)
        .style(red_button)
        .padding(PADDING)
        .width(Length::Fill);

    column![title, body, login, Space::new(Length::Fill, Length::Fill), back]
        .spacing(SPACING)
        .padding(PADDING)
        .into()
}
```

- [ ] **Step 2: Register the view builder**

In `refbox/src/app/view_builders/mod.rs`:

```rust
mod portal_token_expired_action;
pub(super) use portal_token_expired_action::build_portal_token_expired_action;
```

- [ ] **Step 3: Handle `Message::PortalGoToLogin`**

In `refbox/src/app/mod.rs` `update()`:

```rust
Message::PortalGoToLogin => {
    // The existing portal-login screen is reached via AppState::LoginPortal
    // (name may differ — check the existing login flow). Remember that we
    // came from the portal detail page so we can return there on success.
    self.portal_login_return_to_detail = true;
    self.app_state = AppState::LoginPortal;
    Task::none()
}
```

Add the boolean field `portal_login_return_to_detail: bool` to `RefBoxApp`.

Hook into the existing login-success handler: after `self.portal_manager.token_refreshed()` is called in the successful-login path, check the flag:

```rust
if self.portal_login_return_to_detail {
    self.portal_login_return_to_detail = false;
    self.app_state = AppState::PortalDetailPage;
} else {
    self.app_state = AppState::MainPage;
}
```

- [ ] **Step 4: Render in `view()`**

```rust
AppState::PortalTokenExpiredAction => {
    let banner = make_game_time_button(...);
    column![banner, build_portal_token_expired_action()]
        .spacing(SPACING)
        .into()
}
```

- [ ] **Step 5: `just check` and visual verify**

```bash
just check
cargo run -p refbox
```

Set `UWH_PORTAL_SCRAMBLE_TOKEN=1` (env var added in Task 20 — if not yet available, manually corrupt the token in the app state). Trigger a submit, open the detail page, tap the "Portal login expired" row, tap GO TO LOGIN, complete re-login. Verify the app returns to the detail page and the queued items resume retrying.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): add portal token-expired action page

Tapping the token-expired row on the detail page opens a scoped
page explaining the situation and offering GO TO LOGIN / BACK.
GO TO LOGIN navigates to the existing portal login flow; after a
successful re-login the app returns to the detail page (tracked
via a portal_login_return_to_detail flag), and the queued items
resume retrying.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 17: End-game advisory banner on the confirm-score screen

**Files:**
- Modify: `refbox/src/app/view_builders/confirmation.rs`

- [ ] **Step 1: Find the confirm-score view builder function**

In `refbox/src/app/view_builders/confirmation.rs`, locate the function that builds the confirm-score page. It's reached when `AppState::ConfirmScores(_)` is active.

- [ ] **Step 2: Add a conditional advisory banner at the top of the body**

Inside that function, add (before the existing confirm-score content):

```rust
use crate::portal_manager::HealthState;

let advisory = if portal_health == HealthState::Red {
    Some(
        container(
            text(fl!("portal-advisory-at-game-end"))
                .style(white_text)
                .size(SMALL_TEXT),
        )
        .width(Length::Fill)
        .padding(PADDING)
        .style(red_container),
    )
} else {
    None
};
```

Add `portal_health: HealthState` to the function's parameters and thread it from the caller (`self.portal_manager.indicator_state().health`).

Prepend the optional advisory to the column that builds the screen:

```rust
let mut body = column![].spacing(SPACING);
if let Some(a) = advisory {
    body = body.push(a);
}
body = body.push(/* existing confirm-score content */);
```

- [ ] **Step 3: `just check` and verify**

```bash
just check
```

For visual verification, temporarily force the portal into Red state by scrambling the token, end a game in the simulator, and confirm the red strip appears on the confirm-score screen with the spec copy.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/confirmation.rs refbox/src/app/mod.rs
git commit -m "$(cat <<'EOF'
feat(refbox): show portal advisory on confirm-score when red

The confirm-score screen now prepends a red advisory strip with
the copy "Portal issue detected. Score will still be queued — find
an admin to resolve." when and only when portal health is Red at
game end.

Trigger is red-state only (yellow does not qualify). When
confirm_score is disabled, no banner is shown; the persistent red
exclamation mark on the time-banner tile is the signal in that
mode.

Translation key portal-advisory-at-game-end added in Task 21.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 18: Route `handle_game_end()` through `PortalManager`

**Files:**
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Locate the current `handle_game_end` and find `post_game_score` / `post_game_stats`**

In `refbox/src/app/mod.rs`, find the current fire-and-forget implementation (the one that spawns async tasks returning `Message::NoAction`).

- [ ] **Step 2: Replace the fire-and-forget tasks with a single `enqueue_game_end` call**

```rust
// Before:
// if let Some(client) = &self.portal_client {
//     Task::perform(async move { ... post_game_score ... }, |_| Message::NoAction)
// }

// After:
if let (Some(event_id), game_number) = (&self.current_event_id, &self.current_game_number) {
    let stats_json = serde_json::to_string(&self.game_stats()).unwrap_or_default();
    if let Err(e) = self.portal_manager.enqueue_game_end(
        event_id.full().to_string(),
        game_number.to_string(),
        self.snapshot.scores.black,
        self.snapshot.scores.white,
        stats_json,
    ) {
        log::error!("portal_manager.enqueue_game_end failed: {e}");
    }
}
```

Remove the old `Task::perform` calls for `post_game_score` and `post_game_stats`.

- [ ] **Step 3: `just check` and manual smoke test**

```bash
just check
cargo run -p refbox
```

End a game in the simulator with the dev portal reachable. Observe: green-checkmark flash on the tile, score arrives at the dev portal. Queue file `portal_queue.json` should not exist (or be empty) after successful submit.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "$(cat <<'EOF'
feat(refbox): route game-end scores through portal_manager

Replaces the previous fire-and-forget post_game_score /
post_game_stats tasks (which returned Message::NoAction regardless
of outcome) with a single portal_manager.enqueue_game_end call.

The manager writes the score to disk before attempting the submit
and tracks its lifecycle: on success, the item moves to the recent-
successes list; on failure, it stays in the queue and retries
automatically.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 19: `UWH_PORTAL_URL_OVERRIDE` environment variable

**Files:**
- Modify: `refbox/src/main.rs` (or wherever the portal URL is read)

- [ ] **Step 1: Find where the portal base URL is currently set**

```bash
grep -rn "uwhportal.com\|base_url" refbox/src/ uwh-common/src/uwhportal/
```

- [ ] **Step 2: Add the override at the point of portal-client construction**

In the refbox code that constructs `UwhPortalClient`, add:

```rust
let portal_base_url = std::env::var("UWH_PORTAL_URL_OVERRIDE")
    .unwrap_or_else(|_| uwh_common::uwhportal::DEFAULT_BASE_URL.to_string());
if std::env::var("UWH_PORTAL_URL_OVERRIDE").is_ok() {
    log::info!("UWH_PORTAL_URL_OVERRIDE active: using {portal_base_url}");
}
let client = UwhPortalClient::new(portal_base_url, /* token */ ...);
```

If `DEFAULT_BASE_URL` isn't already exposed by `uwh_common`, add a constant there (this is a read-only additive change, not a behavioural change to the portal client — acceptable since the crate already hard-codes it).

Alternatively, leave the default in refbox and only override when the env var is set.

- [ ] **Step 3: Test the override manually**

```bash
UWH_PORTAL_URL_OVERRIDE=https://dev.uwhportal.com cargo run -p refbox
```

Expected: log line announcing the override, and all portal calls go to `dev.uwhportal.com`.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/ uwh-common/src/uwhportal/ 2>/dev/null || git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): add UWH_PORTAL_URL_OVERRIDE env var for dev testing

When UWH_PORTAL_URL_OVERRIDE is set at startup, the portal client
base URL is replaced with its value. Logged at info level so the
operator-of-last-resort can tell they are not hitting production.

When unset (the default, and the only setting in production builds),
the hard-coded production URL is used unchanged.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 20: `UWH_PORTAL_SCRAMBLE_TOKEN` debug-only env var

**Files:**
- Modify: `refbox/src/main.rs` (or wherever the portal client is constructed)

- [ ] **Step 1: Add the debug-only scramble hook**

Behind `#[cfg(debug_assertions)]`, after the portal client is constructed:

```rust
#[cfg(debug_assertions)]
{
    if std::env::var("UWH_PORTAL_SCRAMBLE_TOKEN").is_ok() {
        log::warn!("UWH_PORTAL_SCRAMBLE_TOKEN active: next request will 401");
        // The exact implementation depends on how the token is stored in the
        // UwhPortalClient; if it's a String field, set it to "invalid-debug-token".
        client.set_access_token("invalid-debug-token".to_string());
    }
}
```

If `UwhPortalClient` doesn't expose `set_access_token`, add a minimal setter there (allowed: this is inside the refbox-only branch of work; uwh-common's API surface is unchanged in terms of behaviour).

Actually, to keep the scope constraint ("uwh-common's portal client API is preserved exactly"), prefer to scramble at the refbox call site by constructing the client with the bad token:

```rust
let token_to_use = if cfg!(debug_assertions)
    && std::env::var("UWH_PORTAL_SCRAMBLE_TOKEN").is_ok()
{
    "invalid-debug-token".to_string()
} else {
    token.clone()
};
let client = UwhPortalClient::new(portal_base_url, token_to_use);
```

- [ ] **Step 2: Commit**

```bash
git add refbox/src/
git commit -m "$(cat <<'EOF'
feat(refbox): add UWH_PORTAL_SCRAMBLE_TOKEN debug-only env var

In debug builds only, setting UWH_PORTAL_SCRAMBLE_TOKEN=1 at
startup causes the portal client to be constructed with an
invalid token, producing 401 Unauthorized on the next request.
This exercises the token-expired row and re-login flow during
testing without waiting for a real token to expire.

Stripped from release builds via cfg(debug_assertions).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 21: Translation keys across all language files

**Files:**
- Modify: `refbox/translations/en/refbox.ftl` and 13 other `.ftl` files

- [ ] **Step 1: Add the new keys to the English file first**

Append to `refbox/translations/en/refbox.ftl`:

```fluent
# Portal Health Indicator
portal-summary-connected = PORTAL — CONNECTED · All clear
portal-summary-checking = PORTAL — CHECKING…
portal-summary-issues = PORTAL — ISSUES · Last OK { $duration } ago
portal-row-token-expired = Portal login expired — tap to re-login
portal-row-stuck = G{ $game } · Needs attention · { $attempts } attempts
portal-row-pending = G{ $game } · Pending · { $attempts } attempts · retry in 0:{ $secs }
portal-row-pending-tap = G{ $game } · Pending · { $attempts } attempts · tap to retry
portal-row-pending-stats-only = G{ $game } · Pending · stats only · retry in 0:{ $secs }
portal-row-recent = G{ $game } · Submitted { $duration } ago
portal-action-force-submit = FORCE THIS GAME RESULT
portal-action-discard = DISCARD THIS SUBMISSION
portal-action-discard-confirm = TAP AGAIN TO CONFIRM DISCARD
portal-action-go-to-login = GO TO LOGIN
portal-page-title-attention = Game { $game } · Needs attention
portal-page-title-token-expired = Portal login expired
portal-advisory-at-game-end = Portal issue detected. Score will still be queued — find an admin to resolve.
```

- [ ] **Step 2: Replace any hard-coded English in view builders with `fl!(...)` calls**

In each view builder that currently uses literal strings (from Tasks 13–17), replace them with `fl!("portal-row-pending", game=id.game_number.clone(), attempts=item.attempts, secs=secs)` etc.

- [ ] **Step 3: Add Spanish translations**

Append to `refbox/translations/es/refbox.ftl` (use plausible Spanish translations; mark as "unverified" in the existing project convention if unsure):

```fluent
# Portal Health Indicator
portal-summary-connected = PORTAL — CONECTADO · Todo bien
portal-summary-checking = PORTAL — VERIFICANDO…
portal-summary-issues = PORTAL — PROBLEMAS · Último OK hace { $duration }
# ... etc.
```

- [ ] **Step 4: Add French translations**

Append to `refbox/translations/fr/refbox.ftl`.

- [ ] **Step 5: Add unverified-label stubs for the ~11 other languages**

For each of the non-English/Spanish/French language files, follow the existing "unverified-label" convention (check `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md` for the pattern) and stub each new key with an English fallback + unverified marker.

- [ ] **Step 6: Run `just check`**

```bash
just check
```

Expected: all green. The i18n macro should find every key in every language file.

- [ ] **Step 7: Commit**

```bash
git add refbox/translations/
git commit -m "$(cat <<'EOF'
feat(refbox): add translations for portal health indicator strings

Adds ~17 new Fluent keys to every supported language file
(refbox/translations/*/refbox.ftl). English and Spanish are
hand-written; French follows the project's translation workflow;
remaining languages use the unverified-label fallback pattern.

Keys cover: detail-page title summaries, row labels for each of
the four row types (token-expired, stuck, pending, recent),
action-page button labels (FORCE THIS GAME RESULT / DISCARD
THIS SUBMISSION / TAP AGAIN TO CONFIRM DISCARD / GO TO LOGIN /
BACK), page titles, and the confirm-score advisory.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 22: End-to-end manual verification

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` (remove scaffolding attribute)

- [ ] **Step 0: Remove the scaffolding `#![allow(dead_code)]`**

By this point, every type introduced in Task 2 is reachable from live code. Remove the scaffolding attribute and its comment block at the top of `refbox/src/portal_manager/mod.rs`:

```rust
// DELETE these five lines:
// Scaffolding: types are defined up front and progressively wired up in Tasks
// 3–14 of the portal health indicator plan. This attribute is removed in
// Task 22 once all types have live callers.
#![allow(dead_code)]
```

Run `cargo clippy -p refbox --all-targets -- -D warnings` and confirm no new `dead_code` warnings appear. If any do, that means a type was never consumed — stop and report which type, because something in an earlier task is incomplete.

Commit on its own:

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "$(cat <<'EOF'
chore(refbox): remove portal_manager scaffolding dead_code allow

All public types introduced by the portal health indicator are now
consumed by live code paths, so the blanket #[allow(dead_code)] added
during Task 2 scaffolding is no longer needed.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 1: `just check`**

```bash
just check
```

Expected: fmt, lint, tests, audit all green. Fix any outstanding issues before proceeding.

- [ ] **Step 2: Get dev-portal credentials from the user**

Ask: *"Ready to verify against dev.uwhportal.com — can you share the event ID and access key for the mock tournament?"*

- [ ] **Step 3: Run the six-scenario walk-through**

```bash
UWH_PORTAL_URL_OVERRIDE=https://dev.uwhportal.com cargo run -p refbox
```

Walk through each scenario from the spec's Testing section:

- **Scenario 1 — Happy path:** end a mock game; observe 10-second green checkmark then snap; confirm the score appears on dev portal web UI.
- **Scenario 2 — Transient blip:** block `dev.uwhportal.com` at the OS hosts level for 30s during submit; observe yellow indicator during retry; unblock; automatic retry and recovery without operator intervention.
- **Scenario 3 — Stuck submission (conflict or persistent failure):** submit a score; edit it on dev portal web UI so refbox's next submit collides (409), *or* leave dev-portal blocked for longer than 30 minutes. Observe the indicator escalate from yellow to red after 30 minutes, a red Stuck row appear on the detail page, and — on separate runs — verify both FORCE THIS GAME RESULT (observe `force=true` resubmit succeeds) and DISCARD THIS SUBMISSION (observe two-tap confirmation removes the item) outcomes.
- **Scenario 4 — Token expired:** set `UWH_PORTAL_SCRAMBLE_TOKEN=1`; wait for the next `verify_token` cadence tick; observe red indicator and token-expired row on the detail page; tap GO TO LOGIN; complete re-login; return to detail page; queued items resume retrying.
- **Scenario 5 — Restart with items queued:** close refbox with pending items; reopen; queue re-loads; retries resume; indicator shows correct colour (yellow if all young, red if any stuck).
- **Scenario 6 — End game while red:** with a stuck item in the queue and `confirm_score = true`, end a game; advisory banner appears on the confirm-score screen with exact spec copy.

- [ ] **Step 4: Record results**

Make notes of any anomalies and fix them before the final merge.

- [ ] **Step 5: Open the PR**

Push the branch and open the pull request. Use the standard PR template per `.claude/rules/pr-review.md`:

```bash
git push -u origin feat/refbox/portal-health-indicator
gh pr create --title "feat(refbox): portal health indicator with retry queue" --body "$(cat <<'EOF'
## What changed

Adds a portal health indicator to the refbox time banner so tournament operators can see at a
glance whether game scores are reaching the UWH Portal. Silent submission failures become
visible, transient network failures auto-retry from a persisted on-disk queue, and dedicated
pages let the operator force or discard stuck submissions and recover from expired logins.

## Why

Tournament operators have reported games where the score appeared in the refbox but never
reached the portal. The operator had no way to notice this during the tournament — submission
was silent. See ADR 011 for context.

## Scope

Changes are limited to the `refbox` crate: new `portal_manager/` module, modified time-banner
helper with new health-tile widget, four new view builders (detail page + three action pages),
routing of `handle_game_end()` through the manager, new `Message` variants, new `AppState`
variants, env-var config overrides, and translation keys across all supported languages.

No changes to `uwh-common`, `overlay`, `schedule-processor`, `wireless-remote`, or any other
crate. The portal client API is preserved exactly.

## How to verify

All six scenarios from `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`
pass against `dev.uwhportal.com`:

- [ ] Happy-path submit — green checkmark flashes for 10s, then snaps to plain green dot
- [ ] Transient network blip — yellow indicator during retry, clears to green when network returns
- [ ] Stuck submission (conflict or >30 min persistent failure) — red row, both FORCE and DISCARD outcomes
- [ ] Token expired (via `UWH_PORTAL_SCRAMBLE_TOKEN=1`) — red row, re-login returns to detail page
- [ ] Restart with queued items — queue persists, retries resume
- [ ] End game while red — advisory banner on confirm-score screen with exact spec copy

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Post-Plan Notes

- Each task should be a single commit; if a task's scope grows during implementation, split it.
- `just check` must pass after every task — do not defer lint/format fixes across tasks.
- Any deviation from this plan (new field name, different struct shape) must be reflected in the spec; update the spec document before committing the deviation.
- The queue file format is versioned (`version: 1`); any future change to `QueuedItem` must bump the version and add migration logic.
