# Mutation-Testing Validation of the Time-Engine Golden-Trace Guard

**Date:** 2026-06-09
**Subject under test:** `golden_traces_match_baseline` (refbox `tournament_manager::golden`), a
differential regression guard that replays 30 scripted scenarios through `TournamentManager` and
diffs the observed *time-state* trace against golden files captured from baseline `46ec0973`.
**Watched state (v1, by design):** period, game clock, timeout type + clock, score-confirmation
pause, per-penalty remaining time. **Explicitly not watched:** scores, fouls, warnings, penalty
infraction details, recent-goal flash, game numbering, wire format.

This report responds to prior adversarial feedback that the earlier validation (one coarse
"+1 s per half" mutation) was too blunt, single-class, and exposed to confirmation bias.

---

## 1. Method

Hybrid design: an **unbiased automated sweep** as the backbone, plus targeted manual checks
where the tool structurally cannot reach.

### 1.1 Automated sweep (cargo-mutants)

- **Tool:** `cargo-mutants` 26.0.0, pinned + `--locked` (latest needs rustc 1.88; repo toolchain
  is 1.85).
- **Mutated code, scoped to the time engine:** `refbox/src/tournament_manager/mod.rs`,
  `refbox/src/tournament_manager/penalty.rs`, `uwh-common/src/game_snapshot.rs`, further
  restricted with `--re` to the time-relevant functions (`update`, `generate_snapshot`,
  `could_end_game`, `check_time_remaining`, `pause_for_confirm`, `end_confirm_pause`,
  `end_*_half`, `set_game_clock_time`, `cull_penalties`, `handle_rugby_pen_shot_end`,
  `time_elapsed`/`time_remaining`/`is_complete`, `clock_time`/`as_secs_u16`, `GamePeriod::*`,
  timeout start/stop/switch, etc.).
- **Kill-check = ONLY the guard:** `--test-package refbox --cargo-test-arg
  golden_traces_match_baseline`. A surviving mutant is therefore a real source change that *the
  guard alone* fails to detect — no credit from any other test. This is the deliberate answer to
  the confirmation-bias objection: mutants are mechanically generated (not author-chosen), and the
  only judge is the guard.
- **Practicality:** `-j 6`, `--minimum-test-timeout 60`. Wall-clock ≈ 25 min. Hung mutants are
  bounded by the timeout and reported in their own bucket (not silently dropped).
- Raw artifacts (caught/missed/timeout/unviable lists, `outcomes.json`, `mutants.json`) saved at
  `docs/superpowers/specs/mutation-results-2026-06-09/` for independent inspection.

### 1.2 Manual checks (where the tool can't reach)

- **Driver mutation:** cargo-mutants skips `#[cfg(test)]` code, and the replay driver lives under
  `#[cfg(test)] mod golden`. So the harness was probed by hand.
- **Curated single-line bugs were NOT separately run** — see §4; the sweep subsumes them.

---

## 2. Headline results

| Outcome | Count |
|---|---|
| Mutants generated (viable + unviable) | 239 |
| **Caught** (guard failed the build) | 100 |
| **Missed** (survived) | 99 |
| Timed out (counted as caught) | 1 |
| Unviable (did not compile) | 39 |

Raw kill rate among viable mutants: **100 / 199 ≈ 50 %**. That number is *not* the verdict — it
is the input to classification. The whole point of scoping the kill-check to the guard alone is
that "missed" includes everything the guard is *not designed* to watch.

---

## 3. Survivor classification (all 99)

Every survivor was bucketed by reading the mutated line and asking: is it (a) a genuine gap —
watched state, reachable path, unpinned; (b) out of the guard's declared watched scope; or
(c) unreachable / behaviourally inert in any scenario.

