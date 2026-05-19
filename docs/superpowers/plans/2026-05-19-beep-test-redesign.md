# Beep-Test Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder beep-test UI with a refbox-native design (top time bar, transposed levels table with progress highlighting, Settings sub-page hierarchy), drop the redundant `Pre` cadence state, and disable Reset until the first Start press.

**Architecture:** All changes live inside the `refbox` crate. The cadence engine in `refbox/src/beep_test/` loses its `Pre` period (the standalone-inherited 10s warm-up that duplicated `Level(0)`). The view_builder layer gets a new file for the Settings sub-pages and a complete rewrite of the main view. A new `AppState::BeepTestSettings(BeepTestConfigPage)` enum mirrors refbox's existing `AppState::EditGameConfig(ConfigPage)` pattern.

**Tech Stack:** Rust 2024 edition (MSRV 1.85), `iced` 0.13 (Elm-like UI), `fluent` translations via `fl!`, `confy` for config persistence.

**Spec:** `docs/superpowers/specs/2026-05-19-beep-test-redesign-design.md`

**Process discipline:** Lean per `.claude/rules/plan-execution.md` — refbox-only, no `uwh-common`. No per-task code review; one review at the end. Mechanical tasks skip verification ceremony.

---

## Branch and worktree

- **Branch:** `feat/refbox/beep-test-redesign` (created at the start of this work, stacked on `feat/refbox/beep-test-mode`)
- **Worktree:** `.worktrees/feat-refbox-beep-test-redesign/` — already created
- **Base branch:** `feat/refbox/beep-test-mode` (Branch 1)
- **Working directory for all commands:** `/home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/` — cd into it for every cargo/just command

---

## File Structure

### Files created

| Path | Responsibility |
|------|----------------|
| `refbox/src/app/view_builders/beep_test_settings.rs` | New module: Settings landing (2×2), Sound Settings (3×2), Edit Levels, App Mode pages |

### Files modified

| Path | Why |
|------|-----|
| `refbox/src/beep_test/snapshot.rs` | Remove `Pre` variant from `BeepTestPeriod`; update Display impl, length_for_config, count_for_config, next_period methods |
| `refbox/src/beep_test/cadence.rs` | Remove Pre transitions; `start_beep_test_now` goes directly to `Level(0)`; update tests |
| `refbox/src/config.rs` | Remove `pre: Duration` field from `BeepTest`; update Default, migrate, and tests |
| `refbox/src/app/mod.rs` | Add `beep_test_has_run` field; `AppState::BeepTestSettings(BeepTestConfigPage)` variant; dispatch sub-pages in `view()`; new Message handlers; drop Pre from `maybe_play_beep_test_sound` |
| `refbox/src/app/message.rs` | New Message variants for sub-page navigation and Edit Levels actions |
| `refbox/src/app/view_builders/beep_test.rs` | Complete rewrite of main view per spec |
| `refbox/src/app/view_builders/mod.rs` | Register `pub mod beep_test_settings;` |
| `refbox/translations/en-US/refbox.ftl` | New keys for Settings labels, Sound page controls, Edit Levels buttons, App Mode title |
| `refbox/translations/<locale>/refbox.ftl` × 14 | English placeholders for the non-en/es/fr locales; idiomatic translations for fr and es |

### Files deleted

None.

---

## Task 1: Drop the `Pre` state from the cadence engine

**Goal:** Remove `BeepTestPeriod::Pre` and the `BeepTest::pre` config field. After this task, pressing Start transitions stopped → `Level(0)` directly.

**Files:**
- Modify: `refbox/src/beep_test/snapshot.rs`
- Modify: `refbox/src/beep_test/cadence.rs`
- Modify: `refbox/src/config.rs`
- Modify: `refbox/src/app/mod.rs` (the `maybe_play_beep_test_sound` function)
- Modify: `refbox/src/app/view_builders/beep_test.rs` (just remove the Pre arm from the display logic — the full rewrite is Task 3)

### Steps

- [ ] **Step 1: Remove `Pre` from `BeepTestPeriod`**

Edit `refbox/src/beep_test/snapshot.rs`. Find the enum (~line 48):

```rust
pub enum BeepTestPeriod {
    Pre,
    Level(usize),
}
```

Change to:

```rust
pub enum BeepTestPeriod {
    Level(usize),
}
```

Update every `impl` method on `BeepTestPeriod` to remove the `Pre` arms. The methods include `length_for_config`, `count_for_config`, `next_period`, and the `Display` impl. After this change, the `Level(0)` arms become the "first period" for each method.

For `next_period`, the original behavior was: `Pre → Level(0) → Level(1) → ... → Level(N) → Pre`. The new behavior is: `Level(0) → Level(1) → ... → Level(N) → Level(0)` (wraps back to start with clock-stop handling in `start_next_lap`).

For the `Display` impl, `Pre => write!(f, "Pre")` is removed entirely; only `Level(i) => write!(f, "Level {i}")` remains.

- [ ] **Step 2: Remove `BeepTest::pre` field**

Edit `refbox/src/config.rs`. Find the `BeepTest` struct (~line 130):

```rust
pub struct BeepTest {
    #[serde(with = "secs_only_duration")]
    pub pre: std::time::Duration,
    pub levels: Vec<Level>,
}
```

Change to:

```rust
pub struct BeepTest {
    pub levels: Vec<Level>,
}
```

Update the `Default` impl to remove `pre: ..` from the constructor.

Update `BeepTest::migrate` to remove the `pre` migration block:

