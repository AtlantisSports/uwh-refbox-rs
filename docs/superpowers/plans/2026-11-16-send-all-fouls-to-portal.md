# Send all fouls to the Portal — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A BLACK/WHITE foul may be saved without a player number, it shows with no number anywhere in the refbox,
and every foul (Black, White and Equal) is sent to the Portal at the end of the game.

**Architecture:** Three small, independent changes inside `refbox`: relax the foul screen's save rule and store a
blank number as "no player"; let the summary page's shared infraction box show nothing (not "T") for a numberless
foul; drop the end-of-game foul filter and include the Equal list. No wire-type, `uwh-common` or bridge change.

**Tech Stack:** Rust 2024 (MSRV 1.85), iced 0.13, fluent translations (`fl!`).

**Spec:** `docs/superpowers/specs/2026-11-16-fouls-without-player-to-portal-design.md`

## Global Constraints

- Scope: `refbox` only. Do not touch `uwh-common`, `overlay-bridge`, `overlay`, the "=" button or the "EQUAL" wording.
- A blank keypad number on a BLACK/WHITE foul is stored as `None` — never `Some(0)`.
- Team warnings keep showing "T" (`team-warning-abreviation`); only fouls change.
- No new translation keys, no new dependencies, no new `unwrap()`/`expect()` in production code.
- Heavy jobs (any cargo build/test/clippy/check) only on the refbox coordinator's GO, with `CARGO_BUILD_JOBS=4
  RUST_TEST_THREADS=4`. The first build in this worktree is a FRESH build: announce it to the coordinator first.
- Landing gate: the PR is held unmerged until the coordinator confirms the Portal change is LIVE IN PRODUCTION.

## Review Focus

1. Editing an existing numbered BLACK foul and deleting its number to blank → saved as no player (Task 1 test pins the
   mapping; walk step covers the screen).
2. Moving an Equal foul to BLACK in the editor: the keypad starts at 0, DONE is now enabled, and it saves with no
   player (same mapping; walk step).
3. A numberless BLACK foul in the vMix `/fouls` table: verified from code that `event_rows` is shared with team
   warnings and already handles colour + no player. No change; noted so a reviewer does not re-derive it.
4. Infraction still required: "Unknown" infraction with BLACK and blank number must stay blocked (Task 1 test).
5. The end-of-game stats JSON stays valid when only Equal fouls exist (Task 3 test includes one).

---

### Task 1: Foul screen — number optional, blank stored as no player

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/foul_add.rs` (`foul_add_can_commit` + its tests; add `foul_player_number`)
- Modify: `refbox/src/app/mod.rs` (`Message::FoulEditComplete`, ~line 5281: replace the inline `player_num` mapping with a call to `foul_player_number`)

**Interfaces:**
- Produces: `pub(crate) fn foul_player_number(color: Option<GameColor>, player_num: u32) -> Option<u8>` in
  `foul_add.rs` — `None` when `color` is `None` OR `player_num == 0`, else `Some(n)`. Keep the existing
  `try_into().unwrap()` for the non-zero case (keypad max for fouls is 99, `KeypadPage::max_val`), with a comment
  saying so. Re-export it the way the module already exposes `foul_add_can_commit` if `mod.rs` cannot reach it.

- [ ] **Step 1: Update the tests in `foul_add.rs`**
  - Rename `foul_individual_needs_number` → `foul_individual_number_is_optional`, asserting
    `foul_add_can_commit(StickInfringement, Some(Black), 0)` is **true** and `(…, Some(Black), 5)` is true.
  - Add `foul_individual_without_number_still_needs_infraction`: `foul_add_can_commit(Unknown, Some(White), 0)` is false.
  - Add `foul_blank_number_is_no_player`: `foul_player_number(Some(Black), 0) == None`,
    `foul_player_number(Some(White), 7) == Some(7)`, `foul_player_number(None, 0) == None`,
    `foul_player_number(None, 7) == None`.
- [ ] **Step 2: Run, expect FAIL** (compile error: `foul_player_number` missing; old assertion flips):
  `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox foul_` (coordinator GO first).
