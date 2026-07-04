# Backlog: dynamic font sizing for Game Info cells

**Status:** Idea / unfinished prototype. Not on any branch anymore.
**Preserved:** 2026-06-18, salvaged from the deleted local branch `high-contrast-ui`
(tip commit `5c889b32602b5b152eba8494fff2c8582f60f44d`) before that branch was deleted.
**Source file:** `dynamic_font_sizing.rs` (in this folder) — the original
`refbox/src/app/dynamic_font_sizing.rs` from that branch.

## What it does

Uses the `fontdue` crate to **measure rendered text width and auto-shrink the font** so a
string fits its cell, down to a `MIN_FONT_SIZE` floor (12.0), stepping by `FONT_SIZE_STEP`.
It defines a `GameInfoCell` enum for the cells that needed it on the old Game Info screen:
`LastGame, NextGame, ChiefRef, Timer, WaterRef1, WaterRef2, WaterRef3`.

## Why it's relevant

Directly addresses the **text-wrapping / overflow** concern on the Game Info screen — the same
worry that came up while renaming the "GAME INFO" label to "INFORMATION". A general auto-fit
helper would let long referee names / game labels shrink to fit instead of wrapping or clipping.

## Why it was NOT absorbed as-is

- It's ~8 months old and targets the **old** Game Info layout. The screen has since been rebuilt
  as a table (`refbox/src/app/view_builders/game_info_table.rs`, PR #1255 + #1293), so the
  `GameInfoCell` mapping no longer matches reality.
- It adds a **new dependency** (`fontdue`) — needs discussion before adding to the workspace.
- On the branch it was wired into `refbox/src/app/mod.rs` and `view_builders/main_view.rs`; that
  wiring is gone and would need redoing against the current table.

## How to revive (if pursued)

Treat as its own feature: Scope Card + plan. Port the width-measure/shrink logic onto the current
`game_info_table.rs` cells, decide on the `fontdue` dependency (or reuse an existing font/measure
path), and add tests. The measurement logic in `dynamic_font_sizing.rs` is the reusable core; the
`GameInfoCell` enum and any old-layout wiring should be rewritten, not copied.
