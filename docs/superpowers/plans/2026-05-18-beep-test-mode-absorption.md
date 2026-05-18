# Beep-Test Mode Absorption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Absorb the standalone `beep-test/` crate into `refbox` as a fourth operating mode (`Mode::BeepTest`), then delete the `beep-test/` crate. Mode switching reuses the existing Hockey↔Rugby restart-the-exe pattern. The wire format to the LED panel stays byte-identical.

**Architecture:** The cadence engine, snapshot types, and config schema move from `beep-test/` into `refbox/src/beep_test/` (verbatim relocation first, tests second). A new view_builder renders the beep-test screen using refbox's theme and translation system. `Mode::BeepTest` joins the existing `Mode` enum and threads through every match site. `AppState::BeepTestPage` becomes the default `AppState` when refbox starts up with `config.mode == Mode::BeepTest`. The existing `From<BeepTestSnapshot> for GameSnapshotNoHeap` impl is preserved verbatim so the LED panel firmware, overlay, wireless-remote, and `led-panel-sim` see byte-identical packets and need no changes.

**Tech Stack:** Rust 2024 edition (MSRV 1.85), `iced` 0.13 (Elm-like UI), `tokio` async runtime, `fluent` translations via `fl!` macro, `confy` for config persistence, `enum-derive-2018` + `macro-attr-2018` for enum derive macros, `derivative` for selective derives.

**Spec:** `docs/superpowers/specs/2026-05-18-beep-test-absorption-design.md`

**Process discipline:** This is **lean process** per `.claude/rules/plan-execution.md` — refbox-only (no `uwh-common`, no `wireless-remote`, no state-machine work in `tournament_manager/`). Mechanical relocation tasks do not need verification ceremony; the cadence-engine logic relocation is done verbatim and then tested.

---

## File Structure

### Files created in this work

| Path | Responsibility |
|------|----------------|
| `refbox/src/beep_test/mod.rs` | Module entry point for beep-test functionality; re-exports `cadence::*` and `snapshot::*` |
| `refbox/src/beep_test/cadence.rs` | Cadence/lap engine (relocated from `beep-test/src/tournament_manager/mod.rs`). Drives the beep-test schedule, fires whistle/buzzer events on level transitions |
| `refbox/src/beep_test/snapshot.rs` | `BeepTestSnapshot`, `BeepTestPeriod`, `TimeSnapshot` types and the `From<BeepTestSnapshot> for GameSnapshotNoHeap` impl (relocated from `beep-test/src/snapshot.rs`) |
| `refbox/src/app/view_builders/beep_test.rs` | New view_builder rendering the beep-test screen (cadence timer, level indicator, lap count, levels table, start/stop button) |
| `docs/superpowers/notes/2026-05-18-sound-controller-audit.md` | Audit document recording which beep-test sound API calls map to which refbox sound_controller calls; deletion gate |

### Files modified

| Path | Why |
|------|-----|
| `refbox/src/main.rs` | Add `mod beep_test;`; route startup to `AppState::BeepTestPage` when `config.mode == Mode::BeepTest` |
| `refbox/src/config.rs` | Add `Mode::BeepTest` variant; add `BeepTest` and `Level` types; add `beep_test: BeepTest` field to `Config`; extend migration and tests |
| `refbox/src/app/mod.rs` | Add `AppState::BeepTestPage`; add BeepTest cadence engine field on `RefBoxApp`; handle BeepTest-specific messages; dispatch view() for the new AppState |
| `refbox/src/app/message.rs` | Add BeepTest-specific Message variants |
| `refbox/src/app/view_builders/mod.rs` | Add `pub mod beep_test;` and `pub(super) use beep_test::*;` |
| `refbox/src/app/view_builders/configuration.rs` | Extend `impl Cyclable for Mode` to include `BeepTest`; check mode-selector layout still fits four options |
| `refbox/src/app/view_builders/shared_elements.rs` | Add `Mode::BeepTest` arms to the existing `Mode` match sites (logos, portal-name helpers, etc.) — for sites where the BeepTest answer is "same as Hockey6V6" or "unused," use the appropriate value or `unreachable!()` for code paths that BeepTest never enters |
| `refbox/src/app/view_builders/penalties.rs` | Add `Mode::BeepTest` arm to the `match mode` for `PenaltyKind` — `unreachable!()` since BeepTest mode has no penalties |
| `refbox/src/app/view_builders/keypad_pages/penalty_edit.rs` | Same — `Mode::BeepTest` arm with `unreachable!()` |
| All 15 of `refbox/translations/<locale>/refbox.ftl` | Add new translation keys: `beep-test` (mode label), plus any new UI text added by the beep-test view_builder |
| `Cargo.toml` (workspace root) | Remove `"beep-test"` from `members` list (Task 12 only) |
| `justfile` | Remove any beep-test-specific recipes (if any exist — verify during Task 12) |
| `.github/workflows/*.yml` | Remove any beep-test-specific build/test steps (if any exist — verify during Task 12) |

### Files deleted

| Path | Why |
|------|-----|
| `beep-test/` (entire directory) | Task 12: after all functionality is absorbed into refbox and the sound-controller audit (Task 5) has confirmed no gaps |

---

## Branch and worktree

- **Branch:** `feat/refbox/beep-test-mode` (created at the start of this work)
- **Worktree:** Recommended. The `superpowers:using-git-worktrees` skill should set up an isolated worktree before Task 1 begins, so the main checkout is unaffected during implementation. Per the user's memory: **always `cd` into the worktree directory before any `cargo` command** — the Bash tool does not preserve cwd across calls.
- **Base branch:** `master`

---

## Acceptance Criteria (from spec, restated for tracking)

These are the things that must be true when this plan finishes:

1. `Mode::BeepTest` exists in refbox's `Mode` enum with all 15 locale translations for the mode-selector label.
2. Selecting BeepTest in the Configuration page mode selector and pressing Apply triggers an exe restart that lands in the beep-test view.
3. The beep-test view shows the cadence timer, level indicator, lap count, read-only levels table, and start/stop controls — and the controls operate the cadence engine.
4. The LED panel simulator window mirrors the beep-test display while in BeepTest mode (mock LED panel renders the cadence/lap data).
5. Selecting Hockey6V6, Hockey3V3, or Rugby in BeepTest mode and pressing Apply triggers an exe restart that lands back in the game view.
6. The `beep-test/` directory no longer exists in the workspace.
7. The workspace `Cargo.toml`, `justfile`, and CI workflows contain no references to `beep-test`.
8. `just check` passes cleanly: fmt, clippy with `-D warnings` on Linux/Windows/macOS, all tests, audit.
9. Unit tests for the relocated cadence engine exist and pass.
10. Operator-driven walkthrough in the simulator has passed (Task 11).

