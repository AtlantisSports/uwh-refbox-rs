# Ignore Taps in the 0:00 Gap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An operator tap that lands in the split second after a countdown passes 0:00, before the
engine has moved the game on, is ignored with a log line instead of crashing the app.

**Architecture:** One gate in `handle_message` (`refbox/src/app/mod.rs`), next to the existing
`tap_outlived_its_screen` gate: if the message is an operator tap that reads the live game clock
AND the engine cannot read its game clock at this instant, log a warning and return
`Task::none()`. The decision is a pure function so a test can pin it. No engine
(`tournament_manager`) change, no `uwh-common` change, no UI or translation change.

**Tech Stack:** Rust 2024, iced 0.13, refbox crate only.

**Spec:** PO rulings in the design-lane chat, 2026-11-25 (no separate spec file):
1. Fix F1 now (crash: REF TIMEOUT / PENALTY SHOT tapped in the gap after 0:00).
2. Behaviour: **ignore the tap and log it** — the half ends normally, as if the tap came a moment
   late; the operator taps again if still wanted. Not "start it in the break", not "freeze at 0:00".
3. The same rule covers **every** operator tap that crashes the same way, not just those two.

Root cause: a running countdown past its end makes `ClockState::clock_time(now)` return `None`
(`tournament_manager/mod.rs` `clock_time`, `checked_sub`) until the updater's `update()` moves the
period on. In that window `stop_game_clock` returns `NeedsUpdate`, `game_clock_time` returns `None`
and `generate_snapshot` returns `None`; the handlers `.unwrap()` all three.

Branch: `fix/refbox/ignore-taps-at-zero`, own worktree, off a fresh `origin/master`.

## Global Constraints

