# Court-finished behaviour — decisions in progress

Running record of a design session, 2026-08-17. **Not the spec** — the spec gets written once all
situations are settled. Kept so the decisions survive a context loss.

Prompted by three defects found in four walkthrough scenarios on
`fix/uwh-common/no-next-game-on-court`, all the same shape: the refbox must decide "is this court
finished?" from incomplete information, and each code path guessed differently. See
`docs/superpowers/plans/2026-08-16-no-next-game-on-court-finish.md` for the evidence.

## Governing principle

**The refbox never invents a game.** A next game comes from the schedule or from the operator —
never from arithmetic, a stale cache, or a fallback.

**Boundary (situation 4):** the rule binds only in portal mode. In manual mode there is no schedule
to contradict, so sequential numbering (`N + 1`) is the specification, not a guess. The morning's
bug was precisely this arithmetic running while linked to a portal.

## Decisions

| # | Situation | Decision |
|---|---|---|
| 1 | Normal game ends, more follow | **Unchanged.** Break counts down, next game auto-starts at zero. Auto-start is not the danger; auto-starting the *wrong* game is, and the governing principle kills that at source. |
| 2 | Last game on this court ends | **Unchanged.** Clock stops dead, `END` over `--:--`, dashes, score stays indefinitely, result posted normally, START NOW greyed and inert. |
| 3 | Last game of the tournament ends | **No difference** from a finished court. The refbox only knows its own court; recognising "tournament over" needs cross-court knowledge it does not have and could get wrong. |
| 4 | Starts with no tournament selected | **Unchanged manual mode**, and the principle's boundary drawn here (see above). |
| 5 | Starts mid-tournament, between games | Show the **remembered game from the note immediately**, then confirm or correct from the schedule. Kills the `1 → 4 → 1` flicker: the remembered game replaces the arithmetic guess rather than racing it. |
| 6 | Starts *during* a game | Come back to the **same game, ready to start again**; live clock/score not restored. **Guarantee: never advance past the interrupted game** — skipping it loses the result entirely. Full live-game restore is a separate, larger design. |
| 7 | Starts after this court is finished | **Record the last game played, not the conclusion.** Derive "is the day done?" fresh from the schedule every time. Supersedes the earlier `court_finished: bool` proposal. |
| 8 | Starts after the tournament is over | Same as 7; keep the existing **five-day** restore window, dormant after that. |
| 9 | Starts on a court with no games played yet | Offer the **earliest game on this court**, from the schedule. Safe now because "no anchor" and "anchor = game 6" are distinct facts, where today both collapse to `"0"`. |

## Why decision 7 is the architectural one

The refbox finds the next game by taking the last game played as an anchor and searching the
schedule for the next game on this court after it. Good mechanism — but when the court is finished
it saves *"no game"*, discarding the anchor the search needs. On restart there is nothing to search
from, so it falls back to "offer the earliest game on this court" (game 1) and replays the day.

Recording the **fact** (game 6 was last played) instead of the **conclusion** (this court is
finished) means:

- restart → anchor 6, nothing after it, finished. Correct.
- refresh, any number of times → same search, same answer. Scenario 4's Critical cannot occur,
  because there is no one-shot flag to consume.
- a game added later → the same search finds it. Scenario 3's gap closes for free.

A fact stays true; a conclusion goes stale. It also *removes* the special flag rather than adding
one, and is smaller than the `court_finished` field proposed earlier in the session.

## Decisions (continued)

| # | Situation | Decision |
|---|---|---|
| 10 | Game added to a finished court | **Only on REFRESH.** A finished court stays finished until the operator asks. *Human ruling, against the recommendation to re-read automatically* — an unattended box changing state on its own is its own hazard, and the manual recovery already works. |
| 11 | Game added/rescheduled mid-break | **Only on REFRESH.** A running countdown never changes what it is counting toward. The game named on screen is the game that will start. |
| 12 | Game removed / moved to another court | On REFRESH, **re-run the normal search** from the anchor and show whatever is genuinely next (or finished). No special case, no warning. **Hard guarantee: a schedule change never disturbs a game in progress** — clock, score and penalties are untouched until it ends. |
| 13 | Portal unreachable at startup | **Run from the saved note; never fall back to arithmetic.** Show the game the operator was on and run it normally, or show the finished state and start nothing. Retry in the background. This is what kills the offline phantom. |
| 14 | Portal lost mid-day | **Nothing changes.** The schedule is already held, so nothing is unknown. Results queue on disk; the existing indicator shows the state. |
| 15 | Portal returns | **Send queued results automatically; leave the game state alone until REFRESH.** Sending is one-way and cannot surprise the operator; changing the next game can, and 10/11 already ruled that operator-driven. |
| 16 | Operator picks a different game | **The operator's pick wins** over the derived next game and survives refreshes, until that game is played or they change it again. They can see the pool; the schedule cannot. Guards the existing out-of-order scenario. |
| 17 | Operator switches the portal off | Finished state clears immediately, note deleted, normal break counting, START NOW live, and **numbering restarts at 1**. *Human ruling, against the recommendation to continue from the last game* — manual mode is a clean slate. The collision concern raised against it was wrong: with the portal off nothing is sent, so a second "game 1" cannot clash with anything. |

## Decisions (edge situations)