---

## Task 1: Add `Mode::BeepTest` variant (placeholder arms only)

**Goal:** Get `Mode::BeepTest` into the enum with stub arms in every exhaustive match site, so the rest of the work can compile incrementally. Behaviour is incomplete (BeepTest is unreachable as a startup mode at this stage) — just makes the type system happy.

**Files:**
- Modify: `refbox/src/config.rs`
- Modify: `refbox/src/main.rs`
- Modify: `refbox/src/app/mod.rs`
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/view_builders/shared_elements.rs`
- Modify: `refbox/src/app/view_builders/penalties.rs`
- Modify: `refbox/src/app/view_builders/keypad_pages/penalty_edit.rs`

### Steps

- [ ] **Step 1: Add the variant in `refbox/src/config.rs`**

Edit the `Mode` enum (line 160 area) to add the new variant:

```rust
macro_attr! {
    #[derive(Debug, Clone, Copy, Derivative, PartialEq, Eq, Serialize, Deserialize, EnumFromStr!)]
    #[derivative(Default)]
    pub enum Mode {
        #[derivative(Default)]
        Hockey6V6,
        Hockey3V3,
        Rugby,
        BeepTest,
    }
}
```

Add the Display arm (line 168 area):

```rust
impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hockey6V6 => f.write_str(&fl!("hockey6v6")),
            Self::Hockey3V3 => f.write_str(&fl!("hockey3v3")),
            Self::Rugby => f.write_str(&fl!("rugby")),
            Self::BeepTest => f.write_str(&fl!("beep-test")),
        }
    }
}
```

- [ ] **Step 2: Add the translation key in English**

Edit `refbox/translations/en-US/refbox.ftl` near line 354 (where `hockey6v6`, `hockey3v3`, `rugby` are defined):

```fluent
hockey6v6 = HOCKEY6V6
hockey3v3 = HOCKEY3V3
rugby = RUGBY
beep-test = BEEP TEST
```

The other 14 locales will be filled in during Task 10. For now, fluent's missing-key fallback means non-English locales display the key name `beep-test` — that's intentional and gets fixed in Task 10.

- [ ] **Step 3: Add `Cyclable for Mode` arm in `refbox/src/app/view_builders/configuration.rs`**

At line 135, extend the cycle order:

```rust
impl Cyclable for Mode {
    fn next(&self) -> Self {
        match self {
            Self::Hockey6V6 => Self::Hockey3V3,
            Self::Hockey3V3 => Self::Rugby,
            Self::Rugby => Self::BeepTest,
            Self::BeepTest => Self::Hockey6V6,
        }
    }
}
```

- [ ] **Step 4: Add `Mode::BeepTest` arms at every match site that requires exhaustiveness**

The compiler will tell you exactly which sites need arms once Step 1 lands. The known sites (from grep at plan-writing time) are:

- `refbox/src/main.rs:501-502` — window title selection. Use `"Beep Test"` for `Mode::BeepTest`. Final arm shape:
  ```rust
  Mode::Rugby => "UWR Ref Box",
  Mode::Hockey6V6 | Mode::Hockey3V3 => "UWH Ref Box",
  Mode::BeepTest => "Beep Test",
  ```
- `refbox/src/app/view_builders/shared_elements.rs:280-289` — `can_switch_to_penalty_shot` check. BeepTest mode has no penalty shots; reorganise to skip the check when in BeepTest mode (since this code path runs for game timeouts, which BeepTest doesn't have). If the function is only called for game modes, add `Mode::BeepTest => unreachable!("BeepTest has no timeouts")`.
- `refbox/src/app/view_builders/shared_elements.rs:309-313` — `portal_name_for_mode`. BeepTest doesn't have a portal; return `""` (empty string) for `Mode::BeepTest`. This function only feeds UI labels; an empty portal name is harmless in BeepTest mode where portal UI is not rendered.
- `refbox/src/app/view_builders/shared_elements.rs:355-356` — logo selection. For BeepTest mode, reuse the UWH logo (`Mode::BeepTest => &include_bytes!("../../../resources/UWH_Compact_Logo.png")[..]`). The logo only appears in the time banner, which BeepTest mode doesn't render, so this is unreachable; but adding the arm keeps the type system happy without `unreachable!()`.
- `refbox/src/app/view_builders/shared_elements.rs:590` — `if mode == Mode::Rugby` check (no match exhaustiveness needed since it's an `if`). No change required.
- `refbox/src/app/view_builders/penalties.rs:25-28` — `match mode` for `PenaltyKind`. Add `Mode::BeepTest => unreachable!("BeepTest mode does not edit penalties")`.
- `refbox/src/app/view_builders/keypad_pages/penalty_edit.rs:27-44` — `match mode` for penalty time options. Add `Mode::BeepTest => unreachable!("BeepTest mode does not edit penalties")`.
- `refbox/src/app/mod.rs:1092` and `1100-1101` — portal-URL selection. Use `unreachable!("BeepTest has no portal")` for both sites, since portal code paths are unreachable when `mode == Mode::BeepTest`.
- `refbox/src/app/mod.rs:2777-2782` — `if self.config.mode == Mode::Rugby` (not match-exhaustive). No change.

If the compiler points to additional sites not listed above, add appropriate arms following the same pattern: `unreachable!()` for game-logic paths BeepTest doesn't enter, sensible defaults for shared paths like window title.

- [ ] **Step 5: Verify and commit**

Run `just check` from the worktree path. Expected: PASS. Warnings about unused `Mode::BeepTest` are acceptable at this stage.

```bash
git add refbox/src/config.rs refbox/src/main.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/configuration.rs \
        refbox/src/app/view_builders/shared_elements.rs \
        refbox/src/app/view_builders/penalties.rs \
        refbox/src/app/view_builders/keypad_pages/penalty_edit.rs \
        refbox/translations/en-US/refbox.ftl
