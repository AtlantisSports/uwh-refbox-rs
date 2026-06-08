# DELAY Title + Banner-Fit Check — Design

**Date:** 2026-06-08
**Branch:** `feat/uwh-common/game-block`
**Crate:** `refbox` (view layer + translations only)

## Goal

Add a small "DELAY" title above the behind-schedule figure in the main-view time banner, and confirm — by rendering the densest real banner — that the delay figure coexists with the portal health tile, the game-phase/timeout time, and the UWR play/pause button without crowding.

## Context

The center-top banner is built in `make_game_time_button` ([refbox/src/app/view_builders/shared_elements.rs:394](../../refbox/src/app/view_builders/shared_elements.rs)) and assembled by `build_main_view` ([refbox/src/app/view_builders/main_view.rs:16](../../refbox/src/app/view_builders/main_view.rs)). Horizontally it holds, conditionally:

```
[ health tile ] [ PERIOD / CLOCK   (timeout col)   DELAY ] [ play/pause ]
  portal-linked    "game phase time"    timeout-active   delay   Rugby-only
  only                                  only             >0 only
```

The behind-schedule figure is passed in as `behind_label: Option<String>` (the `-M:SS` string) and currently rendered as a single right-pushed `text` at `MEDIUM_TEXT` in red ([shared_elements.rs:556-563](../../refbox/src/app/view_builders/shared_elements.rs)).

## The keeper change (ships in this branch's PR)

Replace the single delay `text` with a centered two-line column, mirroring the existing period-over-clock stack:

- Line 1 (title): `fl!("delay")` at `SMALL_TEXT` (19), `red_text`.
- Line 2 (figure): the existing `behind_label` string at `MEDIUM_TEXT` (38), `red_text`.
- Column centered (`Alignment::Center`), vertically centered to match the banner.

Literal title text is **DELAY** (caps). The whole column is only present when `behind_label` is `Some` (i.e. the run is behind), exactly as today.

No change to banner widths, the `behind_schedule` calculation, or any other widget. Stacking is also the narrowest layout, which helps the tight worst case.

### Translation

New Fluent key `delay = DELAY` added to `en-US` and given a best-guess translation in all 15 locales (per the project's no-English-placeholders rule). The existing `delay-of-game` key is the *foul* "Delay Of Game" and must NOT be reused.

## Temporary fit-check (Approach A — throwaway, never committed)

A clearly-marked `// TEMP DEMO — REVERT` block at the top of `build_main_view` shadows four locals for rendering only, forcing the densest banner:

- `mode` → `Mode::Rugby` (shows the play/pause button),
- `portal_indicator` → `Some(PortalIndicatorState::default())` (shows the health tile),
- a cloned snapshot with `timeout = Some(TimeoutSnapshot::PenaltyShot(23))` (shows the timeout column),
- `behind_schedule` → `Duration::from_secs(61)` (shows `-1:01` under the new DELAY title).

This uses the real widgets at real sizes, so the on-screen result is a trustworthy fit verdict. The block is removed before the PR; nothing about it is committed.

## Verification

1. Apply the keeper change + the Approach A demo block together; rebuild and launch refbox.
2. Screenshot the dense banner. The operator judges whether all four elements fit without crowding/clipping.
3. If a fit problem appears, it is recorded as a follow-up (e.g. abbreviate or relocate the delay in the dense case) — not silently fixed here.
4. Delete the demo block. Confirm `cargo test -p refbox` and `cargo clippy -p refbox -- -D warnings` are clean with only the keeper change (DELAY title + translations) present.

## Out of scope

- The `behind_schedule` / scheduling / clock logic (unchanged).
- Banner widths and the structure of other view builders.
- Any change to what the dense case does if it *doesn't* fit (that becomes a separate, explicit follow-up after we see it).