- [ ] **Step 3: Implement** — `foul_add_can_commit` becomes `!matches!(infraction, Infraction::Unknown)`
  (keep the signature; update its doc comment: a number is never required). Add `foul_player_number`. Use it in
  `FoulEditComplete` in `app/mod.rs`.
- [ ] **Step 4: Run, expect PASS** (same command).
- [ ] **Step 5: Commit** `feat(refbox): let a foul be saved without a player number`

### Task 2: Summary page — a numberless foul shows no number

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`make_warning_container`, ~line 1886; its `mod tests` ~1968)
- Modify: call sites — `warnings_fouls_summary.rs:31` and `main_view.rs:246, 352` pass "warning";
  `warnings_fouls_summary.rs:56` passes "foul".

**Interfaces:**
- Produces: `fn infraction_who(player_number: Option<u8>, is_foul: bool) -> String` in `shared_elements.rs` —
  `Some(n)` → `"#n"`; `None` + foul → `""`; `None` + warning → `fl!("team-warning-abreviation")`.
  `make_warning_container` gains a trailing `is_foul: bool` parameter and uses it. Do not rename the function.

- [ ] **Step 1: Write tests** in `shared_elements.rs` `mod tests`: `infraction_who_numbered` (`Some(7)`, either
  flag → `"#7"`), `infraction_who_numberless_foul_is_blank` (`None, true` → `""`),
  `infraction_who_team_warning_keeps_t` (`None, false` → `fl!("team-warning-abreviation")`).
- [ ] **Step 2: Run, expect FAIL**: `… cargo test -p refbox infraction_who`.
- [ ] **Step 3: Implement** `infraction_who`, the new parameter, and the four call sites.
- [ ] **Step 4: Run, expect PASS**.
- [ ] **Step 5: Commit** `feat(refbox): show no number for a foul without a player`

### Task 3: Send every foul to the Portal at the end of the game

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — `end_game` foul loop (~line 2226) and its comment; test
  `end_game_sends_only_fully_attributed_fouls` (~line 4564); the comment at ~4551 that names that test.

**Interfaces:** none new. `GameStats::add_foul(&InfractionDetails, Option<Color>)` already serialises null
player/side.

- [ ] **Step 1: Rewrite the test** as `end_game_sends_every_foul` (same setup: one warning, a White #7 Obstruction,
  a Black no-player DelayOfGame, an Equal FreeArm). Assert: no `"warning"` events; exactly 3 `"foul"` events;
  one with `side == "light"`, `playerCapNumber == 7`, `called == "Obstruction"`; one with `side == "dark"`,
  `playerCapNumber` null, `called == "DelayOfGame"`; one with `side` null, `playerCapNumber` null,
  `called == "FreeArm"`. Update the ~4551 comment to the new test name.
- [ ] **Step 2: Run, expect FAIL** (only 1 foul sent): `… cargo test -p refbox end_game_sends_every_foul`.
- [ ] **Step 3: Implement** — loop over `[Some(Color::Black), Some(Color::White), None]`, add every foul in
  `self.fouls[color]` with that `color`, no player check. Replace the held-back comment with one saying all
  fouls are sent and the Portal accepts null player/side (uwhportal#564).
- [ ] **Step 4: Run, expect PASS**; also run `… cargo test -p refbox running_stats` (the 4551 neighbour still passes).
- [ ] **Step 5: Commit** `feat(refbox): send fouls without a player or team to the portal`

### After the tasks (main session)

- The spec and this plan are staged in Task 1's commit (no separate docs commit).
- `just check` (coordinator GO). Pre-PR: `.claude/rules/pr-review.md` — `code-review` skill on the branch diff,
  PO walkthrough (player foul; BLACK foul with no number; Equal foul; edit a numbered foul to blank; check the foul
  list AND the summary page), dangling-work sweep. Leg 3 (wire to a Portal) impossible until the Portal change
  exists — the Task 3 test stands in. Then HOLD for the Portal-live confirmation before any merge.

## Deviations

(none yet)