- MSRV 1.85: no let-chains (`if let … && …`) — they pass every local gate (toolchain is 1.96).
- Clippy `-D warnings`; no new `unwrap()`/`expect()` in production code; no `#[allow]`.
- Every cargo job: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`, and only after the refbox
  coordinator's GO (heavy-job rule). Use `--test-threads=1` for the full refbox suite (ALSA flake).
- Touch only `refbox/src/app/mod.rs` (plus this plan file).
- Never call `clock_is_running()` for this — it reports the TIMEOUT clock. The test is
  `game_clock_time(now).is_none()`.

## Review Focus

1. **A tap outside the gap must never be dropped** — a stopped clock, a running clock before 0:00,
   and the score-confirmation pause (game clock `Stopped`) all read `Some`. Tests 2–4 in Task 1.
2. **Background messages are never dropped** — portal replies (`Recv*`, `PortalEvent`), the
   updater's `NewSnapshot`, `TimeUpdaterStarted`, alarm/spacebar press AND release (a dropped
   release would leave the buzzer on), timers (`AlarmDelayElapsed`, `TimeoutReviveHoldElapsed`,
   `AutoConfirmScores`). Test 5 in Task 1.
3. **A failing updater keeps the engine stale for up to 1 s per retry** (`UPDATER_RETRY_MAX`).
   Those taps are now dropped (each with a warning) instead of crashing. Accepted; disclose in the PR.
4. **Residual race:** the gate reads `Instant::now()` a few microseconds before the handler reads
   its own. A countdown crossing 0:00 inside those microseconds still reaches the old unwrap. The
   window shrinks from milliseconds to microseconds; not closed. Disclose in the PR.
5. **Rugby penalty shot runs the game clock beside a timeout**, so the timeout taps
   (`EndTimeout`, `CancelTimeout`) can meet the gap too — they are on the list.

---

### Task 1: The zero-gap gate

**Files:**
- Modify: `refbox/src/app/mod.rs` — add the gate in `handle_message` directly after the
  `tap_outlived_its_screen` block; add two functions beside `tap_outlived_its_screen`; add a test
  module beside `stale_tap_gate_tests`.

**Interfaces:**
- Produces: `fn tap_reads_the_live_clock(message: &Message) -> bool` and
  `fn tap_lands_in_the_zero_gap(message: &Message, tm: &TournamentManager, now: Instant) -> bool`
  (= `tap_reads_the_live_clock(message) && tm.game_clock_time(now).is_none()`).

`tap_reads_the_live_clock` returns true for exactly these (use each variant's own field shape,
`{ .. }` / `(..)`), `_ => false`:
`TeamTimeout`, `RefTimeout`, `PenaltyShot`, `EndTimeout`, `CancelTimeout`, `EditTime`,
`StartPlayNow`, `StopClock`, `AddNewScore`, `AddScoreComplete`, `ScoreEditComplete`,
`PenaltyOverviewComplete`, `WarningOverviewComplete`, `FoulOverviewComplete`,
`ConfirmationSelected`.

- [ ] **Step 1: Audit the list.** In `handle_message` (and the `apply_*` helpers it calls), find
  every operator-tap handler that, at `Instant::now()`, unwraps `generate_snapshot`,
  `game_clock_time`, `stop_clock`, `start_*_timeout`/`start_penalty_shot`, or anything that
  returns `NeedsUpdate`. Any handler reachable while the game clock runs that is not on the list
  above: add it and report it. Do not add background messages (Review Focus 2).

- [ ] **Step 2: Write the failing tests** (module `zero_gap_gate_tests`). Build a
  `TournamentManager` as the existing tests do (hold its start/stop receiver so `start_clock` does
  not panic); set the game clock to 1 s and `start_clock(t0)`.
  1. `a_clock_tap_after_zero_is_ignored`: `tap_lands_in_the_zero_gap(&Message::RefTimeout(false), &tm, t0 + 2s)` is true.
  2. `a_clock_tap_before_zero_goes_through`: same message at `t0 + 500ms` → false.
  3. `a_clock_tap_on_a_stopped_clock_goes_through`: never started, any `now` → false.
  4. `a_clock_tap_in_the_confirm_pause_goes_through`: after `pause_for_confirm`-style setup the
     existing tests use (or skip with a one-line reason if no public route exists) → false.
  5. `background_messages_are_never_ignored`: at `t0 + 2s`, false for `NewSnapshot(..)`,
     `AlarmPressed`, `AlarmReleased`, `SpacebarReleased`, `RecvSchedule(..)`, `PortalEvent(..)`,
     `TimeUpdaterStarted(..)`, `AutoConfirmScores`, `TimeoutReviveHoldElapsed(..)`, `StartClock`,
     `KeypadButtonPress(..)`.
  6. `every_listed_tap_is_ignored_in_the_gap`: at `t0 + 2s`, true for one value of every listed
     variant.
  7. `the_unguarded_tap_would_have_crashed` (regression proof): at `t0 + 2s`,
     `tm.start_ref_timeout(t0 + 2s)` is `Err(NeedsUpdate)` and `tm.generate_snapshot(t0 + 2s)` is
     `None` — the values the handlers unwrapped.

- [ ] **Step 3: Run them to see 1, 5, 6 fail to compile/fail** (functions absent).
  `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox zero_gap_gate_tests`

- [ ] **Step 4: Implement** the two functions and the gate:
  lock, compute, drop the lock, then if true
  `warn!("Ignoring {message:?}: the clock has passed 0:00 and the game has not moved on yet")`
  and `return Task::none();`. Doc comment on `tap_reads_the_live_clock` says why the list exists
  and that a new tap which reads the clock at `Instant::now()` belongs on it.

- [ ] **Step 5: Run the module, then the refbox suite.**
  `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox zero_gap_gate_tests` → all pass.
  `CARGO_BUILD_JOBS=4 cargo test -p refbox -- --test-threads=1` → all pass.

- [ ] **Step 6: Commit** `fix(refbox): ignore a tap that lands just after the clock hits 0:00`

## Acceptance

- Tests above pass; `just check` green (GO first).
- PO walk (leg 2) — the gap itself cannot be hit by hand (milliseconds, and the clock cannot be
  faked on this machine), so the walk proves **no listed button went dead in normal play**: during
  a running half tap the goal button / goal keypad DONE, TEAM TIMEOUT, REF TIMEOUT, PENALTY SHOT,
  END TIMEOUT, TIME EDIT, the penalty/warning/foul list DONE, and START NOW in a break; each must
  act at once. (A gate that misfired would make one of them do nothing — so each step can fail.)

## Deviations

- **Two more taps on the list** (Step 1 audit): `WarningEditComplete` and `FoulEditComplete`.
  Quick-adding a warning or foul from the main page calls `add_warning`/`add_foul` at
  `Instant::now()` and unwraps their `InvalidNowValue` error in the gap.
- **Test setup:** the tests use the default half length (start a real first half with
  `start_play_now`) rather than a 1 s clock; the confirm-pause test plays through half time,
  because `pause_for_confirm` treats a first-half pause as unreachable.
- **Code review (2026-11-25) found two more crashes in the same class**, now fixed:
  1. A team timeout's or rugby penalty shot's own countdown past 0:00 has no reading either;
     TIME EDIT / STOP then failed in `stop_clock`. The gate now also fires when a timeout is
     active and its clock has no reading. This needed a read-only engine accessor,
     `TournamentManager::timeout_is_active()` — no change to any game-clock rule.
  2. The 3-second revive hold on a used-up team timeout is a timer, not a tap, so the gate
     leaves it alone; its handler unwrapped the snapshot. The revive step moved into
     `revive_at()` and the handler now skips a missing snapshot with a warning (the next
     clock tick redraws the screen).
