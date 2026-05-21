# Beep-Test LAP Tile Shows Total Laps — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 3 at `9c9e47c`)
**Chunk:** 4 of 6 follow-on improvements to the beep-test mode

---

## Goal

The LAP tile on the BeepTest main page should display the cumulative lap
counter for the whole run (0 during warmup, 1 at the first user-level lap,
incrementing by 1 at every lap boundary thereafter) instead of resetting
to 1 at the start of each level.

---

## Motivation

The current LAP tile resets to 1 at the start of each new user level
(showing "within-level lap"). The operator wants a continuous total so
they can see at a glance how far into the test they are without doing
mental arithmetic across level boundaries.

---

## Scope

### Files touched

- `refbox/src/app/view_builders/beep_test.rs` — the `lap_value`
  computation in `build_beep_test_page`.

### Not touched

- The cadence engine — `snapshot.lap_count` already has the right
  shape (0 during warmup, +1 at each lap boundary). No engine change
  needed.
- `active_within_lap` itself — still computed; still drives the
  yellow / blue active-cell highlight in the levels table. Only the
  LAP tile's display value changes.
- The LEVEL tile, the TIME tile, any other UI.
- Translations.
- Theme.

---

## Design

Replace the `lap_value` computation in `build_beep_test_page`. Today:

```rust
let lap_value: String = match snapshot.current_period {
    BeepTestPeriod::Level(0) => 1.to_string(),
    BeepTestPeriod::Level(_) => active_within_lap.unwrap_or(1).to_string(),
};
```

becomes:

```rust
let lap_value: String = snapshot.lap_count.to_string();
```

The `match` collapses because both branches now produce the same value.
`snapshot.lap_count` is maintained by the cadence engine as:

- `0` during the warmup (Level 0) — no laps started yet
- `1` once the warmup completes and Level 1 Lap 1 begins
- `2` when Level 1 Lap 1 ends and Level 1 Lap 2 begins
- `k` at the start of the k-th overall user-level lap

The within-level lap derivation (`active_within_lap` via the
`within_level_lap` helper) is unchanged — it's still needed for the
yellow/blue active-cell highlight in the levels table.

---

## Acceptance criteria

Walking through the running refbox in BeepTest mode:

1. **Fresh state.** LAP tile reads `0` (engine is at Level 0 / warmup).
2. **Press START.** Warmup begins counting down. LAP tile stays at `0`
   for the full 10-second warmup.
3. **Warmup ends, Level 1 Lap 1 begins.** LAP tile reads `1`.
4. **Each subsequent lap boundary** (within Level 1, or transitioning
   into Level 2, etc.): LAP increments by exactly 1.
5. **Across level boundaries.** LAP does NOT reset. If Level 1 has 3
   laps, LAP shows `1`, `2`, `3` for those laps, then `4` when Level 2
   Lap 1 begins. (The LEVEL tile still shows the level number; only
   LAP changes how it counts.)
6. **Press PAUSE.** LAP tile holds at its current value.
7. **Press RESET.** LAP tile returns to `0`.

A `just check` pass plus the above walkthrough is the verification bar.

---

## Out of scope (intentionally deferred)

- The eventual Chunk 5 warmup display rules (visible countdown showing
  Level 0 / Lap 0 / 10s). The LAP tile showing `0` during warmup is
  consistent with that future direction — no additional Chunk 4 work
  required to align.
- Any change to the active-cell highlight in the levels table.
- The remaining chunks (5 and 6).
