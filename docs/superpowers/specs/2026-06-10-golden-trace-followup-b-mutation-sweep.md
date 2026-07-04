# Follow-up B — Unbiased mutation sweep results (response to §5b)

You called the targeted 3-mutant proof the weakest link and asked for an unbiased
`cargo-mutants` sweep over the scoring + between-games-reset logic, golden test as sole kill
check. Done. This is the result and classification. Raw outputs saved at
`docs/superpowers/specs/mutation-results-followup-b-2026-06-10/{caught,missed,unviable}.txt`.

## Sweep parameters

```
cargo mutants -f refbox/src/tournament_manager/mod.rs \
  -F 'add_score|set_scores|start_game|generate_snapshot|reset|update' \
  --test-package refbox --cargo-test-arg golden_traces_match_baseline \
  -j 6 --minimum-test-timeout 60
```
Kill-check = ONLY `golden_traces_match_baseline` (all scenarios incl. the new one + the
two-run determinism assert). **63 mutants: 33 caught, 22 missed, 8 unviable.**

## The columns work: every score / is_old_game VALUE mutant in an exercised path is CAUGHT

| Mutant | Effect | Result |
|---|---|---|
| `mod.rs:2196 delete !` (`is_old_game: !self.has_reset`) | inverts the old-game flag | **CAUGHT** (`old?` column) |
| `mod.rs:1181 == → !=` (reset period guard) | reset fires in wrong period | **CAUGHT** (`old?`) |
| `mod.rs:1182 <= → >` (reset threshold) | reset never fires / fires wrong | **CAUGHT** (`old?`) |
| `mod.rs:106 add_score → ()` | goal not recorded | **CAUGHT** (`score`) |
| `mod.rs:121 += → -=` / `+= → *=` | score miscounted | **CAUGHT** (`score`) |
| `mod.rs:130 set_scores → ()` | score not stored | **CAUGHT** (`score`) |
| `mod.rs:133 == → !=`, `134:13/135:13 && → ||` | SD-end guard flips → wrong period transition | **CAUGHT** (`score`/period) |

So the new columns are demonstrably sensitive to value corruption in scoring and in
is_old_game — not just to the three mutants I hand-picked.

## The 22 survivors — classified; NONE is a score/is_old_game value gap in an exercised path

| Count | Location | Bucket | Why it survives |
|---|---|---|---|
| 9 | `next_update_time` (2207-2241) | out-of-scope **by design** | the fixed-step driver never calls `next_update_time` (spike finding #1) |
| 6 | `generate_snapshot` 2168-2169 | out-of-scope **by design** | the `recent_goal` *clear* condition — `recent_goal` is the field we deliberately dropped |
| 2 | `set_scores` 134:29 (`!= → ==`), 135:16 (delete `!`) | missing-scenario edge | operands of the SuddenDeath-end-*without*-confirm-pause branch; our SD flow ends via the confirm-pause path, so this game-end edge isn't driven. The score VALUE is still set correctly (the caught `set_scores`/`+=` mutants prove it). Not a `score`-column gap. |
| 1 | `reset_game` 202 (→ `()`) | missing-scenario | the *manual* reset (operator "new game"); no scenario invokes it |
| 1 | `update` 1213 (`+ → -`) | out-of-scope / known | between-games auto-advance start time; a documented Follow-up A survivor |
| 1 | `update` 1198 (`< → <=`) | out-of-scope | rugby penalty-shot timing; not score/old-game |
| 2 | `update` 1314 (`& → |`, `& → ^`) | out-of-scope | not score/old-game; documented Follow-up A survivor |

**Verdict:** the unbiased sweep finds no surviving mutant that corrupts a watched `score` or
`is_old_game` value in a path our scenarios exercise. The survivors are (a) out-of-scope by the
guard's deliberate boundary (`next_update_time`, `recent_goal`), (b) two missing-scenario edges
unrelated to the new columns' value capture, or (c) already-catalogued Follow-up A survivors.
§5b is addressed: the targeted proof was not masking a gap.

## Two honest backlog items the sweep surfaced (not blockers, not column gaps)

These are *missing scenarios* (same category as Follow-up A's findings), not failures of the
new columns. Recording them for a possible future round; not fixing on this branch:
1. **SD score set outside a confirm pause** (`set_scores` 134:29/135:16) — would exercise the
   inline `end_game` branch in `set_scores`.
2. **Manual reset** (`reset_game` 202) — would need a `ResetGame` action + scenario.

## §5a and §5c, addressed

- **§5a (no human baseline):** documented in-code at the scenario (`scenarios.rs`), explicitly:
  the scenario pins CURRENT post-baseline (Game Block) behavior, with value resting on
  structural correctness + the mutation sensitivity above. Commit `b2b6abcb`.
- **§5c (`game_block=20` readability shortcut):** you accepted it; both reset mutants are
  confirmed caught at the shortened break, so the shortcut doesn't hide the reset logic.
- Also added (your rec 3): an in-code explanation of WHY a full game must complete (the
  `has_reset` precondition that `SetupPeriod` can't satisfy). Commit `b2b6abcb`.

## State

Branch `feat/refbox/golden-trace-scores-old-game`, 6 commits, all gates green, engine
untouched, **not pushed, no PR**. Awaiting your close-out and the user's PR approval.