```rust
// REMOVE:
// if let Some(value) = old.get("pre") {
//     if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
//         pre = std::time::Duration::from_secs(value);
//     }
// }
```

The destructuring `let Self { mut pre, mut levels } = Default::default();` becomes `let Self { mut levels } = Default::default();`.

The final `Self { pre, levels }` becomes `Self { levels }`.

Old configs with `pre = 10` in the TOML will parse safely — `confy`'s migration path calls `BeepTest::migrate` which ignores fields it doesn't read.

- [ ] **Step 3: Update cadence engine entry point**

Edit `refbox/src/beep_test/cadence.rs`. Find `start_beep_test_now` and confirm it transitions directly to `Level(0)`. The current implementation likely sets state to `Pre` then advances; change it to set state to `Level(0)` directly.

The cadence engine's `start_next_lap` (or equivalent end-of-test handler) previously returned to `Pre` after the last level. Change it to return to `Level(0)` with the clock stopped (matching the engine's "test complete, ready to run again" state).

The `update()` method may also reference `Pre` in its match arms. Replace each `Pre` arm with logic that handles being stopped at `Level(0)` (the new "not started" state).

- [ ] **Step 4: Update cadence engine tests**

The five existing tests in `refbox/src/beep_test/cadence.rs::tests` will all fail to compile due to the `Pre` removal. Update them:

- `starts_stopped`: no change to behavior — `Level(0)` is the new initial state. Assert `current_period == BeepTestPeriod::Level(0)`.
- `start_clock_marks_running`: no change.
- `stop_clock_marks_stopped`: no change.
- `start_beep_test_transitions_pre_to_level_0`: rename to `start_beep_test_starts_at_level_0` (or similar) and assert that after `start_beep_test_now`, the current period is `Level(0)` and the clock is running.
- `full_run_ends_stopped`: drive the engine through all levels and assert that at the end, `current_period == Level(0)` and `clock_is_running() == false`.

- [ ] **Step 5: Update sound trigger**

Edit `refbox/src/app/mod.rs::maybe_play_beep_test_sound`. Find the function (search for `fn maybe_play_beep_test_sound`). It currently has:

```rust
let prereqs = new_snapshot.current_period != BeepTestPeriod::Pre
    && new_snapshot.secs_in_period != self.beep_test_snapshot.secs_in_period;

let is_whistle_period = match new_snapshot.current_period {
    BeepTestPeriod::Level(_) => true,
    BeepTestPeriod::Pre => false,
};
```

Simplify to:

```rust
let prereqs = new_snapshot.secs_in_period != self.beep_test_snapshot.secs_in_period;
let is_whistle_period = matches!(new_snapshot.current_period, BeepTestPeriod::Level(_));
```

`is_whistle_period` is now always `true` (since the only variant is `Level`), but keeping the `matches!` form makes the intent explicit and survives if a future variant is added. Alternatively, simplify to `let is_whistle_period = true;` — the implementer's choice.

- [ ] **Step 6: Update view_builder's Period rendering (interim)**

Edit `refbox/src/app/view_builders/beep_test.rs`. Find the display of the period label (the "PRE" / "LEVEL N" string). The full view rewrite is Task 3, but make a minimal interim change here so the build compiles: replace any `BeepTestPeriod::Pre => fl!("beep-test-pre")` arm with logic that only handles `Level(i)`.

If the existing code uses the `Display` impl on `BeepTestPeriod`, it automatically becomes `Level i` — no change needed.

The `beep-test-pre` translation key can stay in the FTL files for now; Task 3 may remove it if it's truly unused.

- [ ] **Step 7: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

Expected: PASS. Tests pass; clippy clean.

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/beep_test/snapshot.rs refbox/src/beep_test/cadence.rs \
        refbox/src/config.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/beep_test.rs && \
git commit -m "feat(refbox): drop Pre state from beep-test cadence engine"
```

---

## Task 2: Add `beep_test_has_run` state and disable Reset

**Goal:** Track whether Start has been pressed at least once in the current session. Reset button renders disabled when it has not.

**Files:**
- Modify: `refbox/src/app/mod.rs` (struct field + Message handlers)
- Modify: `refbox/src/app/view_builders/beep_test.rs` (Reset button disabled state)

### Steps

- [ ] **Step 1: Add the field to `RefBoxApp`**

Edit `refbox/src/app/mod.rs`. Find the `pub struct RefBoxApp { ... }` definition. Add a field near `beep_test_tm`:

```rust
/// True after the operator has pressed Start at least once in this
/// session. Used to gate the Reset button: it renders disabled until
/// this is set. The flag is set on the first BeepTestStart message and
/// persists for the rest of the process (Stop and Reset do not clear it).
beep_test_has_run: bool,
```

In the `RefBoxApp::new` constructor, initialize:

```rust
beep_test_has_run: false,
```

- [ ] **Step 2: Set the flag on first Start**

In the `update()` function, find the `Message::BeepTestStart` arm. At the top of the arm body, add:

```rust
self.beep_test_has_run = true;
```

Leave the rest of the arm untouched.

- [ ] **Step 3: Plumb the flag to the view_builder**

Edit `refbox/src/app/view_builders/beep_test.rs`. Update `build_beep_test_page` to accept a new parameter:

```rust
pub(in super::super) fn build_beep_test_page<'a>(
    snapshot: &BeepTestSnapshot,
    config: &'a BeepTest,
    clock_running: bool,
    has_run: bool,
) -> Element<'a, Message> {
```

Update the caller in `mod.rs::view()` (the `AppState::BeepTestPage` arm) to pass `self.beep_test_has_run`.

- [ ] **Step 4: Render Reset disabled when `!has_run`**

In `build_beep_test_page`, find the Reset button construction. Use refbox's existing pattern for disabled buttons (look at `make_smallish_button` or equivalent helpers in `shared_elements.rs` — they typically take an `Option<Message>` where `None` means disabled).

Replace the current Reset button with:

```rust
let reset_button = if has_run {
    button(text(fl!("beep-test-reset")).width(Length::Fill).center())
        .width(Length::Fill)
        .style(theme::red_button)
        .on_press(Message::BeepTestReset)
} else {
    button(text(fl!("beep-test-reset")).width(Length::Fill).center())
        .width(Length::Fill)
        .style(theme::gray_button)
        // .on_press omitted — button is non-interactive
};
```

(Exact theme style names depend on the refbox theme module. Use whatever the existing
disabled-button pattern in refbox uses — search for `gray_button` or look at how other
disabled buttons in `view_builders/` are styled.)

- [ ] **Step 5: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/mod.rs refbox/src/app/view_builders/beep_test.rs && \
git commit -m "feat(refbox): disable beep-test Reset until first Start"
```

