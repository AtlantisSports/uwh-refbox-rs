# Response to Adversarial Review — Follow-up B

**To the reviewer:** your critique did its job — the scope is now deliberately narrowed, and
two of your six risks dissolve rather than needing mitigation. I'm accepting four points
(three of them by *cutting scope*, which you pushed for), and rejecting two with evidence.
`file:line` references are attestations from `origin/master` (2026-06-10).

**Reframed objective.** The guard exists to protect the **stable core game business logic** —
timing rules, penalties, timeouts, scoring, period transitions, the game state machine — from
*accidental* regression while UI/navigation work proceeds. Exhaustive coverage of the
`GameSnapshot` is explicitly **not** the goal. That reframing resolves most of the tension you
identified.

---

## Accepted

### §6.1 Coupling-point growth — accepted; the cause is removed, not mitigated.
You were right that ~15 hand-mirrored actions is a first-order risk, and that a comment block
is not adequate mitigation for doubling the coupling surface. Rather than mitigate, we
**eliminate the growth**: the narrowed scope adds **zero or one** new action.

- `scores` is already mutated by existing scenarios (`AddScore` at `scenarios.rs:68-69`; the
  SD `pause_for_confirm`/`set_scores`+`end_confirm_pause` flow). Rendering the column is
  sufficient — **no new action**.
- `is_old_game` needs **one new scenario** (a `BetweenGames` auto-reset run) assembled from
  *existing* primitives (`SetupPeriod(BetweenGames, …)` + `StartClock` + run past
  `reset_game_time`) — **no new action**.

So `apply_action` grows by ≈0 arms. §6.1 no longer applies.

### §6.4 "Every operator action" — withdrawn.
We are **not** mirroring foul/warning add-edit-delete, game-scheduling / Game-Block ops,
`set_config`, or beep-test. This isn't only pragmatism — it's evidenced: `add_foul`
(`mod.rs:758`) and `add_warning` (`mod.rs:730`) are pure `Vec`-appends
(`self.fouls[color].push(...)` / `self.warnings[color].push(...)`) with **no** interaction
with the clock, penalty, timeout, or period machinery. From a time-and-state-engine
regression standpoint they are inert record-keeping; excluding them forfeits negligible
protection while avoiding 2–6 coupling points. You argued for ruthless scope; this is it.

### §6.3 Re-bless risk — accepted and strengthened.
The genuinely new columns (`scores`, `is_old_game`) were **never observed before**, so there
are no prior bytes to diff them against — your "strip-new-columns-and-diff == empty" check
covers the *existing* columns but not these. For the new columns we adopt your stronger
alternative: validate them against **baseline `46ec0973`** behavior, not HEAD, before
blessing. The expansion commit therefore carries two guarantees: (a) already-watched columns
byte-identical (strip-and-diff == empty), and (b) new columns == baseline behavior.

### §6.2 `recent_goal` — proven deterministic, then dropped on *value*, not safety.
You asked me to locate the clear path and prove it's game-time driven before adding it. Done:
`recent_goal` is cleared in `generate_snapshot` (`mod.rs:2167-2172`) iff the goal's period
differs from the current period **or** `goal_time.saturating_sub(cur_time) > RECENT_GOAL_TIME`
— both pure game-clock quantities (`goal_time` set from `game_clock_time(now)` in `add_score`,
`cur_time` is the current game-clock remaining). No wall-clock, no `next_update_time`
dependency. It **is** deterministic under fixed-step. Determinism was the gate to *allow* it;
under the narrowed "core game rules" criterion it's display sugar (highlights the last goal,
no effect on any outcome) and adds transient extra lines — so it's **dropped** anyway.

### §6.6 Faithfulness — verified, no issue.
`add_score` (`mod.rs` ~line 730 region) fires **once per action** at the action instant (the
driver applies each action a single time at its offset, not every tick) and the per-tick
`update` path is untouched by it. No per-call accumulation, no double-count, no skip. (Moot
for foul/warning now they're excluded; confirmed for `scores`.)

### §4.2 Field-completeness test — kept, reframed as a feature.
Destructure-based (no `..`), acknowledged as the pragmatic mechanism. Reframed: it forces a
*conscious* render-or-explicitly-skip decision for every `GameSnapshot` field, so the dropped
fields (`warnings`, `fouls`, `game_number`, `next_game_number`, `recent_goal`,
`next_period_len_secs`, `event_id`) become **auditable, intentional omissions** rather than
silent gaps. It guards against future field-addition rot; it does not (and is not claimed to)
detect semantic changes to existing fields.

---

## Rejected, with evidence

### "Constant columns catch nothing" — incorrect.
Your Decision-1-vs-6 argument leans on this. A column that is invariant across our scenarios
still fails the trace if a regression makes it *unexpectedly vary* (e.g. a bug bumping
`game_number` mid-game). cargo-mutants not happening to generate a mutant that perturbs a
column ≠ that column guarding nothing — those are different statements. So "watch everything"
and "mutation-prove" are two distinct guarantees, not a contradiction.

That said — the point is now **moot**, because we narrowed scope for *clarity/value* reasons
anyway. We are not watching the constant fields (`game_number`, etc.), so no dead-column
tension arises. The tie-break you requested between Decision 1 and Decision 6 doesn't need
resolving under the focused scope.

### `event_id` — agreed drop, stronger than stated.
You called it "almost always constant." It is **hardcoded** `event_id: None` in the snapshot
constructor (`mod.rs:2199`) — literally always `None`, regardless of scenario. Dropped.

---

## Final focused scope (for the record)

**Added to `render()`:**
- `scores`
- `is_old_game`
- penalty **infraction kind** — a render-only freebie: `PenaltySnapshot` already carries
  `infraction: Infraction` (`uwh-common/src/game_snapshot.rs:117`), so no `uwh-common` change
  and no new action.

**Explicitly NOT watched** (intentional, completeness-test-audited omissions): `warnings`,
`fouls`, `game_number`, `next_game_number`, `recent_goal`, `next_period_len_secs`, `event_id`.

**New actions:** none required.

**New scenarios:** (1) score-visibility — likely enhance an existing scenario rather than add
one; (2) `BetweenGames` auto-reset — covers `is_old_game` and the previously-uncatchable
mutants at `mod.rs:1180-1182`.

**Bars retained:** mutation-validation (each new column must kill a previously-surviving
mutant; `scores` and `is_old_game` are the test of that bar); field-completeness test;
behavior-preserving re-bless in a separate commit, validated against baseline `46ec0973`.

---

## Your open questions, answered

1. **`recent_goal` determinism** — proven deterministic (`mod.rs:2167-2172`); dropped on value
   grounds regardless.
2. **Decision 1 vs 6 tie-break** — moot under focused scope; we watch only provable fields
   plus `is_old_game`, so the constant-column tension doesn't arise.
3. **Penalty infraction kind** — included (verified free; snapshot already carries it).
4. **Completeness-test mechanism** — destructure-based, accepted.
5. **Re-bless guard** — strip-and-diff retained for existing columns; **plus** baseline
   validation for the two genuinely new columns.

## Residual risk we accept by design

Blind spots in warnings, fouls, scheduling/Game-Block, and beep-test are accepted: they are
not core time/state logic (fouls/warnings proven inert above). The guard protects the game
*rules* engine, not every field of every snapshot. This is the deliberate trade you pushed us
toward, and we're taking it.
