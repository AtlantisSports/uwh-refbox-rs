# Auto-buzzer: end on a whole pattern boundary

**Date:** 2026-06-18
**Crate:** `refbox` (only) — `refbox/src/sound_controller/mod.rs`
**Process:** lean (refbox audio playback timing; no wire-format / uwh-common / game-state-machine impact)

## Problem

The end-of-period **auto buzzer** plays a short looping clip for a fixed wall-clock
duration of ~2.15s (full-volume portion) regardless of the clip's own length. The
clips are not all the same length, so the fixed timer almost never lands on a clip
boundary. For **Whoop** (a 0.5s clip = ~0.375s rising tone + ~0.125s silence), 2.15s
lands ~0.2s into a fresh 5th whoop, in its loud rising part — so the buzzer ends with
a chopped-off partial whoop. That is the "truncated start of the next pattern."

A prior fix moved the value from 2.0s → 2.15s on purpose, to keep the fade-out out of
the quiet seam between whoops. That choice is exactly what drops the cut into the
middle of a pattern. This design reverses that decision.

### Measured clip lengths (44.1 kHz mono)

| Sound | Samples | Loop period | Trailing silence? |
|-------|---------|-------------|-------------------|
| Whoop | 22050 | 0.500s | yes (~0.125s) |
| Buzz | 22050 | 0.500s | yes (~0.15s) |
| Crazy | 29430 | 0.667s | no (continuous) |
| De-De-Du | 33077 | 0.750s | yes (~0.125s) |
| Two-Tone | 35280 | 0.800s | nearly none |

## Goal

The auto buzzer must always end on a **whole number of complete loop cycles** of its
own clip, with the fade-out **completing at that boundary** so it lands in the clip's
trailing silence (for clips that have it) or smoothly at the pattern's end (for the
continuous clips). Nothing gets chopped; no blip of the next pattern.

This applies to **all** auto-buzzer sounds (they share the timing code), not just
Whoop — it cleans up Buzz / De-De-Du too and leaves Crazy / Two-Tone ending on a
complete pattern.

### Resulting durations (target ~2.15s, snapped to whole cycles)

| Sound | Cycles | Duration |
|-------|--------|----------|
| Whoop | 4 | 2.00s |
| Buzz | 4 | 2.00s |
| Crazy | 3 | 2.00s |
| De-De-Du | 3 | 2.25s |
| Two-Tone | 3 | 2.40s |

## Design

Only the **timed** branch of `Sound::new` changes (auto buzzer). The manual / wired /
wireless button buzzers (`timed = false`, looped, stopped on release) and the whistle
are untouched.

1. Rename the intent of `SOUND_LEN` (2.15s) from "the fixed full-volume length" to
   "the **target** length we snap to whole cycles." Rewrite the now-obsolete
   2.0-vs-2.15 comment.

2. Add a small pure helper, unit-tested:

   ```
   /// Whole loop cycles to play so the buzzer ends on a pattern boundary
   /// nearest the target length (at least one cycle).
   fn whole_cycles_for(loop_period: f64, target: f64) -> u32
   ```
   `= max(1, round(target / loop_period))`, guarding `loop_period <= 0`.

3. In the timed branch, compute:
   - `loop_period = length as f64 / sample_rate as f64` (we already have `length`)
   - `cycles = whole_cycles_for(loop_period, SOUND_LEN)`
   - `played = cycles as f64 * loop_period`
   - Buffer start `t0 ≈ current_time()` (= `fade_end - FADE_LEN`)
   - `fade_out_end = t0 + played` (gain reaches 0 here — the cycle boundary)
   - `fade_out_start = fade_out_end - FADE_LEN` (fade-out occupies the last 50ms
     before the boundary → inside the trailing silence for Whoop/Buzz/De-De-Du)
   - Schedule: hold full to `fade_out_start`, linear-ramp to 0 at `fade_out_end`
   - `end = start + Duration(played)` (wall clock) — when gain hits 0; `stop()` then
     sees `already_silent` and just stops the source cleanly.

4. Remove the now-unused `TIMED_SOUND_LEN` / `TIMED_SOUND_DURATION` consts (replaced
   by the per-sound computation) to avoid dead-code clippy warnings.

The fade-in (first 50ms) is unchanged.

## Testing

- Unit tests on `whole_cycles_for` for each bundled clip period (4/4/3/3/3) plus
  edges: `loop_period == 0` → 1, and `loop_period > target` → 1.
- A regression-guard assertion that `cycles * loop_period` is a whole multiple of
  `loop_period` for each clip (i.e. the buzzer always ends on a pattern boundary) —
  this is the property whose violation caused the bug.
- Manual listen test in the running app (operator confirms): trigger the end-of-period
  buzzer for Whoop (and spot-check the others) and confirm it ends cleanly with no
  chopped partial pattern.

## Out of scope

- Manual / remote-button buzzer hold-and-release behaviour.
- Re-recording or replacing any sound asset file.
- Volume levels, fade-in/out feel, and the default sound selection.
