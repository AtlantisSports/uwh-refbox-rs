# Player rosters before kickoff

**Status:** Approved 2026-09-04; **shipped** — the roster fix is on master (PR #3153, merged
2026-09-05). Branch `fix/refbox/roster-before-kickoff`, based on `origin/master` at `486c5692`.
The post-game closure that was split out of this spec was **ruled closed on 2026-09-04**; see
*Deferred* below.

**Crate scope:** `refbox` only. `uwh-common` is read but not modified.

**Scope changed during execution.** This spec originally also closed the fouls/warnings entry
surfaces during the post-game window. A code review found six real defects in that half, so it was
split out on 2026-09-04 — and then **ruled closed the same day, with no work needed**; see
*Deferred* below. The branch ref that held it, `wip/refbox/post-game-entry-closure`, is gone, but
its commits survive (`e2173939`, `a13c2355`). **What ships here is the roster fix alone.**

---

## The problem

The player picker (the grid of cap numbers on ADD FOUL / ADD WARNING / ADD PENALTY) is only ever
given a roster at kickoff. `game_rosters` is written in exactly one place — `handle_game_start`
— and nothing rewrites it when a game ends.

Two symptoms follow from that one fault:

1. **Before the first kickoff of a session there is no roster at all.** Every picker falls
   through to the plain 0-9 pad. Confirmed on screen 2026-09-04 on court 1 game 27
   (Brisbane A vs Melbourne Scubadorks), with TRACK CAP NUMBER and TRACK FOULS both on and
   FORCE KEYPAD NUMBERS off: both teams offered the pad before kickoff, and the correct
   12-number and 7-number grids immediately after.
2. **During a break the picker offers the finished game's roster.** Observed the same day: the
   info panel named game 15 (Cairns Ornates / GC A Bullrouts) as current while the picker
   offered game 27's players.

Eric has stated (1) as a requirement: **rosters must show in fouls and warnings before the game
starts, not only after.**

A third symptom — a roster that arrives late is never picked up — shares the same root and is
cured *for breaks* as a side effect. Its startup and mid-game cases stay in
`docs/backlog/rosters-not-refetched-on-refresh/`.

## The ruling this design builds on — not re-opened

**Ruled by Eric, 2026-09-04:** a foul or warning entered in the between-game / next-game state
**always** applies to the game about to start. The engine's changeover is correct; the picker is
the thing that is wrong.

## The rule

> **The picker offers the roster of the game an entry made now would land on.**

| Moment | An entry lands on | Roster offered |
|---|---|---|
| Before the first kickoff | The first game | The first game's teams |
| During play | The running game | That game's, frozen at kickoff |
| During a break | The upcoming game | The upcoming game's teams |

---

## Design

### 1. Between games, the picker follows the game about to start

When the period is `BetweenGames`, the picker's roster is worked out live from
`snapshot.next_game_number` each time the page is drawn. When it is not, the pinned
`game_rosters` is used exactly as today.

**Live, not pinned, during the break** — approved 2026-09-04. A roster that lands mid-break
appears rather than being locked out until the next kickoff. Pinning once at the whistle would
instead cache an empty answer for a team whose numbers arrive a second later.

**During play nothing changes.** The kickoff pin stays, and with it the guarantee the original
grid design was built on: *"because a game's grid is fixed from kickoff, a number recorded during
that game is always present on that game's grid."* A REFRESH mid-game still cannot move numbers
under the operator's hand.

**Deliberately not using `GameSnapshot::game_number()`.** That helper looks like the right answer
and is not. It returns `next_game_number` only when `BetweenGames && !is_old_game`; the post-game
window is the *other* half, `BetweenGames && is_old_game`. So for the first two minutes of every
break the helper names the **finished** game, and using it would reintroduce the reported bug for
exactly that window.

The two halves are easy to invert — an earlier draft of this spec and of the code comment both got
the formula the wrong way round while stating the right conclusion. `is_old_game` is `!has_reset`,
and `has_reset` is false throughout normal play, so it is never a standalone test for "the game
has ended".

### 2. Never offer a roster from another court