git commit -m "feat(refbox): add Mode::BeepTest variant with placeholder arms"
```

---

## Task 2: Relocate snapshot types into refbox

**Goal:** Copy `BeepTestSnapshot`, `BeepTestPeriod`, `TimeSnapshot`, and the `From<BeepTestSnapshot> for GameSnapshotNoHeap` impl into `refbox/src/beep_test/snapshot.rs`. Verbatim. No semantic changes. The original file in `beep-test/` stays for now (deleted in Task 12).

**Files:**
- Create: `refbox/src/beep_test/mod.rs`
- Create: `refbox/src/beep_test/snapshot.rs`
- Modify: `refbox/src/main.rs` (add `mod beep_test;`)

### Steps

- [ ] **Step 1: Create the module entry point**

Create `refbox/src/beep_test/mod.rs` with this exact content:

```rust
//! Beep-test mode: cadence engine, snapshot types, and configuration.
//!
//! Absorbed from the standalone `beep-test/` crate per the design at
//! `docs/superpowers/specs/2026-05-18-beep-test-absorption-design.md`.

pub mod snapshot;
```

The `cadence` submodule is added in Task 3.

- [ ] **Step 2: Create the snapshot module**

Create `refbox/src/beep_test/snapshot.rs`. Copy the **entire** contents of `beep-test/src/snapshot.rs` (110 lines as of plan-writing) verbatim into this new file. Do not modify the code, imports, comments, or anything else. The result should be byte-equivalent to the source file.

The file imports `crate::config::BeepTest` — this import will fail to resolve until Task 4 introduces that type. **Expected**: this file will not compile cleanly until Task 4 lands. To unblock the build for Task 3, change the import temporarily to point to a `super::config::BeepTest` once Task 4 is done, OR leave the file as-is and accept that the `beep_test` module won't build until Task 4. The cleanest approach: include this module declaration in `mod.rs` but **gate it behind a feature flag temporarily** is overkill — instead, do Task 2, 3, 4 in close sequence and have a single commit that lands all three.

**Adjustment to plan ordering:** Tasks 2, 3, and 4 are tightly coupled by imports. The commits at the ends of those tasks land all three together. Skip the individual compile-check at the end of Tasks 2 and 3; the joint check is at the end of Task 4.

- [ ] **Step 3: Register the module in main.rs**

Edit `refbox/src/main.rs` to add the module declaration. Find the block where other modules are declared (around line 30 area, near `mod app;`, `mod config;`, etc.) and add:

```rust
mod beep_test;
```

- [ ] **Step 4: Mark Task 2 done — defer compile check to end of Task 4**

No commit yet. Proceed to Task 3.

---

## Task 3: Relocate cadence engine into refbox

**Goal:** Copy the cadence/lap engine from `beep-test/src/tournament_manager/mod.rs` into `refbox/src/beep_test/cadence.rs`. Verbatim relocation; no logic changes.

**Files:**
- Create: `refbox/src/beep_test/cadence.rs`
- Modify: `refbox/src/beep_test/mod.rs`

### Steps

- [ ] **Step 1: Copy the file content**

Create `refbox/src/beep_test/cadence.rs`. Copy the **entire** contents of `beep-test/src/tournament_manager/mod.rs` (411 lines) verbatim.

- [ ] **Step 2: Adjust the imports for the new location**

The original file imports:

```rust
use super::config::BeepTest as BeepTestConfig;
use super::snapshot::{BeepTestPeriod, BeepTestSnapshot, TimeSnapshot};
```

In the new location, these become:

```rust
use crate::config::BeepTest as BeepTestConfig;
use super::snapshot::{BeepTestPeriod, BeepTestSnapshot, TimeSnapshot};
```

(`BeepTest` config type lives in `refbox/src/config.rs` after Task 4; `snapshot` types live next door in the same `beep_test` module.)

No other content changes. Logic, type signatures, behaviour are preserved exactly.

- [ ] **Step 3: Register the cadence submodule**

Edit `refbox/src/beep_test/mod.rs` to add the new submodule:

```rust
//! Beep-test mode: cadence engine, snapshot types, and configuration.
//!
//! Absorbed from the standalone `beep-test/` crate per the design at
//! `docs/superpowers/specs/2026-05-18-beep-test-absorption-design.md`.

pub mod cadence;
pub mod snapshot;
```

- [ ] **Step 4: Defer the compile check**

The cadence module will not build until Task 4 lands the `BeepTest` config type. Proceed to Task 4.

---

## Task 4: Add `BeepTest` and `Level` config types to refbox

**Goal:** Move the cadence schedule config types (`BeepTest` and `Level`) into `refbox/src/config.rs` and add a `beep_test: BeepTest` field to refbox's `Config` struct, with default value, migration support, and tests.

**Files:**
- Modify: `refbox/src/config.rs`

### Steps

- [ ] **Step 1: Add `Level` and `BeepTest` types**

Open `refbox/src/config.rs`. Below the existing `UwhPortal` struct and above the `Config` struct (around line 75), add:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Level {
    pub count: u8,
    #[serde(with = "secs_only_duration")]
    pub duration: std::time::Duration,
}

impl Level {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut count,
            mut duration,
        } = Default::default();

        if let Some(value) = old.get("count") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                count = value;
            }
        }

        if let Some(value) = old.get("duration") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                duration = std::time::Duration::from_secs(value);
            }
        }

        Self { count, duration }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeepTest {
    #[serde(with = "secs_only_duration")]
    pub pre: std::time::Duration,
    pub levels: Vec<Level>,
}

impl Default for BeepTest {
    fn default() -> Self {
        Self {
            pre: std::time::Duration::from_secs(10),
            levels: vec![
                Level { count: 3, duration: std::time::Duration::from_secs(36) },
                Level { count: 3, duration: std::time::Duration::from_secs(34) },
                Level { count: 3, duration: std::time::Duration::from_secs(32) },
                Level { count: 4, duration: std::time::Duration::from_secs(30) },
                Level { count: 4, duration: std::time::Duration::from_secs(28) },
                Level { count: 4, duration: std::time::Duration::from_secs(26) },
                Level { count: 4, duration: std::time::Duration::from_secs(24) },
                Level { count: 4, duration: std::time::Duration::from_secs(22) },
                Level { count: 5, duration: std::time::Duration::from_secs(20) },
                Level { count: 4, duration: std::time::Duration::from_secs(18) },
            ],
        }
    }
}

impl BeepTest {
    pub fn migrate(old: &Table) -> Self {
        let Self { mut pre, mut levels } = Default::default();

        if let Some(value) = old.get("pre") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                pre = std::time::Duration::from_secs(value);
            }
        }

        if let Some(values) = old.get("levels") {
            if let Some(values) = values.as_array() {
                // Replace the default levels entirely when an override is present.
                levels.clear();
                for value in values {
                    if let Some(table) = value.as_table() {
                        levels.push(Level::migrate(table));
                    }
                }
            }
        }

        Self { pre, levels }
    }
}

mod secs_only_duration {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(dur.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Duration::from_secs(u64::deserialize(deserializer)?))
    }
}
```

