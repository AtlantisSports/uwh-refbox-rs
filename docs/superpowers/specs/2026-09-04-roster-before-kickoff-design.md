# Player rosters before kickoff

**Status:** Approved 2026-09-04. Branch `fix/refbox/roster-before-kickoff`, based on
`origin/master` at `486c5692`.

**Crate scope:** `refbox` only. `uwh-common` is read but not modified.

**Scope changed during execution.** This spec originally also closed the fouls/warnings entry
surfaces during the post-game window. A code review found six real defects in that half, three of
them needing decisions rather than fixes, so it was split out on 2026-09-04. It is preserved on
`wip/refbox/post-game-entry-closure` and written up under *Deferred* below. **What ships here is
the roster fix alone.**

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
the new one. A `current_court` of `None` is not treated as a mismatch, so existing callers keep
today's behaviour rather than losing their grid in a state that has never been exercised.

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
- **Everything under *Deferred*, below.**
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

## Deferred: closing the post-game window

Preserved on **`wip/refbox/post-game-entry-closure`** (`is_post_game` plus the gating of EDIT
FOULS, EDIT WARNINGS and PENALTIES). Approved as a design on 2026-09-04, then split out the same
day when review found it incomplete. **Do not resume it without reading this section.**

### Why it exists

Nothing recorded in the post-game window survives. The result is sent to the portal at the whistle
in `handle_game_end`, from the stats snapshot `end_game` took; and `reset()` clears the warnings,
fouls, penalties and scores at the changeover `post_game_duration` later. A foul, warning or
penalty entered in those two minutes reaches neither game — shown briefly, then silently
discarded. Pre-existing on master, affecting all three entry types.

### What was built, and the trap it already avoids

`is_post_game(snapshot)` is `current_period == BetweenGames && is_old_game`. **Both halves are
required.** `is_old_game` is `!has_reset`, and `has_reset` is set false at `start_game`, so
`is_old_game` is *also true throughout normal play*; testing it alone disables foul and warning
entry for entire games while compiling cleanly. The committed test asserts the during-play cases
and has been watched failing on `FirstHalf` with the period check removed.

### Six findings that must be answered first

Three need a decision from Eric, not a patch:

1. **A fourth entry surface was missed.** `main_view.rs` shows an **ADD WARNING** button during
   breaks with an unconditional `on_press`. Gating three buttons and missing the one the operator
   reaches for first is the enumeration failure this project has been bitten by before. The
   class-correct fix is to guard where entries are *committed*, not button by button.
2. **Pages already open stay live.** `apply_snapshot` changes the period but never `app_state`, so
   an overview or keypad page open at the whistle keeps every control working and still commits a
   discarded entry. Same conclusion as 1: guard the commit seam.
3. **A short break swallows the whole window.** `reset_game_time` is
   `break_length.saturating_sub(post_game_duration)`. With a break at or under 120 seconds that is
   zero, the changeover fires only at kickoff, and entry is closed for the *entire* break —
   contradicting the ruling that a break entry belongs to the game about to start. **Decision
   needed.**
4. **Extending a break extends the closure.** Same mechanism: winding the break clock up with TIME
   EDIT keeps the buttons dead far beyond 120 seconds, with nothing on screen explaining why.
   **Decision needed.**
5. **Greying PENALTIES destroys the penalty display.** On the main screen that button *is* the
   readout — the list is printed on it, and `black_button`/`white_button` render `Disabled` as
   `window_background()` with `disabled_color()` text. Both teams' panels go grey-on-grey for two
   minutes, breaking this design's own principle that the finished game's entries stay readable.
   **Decision needed.**
6. **The walkthrough could not have caught (1).** Any resumed walkthrough must assert ADD WARNING
   explicitly, and the predicate's test should cover `HalfTime`, `PreOvertime`,
   `OvertimeHalfTime` and `PreSuddenDeath` — the break periods where `main_view` offers warning
   entry.

### The deeper question behind all of them

Closing the UI is a band-aid on an engine behaviour: entries made before the changeover are
discarded rather than attributed. Eric's ruling that "the engine is right and the picker is the
bug" was given before that discarding was known. Resuming this work should start by asking whether
the right fix is to make break entries actually land on the upcoming game, rather than to close
the door on them.
