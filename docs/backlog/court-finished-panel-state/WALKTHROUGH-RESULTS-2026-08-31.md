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

## Rebase status — this walkthrough is STALE as of 2026-09-04

The branch was rebased onto `origin/master` (+108 commits) on 2026-09-04. Per
`.claude/rules/pr-review.md` a rebase stales both mandatory pre-PR checks, so the table above
records the pre-rebase build and is evidence for nothing until re-walked.

Agreed re-walk scope is **not** all ten criteria — only **2, 4, 5 and 9**: the two restart
Criticals, refresh-adoption, and 9 which has never been walked at all. Criteria 1, 3, 6, 7, 8 and
10 are being carried over on the grounds that master's changes do not touch what they exercise;
that carry-over is a judgement, not a verification, and is disclosed as such.

Two things the rebase changed that bear directly on these criteria:

- **Anchor clearing moved.** It now happens once inside `commit_link_selection`, the funnel every
  APPLY path goes through, instead of at four enumerated call sites. That also covers two paths
  master added in `apply_game_confirmation` which the old four did not. Criterion 10 (switching to
  a court with no history clears the anchor) is the criterion this most affects.
- **It has no automated cover.** Deleting the clearing call entirely leaves all 764 refbox tests
  passing — verified by mutation on 2026-09-04. No test constructs a `RefBoxApp` (53 fields), and
  every test module in `app/mod.rs` tests extracted pure functions instead. Criteria 4 and 10 are
  therefore the only real check on this behaviour.

## Bugs the walkthrough found — NONE caused by this branch
1. **After a start with no network, REFRESH silently does nothing.** The event-list fetch fails at
   boot, and `RequestPortalRefresh` asks only for the schedule — it never retries the event list.
   The schedule then arrives for an event that is in no store, so `RecvSchedule` discards it with
   "Received schedule ... but there is no event list yet" and no on-screen error. A box that booted
   without network needs a RESTART, not a REFRESH. Pre-existing.

   *Mechanism re-checked after the 2026-09-04 rebase and still present, but described differently
   from the original note: master replaced the flat `Option<BTreeMap<..>>` with a per-source
   `EventStore`, so the drop now happens because `self.events.get_mut(source, &event_id)` finds no
   entry, not because the handler sat inside `if let Some(ref mut events) = self.events`. Master's
   new `portal_list_loaded()` only chooses which of the two error messages is logged — it does not
   retry anything. Anyone filing this must describe the current shape, or they will be looking for
   code that no longer exists.*
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