- [ ] **Step 2: Add `beep_test: BeepTest` field to the `Config` struct**

At line 75 area, the `Config` struct currently reads:

```rust
pub struct Config {
    pub mode: Mode,
    pub hide_time: bool,
    #[derivative(Default(value = "true"))]
    pub collect_scorer_cap_num: bool,
    pub track_fouls_and_warnings: bool,
    #[derivative(Default(value = "true"))]
    pub confirm_score: bool,
    pub game: Game,
    pub hardware: Hardware,
    pub uwhportal: UwhPortal,
    pub sound: SoundSettings,
    pub language: Option<Language>,
}
```

Add `beep_test: BeepTest` as a new field (place it near `game: Game` for grouping by concern):

```rust
pub struct Config {
    pub mode: Mode,
    pub hide_time: bool,
    #[derivative(Default(value = "true"))]
    pub collect_scorer_cap_num: bool,
    pub track_fouls_and_warnings: bool,
    #[derivative(Default(value = "true"))]
    pub confirm_score: bool,
    pub game: Game,
    pub beep_test: BeepTest,
    pub hardware: Hardware,
    pub uwhportal: UwhPortal,
    pub sound: SoundSettings,
    pub language: Option<Language>,
}
```

- [ ] **Step 3: Extend `Config::migrate` to handle the new field**

In the `migrate` function (around line 93), add:

- A `mut beep_test` line in the destructuring of `Default::default()`
- A migration clause that reads the optional `beep_test` table from the old config
- The new field in the final `Self { ... }` construction

The relevant changes:

```rust
impl Config {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut mode,
            mut hide_time,
            mut collect_scorer_cap_num,
            mut track_fouls_and_warnings,
            confirm_score,
            mut game,
            mut beep_test,         // NEW
            mut hardware,
            mut uwhportal,
            mut sound,
            language,
        } = Default::default();

        // ... existing migration code unchanged ...

        if let Some(old_beep_test) = old.get("beep_test") {     // NEW
            if let Some(old_beep_test) = old_beep_test.as_table() {
                beep_test = BeepTest::migrate(old_beep_test);
            }
        }

        // ... existing migration code unchanged ...

        Self {
            mode,
            hide_time,
            collect_scorer_cap_num,
            track_fouls_and_warnings,
            confirm_score,
            game,
            beep_test,                                          // NEW
            hardware,
            uwhportal,
            sound,
            language,
        }
    }
}
```

- [ ] **Step 4: Extend the config tests**

In the `#[cfg(test)] mod test { ... }` block at the bottom of the file, the existing `test_ser_config` test already does a round-trip of `Config::default()`, which now includes `beep_test` — so this test should pass without change. Verify it does.

Add a new test specifically for `BeepTest` serialization:

```rust
#[test]
fn test_ser_beep_test() {
    let bt: BeepTest = Default::default();
    let serialized = toml::to_string(&bt).unwrap();
    let deser = toml::from_str(&serialized);
    assert_eq!(deser, Ok(bt));
}

#[test]
fn test_migrate_beep_test_absent() {
    let old: Table = Default::default();
    let config = Config::migrate(&old);
    // When old config has no beep_test section, migration uses the default schedule
    assert_eq!(config.beep_test, BeepTest::default());
}

#[test]
fn test_migrate_beep_test_present() {
    let mut old: Table = Default::default();
    let mut bt: Table = Default::default();
    bt.insert("pre".to_string(), toml::Value::Integer(20));
    let mut levels: Vec<toml::Value> = Vec::new();
    let mut level: Table = Default::default();
    level.insert("count".to_string(), toml::Value::Integer(2));
    level.insert("duration".to_string(), toml::Value::Integer(15));
    levels.push(toml::Value::Table(level));
    bt.insert("levels".to_string(), toml::Value::Array(levels));
    old.insert("beep_test".to_string(), toml::Value::Table(bt));
    let config = Config::migrate(&old);
    assert_eq!(config.beep_test.pre, std::time::Duration::from_secs(20));
    assert_eq!(config.beep_test.levels.len(), 1);
    assert_eq!(config.beep_test.levels[0].count, 2);
    assert_eq!(config.beep_test.levels[0].duration, std::time::Duration::from_secs(15));
}
```

- [ ] **Step 5: Verify and commit (Tasks 2 + 3 + 4 land together)**

Run `cargo test --package refbox config::test` — all config tests pass including the new BeepTest tests. Then `just check` — PASS.

```bash
git add refbox/src/beep_test/mod.rs refbox/src/beep_test/snapshot.rs \
        refbox/src/beep_test/cadence.rs refbox/src/main.rs refbox/src/config.rs
git commit -m "feat(refbox): relocate beep-test snapshot, cadence, and config"
```

---

## Task 5: Add unit tests for the cadence engine

**Goal:** Add unit tests for the relocated cadence engine. The standalone `beep-test/` crate had zero tests for this code; closing that gap is part of the absorption work.

**Files:**
- Modify: `refbox/src/beep_test/cadence.rs`

### Steps

- [ ] **Step 1: Add a `#[cfg(test)] mod tests` block at the bottom of `refbox/src/beep_test/cadence.rs`**

