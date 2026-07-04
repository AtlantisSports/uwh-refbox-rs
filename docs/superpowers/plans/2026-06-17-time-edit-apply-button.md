# Time-Edit "Apply" Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On the refbox time-edit screen, rename the green "Done" button to "Apply" and gray it out whenever the edited time(s) match what they were when the screen opened.

**Architecture:** Capture the original game/timeout durations into one new field on `RefBoxApp` when the time-edit screen is entered. Pass them into `build_time_edit_view`, which compares them against the live edited values and uses iced's "button with no press-action renders grayed" behavior to disable Apply when nothing has changed. No changes to the `AppState::TimeEdit` tuple, so the timeout-start handlers are untouched.

**Tech Stack:** Rust 2024, iced 0.13, fluent translations (`fl!`).

## Global Constraints

- MSRV Rust 1.85; Edition 2024.
- Clippy `-D warnings` must pass: `cargo clippy -p refbox -- -D warnings` (no `--all-targets` locally; mirrors CI/`just lint`).
- All user-facing text goes through `fl!` translation keys. The `apply` key already exists in all 15 locales (used on the config page) — reuse it, do NOT add a new key.
- Literal label value: `Apply` (via existing `fl!("apply")`). Do not rename "Done" anywhere else.
- Scope is `refbox` only: `refbox/src/app/mod.rs` and `refbox/src/app/view_builders/time_edit.rs`. No `uwh-common`, no wire format.
- Lean process (refbox UI): one commit for the feature is fine; verification is the manual walkthrough plus the one unit test below.

---

### Task 1: Remember the original time(s) when the time-edit screen opens

**Files:**
- Modify: `refbox/src/app/mod.rs` (struct field ~line 98–99; constructor ~line 1311–1312; `Message::EditTime` handler ~line 1381–1394)

**Interfaces:**
- Produces: a new field `time_edit_old: (Duration, Option<Duration>)` on `RefBoxApp`, holding `(original_game_time, original_timeout_time)` captured at edit entry. Read by Task 2's view dispatch.

- [ ] **Step 1: Add the field to the `RefBoxApp` struct**

In `refbox/src/app/mod.rs`, in the `pub struct RefBoxApp { ... }` block, next to `last_app_state` (around line 99), add:

```rust
    // The game/timeout clock times captured when the time-edit screen was opened,
    // used to gray out the Apply button when no change has been made.
    time_edit_old: (Duration, Option<Duration>),
```

- [ ] **Step 2: Initialize the field in the constructor**

In the `RefBoxApp { ... }` struct literal (around line 1311, next to `last_app_state: initial_app_state,`), add:

```rust
            time_edit_old: (Duration::ZERO, None),
```

(`Duration` is already imported in this file; if the compiler reports it missing, use `std::time::Duration::ZERO`.)

- [ ] **Step 3: Capture the originals in the `EditTime` handler**

In `Message::EditTime =>` (around lines 1381–1394), the captured time/timeout are already computed for the new `AppState::TimeEdit`. Store the same values into the new field. Replace the handler body so the captured durations are reused for both the state and the new field:

```rust
            Message::EditTime => {
                let now = Instant::now();
                let mut tm = self.tm.lock().unwrap();
                let was_running = tm.clock_is_running();
                tm.stop_clock(now).unwrap();
                let game_time = tm.game_clock_time(now).unwrap();
                let timeout_time = tm.timeout_clock_time(now);
                self.time_edit_old = (game_time, timeout_time);
                self.last_app_state = self.app_state.clone();
                self.app_state = AppState::TimeEdit(was_running, game_time, timeout_time);
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
```

- [ ] **Step 4: Build to verify it compiles**

Run: `cargo build -p refbox`
Expected: builds cleanly (the field is set but not yet read; that is wired in Task 2). If clippy warns about the field being unused, that resolves in Task 2 — do not add `#[allow]`.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): capture original time when entering time edit"
```

---

### Task 2: Rename "Done" → "Apply" and gray it out when unchanged

**Files:**
- Modify: `refbox/src/app/view_builders/time_edit.rs` (signature + footer buttons; add a small pure helper + test)
- Modify: `refbox/src/app/mod.rs` (the `AppState::TimeEdit` view-dispatch arm ~line 3747)

**Interfaces:**
- Consumes: `time_edit_old: (Duration, Option<Duration>)` from Task 1.
- Produces: `build_time_edit_view(data, time, timeout_time, old_time, old_timeout_time)` — new trailing params `old_time: Duration`, `old_timeout_time: Option<Duration>`.

- [ ] **Step 1: Write the failing test for the change-detection helper**

Append to the bottom of `refbox/src/app/view_builders/time_edit.rs`:

```rust
/// Returns true when either the game or timeout clock differs from the values
/// captured when the time-edit screen was opened.
fn time_edit_has_changes(
    time: Duration,
    timeout_time: Option<Duration>,
    old_time: Duration,
    old_timeout_time: Option<Duration>,
) -> bool {
    time != old_time || timeout_time != old_timeout_time
}

