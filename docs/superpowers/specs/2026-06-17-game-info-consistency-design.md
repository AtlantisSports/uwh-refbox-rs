# Game Info button ↔ Game Information page consistency

**Date:** 2026-06-17
**Branch:** `fix/refbox/game-info-consistency` (worktree off master `3ff86b61`)
**Crate:** `refbox` only — view-layer display change. No `uwh-common`, no wire format, no game logic.
**Process:** Lean (refbox UI, display-only).

---

## Problem (confirmed against a live build of current master)

The refbox shows game settings in two places:

- **Game Info button** — the centre panel on the main referee screen, built by
  `config_string` in `refbox/src/app/view_builders/shared_elements.rs`.
- **Game Information page** — the dedicated full page (BACK / SETTINGS), built by
  `details_strings` in `refbox/src/app/view_builders/game_info.rs`.

Two inconsistencies, both verified by running the current code out of Portal Mode:

1. **Referees show on the button when not in Portal Mode.** The button prints the whole
   referee block (Chief Ref, Timer, Water Ref 1/2/3 — all "-") regardless of mode. The page
   already hides referees unless in Portal Mode. Referees only exist when using the Portal, so
   the button is wrong.
2. **"Stop Clock in Last 2 Minutes" is missing from the page.** The page tucks that line inside
   its Portal-only section, so the full "show everything" page omits it out of Portal Mode. It is
   a normal game rule, not Portal-specific, so it should always be shown.

A third, earlier-reported "extra space before the Team Timeouts row" was an artifact of an older
build; the current code renders that row cleanly. No change needed.

---

## Desired behaviour

A single consistent rule across both surfaces:

- **Referees** → shown only in Portal Mode (on both surfaces).
- **"Stop Clock in Last 2 Minutes"** → always shown (on both surfaces), positioned **directly
  above the Team Timeouts row**.
- Out of Portal Mode the box has no source for the stop-clock value, so it reads
  **"Stop Clock in Last 2 Minutes: Unknown"**. This is intentional and accepted; giving it a real
  value out of Portal Mode is out of scope (would need a new game setting in shared core code).

The button stays the **compact summary** (it does not show "Team Timeout Duration" or
"Minimum Time Between Games"); the page stays the **complete** view. The only fields that vary by
mode are the referees.

---

## Changes

### 1. Button — `config_string` (`shared_elements.rs`)

- Move the `stop-clock-last-2` line so it is emitted **before** the `team-timeouts` line.
- Wrap the referee block (the `chief_ref`/`timer`/`water_ref_*` setup, the population loop, and
  the `ref-list` emission) in `if using_uwhportal { … }`, mirroring how `details_strings`
  already gates it. Leave `stop-clock-last-2` unconditional.

**Resulting order:** game number → [Portal: teams] → Game Block → game-config (Half/Sudden
Death/Overtime) → **Stop Clock in Last 2 Minutes** → Team Timeouts → [Portal: referees].

### 2. Page — `details_strings` (`game_info.rs`)

- Move the `stop-clock-last-2` computation and emission **out** of the `if using_uwhportal`
  block so it is always emitted into the left column, positioned **directly above** the
  `team-timeouts` line.
- Leave the referee block (right column) inside `if using_uwhportal`.

**Resulting left-column order:** game number → [Portal: teams] → Game Block → Half/Overtime →
[OT details] → Sudden Death → [pre-SD] → **Stop Clock in Last 2 Minutes** → Team Timeouts →
[Team Timeout Duration] → Minimum Time Between Games. Referees stay in the right column,
Portal-only.

### No translation changes

No `.ftl` keys are added, removed, or reworded. `stop-clock-last-2` and `ref-list` already exist
in every locale. This is purely a change to *order* and *conditions* in Rust. No translation work.

---

## Acceptance criteria (observable)

Out of Portal Mode:
- The main-screen Game Info button shows **no** referee lines (no Chief Ref / Timer / Water Ref).
- The Game Information page shows **"Stop Clock in Last 2 Minutes: Unknown"**.
- On **both** surfaces, "Stop Clock in Last 2 Minutes" appears **directly above** "Team Timeouts".

In Portal Mode (unchanged from today, just reordered):
- Both surfaces show the referee lines.
- Both surfaces show the real YES/NO stop-clock value from the schedule, above Team Timeouts.

---

## Testing

- Unit test(s) on `config_string` (returns a `String`): with `using_uwhportal = false`, the
  output does **not** contain the Chief Ref label; with a Portal schedule and
  `using_uwhportal = true`, it **does**. Assert the "Stop Clock" text appears at a lower string
  index than the "Team Timeouts" text (ordering).
- Equivalent check on `details_strings` (returns `(left, right)`): the **left** string always
  contains "Stop Clock in Last 2 Minutes" and orders it above "Team Timeouts"; referee text
  appears in the **right** string only when `using_uwhportal = true`.
- Mirror any existing test patterns already present for these functions rather than inventing a
  new style.
- `just check` (fmt, clippy `-D warnings`, tests) must pass.

---

## Out of scope

- Giving "Stop Clock in Last 2 Minutes" a real value out of Portal Mode (new game setting in
  `uwh-common`).
- Reconciling cosmetic line-grouping differences between the two surfaces (e.g. the button puts
  Half/Half-Time on one line and folds Sudden Death/Overtime in via `game-config`, while the page
  splits them) — the *information* is the same; the user accepted the button as a compact summary.
- The prev/next game line wording difference between the surfaces.
- Any styling, colours, or layout.