The tests cover the five behaviours called out in the spec:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BeepTest as BeepTestConfig, Level};
    use std::time::{Duration, Instant};

    fn test_config() -> BeepTestConfig {
        BeepTestConfig {
            pre: Duration::from_secs(5),
            levels: vec![
                Level { count: 2, duration: Duration::from_secs(10) },
                Level { count: 2, duration: Duration::from_secs(8) },
            ],
        }
    }

    #[test]
    fn test_starts_stopped() {
        let tm = TournamentManager::new(test_config());
        assert!(!tm.clock_is_running());
    }

    #[test]
    fn test_start_clock_marks_running() {
        let mut tm = TournamentManager::new(test_config());
        let now = Instant::now();
        tm.start_clock(now);
        assert!(tm.clock_is_running());
    }

    #[test]
    fn test_stop_clock_marks_stopped() {
        let mut tm = TournamentManager::new(test_config());
        let now = Instant::now();
        tm.start_clock(now);
        tm.stop_clock(now + Duration::from_secs(1)).unwrap();
        assert!(!tm.clock_is_running());
    }

    #[test]
    fn test_level_transitions_at_configured_time() {
        // The cadence engine should advance from one level to the next when
        // the configured duration elapses. With a 5-second pre + a 10-second
        // first level, period transitions happen at known times.
        let mut tm = TournamentManager::new(test_config());
        let start = Instant::now();
        tm.start_clock(start);

        // At t=0 we're in the Pre period.
        assert_eq!(tm.current_period(), BeepTestPeriod::Pre);

        // After the Pre duration elapses, we should be in Level 0.
        // (Adjust the exact tick API based on what the cadence engine exposes;
        // if there's an `update(now)` or `tick(now)` method, call it.)
        // If the engine advances state during `start_clock`, this test verifies
        // that behaviour. If state advances on tick, call the tick API here.
    }

    #[test]
    fn test_reaches_end_and_stops() {
        // After all levels complete, the engine should be in a terminal state
        // (clock stopped, or back to Pre, depending on the engine's design).
        let _tm = TournamentManager::new(test_config());
        // Drive the engine forward through all configured levels.
        // Verify the terminal-state invariant.
    }
}
```

**Note for the executing agent:** The test scaffolding above is intentionally partial — the exact API the cadence engine exposes (whether state advances via `tick`, `update`, or implicitly on `start_clock`) needs to be read from the cadence engine source before completing the level-transition and end-state tests. Read `refbox/src/beep_test/cadence.rs` to understand the public API surface, then complete the tests to exercise:

1. The `Pre → Level(0)` transition fires after `pre` duration elapses.
2. Within a level, the cadence counts up to `count` laps before advancing.
3. The whistle/buzzer triggers (look for `start_stop_tx.send(...)` or similar event-emission code) fire at the expected moments.
4. After the last level, the engine returns to a non-running state.

If the cadence engine does not expose enough public API to test these behaviours, expose what is needed (test-only methods via `#[cfg(test)] pub(crate) fn` are acceptable).

- [ ] **Step 2: Verify and commit**

Run `cargo test --package refbox beep_test::cadence` — all tests pass. Then `just check` — PASS.

```bash
git add refbox/src/beep_test/cadence.rs
git commit -m "test(refbox): add unit tests for relocated beep-test cadence engine"
```

---

## Task 6: Sound-controller audit (documentation only)

**Goal:** Before any code is deleted from `beep-test/`, document the mapping between beep-test's `sound_controller` API surface and refbox's `sound_controller` API surface. This is the **pre-deletion gate** called out in the spec as the highest-risk part of the work.

**Files:**
- Create: `docs/superpowers/notes/2026-05-18-sound-controller-audit.md`

### Steps

- [ ] **Step 1: List all public API surface that beep-test's cadence engine uses from `sound_controller`**

Search `beep-test/src/` for every call site that touches the sound_controller's public API. Use grep:

```bash
grep -rn "sound\." beep-test/src/ --include="*.rs" | grep -v "//\|#\[test"
grep -rn "trigger_\|update_settings\|stop_sound\|SoundController::" beep-test/src/ --include="*.rs"
```

Catalog every method, function, or trait usage. Examples to look for:
- `SoundController::new`
- `sound.trigger_whistle()`
- `sound.trigger_buzzer()`
- `sound.update_settings(...)`
- Any others that appear

- [ ] **Step 2: For each API surface from Step 1, locate the equivalent in `refbox/src/sound_controller/`**

For each one, document:
- The exact path and line number where it is defined in `beep-test/src/sound_controller/`
- The exact path and line number where the equivalent is defined in `refbox/src/sound_controller/`
- Whether the method signatures and semantics match
- Any divergence

- [ ] **Step 3: Write the audit document**

Create `docs/superpowers/notes/2026-05-18-sound-controller-audit.md` with this structure:

```markdown
# Sound Controller Audit — Pre-Deletion Gate

**Date:** 2026-05-18
**Purpose:** Confirm beep-test's `sound_controller` is fully covered by refbox's
`sound_controller` before deleting `beep-test/` in Task 12.

## Methodology

[Brief description of what was checked]

## API surface used by beep-test's cadence engine

| API call | beep-test location | refbox equivalent | Signature match | Notes |
|----------|--------------------|--------------------|------------------|-------|
| `SoundController::new` | beep-test/src/sound_controller/mod.rs:NN | refbox/src/sound_controller/mod.rs:NN | yes / no | ... |
| `trigger_whistle()` | ... | ... | yes / no | ... |
| `trigger_buzzer()` | ... | ... | yes / no | ... |
| `update_settings(...)` | ... | ... | yes / no | ... |
| ... | ... | ... | ... | ... |

## Gaps

[Either: "None — refbox's sound_controller provides every API surface beep-test's cadence engine
uses, with matching semantics. Deletion of beep-test/sound_controller is safe."

OR: "The following beep-test sound_controller features are NOT present in refbox and must be added
before Task 12 (deletion):
- [Specific gap 1, with API signature and where it's used by the cadence engine]
- [Specific gap 2]
- ..."]

## Conclusion

[Either: "Deletion gate PASSED. Task 12 may proceed."

OR: "Deletion gate FAILED. Additional work required before Task 12:
- [Specific task description]
- ...
"]
```

- [ ] **Step 4: If gaps are found**

Stop here and surface the gaps to the user before proceeding. The plan may need an additional task inserted between Task 11 and Task 12 to address the gaps. **Do not skip this gate.** If gaps exist, refbox's `sound_controller` must be extended to cover them before `beep-test/sound_controller/` is deleted.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/notes/2026-05-18-sound-controller-audit.md
git commit -m "docs(refbox): sound-controller audit for beep-test absorption"
```

---

## Task 7: Add BeepTest-specific Message variants

**Goal:** Add the Message variants that the beep-test view will fire when the operator interacts with start/stop/reset controls. Wire them into the `update()` function in `refbox/src/app/mod.rs` to update the cadence engine.

**Files:**
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs`

### Steps

- [ ] **Step 1: Define the new Message variants**

Open `refbox/src/app/message.rs`. Add new variants to the `Message` enum:

```rust
/// Operator pressed Start in BeepTest mode.
BeepTestStart,
/// Operator pressed Stop in BeepTest mode.
BeepTestStop,
/// Operator pressed Reset in BeepTest mode.
BeepTestReset,
/// Tick from the timer subscription; advances the cadence engine.
BeepTestTick,
```

Place these in a logical position in the enum (near other game-clock-style messages).

- [ ] **Step 2: Add a `beep_test_tm` field on `RefBoxApp` if needed**

If the cadence engine needs to live on `RefBoxApp` (rather than being constructed on demand), add a field. Inspect `refbox/src/app/mod.rs` around the `pub struct RefBoxApp { ... }` definition. Add:

