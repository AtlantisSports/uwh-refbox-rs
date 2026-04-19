# 013 — Cold-Restart State Recovery

**Date:** 2026-04-19
**Status:** proposed

## Context

At the 2026-04-18 tournament the refbox cold-started mid-tournament.
The refbox log shows three separate application termination
timestamps on that day, including a mid-game crash between games 3
and 4. After one of those restarts, the operator resumed play from
"game 1" and re-ran earlier games in the schedule instead of
advancing to the game that should have been played next. As a
result, games 10, 11, and 12 were never played inside the refbox —
they were hand-entered via the UWH Portal web UI in a bulk session
later that evening (18:57 – 19:05).

Three facts are important for the design:

1. **The UWH Portal sent the correct schedule.** The log at
   12:49:41 contains the full portal response, including
   `Game { number: "10", start_time: 2026-04-18 13:14:00 -07:00,
   court: "1", ... }`. Every game in that response had
   `court: "1"`, which matches the tournament's single-court setup.
   Data flow from portal to refbox was working correctly.
2. **The bug is not in portal fetching or scheduling.** The bug is
   that the refbox has no memory of *which* game is in progress
   across an app restart. After cold start, the game state resets
   to defaults — game number "1", clock paused, score zero — and
   the operator has no visible cue that, based on wall-clock time,
   the next scheduled game is actually a later one.
3. **The schedule handler does not look at wall-clock time.** On
   receipt of the portal schedule, `Message::RecvSchedule` in
   `refbox/src/app/mod.rs` calls `tm.set_next_game` only when the
   state machine is already in `BetweenGames`. On a fresh start
   that guard is satisfied and the schedule is stored — but
   nothing cross-references the current wall-clock time against
   each game's `start_time` to decide which game is "up now." The
   operator is left facing game 1 by default.

Related items that were flagged as suspects during the same
investigation but ruled out:

- **"End game and apply" on a game-number change** is working as
  designed. The confirmation dialog offers "Keep current game" as
  a sibling option, so choosing "End" is an explicit
  acknowledgement that the current game was started in error.
- **`using_uwhportal` starting `false` at launch** is real, but the
  reporting operator knew about it and the game-config screen
  confirmed the portal was connected.

Both of those flows behave correctly; neither is what caused the
missing games.

Both the portal-submission investigation (ADR 011) and this one came
out of the same 04-18 log. ADR 011 addresses the downstream
submission-layer weakness (fire-and-forget submission with no retry
or health indicator); this ADR addresses the upstream state-loss
weakness. They are distinct problems with distinct remedies.

## Decision

This ADR records the problem and the shape of the intended fix. The
exact dialog wording, persistence format, and prompt sequencing are
left as "to be brainstormed" and resolved during implementation.

On cold start with a valid portal schedule and a current wall-clock
time that falls inside or after a scheduled game's start window, the
refbox must not silently default to game 1. At minimum, it must
surface the mismatch to the operator. Two complementary remedies
are on the table:

### Option A — Time-based next-game prompt on startup

On application start, after the portal schedule has been received:

- Find the game in the schedule whose `start_time` is closest to
  the current wall-clock time on the configured court (within a
  configurable window — see open questions below).
- If the selected game is not "1", present the operator with a
  prompt of the form:
  *"Based on the current time, the next scheduled game is #N
  (teams X vs Y, start_time HH:MM). Start there, or keep game 1?"*
- The default choice is **Start there** — the common case — but
  the operator can decline if they legitimately want to replay or
  back up.

This is a low-risk, self-contained change. It does not persist any
new state to disk. It uses only data the refbox already has.

### Option B — Persist in-progress game state across restarts

On every significant game event (start, score, foul, penalty, clock
change), write a compact snapshot of the `tournament_manager` state
to disk. On startup, if a recent snapshot exists, offer to resume
the game from that snapshot.

- Storage: a JSON file alongside `confy`'s existing config
  location — the same directory pattern as the queue file proposed
  in ADR 011.
- Freshness policy: the snapshot is valid for a configurable
  window (e.g. 4 hours) to avoid resuming yesterday's game.
  Beyond the window, the snapshot is discarded silently and
  Option A takes over.
- Resume prompt: if the snapshot is fresh,
  *"Resume game #N in progress (HH:MM in second half, score 3-2)?"*
  with **Resume** as the default.

