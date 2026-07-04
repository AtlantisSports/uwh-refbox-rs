# Cancel Timeout button — rename, orange fill, and honest "Switch to" labels

**Date:** 2026-06-16
**Crate:** `refbox` (UI only)
**Process:** Lean (refbox UI, no `uwh-common` / no state-machine change)
**Status:** Design approved in chat 2026-06-16; spec recorded for the record (kept local — not committed to the feature branch or PR, per repo convention for `docs/superpowers/`).

---

## Goal

Three related changes to the refbox timeout controls:

1. **Rename** the "End Timeout" button to "Cancel Timeout" everywhere it appears.
2. **Orange fill** on every Cancel Timeout button, for all timeout types (team, ref, penalty shot).
3. **Honest "Switch to …" labels** — the other ribbon buttons should only say "SWITCH TO …" when
   that switch is actually available. When it isn't, they show that timeout's normal name, greyed.

## Scope boundary

In scope: `refbox/src/app/view_builders/shared_elements.rs`,
`refbox/src/app/view_builders/main_view.rs`, and the 15 `refbox/translations/<locale>/refbox.ftl`
files.

Explicitly **out of scope**:
- Changing *which* switches are allowed (no `tournament_manager` / state-machine change).
- Changing what any button *does* when clicked (`Message::EndTimeout` and the `TeamTimeout` /
  `RefTimeout` / `PenaltyShot` messages are untouched).
- LED panel (`matrix-drawing`) and stream overlay.

## Why this is low-risk

Change (3) is **purely cosmetic — text only**. A button that is greyed out today stays greyed out;
a button that is clickable today stays clickable and still fires the same message. The only thing
that changes on the greyed buttons is the words shown. No `on_press` is added or removed in a way
that changes clickability: the disabled branch shows a label with **no** `on_press` (greyed), exactly
as the "Switch to …" disabled branch is greyed today.

Changes (1) and (2) are a label rename and a style swap.

---

## Change 1 — Rename

The word "TIMEOUT" on the two-line button is a shared Fluent message reference (`{ timeout }`,
= "TIMEOUT") that is *also* used by the Ref Timeout start button. We do **not** touch it. We only
change the "END" part.

| Key | File usage | Before | After |
|-----|-----------|--------|-------|
| `end-timeout` | single-line centre button (`main_view.rs`) | `END TIMEOUT` | `CANCEL TIMEOUT` |
| `end-timeout-line-1` | two-line ribbon button (`shared_elements.rs`, ×4) | `END` | `CANCEL` |
| `end-timeout-line-2` | two-line ribbon button | `{ timeout }` | unchanged |

Both `end-timeout` and `end-timeout-line-1` are translated in **all 15 locales** — no English
placeholders. Where each locale already has a translation for "Cancel" (used elsewhere in the UI),
reuse that term for consistency.

## Change 2 — Orange fill

Five sites currently use `.style(yellow_button)` on the `Message::EndTimeout` button. All five
become `.style(orange_button)` (the existing palette style — not a new colour):

- `main_view.rs` — single-line centre cancel button (1 site).
- `shared_elements.rs` — ribbon cancel button for Black, White, Ref, Penalty Shot timeouts (4 sites).

## Change 3 — Honest "Switch to …" labels

In `build_timeout_ribbon` (`shared_elements.rs`), each slot that is **not** the active timeout
currently always shows "SWITCH TO …" and gates clickability on `can_switch_to_*`. We split that
into two branches:

- **Switch available** (`can_switch_* == Ok`): show "SWITCH TO …", clickable (unchanged).
- **Switch not available** (`can_switch_* == Err`): show that timeout's **own start label**
  (the same `*-timeout-line-*` / `penalty-shot-line-*` keys used when no timeout is active),
  with **no** `on_press` → greyed. Same slot colour/style as today.

No new translation keys (the start labels already exist).

### Resulting ribbon while a timeout runs (✅ clickable, ⛔ greyed)

| Timeout running | Black slot | White slot | Ref slot | Penalty slot |
|---|---|---|---|---|
| **Black team** | **CANCEL** ✅ orange | Switch to White ✅¹ | Ref Timeout ⛔ | Penalty Shot ⛔ |
| **White team** | Switch to Black ✅¹ | **CANCEL** ✅ orange | Ref Timeout ⛔ | Penalty Shot ⛔ |
| **Ref** | Dark Timeout ⛔ | Light Timeout ⛔ | **CANCEL** ✅ orange | Switch to Pen Shot ✅² |
| **Penalty shot** | Dark Timeout ⛔ | Light Timeout ⛔ | Switch to Ref ✅ | **CANCEL** ✅ orange |

¹ If the other team has no timeouts left this half, that slot shows "Light/Dark Timeout" greyed.
² Clickable only during a play period; otherwise shows "Penalty Shot" greyed.

The only switches the game logic allows (unchanged) are **team ↔ other team** and
**ref ↔ penalty shot**; every other combination is correctly greyed and now labelled honestly.

---

## Acceptance criteria

- Every Cancel Timeout button reads "CANCEL TIMEOUT" (centre) / "CANCEL" over "TIMEOUT" (ribbon),
  in every locale, and is filled orange — for team, ref, and penalty-shot timeouts.
- While a timeout runs, no greyed ribbon button reads "SWITCH TO …": greyed slots show the
  timeout's own name instead. Clickable "SWITCH TO …" buttons are unchanged.
- Clicking behaviour is identical to before (cancel still cancels; allowed switches still switch).
- `just check` is clean (fmt, clippy `-D warnings`, tests, audit).

## How to verify (operator-visible)

1. Start a game, start a **team timeout** → the team's button reads "CANCEL"/"TIMEOUT" in orange;
   the Ref and Penalty Shot slots read "Ref Timeout" / "Penalty Shot" greyed (not "Switch to …").
2. Start a **ref timeout** → Ref slot reads "CANCEL"/"TIMEOUT" orange; the two team slots read
   "Dark Timeout" / "Light Timeout" greyed; "Switch to Pen Shot" is clickable.
3. Start a **penalty shot** → Penalty slot reads "CANCEL"/"TIMEOUT" orange; "Switch to Ref" clickable;
   team slots greyed with their own names.
4. Turn "track fouls & warnings" off, start any timeout → the centre button reads "CANCEL TIMEOUT"
   in orange.
