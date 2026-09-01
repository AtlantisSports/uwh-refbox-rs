# Design: what the refbox does when a court's schedule runs out

**Date:** 2026-08-17
**Status:** approved in conversation; implementation not started
**Companion:** `2026-08-17-court-finished-behaviour-decisions.md` (the ruling-by-ruling record,
including the options rejected and why)

---

## Why this exists

Four walkthrough scenarios on `fix/uwh-common/no-next-game-on-court` produced three defects. All
three had the same shape: **the refbox must decide "is this court finished?" from incomplete
information, and each code path filled the gap differently.**

| Found | Failure | Gap filled with |
|---|---|---|
| Restart on a finished court | Replayed game 1 and re-posted a 0–0 over the real result | arithmetic `0 + 1` from a stale cached snapshot |
| Restart with no network | Invented game 1, played it unattended, queued a 0–0, delivered it on reconnect | arithmetic `0 + 1`, no schedule to contradict it |
| Restart then REFRESH | Abandoned the finished state and re-adopted game 1 | a single-use flag, already consumed, falling through to "offer the earliest game" |

Patching a fourth path would move the guess, not remove it. This design removes the need to guess.

Evidence: `docs/superpowers/plans/2026-08-16-no-next-game-on-court-finish.md` and
`docs/backlog/court-finished-panel-state/`.

---

## The five rules

1. **The refbox never invents a game.** A game comes from the schedule or from the operator — never
   from arithmetic, a cached value, or a fallback.
   *Boundary:* this binds in portal/custom mode only. In manual mode there is no schedule to
   contradict, so sequential numbering **is** the specification. Manual game numbers are always
   sequential integers; portal/custom numbers may be alphanumeric labels and are treated as opaque.
2. **Remember facts, not conclusions.** Persist which game was last *played*. Derive "is this court
   finished?" fresh from the schedule every time. A fact stays true; a conclusion goes stale.
3. **No history means ask.** A court the refbox holds no record for always requires an operator
   pick. There is no automatic first choice.
4. **The schedule only moves when the operator asks.** Not on a timer, not on reconnection, not
   while a break is counting down.
5. **The operator outranks the schedule** — within their own court. An explicit pick survives
   refreshes until that game is played or they change it.

---

## The model

### What is persisted

The link note (`portal_link.json`, local, versioned) carries, in addition to today's event/court/mode:

- **`last_played`** — the game most recently played *to a recorded result* on this court. The anchor
  for rule 2. Absent when the refbox has no history for this court.
- **`current_game`** — the game the operator is on right now (in progress, or the confirmed upcoming
  game). Lets the refbox show the right thing instantly, including with no network (rule 1 forbids
  reconstructing it by arithmetic). **Absent whenever nothing is next** — a finished court records
  `last_played` and no `current_game`, which is what makes an offline restart show the finished
  state rather than a phantom.

Both are per-event and per-court. Switching either clears both.

### How "what is next" is decided

In order:

1. An explicit operator pick, if one is outstanding → that game. *(rule 5)*
2. Otherwise, if a schedule is available and `last_played` is known → the next game on **this court**
   whose start time is after `last_played`'s. None → **court finished**.
3. Otherwise, if a schedule is available and `last_played` is absent → **ask the operator**.
   *(rule 3)*
4. Otherwise (no usable schedule) → show `current_game` if present, else show nothing. Never
   compute. *(rule 1)*

`last_played` advances only when a game **ends with a recorded result**. Abandoned and interrupted
games do not advance it.

### The three "nothing is next" states

Internally distinct; **displayed identically** (no upcoming game, clock stopped, START NOW greyed
and inert):

- **Court finished** — schedule read, nothing after `last_played`.
- **Nothing scheduled** — schedule read, this court has no games at all.
- **Unknown** — no usable schedule (unreachable, or unparseable).

They are kept apart internally so an empty or unknown court is never mistaken for a completed one —
the conflation that caused the original defects.

---

## Behaviour

### A game ends

