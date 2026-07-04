# Follow-up B — Expand the golden-trace guard: `scores` + `is_old_game`

**Status:** Design, converged through two rounds of adversarial review (2026-06-10) and
endorsed. Ready for `writing-plans`. This document is the locked, focused scope; an earlier
draft of this file explored a maximal "watch the whole snapshot + mirror every action" version
that was deliberately narrowed — see the companion `*-response-to-critique.md` and `*-reply-2.md`
for the review trail and the evidence behind each cut.

`file:line` references are attestations from `origin/master` (2026-06-10).

---

## 1. Goal

Protect the **stable core game business logic** — timing, penalties, timeouts, scoring,
period transitions, the game state machine — from *accidental* regression introduced by AI
changes, while UI/navigation work proceeds. Any future *intentional* change to core game logic
should be explicit and verified (the guard turns red, you re-bless with a one-line rationale).

**Exhaustive coverage of `GameSnapshot` is explicitly NOT the goal.** This round closes the two
highest-value, lowest-coupling blind spots in the existing guard and stops there.

## 2. Background — the existing guard (attested)

The time-engine golden-trace guard (PR #1041, + Follow-up A PR #1050) drives scripted
`Scenario`s through a real `TournamentManager` via a fixed-step (100 ms) replay driver and
emits a deduplicated, state-change-keyed text trace, compared against checked-in golden files
(`refbox/src/tournament_manager/golden/{mod.rs,scenarios.rs}` + `golden_traces/*.trace`). It is
differential vs. the last human-authored baseline `46ec0973`, and was mutation-validated.

`render(snap: &GameSnapshot) -> String` currently emits: `current_period`, `secs_in_period`
(`clock`), `timeout`, `conf_pause_time`, and `penalties` (color + player# + remaining/TD).
Everything else in the snapshot is invisible.

## 3. The two blind spots this round closes

**3a. Scores are invisible.** `regulation_with_scores` adds a Black goal at t=5 and a White
goal at t=12 (`scenarios.rs:68-69`), but `render()` omits `scores`, so a regression that
mis-recorded a goal passes silently. Scores also gate game-ending logic in sudden death /
overtime, which is core.

**3b. The between-games auto-reset is invisible.** When a game sits in `BetweenGames` and the
clock crosses `reset_game_time`, the engine calls `reset()` (`mod.rs:1180-1182`), flipping
`has_reset`, which drives `is_old_game` (`mod.rs:2196`: `is_old_game: !self.has_reset`).
`render()` omits `is_old_game`, so the two mutations at `mod.rs:1180-1182` survived Follow-up
A's mutation sweep — the deferred case.

## 4. Scope (locked)

**`render()` adds exactly two fields:**
- `scores` (Black / White)
- `is_old_game`

**No new `Action` variants. No change to `apply_action`.** This is the single most important
property of this design: the hand-mirrored coupling surface does not grow.

**Explicitly NOT watched** (intentional, completeness-test-audited omissions), with the
evidence for each cut:
- `warnings`, `fouls` — `add_warning` (`mod.rs:730`) and `add_foul` (`mod.rs:758`) are pure
  `Vec`-appends that touch no clock/penalty/timeout/period state; inert record-keeping from a
  time-engine standpoint.
- penalty **infraction kind** — every driver `StartPenalty` hardcodes `Infraction::Unknown`
  (`PenaltyKind` values are *durations*, not infractions), so the column would be a constant
  `Unknown`; making it vary needs a new action argument for a non-core metadata field.
- `recent_goal` — *verified* game-time deterministic (cleared at `mod.rs:2167-2172` on
  game-clock conditions only), so it was safe to add, but it's display sugar with no outcome
  effect; dropped on value grounds.
- `game_number`, `next_game_number`, `next_period_len_secs` — not core timing/state logic.
- `event_id` — hardcoded `event_id: None` in the snapshot constructor (`mod.rs:2199`); always
  `None`.

## 5. Scenarios

**Scores — no new scenario.** `regulation_with_scores` already drives `AddScore`
(`scenarios.rs:68-69`). Adding the `scores` column makes that existing scenario start guarding
score state. Change = render + re-bless only.

**Between-games auto-reset — ONE dedicated, clearly-named new scenario.**

Binding precondition (verified): `is_old_game = !has_reset`; the constructor sets
`has_reset = true` (so `is_old_game = false` initially); `SetupPeriod`
(`set_period_and_game_clock_time`) does **not** modify `has_reset`; and the auto-reset at
`mod.rs:1180-1182` fires only when `!has_reset`. Therefore a scenario that merely
`SetupPeriod(BetweenGames, …)` + runs leaves `has_reset = true`, the reset never fires, and the
mutants survive again.

The scenario must:
1. Reach `BetweenGames` by **completing a game** (so `start_game` has set `has_reset = false`,
   i.e. `is_old_game = true`) — not via `SetupPeriod`.
2. Run the between-games clock down past `reset_game_time` (`= config.nominal_break` initially,
   recomputed at `mod.rs:1034`).
3. The trace shows `is_old_game` flip **Y → N** at the reset instant — the kill signal for
   mutants `mod.rs:1180-1182`.

A dedicated scenario (not an extension of an existing game-completing scenario) keeps each
scenario single-purpose and easy to review.

## 6. Trace format

All-columns, fixed layout (the existing format extended with two columns in fixed positions),
so diffs make the changed value obvious. The two new columns sit in stable positions on every
line.

## 7. Field-completeness test

A unit test that destructures a `GameSnapshot` with every field named (no `..`), so adding a
field to the struct fails until `render()` is consciously updated. This forces a
render-or-explicitly-skip decision per field, making the §4 omissions auditable and intentional
rather than accidental.

**Acknowledged limitation:** it catches *addition* of new fields; it does **not** detect
semantic/behavioral changes to existing fields.

## 8. Re-bless process (differential discipline)

`scores` and `is_old_game` were never observed, so there are no prior bytes to diff them
against. The column-expansion is done in a **separate, behavior-preserving commit** with two
guarantees:
- (a) already-watched columns are byte-identical — verified by a strip-new-columns-and-diff
  against the old files (== empty);
- (b) the two new columns encode **baseline `46ec0973`** behavior — established by generating
  them against the baseline, not whatever HEAD happens to produce.

## 9. Acceptance criteria

- `render()` emits `scores` and `is_old_game`; the completeness test guards future omission.
- `regulation_with_scores` now exercises `scores`; one dedicated between-games scenario
  exercises the `is_old_game` Y→N flip.
- All golden files re-blessed; `golden_traces_match_baseline` green; both runs byte-identical
  (determinism holds).
- **Mutation-validation:** each new column kills a previously-surviving mutant — `scores`
  against a score-path mutant, `is_old_game` against the `mod.rs:1180-1182` mutants that
  survived Follow-up A. Recorded as the proof that the new columns guard something.

## 10. Boundary / future layers (out of scope)

The guard covers the **engine layer** (everything observable in `GameSnapshot`) only. These
are named future layers, not this round:
- UI→engine wiring — `apply_action` *hand-copies* the real handlers (`app/mod.rs update()` is a
  single ~2,300-line function tangled with iced/UI/hardware, `app/mod.rs:1399`), so the guard
  proves the engine reacts to method Y, not that pressing button X calls Y. Mitigated by the
  "KNOWN COUPLING POINT" lockstep rule.
- On-screen rendering — covered by the existing "layout previews" CI check.
- Hardware / sound — separate integration concerns.

## 11. Process

Lean process per `.claude/rules/plan-execution.md`: this is test-only code in `refbox`; the
engine is never touched. Heavy ceremony is not warranted (no `uwh-common` change, no wire
format, no state-machine edit). The mutation-validation step is the verification gate.
