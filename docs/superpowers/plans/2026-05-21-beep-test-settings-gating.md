# Beep-Test Settings Gating While Running — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gate the four "dangerous" controls on the BeepTest Settings landing (EDIT LEVELS, APP MODE, LANGUAGE, RESTART TO APPLY) on `beep_test_has_run`. Only the ready state (no run started, or after Reset) allows them to be pressable. SOUND SETTINGS stays always-pressable; BACK is unchanged.

**Architecture:** Thread `self.beep_test_has_run` from the app to the settings-landing view builder via a new `has_run: bool` parameter. The builder uses `!has_run` to choose between interactive and disabled rendering for each gated control. RESTART TO APPLY's existing show-when-`staged_mode != live` condition gains an AND `!has_run` clause; it hides rather than greys when disabled, preserving the existing 3-cell bottom-row layout.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-settings-gating-design.md` (committed at `86b2edf`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work). No per-task deviation commits. Final walkthrough at the end.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/view_builders/beep_test_settings.rs` | `build_beep_test_settings_landing` gains `has_run` param; gates four controls on it | Task 1 |
| `refbox/src/app/mod.rs` | Dispatch site for the landing passes `self.beep_test_has_run` | Task 1 |

One task for the code change (signature change forces view + call site in the same commit). One task for walkthrough.

---

### Task 1: Thread `has_run` and gate the four controls

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (function `build_beep_test_settings_landing`, lines 41–108-ish)
- Modify: `refbox/src/app/mod.rs` (the call site for `build_beep_test_settings_landing`, around line 3824)

Also bundle the deferred Chunk 2 deviation note into this commit (per memory `feedback_no_standalone_deviation_commits` — never make standalone doc-only deviation commits):
- Modify: `docs/superpowers/plans/2026-05-20-beep-test-pause-resume.md` — append a Deviations entry for the `CellState::ActivePaused` addition from commit `edcb916` (operator walkthrough finding folded into Chunk 2 after the original plan).

- [ ] **Step 1: Update the function signature**

In `refbox/src/app/view_builders/beep_test_settings.rs`, change:

```rust
pub(in super::super) fn build_beep_test_settings_landing<'a>(
    config: &Config,
    staged_mode: Mode,
) -> Element<'a, Message> {
```

to:

```rust
pub(in super::super) fn build_beep_test_settings_landing<'a>(
    config: &Config,
    staged_mode: Mode,
    has_run: bool,
) -> Element<'a, Message> {
```

- [ ] **Step 2: Gate SOUND SETTINGS, EDIT LEVELS, LANGUAGE on `!has_run`**

SOUND SETTINGS stays unchanged (the spec keeps it always-pressable). For EDIT LEVELS and LANGUAGE, change the existing single-style construction to a conditional one mirroring the existing disabled-vs-enabled pattern used elsewhere in the codebase (e.g. the REMOVE LEVEL button at lines 472–479).

Current (around lines 49–51 and 62–64):

```rust
let edit_levels_button = make_button(fl!("beep-test-edit-levels"))
    .style(light_gray_button)
    .on_press(Message::BeepTestEditOpenLevels);
```

```rust
let language_button = make_button(fl!("language"))
    .style(light_gray_button)
    .on_press(Message::BeepTestEditOpenLanguage);
```

Replace with the disabled-aware pattern:

```rust
let edit_levels_button = if has_run {
    make_button(fl!("beep-test-edit-levels")).style(gray_button)
} else {
    make_button(fl!("beep-test-edit-levels"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenLevels)
};
```

```rust
let language_button = if has_run {
    make_button(fl!("language")).style(gray_button)
} else {
    make_button(fl!("language"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenLanguage)
};
```

SOUND SETTINGS — leave alone:

```rust
let sound_button = make_button(fl!("sound-settings"))
    .style(light_gray_button)
    .on_press(Message::BeepTestEditOpenSound);
```

- [ ] **Step 3: Gate APP MODE via `Some`/`None` on its message**

The APP MODE button uses `make_value_button` (not `make_button`). Passing `None` for the message produces iced's disabled-style rendering, which is exactly what we want here.

Current (around lines 55–60):

```rust
let app_mode_button = make_value_button(
    fl!("app-mode"),
    staged_mode.to_string(),
    (false, true),
    Some(Message::CycleParameter(CyclingParameter::Mode)),
);
```

Change the message argument to be conditional on `has_run`:

```rust
let app_mode_button = make_value_button(
    fl!("app-mode"),
    staged_mode.to_string(),
    (false, true),
    if has_run {
        None
    } else {
        Some(Message::CycleParameter(CyclingParameter::Mode))
    },
);
```

- [ ] **Step 4: Gate RESTART TO APPLY by hiding it during `has_run`**

The bottom row currently has a conditional that shows RESTART TO APPLY only when `staged_mode != config.mode`. Add the `!has_run` condition so that during a run (or paused-mid-run) the button is hidden — filler in its place — even if a mode change is staged.

Current (around lines 78–92):

```rust
let bottom_row: Element<'a, Message> = if staged_mode != config.mode {
    let restart_button = make_button(fl!("restart-to-apply"))
        .style(blue_button)
        .on_press(Message::BeepTestRestartToApply);
    row![back_button, horizontal_space(), restart_button]
        .spacing(SPACING)
        .into()
} else {
    row![back_button, horizontal_space(), horizontal_space()]
        .spacing(SPACING)
        .into()
};
```