This is higher-complexity: it requires a serialisation format for
`tournament_manager` state and careful thought about clock
semantics across a restart gap.

### Recommended sequencing

Option A is the smaller, safer change and solves the specific
04-18 failure mode directly. Option B is more ambitious and solves
additional cases (mid-game crashes, deliberate restarts during
play). Implement A first; B becomes a follow-up if the time-based
prompt proves insufficient in practice.

## What is not changing

- The portal client API is untouched. The existing schedule call
  already returns everything Option A needs.
- The `tournament_manager` state machine shape is untouched by
  Option A. Option B would add a persistence surface but no new
  states.
- The mid-game "change current game" confirmation dialog in
  `app/mod.rs` is untouched. This ADR is about cold start, not
  about voluntarily changing game number while the refbox is
  running.

## Open design questions

- **Window for "next game on this court."** Do we look only
  forward from now, or also backward if the most recent game's
  `start_time` is within a grace period (e.g. 30 minutes)?
  Tournaments often run behind schedule; a `start_time` 15 minutes
  in the past is usually the current game, not the previous one.
- **Interaction with court configuration.** The refbox filters the
  schedule by `game.court == *pool`. If `pool` is unset or
  mismatched, the time-based prompt has nothing to show. Should
  the prompt appear regardless (with all courts), or only when a
  court is configured?
- **Interaction with ADR 011's queue.** If the submission queue
  file has a pending retry for a specific game number, that is a
  stronger signal than wall-clock time for "which game was in
  progress." The two systems should not contradict each other;
  the startup flow needs to consult both.
- **Option B snapshot cadence.** Writing on every user action is
  robust but costs disk I/O. A debounced timer (every 30 s) is
  cheaper but can lose the last few seconds on a crash. To be
  decided during implementation.
- **"Resume or jump" default under Option B.** If a fresh
  snapshot exists *and* the current time matches a later scheduled
  game, which prompt wins? Likely rule: the snapshot takes
  precedence until its game is marked complete, and only then
  does the time-based prompt engage.
- **Sequencing against Select Event / Select Team / portal
  login.** At what point in the cold-start flow does the prompt
  fire? After the schedule is fetched, certainly — but before or
  after the operator has selected court and team colours?

## Consequences

**Becomes easier:**

- After a restart the operator is not silently dropped back to
  game 1. The mismatch between wall-clock time and game number is
  made visible and actionable.
- Scheduled-game skips that happened for non-crash reasons (e.g.
  operator opened the wrong game page and committed it) are also
  caught by the same prompt on the next restart, because the
  prompt is time-based.
- Under Option B, mid-tournament crashes no longer cost an entire
  block of games' worth of data.

**Becomes harder / constrained:**

- Option B introduces a new on-disk state surface. Backup and
  migration tooling will need to consider it, and its schema needs
  versioning from day one.
- The cold-start flow gains a conditional prompt. The Select
  Event / Select Team / View Mode flow sequencing becomes slightly
  more complex.
- Any test or developer workflow that relies on "refbox restarts
  to game 1" will need updating.

**Scope:**

- `refbox` — startup flow changes in `app/mod.rs`; a new
  confirmation page for the time-based prompt (Option A); under
  Option B, a new persistence path in `tournament_manager`.
- `uwh-common` — no change under Option A. Under Option B the
  snapshot type lives in `refbox` (not `uwh-common`), since it is
  `std`-dependent and refbox-internal.
- `overlay`, `schedule-processor`, `wireless-remote`, LED-panel
  crates, `matrix-drawing`, `fonts` — no change.

## References

- `refbox/src/app/mod.rs` — `handle_game_start()`,
  `Message::RecvSchedule`, `apply_snapshot()`. Startup flow and
  the guard that restricts schedule-driven game-pointer updates
  to `current_period() == BetweenGames`.
- `refbox/src/tournament_manager/mod.rs` — `set_game_number`,
  `set_next_game`, `apply_next_game_start`; the state mutators
  either option's fix routes through.
- `uwh-common/src/uwhportal/schedule.rs` — the `Game { number,
  start_time, court, ... }` type whose `start_time` and `court`
  fields the time-based prompt reads.
- ADR 011 — the sibling portal-submission ADR. Shared context
  (same 04-18 log, same investigation) but independent remedies
  and independent implementation.
