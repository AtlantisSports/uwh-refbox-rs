# Sleep Clock Catch-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> This plan is an **architectural sketch with a task list**, per `.claude/rules/plan-execution.md`,
> which caps plans at ~800 lines and asks for "not step-by-step scripts for every line of code".
> Signatures, design decisions and test *cases* are specified; test and function *bodies* are the
> implementer's to write. Where a decision is load-bearing it is stated as a requirement, not a hint.

**Goal:** When the machine has been asleep (or its clock jumped forward), refbox re-places its clocks at the true schedule position on the next tick, without operator action.

**Architecture:** Two pure modules — jump detection and schedule placement — plus one new engine method that sets game number, config, period and clock together. Detection runs from the existing single tick seam in the app's clock updater. The engine is given the selected court's game list so placement can cross game boundaries.

**Tech Stack:** Rust 2024, `std::time::Instant` (monotonic), `time::OffsetDateTime` (wall clock), the existing `TournamentManager` engine, the existing golden-trace harness.

**Spec:** `docs/superpowers/specs/2026-09-10-sleep-clock-catchup-design.md`

**Process:** Heavy, per `.claude/rules/plan-execution.md` — per-task verification and per-task code review. Blast radius is the game engine.

**Out of scope** (from the spec; `.claude/rules/scope.md` applies — flag, do not fix): how Portal
schedule times are fetched or parsed, that path is already correct on both platforms; OS-level
keep-awake assertions; any change to the score-confirmation pause or its duration; any new
operator-facing screen. The known transition defects this work lands near — the offline phantom
POST, the replayed-game repost loop, the stale restore note — are **not** fixed here.

## Global Constraints

- **MSRV 1.85, edition 2024.** No APIs newer than 1.85.
- **`uwh-common` is not touched.** Blast radius stays inside `refbox`. `GamePeriod::duration` is used as-is.
- **No new dependencies.** `time` and `tokio` are already present.
- **No `unwrap()`/`expect()` in production code** without a comment explaining why it cannot panic.
- **Clippy `-D warnings`** clean on all targets.
- **No new UI.** The correction goes to the log and nowhere else.
- **Threshold: 5 minutes**, in one named constant `TIME_JUMP_THRESHOLD`. Raised from 10
  seconds by Eric's ruling during implementation — at 10 seconds a routine clock nudge
  abandoned a live game at a late-running tournament. See the design document's Detection
  section for the full reasoning and the accepted residual.
- **Never wind backwards.** A backwards wall-clock step is logged and ignored.

---

## Gates and rulings — settled, do not re-open

**The spec's precondition is satisfied.** The spec opens with "On the affected Mac, before anything
else: Settings → APPLY", and calls itself void if that does not correct the countdown. That test
was **never run** — the tournament moved on and the broken state was gone. It was replaced by a
stronger check: Eric confirmed the Mac was deliberately slept for about two hours, and the error
was ~126 minutes. The sleep duration was established independently of the error and the two match,
which is what the APPLY test could never have shown — APPLY would have corrected a stale countdown
whatever caused it. Games 2 onward also ran correctly, which rules out a wrong machine clock, wrong
court, or wrong schedule. Do not re-run the APPLY test and do not treat the spec as ungated.

**Ruling (Eric, 2026-09-10): a game landed inside is started, however little time remains.**
The spec's wording — "a game the catch-up lands *inside* has not been passed over: it continues" —
is followed literally. Claude raised the boundary case and recommended treating a near-finished
game as passed over; Eric overruled that. Two consequences are **accepted risks, not oversights**:

- Landing seconds before a game's end starts it fresh at 0-0, and it ends almost at once, so a 0-0
  result can reach the Portal — the outcome the spec rejects elsewhere.
- In a knockout game a level score runs to overtime and then sudden death, which counts **up** with
  no finish condition. The clock wedges there until an operator intervenes.

Do not re-open this in implementation or review. **Do** carry both consequences into the PR body and
into Eric's hardware walkthrough, so the risk is visible to whoever runs the next tournament.

