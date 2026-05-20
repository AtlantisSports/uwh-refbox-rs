# Beep-Test Pause / Resume / Reset-While-Running — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the BeepTest main page's bottom-row buttons reflect the operator's mental model of starting, pausing, resuming, and resetting — and prevent accidental Reset while the engine is running.

**Architecture:** A label + style swap on the existing bottom-row buttons, driven by the existing `clock_running` and `beep_test_has_run` flags, plus a one-line change to the `BeepTestReset` handler so the START/RESUME distinction works correctly across a reset boundary. Translation surgery rolls `beep-test-stop` over to `beep-test-pause` and adds `beep-test-resume` in all 15 locales.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13, Fluent translations via `fl!`.

**Spec:** `docs/superpowers/specs/2026-05-20-beep-test-pause-resume-design.md` (committed at `29960c5`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work). No per-task deviation commits. Final walkthrough at the end.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/translations/<locale>/refbox.ftl` × 15 | Rename `beep-test-stop` → `beep-test-pause`; add `beep-test-resume` | Task 1 |
| `refbox/src/app/view_builders/beep_test.rs` | Bottom-row button state table (START/PAUSE/RESUME and RESET-disabled-while-running) | Task 1 |
| `refbox/src/app/mod.rs` | Clear `beep_test_has_run` in the `BeepTestReset` handler; unit test for the new behavior | Task 2 |

No new files. Translation and view changes must commit together because the view references the new keys (build would break between Task 1's two stages otherwise). Task 2 is independent.

---

## Style constants (for reference)

From `refbox/src/app/theme/`:
- `green_button`, `yellow_button`, `blue_button`, `red_button`, `gray_button` — already in scope inside `beep_test.rs` via `use super::*;`.

---

### Task 1: Translations + bottom-row state table

**Files:**
- Modify: all 15 of `refbox/translations/<locale>/refbox.ftl` (de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN)
- Modify: `refbox/src/app/view_builders/beep_test.rs` (bottom-row block around lines 91–110)

This is a single combined commit because the view code references the new translation keys; splitting them would produce a transient build break.

- [ ] **Step 1: Rename `beep-test-stop` → `beep-test-pause`, update its value**

In each `refbox.ftl`, locate the existing line:

```ftl
beep-test-stop = STOP
```

(or the locale-specific value). Rename the key and update the value per the locale table below.

| Locale | New value |
|--------|-----------|
| `en-US`, `de-DE`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN` | `PAUSE` |
| `es` | `PAUSAR` (was `PARAR`) |
| `fr` | `PAUSE` (was `ARRÊTER`) |

Concretely: every locale's `beep-test-stop = <old-value>` becomes `beep-test-pause = <new-value>`.

- [ ] **Step 2: Add `beep-test-resume` key**

Append immediately after `beep-test-pause` in each locale:

| Locale | Value |
|--------|-------|
| `es` | `REANUDAR` |
| `fr` | `REPRENDRE` |
| All other 13 locales | `RESUME` |

- [ ] **Step 3: Rewrite the bottom-row block in `beep_test.rs`**

Replace the existing `start_stop` and `reset` blocks (currently lines ~91–110) with the four-state logic:

```rust
// ----- Bottom action row -----
//
// Four states drive the START/PAUSE/RESUME button and whether RESET is
// pressable:
//   (clock_running, has_run)
//   (false, false) → START (green)    + RESET disabled
//   (true,  true)  → PAUSE (yellow)   + RESET disabled
//   (false, true)  → RESUME (blue)    + RESET enabled
//   (true,  false) is unreachable: BeepTestStart sets has_run=true
//                    before starting the engine.
let start_stop = if clock_running {
    make_button(fl!("beep-test-pause"))
        .style(yellow_button)
        .on_press(Message::BeepTestStop)
} else if has_run {
    make_button(fl!("beep-test-resume"))
        .style(blue_button)
        .on_press(Message::BeepTestStart)
} else {
    make_button(fl!("beep-test-start"))
        .style(green_button)
        .on_press(Message::BeepTestStart)
};

// Reset is disabled both before any run AND while the engine is running.
// It is only pressable when `has_run` is true and the clock is stopped.
let reset = if has_run && !clock_running {
    make_button(fl!("beep-test-reset"))
        .style(red_button)
        .on_press(Message::BeepTestReset)
} else {
    make_button(fl!("beep-test-reset")).style(gray_button)
};
```

