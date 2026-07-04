# Decision report — roll the two surfaced scenarios into this PR? (for adversarial review)

PR #1059 is open and you closed out the validation as "good enough to merge." The unbiased
`cargo-mutants` sweep surfaced two surviving mutants I classified as *missing-scenario* items
(not gaps in the new `score`/`is_old_game` columns) and recorded as backlog. The question now:
**should these be folded into PR #1059, or stay a separate follow-up?** I want your read,
because it pits two principles you've both pushed against each other.

## The two surfaced items

| Survivor | What it is | To cover it faithfully needs… |
|---|---|---|
| `reset_game` (mod.rs:202, body→`()`) | the operator **manual "new game" reset** | a new `ResetGame` action (mirrors the reset button) + 1 scenario |
| `set_scores` 134:29 / 135:16 | the **sudden-death inline `end_game`** branch, reachable only when a score is set in SD *outside* a confirmation pause | a new score-editor `SetScores` action (the only faithful caller of that path) + 1 scenario |

Neither is a failure of the `score`/`is_old_game` columns — both columns capture their values
correctly; these are *additional operator actions* whose engine effects aren't currently driven.

## The tension

- **One-branch-one-concern / minimal coupling (you pushed hard for this in the design round):**
  this PR's concern is "watch `score` + `is_old_game`." Each new scenario above requires adding a
  **new hand-mirrored `apply_action` arm** — `ResetGame`, and for the SD case a `SetScores`
  editor action. Adding actions is exactly the coupling growth we deliberately cut to one
  (`StartPlayNow`). Rolling these in re-opens that decision and grows the KNOWN COUPLING POINT
  surface to 2–3 new arms.
- **Relatedness / completeness (the human's instinct):** they're the same subsystem
  (between-games + scoring), surfaced by the same sweep, on the same guard. Splitting closely
  related coverage across PRs has its own cost (review overhead, a second branch/CI cycle, the
  reset story left half-finished).

## Faithfulness note (matters for your call)

- `ResetGame` is clean and faithful: the manual reset is a real, common operator action; the
  scenario (play a game → ResetGame → scores clear, `old?`→N, period→BetweenGames) is
  straightforward and mutation-provable against the `reset_game` survivor.
- The SD `set_scores` path is **marginal**: normal SD goals are confirm-gated, so the inline
  `end_game` in `set_scores` is only reachable via a manual score *edit during* sudden death.
  Faithfully driving it needs the `SetScores` editor action we explicitly descoped, and it
  guards an edge most operators never hit. There's a real argument it's near-equivalent/dead in
  practice.

## Options

1. **Add `ResetGame` scenario only.** One new action, clean, core; kills the `reset_game`
   survivor. Leave the SD-editor case as backlog (needs a descoped action, marginal path).
2. **Add both.** Also add `SetScores` + an SD-edit scenario. Closes both survivors but adds two
   new actions for one marginal edge — the coupling growth you warned against.
3. **Keep both as backlog / separate branch.** PR #1059 stays the focused `score`+`is_old_game`
   change; do the reset (and maybe SD-editor) scenarios in their own branch, preserving
   one-branch-one-concern.

## What I'm asking

Given you argued *for* minimal coupling in the design round but *also* for honest completeness
and tracking these as backlog — which option best fits? My lean is **Option 1** (the manual
reset is clean, faithful, and core; the SD-editor case is marginal and needs a descoped action,
so it stays backlog). Push back if you think Option 3 (keep this PR pure) or Option 2 (close
everything now) is the better trade.

State: PR #1059 open, unmerged, CI running; adding scenarios is feasible now (will re-trigger
CI + a re-review).
