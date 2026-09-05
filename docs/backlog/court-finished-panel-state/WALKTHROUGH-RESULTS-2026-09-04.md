# Court-finished walkthrough — results, 2026-09-04 (re-walk after the rebase)

Human-driven walkthrough (pre-PR check 2), Eric at the keyboard, Claude launching and reading the
logs/portal. Agreed re-walk scope after the rebase was **criteria 2, 4, 5 and 9 only**; 1, 3, 6, 7,
8 and 10 are carried over from 2026-08-31 as a disclosed judgement, not a verification.

Rig: local mock portal on **8100** (8099 belongs to a peer session), isolated `XDG_CONFIG_HOME`,
token `walkthrough-test-key`, custom-site URL also repointed at the mock so nothing could reach the
real dev portal. Binary rebuilt from the rebased branch. Branch renamed this session to
`fix/uwh-common/court-finished-behaviour` (review finding 12).

| # | Criterion | Result |
|---|---|---|
| 1 | Last game ends: clock dead, `END --:--`, dashes, score held, START NOW grey | **PASS** (not in scope — re-confirmed as a by-product of reaching the finished state) |
| 2 | Close and reopen twice: finished both times, no countdown, nothing posted | **PASS**, and stronger than August: walked with **two different shutdown kinds** |
| 4 | Offline restart: nothing started, nothing queued, nothing posted on reconnect | **PASS** |
| 5 | Game added, REFRESH: adopted, countdown runs, START NOW live | **PASS** |
| 9 | Portal off from finished: break counts, START NOW live, numbering from 1 | **NOT WALKED** — WSL audio died mid-session; refbox cannot start at all. Third attempt across three sessions. |

## Criterion 1 (incidental) — evidence
Eric played game 3 (Eels 1 – Turtles 0). Screen: `END` over `--:--`, Prior Game 3 with the score
held on the tiles, all game-info values dashed, START NOW grey. Portal side: **exactly one score
POST**, `games/3/scores` `{"dark":{"value":1},"light":{"value":0}}`, nothing for games 1/2/4/5.
Engine: *"Last game on this court is over; stopping the clock"*, *"Not starting the game clock: no
further games on this court"*. Note written: `last_played: "3"`, `current_game: null`.

## Criterion 2 — PASS, with both shutdown kinds
- **Restart 1, abrupt (`kill -9`)** — the pool-side power-cut case. Note survived byte-for-byte.
- **Restart 2, tidy (window close, exit 0)** — the August case.

Both came back with only three engine decisions and nothing else:
```
Restoring Portal link to events/1-A (court Some("1"), game None)
Setting game clock to 00:00.000
No further games scheduled on this court
```
No *"Setting upcoming game info"*, no clock start, no mention of game 1, and the POST count never
moved off 2. Eric confirmed on screen: *"no countdown whatsoever that I could see"* — and because he
flagged he might have missed the first frame, the log settles it: no countdown was ever created.

**Observation (not a defect):** after a restart the panel reads **Prior Game 0** with a blank score,
where before the restart it read Prior Game 3 with the score. That row is fed from the live
snapshot, meaning "the last game that finished *in this session*" (there is a test whose comment
says exactly that). The note's `last_played` drives scheduling, not that display.
**This makes one line of `WALKTHROUGH-2026-08-29.md` wrong** — it says a restart onto a finished
court "looks identical to finishing normally". It does not: the Prior Game row differs.

## Criterion 4 — PASS
Portal stopped (`curl` exit 7, connection refused), then a restart onto the dead network. Over
**2 min 21 s** the engine did three things and nothing more (restore court, clock to 00:00.000,
`Failed to get event list`), with **no game derived or adopted** and **no queue file content**
(`{"version":1,"items":[]}`). Eric confirmed on screen: finished, red portal dot, nothing starting.

On reconnect refbox recovered contact by itself (*"portal token validation successful"*, matching
`GET .../access-keys/verify 200`) and **posted nothing** — POST count still 2. The on-screen
reconnect moment is covered by the engine log (zero engine lines after 13:31:23), not by a
screenshot.

**Second sighting of the known pre-existing defect:** refbox recovered the *health check* but never
re-fetched the event list or schedule, so the dot goes green while the app still has no schedule.
A box that booted with no network needs a **restart**, not a REFRESH. Consequence for this
walkthrough: criterion 5 could not be walked on that instance at all.

