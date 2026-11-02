# Stream Manager: goal, penalty and foul timestamps in each game's video description

**Status:** draft for approval (not started)
**Crate:** `stream-manager` only
**Process:** lean (stream-manager feature work, see `.claude/rules/plan-execution.md`). It doesn't
touch `uwh-common`, the refbox, the overlay or the wire format.

## Goal

After each game, Stream Manager adds a list of clickable timestamps to that game's YouTube video
description. The list covers every goal, penalty and foul, so viewers can jump straight to them.

Example of what a viewer sees at the end of the description:

```
Game highlights
0:00 Start
3:41 Goal — White #7 (White 1–0 Black)
9:12 Penalty — Black #3, 1 min (Obstruction)
12:55 Foul — White #4 (Stick Foul)
24:30 Goal — Black #11 (White 1–1 Black)
Final score: White 1–1 Black
```

## Decisions (agreed with the tournament organiser and Eric, 2026-10-29)

1. **What is listed:** goals, penalties and fouls. Warnings, team timeouts and penalty-shot
   periods are not listed.
2. **Who is named:** team colour and cap number only. No player names.
3. **When it's written:** once, after the game. Nothing is written during the game.
4. **Where in the video each line points:**
   - Normally: **15 seconds before the moment the scorekeeper entered it** on the refbox.
   - If it was entered **during a referee timeout, or within 10 seconds after one ended (was 60 s; changed to 10 s on 2026-11-02)**: **15
     seconds before that referee timeout started**. This applies to goals, penalties and fouls
     alike.
   - Team timeouts never count.
   - The 15 seconds is fixed, not a setting.
   - A timestamp never goes before the start of the video. It stops at 0:00.
5. **A penalty names its foul:** "Penalty — Black #3, 1 min (Obstruction)". The length is the
   penalty's time when it first appears, rounded up to whole minutes. A total dismissal shows as
   "dismissal". (Known limit: if Stream Manager first sees a penalty part-way through, e.g. after
   a restart, the length can read short.) Scorekeepers enter only the penalty when a foul leads
   to one, so there is no separate foul line for it.
6. **Corrections:**
   - A goal taken back on the refbox (the score goes down) removes that team's latest goal.
   - A foul removed from the refbox's list is removed.
   - A penalty that disappears **within 30 seconds** of being added is treated as a mistake and
     removed. One that disappears later has simply been served, and stays.

## Acceptance criteria (what the organiser can check)

1. **Practice run with the refbox simulator** (or a real refbox), on the **dev portal**, with
   **unlisted** videos:
   - Enter 2 goals, 1 penalty and 1 foul; one goal during a referee timeout; delete one foul.
   - After the switch to the next game, the finished game's YouTube description ends with the
     "Game highlights" list: 2 goals, 1 penalty (with its foul) and 0 fouls (the deleted one is
     gone).
   - Clicking each time lands about 15 s before the moment it was entered. The goal entered during
     the referee timeout lands about 15 s before the timeout started.
2. **Restart safety:** closing and reopening Stream Manager mid-game doesn't lose the events entered
   before the restart.
3. **Nothing else changes:** title, next-game link, thumbnail and playlist behave exactly as
   before. A later title update (team names changing) keeps the highlights list.
4. **End day:** the last game of the day also gets its list when End day is pressed.
5. `just check` passes. Each rule in "Decisions" has an automated test.

## How it works (architectural sketch)

### 1. Recording events while the game runs (new: `src/highlights.rs`)

Stream Manager already receives every refbox snapshot for each court (`refbox.rs`,
`GameSnapshot`). A new per-court **tracker** compares each snapshot with the previous one:

| Change seen in the snapshot | Recorded as |
|---|---|
| `scores[colour]` goes up | Goal, colour, cap from `recent_goal` (if it matches the colour) |
| `scores[colour]` goes down | Remove that colour's latest goal |
| New entry in `penalties[colour]` (matched by player number + infraction) | Penalty, cap, infraction, length |
| Penalty entry gone within 30 s of being added | Remove it |
| New entry in `fouls` | Foul, colour (if any), cap (if any), infraction |
| Foul entry gone | Remove it |
| `timeout` becomes `Ref(_)` / stops being `Ref(_)` | Referee timeout start / end time |

- Times are taken from **Stream Manager's own clock** when the snapshot arrives (the mini PC's
  clock), not from the refbox. The refbox Pi's clock doesn't matter.
- Each event belongs to the snapshot's own `game_number`, never "whatever is live". A late
  correction to the previous game still lands on the right video.
- The **anchor time** is worked out when the event is recorded (rule 4): the referee timeout start
  if it applies, otherwise the entry time.
- **No foul/penalty merging.** The scorekeepers enter only the penalty when a foul leads to one,
  and in the refbox a penalty doesn't also create a foul (separate entry screens). So each penalty
  appears once, and it already names its foul.