---

## Spec Corrections — read before starting

Citations verified against master `3d593fdb` on 2026-09-10. **Every line number in the spec is
wrong.** The code is right, the locations are not. Use these:

| Spec says | Actually at |
|---|---|
| `mod.rs:1103-1107` (log lines) | **`mod.rs:1283-1287`**, in `calc_time_to_next_game` |
| `mod.rs:1173-1176` (countdown anchor) | **`mod.rs:1388-1391`**, in `apply_next_game_start` |
| `mod.rs:1231-1233` (end-of-game re-derive) | **`mod.rs:1466-1470`**, in `end_game` |
| `mod.rs:2050` (`set_game_clock_time`) | **`mod.rs:2351`** |
| `mod.rs:1656-1676` (`end_second_half`) | **`mod.rs:1937`** |
| `game_snapshot.rs:187` (`GamePeriod::duration`) | **`game_snapshot.rs:211`** |
| "app call sites 1750, 1898, 5121" (three) | **four**: `app/mod.rs:2829, 2898, 3017, 6700` |

Three substantive corrections, which this plan already accounts for:

1. **Detection cannot be tested by injecting `Instant` alone.** The spec's Testing section says
   "the engine takes the current time as a parameter throughout, so a test simulates a two-hour
   sleep by handing it a jumped time". True for placement, **false for detection**: a sleep *is*
   the wall clock advancing while the monotonic clock does not, so a test must control both.
   Every detection entry point here takes both clocks.
2. **The engine has no schedule.** `TournamentManager` holds only `next_game` — one game ahead.
   Crossing game boundaries needs the app's `schedule` (`app/mod.rs:198`) and `current_court`
   (`app/mod.rs:208`). Task 4 pushes a compact court list into the engine.
3. **`set_game_clock_time` re-bases penalties** (`mod.rs:2360-2375`) when a new time would leave a
   penalty longer than its nominal length. Right for an operator editing the clock, wrong here —
   after a catch-up the penalty genuinely has run that long. Task 3 must not inherit it.

One spec claim held with a caveat: penalty *countdowns* do key off `(period, time, config)`
(`penalty.rs:58, 105, 133`), so they follow the new position unaided. But `Penalty::start_instant`
(`penalty.rs:53`) is a monotonic anchor feeding `calculate_timestamp` for stats, so a penalty
surviving a catch-up carries a wall-clock timestamp wrong by the sleep duration. Minor, arguably
pre-existing; **recorded, not fixed here**.

---

## File Structure

| File | Responsibility |
|---|---|
| `refbox/src/tournament_manager/time_jump.rs` | **New.** Pure detection: holds the previous clock pair, reports lost time. |
| `refbox/src/tournament_manager/placement.rs` | **New.** Pure arithmetic: given a court's games and a moment, which game and where inside it. |
| `refbox/src/tournament_manager/mod.rs` | `place_at_schedule_position`, `set_court_schedule`, `observe_time_jump`, two fields, two `mod` lines. |
| `refbox/src/app/mod.rs` | Supply the court game list; one call at the tick seam. |
| `refbox/src/tournament_manager/golden/{mod,scenarios}.rs` | One `Action` variant, its dispatch arm, one scenario. |
| `refbox/src/tournament_manager/golden_traces/*.trace` | One new baseline file, generated. |

---

## Task 1: Jump detection

**Files:** create `time_jump.rs`; add `mod time_jump;` to `mod.rs` beside `mod game_stats;` (`:31`).

**Produces:**
```rust
pub(crate) const TIME_JUMP_THRESHOLD: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
pub(crate) struct JumpDetector { last: Option<(Instant, OffsetDateTime)> }

impl JumpDetector {
    /// Record this tick's clocks; report the time the monotonic clock lost, if over
    /// the threshold. `None` on the first call, when the clocks moved together, and
    /// when the wall clock moved backwards.
    pub(crate) fn observe(&mut self, mono: Instant, wall: OffsetDateTime) -> Option<Duration>;
}
```