## Criterion 5 — PASS
With the court finished, game 6 was added to the portal *after* boot; refbox had fetched the
schedule exactly once, so it could not know. Eric pressed REFRESH:
```
Got schedule
Setting upcoming game info from received schedule: Game { number: "6", … Rays … Squid …,
    court: "1", description: "Court 1 - added AFTER the court was finished" }
Next Game Info set to … number: "6" … TEST … half_play_duration: 20s …
Next game start time is in the past; using current time as anchor
Setting between games time based on uwhportal info: 45s
```
The break then ran for real — whistle 13:40:31, countdown beeps, buzzer 13:40:59 — and at 13:41:00
it **entered the first half of game 6**. Eric's screenshot of the Game Information page confirms
**Current Game 6, Rays vs Squid** with the TEST rule (0:20 halves, 0:45 half-time). The green
START NOW itself was not photographed; the running break that auto-started the game is the same
state that makes it live.

**Operational fact worth knowing:** a REFRESH that adopts a game gives roughly **45 seconds**
before the box starts playing it, attended or not. Game 6 duly played itself out while we were
looking at the wrong window, and posted a 0-0 (`games/6/scores`). Not a defect — but it is what
"adopted" means.

## Criterion 9 — NOT WALKED (machine, not code)
The route was established before the attempt, and the trap is self-detecting: switching the portal
off calls `reset_to_manual_break`, which sets `game_number = "0"`, so **numbering must show 1**;
the App-page APPLY route would leave it at last-played + 1, i.e. **7** here.

Then WSL audio died again (it had been working — the game 3 buzzer fired). refbox cannot start
without a sound device:
```
ALSA lib pulse.c:242:(pulse_connect) PulseAudio: Unable to connect: Connection refused
thread 'main' panicked at 'InvalidStateError … snd_pcm_open … Connection refused (111)'
  web-audio-api-1.2.0/src/io/cpal.rs:153        → exit 101
```
**There is no workaround, and this was tested, not assumed.** Redirecting ALSA's `default` device to
`null` via `ALSA_CONFIG_PATH` changes nothing (cpal opens the *pulse* device by name); also
overriding `pcm.pulse` gets past the immediate panic, but the app then stalls in audio init
(`pulse_connect: Timeout`), never reaches *"Restoring Portal link"*, and panics with the same
exit 101 anyway. No sound daemon is installed in the distro to substitute
(`pulseaudio`, `pipewire`, `pactl`, `aplay` all absent). Sound is also not gated by config or a CLI
flag — `AudioContext::new` is unconditional. **Only restoring WSLg audio (`wsl --shutdown` from
Windows, which kills every WSL session) unblocks this.**

Criterion 9 still has `test_reset_to_manual_break_starts_the_break_when_schedule_linked`, proven by
mutation. What is missing remains only the on-screen half: numbering showing 1, and START NOW
visibly live.

## Evidence hygiene — two Claude-side mistakes, and what they cost
1. **`pgrep -f 'target/debug/refbox …' | head -1` returns the harness's bash wrapper**, not the app.
   SIGTERM hit the wrapper, the real app kept running, and a second instance was launched alongside
   it — two identical windows for about a minute. **The first criterion 4 attempt was voided and
   re-run.** Criteria 1 and 2 were unaffected (verified against real PIDs, one instance at a time).
2. **A kill loop over `pgrep -x refbox` killed a peer session's refbox and LED simulator.** Match
   `readlink /proc/<pid>/exe` against your own build before signalling anything.
   Also: `pkill -f 'server.py 8100'` matches the issuing shell's own command line and kills it.

Fix adopted for the rest of the session: my instance ran in **dark mode** (colours only) so it was
unmistakable, and every signal was gated on the executable path.

## Still owed by Eric (was three, now two)
1. Rescheduled anchor: an already-played game moved LATER can make a live court read "finished".
   Safer failure direction chosen; the spec does not settle which clock wins.
2. Should changing court/game DURING a running between-games countdown warn? It currently does not.

## Also seen, pre-existing, not filed
The portal connection indicator shows **green for about three seconds at startup** before the first
health check fails (launch 13:31:22, first failure 13:31:25) — a freshly started box briefly claims
a connection it does not have.

---

# Re-walk after the 21:08 rebase — criteria 2, 4, 5 and 9

