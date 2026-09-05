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
