# Sudden-death deciding goal is never recorded — design

**Status:** approved 2026-08-28 · **Branch:** `fix/refbox/sudden-death-goal-not-committed`
**Blast radius:** game state machine → heavy process per `.claude/rules/plan-execution.md`

A goal that ends sudden death is drawn on the score-confirmation screen but never given to the
game engine. If anything prevents the operator from tapping CORRECT, the goal is silently
discarded, the engine still sees a tie, and the game sits in Sudden Death forever — the next game
never starts.

---

## The defect

Every other goal in the app goes through `TournamentManager::add_score`, which records the goal in
the game's goal list and writes the new score. The three sudden-death score entries do something
no other goal entry does: they compute the new score into a **local variable**, park it in
`AppState::ConfirmScores(..)` for display, and tell the engine only "pause for confirmation".

| Site | Operator action |
|---|---|
| `app/mod.rs` `Message::AddNewScore`, SD branch | Tapped a goal, scorer numbers off |
| `app/mod.rs` `Message::AddScoreComplete`, SD branch | Tapped a goal, entered the cap number |
| `app/mod.rs` `Message::ScoreEditComplete`, non-confirmation SD branch | Hand-edited the score to break the tie |

The score reaches the engine only if the operator taps CORRECT
(`Message::ScoreConfirmation { correct: true }` → `set_scores` + `end_confirm_pause`).

If the window expires first, the background clock task calls `end_confirm_pause` **itself**
(`app/mod.rs` time_updater: `else if tm_.pause_has_ended(now)`), and it has no access to
`self.app_state`, so the held goal is unreachable. `end_confirm_pause`'s `SuddenDeath` arm then
tests `scores.are_not_equal()` against an untouched tie and keeps the period at `SuddenDeath`.
`start_game` never runs, so no game-start signal.

Regression point: commit `a3bb6b72` (2025-03-02) added the background auto-confirm path without
updating the three score handlers, which had held the score locally since 2024-12-18.

## Why it is reachable

Three independent routes, only the first of which the window's length affects:

1. **The window expires — while the game clock is running.** For sudden death the window is `minimum_break / 2`
   (`confirm_pause_duration`, the `GamePeriod::SuddenDeath` arm) — *not* the
   `min(pre-OT break, minimum break) / 2` form, which belongs to the end of regulation. PR #2601
   (`b481ff6d`) refuses **zero** length parameters on the settings screens; it does not impose a
   one-second floor, and the minus button floors at one *microsecond* (`app/mod.rs`
   `Duration::from_micros(1)`). So the window's true floor is half a microsecond.
2. **A dropped touchscreen tap** on CORRECT — the defect fixed in v0.5.0. The goal is lost
   identically, and no window length prevents it.
3. **The score-edit route** loses a hand-edited deciding score the same way.

### The window does not expire at all with the clock stopped — and that is deliberate

Found by code review after the first draft of this spec, which wrongly treated route 1 as a single
case. The clock updater schedules no wake-ups while the clock-running latch is false
(`next_updater_wake` returns `None`, locked by `app::updater_wake_tests::
stopped_clock_has_no_scheduled_wake`), and `pause_for_confirm` does not raise the latch. So a
deciding goal scored while the operator has **stopped** the game clock opens a window that never
resolves itself: it waits for the operator.

**Ruled deliberate by the tournament organiser, 2026-08-28:** *"if the operator stops the clock
that is an intentional action and we should not try to circumvent it."* Stopping the clock halts
the game on purpose; the app ending the game by itself during that stoppage would override the
operator. The window waiting is correct there.

What this fix still changes for that case, and the reason it is not a gap: **the goal is recorded
in full** — score, scorer, clock reading — so nothing is lost while the window waits, where before
it was discarded. Only the self-resolution differs, and only where self-resolution would be wrong.

Two tests lock the ruling so that a later reader who spots the asymmetry with the running-clock
case cannot quietly "fix" it: `test_deciding_goal_with_clock_stopped_is_recorded_and_waits` in the
engine, and golden scenario `sudden_death_score_stopped_clock_waits`, whose two-line trace asserts
both halves — the goal recorded, and `BetweenGames` never reached.

## Rejected outright

**Writing the score before opening the window.** `set_scores` ends the game immediately for an
untied sudden-death score unless a confirm pause is already armed, so the period moves to
`BetweenGames`; `pause_for_confirm` then reaches `GamePeriod::BetweenGames => unreachable!()` in
its `clock_at_pause` match and panics. The panic happens while the game-state lock is held,
poisoning it — the same failure shape as the zero-length-half crash, where the app dies on the
operator's next touch. **The order is the whole fix, and reversing it is worse than the bug.**