**The arithmetic — the one part worth specifying exactly.** `time::Duration` is signed and
`std::time::Duration` is not, and the order of these steps is what keeps a rewind from being
reported as a forward gap:

1. `self.last.replace((mono, wall))?` — rebase *unconditionally*, then early-return on the first call.
   Rebasing before the guards is required: a backwards step that left `last` untouched would report
   the whole rewind back as a forward jump on the following tick.
2. `wall - last_wall`; if negative, `warn!` and return `None`.
3. `try_into()` to an unsigned `Duration`; `.ok()?` (only fails on an unrepresentable value).
4. `lost = wall_delta.saturating_sub(mono.saturating_duration_since(last_mono))`.
5. `(lost >= TIME_JUMP_THRESHOLD).then_some(lost)`.

**A corrected system clock is treated exactly like a sleep, deliberately** — the spec's ruling. In
both cases the true time has moved and the schedule position should follow, so do not try to tell
them apart. The threshold is the only defence against a routine time-server nudge, and it is
tunable during implementation: if 10s proves too twitchy, change the constant and record it in
Deviations.

**Tests** (in-file `#[cfg(test)] mod test`):
- `first_observation_reports_nothing`
- `a_two_hour_sleep_reports_two_hours` — monotonic +20ms, wall +7200s; assert within 1s of 7200s
- `nothing_is_reported_when_there_is_no_real_jump` — table over three cases: clocks moving
  together, a gap one second below the threshold, a one-hour backwards step
- `a_backwards_step_still_rebases_so_the_next_gap_is_measured_from_it` — guards step 1 above

- [ ] Write the tests
- [ ] Run `cargo test -p refbox time_jump` — expect FAIL (`cannot find JumpDetector`)
- [ ] Write the implementation
- [ ] Run `cargo test -p refbox time_jump` — expect PASS
- [ ] Commit: `feat(refbox): detect wall-clock jumps the monotonic clock missed`

---

## Task 2: Schedule placement arithmetic

**Files:** create `placement.rs`; add `mod placement;` to `mod.rs`.

**Produces:**
```rust
/// One scheduled game on the selected court, reduced to what placement needs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScheduledGame {
    pub(crate) number: GameNumber,
    pub(crate) start_time: OffsetDateTime,
    pub(crate) config: GameConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Placement {
    BeforeFirstGame { until_start: Duration },
    InGame { index: usize, period: GamePeriod, time_remaining: Duration },
    InBreak { next_index: usize, until_start: Duration },
    PastLastGame,
}

/// Length of a regulation game: both halves plus half time.
pub(crate) fn regulation_length(config: &GameConfig) -> Duration;

/// Where `target` falls in `games`. `games` must be sorted by `start_time` ascending.
pub(crate) fn place(games: &[ScheduledGame], target: OffsetDateTime) -> Placement;
```

**Design requirements:**
- **Regulation only** — first half, half time, second half. Overtime and sudden-death lengths
  depend on the score, and a game nobody played has none. This is the spec's ruling, not an omission.
- `GamePeriod::duration` returns `None` only for `BetweenGames` and `SuddenDeath`, so
  `unwrap_or_default()` on the three regulation periods is safe — say so in a comment.
- Walk the games once: before the first → `BeforeFirstGame`; inside `[start, start+length)` →
  `InGame`; past this game's end but before the next one's start → `InBreak`; fell out of the
  loop → `PastLastGame`. An empty slice is `PastLastGame`.
- Signed-to-unsigned conversions go through one small `positive(time::Duration) -> Duration`
  helper flooring at zero; every caller has already established the sign.
- **`config` is not optional and must never be defaulted.** `Schedule::get_game_timing` returns
  `Option`, and a `GameConfig::default()` stand-in would give that game the wrong period lengths —
  so the catch-up would place the engine confidently at a position that does not exist. A game with
  no timing rule is excluded from the court list entirely (Task 4), never guessed at.

