# Live Updates — refbox design

Status: APPROVED by the PO (Eric) 2026-10-19 and built on `feat/uwh-common/live-updates` (not yet merged). Read
against refbox `origin/master` @ 701ea226. Where the build departs from this text, the plan's "Deviations" section
says so.

**v3 note (2026-10-19, `feat/uwh-common/live-updates-v3`):** this document describes v2 and is left as written. v3
adds three body fields: `lastGoal`, `penalties`, and `timeoutSeconds` / `timeoutRunning` for a White or Black team
timeout. So the `timeout` row below ("the kind only, no seconds"), "Nothing else" under the body table, "a timeout
timer" among the non-goals, and any statement that penalties are not sent no longer hold for a v3 refbox. The fields,
their sources and their send-at-once rules are in the agreement's v3 section:
`uwh-portal-live-schedule/docs/superpowers/specs/2026-10-19-live-schedule-refbox-live-updates-agreement.md`, under
"v3 (PROPOSED by refbox 2026-10-19, portal-reviewed): last goal, active penalties, team-timeout countdown".

## Why

The public event Schedule tab on uwhportal will show games in progress: a LIVE marker, the score, the part of the
game, a ticking game clock, and a "Next Game" countdown between games. Real events run this refbox, which today sends
the portal only the final score and stats after each game. A uwhportal lane (session uwh-portal-ee, "live-schedule")
is building the portal half. This document is the refbox half: a setting, **Live Updates**, and a sender that pushes
live game state to the portal while a game runs.

## The contract with the portal

What is sent, where and when is fixed by an agreement the portal lane drafted and this workspace reviewed. Every item
in it is settled (2026-10-19):
`/home/estraily/projects/uwh-portal-live-schedule/docs/superpowers/specs/2026-10-19-live-schedule-refbox-live-updates-agreement.md`
(a uwhportal worktree — read it, never edit it). The essentials, restated so this document stands alone:

**Send only when ALL hold:** game source is Portal (never Manual, never Custom — custom-site event ids share the
Portal's shape and must never reach the Portal); mode is Hockey 6v6 or 3v3 (Rugby goes to the UWR portal, not our
tenant; Beep Test has no game); an event is selected and the refbox holds an access key for (that Portal, that event);
the path game number is a real schedule number, never `"0"` or blank; and Live Updates is on.

**Request:** `PUT {portal}/api/events/{partialEventId}/live-games/{gameNumber}`, same Portal address and same
`Authorization: Bearer <key>` header as the existing `POST …/schedule/games/{n}/scores`. `gameNumber` is URL-escaped.

