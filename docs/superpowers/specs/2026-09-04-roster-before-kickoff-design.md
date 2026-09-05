# Player rosters before kickoff, and closing the post-game window

**Status:** Approved 2026-09-04. Branch `fix/refbox/roster-before-kickoff`, based on
`origin/master` at `486c5692`.

**Crate scope:** `refbox` only. `uwh-common` is read but not modified.

---

## The problem

The player picker (the grid of cap numbers on ADD FOUL / ADD WARNING / ADD PENALTY) is only
ever given a roster at kickoff. `game_rosters` is written in exactly one place —
`handle_game_start` — and nothing rewrites it when a game ends.

Three symptoms follow from that one fault:

1. **Before the first kickoff of a session there is no roster at all.** Every picker falls
   through to the plain 0-9 pad. Confirmed on screen 2026-09-04 on court 1 game 27
   (Brisbane A vs Melbourne Scubadorks), with TRACK CAP NUMBER and TRACK FOULS both on and
   FORCE KEYPAD NUMBERS off: both teams offered the pad before kickoff, and the correct
   12-number and 7-number grids immediately after.
2. **During a break the picker offers the finished game's roster.** Observed the same day: the
   info panel named game 15 (Cairns Ornates / GC A Bullrouts) as current while the picker
   offered game 27's players.
3. **A roster that arrives late is never picked up**, because nothing re-reads the cache
   between kickoffs.

Eric has stated (1) as a requirement: **rosters must show in fouls and warnings before the game
starts, not only after.**

## The ruling this design builds on — not re-opened

**Ruled by Eric, 2026-09-04:** a foul or warning entered in the between-game / next-game state
**always** applies to the game about to start. The engine's changeover is correct; the picker is
the thing that is wrong.

## The second fault, found while tracing this

Nothing recorded in the **post-game window** survives. The window runs from the final whistle
until the engine's changeover, `post_game_duration` later — **120 seconds** by default.

- The game's result is sent to the portal at the whistle, in `handle_game_end`, from a snapshot
  of the stats taken in `end_game`. Nothing added afterwards can reach it.
- `reset()` clears `warnings`, `fouls`, `penalties` and the scores at the changeover, so
  anything added in that window disappears from the screen too.

So a foul, warning or penalty entered in those two minutes reaches neither game. It is shown
briefly and then silently discarded. This is pre-existing on master and affects all three entry
types.

## The rule

> **A fouls/warnings surface is open only when there is a game for the entry to land on, and it
> offers the roster of that game.**

Everything below follows from that sentence. It is deliberately one rule rather than three
patches — the three symptoms above are one fault seen from three angles.

| Moment | An entry would land on | Surface | Roster offered |
|---|---|---|---|
| Before the first kickoff | The first game | Open | The first game's teams |
| During play | The running game | Open | That game's, frozen at kickoff |
| Post-game (whistle → changeover) | **Nothing — discarded** | **Closed** | n/a |
| Next-game (changeover → kickoff) | The upcoming game | Open | The upcoming game's teams |

---

## Design

### 1. Close the entry surfaces during post-game

ADD/EDIT FOULS, ADD/EDIT WARNINGS (`warnings_fouls_summary.rs`) and the PENALTIES button
(`main_view.rs`) are disabled — greyed, in the same way the score buttons are already greyed
across the whole break — while the app is in the post-game window.

The finished game's fouls and warnings stay **visible** on the summary page throughout. Only
adding and editing are closed, because only adding and editing are discarded.

Penalties are included deliberately. A penalty entered in that window is thrown away by exactly
the same mechanism; closing fouls and warnings but not penalties would fix the symptom that was
observed rather than the fault.

**The gate is `current_period == BetweenGames && is_old_game`, and both halves are required.**
`is_old_game` is `!has_reset`, and `has_reset` is set false at `start_game` — so `is_old_game`
is **also true during normal play**. Gating on `is_old_game` alone would disable foul and
warning entry for the entire game. This compiles, passes every existing test, and would be
found only by playing a game.

This goes in one predicate in `refbox`, used by both call sites and unit-tested, rather than
being written out twice.

### 2. Between games, the picker follows the game about to start

When the period is `BetweenGames`, the picker's roster is worked out live from
`snapshot.next_game_number` each time the page is drawn, via the existing `rosters_for_game`.
When it is not, the pinned `game_rosters` is used exactly as today.

**Live, not pinned, during the break** — approved 2026-09-04. A roster that lands mid-break
appears rather than being locked out until the next kickoff. This is what makes symptom 3
disappear for the break case. The alternative, pinning once at the whistle, would cache an
empty answer for a team whose numbers arrive a second later.

**During play nothing changes.** The kickoff pin stays, and with it the guarantee the original
grid design was built on: *"because a game's grid is fixed from kickoff, a number recorded
during that game is always present on that game's grid."* A REFRESH mid-game still cannot move
numbers under the operator's hand.

**Deliberately not using `GameSnapshot::game_number()`.** That helper looks like the right
answer and is not: it returns the *finished* game's number for the whole post-game window
(`BetweenGames && !is_old_game`). Using it would reintroduce the reported bug for the first two
minutes of every break. With change 1 in place no entry is possible there anyway, but the
picker should not depend on that: gating plainly on `BetweenGames` keeps it correct on its own
terms, whatever happens to the closure later.