**Tests:** `regulation_length_is_both_halves_plus_half_time`, an
`an_empty_schedule_is_past_the_last_game`, and one table-driven
`every_position_in_the_schedule_places_correctly` over a two-game fixture (game 1 at t=0, game 2 at
t=1800, each 600+180+600=1380s long), covering: `-1200` before the first; `0` exactly on kickoff —
**inside** the game, not before it; `100` first half; `700` half time; `1000` second half; `1500`
the break; `2680` second half of the *later* game; `99_999` past the last.

- [ ] Write the tests
- [ ] Run `cargo test -p refbox placement` — expect FAIL (`cannot find place`)
- [ ] Write the implementation
- [ ] Run `cargo test -p refbox placement` — expect PASS
- [ ] Commit: `feat(refbox): calculate the schedule position of a wall-clock moment`

---

## Task 3: The engine placement method

**This is where the review effort belongs.** It is the only genuinely new engine capability: today
the engine reaches a position only by playing through to it. There is no public period setter
(`set_period_and_game_clock_time` is `#[cfg(test)]`), and `set_game_clock_time` (`mod.rs:2351`)
works only on a stopped clock and forces `Stopped`.

**Files:** `mod.rs`, immediately after `apply_next_game_start` (ends `:1409`), so the three ways the
engine adopts a scheduled position sit together.

**Produces:**
```rust
pub fn place_at_schedule_position(
    &mut self,
    now: Instant,
    game: &ScheduledGame,
    period: GamePeriod,
    time_remaining: Duration,
) -> Result<()>;
```

**Design requirements — all seven are load-bearing:**

1. **Two cases, different behaviour.**
   - `game.number == self.game_number`: the catch-up landed inside the game already running. It has
     *not* been passed over. Keep score, penalties, stats and number; change only `current_period`
     and `clock_state`.
   - Otherwise: a fresh game. **Reuse the existing start sequence** rather than re-implementing the
     reset — read `start_game` (`:1517`) and `start_play_now` (`:2270`) and follow the established
     order: populate `next_game`, set the config, call `start_game(now)`, then override
     `current_period` and `clock_state`.
   - `start_game` overwrites `self.config` only when `next_game.timing` is `Some` (`:1537-1540`), so
     populate `next_game` with `timing: None` and set `self.config` from `game.config` *before* the
     call — it will survive. Run `normalize_degenerate_overtime` yourself: with `timing: None`
     `start_game` will not run it for you, and a zero-length overtime crashed the app in the field.
2. **A directly-assigned period skips everything the normal advance does.** Every real period change
   goes through a transition function; a bare `self.current_period = period` does not. Before
   writing this, read `cull_penalties` (`:1064`, called from the transition at `:1783`) and the
   `timeouts_used` handling, and decide **per case** what must be replicated. The same-game case is
   the exposed one: it moves the period under live penalties and a live timeout count. State the
   decision in a comment — "nothing needed here, because X" is a fine answer, silence is not.
3. **Never call `end_game`.** It writes `last_game_info`, which drives the Portal submission
   (`app/mod.rs:2319, 7752, 7817`). A passed-over game must produce no result. Task 6 tests this.
4. **Do not touch `last_game_info`.** A genuinely finished game's result may still be awaiting
   submission; clearing it here would drop a real result.
5. **Do not re-base penalties** the way `set_game_clock_time` does — see Spec Correction 3.
6. **Leave the clock running and say so.** `ClockState::CountingDown { start_time: now,
   time_remaining_at_start: time_remaining }`, and `send_clock_running(true)` if it was not already
   running — otherwise the updater sleeps through the change, the trap `apply_next_game_start`
   documents at `:1400-1404`.
7. **The app will not notice a placed game the way it notices a started one.**
   `handle_game_start` fires only on a `BetweenGames` → play transition (`app/mod.rs:1233-1237`),
   and a placement from one game straight into another never passes through `BetweenGames`. So
   `next_game` is never repopulated and the roster stays on the previous game — meaning the court
   parks at 0:00 when the placed game ends, as though its schedule were finished. **Task 5 must
   arrange the equivalent refresh; do not leave it to the snapshot transition.**

