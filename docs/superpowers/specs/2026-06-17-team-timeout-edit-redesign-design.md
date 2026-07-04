# Team-Timeout Settings Page Redesign — Design

**Date:** 2026-06-17
**Crate:** `refbox` only
**Process:** Lean (refbox UI; no `uwh-common`, no wire format, no timing state machine)

---

## Goal

Redesign the "edit team-timeout settings" page so it reads as a clean, single-purpose
settings panel instead of a generic number-pad data-entry screen.

Today the page reuses the shared 0–9 number pad (left half of the screen) to set the
**number of team timeouts**, plus a HALF/GAME toggle, a `+ / −` length editor, and
CANCEL/DONE on the right half.

In practice the number of team timeouts is only ever **0 or 1**, so the full number pad is
overkill, and the length is almost always the default 1:00. The redesign replaces these with
big, highlight-to-select buttons across the full width of the screen.

## Scope boundary

**In scope (refbox UI only):**
- `refbox/src/app/view_builders/keypad_pages/team_timeout_edit.rs` — rewrite the page body.
- `refbox/src/app/view_builders/keypad_pages/mod.rs` — render the TeamTimeouts page full-width
  (skip the shared number-pad column) and pass the current count into the page builder.
- `refbox/src/app/message.rs` — add message variant(s) to set the count directly (0 or 1) and
  to set the length to a preset value.
- `refbox/src/app/mod.rs` — handle the new message(s) in `update()`.

**Explicitly NOT doing:**
- No change to the team-timeout **game logic / timing** (`tournament_manager/`), to how a
  timeout actually runs, or to the timeout grace-window work (PR #1151).
- No change to `uwh-common`, the config wire format, or `num_team_timeouts_allowed` /
  `team_timeout_duration` / `timeouts_counted_per_half` config field types.
- One new translation key only (`team-timeout-count`, "TEAM TIMEOUT COUNT:"), best-guessed
  across all 15 locales. Every other label already exists in all locales.
- No change to any other keypad page — the shared number pad stays exactly as-is for
  score / penalty / game-number / foul / warning / portal-login.

## Chosen layout — Option A (full-width panel + duration presets)

```
+--------------------------------------------------------------+
|                   NEXT GAME    14:46                         |   <- existing game-time bar
+--------------------------------------------------------------+
|  TEAM TIMEOUT COUNT:            [   0   ]  [   1   ]          |
|                                                              |
|  COUNTED PER:                   [ HALF  ]  [ GAME  ]         |
|                                                              |
|  TIMEOUT LENGTH:                                             |
|     [ 0:30 ]  [ 0:45 ]  [ 1:00 ]  [ 1:15 ]  [ 1:30 ]         |
|                                                              |
|  [         CANCEL         ]    [          APPLY         ]    |
+--------------------------------------------------------------+
```

- The shared 0–9 number pad is **not rendered** for this page; the panel uses the full screen
  width below the game-time bar.
- Row labels: count row = new static **"TEAM TIMEOUT COUNT:"** (`team-timeout-count`);
  counted-per row = existing `timeouts-counted-per` ("COUNTED PER:"); length row = existing
  `timeout-length` ("TIMEOUT LENGTH:"). The count label is **static** — it no longer changes
  with the HALF/GAME setting (COUNTED PER now shows that separately).
- Every selectable control uses the existing selected/unselected blue button styling
  (`blue_selected_button` / `blue_button`), the same pattern HALF/GAME already uses today.
- Length presets are **0:30 / 0:45 / 1:00 / 1:15 / 1:30** (30/45/60/75/90 seconds). The button
  matching the current duration is shown selected. Default is **1:00**.

### Behaviour: count = 0 disables the rest

- When the count is **0**, the HALF/GAME buttons and the five length presets render **inactive**
  (greyed, non-pressable) — they are meaningless when no team timeouts exist.
- CANCEL and APPLY stay active so the operator can still confirm a count of 0.
- The underlying HALF/GAME and length selections are **preserved**, not reset, while disabled —
  flipping the count back to 1 restores the operator's previous choices.

### Confirm button: Cancel / Apply

- The confirm button changes from `fl!("done")` to `fl!("apply")`.
- Rationale: the redesigned page is a multi-setting panel, matching the other multi-setting
  screens (main Configuration page, Beep Test Settings), which both use **Apply**. The
  single-value keypad editors (score, penalty, game number) keep **Done**.
- `apply` already exists in all 15 locales — no translation work.

## State & message plumbing

Current state: `AppState::KeypadPage(KeypadPage::TeamTimeouts(duration, per_half), player_num)`
where `player_num` holds the count (initialised from `config.num_team_timeouts_allowed`).
On `ParameterEditComplete { canceled: false }` the three values are committed to
`edited_settings.config` (`mod.rs` ~2422). HALF/GAME flips via
`ToggleBoolParameter(TimeoutsCountedPerHalf)` (~2576). Length is edited by the `+ / −`
time-editor handler (~1409). This state shape is **kept**.

What's new:

1. **Set count directly to 0 or 1.** The number pad appended digits into `player_num`; the new
   0/1 buttons must *set* it. Add a message to set the team-timeout count (carrying the target
   value) and handle it by writing `player_num` in the `TeamTimeouts` AppState. The two buttons
   show selected/unselected based on the current `player_num`.

2. **Set length to a preset.** The `+ / −` editor only increments/decrements `duration`. The new
   preset buttons must *set* it to a specific value. Add a message carrying the target
   `Duration` (or seconds) and handle it by writing the `dur` field in the `TeamTimeouts`
   AppState. The preset whose value equals the current `dur` shows selected; if `dur` is some
   non-preset value (e.g. a previously-saved 0:50), none show selected until one is pressed.

3. **Pass the count into the page builder.** `make_team_timeout_edit_page` currently receives
   only `(duration, per_half)`. It must also receive `player_num` to render the 0/1 toggle state
   and to decide disabling.

(Exact message-variant names and whether to add one combined or two separate variants is an
implementation-plan decision; both are mechanical message-enum wire-up.)

## Acceptance criteria (operator-observable)

1. Opening the team-timeout settings page shows the full-width panel above — no number pad.
2. The count row shows **0** and **1**; the current value is highlighted; tapping the other
   value switches the highlight.
3. With count = **1**, HALF/GAME and all five length presets are active and selectable; the
   current length preset is highlighted.
4. Setting count to **0** greys out HALF/GAME and the length presets so they can't be pressed;
   CANCEL and APPLY still work.
5. Switching from 0 back to 1 restores the previously-highlighted HALF/GAME and length choices.
6. The bottom-right confirm button reads **APPLY** (was DONE); pressing it saves count,
   counted-per, and length exactly as selected. CANCEL discards.
7. All other keypad pages (score, penalty, game number, foul, warning) are visually unchanged.

## Resolved decisions

- **Count-row label:** new static **"TEAM TIMEOUT COUNT:"** (`team-timeout-count`). The old
  dynamic `num-tos-per-game` / `num-tos-per-half` labels are no longer used by this page (the
  HALF/GAME distinction now lives only on the COUNTED PER row). One new translation key,
  best-guessed across all 15 locales.
- **Confirm button:** **APPLY** (`apply`), matching the other multi-setting panels
  (Configuration, Beep Test).

## Verification

- `just check` (fmt, lint, tests, audit) clean.
- Manual walkthrough of acceptance criteria 1–7 in the running app
  (`cargo run -p refbox`, X11 launch per project notes).