### 3. Before the first kickoff, this already works out

At startup the engine is constructed with `has_reset: true`, so `is_old_game` is false and the
changeover never fires before the first game. That means the app is in **next-game** state from
launch: the surfaces are open, `next_game_number` is the selected game, and change 2 supplies
its roster. Entries made there are kept and carried into that game.

No extra code is needed for the requirement — it falls out of the rule. That is the point of
writing it as one rule.

---

## Explicitly out of scope

- **The engine's changeover, and which game a break-time entry belongs to.** Eric's ruling,
  untouched.
- **`post_game_duration` stays at 120 seconds.** Shortening it was considered and dropped on
  2026-09-04: the same value decides how long the final score stays on the LED scoreboard and
  the stream overlay, and halving that is a poolside decision, not a side effect of this fix.
- **Score entry during the break.** Already closed; not touched.
- **`docs/backlog/rosters-not-refetched-on-refresh/`.** Change 2 cures that note's complaint
  *during breaks* as a side effect. The startup-restore and mid-game cases are deliberately
  left alone.
- **Whether a penalty should be enterable before kickoff at all.** A rules question, not this
  bug.
- **Late foul reports having nowhere to go.** Closing the post-game window makes an existing
  silent loss visible. If referees do report fouls after the whistle and that needs somewhere
  to land, that is a feature — attributing entries to the finished game and re-sending its
  result — and is its own piece of work.

## Files expected to change

| File | Change |
|---|---|
| `refbox/src/app/view_builders/shared_elements.rs` | The post-game predicate, with its tests. |
| `refbox/src/app/view_builders/warnings_fouls_summary.rs` | Gate the two buttons' `on_press`. |
| `refbox/src/app/view_builders/main_view.rs` | Gate the penalties button's `on_press`. |
| `refbox/src/app/mod.rs` | Pick the picker's roster source; tests for that choice. |

No new translation keys — nothing gains new text, buttons only change state. No new
dependencies. No `uwh-common` change, so the wire format and every other crate are untouched.

## Acceptance criteria

Written as things to observe, not code to inspect.

**The trap:** before kickoff the picker shows the pad *whatever* FORCE KEYPAD NUMBERS says, so a
working fix and an absent roster look identical. Every roster criterion below must therefore be
checked at FORCE KEYPAD **both YES and NO** — four states, not two. (FORCE KEYPAD NUMBERS
reached master on 2026-09-05 in PR #3135 and is present on this branch's base.)

Setup: portal event `events/1889-B` on `api.dev.uwhportal.com`, court 1, game 27 —
Brisbane A (7 cap numbers) vs Melbourne Scubadorks (12).

1. **Before the first kickoff, FORCE KEYPAD = NO:** ADD WARNING → BLACK shows Melbourne's 12
   numbers, WHITE shows Brisbane's 7. *(Today: the pad. This is the requirement.)*
2. **Before the first kickoff, FORCE KEYPAD = YES:** both show the 0-9 pad. *(Proves criterion 1
   is a real roster and not the setting.)*
3. **During play, FORCE KEYPAD = NO:** unchanged from today — the correct grids.
4. **During play, FORCE KEYPAD = YES:** the pad. Unchanged from today.
5. **Post-game:** let a game finish. For 2 minutes ADD/EDIT FOULS, ADD/EDIT WARNINGS and
   PENALTIES are greyed and cannot be opened; the finished game's fouls and warnings are still
   listed on the summary page.
6. **Next-game:** after the changeover — the point where the score on screen resets to 0-0 —
   the three buttons come back live, and the picker offers the **upcoming** game's two teams,
   not the finished game's.
7. **Mid-game REFRESH:** with a game running, REFRESH on the Game Info page does not change the
   numbers on offer.
8. **Portal off:** with the portal not in use, every picker shows the pad, exactly as today.

## Risks

- **The `is_old_game` trap in change 1** is the highest-risk item in this design: the wrong gate
  ships green and disables foul entry for whole games. It needs a test that fails if the period
  half of the gate is dropped.
- **`just check` is host-only.** It will not catch a Windows-only break. Nothing here is
  platform-specific, but that is an assumption, not a guarantee.
- **Criterion 5 costs 2 minutes of real waiting** per pass. TIME EDIT cannot shorten it: the
  changeover fires off the break clock, so winding the clock down ends the very window being
  observed. It *is* the fast way into criterion 6 — wind the break clock down to force the
  changeover rather than waiting it out.

## Verified while writing this spec

Claims that were checked in code rather than assumed, so a reviewer does not have to re-derive
them:

- Removing a button's action renders it visibly greyed — `theme/button.rs` gives every style a
  `Status::Disabled` arm (grey background, `disabled_color()` text), with tests asserting it.
  The button does not stay looking live and inert.
- The score buttons are already gated this way across the whole break (`main_view.rs`), so this
  is an existing pattern in the app rather than a new idea.
- `post_game_duration` has exactly one production use: computing `reset_game_time`. It is not
  adjustable in the app, and the portal does not send it — refbox fills it from its own default
  — so portal-linked games use whatever the default is.
