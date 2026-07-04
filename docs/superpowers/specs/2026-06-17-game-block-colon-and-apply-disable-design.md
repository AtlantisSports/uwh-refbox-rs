# Game Block button colon + APPLY disable on red — Design

**Date:** 2026-06-17
**Crate:** `refbox` (UI only)
**Branch (to be cut from current `master`):** `fix/refbox/game-block-colon-and-apply-disable`

## Problem

On the Game Parameters page (portal mode OFF), the Game Block parameter button has two
issues:

1. **Missing colon.** Its label reads `GAME BLOCK` while every sibling parameter button reads
   with a trailing colon (`HALF LENGTH:`, `MINIMUM BRK BTWN GAMES:`, etc.). Visually
   inconsistent.

2. **APPLY not guarded.** When the Game Block is insufficient (shown RED — "too short to fit
   the game plus the minimum break"), the green APPLY button is still pressable. The Game Block
   editor's own "Done" button already blocks the red state, but the page-level APPLY never got
   the same guard, so an operator can commit an invalid Game Block from the main page.

## Goals

- Game Block button label reads `GAME BLOCK:`, matching its neighbours.
- APPLY on the Game Parameters page is disabled while the Game Block is RED (too short), and
  re-enables once it is no longer red.

## Non-goals

- No change to how Game Block validity is *calculated* (the red/yellow thresholds stay as-is).
- No change to other parameter button labels.
- No change to CANCEL behaviour.
- YELLOW ("tight") does **not** disable APPLY — it remains a caution the operator can apply.
  (Confirmed with user; also matches the editor's existing Done-button behaviour.)

## Design

### Part 1 — the colon (label only)

The colon is part of the label *string*, not added in code. Sibling buttons use a dedicated
"button" label string that includes the colon, while the matching editor/help screens use a
shorter colon-free string. Example already in the codebase:

- `half-length` = `HALF LEN` — used as the editor screen title
- `half-length-full` = `HALF LENGTH:` — used on the parameter button

The Game Block parameter only has a single string today (`game-block` = `GAME BLOCK`), which is
reused in three places: the button, the editor screen title, and the help page. Adding a colon
directly to it would wrongly put a colon on the editor title and help page too.

**Approach (chosen):** add a new button-only label string and use it only on the button.

- New FTL key: `game-block-full = GAME BLOCK:` (translated in all 15 locales — best-guess
  translation per locale, never an English placeholder).
- The Game Block button (`make_event_config_page`) uses `fl!("game-block-full")` instead of
  `fl!("game-block")`.
- The editor screen title and help page keep using `fl!("game-block")` unchanged.

### Part 2 — disable APPLY on RED

In `make_event_config_page`, the APPLY button is built inline (the page does not use the shared
`make_cancel_apply_footer`). Today:

```rust
let apply_blocked = settings.uwhportal_incomplete();
let apply_enabled =
    page_has_changes(ConfigPage::Game, settings, page_entry_snapshot) && !apply_blocked;
```

Add a "Game Block too short" condition, gated on the parameter grid actually being on screen
(portal mode OFF — the only mode that renders the Game Block button):

```rust
let game_block_too_short =
    !using_uwhportal && matches!(game_block_validity(config), GameBlockValidity::TooShort);
let apply_enabled = page_has_changes(ConfigPage::Game, settings, page_entry_snapshot)
    && !apply_blocked
    && !game_block_too_short;
```

In portal-ON mode `game_block_too_short` is always `false`, so APPLY behaves exactly as it does
today (no hidden disable with no visible red button to explain it).

## Scope / blast radius

- `refbox/src/app/view_builders/configuration.rs` — `make_event_config_page` only.
- 15 `refbox/translations/*/refbox.ftl` files — one new `game-block-full` string each.
- No `uwh-common`, no state machine, no wire format, no other crate. Low blast radius,
  `refbox`-UI-only.

## Acceptance criteria

- The Game Block button reads `GAME BLOCK:` with a trailing colon, matching neighbouring
  buttons.
- With a RED (too-short) Game Block, the green APPLY button is greyed out and unpressable.
- Widening the Game Block until it is no longer red re-enables APPLY.
- A YELLOW (tight) Game Block still allows APPLY.
- `just check` passes (fmt, clippy `-D warnings`, tests).

## Verification

1. Launch refbox, open Game Parameters with portal OFF.
2. Confirm the button reads `GAME BLOCK:`.
3. Set a too-short Game Block → button turns red, APPLY greys out.
4. Widen it past the red threshold → APPLY becomes pressable.
5. Set a yellow (tight) value → APPLY stays pressable.