**Tests** (in `mod.rs` `mod test`, from `:3109`). Use the accessors the existing tests use; do not
add public accessors to satisfy a test.
- `placing_inside_the_running_game_keeps_its_score` — same number: period and clock move, score and
  number do not
- `placing_into_a_later_game_starts_it_fresh` — different number: new number, new period, clock set,
  **score back to 0-0**, clock running with the start/stop latch set
- `placing_into_a_later_game_adopts_its_timing_rule` — a `half_play_duration` of 999s on the target
  game reaches `tm.config()`
- `placing_into_a_later_game_leaves_a_next_game_to_follow` — guards requirement 7: after placing,
  the engine must not believe the court's schedule is finished

- [ ] Write the tests
- [ ] Run `cargo test -p refbox place_at_schedule_position` — expect FAIL (no such method)
- [ ] Write the implementation
- [ ] Run `cargo test -p refbox place_at_schedule_position` — expect PASS
- [ ] Run `cargo test -p refbox` — whole engine suite, `golden_traces_match_baseline` **unchanged**
- [ ] Commit: `feat(refbox): place the engine at a schedule position in one step`

---

## Task 4: Give the engine the court's schedule

**Why:** the engine holds only `next_game` — one game ahead. Crossing boundaries needs the whole
court list. The app owns `schedule` (`app/mod.rs:198`) and `current_court` (`:208`), and already
builds court-filtered views with `Schedule::next_game_on_court`
(`uwh-common/src/uwhportal/schedule.rs:565`) and `Schedule::get_game_timing` (`:553`).

**Files:** `mod.rs` (field + setter), `app/mod.rs` (populate it).

**Produces:**
```rust
/// Every game on the selected court, sorted by start time. Distinct from `next_game`,
/// which is only ever one game ahead. Empty in manual mode.
court_schedule: Vec<ScheduledGame>,

pub fn set_court_schedule(&mut self, games: Vec<ScheduledGame>);   // sorts by start_time
pub(crate) fn court_schedule(&self) -> &[ScheduledGame];
```

**Building the list.** Filter `schedule.games.values()` on `game.court == *pool`, and take each
game's config from `schedule.get_game_timing(&game.number)`.

**A game with no timing rule is excluded, not defaulted.** `get_game_timing` returns `Option`.
Substituting `GameConfig::default()` would give that game plausible-looking but wrong period
lengths, and the catch-up would then place the engine confidently at a position that does not
exist — a wrong clock is worse than an uncorrected one. Drop the game from the list and `warn!`
with its number.

**Refresh it wherever the schedule or the court changes** — these are the real write sites, and
they are *not* the `apply_next_game_start` call sites:

| `app/mod.rs` | What happens there |
|---|---|
| `:1747-1748` | court and schedule both set together |
| `:1988-1989` | cleared to manual mode |
| `:2046-2047` | cleared to manual mode |
| `:6642` | a freshly received schedule is stored |

Clearing to manual mode must clear the court list too, or a catch-up after unlinking places against
a schedule the operator has just disconnected from. **Write one helper and call it from all four;
four copies is how one gets missed.**

**Tests:** `the_court_schedule_round_trips`; one asserting an out-of-order input comes back sorted;
one asserting a game whose timing rule is missing is excluded rather than defaulted.

- [ ] Write the tests
- [ ] Run `cargo test -p refbox court_schedule` — expect FAIL
- [ ] Add the field, setter and app wiring
- [ ] Run `cargo test -p refbox && cargo clippy -p refbox --all-targets -- -D warnings`
- [ ] Commit: `feat(refbox): give the engine the selected court's game list`

---

## Task 5: The detection hook, its guards, and manual mode

**Files:** `mod.rs` (new `jump_detector: JumpDetector` field, `observe_time_jump`,
`absorb_lost_time`), `app/mod.rs:9910-9916` (one line).

**Produces:**
```rust
pub(super) fn observe_time_jump(&mut self, now: Instant, wall_now: OffsetDateTime) -> Result<()>;
```

