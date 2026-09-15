# Backlog: a single-period game leaves the operator's page open at the whistle

**Status:** found 2026-09-15 while fact-checking the post-game closure section for PR #3433.
**Traced in source and verified line by line; NOT reproduced by running the app.** Treat the
real-world reachability as unconfirmed until a walkthrough or a test shows it.
**Not caused by any current branch.** This is pre-existing engine behaviour.

**Read the severity note before acting on this.** A first draft of this note claimed the app records
a foul against a finished game. It does not. The real defect is smaller and is described below.

## What happens, in plain English

When an ordinary game ends, the app always moves the operator off whatever page they had open — to
the score-confirmation screen, or straight to the main screen when CONFIRM SCORE is off. Either way
the keypad they may have been part-way through goes away.

**On one path it does not.** A single-period game that is already decided when time runs out — or
that is level with neither overtime nor sudden death allowed — ends with no confirmation step at
all, and nothing moves the operator's page. A keypad left open at the final whistle stays open and
still works.

## Severity: this is a wart, not a data fault

An entry made on that still-open keypad **is discarded, not recorded against the finished game.**
By the time the whistle has gone, `end_game` has already copied the game's penalties and fouls into
`current_game_stats` and frozen `last_game_info` (`refbox/src/tournament_manager/mod.rs:1412-1435`),
and the portal upload is built from that frozen copy. A foul added afterwards goes only into
`self.fouls`, stamped `BetweenGames`, and `reset()` (`mod.rs:520-528`) clears it at the changeover.

So this does **not** breach Eric's 2026-09-04 ruling that none of a finished game's fouls or
warnings may be recorded afterwards — nothing is recorded. It is the same discarding Eric already
ruled on and expressly declined to treat as a defect.

What is actually wrong is narrower: on this one path the operator is not moved off their page, and
an entry they make there vanishes with no signal that it went nowhere. Every other ending moves
them; this one does not.

## Which games are affected

Only single-period games (`config.single_half`), and only when the game genuinely ends at time-up:

- decided score at time-up, **or**
- level score with neither overtime nor sudden death allowed.

A single-period game that is **level** with overtime or sudden death allowed is not affected — it
goes on to `PreOvertime`/`PreSuddenDeath`, reaches `OvertimeSecondHalf` or `SuddenDeath`, and
confirms normally from there.

## How reachable is it today

**No current portal event is single-period.** The field's own doc comment says so
(`uwh-common/src/uwhportal/schedule.rs:253-258`): the flag defaults off precisely so that rules from
an older Portal read as two halves, "which is correct: no current event is a single-period game."

The conversion at `schedule.rs:334` does set `config.single_half` from `single_period`, so a portal
event *could* produce one in future. Today the live route is the local parameter editor
(`refbox/src/app/mod.rs:5583-5599`).

This matters for sizing: it is not currently hitting tournaments through the portal.

## The mechanism

1. `check_time_remaining` (`refbox/src/tournament_manager/mod.rs:1587-1600`) reports a game endable
   only when the period is `SecondHalf` or `OvertimeSecondHalf`. A single-period game plays in
   `FirstHalf`, so that branch of `could_end_game` is false throughout it. (`could_end_game` also has
   a `SuddenDeath` branch that does not consult `check_time_remaining` — irrelevant here, since this
   path never reaches sudden death.)
2. `updater_tick` therefore takes the ordinary `update()` branch, not the confirm branch.
3. `end_first_half` (`mod.rs:1895-1899`) sees `config.single_half` and calls `end_game` **directly**
   under the two conditions listed above.
4. So `pause_for_confirm` is never armed and no `Message::ConfirmScores` is raised.
5. The tick is an ordinary `NewSnapshot`. `apply_snapshot` (`refbox/src/app/mod.rs:1231`) contains
   no assignment to `app_state` — verified, zero in the whole function — so nothing moves the
   operator off their page.

## What this is NOT

It is **not** the work that was on `wip/refbox/post-game-entry-closure`, which was abandoned as
inverted: that gated the opening stretch of the *break*, after the confirmation, where entry should
stay available because it belongs to the game about to start. This is on the other side of the
confirmation.

That branch's ref no longer exists in this repo or on the remote. Its two commits survive but are
unreachable from every ref, so they have a `git gc --prune` shelf life: `git branch
wip/refbox/post-game-entry-closure a13c2355` restores it exactly, for as long as they last.

## Where a fix would belong

In the engine's game-ending path — a single-period game should reach the same seam a two-period game
does, so the operator is moved off their page like every other ending.

That is a change to the tournament manager's state machine, which `.claude/rules/plan-execution.md`
lists as a high-blast-radius trigger for the **heavy process**: per-task verification, per-task code
review and strict deviation tracking.

It should also be sized against the in-flight single-period flag work rather than bolted on, since
both touch how a single-period game is configured and ended.

## How to confirm it for real

1. In the app's own settings, configure a single-period game whose score will not be level at
   time-up. (Not from a portal event — no current event is single-period.)
2. Wind the clock down with TIME EDIT until the period is nearly over.
3. Open ADD FOUL or ADD WARNING and part-enter one, leaving the keypad on screen.
4. Let the clock run out.
5. **Expected if this is real:** no confirmation screen appears, and the keypad stays on screen and
   still accepts input — where a two-period game would have replaced it. Anything committed there
   should then be absent from the game's record, which is the discarding described above.
