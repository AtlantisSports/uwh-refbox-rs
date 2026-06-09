# Golden time-traces

These `*.trace` files are the **time-engine regression guard**. Each one is a recording of
exactly how `TournamentManager` reports the passage of time for one scripted game scenario:
the period, the game clock, any timeout clock, the score-confirmation pause, and every
penalty's remaining time — sampled as the game plays out and deduplicated to the sequence of
distinct observable states.

They were captured once from the last human-authored engine commit (`46ec0973`) and are the
**trusted baseline**. The test `golden_traces_match_baseline` (in `../golden/mod.rs`) replays
every scenario through *today's* engine and compares the result to these files. If today's
engine computes different time-state, the build fails with a per-scenario, line-by-line diff.

## What is and isn't watched

The guard watches **time-state only** (period, game clock, timeout type + clock,
score-confirmation pause, penalty remaining). Scores, fouls, warnings, and penalty infraction
details are intentionally *out of scope* for now. The single place that decides what is watched
is the `render()` function in `../golden/mod.rs` — extend it there if the watched set grows.

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