### 2. Saving as it goes (`prepare.rs`, `VideoState`)

- `VideoState` gets `#[serde(default)] highlights: Vec<Highlight>`. Each `Highlight` holds the
  kind, colour, cap, infraction, length and anchor time.
- The record is saved after each change, the same way as the existing fields (`update_state`), so a
  restart keeps them. The open referee timeout is kept in memory only. After a restart mid-timeout
  the rule falls back to the entry time, which is acceptable.
- Older event records without the field load as an empty list.

### 3. Writing the description after the game (`live.rs`)

- **When:**
  - In the switch, after the old video has ended, alongside the existing "Next game" link step
    (`add_next_link`). It's one description update covering both.
  - In End day, after the last video ends.
- **Video start time:** YouTube's `actualStartTime` for the finished video. That's one cheap read
  (1 unit), and it's exactly where the video's 0:00 is. The offset of each line is the anchor time
  minus 15 s minus the start, with a minimum of 0.
- **Quota:** one update per game (50 units), counted as an "extra" like the next-game link, so it's
  skipped when the allowance is low. It's never done before the switch itself.
- If there are no events, nothing is added (no empty "Game highlights" heading).

### 4. Keeping the list when descriptions are rebuilt (`prepare.rs`, `title_sync.rs`)

- Today the description on YouTube is `with_next_link(generated, next_game_link)`. That becomes one
  composer: **generated text, then the next-game link, then the highlights block**.
- It's used everywhere a description is compared or written: `needed_update`, title sync and
  `add_next_link`. So Prepare and the 10-minute title check never wipe the list.

### 5. Format details

- Times are `M:SS` under an hour and `H:MM:SS` from an hour, as YouTube expects.
- The `0:00 Start` line makes YouTube show chapters when there are 3+ lines, each at least 10 s
  apart. When lines are closer, YouTube just shows clickable times, which is also fine.
- YouTube's description limit is 5,000 characters. If a very long list would pass it, foul lines
  are left out (goals and penalties are always kept), and a "(more fouls not shown)" line is
  added.
- Angle brackets are stripped, as for the other description text.

## Tasks (rough)

1. `highlights.rs`: the tracker (snapshot diffing, referee-timeout window, corrections) and the
   line formatter, with unit tests for every decision above.
2. Feed the tracker from the existing per-court refbox loop. Persist to `VideoState.highlights`.
3. The description composer used by `needed_update`, title sync and `add_next_link`, plus tests
   that a title update keeps the list.
4. Write the list in the switch and in End day (one update together with the next-game link). Read
   `actualStartTime`.
5. Practice-mode logging: in practice mode, log the list that *would* be written instead of
   updating YouTube.
6. Manual walkthrough (acceptance criteria 1–4) on the dev portal, unlisted. Then `just check` and
   a PR for Eric with a plain-English description.

## Out of scope

- Any refbox, `uwh-common` or overlay change. The "referee buzzer" idea is parked.
- Player names (cap numbers only, for now).
- Updating the list during the game.
- Fixing timestamps after a video is trimmed at the start in YouTube Studio. Editing the
  description by hand still works.
- The "one playlist per event" branch (separate PR, after the tournament).

## Deviations

- **Where the list is kept:** in `EventState.highlights` (game number → list), not in `VideoState`.
  That way a game's list doesn't depend on its video being recorded. `VideoState` gets
  `highlights`, the block as written on YouTube, so title updates keep it.
- **Video 0:00 (found in the 2026-10-30 practice run):** YouTube's `actualStartTime` comes several
  seconds (~8 s) after the recording starts, so every link landed ~23 s before the entry instead
  of 15. Stream Manager now saves the moment it asks YouTube to go live (`VideoState.live_requested`)
  and uses that as 0:00. `actualStartTime` is the fallback for older records.
- **Running score:** each goal line shows the score after it, but only when every goal of the
  game was seen. Otherwise only the final score is shown.
- **Log lines:** each goal, penalty or foul noted shows in the court's log ("Game 5: noted Goal —
  White #7").
- Lists with nothing in them (e.g. the refbox's placeholder game "0") aren't saved.
- **Leftover lists (found 2026-11-02):** after test videos were deleted and made again, game 1's
  new description listed a penalty from the old test at 0:00. Now:
  - Test tools → Delete clears the saved lists;
  - Prepare clears a game's list when it makes a new video for it;
  - items entered more than 30 minutes before a video's 0:00 aren't listed.
- **Refbox reset:** if everything (score, penalties, fouls; at least two items) disappears at once,
  it's treated as a reset, not as corrections. The list is kept, and play carries on from the
  cleared state.
- **Verified 2026-11-02:** with the saved go-live moment as 0:00, links land 15 s before each entry
  (and 15 s before the referee timeout's start for items decided in one).