**Order is fixed and behaviour-changing: `observe` runs FIRST, before every guard.** It rebases the
detector on every call, so a tick that is guarded out still keeps the baseline current. Guarding
before observing would let a held clock accumulate an arbitrary "gap" and fire a huge spurious
catch-up the moment the hold ended.

**Guards — a catch-up must NOT happen when:**
- **the game clock is stopped** — use `self.clock_state.is_running()`, **not** `clock_is_running()`.
  `clock_is_running()` reports the *timeout* clock whenever one exists (`mod.rs:208-216`), so it
  answers a different question and would let a catch-up fire mid-timeout;
- **a timeout is active** (`self.timeout_state.is_some()`) — the same operator hold by another name,
  and nothing on the placement path clears a timeout;
- a score-confirmation pause is active (`self.time_pause_confirmation.is_some()`);
- the gap is backwards or below threshold (already handled inside `JumpDetector`).

These are the spec's deliberate choice: the schedule has moved on around a held clock, but taking
the game out of the operator's hands because a screen slept is worse than a stale clock they can see.

**No tick runs while the clock is stopped, so the detector never observes during a hold.**
`next_updater_wake` returns `None` when the clock is not running (`app/mod.rs:9714-9720`), and the
updater then blocks on the start/stop latch with no timer. A machine slept while stopped therefore
resumes with a stale baseline and reports the entire stopped period as lost — re-placing the game
the operator just started by hand. **Reset the detector when the clock resumes** (clear
`jump_detector` in `start_clock`) so the first observation after a hold rebases instead of firing.
This is the failure Task 5's naive stopped-clock test cannot catch, so test it directly:
stop, sleep, start, assert nothing moved.

**Manual mode is checked FIRST, before `place` is consulted.** An empty `court_schedule` and
"past the last game" are different situations that `place` reports identically (Task 2 returns
`PastLastGame` for an empty slice), and treating manual mode as past-the-last would zero a
perfectly good manual clock.

**Then dispatch on `placement::place(&self.court_schedule, wall_now)`:**

| Placement | Action |
|---|---|
| `BeforeFirstGame` / `InBreak` | `current_period = BetweenGames` **and** `clock_state = CountingDown { start_time: now, time_remaining_at_start: until_start }` |
| `InGame { index, period, time_remaining }` | clone `self.court_schedule[index]`, call `place_at_schedule_position` |
| `PastLastGame` | `current_period = BetweenGames`, `Stopped { clock_time: ZERO }`, `send_clock_running(false)` |

**Setting the period is not optional in the break cases.** Leaving `current_period` at, say,
`FirstHalf` while the clock counts down to the next kickoff means the expiry runs `end_first_half`
instead of starting a game — a wrong period with a right-looking clock.

> **The sharpest interaction in this change, and it must be solved before Task 5 is called done.**
> Moving `current_period` to `BetweenGames` makes the app's `apply_snapshot` call
> `handle_game_end(&game_number)` (`app/mod.rs:1233-1235`), because that is how a game ending is
> normally detected. On this path the game did **not** end — it was slept through — and
> `handle_game_end` is a result-publishing path. Letting it fire would publish a result for a
> passed-over game, breaking acceptance criterion 3 in the one place its own tests do not look.
> Conversely a placement into a *different* game never reaches `handle_game_start` (Task 3
> requirement 7), so the refresh that *should* happen does not.
> **Decide explicitly how the app is told a catch-up happened rather than inferring it from the
> period transition**, and write the decision into the plan's Deviations section. A distinct
> `TickKind`, or an explicit call from the placement path, are both reasonable; silently relying on
> `apply_snapshot` is not.

Indexing `court_schedule[index]` is safe — `place` only returns an index into the slice it was
given, and nothing mutates it in between. Say so in a comment.

**Manual mode** (`court_schedule` empty): `absorb_lost_time(lost)` — take the lost time off whatever
is running. `CountingDown`: `time_remaining_at_start.saturating_sub(lost)`; `CountingUp`:
`time_at_start + lost`; `Stopped`: nothing.