**Holding the goal until the window resolves** (engine stores a *pending* goal; both routes apply
it on resolution). Rejected by the tournament organiser: the poolside display and the stream
overlay should update immediately, as they do for any other goal. The confirmation window exists
to confirm the **final score** against an erroneous entry, not to withhold the goal.

**Making the background task defer to the app.** The task would stop calling `end_confirm_pause`
and dispatch a message for the app's handler to resolve. It would re-fire on every tick until the
app caught up, and a missed message would strand the game in the window. It also leaves the goal
living in UI state, so route 2 above stays broken.

## Approach

Treat a sudden-death goal **exactly like any other goal** — record it, log it, show it — and let
the confirmation window decide one thing only: *is the game over?* That is the same job it does at
the end of any other game, where the score is likewise already in the engine.

The ordering rule goes into the engine **once**, as a single operation the three screens call, so
no call site can get it wrong.

## Design

### 1. Two engine operations own the order

`refbox/src/tournament_manager/mod.rs`:

```rust
/// Record the goal that ends sudden death, and open the score-confirmation window.
pub fn add_deciding_goal(&mut self, color: Color, player_num: u8, now: Instant) -> Result<()>

/// Apply an edited score that ends sudden death, and open the window. No goal is logged —
/// an edit is the operator stating a score, not scoring one.
pub fn set_deciding_scores(&mut self, scores: BlackWhiteBundle<u8>, now: Instant) -> Result<()>
```

Both do the same two things in the same order, and the order is documented at the call:

1. `pause_for_confirm(now)?` — arm the window **first**. It is the fallible step, so nothing is
   recorded if it fails. It does not touch `current_period`, so the period is still `SuddenDeath`
   for step 2.
2. Apply the change. `set_scores`'s existing `&& !self.in_score_confirm_pause()` guard now holds,
   so the game is **not** ended here — the window resolves it.

`add_deciding_goal` reads the sudden-death clock **before** step 1, because `pause_for_confirm`
freezes `clock_state` at 10 ms and the goal's entry in the goal list must carry its real reading.
To share the goal-recording lines rather than hand-copy them, `add_score` splits into the existing
public entry point plus a private `add_score_at(color, player_num, clock_time, now)` that both use.

Consequence of sharing: the sudden-death increment becomes `+= 1` (as every other goal already
uses) rather than the screens' `saturating_add`. Unreachable in practice — it needs 255 goals — and
consistency with every other goal is the point of the change.

### 2. The three screens

Each replaces its hand-rolled sequence with one call, and takes the score for the confirmation
screen from the engine (`tm.get_scores()`) rather than its own copy, so screen and engine cannot
disagree:

| Site | Call |
|---|---|
| `Message::AddNewScore` | `tm.add_deciding_goal(color, 0, now)` |
| `Message::AddScoreComplete` | `tm.add_deciding_goal(color, player.try_into().unwrap(), now)` |
| `Message::ScoreEditComplete` | `tm.set_deciding_scores(scores, now)` |

`Message::AddNewScore`'s sudden-death branch additionally generates and applies a snapshot, which
its own non-sudden-death branch and both branches of `Message::AddScoreComplete` already do. That
is what pushes the new score to the LED panel and the overlay promptly.

`TEAM SCORE` needs no handling: the keypad already forces the cap number to 0 when it is selected
(`BoolGameParameter::TeamScore`), so the sudden-death branch uses the identical expression to the
non-sudden-death line beside it.

The split of responsibility is deliberate: each call site keeps the condition deciding *whether*
this change needs confirming at all (`current_period() == SuddenDeath`, plus
`scores.black != scores.white` on the edit site), and the engine operation owns only the *order* in
which the confirmed change is applied. Nothing about which changes get a confirmation window moves.

### 3. Nothing else in the app changes

`Message::ScoreConfirmation { correct: true }` keeps its `set_scores` + `end_confirm_pause`. The
score it passes is now the engine's own post-goal value, so the write is a no-op and the goal is
already logged. INCORRECT → edit → `Message::ScoreEditComplete { is_confirmation: true }` still
overwrites with the operator's corrected score; if that restores the tie, `end_confirm_pause`
keeps the period at `SuddenDeath` and play continues.

### 4. Golden harness, updated in lockstep