---

## Task 3: Rewrite the main beep-test view

**Goal:** Replace the placeholder view with the spec's layout: refbox-standard time bar at top, `[LEVEL: N]` `[LAPS: N]` widget row, transposed levels table with progress highlighting, bottom row from Branch 1 (kept).

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test.rs` (full rewrite)
- Modify: `refbox/translations/en-US/refbox.ftl` (new keys; old `beep-test-pre`, `beep-test-column-*` may become unused — keep them, Task 9 can prune)

### Steps

- [ ] **Step 1: Read the existing refbox time bar implementation**

Before writing, read `refbox/src/app/view_builders/main_view.rs` (or wherever the game-mode time banner is built — grep for `time_banner` or `time_bar`). The new BeepTest timer bar should reuse the same widget pattern (probably a custom centered text widget with the existing yellow timer style).

Also read `refbox/src/app/theme/` to find the timer-text style.

- [ ] **Step 2: Build the timer bar**

In `beep_test.rs`, construct a row at full width containing the timer text formatted as `MM:SS` from `snapshot.secs_in_period`. Use the same time-formatting helper game mode uses (search for `secs_to_time_string` or similar). Style it with refbox's existing yellow-on-light-gray timer style.

- [ ] **Step 3: Build the [LEVEL: N] [LAPS: N] widget row**

Two boxes side by side, each ~50% width. Each box contains a centered label. The level box reads from `snapshot.current_period` (now always `Level(N)` since Pre is gone — render the level number directly from the variant data; show it 0-indexed as `LEVEL 0`, `LEVEL 1`, etc., matching the existing Display impl). The laps box reads `snapshot.lap_count` and displays it as `LAPS: N`.

The box style should match the refbox light-gray container style used for similar info widgets in game mode.

- [ ] **Step 4: Build the transposed levels table**

For each level in `config.levels`:
- A column header showing the level number (1-indexed for the operator).
- Below the header, `level.count` cells stacked vertically, each showing the duration in seconds (e.g. `36`).

For up to 10 levels, place them in a single band of header+cell rows. If `config.levels.len() > 10`, wrap to additional bands of header+cell rows below. The number of cell rows per band is `max(count for the levels in this band)`.

Use `column!` and `row!` macros to assemble. Cells use refbox's existing cell style (search the theme module for `cell_style` or equivalent).

- [ ] **Step 5: Highlight the active cell**

Determine the active level and active lap-within-level from the snapshot:

```rust
let active_level = match snapshot.current_period {
    BeepTestPeriod::Level(i) => Some(i),
};
let active_lap = snapshot.lap_count; // current lap (1-indexed within the active level)
```

For each cell, apply a highlight style if it matches the active level AND its row index (0-based) equals `active_lap - 1`. Apply a "muted" style if it matches the active level AND its row index is less than `active_lap - 1` (already completed).

Pick highlight colors that match refbox's existing "active period" highlight in game mode (likely the yellow used for active half/quarter — verify in `refbox/src/app/theme/`).

When the clock is stopped (test not running or reset), no cells are highlighted — they all render in the default style.

- [ ] **Step 6: Add fallback collapsed layout for tight vertical space**

If the table requires more than 2 bands (i.e. `config.levels.len() > 20`), collapse the timer bar and widget row into a single horizontal row: `[Time 0:36] [LEVEL: N] [LAPS: N]`.

Implement this as a conditional in `build_beep_test_page`: count the bands, and if >2, switch to the collapsed layout.

- [ ] **Step 7: Wire the existing bottom row from Branch 1**

The `[RESET] [SETTINGS] [START/STOP]` bottom row from Branch 1 stays — keep the existing code that builds it. Just make sure it renders below the new table.

- [ ] **Step 8: Add new translation keys**

Edit `refbox/translations/en-US/refbox.ftl`. Add any new keys the rewritten view uses. Likely:

```fluent
# These may already exist from Branch 1 — verify, only add if missing
beep-test-level-prefix = LEVEL
beep-test-laps-prefix = LAPS:
```

If the existing `beep-test-level = LEVEL { $level }` and `beep-test-laps = LAPS: { $laps }` patterns work, reuse them. The implementer's call.

For other 14 locales, see Task 9 — defer translation updates to that task.

- [ ] **Step 9: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/view_builders/beep_test.rs refbox/translations/en-US/refbox.ftl && \
git commit -m "feat(refbox): redesign beep-test main view with transposed levels table"
```

---

## Task 4: AppState plumbing and Settings landing page

