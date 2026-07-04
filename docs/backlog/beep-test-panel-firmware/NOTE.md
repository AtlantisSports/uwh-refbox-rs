# Backlog: re-flash Matrix panel firmware so beep test blanks the black side

**Status:** Idea / hardware task. Not started, not on any branch. Surfaced 2026-06-19 while
triaging v0.4.3 updates.
**Type:** `led-panel` firmware (the scoreboard's own onboard program) — **NOT a refbox change.**

## The problem

During beep-test mode, only the white-score panel should show the lap count; the black-score
panel should be **dark/blank**. On the **physical Matrix scoreboard** the black panel instead
shows **"0"**.

## Why it is NOT a refbox / v0.4.3 task

There are two separate programs:

- **refbox** (laptop/Pi app) — sends the scoreboard only a compact message: lap count + a
  "beep-test" flag (`TransmittedData::encode` in `matrix-drawing/src/transmitted_data.rs`,
  beep-test = bit 2). It does **not** send pixels. The refbox self-update (v0.4.x) updates only
  this program.
- **`led-panel` firmware** — lives **on the scoreboard hardware** and decides how to draw what
  it receives. It is **external to this repo** (no workspace crate; named in
  [docs/domain.md:42](docs/domain.md#L42)). The refbox auto-update does **not** touch it.

The refbox has no lever here: scores are plain numbers (`u8`), so there is no "blank" value it
could send instead of `0`. The decision to skip drawing the black side belongs to the panel.

## The fix already exists in the shared drawing code

The shared, no_std `matrix-drawing` crate **already** skips the black score during beep test:
[matrix-drawing/src/drawing.rs:239](matrix-drawing/src/drawing.rs#L239) and
[matrix-drawing/src/drawing.rs:272](matrix-drawing/src/drawing.rs#L272) only draw a score panel
when it is the white (lap-count) side. (`refbox/src/beep_test/snapshot.rs` sets `white = lap_count`,
`black = 0`.) This has shipped since commit `16705d73` "Ref beep test (#448)", 2025-02-28 — i.e.
it predates v0.4.1.

Proof it is correct in current code: the **panel simulator** (Default layout, which mirrors the
physical Matrix panel — `refbox/src/sim_app/mod.rs`, clears the buffer then calls `draw_panels`)
already leaves the black side dark in beep test. User confirmed the simulator behaves correctly
2026-06-19.

**Conclusion:** the physical scoreboard's onboard program is simply **older than this change** (or
was built from a pre-#448 `matrix-drawing`). It needs to be rebuilt with the current drawing code
and re-flashed.

## How to do it (when pursued)

Separate, hardware-in-hand firmware task — treat with the same caution as `wireless-remote`
(explicit discussion + physical hardware required; embedded rules apply):

1. Locate the external `led-panel` firmware project and confirm how it consumes `matrix-drawing`
   (git dependency / vendored copy / pinned version).
2. Rebuild it against a current `matrix-drawing` (one that includes #448's beep-test skip).
3. Re-flash a **spare** Matrix panel first; verify in beep test that the black side is dark and
   the lap count still shows on the white side.
4. Only then re-flash production panels.

Note: the custom 7-segment ("Sunlight") panels have not been tested at all yet and use a different
render path (`DisplayState::from_transmitted_data`, in `led-panel-sim`); whether they blank the
black side in beep test is a separate, unverified question.