Change the outer guard from `if staged_mode != config.mode` to `if staged_mode != config.mode && !has_run`:

```rust
let bottom_row: Element<'a, Message> = if staged_mode != config.mode && !has_run {
    let restart_button = make_button(fl!("restart-to-apply"))
        .style(blue_button)
        .on_press(Message::BeepTestRestartToApply);
    row![back_button, horizontal_space(), restart_button]
        .spacing(SPACING)
        .into()
} else {
    row![back_button, horizontal_space(), horizontal_space()]
        .spacing(SPACING)
        .into()
};
```

The comment immediately above (currently lines 78–80) should be updated to reflect the additional condition:

```rust
// Bottom row keeps a stable 3-cell layout. When the staged mode differs
// from the live mode AND the operator is in the ready state (no run
// started yet, or post-Reset), the right cell becomes a blue RESTART TO
// APPLY button; otherwise it stays a filler so the BACK button doesn't
// shift. Hiding restart-to-apply during a run avoids losing an
// in-progress beep test to an accidental restart.
```

- [ ] **Step 5: Update the call site in `refbox/src/app/mod.rs`**

Find the call to `build_beep_test_settings_landing` (around line 3824). It looks like:

```rust
build_beep_test_settings_landing(&self.config, staged_mode)
```

Change to:

```rust
build_beep_test_settings_landing(&self.config, staged_mode, self.beep_test_has_run)
```

If the surrounding context provides `staged_mode` differently (e.g., via destructuring), preserve that; only the third argument is new.

- [ ] **Step 6: Append Chunk 2 deviation note to its plan file**

Edit `docs/superpowers/plans/2026-05-20-beep-test-pause-resume.md`. In its Deviations section, append:

```markdown
### Walkthrough fix-up: `CellState::ActivePaused` (commit `edcb916`)

After Task 3's walkthrough, the operator asked that the active lap cell
in the levels table change color from yellow to blue when the engine
is paused (matching the RESUME button color). Added a new
`CellState::ActivePaused` variant to `beep_test.rs`, threaded
`clock_running` into `build_levels_table`, and routed the running-state
active lap to `Active` (yellow) and the paused-state active lap to
`ActivePaused` (blue → `blue_container`). Original spec only specified
the bottom-row button colors; this extends the same scheme to the table.
```

(This is being bundled into the Task 1 commit rather than its own doc commit per the lean-process rule against standalone deviation commits.)

- [ ] **Step 7: Verify build**

```
just check
```

Must pass.

- [ ] **Step 8: Commit (everything together)**

```
git add refbox/src/app/view_builders/beep_test_settings.rs refbox/src/app/mod.rs docs/superpowers/plans/2026-05-20-beep-test-pause-resume.md
git commit -m "feat(refbox): gate beep-test settings buttons on has_run"
```

Include Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(Per memory `feedback_cd_worktree_before_cargo`: `--manifest-path` form is preferred over `cd && cargo run` for background launches, because cd-prefix can silently drop.)

- [ ] **Step 2: Verify the five spec acceptance criteria**

Open BeepTest mode (Settings → App config → Mode → BeepTest if not already), then:

1. **Fresh launch → Settings landing.** Sound, Edit Levels, App Mode, Language all interactive. Restart-to-apply hidden.
2. **Cycle App Mode** to a different mode (e.g., Hockey 6v6). Restart-to-apply appears, blue.
3. **Back to main page, press START.** Return to Settings landing.
   - Sound Settings still interactive.
   - Edit Levels grayed, no tap response.
   - App Mode tile shows current staged mode, disabled-grayed; tapping does nothing.
   - Language grayed.
   - Restart-to-apply gone — filler in its place.
4. **Back to main page, press PAUSE.** Return to Settings landing. Same as #3 (still gated).
5. **Back to main page, press RESET.** Return to Settings landing. All controls return to interactive. Restart-to-apply visible again (staged mode change persists across Reset since it's part of `edited_settings`, not the live config).

If #5 fails because the staged mode change was discarded by Reset, that's a separate bug worth flagging but doesn't fail this chunk's acceptance (it'd be a problem with Reset, not gating).

- [ ] **Step 3: Run the full check**

```
just check
```

Must pass.

- [ ] **Step 4: Hand back to operator**

Report walkthrough results. Do not push. Branch held for stacked PR.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:** SOUND/EDIT LEVELS/APP MODE/LANGUAGE gating maps to Task 1 Steps 2–3; RESTART TO APPLY gating maps to Step 4; threading maps to Steps 1, 5. Walkthrough acceptance maps to Task 2.
- **Type consistency:** `has_run: bool` is the single new parameter; same name throughout. No new types.
- **No placeholders:** all code blocks are complete. Each step shows concrete changes.
- **Lean process:** two tasks; one combined code commit (signature change forces view + call site together); walkthrough at end.
- **Bundled Chunk 2 deviation note:** Task 1 Step 6 explicitly folds the prior chunk's missing deviation note into this commit, satisfying `feedback_no_standalone_deviation_commits`.
