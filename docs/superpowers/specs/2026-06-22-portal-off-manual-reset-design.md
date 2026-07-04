# Clean slate when switching the portal off (portal → manual reset)

**Date:** 2026-06-22
**Status:** design approved (brainstorming); pending written-spec review, then writing-plans
**Crate scope:** `refbox` only (no `uwh-common`, no wire format, no other crates)
**Process:** heavy — touches the game-clock engine (`tournament_manager`). Per-task
verification, full `just test`, and a golden-trace check are required.

---

## Goal

When the operator turns the **Using UWH Portal** setting OFF (switching back to manual
mode) and applies it, the refbox should return to the same state it shows on a fresh
manual launch:

- The loaded event, court, game, and schedule are cleared.
- The leftover portal-scheduled start time no longer drives the before-game countdown.
- The before-game clock returns to the **nominal break**, shown stopped.
- The saved portal token is **kept** (the operator stays logged in).

This reverses the current behavior, in which switching the portal off preserves the loaded
selections and leaves the portal's scheduled start time silently driving the manual-mode
countdown.

## Background — why the old time sticks around today

The before-game countdown prefers the loaded schedule's start time whenever one exists, and
falls back to an internal grid value otherwise
([`next_game_scheduled_start`](../../refbox/src/tournament_manager/mod.rs#L958-L972)).

Turning the portal OFF today does **not** clear that scheduled start time:

- The existing "switch to manual" cleanup only clears the internal grid fallback
  ([`clear_scheduled_game_start`](../../refbox/src/tournament_manager/mod.rs#L166-L168)),
  not the schedule-derived start time, and it only runs inside the config-change /
  game-number-change branches of the apply path
  ([apply path](../../refbox/src/app/mod.rs#L983-L988)) — so a bare portal toggle may not
  trigger it at all.
- The loaded schedule / event / court are deliberately preserved in memory on OFF per an
  accepted decision record — see ADR 017 below.

So the leftover scheduled start time keeps the old countdown visible after switching to
manual.

## Decision record impact — ADR 017

[ADR 017 "Portal Data Lifecycle"](../../docs/decisions/017-portal-data-lifecycle.md)
(proposed 2026-05-12, accepted 2026-05-16) decided, under *"Cached data on toggle
transitions"*:

> **ON → OFF:** cached `self.events` / `self.schedule` / `current_event_id` are preserved
> in memory ... No proactive clearing — keeps the toggle as a UI-level switch rather than a
> destructive state purge.

That decision was weighing wasted portal network requests, not the operator-confusion of a
leftover before-game time. The user has approved reversing it. **This work amends ADR 017's
ON → OFF sub-section** to record that switching the portal off is now a clean wipe (clear
selections + return the before-game clock to the nominal break), with the operator-clarity
rationale. The token credential is still kept (not a logout).

## Approved approach (A): reset logic in the game-clock engine, reuse the existing prompt

The one clock-touching piece lives inside the game-clock engine so it is covered by the
engine's automated tests; the app layer orchestrates it and reuses the existing mid-game
confirmation prompt.

## Behavior specification

### Trigger
The committed `using_uwhportal` value transitions from `true` to `false` and is applied.
Detected in the Game-Options apply path by comparing the prior committed
`self.using_uwhportal` against the newly applied value. The reset must fire on this
transition **regardless** of whether a game setting or game number also changed in the same
apply (today's cleanup is gated behind those changes and would otherwise be skipped).

Off → on is unchanged. Applying with the toggle unchanged is unchanged.

### The "fresh manual slate" (what resets)

App-layer state (`refbox/src/app/mod.rs`):
- `current_event_id` → `None`, routed through `set_current_event_id` so the portal-health
  shared handle stays in sync (ADR 011).
- `current_court` → `None`.
- `schedule` → `None`.
- Game number → the manual-launch default (match a fresh launch; do **not** blank it).
- `config.uwhportal.token` → **kept** (no logout).
- The staged `edited_settings` equivalents are cleared on the toggle itself, mirroring the
  existing OFF → ON blank-slate handling
  ([toggle handler](../../refbox/src/app/mod.rs#L3199-L3219)), so the editor reflects the
  cleared selections immediately.

Game-clock engine state (`refbox/src/tournament_manager/mod.rs`) — new routine:
- Clear `next_game` (drops the schedule-derived start time).
- Clear `next_scheduled_start` (the grid fallback).
- Set the before-game clock to `Stopped { clock_time: config.nominal_break }`, matching a
  fresh launch ([fresh-launch state](../../refbox/src/tournament_manager/mod.rs#L70-L73)).

### Nominal vs. minimum break (correctness detail)
A fresh launch shows the **nominal** break, but the existing end-of-game reset routine drops
the clock to the **minimum** break
([`reset_game`](../../refbox/src/tournament_manager/mod.rs#L201-L208)). The new routine must
land on the **nominal** break in every path, including the mid-game "End game & apply" path,
so the result matches a fresh launch as the user requested.

### Two situations

**Between games (normal case):** apply immediately, no prompt. Result: fresh manual slate,
clock stopped at the nominal break.

**During a live game (clock running or paused mid-period):** raise the same style of
confirmation the refbox already uses for mid-game parameter changes
([existing confirmations](../../refbox/src/app/mod.rs#L1052-L1135)), offering:
- **End game & apply** → end the current game, then land on the fresh manual slate (clock at
  the nominal break). Note: the existing end path uses the minimum break, so this path must
  explicitly land on the nominal break.
- **Keep game & apply** → the current game keeps running untouched. The portal selections
  and the schedule-derived next-game time are cleared now; when the current game ends, the
  between-games countdown naturally falls back to the nominal break via the existing
  fallback ([fallback](../../refbox/src/tournament_manager/mod.rs#L994-L997)).
- **Discard / Go back** → nothing changes.

This likely needs a new confirmation kind that mirrors the existing
`GameConfigChangedFromApply` / `GameNumberChangedFromApply` variants
([enum](../../refbox/src/app/mod.rs#L322-L329)), because switching the portal off may not
change the game config or number and so would not raise either existing confirmation. The
new variant reuses the existing `EndGameAndApply` / `KeepGameAndApply` / `DiscardChanges` /
`GoBack` options ([options](../../refbox/src/app/message.rs#L897-L906)).

## Architecture sketch (files that change)

1. `refbox/src/tournament_manager/mod.rs`
   - New routine (working name `reset_to_manual_break`) performing the engine-side reset
     above. Plus unit tests.
2. `refbox/src/app/mod.rs`
   - Detect the ON → OFF transition in the apply path; clear app-layer selections; call the
     new engine routine directly (between games) or behind the confirmation (mid-game).
   - Clear the staged `edited_settings` selections in the toggle handler (mirror OFF → ON).
   - Likely a new `ConfirmationKind` variant + its handling, reusing existing options.
3. `docs/decisions/017-portal-data-lifecycle.md`
   - Amendment recording the reversed ON → OFF behavior.

## Acceptance criteria (operator-observable)

1. Load a portal schedule; the before-game countdown shows the scheduled first-game time.
   Switch to manual **between games** → event/court/game cleared, before-game clock shows
   the **nominal break, stopped** (same as a fresh launch).
2. Switch to manual **mid-game** → the confirmation prompt appears.
   - **End game & apply** → game ends, fresh manual slate, clock at the nominal break.
   - **Keep game & apply** → game continues; after it ends, the break is the nominal break
     (no leftover portal time).
   - **Discard / Go back** → nothing changes.
3. Re-enable the portal afterward → still logged in (token kept); pickers start blank
   (existing OFF → ON behavior, unchanged).

## Testing

- Engine unit tests for the new routine: the schedule-derived next-game time and the grid
  fallback are cleared, and the clock is set to the nominal break (stopped).
- A test that a "kept" game, after ending with the portal info cleared, lands the
  between-games break on the nominal break via the existing fallback.
- `just test` green (mandatory for `tournament_manager` changes).
- Golden-trace guard: expected **unchanged** — the golden scenarios do not toggle the
  portal — but run them to confirm no drift.
- Manual walkthrough per the acceptance criteria above.

## Out of scope

- The OFF → ON (turning the portal on) blank-slate behavior — unchanged.
- Game-block / before-game timing during normal play — unchanged.
- Logout / clearing the saved portal token — explicitly kept.
- Any change outside the `refbox` crate.

## Suggested branch

`feat/refbox/portal-off-manual-reset`
