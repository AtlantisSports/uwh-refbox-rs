# Court-finish freezes the ref timeout clock — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (native: one task,
> within the "tiny task stays in the main session" limit). Steps use checkbox (`- [ ]`) syntax.

**Goal:** A schedule refresh that finishes the court while a REF TIMEOUT is counting up between
games no longer freezes the timeout clock on screen.

**Architecture:** `set_no_next_game` parks the break at 0:00 and tells the time updater "the
clock stopped" whenever `clock_is_running()` is true. Between games under a counting-up ref
timeout that call reports the TIMEOUT clock, so the updater is told "stopped" while the timeout
clock still runs, and stops redrawing it. Keep the park (PR #4144's `court_is_parked` and the
order-A revive depend on the break sitting at 0:00), but only send "stopped" when nothing on
screen is still running. The updater must then hear "stopped" when that timeout ends on a
finished court and the refused restart leaves nothing running, so `end_timeout` sends it there.

**Tech Stack:** Rust 2024, refbox crate, `TournamentManager` (refbox/src/tournament_manager/mod.rs).

**Spec:** PO rulings 2026-11-03 (engine session): problem 4 = fix on its own branch; problem 1
moved to branch C; problem 5 = leave as is. Caller audit: every other `clock_is_running` caller
means "the clock on screen" and is correct.

**Base:** `origin/master` AFTER PR #4144 merges (it rewrites `end_timeout`'s top and adds
`court_is_parked`/`revive_court`, which this fix must stay compatible with). Branch
`fix/refbox/court-finish-freezes-timeout-clock`, own worktree. Build with `CARGO_BUILD_JOBS=4`,
coordinator GO; a fresh-worktree build is announced to the coordinator first.

## Global Constraints

- MSRV 1.85 / edition 2024; clippy `-D warnings`; no new deps; no `unwrap()` outside tests.
- Engine-only change: `refbox/src/tournament_manager/mod.rs`. No app, UI, translation or `uwh-common` change.
- Heavy process (state machine): run the whole `tournament_manager` test module, then `just check`.

## Review Focus

(Task 2 adds: 6. a Settings pick on a court parked under a running ref timeout must not count
down or kick off under it — C1; 7. a pick during any break under a timeout (running or
stopped) must not start the break under it — C3, C4; 8. a pick with no timeout is unchanged — C2.)

