# Player number grid: pick a player instead of typing a number

**Date:** 2026-08-08
**Branch (proposed):** `feat/refbox/player-number-grid` (off `origin/master`)
**Crate:** `refbox` only

## Problem

Every player attribution in refbox — a goal, a penalty, a foul, a warning — is entered by
typing digits on a number pad and reading the result back from a small indicator. The pad
accepts any value from 0 to 99, so a mistyped digit records a player who is not in the game,
and the operator has no way to tell from the screen that anything is wrong. Under poolside
time pressure that is a real source of bad data, and bad data reaches the portal.

The set of players is known in advance. The portal holds each team's roster with cap numbers,
and team sizes are capped by the rules. There is no reason to type.

## Decision

On the four pages that ask *which player*, replace the number pad and its readout with a
3-column grid of that team's actual cap numbers. Where refbox has no usable roster for the
selected team, the page keeps today's number pad unchanged.

Scope is `refbox` only. Nothing about the wire format, the LED panel, the overlay, the
wireless remote, or the portal submission changes.

### Explicitly not doing

- Not changing how a goal, foul, warning or penalty is recorded once a player is chosen.
- Not showing player names anywhere.
- Not persisting rosters to disk (see "Deliberate limitations").

## What the operator sees

### Where it applies

The four pages that ask which player: **goal** (`AddScore`), **penalty**, **foul**,
**warning**. `GameNumber`, `TeamTimeouts` and `PortalLogin` need free digit entry and keep the
number pad exactly as it is.

### The grid

Three columns, top-aligned, occupying the space the pad and its number readout use today. The
readout row is gone; the grid starts at the top of the panel. Row count is fixed by the mode:

| Mode | Max roster | Grid |
|---|---|---|
| `Hockey3V3` | 6 | 3 × 2 |
| `Hockey6V6` | 12 | 3 × 4 |
| `Rugby` (UWR) | 15 | 3 × 5 |
| `BeepTest` | — | no player entry exists |

Cells are filled with the team's cap numbers in ascending order, left to right then top to
bottom. Cells left over after the roster runs out are greyed and not tappable.

```
  3v3 — 6 cells        6v6 — 12 cells       UWR — 15 cells
  ┌───┬───┬───┐        ┌───┬───┬───┐        ┌───┬───┬───┐
  │ 1 │ 2 │ 3 │        │ 1 │ 2 │ 3 │        │ 1 │ 2 │ 3 │
  ├───┼───┼───┤        ├───┼───┼───┤        ├───┼───┼───┤
  │ 4 │ 5 │ ░ │        │ 4 │ 5 │ 7 │        │ 4 │ 5 │ 7 │
  └───┴───┴───┘        ├───┼───┼───┤        ├───┼───┼───┤
                       │ 8 │ 9 │11 │        │ 8 │ 9 │11 │
   5 numbered          ├───┼───┼───┤        ├───┼───┼───┤
   players, 1 cell     │12 │15 │ ░ │        │12 │14 │15 │
   left over           └───┴───┴───┘        ├───┼───┼───┤
                                            │16 │ ░ │ ░ │
                        ░ = leftover,       └───┴───┴───┘
                            greyed, not
                            tappable
```

The grid never grows beyond the mode's size: the portal restricts team size, so a roster
cannot exceed it. Note the limit is on the *count* of players, not on the numbers themselves —
a 13-player UWR roster may legitimately include cap number 16, as in the example above.

### Button size

Buttons are square, one size for all modes, chosen so the worst case (3 × 5) fits with margin.

The keypad pages render a time banner (`MIN_BUTTON_SIZE`, 89px), the panel, and the timeout
ribbon (89px). In the 691px default window that leaves roughly 481px of usable panel height
after container padding. Five rows at the standard 89px come to 477px — it fits by four
pixels, which is too fine a margin to depend on across DPI and text scaling. **Target ~80px
square**, confirmed by measurement during implementation.

### Using it

- Tap a number to select it; the cell highlights.
- Tap the selected number again to clear the selection.
- **DONE** commits and **CANCEL** abandons, exactly as today. The grid replaces the typing,
  not the flow.
- The existing DONE gating is unchanged: fouls and warnings still require a player number
  unless they are team/equal entries; penalties require a player number.

All four pages also open in edit mode on an existing entry. There the stored player number is
preselected — its cell is highlighted on arrival, the same way the pad arrives preloaded
today. See "Deliberate limitations" for the case where the stored number is not on the grid.

### Per team

The panel follows whichever team button is currently lit. Toggling teams switches the panel to
that team's grid — or to the number pad if that team has no usable roster. One of each on the
same page is normal and expected.

Switching teams **clears the number already selected**: #7 on one roster is not #7 on the
other.

With no team selected at all — an "equal" foul (`color == None`) or a team warning
(`team_warning == true`) — the panel greys out, as the pad does today.

### Goals with no scorer

Unchanged. Nothing selected means the goal is recorded to the team (player number 0). No new
button; `AddScoreComplete` already accepts 0.

### When the number pad appears instead

Any of the following, evaluated for the **selected team**:

- the Using-UWH-Portal setting is off;
- the game slot has no portal team assigned (a placeholder such as "winner of A");
- no roster fetch has ever succeeded for that team;
- the roster came back with nobody holding a cap number.

In all of these the page looks and behaves precisely as it does today.

## Where the numbers come from

Refbox keeps a session roster cache: portal team ID → that team's cap numbers. Roster entries
with no cap number are skipped — there is nothing to tap.

