# Open review findings — court-finished branch, 2026-09-04

Two reviews ran against the rebased branch on 2026-09-04. The first found 13 findings, the second
15. Everything traced to this branch's own work, or to today's fixes, was closed — see the plan's
Deviations. What follows is what was **found and deliberately not fixed**, so it is not lost with
the session. None blocks the branch; each is either pre-existing or wants its own scope.

## Recorded in code, wants its own branch

**Portal environments are indistinguishable in the link note.** `NoteSite::Portal` records that a
note belongs to the Portal but not *which* one. A refbox launched with `UWH_PORTAL_URL_OVERRIDE`
pointed at the dev portal writes a note that reads as a production note, so relaunching without the
override restores a dev event id, court and anchor against production — real values, wrong server.
Event ids collide across environments by design, which is why `NoteSite::Custom` carries its whole
address. Closing it means recording the portal base URL too. It changes the note every ordinary
session writes, so it needs its own branch and its own walkthrough. It bites hardest during testing,
which is exactly when the override is in use. Recorded at the variant in `link_session.rs`.

## Pre-existing on this branch, not caused by the rebase or today's fixes

1. **The "don't know yet" states render as END.** `court_schedule_finished()` is derived from a blank
   next-game number alone, so `NeedsPick`, `NothingScheduled` and `Unknown` all draw the banner as
   END over `--:--`, dash the game-info table, and assert the day is over on a court that may have
   games left. The engine can tell these apart (`no_next_game` versus linked-with-nothing-named);
   `GameSnapshot` cannot, because it carries only the blank. Fixing it means a new snapshot field,
   which is the wire format the LED panel, the stream overlay and the overlay bridge all read
   (~30 files, including the `no_std` drawing crate and a wire-contract test). Discussed with Eric
   on 2026-09-04 and deliberately left: too big to attach here, and today's behaviour is the safe
   direction, since nothing auto-starts.

2. **A blank game number reaches the LED panel and overlay.** In the restored / needs-pick finished
   state `GameSnapshot::game_number()` returns `""`, which the bridge publishes and the overlay uses
   to request a game from the portal — a GET for game `""` — while rendering "GAME #". Same root as
   1, and gated behind the same wire-format change.

3. **`anchor_after_game_end`'s "leave the anchor alone" branch is unreachable at its only caller**,
   which already applies the identical predicate in its match guard. Two of the four `anchor_tests`
   assert on inputs the call site cannot produce, so they would keep passing if the real guard broke.
   Same class as the link-note guard closed today: the honest fix is to test the invariant that makes
   the branch unreachable, or to drive the tests through the real call site.

4. **TIME EDIT on a finished court stores a value it will neither show nor run.** The clock is parked,
   so `set_game_clock_time` succeeds but `start_clock` is never reached and would be refused anyway;
   the banner still draws `--:--`. The operator sees their edit vanish with no error. Worth knowing
   before any walkthrough, because timing a walkthrough is exactly what TIME EDIT is used for.

5. **Referee rows change height after a restart into the finished state.** `referee_rows_no_game`
   keys off `snapshot.game_number`, which is the fresh-construction `"0"` after a restart, so the
   schedule lookup misses and the six-row individual layout renders where the live case renders two.
   The comment beside it says the layout is kept so the table does not change height when the day
   ends.

6. **`reset()` seeds `GameStats` with a blank game number** whenever the engine is linked with nothing
   next, so a later game's stats can carry an empty number into the portal payload. `reset_game` also
   leaves the clock at `minimum_break` rather than parked, a second finished state subtly unlike the
   one `end_game` and `set_no_next_game` produce.

## Tidy-ups

7. **Two spellings of the dash in one file.** `game_info_table.rs` now imports the shared `NO_VALUE`
   but still has roughly ten hard-coded `"-"` literals for the same purpose in `referee_layout_rows`
   and its helpers. Changing the placeholder needs both found.

8. **Duplicated test fixtures.** Near-identical `game_at` / `two_court_schedule` builders exist in
   three crates' test modules, differing only in game numbers. Adding a field to `Game` means editing
   four fixtures, and the two schedules can drift apart. A single test-support builder in `uwh-common`
   would serve all three.