| Situation | Behaviour |
|---|---|
| Normal game ends, more follow | Unchanged. Result posted, break counts down, next game **auto-starts** at zero. Auto-start is not the hazard; auto-starting the *wrong* game is, and rule 1 removes that. |
| Last game on this court ends | Clock stops dead. `END` over `--:--`; middle block, settings and referee rows dashed; finished score stays on the tiles indefinitely; result posted normally; START NOW greyed and inert. |
| Last game of the tournament ends | No different from any finished court. The refbox knows only its own court. |
| Game abandoned (no result) | `last_played` does **not** advance — the same game is offered again. Skipping it is unrecoverable and surfaces only at reconciliation. |
| Last game goes to overtime / sudden death | **Its own breaks behave completely normally** — whistle, countdown beeps, buzzer, working START NOW. "Finished" describes the schedule *after* this game. The court is flagged finished from the moment the last game *starts*, so this rule is load-bearing, and the last game is often a final. |

### Startup

| Situation | Behaviour |
|---|---|
| No event selected | Manual mode, unchanged: sequential numbering, auto-start, nothing fetched, no note kept. |
| Mid-tournament, between games | Show `current_game` from the note **immediately**, then confirm or correct from the schedule. Removes the observed `1 → 4 → 1` flicker: the remembered game replaces the guess rather than racing it. |
| Mid-tournament, during a game | Return to **the same game**, ready to start again; live clock/score not restored. **Guarantee: never advance past it** — skipping loses the result entirely. |
| After this court finished | Anchor is `last_played`; search finds nothing after it; finished state. Correct on every subsequent refresh, because nothing is consumed. |
| After the tournament is over | As above. Existing **five-day** restore window retained; dormant beyond it. |
| Court with no history (fresh morning **or** replacement box) | **Ask the operator.** The refbox cannot tell these apart, and every automatic answer risks a played game. |
| Court with no games at all | "Nothing scheduled" — displayed as finished, distinct internally. |

### The schedule changes

| Situation | Behaviour |
|---|---|
| Game added to a finished court | **Only on REFRESH.** A finished court stays finished until the operator asks. |
| Game added/rescheduled mid-break | **Only on REFRESH.** A running countdown never changes what it is counting toward — the game named on screen is the game that will start. |
| Game removed / moved away | On REFRESH, re-run the normal search and show whatever is genuinely next, or the finished state. No special case, no warning. |
| Court closed, games moved elsewhere | Finished state. **The refbox never re-points itself at another court** — that is the original damage class. |
| **Any schedule change during play** | **Hard guarantee: never disturbs a game in progress.** Clock, score, penalties and timeouts are untouched until it ends. |

### Connectivity

| Situation | Behaviour |
|---|---|
| Portal unreachable at startup | Run from the note: show `current_game` and run it normally, or show the finished state and start nothing. Retry quietly. **Never fall back to arithmetic** — this is what kills the phantom. |
| Portal lost mid-day | Nothing changes. The schedule is already held; results queue on disk; the existing indicator shows the state. |
| Portal returns | **Send queued results automatically; leave the game state alone until REFRESH.** Sending is one-way and cannot surprise the operator; changing the next game can. |
| Schedule unparseable | Identical to "unreachable". One behaviour, one path, one thing to test. |

### The operator acts

| Situation | Behaviour |
|---|---|
| Picks a different game | **The pick wins** over the derived answer and survives refreshes, until that game is played or they change it. |
| Picks a game listed on another court | **Not offered.** The picker shows only this court's games. *Consequence:* if a game genuinely moves here late, the routes are correcting the portal schedule, or manual mode. |
| Changes court mid-day | Clear the anchor; **ask** for a game. No history exists for the new court. |
| Switches to a different event | Clear anchor, current game and court; require re-selection. **Game numbers are per-event**, so a carried-over anchor would point at a real but wrong game and look entirely plausible. |
| Switches the portal off | Finished state clears immediately, note deleted, normal break counting, START NOW live, **numbering restarts at 1**. Manual mode is a clean slate; nothing is sent, so a second "game 1" collides with nothing. |