**Bulk load, when the schedule arrives.** Refbox already fetches the event schedule and team
list when an event is linked or REFRESH is pressed (`request_schedule`,
`refbox/src/app/mod.rs:752`). At the same point it pulls a roster for every team appearing in
that schedule — one call per team, once, while there is time and network. This covers the
first game of the day.

**Targeted refresh during the break.** Refbox already determines the next game the moment a
game starts (`handle_game_start`, `refbox/src/app/mod.rs:842`). That is where the two rosters
for the *upcoming* game are re-pulled, giving the fetch the whole break to land rather than
the instant of kickoff. Success overwrites the cached copy; failure leaves the cached copy
untouched. The same refresh fires when the operator selects a different next game by hand.

**Kickoff takes a copy.** When a game starts it takes its own copy of both rosters from the
cache. Nothing changes it for the rest of that game — a REFRESH mid-game re-pulls the event
but cannot move the grid under the operator's hand. The fresh copy is adopted at the next
kickoff.

This ordering is what makes the design safe: because a game's grid is fixed from kickoff, a
number recorded during that game is always present on that game's grid.

### The portal call

`UwhPortalClient::get_team_roster(team_id)` already exists and is unit-tested
(`uwh-common/src/uwhportal/mod.rs:671`). It calls `/api/admin/get-event-team`, which despite
the route is marked `[AllowAnonymous]` in the portal
(`api/Controllers/AdminController.Events.cs:173`) — refbox needs no admin login and no new
permission.

A bulk endpoint `get-event-teams` exists and returns every team's roster in a single call, but
its response omits team IDs, so rosters could only be matched to games by team name.
Rejected: one small call per team keyed by ID is unambiguous and reuses tested code.

## Deliberate limitations

- **The cache is in memory only.** After a refbox restart with no network, every team falls
  back to the number pad until a fetch succeeds. Persisting rosters to disk is a reasonable
  follow-up and is deliberately not in this work.
- **Nothing is stored in the game.** Cap numbers are recorded exactly as today — a plain
  number on the goal, foul, warning or penalty.
- **Safety net for off-grid numbers.** If an entry being edited holds a number the game's grid
  does not contain, that edit shows the number pad with the value loaded, so nothing already
  recorded can become invisible. The ordering above should make this unreachable; it is kept
  because it costs almost nothing and removes the need to prove impossibility.

## Forward compatibility

A planned future feature lets a coach mark which rostered players are actually available for a
given game (a 12-player roster may field only 10). This design already carries "the numbers
this team may use *in this game*" as a per-game copy taken at kickoff, and the grid packs
whatever it is given — 10 of 12 simply fills ten cells and greys two, with no layout change.
Availability would filter between the fetch and the kickoff copy. Not in scope here.

## Implementation sketch

Everything is inside `refbox`. `uwh-common` is untouched.

| File | Change |
|---|---|
| `refbox/src/app/view_builders/keypad_pages/mod.rs` | Selects the left-hand panel — grid or pad — from the selected team's numbers. Today's inline pad moves into its own function, otherwise unchanged. |
| `refbox/src/app/view_builders/keypad_pages/player_grid.rs` *(new)* | Builds the grid: packing, leftover greying, selection highlight. |
| `refbox/src/app/message.rs` | One new action, "player number tapped", mirroring `SetTeamTimeoutCount`, which already sets a value directly rather than digit by digit. |
| `refbox/src/app/mod.rs` | Roster cache, the two fetch points, the kickoff copy, and handling the new action. |
| `refbox/src/app/theme/` | Reuses existing `blue_button` / `blue_selected_button`. Leftover cells are buttons with no `on_press`, which already renders as disabled. |

No new translation keys — the grid is numbers only. No new dependencies.

The core is one pure function: given a team's cap numbers and the mode, produce the grid's
rows. Every visible behaviour agreed above — packing order, leftover greying, grid size per
mode, "no usable numbers" — is decided there.

## Acceptance criteria

Automated:

- The grid-building function is unit-tested for: packing order; a full roster with no
  leftovers; a short roster with leftovers; gaps in cap numbers; unnumbered players skipped;
  an empty roster reported as "no usable numbers"; grid size per mode.
- Existing `foul_add_can_commit` / `warning_add_can_commit` / `penalty_edit_can_commit` tests
  still pass unchanged — the commit rules are not being modified.
- `just check` clean.

Observable by the human, on a linked event (the San Diego Beach Bash sandbox, `events/177-B`):

1. Open a foul on a linked team: a grid of that team's numbers, no keypad, no readout.
2. Toggle to a team without a portal link on the same page: today's number pad appears for
   that team only, and the number already selected is cleared.
3. Tap a number, then tap it again: selection clears; on the goal page pressing DONE then
   records a team goal, as it does today.
4. Record a foul from the grid and confirm the penalty/foul list shows that number.
5. Switch to 3v3 and to UWR: 2-row and 5-row grids, same button size, no clipping against the
   timeout ribbon.
6. Pull the network, start the next game, and confirm the previously cached grid is still
   shown rather than falling back to the pad.

## Open items for implementation

- Measure the panel height on the Pi's actual screen and fix the button size; ~80px is the
  starting target, not a measured result.
- The targeted refresh must fire from every path that establishes the next game, not only
  `handle_game_start`. In `refbox/src/app/mod.rs` those are: `handle_game_start` (869),
  `apply_game_options` (1188 and 1243), `apply_game_confirmation` (1337), and the
  `RecvSchedule` arm that selects the next game by number (4206).