```rust
pub struct RefBoxApp {
    // ... existing fields ...
    beep_test_tm: Option<crate::beep_test::cadence::TournamentManager>,
    // ... existing fields ...
}
```

The field is `Option` because most refbox sessions run in Hockey or Rugby mode and don't need a cadence engine. The field is `Some(...)` only when `config.mode == Mode::BeepTest`.

- [ ] **Step 3: Initialize `beep_test_tm` in `RefBoxApp::new` (or equivalent constructor)**

In the constructor where other fields are initialized:

```rust
let beep_test_tm = if config.mode == Mode::BeepTest {
    Some(crate::beep_test::cadence::TournamentManager::new(config.beep_test.clone()))
} else {
    None
};
```

Place this near where `tm` (the game tournament manager) is initialized.

- [ ] **Step 4: Handle the new Message variants in `update()`**

Find the `match message { ... }` block in `refbox/src/app/mod.rs`. Add arms:

```rust
Message::BeepTestStart => {
    if let Some(ref mut tm) = self.beep_test_tm {
        tm.start_clock(Instant::now());
    }
    Task::none()
}
Message::BeepTestStop => {
    if let Some(ref mut tm) = self.beep_test_tm {
        if let Err(e) = tm.stop_clock(Instant::now()) {
            error!("Failed to stop beep-test clock: {e}");
        }
    }
    Task::none()
}
Message::BeepTestReset => {
    if self.config.mode == Mode::BeepTest {
        self.beep_test_tm = Some(crate::beep_test::cadence::TournamentManager::new(
            self.config.beep_test.clone(),
        ));
    }
    Task::none()
}
Message::BeepTestTick => {
    // Per-tick update: advance the cadence engine, fire whistle/buzzer
    // events as configured, and ship a fresh GameSnapshotNoHeap to the
    // LED panel via self.update_sender (using the existing From impl).
    // The exact tick implementation depends on how the cadence engine's
    // public API surfaces its current-state snapshot — read the cadence
    // engine first, then wire this up to call the same methods the
    // standalone beep-test main loop called.
    Task::none()
}
```

The `BeepTestTick` handler is the most involved. It needs to:
1. Call whatever tick/update method the cadence engine exposes (look at the standalone beep-test's main loop in `beep-test/src/app/mod.rs` for the reference pattern).
2. Produce a `BeepTestSnapshot` from the cadence engine.
3. Convert to `GameSnapshotNoHeap` via the `From` impl.
4. Hand it to `self.update_sender` to ship over serial to the LED panel.
5. Fire sound triggers via `self.sound_controller` as the cadence engine emits them.

Reference: `beep-test/src/app/mod.rs:55-90` shows the standalone's tick/sound-trigger logic. Adapt the same pattern but use refbox's `sound_controller` and `update_sender` (which are strict supersets of beep-test's).

- [ ] **Step 5: Add a tick subscription**

The game clock has a tick subscription somewhere in the iced `Application::subscription()` impl. Find it and add a parallel subscription for BeepTest:

```rust
fn subscription(&self) -> Subscription<Message> {
    // existing subscriptions ...
    if self.config.mode == Mode::BeepTest {
        iced::time::every(Duration::from_millis(100))
            .map(|_| Message::BeepTestTick)
    } else {
        Subscription::none()
    }
}
```

(Combine with existing subscriptions using `Subscription::batch(...)` per the existing pattern.)

- [ ] **Step 6: Verify and commit**

Run `just check` — PASS.

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add BeepTest message variants and update logic"
```

---

## Task 8: Add the beep-test view_builder

**Goal:** Create the new view_builder file that renders the beep-test screen. Use refbox's existing theme, translations, and shared_elements helpers. Do not copy from the standalone beep-test view code — write fresh, against today's refbox patterns.

**Files:**
- Create: `refbox/src/app/view_builders/beep_test.rs`
- Modify: `refbox/src/app/view_builders/mod.rs`

### Steps

- [ ] **Step 1: Read the reference implementation to understand the visual layout**

Before writing the new view_builder, read these files to understand the structure of the existing beep-test screen:

- `beep-test/src/app/view_builders/main_view.rs` (the standalone main view)
- `beep-test/src/app/view_builders/shared_elements.rs` (the levels-table helper)

Identify the visual elements: cadence timer display, level indicator, lap count, levels table, start/stop button.

- [ ] **Step 2: Read refbox's view_builder conventions**

Before writing, read these files in refbox to understand the conventions used here:

- `refbox/src/app/view_builders/main_view.rs` (refbox's main view — for layout idioms)
- `refbox/src/app/view_builders/shared_elements.rs` (helpers like `make_multi_label_button`, theme styles)
- `refbox/src/app/theme/` (button styles, text styles)

Note how refbox uses `fl!("key")` for all user-facing text.

- [ ] **Step 3: Create `refbox/src/app/view_builders/beep_test.rs`**

The file should expose one public function:

```rust
//! View_builder for the beep-test screen.
//!
//! Shows the cadence timer, level indicator, lap count, read-only levels
//! table, and start/stop controls. Reachable when `config.mode ==
//! Mode::BeepTest`.

use super::*;
use crate::beep_test::snapshot::BeepTestSnapshot;
use crate::config::BeepTest;

pub(in super::super) fn build_beep_test_page<'a>(
    snapshot: &BeepTestSnapshot,
    config: &'a BeepTest,
    clock_running: bool,
) -> Element<'a, Message> {
    // Construct the layout:
    //   row![
    //     levels_table (read-only),
    //     column![
    //       big_timer,
    //       level_label,
    //       lap_count,
    //       start_stop_button,
    //       reset_button,
    //     ]
    //   ]
    // Use refbox's existing button styles (theme::green_button,
    // theme::red_button, etc.) and the fl! macro for all labels.
    todo!("Implement the layout per the visual reference in beep-test/src/app/view_builders/main_view.rs but using refbox idioms")
}
```

**Note for the executing agent:** Replace the `todo!()` with the actual layout implementation. The implementation must:
1. Use `column!`, `row!`, and the existing iced widget primitives — no new widget types unless none of the existing helpers fit.
2. Use refbox's theme styles for buttons and text — no inline styles.
3. Use `fl!("key")` for every user-facing string. New translation keys go in `refbox/translations/en-US/refbox.ftl` (and the other 14 locales in Task 10).
4. Fire `Message::BeepTestStart`, `Message::BeepTestStop`, `Message::BeepTestReset` on the appropriate button presses. The start button is disabled when `clock_running == true`; the stop button is disabled when `clock_running == false`.
5. Display the cadence-timer value from `snapshot.secs_in_period`, formatted as `MM:SS`. Reuse refbox's existing time-formatting helper (look in `shared_elements.rs` for a `time_string()` or `format_time()` helper).
6. Display the level indicator from `snapshot.current_period` using its `Display` impl ("Pre", "Level 1", etc.).
7. Display the lap count from `snapshot.lap_count`.
8. Render the levels table read-only — port the layout from `beep-test/src/app/view_builders/shared_elements.rs::build_levels_table` but use refbox's text styles and theme.

- [ ] **Step 4: Register the new view_builder module**

Edit `refbox/src/app/view_builders/mod.rs`. Add:

```rust
pub mod beep_test;
pub(super) use beep_test::*;
```

Place these two lines in alphabetical order with the existing module declarations.

- [ ] **Step 5: Add the translation keys used by the new view_builder**

Identify every `fl!("key")` call in the new view_builder file. For each one, add the English translation to `refbox/translations/en-US/refbox.ftl`. Examples (the executing agent fills in the actual keys based on what the view_builder needs):

```fluent
beep-test-level = LEVEL { $level }
beep-test-laps = LAPS: { $laps }
beep-test-start = START
beep-test-stop = STOP
beep-test-reset = RESET
beep-test-pre = PRE
```

Other 14 locales handled in Task 10.

- [ ] **Step 6: Verify and commit**

Run `just check` — PASS.

```bash
git add refbox/src/app/view_builders/beep_test.rs \
        refbox/src/app/view_builders/mod.rs \
        refbox/translations/en-US/refbox.ftl