**Goal:** Add the `AppState::BeepTestSettings(BeepTestConfigPage)` variant, dispatch each sub-page in `view()`, and implement the 2×2 landing page.

**Files:**
- Modify: `refbox/src/app/mod.rs`
- Modify: `refbox/src/app/message.rs`
- Create: `refbox/src/app/view_builders/beep_test_settings.rs`
- Modify: `refbox/src/app/view_builders/mod.rs`

### Steps

- [ ] **Step 1: Add the new enum**

Edit `refbox/src/app/mod.rs`. Near the existing `ConfigPage` enum, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeepTestConfigPage {
    Main,
    Sound,
    EditLevels,
    AppMode,
}
```

- [ ] **Step 2: Add the AppState variant**

In `AppState`, add:

```rust
BeepTestSettings(BeepTestConfigPage),
```

The compiler will demand new match arms in every exhaustive `match self.app_state` site. Add arms that match `AppState::MainPage` behavior as the default, except for `view()` where each variant gets its own dispatch (Step 4).

- [ ] **Step 3: Update Settings button to route to the new landing page**

The Settings button on the BeepTest view currently fires `Message::EditGameConfig`, which lands in `AppState::EditGameConfig(ConfigPage::Main)`. Change it to fire a new Message that lands in `AppState::BeepTestSettings(BeepTestConfigPage::Main)`.

Add to `refbox/src/app/message.rs`:

```rust
/// Open the BeepTest Settings landing page. Fired by the Settings
/// button on the BeepTest main view.
BeepTestOpenSettings,
/// Close the BeepTest Settings (returns to BeepTest main view).
BeepTestCloseSettings,
/// Navigate to a sub-page within BeepTest Settings.
BeepTestNavigateTo(BeepTestConfigPage),
```

Add handlers in `update()`:

```rust
Message::BeepTestOpenSettings => {
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
Message::BeepTestCloseSettings => {
    self.app_state = AppState::BeepTestPage;
    Task::none()
}
Message::BeepTestNavigateTo(page) => {
    self.app_state = AppState::BeepTestSettings(page);
    Task::none()
}
```

Update the Settings button in `view_builders/beep_test.rs` to fire `Message::BeepTestOpenSettings` instead of `Message::EditGameConfig`.

- [ ] **Step 4: Dispatch the new AppState in `view()`**

In the `view()` function's `match self.app_state` block, add:

```rust
AppState::BeepTestSettings(page) => {
    use crate::app::view_builders::beep_test_settings::*;
    match page {
        BeepTestConfigPage::Main =>
            build_beep_test_settings_landing(),
        BeepTestConfigPage::Sound =>
            build_beep_test_sound_settings(
                &self.edited_settings.as_ref().expect("...").sound,
            ),
        BeepTestConfigPage::EditLevels =>
            build_beep_test_edit_levels(
                self.edited_settings.as_ref().expect("...").beep_test_levels.as_ref(),
                self.edited_settings.as_ref().expect("...").selected_level,
            ),
        BeepTestConfigPage::AppMode =>
            build_beep_test_app_mode(
                self.edited_settings.as_ref().expect("...").mode,
            ),
    }
}
```

(Function signatures and parameter shapes are sketches — adjust per the existing
`EditableSettings` struct's shape. The implementer reads `EditableSettings` to understand
how to plumb in staged edits.)

- [ ] **Step 5: Create the new view_builder file with the Settings landing page**

Create `refbox/src/app/view_builders/beep_test_settings.rs`:

```rust
//! View_builders for the BeepTest Settings sub-pages.
//!
//! Reachable when `app_state == AppState::BeepTestSettings(_)`. Each
//! function builds one sub-page: the 2x2 landing, Sound Settings, Edit
//! Levels, and App Mode. The Language sub-page reuses the existing
//! `AppState::EditGameConfig(ConfigPage::Language)` flow.

use super::*;
use crate::app::BeepTestConfigPage;

pub(in super::super) fn build_beep_test_settings_landing<'a>() -> Element<'a, Message> {
    let sound_button = button(text(fl!("sound-settings")).width(Length::Fill).center())
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(theme::light_gray_button)
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::Sound));

    let edit_levels_button = button(text(fl!("beep-test-edit-levels")).width(Length::Fill).center())
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(theme::light_gray_button)
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::EditLevels));

    let app_mode_button = button(text(fl!("app-mode")).width(Length::Fill).center())
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(theme::light_gray_button)
        .on_press(Message::BeepTestNavigateTo(BeepTestConfigPage::AppMode));

    let language_button = button(text(fl!("language")).width(Length::Fill).center())
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(theme::light_gray_button)
        // Reuse the existing language page flow
        .on_press(Message::ChangeConfigPage(ConfigPage::Language));

    let row1 = row![sound_button, edit_levels_button].spacing(8);
    let row2 = row![app_mode_button, language_button].spacing(8);

    let back_button = button(text(fl!("back")).width(Length::Fill).center())
        .width(Length::Fill)
        .style(theme::red_button)
        .on_press(Message::BeepTestCloseSettings);

    let done_button = button(text(fl!("done")).width(Length::Fill).center())
        .width(Length::Fill)
        .style(theme::green_button)
        .on_press(Message::BeepTestCloseSettings);

    let bottom_row = row![back_button, Space::with_width(Length::Fill), done_button].spacing(8);

    column![row1, row2, Space::with_height(Length::Fill), bottom_row]
        .spacing(8)
        .padding(8)
        .into()
}

