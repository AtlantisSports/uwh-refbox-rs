# Court-finished walkthrough — results, 2026-08-31

Human-driven walkthrough (pre-PR check 2), Eric at the keyboard, Claude launching and reading the
logs/portal. Rig: local mock portal (court 1 = games 1, 3, later 6; court 2 = 2, 4, 5), isolated
XDG_CONFIG_HOME, binary built from this branch.

**9 of 10 acceptance criteria PASSED. Criterion 9 is blocked by the machine, not the code.**

| # | Criterion | Result |
|---|---|---|
| 1 | Last game ends: clock dead, `END --:--`, dashes, score held, result posted for that game only | **PASS** — 1-1 held on the tiles; exactly one POST, `games/3/scores`; queue empty |
| 2 | Close and reopen twice: finished both times, no countdown, nothing posted | **PASS** — both reopens parked at `00:00.000`; POST count stayed 2 |
| 3 | REFRESH repeatedly: finished every time | **PASS** — 5 schedule fetches, 5 x "No further games", never once "Setting upcoming game info" |
| 4 | Offline restart: nothing started, nothing queued, nothing posted on reconnect | **PASS** — no queue file at all; 0 POSTs across 60s reconnected |
| 5 | Game added, REFRESH: adopted, countdown runs, START NOW live | **PASS** — held finished while game 6 sat on the portal untouched; adopted only when asked |
| 6 | Mid-event restart: same game, never skipped | **PASS** — came back to game 6 by DERIVING it from anchor 3, even though the note lagged |
| 7 | Out-of-order pick survives REFRESH | **PASS** — picked game 1 over the derived 6; three refreshes kept game 1 |
| 8 | Half-time of the last game keeps whistle and START NOW | **PASS** — court flagged finished AT kickoff, yet the 30s whistle fired and START NOW began the second half early (18.75s left) |
| 9 | Portal off from finished: break counts, START NOW live, numbering from 1 | **BLOCKED** — refbox cannot start: WSLg PulseServer socket is dead, `snd_pcm_open ... Connection refused`, exit 101. Needs `wsl --shutdown`. NOT a code failure. |
| 10 | Court with no history: offers nothing, asks for a pick | **PASS** — switching to court 2 cleared the anchor (`last_played: null`) and required a pick |

## What criterion 9 does have, pending the visual check
`test_reset_to_manual_break_starts_the_break_when_schedule_linked` passes, and was proven by
MUTATION during review: commenting out its single production line turns it red with "the break
clock should be running (counting down) even when the engine was schedule-linked". Unusual for
this set — most criteria have no automated cover. What is missing is only the on-screen half:
numbering showing 1, and START NOW visibly live.

## Bugs the walkthrough found — NONE caused by this branch
1. **After a start with no network, REFRESH silently does nothing.** The event-list fetch fails at
   boot; `RecvSchedule`'s handling is nested inside `if let Some(ref mut events) = self.events`, and
   a refresh never retries the event list. Every refreshed schedule is discarded with
   "Received schedule ... but there is no event list yet" and no on-screen error. A box that booted
   without network needs a RESTART, not a REFRESH. Pre-existing.
2. **A refresh that adopts a game does not write it to the note** for up to ~5 minutes (the health
   heartbeat). Safe direction and self-healing, but inside that window an offline restart shows
   nothing rather than the adopted game, which the spec's connectivity row expects.
3. **No audio device takes the whole refbox down** (panic, exit 101, inside `web-audio-api`).
   On a poolside Pi with a flaky sound card this loses the clock, scores and penalties, not just
   the buzzer. Pre-existing and already known; this walkthrough is a second sighting.

## Operator observation worth a decision
Changing court or game DURING a running between-games countdown is accepted silently — no
confirmation — even though that countdown was seconds from auto-starting a game. The confirmation
only fires when a game is actually in play. Existing behaviour, not specified either way by the
spec's decisions. Eric raised it; logged as a follow-up rather than changed here.