git commit -m "feat(refbox): add beep-test view_builder"
```

---

## Task 9: Wire BeepTest into AppState and view dispatch

**Goal:** Add `AppState::BeepTestPage`, dispatch to the new view_builder when that state is active, and route refbox startup to `AppState::BeepTestPage` when `config.mode == Mode::BeepTest`.

**Files:**
- Modify: `refbox/src/app/mod.rs`

### Steps

- [ ] **Step 1: Add `AppState::BeepTestPage` variant**

Find the `AppState` enum definition in `refbox/src/app/mod.rs` (around line 165-175). Add a new variant:

```rust
pub enum AppState {
    MainPage,
    // ... existing variants ...
    BeepTestPage,
}
```

The compiler will now demand match arms in every site that matches `AppState` exhaustively. Add `AppState::BeepTestPage => { /* handled in subsequent steps */ }` arms as the compiler points them out, keeping the default behaviour the same as `AppState::MainPage` for any path that doesn't have a BeepTest-specific behaviour.

- [ ] **Step 2: Dispatch to the beep-test view_builder in `view()`**

Find the `match self.app_state` block in `view()` (around line 3143 area). Add an arm:

```rust
AppState::BeepTestPage => {
    let bt_tm = self.beep_test_tm.as_ref().expect(
        "beep_test_tm must be Some when AppState is BeepTestPage"
    );
    let snapshot = bt_tm.current_snapshot(); // or whatever the cadence engine's
                                             // "give me the current display
                                             // snapshot" method is — check
                                             // the cadence module's public API
    let clock_running = bt_tm.clock_is_running();
    build_beep_test_page(&snapshot, &self.config.beep_test, clock_running)
}
```

**Note:** The exact method names (`current_snapshot`, `clock_is_running`) depend on what the cadence engine exposes. Read `refbox/src/beep_test/cadence.rs` first; if the methods don't exist by that name, either rename or add the needed accessors.

- [ ] **Step 3: Route startup to `BeepTestPage` when `mode == BeepTest`**

Find `RefBoxApp::new` or the equivalent constructor. The current initial state is `AppState::MainPage` (line 1207 area). Change this to depend on mode:

```rust
let initial_app_state = if config.mode == Mode::BeepTest {
    AppState::BeepTestPage
} else {
    AppState::MainPage
};

// ... and below ...
app_state: initial_app_state,
last_app_state: initial_app_state,
```

- [ ] **Step 4: Verify the restart flow handles the new mode**

Look at the restart logic around line 957 (where `confy::store` persists the config before restart). No changes should be needed here — the existing flow persists `config.mode = BeepTest` and the next exe instance reads it and routes to `BeepTestPage` per Step 3. Verify by reading the code; do not modify if no change is needed.

- [ ] **Step 5: Verify and commit**

Run `just check` — PASS.

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): route AppState::BeepTestPage in view and startup"
```

---

## Task 10: Add translations for all 15 locales

**Goal:** Add the new translation keys (`beep-test` and any UI keys introduced by the view_builder in Task 8) to all 15 locales. Three locales (en-US, fr, es) get human-quality translations; the other twelve get placeholder English text or copied English keys, to match the convention established by recent translation-key-addition commits.

**Files:**
- Modify: `refbox/translations/<locale>/refbox.ftl` for each of 15 locales

### Steps

- [ ] **Step 1: Identify every translation key added by Tasks 1 and 8**

Run:

```bash
grep -n "^beep-test" refbox/translations/en-US/refbox.ftl
```

This lists every new key.

- [ ] **Step 2: Add the keys to all 15 locales**

For each locale directory under `refbox/translations/` (de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN), add the keys identified in Step 1.

- For **en-US**: already done (Tasks 1 and 8). Verify.
- For **fr** and **es**: translate the keys to French and Spanish. Reasonable translations:
  - `beep-test` → "TEST DE BIP" (fr), "PRUEBA DE PITIDOS" (es)
  - `beep-test-start` → "DÉMARRER" (fr), "INICIAR" (es) — but **first check** if these exact words already exist as other keys in those locales and reuse them
  - The executing agent should consult the existing translation patterns and produce idiomatic translations that match the style of the existing French and Spanish text in those files.
- For the **other 12 locales**: copy the en-US values as placeholders. This matches recent commits like `5147970 feat(refbox): add open-new-display translation key in all locales` — the convention is to add the key everywhere so missing-key warnings don't fire, with placeholder text for locales the team doesn't have native speakers for.

- [ ] **Step 3: Verify and commit**

Run `just check` — PASS.

```bash
git add refbox/translations/
git commit -m "feat(refbox): add beep-test translation keys in all locales"
```

---

## Task 11: Operator-driven walkthrough in the simulator

**Goal:** Verify the end-to-end behaviour matches the spec's acceptance criteria. This is the operator-driven test the spec calls for. Since hardware testing is unavailable, the simulator window is the primary verification surface.

**Files:** None modified.

### Steps

