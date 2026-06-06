# Design: Persistent "Behind Schedule" indicator + App-Options toggle

Date: 2026-06-06
Crates: `refbox` (tournament-manager timing read-model, config setting, App-Options UI, main-screen
indicator), translations.
Status: approved (brainstorm)
Builds on: the Game Block feature (`docs/superpowers/specs/2026-06-05-game-block-refbox-design.md`,
implemented on branch `feat/uwh-common/game-block`). This reworks that feature's main-screen overrun
indicator (Phase E1) into a persistent, schedule-aware figure and adds an admin on/off setting.

## Summary

Replace the per-game overrun figure on the main time bar with a **persistent "behind schedule"
figure**: a quiet red `-M:SS` showing how far behind its scheduled start times the run of games
currently is. It carries across games and between-games breaks, is reduced as breaks claw delay
back, and is hidden when the games are on time or ahead. A new saved setting, **"Show Behind
Schedule Time"** (default On), lets admins turn the figure off; it lives as a half-width on/off
button on the App Options page.

This is a refbox-only, read-and-display change. It does **not** change how games are scheduled, the
Game Block math, the red/yellow editor validation, the config wire format to the LED panel/overlay,
or the portal data model. It reads timing the box already tracks and adds one display + one setting.

## Definitions

- **Scheduled start of a game** — when the game is *planned* to begin:
  - **Portal/CSV schedule loaded:** the printed start time for that game (the same source the
    between-games countdown already uses, `calc_time_to_next_game` via the scheduled game's
    `start_time`).
  - **Manual mode (no schedule):** the first game's actual start time plus one **Game Block** per
    game thereafter — i.e. the existing `next_scheduled_start` grid the tournament manager maintains
    (`start_game` sets `next_scheduled_start = previous_scheduled_start + game_block`).
- **Minimum break** — the existing hard floor on the gap between consecutive games
  (`config.minimum_break`). A game can never start sooner than this after the previous one, so it is
  always enforced and is always part of the claw-back calculation.
- **Scheduled break of a game** — the planned gap before it = its scheduled start minus the previous
  game's scheduled end. In manual mode this equals `Game Block − regulation play`.
- **Slot slack (claw-back capacity) of a break** — `scheduled break − minimum break`. The most a
  single break can recover. In manual mode this equals the Game Block buffer
  (`game_block − regulation play − minimum break`).
- **Behind-schedule amount** — how much later than its scheduled start a game actually starts (or is
  forced to start), carried forward; never negative (on-time/ahead reads zero).

## Behaviour

### 1. The figure on the main time bar

- A red `-M:SS` at the right end of the time banner (the placement already chosen for the Game
  Block overrun indicator), formatted like the game clock.
- Shown whenever the behind-schedule amount is greater than zero, **during games and between
  games**. Hidden (renders nothing) when the amount is zero — i.e. on time or ahead.
- Gated by the new **Show Behind Schedule Time** setting: when Off, never shown.

### 2. How "behind schedule" accumulates and recovers

Worked example (Game Block 48:00 → games planned 48 minutes apart in manual mode):

1. Game 1 starts on time → nothing shown.
2. Game 1's stoppages push Game 2's actual start 6 minutes late → throughout Game 2 the bar shows
   `-6:00`.
3. Game 2 also runs 2 minutes long → by Game 3 it shows `-8:00`.
4. Game 4 is quick and the following game starts 3 minutes earlier than planned → drops to `-5:00`.
5. …until a game starts on or ahead of plan → the figure disappears.

A **deliberately longer scheduled break** (e.g. before lunch) pushes that game's scheduled start
later, so an existing delay is absorbed by it: if the break's slot slack covers the delay the next
game starts on time and the figure clears; otherwise only the leftover carries forward. The
recovery is always capped so the gap never drops below the **minimum break**.

### 3. Calculation details (for implementation)

`behind_schedule(now) -> Duration` on the tournament manager, returning `ZERO` when off-schedule
tracking does not apply (before the first game / no anchor) or when on-time-or-ahead.

Let `sched_start(g)` be the scheduled start of game `g` and `sched_next` the scheduled start of the
next game.

- **During a live game N** (current period is a play or break period of a game in progress):
  - `inherited = saturating(actual_start_N − sched_start(N))` — lateness carried in at this game's
    start (captures all prior accumulation).
  - `developing = saturating(accumulated_overrun(now) − slot_slack_N)` — the part of this game's
    *already-elapsed* stoppage time that has eaten past its slot's slack and will push the next game
    late. `accumulated_overrun` is the existing helper (real time elapsed since game start minus
    scheduled game-clock time consumed); `slot_slack_N = (sched_next − sched_start(N)) − regulation
    play − minimum_break` (in manual mode = `game_block_buffer`).
  - `behind = inherited + developing`.
