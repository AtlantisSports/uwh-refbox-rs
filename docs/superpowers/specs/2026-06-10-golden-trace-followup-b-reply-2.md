# Closing Reply to Adversarial Review — Follow-up B

Endorsement noted. All four tightening points are accepted and folded in; one of them
reverses a claim I made, and the net effect shrinks the render change to exactly the two
fields you originally argued for — by evidence, not capitulation. `file:line` are
attestations from `origin/master` (2026-06-10).

## 1. `scores` — render-only, no new scenario. Confirmed.
`regulation_with_scores` already calls `AddScore` (`scenarios.rs:68-69`). Adding `scores` is a
`render()` column change plus re-bless; it needs **no** new scenario. The only new scenario in
this round is the between-games reset (possibly even zero new scenarios if we instead extend
an existing game-completing scenario — see §2).

## 2. `BetweenGames` auto-reset scenario — shape pinned, with a binding precondition.
You asked for its shape. The controlling fact: `is_old_game = !has_reset` (`mod.rs:2196`); the
constructor sets `has_reset = true` (so `is_old_game = false` initially); and `SetupPeriod`
(`set_period_and_game_clock_time`) does **not** modify `has_reset`. The auto-reset at
`mod.rs:1180-1182` fires only when `!has_reset && current_period == BetweenGames &&
game_clock_time(now) <= reset_game_time`.

Consequence: a scenario that merely `SetupPeriod(BetweenGames, …)` + runs leaves
`has_reset = true`, the reset never fires, and the mutants survive **again**. The scenario
must instead:
1. Reach `BetweenGames` by **completing a game** (so `start_game` has set `has_reset = false`,
   i.e. `is_old_game = true`),
2. Run the between-games clock down past `reset_game_time` (`reset_game_time` =
   `config.nominal_break` initially, recomputed at `mod.rs:1034`),
3. Observe `is_old_game` flip **Y → N** at the reset instant.

That flip is the kill signal for the previously-uncatchable mutants at 1180-1182, and the
mutation-validation step must demonstrate it.

## 3. Penalty infraction kind — withdrawn.
You were right to ask whether scenarios vary the infraction. They don't: every `StartPenalty`
in the driver hardcodes `Infraction::Unknown` (the `PenaltyKind` values — `OneMinute`,
`TwoMinute`, `ThirtySecond`, `TotalDismissal` — are *durations*, not infractions, per
`scenarios.rs`). The column would be a constant `Unknown`. Making it vary requires adding an
infraction argument to `StartPenalty` plus a scenario — new coupling for a *descriptive
metadata* field that isn't core timing/state logic (the same reasoning that excluded
fouls/warnings). So it's **dropped**. My earlier "freebie" framing was wrong: free to render,
not free to make useful.

## 4. Completeness-test limitation — acknowledged.
The destructure-based test guards against *addition* of new `GameSnapshot` fields going
unrendered; it does **not** detect semantic/behavioral changes to existing fields. That's its
known boundary, not an oversight.

## Locked scope
- **`render()` adds:** `scores`, `is_old_game`. Nothing else.
- **Dropped (intentional, completeness-test-audited):** `warnings`, `fouls`, `game_number`,
  `next_game_number`, `recent_goal`, `next_period_len_secs`, `event_id`, penalty
  infraction-kind.
- **New actions:** none.
- **New scenarios:** one between-games auto-reset (via game completion), or an extension of an
  existing game-completing scenario; plus `scores` becoming visible in the existing scoring
  scenario.
- **Bars retained:** each new column must kill a previously-surviving mutant (cargo-mutants);
  field-completeness test; behavior-preserving re-bless in a separate commit, with the two new
  columns validated against baseline `46ec0973`.

No open questions remain. Proceeding to the implementation plan.
