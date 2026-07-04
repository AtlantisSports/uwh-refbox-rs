# Golden-trace guard — missing-scenario coverage (follow-up A)

**Date:** 2026-06-09
**Branch:** `feat/refbox/golden-trace-missing-scenarios` (stacked on `feat/refbox/time-golden-trace-guard` / PR #1041).
**Origin:** the cargo-mutants validation of the time-golden-trace guard surfaced ~5 coverage gaps —
game situations the 30-scenario library never exercises, so mutations in those paths survived.
This adds focused scenarios to close them. Test-only; engine never modified; `render()` unchanged
(watched state stays time-only — expanding beyond time is the separate follow-up B).

## Approach (chosen)

One focused scenario per gap (1:1 gap → scenario → the specific previously-surviving mutant it now
kills). Existing 30 golden files are untouched (purely additive — no re-bless of existing traces).
Reuses the existing driver; **no new `Action` variants needed** (natural expiry = start the
timeout/shot and simply omit `EndTimeout`, running long enough for `update` to expire it).

## Scenarios

| Scenario | Mechanism | Kills (was "missed") |
|---|---|---|
| `sudden_death_no_overtime` | config overtime=false, sudden_death=true; SecondHalf tie → confirm-pause → PreSuddenDeath → SuddenDeath | `pause_for_confirm` mod.rs:1846 (SD-branch confirm-pause duration) |
| `manual_clock_edit_rewinds_penalty` | StartPenalty; StopClock; `SetGameClock` to a clock value *earlier in the period than the penalty start* → penalty `time_remaining > full duration` → rebase branch fires | `set_game_clock_time` mod.rs:1781, 1783 |
| `team_timeout_expires` | StartTeamTimeout; **no** EndTimeout; run past `team_timeout_duration` → natural expiry via `update` (game clock resumes) | `update` mod.rs:1330 |
| `rugby_penalty_shot_expires` | StartRugbyPenaltyShot; **no** EndTimeout; run past expiry → `handle_rugby_pen_shot_end` via `update` | `update` mod.rs:1348; `handle_rugby_pen_shot_end` 1443, 1478 |
| `single_half_to_overtime` | config single_half=true, overtime=true; FirstHalf tie at end → `end_first_half` single-half branch → PreOvertime | `end_first_half` mod.rs:1366-1368 cluster |
| `between_games_reset` *(attempt; defer if fragile)* | run a full game into BetweenGames; keep running until the break clock reaches the reset threshold → `reset()` | `update` mod.rs:1181, 1182 |

`between_games_reset` depends on the `reset_game_time` mechanic and a long enough fixed-step run.
If it does not trigger cleanly/deterministically, document and defer it rather than ship a flaky
scenario. Target: 5 solid scenarios, 6 if the reset one proves clean.

## Acceptance criterion ("prove the kill")

For each new scenario:
1. Add it to `scenarios::all()`; capture its golden trace via `UPDATE_GOLDEN=1`; inspect for sanity.
2. Confirm the full guard test stays green and the existing 30 traces are unchanged (additive only).
3. **Kill-proof:** re-run *only* the specific previously-surviving mutant(s) for that gap (targeted
   `cargo mutants -f <file> --re <function>` or line-targeted, golden test as the sole kill-check)
   and confirm it flips **missed → caught**. Record the before/after per scenario.

## Out of scope

- No `render()` change (watched state stays time-only — that's follow-up B).
- No engine change. If a scenario reveals an engine bug, report it; fix on a separate branch.
- No full 239-mutant re-sweep — only the targeted re-runs above.

## Deliverables

- New entries in `golden/scenarios.rs` (+ any new action arrays).
- New `golden_traces/*.trace` per scenario.
- A per-scenario kill-proof record (in the PR body / a short results note).
- Commit(s): `test(refbox): add <scenario> golden scenario (closes <gap>)` or grouped.