**Everything above this line is superseded.** Those results were obtained against base `486c5692`.
While the walkthrough was in progress a peer session rebased this branch onto `1229c396` (master had
moved +8 — the roster-before-kickoff work merged), which stales both mandatory checks by the
project's own rule. What follows was walked afterwards, in full, on the rebased branch.

**Commit walked: `808b146b`.** HEAD was read before and after every step and never moved, so these
results describe the branch as it actually stands. `just check` on the same commit: **1228 tests,
0 failures**, run in this worktree with HEAD identical before and after. Binary rebuilt from that
commit, with zero source files newer than it.

Rig: mock portal on 8100, isolated `XDG_CONFIG_HOME`, court 2 (its last game is 5). Eric at the
keyboard for criteria 5 and 9; Claude drove the restarts for 2 and 4 and read the logs and the
portal's post record throughout.

| # | Criterion | Result |
|---|---|---|
| 2 | Close and reopen twice: finished both times, no countdown, nothing posted | **PASS** — walked with two shutdown kinds, a polite SIGTERM and a hard SIGKILL |
| 4 | Offline restart: nothing started, nothing queued, nothing posted on reconnect | **PASS** |
| 5 | Game added, REFRESH: adopted, countdown runs, START NOW live | **PASS** |
| 9 | Portal off from finished: break counts, START NOW live, numbering from 1 | **PASS** — walked for the first time in four sessions |

## Criterion 9 — evidence, and the fix it validates

Game 7 was played out to reach the finished state, the source was switched off UWH PORTAL and
applied on the settings page. On screen: break counting from 15:00, START NOW green, **Current Game
1** (not 8 — the App-page route would have numbered from the anchor). Score read 1-0 at 14:47 and
0-0 at 12:58. The engine log:

```
[15:00.000 BTWNGMS] Will reset game at 780s     <- start_nominal_break arming it (900s - 120s)
[13:05.013 BTWNGMS] Starting the game clock     <- TIME EDIT, to avoid waiting two minutes
[12:59.998 BTWNGMS] Resetting game              <- the old score cleared, on 13:00
```

Before the fix that first line read `0ns`, and the finished game's score, penalties and timeouts
stayed on the display for the whole 15-minute break.

## Posts

Five score posts exist in the mock's log across the whole day, every one of them a game somebody
played (3, 6, 6, 5, 7). Nothing was posted during an outage, on reconnect, across four restarts, or
during any break. The count was checked before and after every step.

## Two rig facts worth not rediscovering

**Criterion 2 must be walked before criterion 9.** Criterion 9 leaves the court in a manual break,
and there is no way back to the finished state except replaying a last game — switching the source
back to the portal requires picking a game, which starts you at the beginning of that game. Walking
9 first cost a full re-setup.

**A restored finished court does not reproduce criterion 9's precondition.** Restarting into a
finished court leaves `reset_game_time` at its constructed default rather than the zero that
`end_game`'s finished branch writes, so the defect the fix addresses does not arise. The finished
state has to be reached by playing the last game out.

## Not walked

**Criterion 5 could not be walked in the offline-started instance** and was walked after an online
restart. Criteria 1, 3, 6, 7, 8 and 10 remain carried over from 2026-08-31 as a disclosed judgement,
not a verification.

---

# New step, 2026-09-05 — a letter-prefixed game number carried into manual mode

Walked by Eric at commit `fae8becf` (the three fourth-review commits), on an isolated rig: a second
mock portal on **8101** with its own schedule and its own `XDG_CONFIG_HOME`, so the session already
running on 8100 was never touched. Binary built into a separate target directory for the same
reason, which also made the two windows tellable apart by `readlink /proc/<pid>/exe`.

**Why a new schedule was needed.** Every game in the 8100 schedule is numbered 1-7. The commit under
test only changes what happens when the number is **not** an integer, so walked against those games
the step passes whether or not the fix exists. The rig schedule numbers court 1's games **G27** and
**G28**. Eric confirmed the same day that real events do sometimes label games this way, depending on
the organiser — so this is a field case, not an invention.

**Route:** link to the portal, pick court 1 / game G27, START NOW, then mid-game set MANUAL GAMES to
ON and APPLY, answering the confirmation with **KEEP CURRENT GAME AND APPLY CHANGE**. Play G27 out
and confirm the score.

