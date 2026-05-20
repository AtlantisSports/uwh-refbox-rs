# Beep-Test Pause / Resume / Reset-While-Running — Design

**Date:** 2026-05-20
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing from Chunk 1 at `89dc20a`)
**Chunk:** 2 of 6 follow-on improvements to the beep-test mode

---

## Goal

Make the bottom-row buttons on the BeepTest main page reflect the operator's
mental model of starting, pausing, resuming, and resetting a beep test —
rather than the literal start/stop toggle they show today. Specifically:

- Distinguish "first start" from "resume after a pause" so the operator can
  see at a glance whether they're entering or returning to a run.
- Prevent accidental RESET while the engine is running.

---

## Motivation

Today, the bottom row alternates between two states:
- Stopped: green **START** button (plus RESET, gray-disabled until first start).
- Running: orange **STOP** button (plus RESET, red).

The operator's feedback after walking through the running app:

- "Stop" overstates what the button does — it actually pauses, not ends the
  run. After "stopping," the engine resumes from the same lap when pressed
  again. Calling it Pause matches the behavior.
- The same button shouldn't be labelled START both when the operator is
  starting fresh and when they're resuming a paused run — the two are
  semantically different.
- RESET being live during a run is dangerous. A misclick discards the run.

---

## Scope

### Files touched

- `refbox/src/app/view_builders/beep_test.rs` — bottom-row button rendering.
- `refbox/src/app/mod.rs` — one-line change to the `BeepTestReset` handler.
- `refbox/translations/*/refbox.ftl` — rename one key, add one key, across 15 locales.

### Not touched

- The cadence engine (`refbox/src/beep_test/cadence.rs`) — Resume already
  behaves identically to Start once a level is active; no engine change needed.
- The `Message::BeepTestStop` and `Message::BeepTestStart` variant names. The
  labels change but the internal message names stay (Stop still means "stop
  the clock"; Start still means "start/resume the clock").
- Any other page (settings landing, sub-pages, hockey/rugby/UWR modes).
- The orange `STOP` button style elsewhere in the app — only beep-test's
  bottom-row STOP usage is replaced.

---

## Design

### Bottom-row state table

Two boolean flags drive the bottom-row state: `clock_running` (cadence engine
is ticking) and `beep_test_has_run` (the operator has pressed Start at least
once since the last Reset). The four combinations:

| `clock_running` | `has_run` | START/STOP button                           | RESET button                       |
|-----------------|-----------|---------------------------------------------|------------------------------------|
| false           | false     | **START** — green, fires `BeepTestStart`    | gray, no `on_press` (unchanged)    |
| true            | true      | **PAUSE** — yellow, fires `BeepTestStop`    | gray, no `on_press` (**new**)      |
| false           | true      | **RESUME** — blue, fires `BeepTestStart`    | red, fires `BeepTestReset` (unchanged) |
| true            | false     | (unreachable — `BeepTestStart` always sets `has_run = true` before starting the engine) | (unreachable) |

The unreachable row exists because the handler sequence in `BeepTestStart`
sets `has_run` before delegating to the engine; there is no observable state
in which `clock_running = true` but `has_run = false`.

### Color rationale

- **Green START** — go, fresh run. Matches the green button used elsewhere
  in the refbox for initial-action affordances.
- **Yellow PAUSE** — caution / hold. Distinct from the previous orange STOP,
  and distinct from the destructive red RESET.
- **Blue RESUME** — continue. Blue distinguishes "you are mid-run" from the
  green "you are starting fresh," giving the operator a glance-level cue.
- **Red RESET** — destructive, clears the run. Already red today.

### Reset handler change

`Message::BeepTestReset` currently calls `bt_tm.reset_beep_test_now(...)` and
clears `self.beep_test_snapshot`. It does NOT clear `self.beep_test_has_run`,
so the run-state flag stays `true` for the lifetime of the program once Start
has been pressed.

Add one line: `self.beep_test_has_run = false;` at the end of the handler so
that after Reset, the button reverts to **START** (initial state) rather than
**RESUME** (paused-mid-run state).

### Translation surgery

In all 15 locale files (`refbox/translations/<locale>/refbox.ftl`):

1. **Rename** `beep-test-stop` → `beep-test-pause`, updating its value:
   - `en-US`, `de-DE`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`,
     `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN` → `PAUSE`
   - `es` → `PAUSAR` (was `PARAR`)
   - `fr` → `PAUSE` (was `ARRÊTER`)
2. **Add** new key `beep-test-resume`:
   - `es` → `REANUDAR`
   - `fr` → `REPRENDRE`
   - All other 13 locales → `RESUME` (matching the existing pattern where
     locales without dedicated translations use English originals).

The `beep-test-start` key is unchanged in name and value.

---

## Acceptance criteria

Walking through the running refbox in BeepTest mode:

1. **Fresh state.** Bottom-right button reads **START** in green. RESET button
   is gray (unchanged).
2. **Press START.** Engine begins ticking. Bottom-right button reads
   **PAUSE** in yellow. RESET button is gray (no longer red).
3. **Press PAUSE.** Engine stops ticking. Bottom-right button reads
   **RESUME** in blue. RESET button is red and pressable.
4. **Press RESUME.** Engine resumes from the same lap and level it was on.
   Bottom-right button reads **PAUSE** in yellow again. RESET is gray again.
5. **Press PAUSE then RESET.** Engine state clears. Bottom-right button reads
   **START** in green. RESET button returns to gray.

A `just check` pass plus the above walkthrough is the verification bar.

---

## Out of scope (intentionally deferred)

- Renaming `Message::BeepTestStop` / `Message::BeepTestStart` to better match
  the new labels. The message names describe what the handler does to the
  engine, not what the button says, so they can stay accurate.
- Audio cues on pause/resume transitions.
- Any change to the orange-button style itself (other refbox uses, like the
  game STOP button in hockey mode, are not affected).
- The remaining five follow-on chunks (Settings gating, total lap count,
  warmup countdown, LED panel score hiding, etc.) — each gets its own
  spec/plan cycle.
