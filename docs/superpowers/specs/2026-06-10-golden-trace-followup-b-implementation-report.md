# Implementation Report — Follow-up B (for adversarial review)

**To the reviewer:** the focused design you endorsed is now implemented on branch
`feat/refbox/golden-trace-scores-old-game` (5 commits off `origin/master` cf4661ec; #1050
confirmed merged). NOT pushed, no PR. Attack the *implementation and its validation*, not the
design (that's settled). I have flagged the three weakest points myself in §5 — start there.
`file:line` and commit SHAs are attestations from the branch.

---

## 1. What was built (vs. the locked scope)

| Locked scope item | Delivered | Commit |
|---|---|---|
| `render()` adds `score=B/W` | yes, fixed column | d30ee05f |
| `render()` adds `old?=Y/N` (`is_old_game`) | yes, fixed column | d30ee05f |
| Re-bless 37 existing traces (behavior-preserving) | yes | d30ee05f |
| Field-completeness destructure test | yes (`render_accounts_for_every_snapshot_field`) | 3abb1d58 |
| Dedicated between-games-reset scenario | yes (`between_games_auto_reset`) | e7053bd6 |
| README update | yes | 721b2e52 |
| **One new action `StartPlayNow`** (the planned deviation) | yes | 92c65658 |

`render()` line is now (`golden/mod.rs`):
`period=… | clock=…s | score=B/W | timeout=… | conf_pause=… | old?=Y/N | pens=[…]`

## 2. The one deviation from the design (was approved, re-flagged)

The design's "no new actions" property did **not** hold. Verified engine fact: `has_reset=false`
is set only in `start_game` (`mod.rs:1076`), reachable only via `start_play_now`-from-BetweenGames
(`mod.rs:1708`) or the end-of-break auto-advance (`mod.rs:1213`). Every existing scenario uses
`SetupPeriod`+`StartClock`, which never call `start_game` — which is exactly *why* the 1180-1182
mutants survived Follow-up A. So closing the gap required `Action::StartPlayNow` (mirrors
`Message::StartPlayNow` → `tm.start_play_now(now)`). One new coupling point; also a fidelity
improvement (real games start this way). The user approved this mid-execution.

## 3. Validation evidence

**3a. Baseline cross-check (the 37 existing scenarios).** Copied the updated `golden/` module
into a `46ec0973` worktree (removed the one `game_block` config line that doesn't exist at
baseline; compiled clean against the baseline engine), regenerated traces, and diffed:
**all 37 existing scenarios byte-identical baseline-vs-HEAD, including the two new columns.**
So `scores`/`is_old_game` on pre-existing flows encode trusted baseline behavior, not AI drift.

**3b. Mutation proof (manual, reverted).** Golden test as sole kill-check:
- `mod.rs:1181` `== GamePeriod::BetweenGames` → `!=` → FAIL on `between_games_auto_reset` ✅
- `mod.rs:1182` `<= reset_game_time` → `>` → FAIL on `between_games_auto_reset` ✅ (the diff: `old?` stays Y instead of flipping to N)
- `mod.rs:121` `scores[color] += 1` → `+= 0` → FAIL on `regulation_with_scores`, `overtime_score_ends`, `single_half`, `single_half_decided` ✅
- All three reverted; `git status` clean; engine untouched.

**3c. Other gates.** 229/229 refbox tests pass; `cargo clippy -p refbox -- -D warnings` clean;
determinism assert (each scenario run twice, byte-identical) green for the new scenario; a final
independent code review returned APPROVED, no issues, scope boundary confirmed (only
`golden/` + `golden_traces/` touched).

**3d. The new scenario's trace** (compact, 19 lines; the flip is the point):
```
period=FirstHalf     | clock=  3s | score=B0/W0 | … | old?=Y | pens=[]
… FirstHalf→HalfTime→SecondHalf, all old?=Y …
period=SecondHalf    | clock=  0s | … | conf_pause=0s | old?=Y | pens=[]
period=BetweenGames  | clock= 10s | … | conf_pause=none | old?=Y | pens=[]   ← break begins
period=BetweenGames  | clock=  9s | … | conf_pause=none | old?=N | pens=[]   ← AUTO-RESET fires
… old?=N down to clock=6s …
```

## 4. A finding worth recording

Existing scenarios DO show `old?=Y` — but that is **game 2** reached via the between-games
*auto-advance* (`start_game` at `mod.rs:1213`), not the *auto-reset*. None of them run long
enough to hit game 2's own reset, so 1180-1182 stayed uncovered until this scenario. (This also
means `is_old_game` was already exercised for the auto-advance path; the baseline cross-check in
§3a covers that, and it diffed clean.)

---

## 5. The three weakest points — attack these

**5a. The new scenario has NO baseline to validate against.** Unlike the 37 existing scenarios
(§3a), `between_games_auto_reset` exercises the Game Block era's between-games timing, which did
not exist at `46ec0973` (no `game_block` field). So its trace is pinned to **current** behavior,
not a trusted human baseline. **The honest risk:** if the *current* between-games-reset behavior
is itself an AI-introduced bug, this scenario blesses the bug. My defense is only structural: the
observed behavior (`is_old_game` flips Y→N once the break clock passes `reset_game_time =
break_total − post_game_duration`) matches the documented intent, and the §3b mutants confirm the
guard is *sensitive* to that logic. Is "pin current + mutation-sensitive, no human baseline"
acceptable for post-baseline logic, or do you want a hand-derived expected-trace cross-check?

**5b. Mutation proof was 3 hand-picked mutants, not an unbiased sweep.** I targeted the two
1180-1182 survivors named in Follow-up A plus one representative score mutant. This proves those
specific mutants die — but it's confirmation-biased (I picked mutants I expected the new columns
to catch). Follow-up A ran a full `cargo-mutants` sweep over the engine regions and classified
*all* survivors. I did not re-sweep. **Attack:** is targeted proof sufficient here, or should I
run `cargo-mutants` (installed, v26.0.0) over the scoring + reset code regions with the golden
test as kill-check, to find any score/is_old_game mutant the new columns *miss*? (I lean toward
running it — it's the rigorous, non-circular check, and it's cheap.)

**5c. `between_games_config` sets `game_block: 20s` purely for trace readability.** This shrinks
the between-games clock from the default 2880s to ~10s so the trace is compact. I argued this
doesn't weaken the test (the reset *logic* is identical; only the clock *duration* differs, and
§3b confirms both reset mutants still die at this duration). **Attack:** does shrinking the break
hide any edge case — e.g., could a mutant survive at a 10s break that would be caught at a
realistic break length, or vice-versa?

## 6. Specific questions

1. §5a — accept "no human baseline, pinned current + mutation-sensitive" for the new scenario, or require a hand-derived expected trace?
2. §5b — run the unbiased `cargo-mutants` sweep over scoring + reset regions before merge, or is the targeted 3-mutant proof enough?
3. §5c — is the `game_block=20` readability shortcut sound, or should the scenario use a realistic break length and accept a longer trace?
4. Anything in §1–§4 that looks wrong, over-claimed, or too clean.

State of play: implementation complete, all gates green, on the branch, **not pushed**. No PR until this review closes and the user approves.
