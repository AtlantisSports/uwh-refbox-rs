# 014 — Live Settings Preview

**Date:** 2026-04-20
**Status:** proposed

## Context

ADR 009 restructures settings into a per-page Cancel/Apply model:
each page's edits are committed to the saved config when `APPLY` is
pressed and discarded when `CANCEL` is pressed. That works cleanly
for most settings.

Some settings, however, benefit from being **previewed** while the
operator is still editing them — so the operator can confirm the
choice is what they want before committing:

- **Sound volumes and enable toggles** — an operator picking a
  whistle volume wants to **hear** the new volume before committing.
  Under the ADR 009 model alone, the new volume does not take effect
  until Apply, so the operator either commits blind or re-opens
  settings after committing to hear the result.
- **Starting sides** (`white_on_right`) — the LED panel flips its
  team-colour assignment when this toggles. An operator wants to see
  the panel flip while choosing, not only after Apply.
- **LED panel brightness** — same principle: see the new brightness
  on the panel while choosing, not only after Apply.

Other settings do not need live preview. The `Hide time for last
15 seconds` toggle on Display Options has no audible or visible
effect until a game is in its final 15 seconds — it can wait for
Apply without any loss. Game Options (clock lengths, overtime
toggles) and App Options (mode, tracking toggles) affect future
games, not the current screen, so nothing is lost by waiting for
Apply either.

The web refbox does not currently implement live preview for these
fields. This ADR documents the Rust-side enhancement so that, if
the web refbox is later updated to match, the spec is already
recorded.

## Decision

Add live-preview behaviour to a specific set of settings fields.
While the operator is editing one of the affected pages, the
subsystem that consumes the setting (sound controller, LED panel
update stream) reads from the in-flight buffer rather than the
committed config. On Apply, the buffer is persisted to config. On
Cancel, the buffer is discarded and the subsystem is pushed the
pre-edit value to undo the live preview.

### Fields with live preview

**Sound Options page** — every sound-related field behaves live:

| Field | Effect while editing |
|-------|----------------------|
| `sound_enabled` (master) | Toggling off silences any sound the refbox would emit during editing. |
| `whistle_enabled` | Toggling off silences any whistle the refbox would emit during editing. |
| `whistle_vol` | Next whistle plays at the buffered volume. |
| Other volume cycles (alert, under-water) | Next sound of that type plays at the buffered volume. |

**Display Options page:**

| Field | Effect while editing |
|-------|----------------------|
| `white_on_right` (starting sides) | LED panel and simulator flip their team-colour assignment immediately on toggle. |
| `brightness` | LED panel dims/brightens immediately as the cycle advances. |

### Fields explicitly **not** in live preview

- `hide_time` on Display Options — no live preview; takes effect on
  Apply.
- Every field on Game Options, App Options, Manage Remotes, and
  Language — no live preview; takes effect on Apply per ADR 009.

### Cancel behaviour

Pressing `CANCEL` on a page with live-preview fields must:

1. Discard the page's in-flight edits from the buffer.
2. Push the pre-edit value(s) back to the affected subsystem so the
   live preview is undone.

Example: on Display Options, if the operator toggled
`white_on_right`, pressing `CANCEL` must push the original value to
the LED panel so the panel flips back to the orientation it had
before the page was entered.

### Apply behaviour

Pressing `APPLY` on a page with live-preview fields performs the
same config write as any other ADR 009 Apply — the page's buffered
fields are persisted to the saved config. The subsystem is already
running at the new value, so no additional push is required on
Apply.

## What is **not** changing

- Per-page Cancel/Apply semantics from ADR 009 are unchanged.
- Navigation structure and chrome from ADR 009 are unchanged.
- Game rules, clock behaviour, and wire format are not affected.
- No new settings are added and no existing settings are removed —
  this ADR only changes *when* certain values reach their
  subsystems.

## Open design questions (to resolve during implementation)

- **Subsystem hooks.** The sound controller's `update_settings()`
  and the LED panel's `update_sender` both accept pushes today as
  part of the final commit. Implementation must decide whether to
  reuse those same calls during editing (simplest) or introduce a
  distinct "preview" API that is distinguishable from committed
  pushes. Either is acceptable; the operator-visible behaviour is
  what matters.
- **Test buttons.** This ADR does not add a test-whistle or
  test-sound button to Sound Options. If one is added later, it
  naturally benefits from live preview because the sound controller
  already reads the buffered state. Flagging here so the option is
  not missed.
- **Active-game edge case.** If the refbox is in an active game
  while settings are being edited (unusual but possible), any
  game-triggered sound during editing plays at the buffered
  volume. This is accepted as a consequence of the simple design.

## Consequences

**Becomes easier:**

- Operators can confirm a sound volume or a panel orientation
  before committing, reducing trial-and-error through repeated
  settings visits.
- Setup time before a tournament is shorter because fewer
  commit-then-re-enter cycles are needed to land on the desired
  values.

**Becomes harder / constrained:**

- The sound controller and LED panel update stream now receive
  pushes during settings editing, not only on final commit. Code
  paths that assumed pushes happen exactly once per settings
  session need to handle multiple pushes per session.
- Cancel on Display Options and Sound Options gains a second
  responsibility: undoing the live preview. This is more work than
  Cancel on Game Options or App Options, which only discards buffer
  state.
- Testing must cover the revert-on-cancel path for every live field
  to ensure the subsystem state matches the committed config after
  a cancel.

**Scope:**

- `refbox` — Sound Options and Display Options view builders gain
  live-preview push calls; the application's Cancel handler for
  those pages gains the revert step. No change outside `refbox`.
- `uwh-common`, `overlay`, `schedule-processor`, LED panel crates,
  `wireless-remote` — no change.

## References

- ADR 009 — per-page Cancel/Apply model this ADR builds on.
- `refbox/src/sound_controller/mod.rs` — sound subsystem that
  live-preview pushes target.
- `refbox/src/app/mod.rs` — `update_sender` that pushes LED panel
  state; live preview for `white_on_right` and `brightness` flows
  through here.
- `memory/feedback_backport_web_is_standard.md` — back-porting
  rule; this ADR is a Rust-side enhancement that the web may later
  adopt.
