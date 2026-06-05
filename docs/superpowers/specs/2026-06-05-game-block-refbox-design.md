# Design: Game Block (refbox side of ADR-008, refined)

Date: 2026-06-05
Crates: `uwh-common` (data model + migration + portal decoder), `refbox` (scheduling, UI, validation, main-screen indicator)
Status: approved (brainstorm)
Builds on: ADR-008 (`docs/decisions/008-game-block.md`, status proposed)

## Summary

Introduce **Game Block** — the authoritative, start-to-start duration of a game's slot — as the
operator-facing scheduling parameter, replacing "Nominal Break Between Games" in the refbox UI.
This implements the **full refbox side of ADR-008**, refined by two decisions made during
brainstorming:

1. **Always show Game Block** (no ADR-008 "old-world / new-world" dual UI). The refbox always
   derives and displays Game Block; the "Nominal Break Between Games" button is retired.
2. **Portal is prepared-for, not required.** The portal `TimingRule` decoder gains an *optional*
   `game_block` field; when present it is authoritative, otherwise the refbox derives Game Block
   locally. The portal does not send Game Block today (confirmed in the `uwh-portal` repo: its
   timing model still uses `nominalBreak` / `minimumBreak`), so this round ships no portal change.

## Definitions

- **Regulation play** — the game's playing time excluding overtime:
  - Two-period game: `2 × half_play + half_time`
  - Single-period game (`single_half`): `half_play`
- **Game Block** — the start-to-start slot duration. Authoritative, operator-edited, stored.
- **Derived break** — `Game Block − regulation play`. No longer an independent setting.
- **Math minimum** — the smallest valid Game Block: `regulation play + minimum_break`.
- **Buffer** — `Game Block − math minimum`. The slack a game has before it eats the minimum break.

## Behaviour

### 1. Game Block as the authoritative cadence

- The operator edits **Game Block** directly. It is a fixed cadence: changing the half length
  (or 2 Halves / 1 Period) leaves the Game Block number unchanged and flexes the *derived break*
  instead. The next game's scheduled start does not move when play durations change.
- The **Minimum Break Between Games** setting is unchanged — it remains a separate hard floor on
  the actual gap between games.

### 2. Scheduling math

- The next game's scheduled start = this game's scheduled start **+ Game Block**. This replaces
  the current `sched_start + regulation_play + nominal_break` expression in
  `refbox/src/tournament_manager/mod.rs` (`start_game`, and the `#[cfg(test)]` `set_game_start`
  helper).
- `calc_time_to_next_game` is otherwise unchanged: it still derives the between-games countdown
  from `next_scheduled_start` with `minimum_break` as the floor.

### 3. Data model and migration (`uwh-common`)

- `GameConfig`/`Game` gains a `game_block: Duration` field, which becomes the authoritative
  scheduling value. `nominal_break` stops being an independent operator setting and becomes a
  derived/legacy value (kept on the struct per ADR-008; not exposed in the UI).
