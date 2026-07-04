# Timeout-Revive: Release-to-Confirm (remove decide window) — Design

**Date:** 2026-06-17
**Crate:** `refbox` only
**Branch:** `feat/refbox/timeout-revive-long-press` (same feature branch; refinement of the revive interaction)
**Process:** lean (state-machine simplification within the app layer + one view key rename)

## Problem

The current timeout-revive long-press has three states:

1. **RED "HOLD TO / RESTORE"** — finger down, 5-second countdown. Releasing cancels (nothing restored).
2. At 5s the timeout is restored, entering **YELLOW "TIMEOUT / RESTORED"** — a fixed **2-second** window.
3. Within that window: releasing banks the restored timeout; **holding through** the 2 seconds immediately **starts** a new team timeout (spending the just-restored timeout).

The "hold-through to start" path is an easy-to-trigger trap (hold a moment too long and you start a timeout you didn't intend), and the hidden 2-second timer makes the interaction less predictable.

## Goal

Make the restore an explicit hold-then-release gesture:

- RED still requires a ~5-second hold to trigger the restore (unchanged).
- After the restore, **YELLOW "RESTORED" persists for as long as the button is held** — no timer.
- **Releasing confirms the restore.** The timeout is back and the button returns to a normal, pressable team-timeout button. To *use* the restored timeout the operator presses that button again normally.
- The "hold-through immediately starts a timeout" behaviour is **removed**.

This is also safer: a timeout can no longer be started by the same continuous press, because the "start timeout" affordance only reappears after the finger is lifted.

## Approach (chosen: A — persistent YELLOW until release)

Considered and rejected: skipping the YELLOW state entirely (loses the useful "RESTORED" confirmation feedback the operator wants to see).

## Design

### State (`refbox/src/app/mod.rs`)

- `RevivePhase` keeps two variants. Rename `Deciding` → `Restored` (it is no longer a decision window): `RevivePhase::{ Reviving, Restored }`. Update the doc comment accordingly.
- `ReviveHold { color, phase, token }` unchanged in shape.

### Messages / handlers (`refbox/src/app/mod.rs`)

- **`TimeoutRevivePressed(color)`** — unchanged: start, `phase = Reviving`, spawn the 5s hold timer (`TimeoutReviveHoldElapsed`).
- **`TimeoutReviveHoldElapsed(token, color)`** — at 5s, if still the live `Reviving` hold: restore the timeout (`revive_team_timeout`), apply the snapshot, then enter `RevivePhase::Restored` with **no follow-up timer**. (Remove the decide-timer spawn.)
- **`TimeoutReviveReleased(color)`** — unchanged in effect: clears the hold. In `Reviving` this cancels (nothing restored); in `Restored` the timeout is already restored, so clearing simply confirms/banks it.
- **`TimeoutReviveDecideElapsed`** — **removed entirely** (message variant + handler).
- **`TIMEOUT_REVIVE_DECIDE_DURATION`** — **removed** (no longer referenced).

### View (`refbox/src/app/view_builders/shared_elements.rs`)

- The YELLOW face (both black and white) keys off `RevivePhase::Restored` instead of `RevivePhase::Deciding`. Labels unchanged: line 1 `{ timeout }` (TIMEOUT), line 2 `revive-deciding-line-2` (RESTORED).
- No new translation keys. (The existing `revive-deciding-line-2` key name is retained as-is to avoid churning all 15 locale files for a rename with no user-visible effect.)

### Message enum (`refbox/src/app/message.rs`)

- Remove the `TimeoutReviveDecideElapsed` variant.

## Out of scope

- The RED build-up duration and labels (HOLD TO / RESTORE) — unchanged.
- Label text (TIMEOUT / RESTORED) — unchanged.
- The merged Cancel/End grace-window feature — untouched.
- Renaming the `revive-deciding-line-2` translation key — intentionally left (internal name only; no user-facing change).

## Acceptance criteria (observable)

1. Hold a greyed (used-up) team-timeout button: it turns RED "HOLD TO / RESTORE".
2. Releasing during RED restores nothing (button stays greyed/used-up).
3. Holding ~5s turns it YELLOW "TIMEOUT / RESTORED" and the timeout is restored.
4. YELLOW stays as long as the button is held — it does **not** auto-start a timeout no matter how long it is held.
5. Releasing from YELLOW leaves the timeout restored and the button as a normal, available team-timeout button; pressing it then starts a timeout normally.
6. The Cancel/End grace-window behaviour on a *running* timeout is unaffected.
7. `just check` passes.