---

## What this deletes

The design is mostly subtraction, and the deletions are the point:

- **The single-use finished flag** (`pending_restore_court_finished` + its `std::mem::take`).
  Superfluous once the anchor is persisted — and its one-shot nature *is* scenario 4's Critical.
- **The "offer the earliest game on this court" branch** (`anchor_num == "0"` in
  `next_game_from_schedule`). Direct cause of that Critical, and rule 3 leaves it no legitimate
  caller. **Delete it; do not guard it.**
- **The arithmetic fallback in portal mode.** `next_game_number()`'s `game_number + 1` must not be
  reachable while linked to a portal.
- **The parse-to-`"1"` default** (`tournament_manager/mod.rs:219-228`). Manual numbers are always
  integers, so a parse failure is a bug — make it an error, not a silent default.

---

## Assumptions

- One game per court per start-time slot; anything else is an upstream data error, not a case to
  handle.
- Manual-mode game numbers are always sequential integers.
- Portal/custom game numbers may be alphanumeric and are ordered by scheduled time, never by value.
- Results are recorded against a game, not a court.

## Out of scope

- **Restoring a live game** (clock, score, penalties) across a restart — a larger design that must
  answer what happened during the outage. Related: an unconfirmed score is lost if the box is shut
  down at the confirmation screen (observed today); same capability, same separate design.
- **Two refboxes on one court.** Needs portal-side arbitration. A "warn if a result exists"
  half-measure was rejected: it fires on legitimate corrections and re-sends, so it would be trained
  away. Known limitation.
- **LED panel and stream overlay** display of the finished state — `docs/backlog/court-finished-panel-state/NOTE.md`.
- **End-of-game buzzer precision** — pre-existing, own branch.
- Undelivered queued results at shutdown — real, not specific to this design.

## Known open item

The LED panel shows a frozen `NEXT GAME IN 0:30` in the finished state, unexplained. Engine clock is
provably 0, the panel draws only `secs_in_period`, single refbox and single simulator. Two
code-reading explanations were both wrong. **Instrument the wire; do not theorise.** The technique
that worked for the note race is in `reference_local_mock_portal_recipe`.

---

## Acceptance criteria

Observable by the operator, against a two-court event where court 1's games are exhausted:

1. Play out the last game → clock stops dead, `END --:--`, dashes, score retained, result posted for
   **that game only**.
2. Close and reopen **twice in quick succession** → still finished both times. No countdown, no game
   started, nothing posted. *(This is the morning's Critical.)*
3. Reopen and press **REFRESH repeatedly** → still finished every time. *(Scenario 4's Critical.)*
4. Reopen **with the network off** → finished, nothing started, nothing queued, and nothing posted
   when the network returns. *(Scenario 2's Critical.)*
5. Add a game to the court, press REFRESH → adopted; countdown runs; START NOW live; previous score
   stays until the new game starts.
6. Restart mid-event between games → resumes at the same game; the note is never overwritten while
   the schedule is unknown.
7. Pick an out-of-order game, press REFRESH → the pick survives.
8. During the last game on a court, half-time and pre-overtime breaks keep their whistle, beeps and
   a working START NOW.
9. Switch the portal off from the finished state → break counting, START NOW live, numbering from 1.
10. Point the refbox at a court it has no history for → offers nothing, asks for a pick.

Automated coverage is expected for the decision function (which game is next, given schedule, anchor,
pick and connectivity) and for the note's read/write rules. The app's `update()`/`apply_*` handlers
are not unit-testable in this codebase — those are the manual criteria above.

## Documents to correct on implementation

- `refbox/tests/features/court-finished.feature` — "A game added later is picked up by a refresh"
  (REFRESH is now required) and "A fresh launch offers the earliest game on the selected court"
  (superseded by rule 3).
- Walkthrough scenario 8 in the finishing plan, same reason.