// Stubs for the other pages — implemented in Tasks 5, 6, 7.
pub(in super::super) fn build_beep_test_sound_settings<'a>(
    _sound: &'a crate::config::SoundSettings,
) -> Element<'a, Message> {
    text("TODO: Sound Settings (Task 5)").into()
}

pub(in super::super) fn build_beep_test_edit_levels<'a>(
    _levels: &'a [crate::config::Level],
    _selected: usize,
) -> Element<'a, Message> {
    text("TODO: Edit Levels (Task 6)").into()
}

pub(in super::super) fn build_beep_test_app_mode<'a>(
    _mode: crate::config::Mode,
) -> Element<'a, Message> {
    text("TODO: App Mode (Task 7)").into()
}
```

(Module imports, theme style names, and message variants will need adjustment — the
implementer adapts to refbox's actual conventions. The `Message::ChangeConfigPage` may
not exist with that exact name; use whatever the existing Settings-button-to-config-page
path uses.)

- [ ] **Step 6: Register the module**

Edit `refbox/src/app/view_builders/mod.rs`. Add:

```rust
pub mod beep_test_settings;
```

- [ ] **Step 7: Update `ConfigEditComplete` to return to BeepTest Settings**

When the operator hits Done/Save on the Language sub-page in BeepTest mode, they should
return to the BeepTest Settings landing, not the Hockey main view or the game-mode
Configuration landing.

Find the `ConfigEditComplete` handler in `mod.rs` (Branch 1 added a check: if
`config.mode == Mode::BeepTest`, return to `AppState::BeepTestPage`). Update this to
return to `AppState::BeepTestSettings(BeepTestConfigPage::Main)` instead.

- [ ] **Step 8: Add new translation keys**

Edit `refbox/translations/en-US/refbox.ftl`. Add (if missing):

```fluent
sound-settings = SOUND SETTINGS
beep-test-edit-levels = EDIT LEVELS
app-mode = APP MODE
language = LANGUAGE
back = BACK
done = DONE
```

Most of these probably already exist. Verify before adding to avoid duplicates.

Defer the 14 other locales to Task 9.

- [ ] **Step 9: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/mod.rs refbox/src/app/message.rs \
        refbox/src/app/view_builders/beep_test_settings.rs \
        refbox/src/app/view_builders/mod.rs \
        refbox/src/app/view_builders/beep_test.rs \
        refbox/translations/en-US/refbox.ftl && \
git commit -m "feat(refbox): add beep-test settings landing page and navigation"
```

---

## Task 5: Sound Settings page

**Goal:** Implement the 3×2 Sound Settings grid with disabled-gating.

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (replace the stub)
- Modify: `refbox/src/app/message.rs` (new variants for sound toggles)
- Modify: `refbox/src/app/mod.rs` (handlers for sound toggles, Save/Cancel)
- Modify: `refbox/translations/en-US/refbox.ftl` (control labels)

### Steps

- [ ] **Step 1: Read the existing Sound config sub-page**