Game numbers are unique across an event, not per court. When no next game is scheduled — the last
game on a court, or before any game has been selected — the engine synthesises `next_game_number`
by incrementing. That invented number can name a real game being played elsewhere, and the roster
lookup previously had no court check, so the picker would have offered two teams who are not in
the pool with nothing on screen to say so.

The lookup now refuses a game that is not this court's, which guards every caller rather than only
the new one.

A `current_court` of `None` is not treated as a mismatch. **Ruled by Eric, 2026-09-04:** a game
cannot be selected before a court is, so no court-less state has a game selection for a roster to
resolve against. The code agrees — `EditableSettings::uwhportal_incomplete`
(`view_builders/configuration.rs:85-99`) requires `current_court` to be set *and* the selected
game to be on it before a portal setup can be applied. An earlier draft justified the same choice
as "a state that has never been exercised", which was an assertion rather than a check; this is
the checkable reason.

**This makes one reader honest; it does not stop the number being invented.** `RecvSchedule` still
adopts the synthesised number as the engine's next game, and the Game Info page still names that
game and its teams with no court check — so the wrong game is already visible elsewhere. An
earlier draft of this section claimed there was "nothing on screen to say anything was wrong";
that was wrong. Fixing it at the source is separate work.

Found by code review on 2026-09-04, not by design. Before this change the affected states offered
nothing; without the check, this work would have turned "nothing" into "confidently wrong".

### 3. Before the first kickoff, this already works out

At startup the engine is constructed with `has_reset: true`, so the changeover never fires before
the first game. The app is in next-game state from launch: `next_game_number` is the selected game
and design 1 supplies its roster. Entries made there are kept and carried into that game.

No extra code is needed for the requirement — it falls out of the rule.

---

## Explicitly out of scope

- **The engine's changeover, and which game a break-time entry belongs to.** Eric's ruling,
  untouched.
- **`post_game_duration` stays at 120 seconds.** Shortening it was considered and dropped on
  2026-09-04: the same value decides how long the final score stays on the LED scoreboard and the
  stream overlay, and halving that is a poolside decision, not a side effect of this fix.
- **Everything under *Deferred*, below** — which has since been ruled closed, with nothing outstanding.
- **The startup-restore and mid-game halves of `rosters-not-refetched-on-refresh`.**

## Known consequence, accepted

**The grid changes at the whistle.** With a keypad page open when a game ends, the panel switches
from the finished game's roster to the upcoming game's. An entry in progress at that moment is
discarded by the engine either way — see *Deferred* — so freezing the grid would only make a
doomed entry look tidier. It is recorded with the deferred work rather than papered over here.

## Files changed

| File | Change |
|---|---|
| `refbox/src/app/mod.rs` | `picker_roster_game`, the court-aware `rosters_for_scheduled_game`, the view wiring, and tests for both. |

No new translation keys, no new dependencies, no `uwh-common` change — so the wire format and
every other crate are untouched.

## Acceptance criteria