1. Order A (#4144): break running → REF TIMEOUT → finishing REFRESH → reviving REFRESH → END REF
   TIMEOUT must still count the owed break, not kick off at 0:00 — #4144's order-A tests must stay green.
2. A finishing refresh during a plain running break (no timeout) must still tell the updater
   "stopped" — existing test at the `set_no_next_game` park (`rx.has_changed()` + `!borrow`).
3. Ending the ref timeout on a finished court with no revive: break stays at 0:00, no game
   starts, updater told "stopped" (else the hot loop `set_no_next_game`'s comment describes) — Task 1 test B.
4. A court flagged finished at kickoff keeps its game clock — existing test
   `a_court_flagged_finished_at_kickoff_keeps_its_game_clock`.
5. A ref timeout that is STOPPED when the court finishes: no change (nothing was running).

---

### Task 1: Don't tell the updater "stopped" while a timeout clock still runs

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — `set_no_next_game` park block; `end_timeout`
  Ref/PenaltyShot `CountingUp` arm (and the Team `CountingDown` arm only if a team timeout can
  exist between games — check `can_start_team_timeout`).
- Test: same file, `#[cfg(test)]` module, next to the existing `set_no_next_game` park test.

- [x] **Step 1: Write failing tests**
  - A `a_finishing_refresh_under_a_running_ref_timeout_keeps_the_timeout_clock_running`:
    BetweenGames, next game "12", break 300 s counting down, `start_ref_timeout` (counting up);
    take `get_start_stop_rx()`, `borrow_and_update()`; `set_no_next_game()`. Assert
    `tm.clock_is_running()` (timeout clock), `clock_state == Stopped{ZERO}` (break parked), and the
    updater was NOT told stopped: `!rx.has_changed().unwrap() || *rx.borrow_and_update()`.
  - B `ending_that_timeout_on_a_finished_court_parks_and_tells_the_updater`: continue A;
    `borrow_and_update()`; `end_timeout(later)`. Assert period `BetweenGames`,
    `!tm.clock_is_running()`, `clock_state == Stopped{ZERO}`, `rx.has_changed().unwrap()` and
    `!*rx.borrow_and_update()`.
- [x] **Step 2: Run, confirm A fails** (updater told stopped) — B may pass or fail; record which.
  `CARGO_BUILD_JOBS=4 cargo test -p refbox --lib tournament_manager -- --test-threads=1 finishing_refresh_under ending_that_timeout`
- [x] **Step 3: Fix.** In `set_no_next_game`, keep the park condition and parking; replace the
  unconditional `self.send_clock_running(false)` with a send only when `!self.clock_is_running()`
  after parking, and update its comment. In `end_timeout`'s `CountingUp` arm, after clearing the
  timeout, `send_clock_running(false)` when `!self.clock_state.is_running()`.
- [x] **Step 4: Run the whole module:** `CARGO_BUILD_JOBS=4 cargo test -p refbox --lib tournament_manager -- --test-threads=1` — all green.
- [x] **Step 5: `just check`** (coordinator GO) — green.
- [x] **Step 6: Commit** `fix(refbox): keep the ref timeout clock running when the court finishes`

---

### Task 2: A game picked in Settings on a finished court waits for the timeout (problem C) + review round 2

**Why now (PO ruling 2026-11-03, "1: fold in"):** with Task 1 the updater keeps ticking under a
running ref timeout, so a Settings game-pick on a court parked under that timeout (review R1)
now visibly counts the break down under the timeout, leaves the reset point unarmed (old score
stays up) and kicks off with REF TIMEOUT on screen — `start_game` never clears `timeout_state`.
On master the Task 1 freeze hid this. The fix reuses the 11-02 rule `revive_court` already
implements: game recorded at once, countdown waits for the timeout's end.

**Scope change:** this task touches `refbox/src/app/mod.rs` (two call sites), lifting the
"engine-only" constraint for Task 2 only. `EndGameAndApply` (app ~3580) is NOT changed:
`reset_game` clears `timeout_state` first. A pick on a court that is NOT parked is unchanged.

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — new `apply_picked_game`; `update()`'s
  between-games expiry park (~2448, R4); `reset_game` comment (~894, R7); tests.
- Modify: `refbox/src/app/mod.rs` — `apply_game_options`, the `uses_remote()` branches at
  ~3392 (config-changed path) and ~3461 (game-number path).
- Modify: this plan (R8).

**Interfaces:**
- Produces: `pub fn apply_picked_game(&mut self, info: NextGameInfo, now: Instant) -> Result<()>`
  on `TournamentManager`. If `self.court_is_parked()` → `self.revive_court(info, now).map(|_| ())`;
  else `self.set_next_game(info); self.apply_next_game_start(now)`. Doc comment: the operator's
  Settings pick, same rule as a reviving REFRESH (cite `revive_court`, PO 2026-11-02 / 11-03).

- [x] **Step 1: Failing tests** (engine test module, after the Task 1 tests; reuse
  `parked_under_a_running_ref_timeout`, `parked_court`, `revived_game`, `revived_court_config`):
  - C1 `a_game_picked_on_a_court_parked_under_a_running_ref_timeout_waits_for_the_timeout`:
    `apply_picked_game(revived_game("13"), start+20s)` → `clock_state == Stopped{ZERO}`,
    `next_game_info().unwrap().number == "13"`, `clock_is_running()` (timeout clock).
    Then `end_timeout(start+30s)` → `timeout_state == None`, `clock_state.is_running()`,
    `reset_game_time != ZERO` (reset armed), updater latch `true`.
  - C2 `a_game_picked_on_a_parked_court_with_no_timeout_counts_down_at_once`: `parked_court`,
    rx `borrow_and_update`, `apply_picked_game(revived_game("13"), start)` →
    `clock_state.is_running()`, `rx.has_changed()` and `*rx.borrow()`.
  - C3 `a_game_picked_during_a_running_break_under_a_ref_timeout_keeps_the_break_running`:
    running break (game "12", 300 s) + `start_ref_timeout`, then `apply_picked_game("13")` →
    `clock_state` is `CountingDown`, `!break_owed_after_timeout` (unchanged behaviour).
  - R3 `a_site_switch_from_a_running_break_with_no_timeout_tells_the_updater_stopped`: linked,
    game "12", 300 s break running, rx `borrow_and_update`, `reset_for_site_switch(true, ..)` →
    `clock_state == Stopped{ZERO}`, `rx.has_changed()`, `!*rx.borrow()`.
  - R4 `a_break_expiring_under_a_running_ref_timeout_with_no_next_game_keeps_the_timeout_clock`:
    running break 60 s (game "12") + `start_ref_timeout`, rx `borrow_and_update`, then force
    `tm.next_game = None; tm.no_next_game = true;` (comment: no public path reaches this once
    C lands — this pins the third park site to the same answer as the other two), `update(start+61s)`
    → period `BetweenGames`, `clock_is_running()`, updater NOT told stopped
    (`!rx.has_changed().unwrap() || *rx.borrow_and_update()`).
  - R6: in `ending_a_running_timeout_on_an_owed_break_keeps_the_updater_running`, add
    `assert_ne!(tm.reset_game_time, Duration::ZERO, "the owed break arms the mid-break reset")`
    after `end_timeout`, and a comment that the latch is already `true` here, so the `running`
    send itself is pinned by `ending_the_timeout_on_an_owed_break_wakes_the_updater` (stopped timeout).
- [x] **Step 2 RED result (2026-11-03):** 286 passed, 1 failed (C1, as expected). R4 PASSED on
  old code: `start_ref_timeout` stops the break clock, so `update()` never reaches the
  between-games expiry under a timeout through this setup. C3 passing showed a pick under a
  ref timeout restarts the break UNDER it on any court (master too) — the bug C3 pinned as "unchanged".

> **REVISION (PO ruling 2026-11-03, after the RED run — supersedes the Interfaces block, C3, R4
> and Step 3 below).** Branch C's ruling covers EVERY Settings pick under a timeout, not only a
> parked court: the game is recorded at once and the break waits for the timeout's end.
> - `apply_picked_game` body = `self.revive_court(info, now).map(|_| ())` — `revive_court`
>   already defers whenever `timeout_state.is_some()` and otherwise calls `apply_next_game_start`
>   (unchanged no-timeout behaviour). Doc comment: the operator's Settings pick follows the same
>   rule as a reviving REFRESH on any court (PO 2026-11-02 / 11-03); no `court_is_parked` branch.
> - C3 REWRITE → `a_game_picked_during_a_break_under_a_running_ref_timeout_waits_for_the_timeout`:
>   300 s break running (game "12"), `start_ref_timeout`, `apply_picked_game("13")` →
>   `!clock_state.is_running()`, `next_game_info().unwrap().number == "13"`,
>   `break_owed_after_timeout`; `end_timeout(later)` → `timeout_state == None`,
>   `clock_state.is_running()`, `reset_game_time != ZERO`.
> - C4 NEW `a_game_picked_under_a_stopped_ref_timeout_does_not_start_the_break` (problem 1,
>   folded into C on 11-03: TIME EDIT froze a break counting down under a STOPPED timeout, because
>   `EditTime` reads `clock_is_running`; with this fix that state cannot arise): break running,
>   `start_ref_timeout`, stop the timeout clock the way
>   `a_court_revived_under_a_stopped_ref_timeout_does_not_kick_off_under_it` does, pick "13" →
>   `!clock_state.is_running()` and `!clock_is_running()`. Comment names the TIME EDIT freeze.
> - R4: DELETE the test and add NO guard — answered by argument (Deviations).
> - Step 3 becomes: the `apply_picked_game` body above + the R7 `reset_game` comment fix only.
> - Commit message: `fix(refbox): hold a settings game pick until the timeout ends`
> - RED for C3/C4: run the module with the stub first; C1, C3 (rewritten) and C4 must FAIL.

- [x] **Step 2: Stub, then RED.** Add `apply_picked_game` with the OLD body only
  (`set_next_game` + `apply_next_game_start`). Run the module:
  `CARGO_BUILD_JOBS=4 cargo test -p refbox --bin refbox -- --test-threads=1 tournament_manager`.
  Expect C1 and R4 to FAIL; C2, C3, R3, R6 pass (they pin unchanged behaviour). Record which.
  If R4 passes because `update()` never reaches the between-games expiry while a ref timeout
  exists, STOP and report — that means the site is unreachable under a timeout and R4 is
  answered by argument, not by a guard.
- [x] **Step 3: Fix.** Add the `court_is_parked()` branch to `apply_picked_game`. In `update()`'s
  between-games no-startable-game park, send `false` only when no timeout clock is running
  (`!(self.timeout_state.is_some() && self.clock_is_running())`; the game clock is still
  `CountingDown` at that point, so `!clock_is_running()` would be wrong), with the same comment
  shape as the other two sites. Fix `reset_game`'s comment: the other park sites now notify only
  when nothing on screen still runs; `reset_game` clears the timeout first, so unconditional is right here.
- [x] **Step 4: App wiring.** At both `uses_remote()` sites in `apply_game_options`, replace
  `tm.set_next_game(info)` + `tm.apply_next_game_start(..)` with `tm.apply_picked_game(info, now)`;
  the non-remote branches keep `set_next_game` (+ `clear_scheduled_game_start` at ~3392). Keep a
  `// Safety:` comment on the `unwrap()` (period checked BetweenGames above; next game passed in).
- [x] **Step 5: GREEN** — whole module, then `just check` (coordinator GO, `-j 4`).
- [x] **Step 6: R8** — fix this plan's Task 1 Step 1 fixture text to match the tests ("12",
  300 s, `clock_state == Stopped{ZERO}`) and add the Deviations lines below.
- [ ] **Step 7: Commit** `fix(refbox): hold a settings game pick on a finished court until the timeout ends`
- [ ] **Step 8: Code review again** (code-review skill, high) on 77594248..HEAD, worktree, base and
  file list passed explicitly.

## Deviations

- Test command: refbox has no lib target, so `cargo test -p refbox --bin refbox` replaces
  `--lib` (same tests).
- Team `CountingDown` arm of `end_timeout` left unchanged: `can_start_team_timeout` allows team
  timeouts only during a half, so none exists between games where the restart can be refused.
- #4144's `ending_a_running_timeout_on_an_owed_break_wakes_the_updater` asserted, as a fixture
  check, that parking told the updater "stopped" under a running ref timeout — problem 4
  itself. Rewritten as `..._keeps_the_updater_running`: the updater agrees with the engine
  before and after the timeout ends.
- Code review found the same freeze in `reset_for_site_switch` (reachable: between games a
  site switch is refused only while results are queued). PO ruling 2026-11-03: fold it into
  this branch. Same guard, test `a_site_switch_under_a_running_ref_timeout_keeps_the_timeout_clock_running`.
- Task 2 widened (PO 2026-11-03, after RED): branch C's ruling covers every Settings pick under
  a timeout, not only a finished court; Task 2 touches `refbox/src/app/mod.rs` (two call sites).
- Review R4 (update()'s expiry park sends "stopped" unguarded) answered by argument, no guard:
  `start_ref_timeout` / `start_penalty_shot` stop the break clock, and with Task 2 no Settings
  pick restarts it under a timeout (a REFRESH re-time is already deferred via `retime_owed`), so
  no known path runs the expiry while a timeout exists. The RED run proved the first half (the
  R4 test passed on old code); the rest is a caller argument, not a proof.