Refbox already has a Sound configuration sub-page (`AppState::EditGameConfig(ConfigPage::Sound)`).
Read `refbox/src/app/view_builders/edit_game_config.rs` or whichever file owns it. Note:
- What `EditableSettings.sound` looks like (it's likely a `SoundSettings` clone for staged edits)
- The Message variants that toggle `sound_enabled`, `whistle_enabled`, change `buzzer_sound`, etc.
- The Save/Cancel pattern (Save commits `edited.sound` to `self.config.sound` then persists; Cancel discards)

Reuse those Message variants where they exist. Add new ones only for navigation
(BeepTestSoundSettingsSave / BeepTestSoundSettingsCancel) if the existing Save/Cancel
messages don't fit the BeepTest navigation flow.

- [ ] **Step 2: Build the 3×2 grid**

Replace the `build_beep_test_sound_settings` stub. Construct 6 controls:

| Position | Control | Type | Reads/Writes |
|----------|---------|------|--------------|
| (0,0) | SOUND ENABLED | Toggle | `sound.sound_enabled` |
| (0,1) | ABOVE WATER VOL | Volume control (slider or cycle) | `sound.above_water_vol` |
| (0,2) | WHISTLE ENABLED | Toggle | `sound.whistle_enabled` |
| (1,0) | BUZZER SOUND | Cycle button | `sound.buzzer_sound` |
| (1,1) | BELOW WATER VOL | Volume control | `sound.below_water_vol` |
| (1,2) | WHISTLE VOL | Volume control | `sound.whistle_vol` |

Use refbox's existing volume-control widget (search for `Volume` in `shared_elements.rs` — refbox already has volume widgets for the existing Sound page; reuse them).

Each cell is 1/3 width × 1/2 of the available vertical space (between the title row and the bottom Save/Cancel row).

```rust
let row1 = row![
    sound_enabled_control,
    above_water_vol_control,
    whistle_enabled_control,
].spacing(8);

let row2 = row![
    buzzer_sound_control,
    below_water_vol_control,
    whistle_vol_control,
].spacing(8);

column![
    title,
    row1,
    row2,
    Space::with_height(Length::Fill),
    bottom_row,
].spacing(8).padding(8).into()
```

- [ ] **Step 3: Apply disabled-gating**

When `sound.sound_enabled == false`:
- All 5 other controls render disabled. The `on_press` is omitted; the style is `theme::gray_button` or equivalent.
- Toggle/cycle buttons still display their current value but are non-interactive.

When `sound.whistle_enabled == false`:
- Whistle Vol renders disabled (in addition to any Sound-Enabled gating).

The implementer constructs each control via a small helper that takes a `enabled: bool` parameter and emits the right style + `on_press`.

- [ ] **Step 4: Wire Save and Cancel**

Bottom row:

```rust
let cancel = button(text(fl!("cancel")).width(Length::Fill).center())
    .width(Length::Fill)
    .style(theme::red_button)
    .on_press(Message::BeepTestSoundSettingsCancel);

let save = button(text(fl!("save")).width(Length::Fill).center())
    .width(Length::Fill)
    .style(theme::green_button)
    .on_press(Message::BeepTestSoundSettingsSave);

let bottom_row = row![cancel, Space::with_width(Length::Fill), save].spacing(8);
```

Add the Message variants and their handlers:

```rust
Message::BeepTestSoundSettingsSave => {
    if let Some(edited) = self.edited_settings.take() {
        self.config.sound = edited.sound;
        // Persist to disk
        confy::store(APP_NAME, None, &self.config).ok();
    }
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
Message::BeepTestSoundSettingsCancel => {
    self.edited_settings = None;
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
```

On entry to the Sound Settings page (in `Message::BeepTestNavigateTo(BeepTestConfigPage::Sound)`),
initialize `self.edited_settings.sound` from `self.config.sound.clone()`.

- [ ] **Step 5: Add translation keys**

```fluent
sound-enabled = SOUND ENABLED
above-water-vol = ABOVE WATER VOL
whistle-enabled = WHISTLE ENABLED
buzzer-sound = BUZZER SOUND
below-water-vol = BELOW WATER VOL
whistle-vol = WHISTLE VOL
cancel = CANCEL
save = SAVE
```

Many of these likely already exist (refbox's Sound page already has volume controls).
Reuse existing keys where they exist.

- [ ] **Step 6: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/message.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/beep_test_settings.rs \
        refbox/translations/en-US/refbox.ftl && \
git commit -m "feat(refbox): add beep-test sound settings page"
```

---

## Task 6: Edit Levels page

**Goal:** Implement the interactive transposed levels editor.

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs`
- Modify: `refbox/translations/en-US/refbox.ftl`

### Steps

- [ ] **Step 1: Define the page's staged state**

The page needs to track:
- A clone of `config.beep_test.levels` for editing
- The currently selected level index (defaults to 0)

Add to `EditableSettings` (or wherever staged edits live):

```rust
pub struct EditableSettings {
    // ... existing fields ...
    pub beep_test_levels: Option<Vec<Level>>,
    pub selected_level: usize,
}
```

On entry to the Edit Levels page (in `Message::BeepTestNavigateTo(BeepTestConfigPage::EditLevels)`),
initialize:

```rust
self.edited_settings = Some(EditableSettings {
    beep_test_levels: Some(self.config.beep_test.levels.clone()),
    selected_level: 0,
    // ... other defaults ...
});
```

- [ ] **Step 2: Add Message variants**

Add to `refbox/src/app/message.rs`:

```rust
/// Select a level in the Edit Levels page.
BeepTestEditSelectLevel(usize),
/// Increment the selected level's count.
BeepTestEditCountInc,
/// Decrement the selected level's count.
BeepTestEditCountDec,
/// Increment the selected level's duration (seconds).
BeepTestEditDurationInc,
/// Decrement the selected level's duration.
BeepTestEditDurationDec,
/// Append a new level with default values; select it.
BeepTestEditAddLevel,
/// Remove the currently selected level. No-op if only one level remains.
BeepTestEditRemoveLevel,
/// Save the staged levels and return to settings landing.
BeepTestEditLevelsSave,
/// Discard staged level edits and return to settings landing.
BeepTestEditLevelsCancel,
```

- [ ] **Step 3: Add handlers in `update()`**

Each Message variant gets a handler. Examples:

```rust
Message::BeepTestEditSelectLevel(idx) => {
    if let Some(ref mut edited) = self.edited_settings {
        edited.selected_level = idx;
    }
    Task::none()
}
Message::BeepTestEditCountInc => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            if let Some(level) = levels.get_mut(edited.selected_level) {
                level.count = level.count.saturating_add(1);
            }
        }
    }
    Task::none()
}
Message::BeepTestEditCountDec => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            if let Some(level) = levels.get_mut(edited.selected_level) {
                if level.count > 1 {
                    level.count -= 1;
                }
            }
        }
    }
    Task::none()
}
Message::BeepTestEditDurationInc => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            if let Some(level) = levels.get_mut(edited.selected_level) {
                level.duration = level.duration.saturating_add(std::time::Duration::from_secs(1));
            }
        }
    }
    Task::none()
}
Message::BeepTestEditDurationDec => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            if let Some(level) = levels.get_mut(edited.selected_level) {
                if level.duration > std::time::Duration::from_secs(1) {
                    level.duration -= std::time::Duration::from_secs(1);
                }
            }
        }
    }
    Task::none()
}
Message::BeepTestEditAddLevel => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            levels.push(Level {
                count: 4,
                duration: std::time::Duration::from_secs(20),
            });
            edited.selected_level = levels.len() - 1;
        }
    }
    Task::none()
}
Message::BeepTestEditRemoveLevel => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            if levels.len() > 1 {
                levels.remove(edited.selected_level);
                if edited.selected_level >= levels.len() {
                    edited.selected_level = levels.len() - 1;
                }
            }
        }
    }
    Task::none()
}
Message::BeepTestEditLevelsSave => {
    if let Some(ref edited) = self.edited_settings {
        if let Some(ref levels) = edited.beep_test_levels {
            self.config.beep_test.levels = levels.clone();
            confy::store(APP_NAME, None, &self.config).ok();
        }
    }
    self.edited_settings = None;
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
Message::BeepTestEditLevelsCancel => {
    self.edited_settings = None;
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
```

- [ ] **Step 4: Build the interactive table**

Replace the `build_beep_test_edit_levels` stub in `beep_test_settings.rs`.

The top half of the page is the same transposed table from Task 3's main view, except:
- Tapping any cell or column header fires `Message::BeepTestEditSelectLevel(i)`.
- The selected level is highlighted with a distinct style (different from the main view's "active lap" highlight to avoid confusion).
- An extra `[+NEW]` column header appears at the end, firing `Message::BeepTestEditAddLevel`.

The bottom half is the per-level edit panel:

```
Selected: Level N+1
Time:  [Ts]    [-]  [+]
Count: [C]     [-]  [+]
[REMOVE LEVEL]
```

Built as a column of rows, each with the label, current value, and `[-]` `[+]` buttons firing the appropriate Messages.

`REMOVE LEVEL` is disabled when `levels.len() == 1`.
The `[-]` next to Count is disabled when count == 1.
The `[-]` next to Duration is disabled when duration == 1 second.

- [ ] **Step 5: Wire Save and Cancel at the bottom of the page**

Same pattern as Sound Settings:

```rust
let bottom_row = row![cancel, Space::with_width(Length::Fill), save].spacing(8);
```

Cancel fires `Message::BeepTestEditLevelsCancel`; Save fires `Message::BeepTestEditLevelsSave`.

- [ ] **Step 6: Add translation keys**

```fluent
beep-test-edit-selected = Selected: { $level }
beep-test-edit-time = Time
beep-test-edit-count = Count
beep-test-edit-new = + NEW
beep-test-edit-remove = REMOVE LEVEL
```

- [ ] **Step 7: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/message.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/beep_test_settings.rs \
        refbox/translations/en-US/refbox.ftl && \
git commit -m "feat(refbox): add beep-test edit levels page"
```

---

## Task 7: App Mode page

**Goal:** Implement the App Mode picker with the existing cycle-button pattern.

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs`

### Steps

- [ ] **Step 1: Read the existing Mode cycler**

Find where the game-mode Configuration page renders the Mode cycler (search for `Cyclable<Mode>` or `Mode::Hockey6V6` in `view_builders/`). Reuse that widget pattern.

- [ ] **Step 2: Build the App Mode page**

Replace the `build_beep_test_app_mode` stub. The page consists of:
- A title row ("APP MODE")
- The cycle button centered, taking up most of the page

```rust
let cycle_button = make_smallish_button(
    fl!(mode_to_translation_key(staged_mode)),
    Some(Message::BeepTestEditCycleMode),
);

let title = text(fl!("app-mode")).size(48);

let cancel = button(text(fl!("cancel")).width(Length::Fill).center())
    .style(theme::red_button)
    .on_press(Message::BeepTestEditAppModeCancel);

let apply = button(text(fl!("apply")).width(Length::Fill).center())
    .style(theme::green_button)
    .on_press(Message::BeepTestEditAppModeApply);

let bottom_row = row![cancel, Space::with_width(Length::Fill), apply].spacing(8);

column![
    title,
    Space::with_height(Length::Fill),
    cycle_button,
    Space::with_height(Length::Fill),
    bottom_row,
].spacing(16).padding(16).into()
```

- [ ] **Step 3: Add Message variants and handlers**

```rust
Message::BeepTestEditCycleMode => {
    if let Some(ref mut edited) = self.edited_settings {
        edited.mode = edited.mode.next();
    }
    Task::none()
}
Message::BeepTestEditAppModeApply => {
    let new_mode = self.edited_settings.as_ref()
        .map(|e| e.mode)
        .unwrap_or(self.config.mode);
    self.edited_settings = None;
    if new_mode != self.config.mode {
        // Reuse the existing mode-change-restart flow.
        self.config.mode = new_mode;
        // Trigger the restart pattern used by the existing
        // Game Config page. Find it (search for `ChangeConfigComplete`
        // or the existing mode-change flow) and call the same code.
        // Likely: persist config, then exec the new binary.
        return self.persist_and_restart();
    }
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
Message::BeepTestEditAppModeCancel => {
    self.edited_settings = None;
    self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
    Task::none()
}
```

On entry to the App Mode page, initialize `edited.mode = self.config.mode`.

- [ ] **Step 4: Add translation keys**

```fluent
apply = APPLY
```

- [ ] **Step 5: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/message.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/beep_test_settings.rs \
        refbox/translations/en-US/refbox.ftl && \
git commit -m "feat(refbox): add beep-test app mode picker page"
```

---

## Task 8: Language navigation back-target

**Goal:** When the operator returns from the Language page in BeepTest mode, they should land on the BeepTest Settings landing, not the Hockey main view.

**Files:**
- Modify: `refbox/src/app/mod.rs`

### Steps

- [ ] **Step 1: Find the Language page's return path**

Search for `ConfigPage::Language` handling and the Save/Back/Done buttons on that page. The return path is likely something like `ConfigEditComplete` or `ChangeConfigComplete` setting `app_state = AppState::MainPage` or `AppState::EditGameConfig(ConfigPage::Main)`.

- [ ] **Step 2: Branch on BeepTest mode**

Update the return handler so that when `self.config.mode == Mode::BeepTest`, it sets:

```rust
self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main);
```

Branch 1 already added a similar check that returns to `AppState::BeepTestPage`. Update that check to return to `AppState::BeepTestSettings(BeepTestConfigPage::Main)` instead — the operator came from the Settings landing and should return there.

- [ ] **Step 3: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/mod.rs && \
git commit -m "feat(refbox): return from language page to beep-test settings"
```

---

## Task 9: Translations for all 15 locales

**Goal:** Add all new translation keys to all 15 locales. English placeholders for the 12 non-en/es/fr locales; idiomatic translations for fr and es.

**Files:**
- Modify: `refbox/translations/<locale>/refbox.ftl` × 14 (en-US is done as we go in earlier tasks)

### Steps

- [ ] **Step 1: List new keys**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
grep -nE "^(beep-test-edit|sound-enabled|whistle-enabled|above-water-vol|below-water-vol|whistle-vol|buzzer-sound|sound-settings|app-mode|language|back|done|cancel|save|apply)" refbox/translations/en-US/refbox.ftl
```

This is the canonical list of keys to add to all other locales.

- [ ] **Step 2: French (fr) translations**

Use idiomatic French. Some suggestions (verify against existing keys before duplicating):

```fluent
sound-settings = RÉGLAGES DU SON
beep-test-edit-levels = MODIFIER LES NIVEAUX
app-mode = MODE D'APPLICATION
language = LANGUE
back = RETOUR
done = TERMINÉ
sound-enabled = SON ACTIVÉ
above-water-vol = VOL. HORS DE L'EAU
whistle-enabled = SIFFLET ACTIVÉ
buzzer-sound = SON DU BUZZER
below-water-vol = VOL. SOUS L'EAU
whistle-vol = VOL. SIFFLET
cancel = ANNULER
save = ENREGISTRER
apply = APPLIQUER
beep-test-edit-selected = Sélectionné : { $level }
beep-test-edit-time = Durée
beep-test-edit-count = Compte
beep-test-edit-new = + AJOUTER
beep-test-edit-remove = SUPPRIMER NIVEAU
```

Cross-check with existing fr keys before finalizing.

- [ ] **Step 3: Spanish (es) translations**

```fluent
sound-settings = AJUSTES DE SONIDO
beep-test-edit-levels = EDITAR NIVELES
app-mode = MODO DE APLICACIÓN
language = IDIOMA
back = ATRÁS
done = HECHO
sound-enabled = SONIDO ACTIVADO
above-water-vol = VOL. FUERA DEL AGUA
whistle-enabled = SILBATO ACTIVADO
buzzer-sound = SONIDO DEL ZUMBADOR
below-water-vol = VOL. BAJO EL AGUA
whistle-vol = VOL. SILBATO
cancel = CANCELAR
save = GUARDAR
apply = APLICAR
beep-test-edit-selected = Seleccionado: { $level }
beep-test-edit-time = Tiempo
beep-test-edit-count = Conteo
beep-test-edit-new = + AÑADIR
beep-test-edit-remove = ELIMINAR NIVEL
```

- [ ] **Step 4: Other 12 locales — English placeholders**

For de-DE, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN — copy the English values from en-US verbatim. Match the existing convention.

- [ ] **Step 5: Verify and commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

Verify no missing-key build warnings remain for the new keys.

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/translations/ && \
git commit -m "feat(refbox): add beep-test redesign translation keys for all locales"
```

---

## Task 10: Operator walkthrough

**Goal:** Run through the 13 walkthrough scenarios from the spec (A through M). The operator drives; Claude launches refbox and records pass/fail per scenario.

**Files:** None (notes appended to a new walkthrough doc).

### Steps

- [ ] **Step 1: Launch refbox**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
WAYLAND_DISPLAY= cargo run -p refbox
```

(Background launch with `dangerouslyDisableSandbox: true`.)

- [ ] **Step 2: Walk through scenarios A–M**

From the spec's walkthrough table. Operator reports pass/fail per scenario. Any fail is investigated and fixed before the next task.

- [ ] **Step 3: Document results**

Create `docs/superpowers/notes/2026-05-19-beep-test-redesign-walkthrough.md` with the
scenario table and pass/fail per row, including any deviations or follow-up items.

- [ ] **Step 4: Commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add docs/superpowers/notes/2026-05-19-beep-test-redesign-walkthrough.md && \
git commit -m "docs(refbox): record beep-test redesign walkthrough results"
```

---

## After Task 10: code review and PR readiness

When all 10 tasks are complete and the walkthrough passes:

- Run `superpowers:requesting-code-review` for a final review of the branch.
- Address any code-review findings as additional commits.
- PR title: `feat(refbox): redesign beep-test mode UI and Settings sub-pages`
- PR body follows `.claude/rules/pr-review.md`:
  - **What changed:** plain-English summary of the new UI and the dropped Pre state
  - **Why:** the absorbed UI was a placeholder; this redesign matches refbox conventions
  - **Scope:** refbox crate only; explicit out-of-scope items match the spec
  - **How to verify:** the 13 walkthrough scenarios from the spec
- Branch 2 PR is stacked on Branch 1 PR. Branch 1 must merge first, then Branch 2's base re-targets to master and the PR is reviewed.

---

## Spec coverage check

| Spec requirement | Covered by |
|------------------|-----------|
| Drop `Pre` state | Task 1 |
| Reset disabled until first Start | Task 2 |
| New main view layout (timer bar, widgets, table, highlight) | Task 3 |
| Settings landing (2×2) | Task 4 |
| Sound Settings (3×2 with gating) | Task 5 |
| Edit Levels (interactive transposed table) | Task 6 |
| App Mode (cycle button) | Task 7 |
| Language reuse | Task 8 |
| All 15 locales | Task 9 |
| Walkthrough scenarios A–M | Task 10 |
| `just check` clean | Every task ends with `just check` |