#[cfg(test)]
mod tests {
    use super::time_edit_has_changes;
    use std::time::Duration;

    #[test]
    fn no_change_is_false() {
        let t = Duration::from_secs(432);
        assert!(!time_edit_has_changes(t, None, t, None));
        assert!(!time_edit_has_changes(
            t,
            Some(Duration::from_secs(30)),
            t,
            Some(Duration::from_secs(30))
        ));
    }

    #[test]
    fn game_time_change_is_true() {
        assert!(time_edit_has_changes(
            Duration::from_secs(433),
            None,
            Duration::from_secs(432),
            None
        ));
    }

    #[test]
    fn timeout_change_is_true() {
        let t = Duration::from_secs(432);
        // A timeout started during edit: original None, now Some.
        assert!(time_edit_has_changes(t, Some(Duration::from_secs(60)), t, None));
    }

    #[test]
    fn round_trip_back_to_original_is_false() {
        // +1s then -1s returns to the exact original duration.
        let original = Duration::from_secs(432);
        let after_round_trip = original + Duration::from_secs(1) - Duration::from_secs(1);
        assert!(!time_edit_has_changes(after_round_trip, None, original, None));
    }
}
```

- [ ] **Step 2: Run the test to verify it compiles and passes the helper logic**

Run: `cargo test -p refbox time_edit`
Expected: the four tests in `time_edit::tests` PASS. (The helper is pure equality, so they pass immediately — this locks the behavior against future edits.)

- [ ] **Step 3: Extend `build_time_edit_view` signature and footer**

In `refbox/src/app/view_builders/time_edit.rs`, change the function signature (lines 9–13) to accept the originals:

```rust
pub(in super::super) fn build_time_edit_view<'a>(
    data: ViewData<'_, '_>,
    time: Duration,
    timeout_time: Option<Duration>,
    old_time: Duration,
    old_timeout_time: Option<Duration>,
) -> Element<'a, Message> {
```

Then replace the footer `row![ ... ]` that builds the Cancel/Done buttons (lines 51–62) with:

```rust
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    time_edit_has_changes(time, timeout_time, old_time, old_timeout_time)
                        .then_some(Message::TimeEditComplete { canceled: false })
                ),
        ]
        .spacing(SPACING),
```

(When `on_press_maybe` gets `None`, iced renders the button grayed/disabled — the established pattern in this codebase. See `reference_iced_button_no_onpress_disabled`.)

- [ ] **Step 4: Pass the originals from the view dispatch**

In `refbox/src/app/mod.rs`, update the `AppState::TimeEdit` arm (around line 3747):

```rust
            AppState::TimeEdit(_, time, timeout_time) => build_time_edit_view(
                data,
                time,
                timeout_time,
                self.time_edit_old.0,
                self.time_edit_old.1,
            ),
```

- [ ] **Step 5: Build, lint, and test**

Run:
```bash
cargo build -p refbox
cargo clippy -p refbox -- -D warnings
cargo test -p refbox time_edit
```
Expected: all clean; the `time_edit::tests` pass.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/time_edit.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): relabel time-edit Done to Apply and disable when unchanged"
```

---

### Task 3: Manual walkthrough (the operator-visible acceptance test)

**Files:** none (verification only)

- [ ] **Step 1: Build the runnable binary** (clippy/test build a different binary — see `reference_rebuild_binary_before_walkthrough`)

Run: `cargo build -p refbox`

- [ ] **Step 2: Launch the app**

Run (background, sandbox disabled, force X11 on WSLg):
`WAYLAND_DISPLAY= ./target/debug/refbox`

- [ ] **Step 3: Verify behavior**

1. Open the time-edit screen (the screen with GAME TIME + and − buttons).
2. Confirm the green button now reads **APPLY** (not DONE).
3. Before touching anything, confirm **APPLY is grayed out / not pressable**, while **CANCEL** is fully active.
4. Press **+** once: APPLY lights up (becomes pressable).
5. Press **−** once to return to the original value: APPLY grays out again.
6. If a timeout is active, edit the timeout time and confirm APPLY also lights up.
7. Press APPLY after a real change and confirm the new time is applied (same as the old Done behavior).

**Acceptance:** All of the above observed. No regression to Cancel.

---

## Self-Review

- **Spec coverage:** Relabel Done→Apply (Task 2 Step 3); gray-out-when-unchanged (Task 1 + Task 2); reuse existing `apply` key (Task 2 Step 3); no new translation key (Global Constraints). Covered.
- **Placeholder scan:** none.
- **Type consistency:** `time_edit_old: (Duration, Option<Duration>)` set in Task 1, read in Task 2 Step 4 as `.0`/`.1`; `build_time_edit_view` new params `old_time: Duration`, `old_timeout_time: Option<Duration>` match the call site and the helper. Consistent.