`golden/mod.rs` carries an explicit KNOWN COUPLING POINT warning that `apply_action` hand-copies
these handlers. Required updates:

- `Action::ScoreSuddenDeath` → `tm.add_deciding_goal(color, 0, now)`, doc comment rewritten.
- `Action::ConfirmScore(color)` must stop incrementing, which makes it identical to
  `Action::ConfirmGameEnd`. Merge them: use `ConfirmGameEnd` at the existing call sites and delete
  `ConfirmScore`.

## Files to change

| File | Why |
|---|---|
| `refbox/src/tournament_manager/mod.rs` | Split `add_score`; add the two operations; new tests |
| `refbox/src/app/mod.rs` | Three sudden-death sites; snapshot in the `AddNewScore` branch |
| `refbox/src/tournament_manager/golden/mod.rs` | `apply_action` lockstep; merge two actions |
| `refbox/src/tournament_manager/golden/scenarios.rs` | Action rename; new scenario |
| `refbox/src/tournament_manager/golden_traces/*.trace` | 1 new baseline, ~4 updated |

**Not touched:** `uwh-common`, the LED panel wire format, `wireless-remote`, `overlay`. Not the
window's length, the confirmation screen's design, or the `confirm_score` setting.

## Acceptance criteria

Observable by the operator, in a config with sudden death enabled and a short `minimum_break`
(30 s gives a 15 s window, long enough to act in and short enough to wait out):

1. Score a goal in sudden death → poolside display and overlay show the new score immediately;
   the confirmation screen appears with its countdown.
2. Tap CORRECT → the game ends, the between-games break starts, the next game's start signal
   fires on schedule.
3. Score a goal and **wait for the countdown to expire without touching anything** → same outcome
   as 2. *This is the reported bug; today the game stays in Sudden Death.*
4. Tap INCORRECT, restore the tie, confirm → play continues in sudden death.
5. The deciding goal appears in the game's goal list with the scorer's cap number and its real
   sudden-death clock reading.

## How this is verified

- **Engine tests** (new): the game is not ended while the window is open; the window expiring with
  no operator input ends the game and logs the goal with its cap number and the real clock reading
  (not the frozen 10 ms); an operator edit still overrides.
- **Golden trace** (new scenario `sudden_death_score_auto_confirm`): score in sudden death, never
  confirm. Today the trace holds `score=B0/W0` and `period=SuddenDeath` to the end; after the fix
  the goal lands at the tap and the period reaches `BetweenGames` when the window closes. Paired
  with a hard assertion that fails on today's code without needing a saved baseline.
- **Four existing baselines change**, all the same way — the score appears at the goal instead of
  at the confirmation tap: `sudden_death`, `sudden_death_score_clock_stopped`,
  `sudden_death_score_during_ref_timeout`, `sudden_death_score_during_penalty_shot`. These are
  exactly the four scenarios using `Action::ScoreSuddenDeath`. Diffs to be reviewed with the
  human, not silently blessed.
- `just check` (fmt, lint, tests, audit).
- **The human's own manual walkthrough of all five acceptance criteria — mandatory before any PR**
  per this workspace's rules. Claude launches the app and hands over numbered steps.
- `superpowers:requesting-code-review` once at the end, per `.claude/rules/plan-execution.md`.

## Amendment, 2026-09-15 — the window is armed once, and only when it decides something

Found by the code review re-run after the rebase onto current `master`, and confirmed by running
it. Committing the goal at scoring time cost the operation its **idempotence**, which the old
hold-it-in-the-UI code had for free: a second delivery used to recompute the same score from the
engine and redraw the same screen, because the engine score had not moved.

`Message::AddNewScore`'s sudden-death branch has no `app_state` guard, so a touchscreen bounce
delivering it twice before the view rebuilds reached the engine twice. Measured: **3-1 with two
goals recorded, the second stamped 0.01 s into sudden death**, because re-arming the window
recomputes `clock_at_pause` from the frozen 10 ms clock. Only reachable with
`collect_scorer_cap_num` off, since that setting routes the tap to the keypad instead.

**Correction, found by the re-review of this amendment:** an earlier draft said the keypad and
score-edit routes "destructure `app_state` and so reject a repeat". That is wrong. They destructure
it and then `unreachable!()` on the else arm (`AddScoreComplete` app/mod.rs:4708,
`ScoreEditComplete` :4157, `ScoreConfirmation` :6394) — a repeat there **panics** rather than being
rejected. The repeat-message filter (app/mod.rs:3957) only drops a message equal to the immediately
preceding one, and `last_message` is overwritten by every `NewSnapshot`, which the updater emits
about once a second throughout the window — so two taps a second apart are both delivered. This is
a **pre-existing** class, untouched and unaffected by this branch (a sudden-death goal reached
`AppState::ConfirmScores` before the fix too), and out of scope here per `.claude/rules/scope.md`.
Raised separately; do not let this paragraph's presence imply this branch fixed it.