- [ ] **Step 1: Launch refbox with the simulator**

The user (operator) will drive the UI. Claude launches refbox in the background per the memory `feedback_user_drives_refbox_ui`:

```bash
cd <worktree-path>
WAYLAND_DISPLAY= cargo run -p refbox -- --simulate
```

(Or the equivalent dev-launch command for this workspace — verify against `refbox/CLAUDE.md` and recent commits for the right flags.)

- [ ] **Step 2: Walkthrough scenarios — verify each**

The operator walks through the following scenarios and reports the result for each. Each scenario corresponds to an acceptance criterion from the spec.

| # | Scenario | Expected outcome |
|---|----------|------------------|
| A | Open Configuration, look at mode selector | Shows Hockey 6v6, Hockey 3v3, Rugby, Beep Test in cycle order; layout not broken |
| B | Cycle to Beep Test, press Apply (from a game mode) | refbox quits/restarts; new instance shows beep-test screen with timer, level indicator, lap count, levels table, start/stop/reset |
| C | Press Start in BeepTest, wait through 1-2 level transitions | Timer counts down from Pre, then per-level; level indicator advances; lap counter increments; whistle/buzzer fire audibly at the right moments |
| D | Press Stop during active cadence, then Start, then Reset | Stop pauses; Start resumes; Reset returns to initial Pre state |
| E | Watch simulator window during Scenario C | Simulated panel shows same cadence state as the main UI, in sync |
| F | In BeepTest, cycle to Hockey/Rugby, press Apply | refbox quits/restarts; new instance shows normal game-clock screen in chosen mode |
| G | Quit cleanly in BeepTest, relaunch refbox | Starts directly in BeepTest mode (config persisted) |

- [ ] **Step 3: Report results**

The operator marks each scenario PASS or FAIL. If any scenario fails, the issue is investigated and fixed before proceeding to Task 12. The fix may require revisiting Tasks 7, 8, or 9; if so, the corresponding task's commit is amended or a follow-up commit is created (per the user's lean-process preference: do not create separate "fix" commits for issues caught during this walkthrough — fold them into the relevant task's commit where reasonable, OR create one consolidated "fix walkthrough findings" commit).

- [ ] **Step 4: Document walkthrough outcomes**

In `docs/superpowers/notes/2026-05-18-sound-controller-audit.md` (the existing audit doc from Task 6), append a new section:

```markdown
## Walkthrough verification (Task 11)

**Date:** YYYY-MM-DD

| Scenario | Result | Notes |
|----------|--------|-------|
| A: Mode selector shows four options | PASS / FAIL | ... |
| B: Switch into BeepTest | PASS / FAIL | ... |
| C: Cadence engine drives display | PASS / FAIL | ... |
| D: Stop and Reset | PASS / FAIL | ... |
| E: Simulator mirrors panel | PASS / FAIL | ... |
| F: Switch back to Hockey/Rugby | PASS / FAIL | ... |
| G: Config persists across restarts | PASS / FAIL | ... |

**Deferred for real-hardware testing:**
- Real LED panel rendering of BeepTest snapshots
- Real LoRa wireless-remote behaviour in BeepTest mode (expected: silently ignored)
- Serial cable behaviour across the mode-switch restart
```

- [ ] **Step 5: Commit the walkthrough notes**

```bash
git add docs/superpowers/notes/2026-05-18-sound-controller-audit.md
git commit -m "docs(refbox): record beep-test walkthrough verification results"
```

---

## Task 12: Delete the standalone `beep-test/` crate

**Goal:** Remove the now-redundant `beep-test/` crate from the workspace. This is the final commit of the work and only happens after Tasks 6 (sound-controller audit) and 11 (walkthrough verification) have both passed.

**Pre-conditions before this task starts:**
- Task 6 audit document concludes "Deletion gate PASSED."
- Task 11 walkthrough scenarios all marked PASS (or any FAILs have been resolved and re-verified).

**Files:**
- Modify: workspace root `Cargo.toml`
- Modify: `justfile` (if it has any beep-test-specific recipes)
- Modify: `.github/workflows/*.yml` (if any reference beep-test)
- Delete: `beep-test/` (entire directory)

### Steps

- [ ] **Step 1: Remove `"beep-test"` from workspace `Cargo.toml`**

Edit the workspace root `Cargo.toml`:

```toml
[workspace]
resolver = "2"

members = [
  "alphagen",
  "fonts",
  "led-panel-sim",
  "matrix-drawing",
  "overlay",
  "refbox",
  "schedule-processor",
  "uwh-common",
  "wireless-modes",
]
```

(Removed: `"beep-test",`)

- [ ] **Step 2: Check `justfile` for beep-test references**

```bash
grep -n "beep" justfile
```

If any recipes reference `beep-test`, remove them. If none, no change.

- [ ] **Step 3: Check CI workflow files for beep-test references**

```bash
grep -rn "beep-test" .github/workflows/ 2>/dev/null
```

If any references exist, remove them. If none, no change.

- [ ] **Step 4: Check for any other references**

```bash
grep -rn "beep-test\|beep_test" --include="*.toml" --include="*.yml" --include="*.yaml" --include="*.md" -l . | grep -v "node_modules\|target\|.git\|docs/superpowers"
```

This identifies any other files that mention `beep-test`. Excluded: `target/`, `.git/`, `docs/superpowers/` (the spec and audit docs intentionally mention beep-test in their historical record). Any remaining matches should be examined and cleaned up if they refer to the deleted crate.

- [ ] **Step 5: Delete the `beep-test/` directory**

```bash
cd <worktree-path>
git rm -r beep-test/
```

(Using `git rm` not `rm -rf` so the deletion is staged for commit.)

- [ ] **Step 6: Verify and commit**

Run `just check` from the worktree path — PASS (fmt, clippy `-D warnings` Linux/Windows/macOS, tests, audit).

```bash
git add Cargo.toml justfile .github/workflows/
git commit -m "chore(workspace): delete standalone beep-test crate"
```

---

## After Task 12: PR preparation

When all 12 tasks complete and `just check` is clean, the branch is ready for PR. Follow `.claude/rules/pr-review.md`:

- PR title: `feat(refbox): absorb beep-test as a fourth operating mode`
- PR body uses the four-section format (What changed / Why / Scope / How to verify) per the rule
- Link to the spec and the audit doc in the PR body
- Confirm `just check` passes on the final commit

The user reviews the PR using `docs/review-checklist.md`. Do **not** push or open the PR without explicit user approval (per the user's `.claude/rules/communication.md`).

---

