# Design: Behind-schedule in-game figure = raw deviation tally (no in-game buffer)

Date: 2026-06-08
Crates: `refbox` only (`src/tournament_manager/mod.rs`).
Status: approved (brainstorm) — pending written-spec review.
Supersedes the in-game calculation of `2026-06-08-behind-schedule-projection-refinement-design.md`
(which kept a `slot_buffer` term). Confirmed with the domain expert.

## Problem

The in-game figure subtracts the slot's slack (`slot_buffer = game_block − regulation_play −
minimum_break`) during the game. That pre-applies the break-compression catch-up *while the game is
still running*, which hides in-game time savings: skipping half-time, or trimming the clock, does not
reduce the figure until the saving exceeds the slack — and never reduces lateness that was *carried in*
from a late start. The operator wants the in-game figure to be a straightforward running tally.

## The model (confirmed)

> **Delay = how late the game started, plus every second the clock is frozen or edited up, minus every
> removal (skipping a half early, editing the clock down). Floored at zero.**

There is **no buffer term during the game**. The slot's slack (the catch-up / "clawback") is realised
**between games**, where the break actually compresses toward the minimum — the figure steps down at the
break by however much the break is compressed. This is the chosen, confirmed behaviour.

### In-game formula

```
deviation(now) = (real_elapsed + remaining_regulation(now)) − regulation_play     // signed
behind         = max(0, inherited + deviation(now))
```

- `real_elapsed = now − game_start_time`.
- `remaining_regulation(now)` — unchanged from the projection refinement: live remaining on the current
  regulation period + full durations of later regulation periods; ZERO in extra time / between games.
- `inherited = game_start_time − current_scheduled_start` (≥ 0) — how late the game started, shown in full.
- `deviation` is the net of stoppages/edits-up (positive) and removals/skips/edits-down (negative); it can
  be negative, so a removal reduces the figure even below the inherited amount. The `max(0, …)` is applied
  to the **whole** `inherited + deviation`, so the figure never goes negative.
- **No `slot_buffer`.** Remove that term from the in-game branch entirely.

### Between-games (unchanged)

```
behind = max(0, (now + remaining_break_clock) − scheduled_next_start)
```

This already realises the clawback: when a game ends behind, `remaining_break_clock` is floored at the
minimum break, so the projected next start (and thus the figure) reflects the compressed break. The
figure therefore **steps down by the slot's slack at the game-end boundary** when behind — that step is
the catch-up, shown at the moment it happens. (This replaces the prior "continuous at the boundary"
property, which depended on the now-removed in-game buffer.)

### Behaviour summary

- Late start → figure shows the lateness; stays there while the game plays normally.
- Any freeze (pause, team/ref timeout, normal penalty shot, edit-up) → climbs second-by-second.
- Rugby penalty shot (main clock runs) → frozen.
- Skipping a half early, or editing the clock down → drops immediately by that amount.
- Overtime/sudden-death → climbs (remaining_regulation = 0).
- Game ends behind → figure steps **down** by the compressed-break slack (clawback) as the break begins.
- Long scheduled gap (e.g. lunch) → absorbs the lateness **at the break** (figure drops then), not during
  the prior game.

## Implementation

Single production change in `behind_schedule`'s in-game (`else`) branch:
- Delete the `slot_buffer` computation.
- Replace `let developing = overrun.saturating_sub(slot_buffer); inherited + developing` with a signed
  combine + single floor:
  ```rust
  let projected_total = real_elapsed + self.remaining_regulation(now);
  let reg = self.config.regulation_play();
  if projected_total >= reg {
      inherited + (projected_total - reg)
  } else {
      inherited.saturating_sub(reg - projected_total)
  }
  ```
`remaining_regulation`, `inherited`, the between-games branch, the display, the setting, and config are
unchanged. No `uwh-common`/wire-format change.

## Testing

Existing in-game-branch tests had the buffer subtracted, so their expected literals shift; recompute each
by hand from the new formula (do not weaken assertions):

- `..._grows_with_in_game_stoppage_beyond_buffer`: 20s stopped → **20** (was 5); the 10s-stopped sample → **10** (was 0). (Rename is optional; the slack no longer applies in-game.)
- `..._frozen_while_running_clock_exceeds_period_len`: keep `a == b`; frozen value → **30** (was 15).
- `..._in_game_edit_up_…`: `before` → **50** (was 5); keep `after == before + 30s` (→ 80).
- `..._in_game_edit_down_…`: `before` → **90** (was 45); keep `after == before − 30s` (→ 60).
- `..._accrues_during_time_pause`, `..._accrues_during_ref_timeout`: recompute the absolute literals (the
  per-second climb is unchanged; only the offset that the buffer used to remove changes).
- `..._inherited_lateness_persists_in_manual_mode`, `..._single_period_game`: recompute (likely unchanged
  when deviation is 0, since `inherited + 0`).
- `..._frozen_through_half_time`: still **0** (deviation 0).
- `..._climbs_in_overtime_beyond_buffer`, `..._robust_to_midgame_half_length_change`: assertions are
  inequalities / equal-samples (frozen), so they still hold; verify.

Two tests need **rewriting** (their premise changes):
- `..._continuity_at_game_end_manual` → assert the **step-down**: the in-game value just before `end_game`
  exceeds the between-games value just after by exactly the slot slack (`game_block − regulation_play −
  minimum_break`). Compute both and assert the difference.
- `..._long_portal_gap_absorbs_overrun` → the long gap now absorbs **at the break**, not during the game:
  during the game the figure shows the raw overrun; after `end_game` into the long gap it drops to ZERO.
  Restructure to sample in-game (raw, > 0) then after the game ends (absorbed → 0).

Between-games tests (`..._between_games_overdue`, `..._recovered_by_long_break`,
`..._between_games_climbs_when_break_overdue`, `..._between_games_follows_break_edit`,
`..._zero_before_first_game_and_when_ahead`) use the unchanged branch and should pass as-is.

## Acceptance criteria

1. Skipping a half early (Start Now during half-time) drops the figure by the skipped amount, **even when
   the delay was carried in from a late start**.
2. The figure shows the raw delay during a game (start-lateness + freezes − removals) and steps down by the
   compressed-break slack when a game ends behind.
3. `just check` passes; all behind-schedule tests pass with recomputed literals (none weakened).

## Blast radius & process

High — game-clock read-model; many test literals change. Heavy process, test-first. Net production change
is a few lines (removing the buffer term). Confined to `refbox/src/tournament_manager/mod.rs`.