**Result: PASS.** Prior Game **G27** (score Black 1, White 0), Current Game **1**, the NEXT GAME
clock counting down, START NOW green.

```
22:20:24  [00:26.090 BTWNGMS] Entering first half of game G27
22:20:59  Schedule-linked set to false          <- KEEP CURRENT GAME AND APPLY CHANGE
22:21:48  [00:00.010 BTWNGMS] Ending game G27. Score is Black: 1, White: 0
22:21:48  [00:00.010 BTWNGMS] Entering between games, time to next game is 905.2s
```

The engine's `game_number` was still the literal `"G27"` when the game ended — `KeepGameAndApply`
calls only `clear_portal_next_game`, which does not touch it, and `reset_to_manual_break` cannot have
run because the period was not `BetweenGames` at 22:20:59. So `next_game_number()` reached the
unparseable arm, which is the line this commit changes. Before it, that arm returned a blank and the
court parked at 0:00 with START NOW greyed.

## A false pass caught on the way, worth not repeating

The first screenshot showed Current Game **1** with the clock counting down — the pass state — from a
box that had done nothing at all since launch. Fresh-launch manual numbering starts at 0, so the next
game reads 1 on its own. **The discriminator is the Prior Game row:** it must name the finished game
(`G27`). A step whose expected reading is also the startup reading cannot fail.

## Two observations, neither a defect of this branch

- **The portal game's timing survives the switch to manual.** Half Length stayed 0:20 rather than
  returning to the manual 15:00, because KEEP CURRENT GAME AND APPLY applies the settings as they
  stood on the page, which held the portal game's timing rule. Pre-existing; not in scope here.
- **Nothing is posted for G27.** The only portal traffic in the whole run was the login. Correct: the
  portal was switched off before the game ended.

## Known gap

The state now resolves correctly and **silently**. The `set_game_number` warning updated in
`09183f21` sits in a function this route never calls, so nothing records that numbering restarted.
That matches Eric's ruling (carry on from 1, rather than park and announce), but it means a field
occurrence leaves no trace in the log.

---

# The six carried-over criteria, walked at last — 2026-09-05

Criteria 1, 3, 6, 7, 8 and 10 had not been walked since 2026-08-31, and two rebases had landed
since. Carried forward twice as "a disclosed judgement, not a verification". Eric chose to walk them
rather than carry them a third time. All at `e3f2ee31`, on the isolated rig (mock portal 8101, own
`XDG_CONFIG_HOME`, own target dir), run as one continuous session on court 1.

| # | Criterion | Result |
|---|---|---|
| 6 | Mid-event restart: same game, never skipped | **PASS** — returned to game 3, Turtles v Eels |
| 7 | Out-of-order pick survives REFRESH | **PASS** — and the refresh genuinely fetched |
| 8 | Half-time of the last game keeps whistle and START NOW | **PASS** on observation; see the gap below |
| 1 | Last game ends: clock dead, `END --:--`, dashes, score held, one result posted | **PASS** |
| 3 | REFRESH repeatedly: finished every time | **PASS** — 5 refreshes, 0 adoptions |
| 10 | Court with no history: offers nothing, asks for a pick | **PASS** — GAME field blank |

## What made each falsifiable

The recurring risk in this set is a step that reads the same whether or not the code works. Two were
nearly that, and were tightened before being recorded:

- **Criterion 7** would "pass" if REFRESH did nothing at all. The mock's access log shows the fetch
  and the app log shows it acted: `22:47:39 Got schedule` → `22:47:40 Setting upcoming game info …
  Game 1, Sharks/Barracudas`. The box received a schedule that pointed at game 3 and *chose* to keep
  the operator's pick.
- **Criterion 3** likewise. Six `Got schedule` since game 3 ended (one at game end, five from Eric),
  six `No further games scheduled on this court`, and **zero** `Setting upcoming game info`.
- **Criterion 10** could not be told from its failure by the log alone — both readings end with game
  2 selected. Eric confirmed the GAME field was **blank and required a pick**, which is the pass.
  Recorded on his direct observation, not inferred.

## Criterion 8 — the precondition genuinely held

Worth stating because it is the criterion most easily walked hollow. At kickoff of game 3:

```
22:51:10  No games scheduled on court 1 after game 3
22:51:10  No further games scheduled on this court
```