| # | Situation | Decision |
|---|---|---|
| 18 | Game abandoned partway (no result recorded) | **Anchor does not advance — offer the same game again.** An abandoned game has no result, so as far as the tournament is concerned it has not happened. Skipping it is unrecoverable and only surfaces at reconciliation; re-offering costs seconds, and decision 16 lets the operator move past it. Same guarantee as decision 6. |
| 19 | Long gap between blocks (e.g. lunch) | **Nothing special.** A later game exists, so the court is not finished — already correct. A long countdown is accurate information. No threshold for "long", which would be wrong for someone's event. |
| 20 | Court closed mid-day, games moved elsewhere | **Show the finished state; never follow the games.** On REFRESH the search finds nothing left here, which is true. The refbox must never re-point itself at another court — that is the original damage class. The operator moves the box and selects deliberately. |
| 21 | Operator changes court mid-day | **Offer nothing; require a pick.** The refbox has no history for the new court (another box ran it), so it cannot tell which games are played. Every automatic answer is a guess: "earliest game" is one played hours ago, and clock-based choices skip the overdue game actually about to be played. |
| 22 | Tournament runs past midnight | **Nothing special.** Games are ordered by full timestamp, so a 00:30 game is simply later than a 23:00 one. Midnight is not a boundary the refbox notices. Confirmed no operational day-boundary exists. |
| 23 | Two refboxes on the same court | **Out of scope.** Detection needs portal-side arbitration the refbox cannot do. Recorded as a known limitation. A "warn if a result exists" half-measure was rejected — it fires on legitimate corrections and re-sends, so it would be trained away. |
| 24 | Selected court has no games at all | **Display identical to a finished court**, with the difference kept internally so an empty court is never mistaken for a completed one. No new wording. |
| 25 | Operator switches to a different event | **Clear everything event-specific** — anchor, remembered game, court — and require re-selection. Game numbers are per-event, so a carried-over anchor would point at a real but wrong game and look entirely plausible. |
| 26 | Last game ends, score never confirmed | **Unchanged; risk recorded.** Confirmation stays required and an unconfirmed result is not sent. Auto-submitting defeats the gate's purpose at the one moment nobody is watching. Surviving a restart is the same capability as live-game restore (decision 6) and belongs with it. |
| 27 | Last game goes to overtime / sudden death | **Its own breaks behave completely normally** — whistle, beeps, buzzer, working START NOW. "Finished" describes the schedule *after* this game. Written as an explicit rule: the last game is often a final, and this is the easiest thing for a future change to break. |
| 28 | Operator picks a game listed on another court | **Not allowed — the picker offers only this court's games.** *Human ruling, against the recommendation to allow deliberate cross-court picks.* Consequence: if a game genuinely moves here late, the routes are to correct the portal schedule or use manual mode. |
| 29 | Schedule downloads but cannot be parsed | **Treat exactly as "no schedule"** (decision 13). Indistinguishable from the refbox's view, and folding them together means one behaviour to specify and test rather than two that drift apart. |
| 30 | Non-numeric game numbers | **Manual mode is always sequential integers; portal/custom may use alphanumeric labels** (human confirmation). Portal mode never does arithmetic, so labels are safe. The existing "couldn't parse, default to game 1" fallback (`tournament_manager/mod.rs:219-228`) is therefore **unreachable — if it fires it is a bug, not a fallback.** |
| 31 | Two games in the same slot on one court | **Not possible** (human confirmation). State as an assumption: one game per court per slot. Anything else is an upstream data error, not a case to handle. |

## Decision 9 is SUPERSEDED

Decisions 9 and 21 were the same situation internally — **no anchor for this court** — with opposite
answers, and the refbox cannot tell them apart. A replacement box brought out mid-day has no anchor,
so decision 9 would have confidently offered game 1, played hours earlier. The replay bug again.

**Resolved: a court with no recorded history always requires an operator pick**, morning or not.
One selection per court per day.

This does better than reconcile the two — it **deletes the branch that caused scenario 4's
Critical**. The `anchor_num == "0"` → "offer the earliest game on this court" path is precisely what
re-adopted game 1 after a restart. Removing it beats guarding it.

## Consequences to carry into the spec

- **The note must hold two things**, not one: which game was last *played* (the anchor for deriving
  what is next) and which game the operator is currently *on* (so it can be shown instantly, and
  offline). Decision 7 gives the first; decision 13 requires the second.
- **`refbox/tests/features/court-finished.feature` is now wrong by decision, in two places.**
  "A game added later is picked up by a refresh" claims it happens with no operator action —
  decision 10 requires REFRESH. "A fresh launch offers the earliest game on the selected court" is
  contradicted by the supersession of decision 9. Both must be rewritten, along with walkthrough
  scenario 8.
- **Delete, do not guard, the "offer the earliest game on this court" branch.** It is the direct
  cause of scenario 4's Critical and has no remaining legitimate caller.
- **The parse-to-`"1"` fallback should become an error, not a default** (decision 30).
- **Decisions 7 + 16 must not fight.** The anchor-derived game is a *default*, not an override; an
  explicit pick outranks it.
- Deliberately parked: results can sit undelivered in the on-disk queue when the box is switched off
  for the last time (raised under situation 3) — real, but not specific to tournament end.
