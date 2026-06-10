# Golden time-traces

These `*.trace` files are the **time-engine regression guard**. Each one is a recording of
exactly how `TournamentManager` reports core game state for one scripted game scenario:
the period, the game clock, any timeout clock, the score-confirmation pause, every penalty's
remaining time, the **score** (`score=B/W`), and the **between-games "old game" flag**
(`old?=Y/N`, i.e. `is_old_game`) — sampled as the game plays out and deduplicated to the
sequence of distinct observable states.

They were captured once from the last human-authored engine commit (`46ec0973`) and are the
**trusted baseline**. The test `golden_traces_match_baseline` (in `../golden/mod.rs`) replays
every scenario through *today's* engine and compares the result to these files. If today's
engine computes different time-state, the build fails with a per-scenario, line-by-line diff.

## What is and isn't watched

The guard watches the core game-state engine: period, game clock, timeout type + clock,
score-confirmation pause, penalty remaining, **score**, and **`is_old_game`** (the
between-games auto-reset flag). Intentionally *out of scope* — they are record-keeping or
display fields, not core timing/state logic, so a regression in them can't corrupt the clock or
game flow: **fouls, warnings, penalty infraction kind, game numbers, recent-goal marker,
next-period length, and event id**.

The single place that decides what is watched is the `render()` function in `../golden/mod.rs`.
A companion test, `render_accounts_for_every_snapshot_field`, destructures `GameSnapshot` with
no `..`, so adding a new field to the snapshot fails to compile until someone consciously
decides whether `render()` should watch it — the out-of-scope omissions above are deliberate,
not accidental.

## When a change makes a `.trace` file change

A changed `.trace` file means the engine's observable time behaviour changed. That is either a
**regression to fix** or an **intended change**. Decide which, then:

- **Regression:** fix the engine; the trace should go back to matching with no re-bless.
- **Intended change:** re-bless (below), and **add one line per changed scenario to the PR body**
  classifying it, e.g. `Blessed sudden_death: pre-sudden-death break shortened (feat #1234)`.

**Rule: every PR that changes a `.trace` file must classify each change, one line each, in the
PR body.** A re-bless with no explanation is not acceptable — it defeats the guard.

## How to re-bless

```sh
UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline
```

This rewrites every `.trace` file from the current engine. Review the resulting `git diff`
before committing — it is the exact behaviour change you are pinning. (Any value of
`UPDATE_GOLDEN` triggers a re-bless; it is read only inside the test, and CI never sets it, so
CI always compares.)

## Adding a scenario

Add it to `scenarios::all()` in `../golden/scenarios.rs`, then run the re-bless command once to
generate its `.trace` file. Commit the scenario and its new golden file together.
