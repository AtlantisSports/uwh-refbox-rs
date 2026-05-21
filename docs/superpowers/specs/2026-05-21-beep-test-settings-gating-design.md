# Beep-Test Settings Gating While Running — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 2 at `edcb916`)
**Chunk:** 3 of 6 follow-on improvements to the beep-test mode

---

## Goal

Prevent the operator from accidentally changing dangerous configuration —
the level table, the operating mode, the language, or the staged-mode
restart — while a beep test is in progress or paused mid-run. Only the
ready state (no run started yet, or after a Reset) allows access to those
controls. Sound settings remain accessible at all times so the operator can
adjust volume / mute mid-run if needed.

---

## Motivation

After Chunk 2, the BeepTest main page distinguishes three operator states
via `beep_test_has_run`:

- **Ready** — fresh launch or post-Reset (`!has_run`).
- **Running** — clock is ticking (`has_run && clock_running`).
- **Paused** — mid-run, clock stopped (`has_run && !clock_running`).

The Settings landing currently treats all three states identically: every
control is pressable. The operator's feedback (paraphrased): editing the
level table, switching App Mode, or pressing RESTART TO APPLY mid-session
either loses the in-progress run or puts the engine into a confusing state.
Only Sound Settings is safe (and useful) to expose mid-run.

---

## Scope

### Files touched

- `refbox/src/app/view_builders/beep_test_settings.rs` —
  `build_beep_test_settings_landing` gains a `has_run` parameter and gates
  four controls on it.
- `refbox/src/app/mod.rs` — the dispatch site for the landing passes
  `self.beep_test_has_run` through.

### Not touched

- Sub-pages themselves (`build_beep_test_sound_settings_page`,
  `build_beep_test_edit_levels_page`, `build_beep_test_language_picker`).
  By construction the operator can only reach Sound Settings while a run
  is in progress — the other sub-pages aren't reachable because the gate
  on the landing blocks navigation into them.
- The SETTINGS button on the main page itself — operator can reach the
  Settings landing at any time, including mid-run, to adjust sound.
- The main-page bottom-row buttons (Chunk 2).
- The BACK button on the landing — always pressable.
- Any translation key.
- Any theme/style addition.

---

## Design

### One flag drives all four gates

The landing builder receives `has_run: bool` (the value of
`self.beep_test_has_run`). Every gated control checks `!has_run` to decide
whether to render as interactive or disabled.

### Gating per button

| Button             | `has_run = false` (ready)                              | `has_run = true` (running or paused)       |
|--------------------|--------------------------------------------------------|--------------------------------------------|
| **SOUND SETTINGS** | `light_gray_button` + `on_press` (unchanged)           | `light_gray_button` + `on_press` (unchanged) |
| **EDIT LEVELS**    | `light_gray_button` + `on_press`                       | `gray_button`, no `on_press`               |
| **APP MODE** (`make_value_button`) | `Some(CycleParameter(Mode))` (interactive)             | `None` → iced renders disabled-style       |
| **LANGUAGE**       | `light_gray_button` + `on_press`                       | `gray_button`, no `on_press`               |
| **RESTART TO APPLY** | Shown when `staged_mode != config.mode`, `blue_button` + `on_press` | Hidden (filler cell in its place) |

### Why disable RESTART TO APPLY by hiding rather than graying

The button is already conditional (only shown when there's a staged
change). Adding a gray-while-running state would introduce a third visual,
which is more visual noise than the operator needs. Hiding it during
`has_run` keeps the bottom row's stable 3-cell layout consistent with how
"no staged change" already works.

### Why APP MODE uses `make_value_button(..., None)` directly

Iced 0.13's button-without-on_press fades to a gray disabled style — exactly
the look the operator wants for "you can't change this right now." On the
top-row info tiles (Task 2 fix-up) the disabled look was wrong because the
tiles aren't logically disabled, just non-interactive. Here the disabled
look is correct: the button IS disabled. No theme change needed; the
existing iced default does the right thing.

### Threading `has_run` to the call site

In `refbox/src/app/mod.rs` around the existing call:

```rust
build_beep_test_settings_landing(&self.config, staged_mode)
```

becomes:

```rust
build_beep_test_settings_landing(&self.config, staged_mode, self.beep_test_has_run)
```

The landing's function signature gains a third parameter:

```rust
pub(in super::super) fn build_beep_test_settings_landing<'a>(
    config: &Config,
    staged_mode: Mode,
    has_run: bool,
) -> Element<'a, Message>
```

---

## Acceptance criteria

Walking through the running refbox in BeepTest mode:

1. **Fresh launch → Settings.** All five gateable controls are enabled
   visuals. EDIT LEVELS / SOUND SETTINGS / LANGUAGE buttons look interactive.
   APP MODE tile is interactive. RESTART TO APPLY is hidden (no staged
   change yet).
2. **Cycle App Mode to a different mode.** RESTART TO APPLY appears (blue).
3. **Press BACK, press START on main page, return to Settings.**
   - SOUND SETTINGS still pressable.
   - EDIT LEVELS rendered gray, no response on tap.
   - APP MODE tile shows current staged mode but disabled-grayed (no cycling).
   - LANGUAGE rendered gray, no response on tap.
   - RESTART TO APPLY no longer visible — filler in its slot.
4. **Press PAUSE on main page, return to Settings.** Same as #3 (still gated;
   `has_run` is still true while paused).
5. **Press RESET on main page, return to Settings.** All controls return to
   interactive visuals. RESTART TO APPLY visible again because the staged
   mode change still differs from the live mode.

A `just check` pass plus the above walkthrough is the verification bar.

---

## Out of scope (intentionally deferred)

- Any gating on sub-pages themselves. Operator can only reach Sound
  Settings during a run; the other sub-pages aren't reachable because
  the landing-page gate blocks navigation.
- Any change to the SETTINGS button on the main page (always pressable).
- Any change to BACK button behavior (always pressable).
- Any change to RESTART TO APPLY when in the ready state — it stays
  driven by `staged_mode != config.mode`.
- The remaining chunks (4–6): total lap count, warmup countdown, LED panel
  score hiding.