The previous code used `orange_button` for STOP; this is replaced with `yellow_button` for PAUSE. The RESET condition changes from `if has_run` to `if has_run && !clock_running`. The new `else if has_run` branch is added for RESUME between the two existing branches.

If `orange_button` is no longer referenced in this file, no other change is needed (the import is via `use super::*;` and unused symbols there don't cause clippy warnings).

- [ ] **Step 4: Verify build**

```
just check
```

Must pass. Pay attention to clippy — an unused variable or stale key reference would surface here.

- [ ] **Step 5: Commit**

```
git add refbox/translations refbox/src/app/view_builders/beep_test.rs
git commit -m "feat(refbox): beep-test bottom row shows START/PAUSE/RESUME"
```

Include the Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Clear `has_run` on Reset

**Files:**
- Modify: `refbox/src/app/mod.rs` (the `BeepTestReset` arm, currently around line 3238)

No unit test is added: `RefBoxApp::new` takes a `RefBoxAppFlags` struct with many runtime-dependent fields (`serial_ports`, `binary_port`, `json_port`, `sim_children`, etc.) and the test harness for `RefBoxApp` doesn't have a minimal constructor. Building one just to assert a single bool would mean wiring half the app's startup. Per `.claude/rules/plan-execution.md` lean process, mechanical refbox UI handler changes can skip unit-test ceremony and rely on the walkthrough at Task 3 for verification.

- [ ] **Step 1: Make the source change**

Locate `Message::BeepTestReset` in `refbox/src/app/mod.rs` (search for `BeepTestReset =>`). The current body looks like:

```rust
Message::BeepTestReset => {
    if let Some(ref mut bt_tm) = self.beep_test_tm {
        bt_tm.reset_beep_test_now(Instant::now());
    }
    self.beep_test_snapshot = BeepTestSnapshot::default();
    Task::none()
}
```

Add one line after `self.beep_test_snapshot = BeepTestSnapshot::default();`:

```rust
Message::BeepTestReset => {
    if let Some(ref mut bt_tm) = self.beep_test_tm {
        bt_tm.reset_beep_test_now(Instant::now());
    }
    self.beep_test_snapshot = BeepTestSnapshot::default();
    self.beep_test_has_run = false;
    Task::none()
}
```

- [ ] **Step 2: Verify build**

```
just check
```

Must pass.

- [ ] **Step 3: Commit**

```
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): BeepTestReset clears beep_test_has_run"
```

Include Co-Authored-By.

---

### Task 3: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(Per memory: `--manifest-path` is preferred over `cd && cargo run` for background launches.)

- [ ] **Step 2: Verify each acceptance criterion from the spec**

In BeepTest mode (switch from Settings → Mode if needed):

1. **Fresh state.** Bottom-right button reads **START** in green. RESET is gray.
2. **Press START.** Engine ticks. Button reads **PAUSE** in yellow. RESET is gray.
3. **Press PAUSE.** Engine stops. Button reads **RESUME** in blue. RESET is red and pressable.
4. **Press RESUME.** Engine resumes from where it left off. Button reads **PAUSE** in yellow. RESET is gray.
5. **Press PAUSE then RESET.** Engine clears. Button reads **START** in green. RESET is gray.

Report any failure with the specific step and observed behavior.

- [ ] **Step 3: Run the full check**

```
just check
```

Must pass.

- [ ] **Step 4: Hand back to operator**

Report walkthrough results. Do not push. Do not open a PR — branch held for stacked PR (per memory `project_beep_test_redesign_b2_complete`).

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:** the four-state button table (spec §Design) maps to Task 1 Step 3; reset handler change maps to Task 2; translation surgery maps to Task 1 Steps 1–2; walkthrough scenarios 1–5 map to Task 3 Step 2.
- **Type consistency:** no new types introduced. All references match (`yellow_button`, `blue_button`, `green_button`, `red_button`, `gray_button` already in scope via `use super::*;`).
- **No placeholders:** all steps have concrete code/commands. Task 2 Step 1 acknowledges a fallback (skip the unit test if RefBoxApp construction is impractical) — this is an intentional escape valve, not a placeholder.
- **Lean process:** three tasks, ~250 lines of plan. Single combined commit for Task 1 to avoid transient build break. Walkthrough verification at end.