> **Judgement call, flagged for review.** If the subtraction reaches zero, `absorb_lost_time` should
> park at `Stopped { ZERO }` rather than let the clock expire into a period change — letting a
> between-games countdown expire would auto-start a game, which the spec forbids in manual mode
> ("takes the lost time off the running clock and stops there; it never starts a game on its own").
> If review prefers the expiry to run, change it and record it in Deviations.

**App wiring — one line, inside the existing `catch_unwind` closure at `app/mod.rs:9910`,** so it
inherits the existing lock, panic guard and failure reporting:

```rust
let mut tm_ = tm.lock();
let now = Instant::now();
tm_.observe_time_jump(now, OffsetDateTime::now_utc())?;   // NEW
let (kind, snapshot) = tm_.updater_tick(now)?;
```

`updater_tick`'s signature stays unchanged deliberately: it has six call sites (`app/mod.rs:9914`,
`zero_probe.rs:135, 328, 381`, `mod.rs:3183, 3233`) and none of the test ones want a wall clock.

**Log the correction.** The spec's "no new UI" ruling makes the log the only record that anything
happened, so every branch above emits one `info!` saying how much was lost and where it landed —
including the guarded-out cases, which are the ones an operator will ask about.

**Tests.** Each drives `observe_time_jump` twice — once to prime the detector, once with the jump.
- `a_sleep_before_the_first_game_corrects_the_countdown` — **the reported fault**: kickoff 146 min
  out, sleep 126 min, assert the countdown reads ~20 min
- `a_catch_up_into_a_break_sits_between_games` — asserts the *period* is `BetweenGames`, not just
  that the clock looks right
- `a_stopped_clock_is_left_alone`, `a_timeout_is_left_alone`, `a_confirmation_pause_is_left_alone`
- `resuming_after_a_stop_does_not_fire_a_catch_up` — stop, sleep two hours, start; nothing moves.
  Fails against a detector that is not reset on resume
- `manual_mode_takes_the_lost_time_off_and_starts_nothing` — no court schedule; clock down by exactly
  the gap, period unchanged
- `no_jump_leaves_every_clock_alone` — table over a below-threshold gap and a backwards gap

- [ ] Write the tests
- [ ] Run `cargo test -p refbox observe_time_jump` — expect FAIL (no such method)
- [ ] Write the implementation and the app wiring
- [ ] Resolve the `handle_game_end` / `handle_game_start` question above and record it in Deviations
- [ ] Run `cargo test -p refbox && cargo clippy -p refbox --all-targets -- -D warnings`; `golden_traces_match_baseline` **unchanged**
- [ ] Commit: `feat(refbox): catch the clocks up after the machine sleeps`

---

## Task 6: Passed-over games publish nothing

Acceptance criterion 3, and the one with real tournament consequences: a phantom result on the
Portal moves points, standings and goal difference. This area already carries known defects — the
offline phantom-game POST and the replayed-game repost loop — so it gets its own task and its own
review.

**Files:** tests only, in `mod.rs` `mod test`. `end_game` is private; drive a game to its end the
way the existing tests do rather than widening its visibility.

**Tests:**
- `a_game_slept_through_records_no_result` — game 1 now with a 5-2 score, game 2 an hour out; sleep
  65 minutes. Assert `last_game_info()` is `None` **and** the score is back to 0-0: the passed-over
  game's score is discarded, not carried into game 2.
- `a_finished_games_pending_result_survives_a_catch_up` — end a game properly so `last_game_info` is
  populated, *then* catch up. Assert the recorded result is byte-identical afterwards. This is the
  test that stops requirement 3 being "fixed" by clearing `last_game_info`.

- [ ] Write the tests
- [ ] Run `cargo test -p refbox slept_through pending_result`. **If they pass immediately**, confirm
      by reading `place_at_schedule_position` that it is because `end_game` is never called on that
      path — not by luck. State which in the commit message.