So the court was flagged finished eight minutes *before* the half-time being tested. The whistle
fired at 22:51:59 (`Playing whistle once`) and Eric confirmed he **heard** it — the log only proves
the app tried. START NOW was live and pressable.

**Gap, disclosed:** half-time ran its full 45 seconds (22:51:40 → 22:52:24), so START NOW was seen
live but never *pressed*. "START NOW works to begin the second half early" is therefore observed,
not exercised. Folded into the overtime run below rather than contrived separately.

## Criterion 1 — the posting half

Across the entire run the rig logged exactly two results: game 1 at 1-0 and game 3 at 2-1. One per
game actually played, nothing invented, nothing duplicated, nothing posted during a break or a
refresh.

## A gap in the criteria themselves, raised by Eric

**None of the ten cover overtime or sudden death on a finished court** — and the last game on a court
is very often a final, which is exactly where they happen. Checked in code: all three suppression
points test the period, not the game — `court_schedule_finished()` is
`current_period == BetweenGames && next_game_number.is_empty()`, `start_play_now`'s refusal sits
inside its `BetweenGames` arm, and the sound gate is `period == BetweenGames && …`. No in-game break
can be `BetweenGames`, so the class is structurally excluded rather than enumerated.

**But nothing automated covers it.** The golden trace does not render `game_number` or
`next_game_number` (both marked "not core timing/state", design doc §4), and no golden scenario
references `set_no_next_game`, `no_next_game`, `schedule_linked` or `court_schedule_finished` — the
state never occurs under the trace at all. Follow-up worth filing: **add a golden-trace scenario for
a finished court.** Not on this branch.

---

# Criterion 11 (new) — overtime and sudden death on a finished court — 2026-09-05

**Raised by Eric, and it was a real gap: none of the original ten cover it.** The last game on a
court is very often a final, and a final is exactly where overtime and sudden death happen. Walked
at `e3f2ee31` on the isolated rig, with the TEST timing rule given `overtimeAllowed`,
`suddenDeathAllowed`, and 45s breaks so the 30-second whistle could fire in each.

**Result: PASS**, and it also closes the piece criterion 8 could not.

Game 3 — the last on court 1, so the court was flagged finished at kickoff
(`23:21:54 No further games scheduled on this court`) — was played 0-0 through regulation and both
overtime halves, into sudden death. Every in-game break counted down normally and **START NOW
started the next period early in all four of them**:

| Break | Entered | Started early with | START NOW |
|---|---|---|---|
| Half-time | 23:22:23 | 22.8s of 45s left | worked |
| Pre-overtime | 23:23:14 | 13.3s of 45s left | worked |
| Overtime half-time | 23:24:02 | 36.7s of 45s left | worked |
| Pre-sudden-death | 23:24:30 | 23.8s of 45s left | worked |

Whistles fired at 23:23:29 (pre-overtime) and 23:24:41 (pre-sudden-death). The overtime half-time
shows none only because it was started 8s in, before the 30-second mark — not a miss.

Periods reached in order: FRSTHLF, HLFTIME, SCNDHLF, PREOVTM, OTFRSTH, OTHLFTM, OTSCNDH, PRESDND,
SUDNDTH, then `Last game on this court is over; stopping the clock`. Final: Score is Black: 0, White: 1

**Criterion 8's open half is closed by this run.** Half-time ended with 22.8s still on a 45s clock,
logged as "Second Half manually started by refs" — START NOW was pressed, on a finished court, and
it worked.

## Why this was worth walking even though the code argument was sound

All three suppression points test the period, not the game, so no in-game break can be
`BetweenGames` and the class is structurally excluded. That argument is correct. It was still worth
walking, because **twice on this branch a "same class, must be fine" argument has been wrong about a
site nobody had checked**: the fourth review's finding was the third site of a pattern already fixed
at two, and the peer session's `reset_game` fix was the fourth. Two of three behaving is not
evidence for the third.

## Posting

Four games were played tonight and four results posted, one per play: game 1, game 3, game 2, and
game 3 again (this run). Game 3 appearing twice is the deliberate replay, not an invented post.

## Follow-up this leaves

The golden trace still cannot see any of this — it renders neither game number, and no scenario ever
reaches the finished-court state. **Add a golden-trace scenario for a finished court**, including an
overtime path. Not on this branch.