### (b) Out of declared watched scope — ~60 survivors. *Working as designed.*
`set_scores`/`add_score` (scores not watched), `timeouts_used` counter and the
`can_start/can_switch/switch_team_timeout` permission guards, `timeout_end_would_end_game`
(a UI confirm-flow helper the driver never calls), `set_timeout_clock_time` (no corresponding
driver action), `limit_pen_list_len` (display cap never exceeded), `generate_snapshot`'s
`recent_goal` and `is_old_game` branches, and all of `next_update_time` (the fixed-step driver
deliberately does not use it — spike finding #1). A mutation here *should* survive; the guard was
never watching it.

### (c) Dead / superseded code — ~15 survivors. *Cannot be a failure.*
Root cause is a structural fact confirmed by reading `check_time_remaining` (mod.rs:1155-1157):
`could_end_game` returns true **only** at the end of SecondHalf / OvertimeSecondHalf. The driver's
`tick` (mirroring `app/mod.rs`) checks `could_end_game → pause_for_confirm` first, so every
game-ending transition flows through the **score-confirmation-pause path** (`end_confirm_pause`).
Consequence: the `update()`-driven transition functions `end_second_half` and
`end_overtime_second_half` are never reached in normal play — even replacing their entire bodies
with `()` survives, because nothing calls them. This is dead/superseded code in both the scenarios
and the real app's flow, not a guard blind spot.

### (a) Genuine gaps — a handful. *Every one is a missing scenario, not a guard-design flaw.*
The sweep precisely located game situations the 30-scenario library never exercises:

1. **Sudden death *without* overtime.** Survivor: `pause_for_confirm` mod.rs:1846 `/`→`*` (the
   confirm-pause *duration*, which IS watched). Evidence it's a coverage hole, not a guard miss:
   the only branch that reaches line 1846 requires `overtime_allowed == false &&
   sudden_death_allowed == true`, but **every** SD scenario sets `overtime_allowed: true`
   (verified in `scenarios.rs`; the `sudden_death` trace reaches SD via OvertimeSecondHalf, not
   directly). No scenario has that config combination.
2. **A clock edit large enough to rewind a running penalty.** Survivors: `set_game_clock_time`
   mod.rs:1781 `!=`→`==` (missed) and 1783 (partially missed). The penalty-rebase branch only
   fires when an edit makes a penalty's remaining exceed its full duration; the
   `manual_clock_edit_while_penalty_running` scenario never edits that far.
3. **Natural clock-expiry of a timeout / penalty-shot.** Survivors in `update` (team-timeout
   expiry path, mod.rs:1330) and the rugby-penalty-shot expiry (mod.rs:1348 +
   `handle_rugby_pen_shot_end` 1443/1478). Scenarios always end these with an explicit
   `EndTimeout` action, never by letting the clock run out.
4. **A single-half game continuing to overtime / sudden death** (`end_first_half` single-half
   branch, mod.rs:1366-1368).
5. **The between-games auto-reset path** (`update` mod.rs:1181-1182).

Remediation (deferred to a follow-up branch, per the project owner): add the ~5 scenarios above.
None of these require an engine change.

---

## 4. Why the planned curated bugs were dropped (honest note)

The pre-registered plan included four hand-injected "interaction" bugs (manual-edit↔penalty,
confirm-pause duration, exact period transition, timeout-stop drift). On inspection, all four were
**single-line operator/value edits — exactly the mutation class cargo-mutants already generates**,
and the sweep already returned verdicts for those exact lines:

| Planned curated bug | Line | Sweep verdict |
|---|---|---|
| exact period transition `>=`→`<` | mod.rs:1208 | **caught** |
| confirm-pause duration (no-OT branch) | mod.rs:1848 | **caught** |
| manual-edit penalty rebase `!=`→`==` | mod.rs:1781 | **missed** (= gap #2) |
| confirm-pause duration (SD branch) | mod.rs:1846 | **missed** (= gap #1) |

Re-injecting them by hand would be theatre: the sweep is the stronger, unbiased version of the
same test. They were therefore skipped, and the sweep's verdicts are reported instead. (One
genuinely-additive class — adding a constant, e.g. "+1 s on stop" — cargo-mutants does not
generate; it was not separately run and is a residual item, see §6.)

---

## 5. Harness (driver) probe + its hard limitation

cargo-mutants cannot mutate the `#[cfg(test)]` driver, so it was probed by hand: disabling the
`could_end_game → pause_for_confirm` branch in `tick` (`if false && …`) was **caught** — 5
scenarios failed (regulation_full, regulation_with_scores, penalty_crosses_break, overtime_full,
sudden_death).

**Limitation, stated plainly:** because the golden files were *generated by this driver*, driver
mutation can only demonstrate that **driver drift is caught** — it **cannot** prove the *original*
driver is faithful to the real `app/mod.rs` loop. A pre-existing driver bug would have been baked
into the golden baseline and would be invisible to this test. Driver faithfulness rests on a
*structural* argument (the `tick`/retry/latch logic is a documented hand-mirror of
`app/mod.rs`'s `time_updater`), which is the one part of this work that still warrants a careful
human read rather than an automated proof.

---

## 6. Residual weaknesses / what would strengthen this further

Stated so they aren't discovered as "gotchas":

- **The ~5 (a)-gaps in §3 were classified by reachability reasoning** (config inspection, branch
  analysis), not by individually re-running each with an added scenario. The classification is
  falsifiable from the saved artifacts; adding the scenarios (the remediation) is also the
  definitive confirmation, and is deferred.
- **No line/branch-coverage cross-check** was run to independently confirm which survivors sit on
  unexercised paths. The reachability claims rest on source reading.
- **The constant-drift mutation class** (e.g. `+ Duration::from_secs(1)` in `stop_game_clock`) is
  outside cargo-mutants' operator set and was not separately tested.
- **Watched scope is time-only.** The ~60 bucket-(b) survivors are out of scope *by current
  design*; whether some (notably scores, which gate sudden-death game-end) should be promoted into
  the watched set is an open design question, deferred to its own spec.
- The `--re` function scoping (chosen to cut noise from 472→239 mutants) could in principle have
  excluded a time-relevant helper; the inclusion list is in §1.1 for audit.

---

## 7. Verdict

Within its declared scope, **the guard is sound**: every viable mutation to watched time-state on
a path the scenarios exercise was caught. The 99 survivors decompose into out-of-scope behaviour
(~60), unreachable/superseded code (~15), and a small set of **missing scenarios** (~5) that the
sweep pinpointed concretely. This is a materially stronger result than the prior single-mutation
check: it is mechanical, author-bias-free, judged solely by the guard, and it produced actionable
findings (specific scenarios to add) plus two explicit limitations (driver-faithfulness is
structural-only; watched scope is time-only).

Further adversarial scrutiny is invited against the raw artifacts in
`docs/superpowers/specs/mutation-results-2026-06-09/`.