- **Migration** (`Config::migrate`, invoked from `refbox/src/main.rs` on first load): a config
  written by an older version has no `game_block`. Convert it so the prior cadence is preserved:
  `game_block = regulation_play + nominal_break` (using that config's half/half-time/single_half).
  The migrated config is rewritten to disk; the operator sees their existing cadence under the
  new name with no manual action.
- **Portal decoder** (`uwh-common/src/uwhportal/schedule.rs`): `TimingRule` gains an optional
  `game_block` field. If present, it is used as the authoritative Game Block; if absent (today's
  case), the refbox derives Game Block from the rule's existing fields. No wire-format change is
  forced (additive optional field).

### 4. Game Block button + editor validation (Game Options)

The "Nominal Break Between Games" button is replaced by a **Game Block** button. The button and
its value editor use warning-only colouring (normal grey when healthy), and the colour updates
live as the operator types in the editor:

- **RED, Done disabled:** Game Block is below the **math minimum** — it can't fit the game plus
  the minimum break. The value cannot be saved.
- **YELLOW, Done enabled:** Game Block meets the minimum but the **buffer is smaller than both
  teams' full team-timeout allotment**, so timeouts could push games past their slot. Saving is
  allowed; the operator is warned.
  - Both teams' full team-timeout allotment = `2 (teams) × timeouts_per_game × team_timeout_duration`,
    where `timeouts_per_game = num_team_timeouts_allowed`, doubled when `timeouts_counted_per_half`
    over a two-period game (one period → counted once). Referee timeouts are excluded (unbounded).
- **No colour (grey):** buffer ≥ that allotment. Visually identical to the other timing buttons.

### 5. Main-screen overrun indicator

A silent-by-default indicator on the **right side of the time bar** on the main game screen:

- **Overrun** = wall-clock time the current game has lost to stoppages (real time elapsed minus
  game-clock time consumed) — only what has *already* happened, no projection of future timeouts.
- While `overrun ≤ buffer`: the indicator is **not shown**.
- When `overrun > buffer`: show **`-M:SS` in red** = `overrun − buffer`, i.e. how far this game
  has dug into the minimum-break cushion. Format matches the game clock (e.g. `-1:15`).

## Scope

**In scope**
- `uwh-common`: `game_block` field on the game config; `Config::migrate` conversion; optional
  `game_block` on the portal `TimingRule` decoder (accept-when-present).
- `refbox`: scheduling switched to Game Block; Game Options button + editor with red/yellow
  validation; info displays ("time between games" line) show Game Block; main-screen overrun
  indicator; `LengthParameter` rename (`NominalBetweenGame` → `GameBlock`).

**Out of scope**
- Any portal-side (uwh-portal repo) change to *send* Game Block — prepared for, not done.
- `schedule-processor` changes (ADR-008 notes the `occupied_time` helper could later share a
  `game_block_minimum()`; not required here).
- Removing the legacy `nominal_break` field from `GameConfig` (kept for compatibility).
- `wireless-remote`.

## Open items deferred (noted, not blocking)

- **Portal sending an invalid Game Block** (below its own minimum): not reachable this round
  (portal sends no Game Block). When the portal eventually sends one, decide refuse/warn/cap.
  For now the decoder simply uses a present value as authoritative.

## Blast radius & process

High. Touches `uwh-common` (shared types, migration, portal decoder) and the `refbox`
tournament-manager scheduling (state/timing). Per `.claude/rules/plan-execution.md` this is
**heavy process**: per-task verification, tests for the math (migration, scheduling, validation
thresholds, overrun), and careful review. Downstream crates that consume `GameConfig`
(`refbox`, `schedule-processor`, `overlay`, `led-panel-sim`) must still build.

## Dependency on Change 1

The Game Block math minimum uses the 2-Halves / 1-Period distinction (`single_half`), and the
scheduling rewrite supersedes Change 1's `start_game` single-period timing fix. Therefore Change 2
should branch off `master` **after Change 1 (PR #1002) merges**, so it builds on the `single_half`
field and timing fix and rewrites that expression once. (If implemented before #1002 merges,
expect a small merge conflict in `start_game`.)

## Acceptance criteria (operator-observable)

1. Game Options shows a **Game Block** button (no "Nominal Break Between Games"); its value equals
   the old `playing time + nominal break` for a migrated config.
2. Editing Game Block below the math minimum turns it **red** and disables **Done**; a tight value
   turns it **yellow** but is savable; a comfortable value is grey.
3. Changing the half length leaves the Game Block number unchanged (the derived break absorbs it),
   and the next game's scheduled start is `this start + Game Block`.
4. During a game, after enough stoppage time to exceed the buffer, a red `-M:SS` overrun figure
   appears on the time bar and grows; it is absent otherwise.
5. An older config file loads and is silently migrated to an equivalent Game Block.
6. `just check` passes; downstream crates build; new unit tests for migration, scheduling,
   validation thresholds, and overrun all pass.