- **Between games** (waiting for the next game to start):
  - `behind = saturating( max(now, game_end + minimum_break) − sched_next )` — the next game cannot
    start before `game_end + minimum_break` (hard floor, already determined); once `now` passes that
    without the game starting, the overdue time grows; a far-out `sched_next` (long break) keeps this
    at zero (recovered).
- The two cases **agree at the game-end boundary** (substituting `game_end = actual_start_N +
  regulation + overrun` into the between-games formula yields `inherited + (overrun − slot_slack)`),
  so the figure is continuous across the transition.
- No projection of *future* play time is used — only what has already elapsed, plus the
  always-enforced minimum break.

The reading is computed in the refbox view layer each render (cheap), from live tournament-manager
state + `now` + config; **no serialized `GameSnapshot`/wire change**.

### 4. Data the tournament manager must expose

- The **scheduled start of the current game** (`sched_start(N)`): recorded when the game starts.
  - Manual mode: `next_scheduled_start − game_block` at start (the grid slot the game occupies).
  - Portal/CSV mode: the started game's printed `start_time` (the value `calc_time_to_next_game`
    already derives for the countdown). The current game's scheduled start must be retained at
    `start_game` time (today `next_game` is consumed on start), e.g. in a
    `current_scheduled_start: Option<Instant>` field.
- The **scheduled start of the next game** (`sched_next`): the same source the between-games
  countdown uses — the next scheduled game's printed `start_time` if present, else
  `next_scheduled_start`.
- **Game end time** for the between-games case (when the current game finished). The manager already
  tracks clock state at game end; expose or reuse it.

(Exact field plumbing and the `Instant` vs portal-`OffsetDateTime` conversion are left to the
implementation plan; the plan must reuse `calc_time_to_next_game`'s existing portal-time handling so
the two stay consistent.)

### 5. The "Show Behind Schedule Time" setting

- New persisted boolean on the refbox app config (alongside `track_fouls_and_warnings`,
  `hide_time`, etc.), **default `true`**, with the standard `Config::migrate` `get_boolean_value`
  handling so existing config files adopt it (defaulting On).
- Surfaced as a half-width on/off value button on the **App Options** page
  (`make_app_config_page`), in the first currently-empty row, styled and wired like the existing
  `track-fouls-and-warnings` / `confirm-score-at-game-end` toggles (a new `BoolGameParameter`
  variant + `ToggleBoolParameter` handling + apply-to-config).
- Label **"Show Behind Schedule Time"** (title case, matching sibling labels), in all 15 locales.

## Scope

**In scope (all in `refbox` + translations):**
- `behind_schedule(now)` read-model on the tournament manager + the `current_scheduled_start` (and
  any game-end) plumbing it needs; unit tests.
- New `show_behind_schedule_time` config bool + migration + default On.
- `BoolGameParameter` variant + toggle wiring + App-Options button.
- Main-screen indicator switched from the per-game overrun figure to `behind_schedule`, gated on the
  setting; shown during games and between games.
- Translations for the new label in 15 locales.

**Out of scope:**
- Any change to scheduling math, Game Block, the red/yellow editor validation, or the between-games
  countdown logic itself.
- Any `uwh-common` wire-format / `GameSnapshot` / portal-data change.
- `schedule-processor`, `overlay`, `led-panel-sim` behaviour; `wireless-remote`.
- Showing an "ahead of schedule" figure (explicitly not wanted).
- Exact tracking of *irregular* portal gaps beyond what the scheduled start times already encode
  (the printed start times already carry deliberate long gaps, so this is handled; nothing extra).

## Acceptance criteria (operator-observable)

1. With **Show Behind Schedule Time** On, a run of games that falls behind shows a growing red
   `-M:SS` on the main time bar that **persists across games and breaks**, and is **absent** when on
   time or ahead.
2. A game that starts late shows the inherited lateness for its whole duration; a long stoppage
   grows the figure live; a short game / compressible break **reduces** it; a game starting on/ahead
   of plan clears it.
3. A deliberately longer scheduled break absorbs delay (figure reduces or clears), never compressing
   the gap below the **minimum break**.
4. The **App Options** page shows a half-width **Show Behind Schedule Time** on/off button; turning
   it Off hides the figure entirely; the setting persists across restarts and is adopted by an older
   config file (defaulting On).
5. `just check` passes; new tournament-manager unit tests for inherit / grow / recover / zero-when-
   ahead pass; downstream crates build.

## Blast radius & process

Moderate–high: touches the tournament-manager timing read-model (a new read-only method + a small
amount of recorded state) and the refbox config (a new persisted setting + migration). No wire
format or scheduling-math change. Per `.claude/rules/plan-execution.md`, the tournament-manager
read-model and the config/migration get **per-task verification with real unit tests**; the UI
button/translation wiring uses build + `just check` + manual.