- [ ] Fix anything they catch. If a result *is* recorded, the cause is a call to `end_game` on the
      placement path — remove it. **Do not** paper over it by clearing `last_game_info`, which would
      also drop legitimately pending results.
- [ ] Run `cargo test -p refbox`
- [ ] Commit: `test(refbox): a game slept through publishes no result`

---

## Task 7: Golden trace and full validation

**Files:** `golden/mod.rs` (one `Action` variant + dispatch arm), `golden/scenarios.rs` (one
scenario), one generated `.trace`.

The golden harness does **not** route through `updater_tick` (see its comment at
`golden/mod.rs:120-130`), so the new action calls the placement method directly. Detection is
covered by Task 5's unit tests.

- [ ] Add `Action::CatchUpToGame(&'static str, GamePeriod, Duration)` beside the existing variants
      (`golden/mod.rs:35`) and its dispatch arm, building a `ScheduledGame` from the scenario's own
      config and calling `place_at_schedule_position`
- [ ] Add scenario `sleep_catchup_across_game_boundary` to `all()` (`scenarios.rs:711`) on
      `reg_config()` (half=20s, halftime=8s): start play, score for black at t=2, then at t=5 catch
      up to game "4" `SecondHalf` with 12s remaining. `run_secs: 45`
- [ ] Generate the baseline: `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline`
- [ ] **Review the diff before trusting it.** `git diff --stat refbox/src/tournament_manager/golden_traces/`
      must show **exactly one new file**. If any *existing* `.trace` changed, STOP — the engine's
      behaviour changed for scenarios unrelated to this feature, which is a regression, not a
      re-bless. Read `golden_traces/README.md`
- [ ] Read the new trace by eye: score returns to `B0/W0` at the catch-up, and the period goes from
      `FirstHalf` straight to `SecondHalf` with **no `HalfTime` line between**
- [ ] Do **not** expect the trace to evidence the game *number*. `render()` deliberately omits game
      numbers (`golden_traces/README.md`, "What is and isn't watched"), so the trace proves the
      period and clock jump only; the number is proven by Task 3's unit test. Do not widen
      `render()` to cover it — that re-blesses every existing trace
- [ ] Run `just check` — fmt, lint, tests, audit
- [ ] Also run `cargo clippy --workspace --all-targets --all-features -- -D warnings`: `just lint`
      is not `--all-targets`, and `just check` is host-only so a Windows-target break is invisible here
- [ ] Commit: `test(refbox): golden trace for a catch-up across a game boundary`

---

## Deviations

Record anything that diverged from this plan during execution, one line each. No standalone
deviation commits.

- (none yet)

---

## Acceptance Criteria

| # | Criterion | Proven by |
|---|---|---|
| 1 | Two-hour gap before game 1 → countdown reads the true time | Task 5 `a_sleep_before_the_first_game_corrects_the_countdown` |
| 2 | Gap into a later game → that game, its number and timing rule, at the true position | Task 3 `..._starts_it_fresh` + `..._adopts_its_timing_rule` (the number and rule); Task 7 trace (the period and clock jump only — golden does not watch game numbers) |
| 3 | No Portal submission for a passed-over game; a game landed inside keeps its score | Task 6 both tests; Task 3 `placing_inside_the_running_game_keeps_its_score` |
| 4 | A backwards wall-clock step changes no clock | Task 1 + Task 5 |
| 5 | A gap below the threshold changes no clock | Task 1 + Task 5 |
| 6 | No schedule → lost time off the clock, no game started | Task 5 `manual_mode_takes_the_lost_time_off_and_starts_nothing` |
| 7 | `just check` clean | Task 7 |
| — | Every catch-up and every guarded-out jump is logged | Task 5 "Log the correction" — with no UI, the log is the only record an operator can be pointed at |
| 8 | **A real lid-shut sleep on a Mac leaves the countdown correct** | **Not provable here — Eric, on hardware.** No Mac in the dev environment and CI cannot suspend a machine. This is the only real proof the feature works. |
