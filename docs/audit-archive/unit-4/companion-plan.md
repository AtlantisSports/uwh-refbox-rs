# Manual Alarm — Uniform Hold Model (Delta Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the already-merged manual alarm button code into alignment with the approved uniform-hold design. See `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md` for the canonical design; see `refbox/tests/features/manual_alarm.feature` for the full scenario coverage.

**Why this plan exists:** The manual alarm feature was implemented on branch `feat/refbox/manual-alarm-button` and merged into the current workspace. The original plan (`2026-04-14-manual-alarm-button.md`) encoded a mixed model (tap during active play; 1-second hold only during Between Games; disabled in other states). A subsequent design review replaced that with a **uniform hold model**: every press schedules a fire timer — 250ms during active play with no timeout, 1 second everywhere else. The button is never greyed out while the feature is enabled; colour (red/blue) and label ("Alarm" vs "Hold to Test") switch based on state.

This plan is the targeted delta to get the code there.

**Scope:** `refbox` crate only. Two files touch behaviour; one file updates the view.

---

## Summary of Behaviour Changes

| Concern | Current code | Target design |
|---|---|---|
| Fire during active play (no timeout) | Immediate `start_manual_buzzer()` on press | Schedule 250ms delay → `start_manual_buzzer()` if still held |
| Fire during Between Games | Schedule 1s delay → `start_manual_buzzer()` if still held | Unchanged |
| Fire during Half Time / Pre-OT / OT Half / Pre-Sudden Death | Disabled / greyed — no fire | Schedule 1s delay → `start_manual_buzzer()` if still held |
| Fire during a timeout in a play period | Disabled / greyed — no fire | Schedule 1s delay → `start_manual_buzzer()` if still held |
| Button colour | Red in active play; blue in Between Games; greyed elsewhere | Red in active-play-no-timeout; blue everywhere else; never greyed |
| Button label | "Alarm / Or press Spacebar" everywhere except Between Games (which shows "Hold to Test / Or hold Spacebar") | "Alarm / Or press Spacebar" in active-play-no-timeout; "Hold to Test / Or hold Spacebar" everywhere else |
| Interactivity (mouse_area) | Wrapped conditionally — greyed states are non-interactive | Always wrapped — button is always interactive while the feature is enabled |
| Pressed-state containers (`red_pressed_container`, `blue_pressed_container`) | Kept — visual feedback while held | Kept — logic needs to switch on `is_active_play` instead of `is_between_games` |
| `start_manual_buzzer()` / `stop_manual_buzzer()` usage | Already in place | Unchanged — already correct |
| Mouse-vs-spacebar independent hold tracking | Already in place | Unchanged — already correct |

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Modify | `refbox/src/app/mod.rs` | `AlarmPressed`, `SpacebarPressed`: always schedule a delay (duration depends on state, no immediate `start_manual_buzzer` branch). Drop the active-play-no-timeout fast path. Widen the state gate so all non-excluded states are eligible. |
| Modify | `refbox/src/app/view_builders/main_view.rs` | Replace `is_between_games` with `is_active_play` for colour and label selection. Remove `alarm_available` / `disabled_container` path. Always wrap the alarm face in `mouse_area`. |
| Verify | `refbox/tests/features/manual_alarm.feature` | No changes — the file already encodes the target design. Used to sanity-check behaviour after code changes. |

---

### Task 1: Unify the press handlers around a per-state delay

**Files:**
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Replace the `AlarmPressed` handler**

In `refbox/src/app/mod.rs`, find the `Message::AlarmPressed` match arm (around line 2195). Replace its entire body with:

```rust
Message::AlarmPressed => {
    // Mouse press on the alarm button.
    // Uniform hold model: always schedule a delay; duration depends on game state.
    if !(self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled) {
        return Task::none();
    }
    if self.mouse_alarm_held {
        return Task::none();
    }
    let was_active = self.spacebar_held;
    self.mouse_alarm_held = true;
    if was_active {
        return Task::none();
    }
    let hold_duration = match (self.snapshot.current_period, self.snapshot.timeout) {
        (
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath,
            None,
        ) => Duration::from_millis(250),
        _ => Duration::from_secs(1),
    };
    self.alarm_delay_token += 1;
    let token = self.alarm_delay_token;
    info!("Manual alarm delay started (mouse), duration={hold_duration:?}, token={token}");
    Task::future(async move {
        sleep(hold_duration).await;
        Message::AlarmDelayElapsed(token)
    })
}
```

Key differences from the current code:
- No immediate `start_manual_buzzer()` branch — every press goes through `AlarmDelayElapsed`.
- No early return when in a "disabled" state — every state that isn't fully blocked (i.e. feature enabled) schedules a timer.
- Hold duration: 250ms for active play with no timeout; 1 second otherwise.

- [ ] **Step 2: Replace the `SpacebarPressed` handler**

Mirror the change in `Message::SpacebarPressed` (around line 2241). Swap the roles of `mouse_alarm_held` and `spacebar_held`:

```rust
Message::SpacebarPressed => {
    // Keyboard press — spacebar_held guards against OS key-repeat.
    if !(self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled) {
        return Task::none();
    }
    if self.spacebar_held {
        return Task::none();
    }
    let was_active = self.mouse_alarm_held;
    self.spacebar_held = true;
    if was_active {
        return Task::none();
    }
    let hold_duration = match (self.snapshot.current_period, self.snapshot.timeout) {
        (
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath,
            None,
        ) => Duration::from_millis(250),
        _ => Duration::from_secs(1),
    };
    self.alarm_delay_token += 1;
    let token = self.alarm_delay_token;
    info!("Manual alarm delay started (spacebar), duration={hold_duration:?}, token={token}");
    Task::future(async move {
        sleep(hold_duration).await;
        Message::AlarmDelayElapsed(token)
    })
}
```