**The trap:** before kickoff the picker shows the pad *whatever* FORCE KEYPAD NUMBERS says, so a
working fix and an absent roster look identical. Every roster criterion is therefore checked at
FORCE KEYPAD **both YES and NO** — four states, not two. (FORCE KEYPAD NUMBERS reached master on
2026-09-05 in PR #3135 and is present on this branch's base.)

Setup: portal event `events/1889-B` on `api.dev.uwhportal.com`, court 1, game 27 — Brisbane A
(7 cap numbers) vs Melbourne Scubadorks (12).

1. **Before the first kickoff, FORCE KEYPAD = NO:** BLACK shows Melbourne's 12, WHITE shows
   Brisbane's 7. *(Today: the pad. This is the requirement.)*
2. **Before the first kickoff, FORCE KEYPAD = YES:** both show the 0-9 pad. *(Proves criterion 1
   is a real roster and not the setting.)*
3. **During play, FORCE KEYPAD = NO:** unchanged from today — the correct grids.
4. **During play, FORCE KEYPAD = YES:** the pad. Unchanged from today.
5. **After the changeover in a break:** the picker offers the **upcoming** game's two teams, not
   the finished game's.
6. **Mid-game REFRESH:** does not change the numbers on offer.
7. **Portal off:** every picker shows the pad, exactly as today.

Criterion 2 in `rosters_for_scheduled_game_tests` covers the other-court case by unit test; it is
not reachable in a walkthrough without a multi-court event and a finished last game.

---

## Deferred: closing the post-game window — **RESOLVED, no work needed**

**Ruled by Eric, 2026-09-04, after this work was built and split out:**

> Once a game ends only the confirm score happens — no fouls or warnings or penalties. A game is
> over when the confirm final score lands. None of the prior game's fouls and warnings are to be
> recorded after the game is done.

**The app already enforces this.** Note this is the *whistle-to-confirmation* window, not the
`BetweenGames && is_old_game` window that §"Deliberately not using `GameSnapshot::game_number()`"
calls post-game; distinguishing the two is the whole point of this section. Between the whistle and
the confirmation, no screen the operator can reach commits a foul, warning or penalty:

- The ordinary route is `Message::ConfirmScores` (`app/mod.rs:6338-6358`). With **CONFIRM SCORE on**
  — the settings row is labelled exactly that — it opens `AppState::ConfirmScores`, whose only
  controls are the two options at `view_builders/confirmation.rs:260-267`,
  `ScoreConfirmation { correct: true }` and `{ correct: false }`; the rest of the page is the clock
  readout and, when the portal indicator is red, an advisory. Answering *no* leads to
  `ScoreEdit { is_confirmation: true }`, the score editor. Neither screen reaches fouls, warnings or
  penalties. With the setting **off** the same route calls `end_confirm_pause` at once and returns
  to `MainPage`, so it passes through in an instant.
- **The setting is not the only way in.** The sudden-death scoring paths (`app/mod.rs:4083`,
  `:4142` — the score editor completing — and `:4686`, each gated on `GamePeriod::SuddenDeath`) and
  a game-ending timeout (`:6469`, reached when `timeout_end_would_end_game` holds, which in
  practice means a rugby penalty shot still counting down) set `AppState::ConfirmScores` **without
  consulting the setting**. So a game decided in sudden death shows the confirm page even with
  CONFIRM SCORE off.
- **The window can also close by itself.** Once `confirm_pause_duration` has elapsed, `updater_tick`
  ends the pause and raises `AutoConfirmScores` (`app/mod.rs:6390`), which drops to `MainPage` with
  no operator action at all.

These routes differ in how the window opens and closes, not in what is reachable inside it — every
one of them lands on the confirm page or on `MainPage`, and none exposes foul, warning or penalty
entry. That is the claim the ruling needs, and it does not rest on the list above being exhaustive.

The confirmation happens while the period is still the one just finished: `pause_for_confirm`
(`tournament_manager/mod.rs:2424`) is reachable from `SecondHalf`, `OvertimeSecondHalf` and
`SuddenDeath`, and marks every other period `unreachable!()`. (The game-ending-timeout route goes
through `end_game_ending_timeout`, `tournament_manager/mod.rs:2242`, which arms the pause directly
and checks no period.)

`end_confirm_pause` (`tournament_manager/mod.rs:2483`) is what moves the period on, and **where it
moves to depends on the score.** Decided — or level with neither overtime nor sudden death allowed,
as in an ordinary round-robin where draws stand — it goes to `BetweenGames`, and the operator's
"game over" and the engine's start-of-break are the same moment. Level with one of them allowed, it
goes to `PreOvertime` or `PreSuddenDeath` instead, so the confirm page can also appear *mid-game*,
before extra time, where it is not a game ending at all.

### Why the built work was wrong, not merely incomplete

The branch gated `BetweenGames && is_old_game` — the opening stretch **of the break**, *after* the
confirmation, running up to `post_game_duration` (120 seconds by default, but configurable and
set per event from the portal schedule) and covering the break entirely when the break is no longer
than that (see finding 3). Under the ruling above that is precisely the window where entry
should stay available, because an entry there belongs to the game about to start. So the branch
closed a window that should be open, while the window it was meant to close was already shut by the
app. **It is inverted, not unfinished. Do not resume it.** The branch *ref* is gone from this repo
and the remote, but its two commits survive — `e2173939` and `a13c2355`, recorded in the companion
plan — so if it were ever wanted, `git branch wip/refbox/post-game-entry-closure a13c2355` restores
it exactly rather than rebuilding it from this description.

Its six review findings are kept below, but they are not all about the same thing. Findings 1, 2
and 6 describe master and are worth reading before any similar change. Findings 4 and 5 describe
behaviour that only the abandoned gate produced, so neither exists on master. Finding 3 is split:
its `reset_game_time` formula is master's, while the closure it describes was the gate's.

### What was actually being chased, and why it was dropped

Anything recorded in the opening stretch of a break — up to `post_game_duration`, or the whole
break when the break is no longer than that — is discarded by the engine's `reset()`. Claude framed that as a
bug — entries that "ought to count" being lost — and proposed making them
survive. **Eric ruled that scenario impossible and unwanted:** nothing of the prior game is to be
recorded once the game is done. The discarding is not a defect to fix.

### The six review findings, kept for their value about the codebase

Three were framed as needing a decision; the ruling above removes that need, so all six are
recorded as observations only. **Where a finding prescribes a fix, read it as how such a change
would have to be made if one were ever wanted — not as work outstanding.** Read them with the
classification above in mind:

1. **A fourth entry surface was missed.** `main_view.rs` shows an **ADD WARNING** button during
   breaks with an unconditional `on_press`. Gating three buttons and missing the one the operator
   reaches for first is the enumeration failure this project has been bitten by before. The
   class-correct fix is to guard where entries are *committed*, not button by button.
2. **Pages already open stay live across a period change.** `apply_snapshot` changes the period but
   never `app_state`. This does **not** apply at the whistle, where the handlers force the page to
   the confirm screen or `MainPage` — it applies at the mid-break changeover, where an overview or
   keypad page keeps every control working and still commits an entry `reset()` discards. Same
   conclusion as 1: guard the commit seam.
3. **A short break swallows the whole window.** `reset_game_time` is
   `break_length.saturating_sub(post_game_duration)`. With a break at or under 120 seconds that is
   zero, the changeover fires only at kickoff, and entry is closed for the *entire* break —
   contradicting the ruling that a break entry belongs to the game about to start. **Settled by the
   ruling above; the `reset_game_time` formula is the part that still describes master.**
4. **Extending a break extends the closure.** Same mechanism: winding the break clock up with TIME
   EDIT keeps the buttons dead far beyond 120 seconds, with nothing on screen explaining why.
   **Settled by the ruling above; a property of the abandoned gate only.**
5. **Greying PENALTIES destroys the penalty display.** On the main screen that button *is* the
   readout — the list is printed on it, and both button styles render `Disabled` against
   `window_background()` with `disabled_color()` text (`white_button` inherits that text colour from
   `gray_button`, and switches its background to `HC_WHITE_DISABLED` in high-contrast mode). Both
   teams' panels go grey-on-grey for two minutes, breaking this design's own principle that the
   finished game's entries stay readable.
   **Settled by the ruling above; a property of the abandoned gate only.**
6. **The walkthrough could not have caught (1).** Any resumed walkthrough must assert ADD WARNING
   explicitly, and the predicate's test should cover `HalfTime`, `PreOvertime`,
   `OvertimeHalfTime` and `PreSuddenDeath` — the break periods where `main_view` offers warning
   entry.

### The deeper question behind all of them

**Superseded by the ruling at the top of this section — recorded for the history only.**

At the time these findings were written, the open question was this: closing the UI is a band-aid
on an engine behaviour, since entries made before the changeover are discarded rather than
attributed, and Eric's ruling that "the engine is right and the picker is the bug" was given before
that discarding was known. The suggestion was that resuming the work should start by asking whether
break entries ought instead to land on the upcoming game.

Eric's 2026-09-04 ruling answered it: nothing of the prior game is recorded once the game is done,
and that scenario is neither possible nor wanted. **This is not an invitation to resume the work.**