**Which game the path names:**
- In play (any period except `BetweenGames`): the current game.
- `BetweenGames` with a next game: the NEXT game (this is how the first game of the day gets its countdown, since the
  refbox's own current number is `"0"` then).
- `BetweenGames` with no next game (court finished): the just-finished game, ONE send only, then no sends until a next
  game exists again. If the just-finished number is also `"0"`/blank, send nothing.

**Body (JSON):**

| Field | Value |
|---|---|
| `period` | `GamePeriod` variant name exactly (`BetweenGames`, `FirstHalf`, … `SuddenDeath`). |
| `clockSeconds` | `secs_in_period`. Counts down everywhere except `SuddenDeath` (counts up). In `BetweenGames`, the countdown to the next game. |
| `clockRunning` | The GAME clock's running state — `TournamentManager.clock_state.is_running()`. **Not** `clock_is_running()`, which reports the timeout clock whenever a timeout exists. False during any timeout and the score-confirmation pause. |
| `timeout` | `"White"`, `"Black"`, `"Ref"`, `"PenaltyShot"` or `null` — the kind only, no seconds. |
| `scores` | `{ "black": n, "white": n }` in play; **`null` in `BetweenGames`** (the refbox resets to 0-0 mid-break while still naming the old game). |

Nothing else. Every field is public on the Schedule tab, so adding one is a change to the agreement, not a refbox call.

**Timing:** a send every 5 seconds while the conditions hold (including while the clock is stopped), plus one straight
away when the score, the period, the timeout kind, the game clock's running state, or the path game number changes.
*(Claude's call, after the builder check)*: the ticking clock reading is **not** a change — comparing whole bodies
would send every second while the clock runs, five times the data the PO approved. The one exception: while the game
clock is **stopped**, a different `clockSeconds` is an operator's clock edit (TIME EDIT) and sends straight away.

**Court-finished send** *(Claude's call)*: the body is exactly the agreement's — `clockSeconds: 0`,
`clockRunning: false`, `timeout: null`, `scores: null` — set explicitly, not read from the snapshot. The agreement is
the contract and the portal shows nothing live for this state. If the parked clock turns out not to read 0, tell the
portal lane; do not change what is sent.

**The refbox's three guarantees** (the portal relies on them instead of a sequence number):
1. At most ONE live send in flight. A send due while one is in flight is not queued behind it; the next send after it
   finishes carries the newest state (coalesce, don't stack).
2. A short request timeout (a few seconds). A send stuck on bad wifi is abandoned, never delivered late.
3. Values are read when the send is built — never older than the latest snapshot (≤ ~1 s while the clock runs).

**Failures:** any non-2xx or transport error is logged once per distinct failure (not every 5 s) and dropped. Never
retried, never queued, never surfaced on screen (PO ruling below).

**Data:** ~150-byte body, ~1 KB per exchange with the connection kept open, ~0.7–1 MB per court per hour. Measure it
during the build (see Verification) and report the real figure to the PO.

## What the PO decided

1. **Default ON.** A fresh install and an upgraded install with no stored value both get Live Updates on. Operators at
   limited-data venues switch it off. (PO, 2026-10-19, after the data figures above.)
2. **The switch lives on APP OPTIONS**, and that page is rearranged to make room. Left column unchanged; right column
   becomes, top to bottom:

   ```
   ┌──────────────────┬──────────────────┐
   │ [ game clock / status bar ]         │
   ├──────────────────┼──────────────────┤
   │ APP: HOCKEY 6V6  │ CONFIRM SCORE    │
   ├──────────────────┼──────────────────┤
   │ TRACK CAP NUMBER │ LAST 10 SECONDS  │   ← new plain tile (no value line); opens the page below
   │                  │ COUNTDOWN        │
   ├──────────────────┼──────────────────┤
   │ FORCE KEYPAD     │ SHOW BEHIND TIME │   ← moved up one space
   ├──────────────────┼──────────────────┤
   │ TRACK FOULS      │ LIVE UPDATES: YES│   ← new
   ├──────────────────┴──────────────────┤
   │ CANCEL │ CHECK VERSION │   APPLY    │
   └─────────────────────────────────────┘
   ```

   - The tile's label is literally `LAST 10 SECONDS` on one line and `COUNTDOWN` on the next (PO wording — do not
     rephrase).
   - **LIVE UPDATES** shows YES/NO like its neighbours. Where it cannot work (game source Manual or Custom, or Rugby
     or Beep Test mode) it is greyed and reads **NO** — the value in force, not the stored one — exactly as TRACK FOULS reads NO
     when cap numbers are off. The operator's stored YES is kept and comes back when the conditions return.
   - It can be changed mid-game (the App page's APPLY is not locked during a game). Turning it off stops sends at
     once; turning it on starts them at once.
3. **The LAST 10 SECONDS COUNTDOWN page** holds the two existing switches (SHOW COUNTDOWN — stored inverted as
   `hide_time` — and AUDIBLE COUNTDOWN). It copies the **Half Time Length / Half Length** editor pages in format,
   styling and behaviour:

   ```
   ┌─────────────────────────────────────────────┐
   │ [ game clock / status bar ]                 │
   ├─────────────────────────────────────────────┤
   │          LAST 10 SECONDS COUNTDOWN          │
   │   SHOW COUNTDOWN       [▓ YES ▓] [   NO   ] │
   │   AUDIBLE COUNTDOWN    [▓ YES ▓] [   NO   ] │
   │                                             │
   ├─────────────────────────────────────────────┤
   │ CANCEL   │                     │   APPLY    │
   └─────────────────────────────────────────────┘
   ▓ = blue_selected_button (current choice); the other is light_gray_button
   ```

   - The YES/NO pairs use the TWO HALVES / ONE PERIOD selector style.
   - **Row wording is shortened to one line** (PO, 2026-10-19): `SHOW COUNTDOWN` and `AUDIBLE COUNTDOWN`, because the
     page title already says "last 10 seconds". Today those keys (`show-countdown-for-last-10-seconds`,
     `audible-countdown-for-last-10-seconds`) carry a second line, `FOR LAST 10 SECONDS`. Change their **values** in
     all 15 locales; keep the key names (renaming is out of scope). In about ten locales the "last 10 seconds" part is
     woven into the phrase (de, ja, ko, zh, tr…), so those need re-wording, not trimming. Disclose them as unchecked.
   - APPLY stages the two values into the App page's edit and returns to APP OPTIONS; the App page's own APPLY saves
     them (the length-editor pattern: `ParameterEditComplete` writes into `edited_settings`). CANCEL returns to APP
     OPTIONS with this page's changes dropped. CANCEL reads BACK when nothing has changed, as on the editor pages.
   - No "?" help button (PO accepted; it would need new help text in 15 languages).
4. **Nothing new on screen about whether live sends succeed** — log only. Result uploads keep their existing
   indicator and CONNECTION STATUS page, untouched.

## Goals, non-goals, anti-goals

**Goals:** the Schedule tab shows live games from every Portal-linked hockey refbox with no operator action; one
switch turns it off; the score/stats queue behaves exactly as today.

**Non-goals:** any portal work (the portal lane owns it); Rugby/UWR; custom sites; a timeout timer; any change to
what the LED panel, overlay, overlay-bridge or wireless remote receive; a live-send status display.

**Anti-goals — a failure even if it "works":**
- A live send that delays, blocks, reorders or shares a retry path with the score/stats queue.
- Any live send to a Custom site or the UWR portal, or a custom-site event id reaching the Portal.
- An invented game number (sending for `"0"`, blank, or a guessed number).
- A non-zero score sent for a game that has not got one (the mid-break 0-0, or the old game's score under the next
  game's number).
- Sends piling up on a slow link (more than one in flight), or a stale send arriving late.
- Log spam: a failing endpoint writing a line every 5 seconds all day.

## Constraints

- **Crates:** `refbox` (setting, pages, sender) and `uwh-common` (one new method on `UwhPortalClient` in the
  std-only `uwhportal` module, beside `post_game_scores`, so the auth header stays private to the client). The
  `uwh-common` change is additive and touches no wire format, but per `.claude/rules/plan-execution.md` any
  `uwh-common` change gets the heavy process for that task. No third crate. No new dependencies.
- **Translations:** two new keys — the tile/page title (`LAST 10 SECONDS` / `COUNTDOWN`) and `LIVE UPDATES` — plus
  the two shortened row values above, in all 15 locales in the same change (`translation_consistency.rs` refuses an en-US-only key). CJK and Thai wording must
  only use characters the subset font carries, or the build hard-errors. Non-English wording drafted by Claude is
  unchecked by a native speaker and must be disclosed as such.
- MSRV 1.85, clippy `-D warnings`, no `unwrap`/`expect` without a reason, `just check` green.

## Traps already paid for (do not rediscover)

- `clock_is_running()` reports the TIMEOUT clock when a timeout exists — use `clock_state.is_running()`.
- **No tick runs while the game clock is stopped**: the updater blocks with no timer. The 5-second heartbeat needs its
  own timer, independent of the updater. Check that operator actions taken while stopped (score added, timeout
  started/ended) still produce a fresh snapshot the sender sees; if one does not, the change send must be triggered
  from that action.
- A new settings value needs `EditableSettings`, a `PageEntrySnapshot::App` entry **and its line in the
  `page_has_changes` `||` chain** (miss it and APPLY never enables for it), `revert_into`, and a commit site where the
  apply *succeeds* — every escape path (Cancel, Back, `ConfigEditComplete`, discard) must leave it uncommitted.
- Config migration: an old config with no `live_updates` key must load as `true` (test it, as
  `test_migrate_audible_countdown_defaults_false_when_absent` does for its key).
- `GameSnapshot::game_number()` (the method, `uwh-common/src/game_snapshot.rs`) already returns the NEXT game in
  `BetweenGames` once the break has reset (`!is_old_game`), and the raw `game_number` field otherwise. Before that
  reset, the raw field is the just-finished game and `next_game_number` the next. Derive the path number from the raw
  fields with all three cases pinned in tests; the court-finished case needs the raw field, because the method returns
  the blank next number there.
- Access keys are stored per **(site, event)**; event ids collide across sites. Fetch the key with the Portal's
  address, never by event alone.
- The score-confirmation pause: confirm `clock_state` reads stopped during it; if it does not, `clockRunning` must
  also be false while `time_pause_confirmation` is set.
- Tests that inject messages never reach the wire, and a test cannot observe a `Task` that `update()` returns. Pin the
  send rules in pure functions, and prove the HTTP request itself against a local server.
- Tests on this machine have no sound card: run with `--test-threads=1` if a few ALSA tests fail at random.

## Left to the builder

- Where the sender lives (a small task fed by the app's snapshots through a "latest value" channel is the obvious
  shape) and how it reads the Live Updates flag, mode, source, event and key.
- The exact request timeout (2–4 s), and the de-duplication rule for failure logging.
- Whether "changed" is detected by comparing successive built bodies plus the path game number (simplest).
- Unit-level structure of the two new pages, reusing the existing editor-page helpers.

## Verification

- **Pure-function tests** pinning: the send conditions (each condition off → no send); the path game number in each
  case (in play / break with next game / finished court / `"0"`); the body in each period, with `scores` null in
  `BetweenGames`, `SuddenDeath` counting up, and `clockRunning` false during a timeout; the one-send-then-stop rule and
  the resume when a next game appears.
- **A local-server test** (an HTTP listener in the test) proving the actual PUT: path, header, JSON body, a timeout
  that abandons, never more than one in flight, and that a stalled live endpoint does not delay a queued score post.
- **Data measurement:** byte count over a stretch of play against the dev portal once the portal lane's endpoint is
  deployed there; until then, measure against a local server and say which was used.
- **Human walkthrough (PR check 2):** the APP OPTIONS layout, the countdown page, the greyed NO in Manual/Custom/Rugby,
  and the dev Portal's Schedule tab showing a live game, once the portal side is deployed.