- [ ] **Step 3: Leave `AlarmReleased`, `SpacebarReleased`, and `AlarmDelayElapsed` unchanged**

The existing implementations are correct under the new model:
- `AlarmReleased` and `SpacebarReleased` stop the manual buzzer when no other input is held — this gives the "let current tone finish naturally" behaviour via `stop_manual_buzzer()`.
- `AlarmDelayElapsed` fires only if the token matches and at least one input is still held — this handles early-release cancellation.

No edits needed.

- [ ] **Step 4: Consider factoring the hold-duration match**

The same match block now appears in both handlers. Extract a helper (private method on `RefBoxApp`) to avoid duplication:

```rust
fn manual_alarm_hold_duration(&self) -> Duration {
    match (self.snapshot.current_period, self.snapshot.timeout) {
        (
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath,
            None,
        ) => Duration::from_millis(250),
        _ => Duration::from_secs(1),
    }
}
```

Use the helper in both press handlers. Optional but recommended for clarity.

- [ ] **Step 5: Verify compilation and tests**

```bash
cargo check -p refbox
just test
```

Expected: no errors; all tests pass.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): unify manual alarm press handlers under uniform hold model"
```

---

### Task 2: Switch the view between red/blue on active-play state

**Files:**
- Modify: `refbox/src/app/view_builders/main_view.rs`

- [ ] **Step 1: Replace `alarm_available` and `is_between_games` logic**

Find the alarm zone block in `build_main_view` (currently around lines 121–187). Replace the `alarm_available` computation, the `is_between_games` variable, the label selection, the container style selection, and the conditional `mouse_area` wrap with:

```rust
// Red + tap prompt during active play with no timeout; blue + hold prompt everywhere else.
// The button is always interactive while the feature is enabled.
let is_active_play = matches!(
    (snapshot.current_period, snapshot.timeout),
    (
        GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath,
        None,
    )
);
let alarm_label = if is_active_play {
    fl!("alarm")
} else {
    fl!("hold-to-test")
};
let spacebar_label = if is_active_play {
    fl!("or-press-spacebar")
} else {
    fl!("or-hold-spacebar")
};
let alarm_face_container = container(
    column![
        text(alarm_label)
            .size(SMALL_PLUS_TEXT)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
        text(spacebar_label)
            .size(SMALL_TEXT)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
    ]
    .align_x(Alignment::Center)
    .width(Length::Fill),
)
.style(match (is_active_play, alarm_held) {
    (true, true) => red_pressed_container,
    (true, false) => red_container,
    (false, true) => blue_pressed_container,
    (false, false) => blue_container,
})
.padding(PADDING)
.width(Length::Fill)
.height(Length::Fill)
.align_x(Horizontal::Center)
.align_y(Vertical::Center);

let alarm_face: Element<'a, Message> = mouse_area(alarm_face_container)
    .on_press(Message::AlarmPressed)
    .on_release(Message::AlarmReleased)
    .into();
```

Key differences from the current code:
- Replaces `is_between_games` with `is_active_play` as the state predicate.
- Removes `alarm_available` entirely — button is always interactive.
- Removes the `disabled_container` branch — it is no longer reachable.
- Always wraps the face in `mouse_area` (no conditional).

- [ ] **Step 2: Remove unused imports**

If `disabled_container` is no longer used anywhere in this file after Step 1, remove it from the imports at the top of `main_view.rs`. Check with:

```bash
grep -n disabled_container refbox/src/app/view_builders/main_view.rs
```

Expected: no matches after the change. If so, drop the import. If `disabled_container` is still used for another widget in this file, leave the import alone.

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p refbox
```

Expected: no errors.

- [ ] **Step 4: Run all tests**

```bash
just test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/main_view.rs
git commit -m "feat(refbox): switch alarm button colour on active-play state, not BetweenGames"
```

---

### Task 3: Full validation pass

**Files:** None — verification only.

- [ ] **Step 1: Run full workspace check**

```bash
just check
```

Expected: formatting clean, linter clean, tests pass, audit clean.

- [ ] **Step 2: Manual verification**

Run the refbox app (or simulator) and walk through the Manual Verification Checklist at the bottom of `docs/superpowers/plans/2026-04-14-manual-alarm-button.md`. All 11 items should pass.

Particular attention:
- Item 4 (active play, no timeout): a brief tap under 250ms does not fire; a deliberate hold past 250ms fires and continues.
- Items 5, 6, 7 (all non-active-play states): button is blue with "Hold to Test / Or hold Spacebar"; 1-second hold fires; short press does not.
- Items 8, 9 (layout): fouls/warnings split and full-width layouts still work.

- [ ] **Step 3: Commit the combined change notes in the PR body, not in a separate commit**

No separate commit for Task 3 — its work product is the verification pass itself, captured in the PR description.

---

## Follow-ups Not in Scope

- The plan at `docs/superpowers/plans/2026-04-14-manual-alarm-button.md` has been updated to reflect the new design (Tasks 1–8 now describe the target state). That plan is now mostly a record of the feature's design; this delta plan is what should actually be executed.
- If further refinements are needed (for example, tuning the 250ms threshold after real tournament use), track them separately — the constant is localised to `manual_alarm_hold_duration()` and can be adjusted without further structural change.