Both operations now guard on the engine state rather than on the call site, per *guard the class,
not the case*:

- `add_deciding_goal` ignores a repeat while a window is already open. Sudden death ends on the
  first goal, so there is no second deciding goal — a repeat is a duplicate delivery, not a goal.
- `set_deciding_scores` arms a window only when the score actually decides the game. A **tie**
  decides nothing, and arming for it froze the sudden-death clock at 10 ms behind a window whose
  expiry changed nothing. The score is still applied — refusing it would silently discard the
  operator's edit.

Locked by `test_repeat_deciding_goal_does_not_score_twice` and
`test_tied_deciding_score_does_not_open_a_window`, both observed failing with their guard removed.

The same review found that `stopped_clock_confirm_window_waits_for_the_operator` **could not fail**
on the rule it names: the golden driver only ticks while the start/stop latch is true, and that
scenario never starts the clock, so the `BetweenGames` assertion held whatever the engine did.
The trace is kept as the readable statement of the rule, its doc comment now says what it does and
does not prove, and the rule is guarded by two checks that can fail —
`app::updater_wake_tests::stopped_clock_has_no_scheduled_wake` and a new tick-past-a-ripe-window
assertion in `test_deciding_goal_with_clock_stopped_is_recorded_and_waits`, observed failing when
`update` was given the power to end a ripe window.

## Known limitations, accepted

- **INCORRECT after a mis-tapped goal leaves the goal in the goal list**, so the list will not add
  up to the final score. This is already true of every score edit in the app — tap a goal for the
  wrong team in the first half today and the same phantom is recorded. It becomes reachable in
  sudden death, where today nothing is logged at all. Reconciling edits against the goal list is a
  separate piece of work.
- **An INCORRECT correction that runs out of time is lost, and the game ends.** If the operator taps
  INCORRECT and is still on the edit screen when the window expires, `Message::AutoConfirmScores`
  returns them to the main page and the game has already ended on the recorded score. Before this
  change sudden death merely stayed in sudden death, so they got another attempt. This is exactly how
  every end-of-regulation confirm window already behaves, so it is consistency rather than a new class
  of defect — and it follows from the ruling that the goal counts when it is scored. But with a 30 s
  `minimum_break` the window is 15 s, which is tight for spotting and fixing a wrong-team tap. Worth
  timing during the walkthrough. Found by code review.
- **The overlay's recent-goal flag behaves inconsistently for a deciding goal.** `pause_for_confirm`
  freezes the clock at 10 ms, so `generate_snapshot`'s staleness test drops `recent_goal` once the
  goal's clock reading exceeds `RECENT_GOAL_TIME` (15 s). A deciding goal in the first 15 seconds of
  sudden death raises the flag; a later one does not. Not a regression — `recent_goal` was never set
  for a deciding goal before — but a partial miss on "treat it exactly like any other goal", and two
  otherwise identical goals will look different on the stream. Found by code review. One case is
  fixed rather than accepted (2026-10-12): when the operator disallows the goal by restoring the
  tie, `end_confirm_pause` clears the flag. The staleness test only works on a clock counting down,
  so an early deciding goal's flag otherwise stayed on the stream for the rest of sudden death.
- **With the clock stopped, the on-screen confirmation countdown freezes and never expires.** The
  ruling above says the window must wait for the operator, and the countdown is drawn from the
  snapshot — but no snapshots are produced while the clock-running latch is false, so the number
  simply sits there. The operator sees a countdown that does not move and, unlike every other
  window they have seen, will not resolve itself. Correct per the ruling, but it looks like the app
  has hung. A visible "waiting for you" state is a usability follow-up, out of scope here. Found by
  code review, 2026-09-15.
- **Killing the app during the window loses the goal.** There is no live-game persistence anywhere
  in the refbox; this does not make it worse.
- **The window is still far too short at its floor** (half a microsecond) for an operator to
  dispute a score. Correctness no longer depends on it, so this is a usability follow-up, out of
  scope here.
- `add_deciding_goal`'s log line reads the frozen clock in its status prefix, a cosmetic difference
  in the log only.
