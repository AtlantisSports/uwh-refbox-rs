# Game-Info Table Redesign — Design Spec

- **Date:** 2026-06-17
- **Status:** Draft for user review
- **Crate scope:** `refbox` only
- **Planned branch:** `feat/refbox/game-info-table` (off fresh `origin/master`, tip `249de8ff` at time of writing)
- **Origin:** Rebuild of the legacy reference branch `origin/uwh-refbox-game-info-layout` (design reference only — see Audit below)

---

## 1. Goal

Replace the free-text game-information display with a **table** that re-presents the same
game data in a labelled, scannable grid, and make the whole table a tap target that jumps
straight to **Game Options**. The table appears on two surfaces:

1. **Game Information page** — the full table (the dedicated page reached from the main view).
2. **Main UI page** — a condensed version of the same table, replacing today's text list in
   the centre panel.

This is a **content + presentation** redesign (not a pure text-to-table port): it introduces
state-dependent game blocks (Last / Current / Next), team-name rows with scores, and a
referee section.

### Acceptance criteria (observable)

- On the Game Information page, game data renders as a grid of labelled cells, not a text blob.
- The two game blocks shown depend on game state (see §4.1): **Last + Current** between games,
  **Current + Next** during a game.
- Team rows are colour-coded (light = white team, dark/black background with white text = black
  team) and show scores per §4.3.
- Tapping anywhere on the table opens the **Game Options** settings page directly.
- The main UI page shows the condensed table (no Water referees / no Helper; Chief + Time/Score
  Keeper only) in place of the old text list, and still collapses to game-numbers-only when 4+
  warnings crowd the panel.
- All new labels are translated in every locale (no English placeholders).
- `just check` passes; behaviour-level tests cover the row model and state-dependent block
  selection (see §6).

---

## 2. Audit of the legacy reference branch

`origin/uwh-refbox-game-info-layout` (tip `3784e892`) is **reference only — do not merge/rebase**:
~9 months old, 515 commits behind master, 84 files / +17.7k−1.5k of mostly unrelated/superseded
changes, plus a stray 13 MB `rustup-init.exe`.

Its game-info idea matches ours: a `TableRow { left_label, left_value, center_label, center_value }`
vector rendered as rounded-box cells, with the whole table wrapped in a `button` →
`Message::EditGameConfigPage(ConfigPage::Game)`. We reuse the **concept**, not the code, because the
legacy implementation:

- Hardcoded English strings (no translations).
- Used fragile fixed pixel widths.
- Showed stale/placeholder data (hardcoded "Unknown" referees; predates Game Block, single-period
  handling, and real referee-name resolution that master now has).
- Wired to message/state names that have since been renamed.

---

## 3. Current state on master (what we are replacing)

- **Game Information page** — `refbox/src/app/view_builders/game_info.rs`,
  `build_game_info_page` (dispatched from `mod.rs` `AppState::GameDetailsPage`). Builds two
  newline-joined strings via `details_strings(...)`: left = game settings, right = referee list
  (portal-only), rendered as two `text()` columns. Footer: Back / Refresh-or-spacer / Settings.
- **Main UI page** — `refbox/src/app/view_builders/main_view.rs` (~line 256). Centre panel shows
  `config_string(...)` as a tappable text list (→ `Message::ShowGameDetails`) when
  `max_num_warns < 4`; otherwise it shrinks to `config_string_game_num(...)` (game numbers only)
  to make room for the warnings panel. `config_string`/`config_string_game_num` live in
  `shared_elements.rs`.
- **Navigation** — footer "Settings" fires `Message::EditGameConfig`, which initialises
  `edited_settings` and lands on `AppState::EditGameConfig(ConfigPage::Main)` (the settings menu).
  Landing on the **Game Options** sub-page is `ConfigPage::Game`. `Message::EditGameConfig` has
  exactly **one** call site today (the footer button), so changing how it lands is low-risk.
- **Data available now:** team names (`game.dark` / `game.light` via schedule, portal-only), all
  game-config durations (incl. `game_block`, `single_half`), the per-game timing rule
  (`schedule.get_game_timing`), live scores (`snapshot.scores: BlackWhiteBundle<u8>`), the
  just-finished game's scores (`tm.last_game_info()`), and referee assignments
  (`game.referee_assignments`: flat `Vec<{role, user_id, display_name}>`; roles seen today are
  `Chief`, `TimeOrScoreKeeper`, `Water1`, `Water2`, `Water3`).
- **Data NOT available now:** **Game Type** (no field; only Division/Pod grouping exists) and a
  **Time/Score Helper** role string. Both are deferred to a Portal-side task (see §8).

---

## 4. The table design

One shared row model + renderer, two density variants ("full" for the Game Information page,
"compact" for the main UI page). The renderer draws a 4-column grid; some rows span columns.

### 4.1 Game blocks (state-dependent — always exactly two)

The **Current** game block + settings grid + referees is the anchor and is always shown. One
**context** block flips with game state:

- **Between games (pre-game):** `Last Game` block **above** the Current block. Current block =
  the upcoming game whose config is loaded.
- **During a game (in-game):** `Next Game` block **below** the referees. Current block = the
  game in progress.

Ordering template (a bracketed block shows only in its state):

```
[ Last Game block ]        <- only between games
  Current Game block
  Settings grid
  Referees
[ Next Game block ]        <- only during a game
```

Per-block contents (4-column grid; team rows on the right span the two label columns on the left):

| Block | Left col (row 1) | Left col (row 2) | Right (row 1) | Right (row 2) |
|-------|------------------|------------------|---------------|---------------|
| Last Game | `Last Game:` + number | *(blank — no Game Block for prior game)* | White team name + score | Black team name + score |
| Current Game | `Current Game:` + number | `Game Block:` + value | White team name + score | Black team name + score |
| Next Game | `Next Game:` + number | `Game Block:` + value | White team name *(no score)* | Black team name *(no score)* |

- Game number = the schedule's display number for that game (same resolution as today, incl.
  the `game-number-error` / `None` fallbacks).
- "White" row = light team (`game.light`), light background. "Black" row = dark team
  (`game.dark`), black background with white text.
- Team **names** come from the schedule (portal). When not using the portal (no names), the row
  still renders colour-coded with its score but with an empty / neutral name cell. *(Detail to
  confirm in walkthrough: blank vs. a generic "White"/"Black" label.)*

### 4.2 Settings grid (belongs to the Current game)

Ordered list of label/value items, flowed **two per row** (4-column grid). Conditional items
hide exactly as today, so the grid reflows when they are absent:

1. Half Length | Half-Time Length *(single-period game: show "Game Length", hide Half-Time)*
2. Timeouts (count + `/Half` or `/Game`, or `0`) | Team Timeout Duration *(hidden if timeouts = 0)*
3. Overtime Allowed | Sudden Death Allowed
4. *(if Overtime)* Pre-Overtime Break | *(if Sudden Death)* Pre-Sudden-Death Break
5. *(if Overtime)* Overtime Half Length | Minimum Game Break
6. *(if Overtime)* Overtime Half-Time Length | Stop Clock in Last 2 Min

(`Game Block` is NOT in this grid — it lives in the Current block's left column, §4.1.)
The exact pairing above reflects the all-features-on case from the mockup; when conditional
items drop out, items re-pair in order. Final on-screen pairing confirmed in the walkthrough.

Stop Clock in Last 2 Min reads `Unknown` when no timing rule is available (unchanged from today).

### 4.3 Scores

- **Current** block: live score from `snapshot.scores` (white→light row, black→dark row).
- **Last** block: the just-finished game's final score from `tm.last_game_info()`. Requires
  threading those scores into the view (small plumbing addition — see §5).
- **Next** block: **no score** (game has not happened).

### 4.4 Referees

Full-width single rows (label left, name right; `-` for an assigned-but-unnamed slot, as today).

- **Full page:** Chief Referee, Time/Score Keeper, **Time/Score Helper** *(row shown only when the
  portal sends that assignment)*, Water Referee 1, Water Referee 2, Water Referee 3.
- **Main UI page (compact):** **Chief Referee + Time/Score Keeper only** (no Helper, no Water).
- Referees are portal-only (no portal → section omitted), unchanged from today.

### 4.5 Game Type

**Removed from the table** for now (no data source). No row, no placeholder. Recorded as a Portal
follow-up (§8) for possible future re-introduction.

---

## 5. Architecture sketch (Approach A — typed row model + shared renderer)

- **New module** (e.g. `refbox/src/app/view_builders/game_info_table.rs`): a typed description
  of the table and a renderer.
  - A row model, e.g. an enum of row kinds: `GameBlockHeader { label, number }`,
    `GameBlockBlock { team_name, color, score: Option<u8> }`, `SettingPair { left, right }`,
    `Referee { label, name }`, etc. (Exact shape decided in the plan; the point is data is
    separated from iced widgets so it is unit-testable.)
  - A builder `fn game_info_rows(snapshot, config, using_uwhportal, schedule, teams,
    last_game_scores, variant: Variant) -> Vec<Row>` that encodes §4 (state selection, conditional
    settings, referee variant). Pure/testable.
  - A renderer `fn render_rows(rows) -> Element<Message>` that draws the 4-column grid using
    existing theme styles (`light_gray_button`, container rounded-box, `SMALL_TEXT`). **No fixed
    pixel widths** — use `Length::Fill` / `FillPortion`.
- **`game_info.rs`:** `build_game_info_page` calls the builder with `Variant::Full`, wraps the
  rendered table in a single `button(...).on_press(<go to Game Options>)`, keeps the existing
  footer.
- **`main_view.rs`:** the centre panel calls the builder with `Variant::Compact`, keeps its
  existing tap (`Message::ShowGameDetails`) and the `max_num_warns >= 4` → game-numbers fallback.
- **Navigation:** add the ability to land directly on `ConfigPage::Game` from the table tap
  (e.g. parameterise `Message::EditGameConfig(ConfigPage)` — only one existing call site — or add
  a sibling message that shares the same `edited_settings` initialisation). Footer "Settings"
  continues to land on `ConfigPage::Main`. Exact message shape decided in the plan.
- **View plumbing:** thread the just-finished game's scores (`tm.last_game_info()`) into the view
  dispatch (`mod.rs`) so the Last block can show final scores. Live current scores already arrive
  via the snapshot.
- **Translations:** new keys for `Last Game` / `Current Game` / `Next Game`, referee labels
  (`Time/Score Helper` etc. as needed), and any table-fit-shortened labels — added to **all 15
  locales** with real translations. Where existing keys already carry the right label, reuse them.

The old `details_strings` (game_info.rs) and the text path of `config_string` (main_view.rs) are
replaced by the table; `config_string_game_num` stays for the warnings-crowded fallback.

---

## 6. Testing (TDD)

Because the row model is data, not widgets, the meaningful behaviour is unit-testable without a
renderer:

- Between games → rows contain a `Last Game` block and a `Current Game` block, no `Next Game`.
- During a game → rows contain `Current Game` + `Next Game`, no `Last Game`.
- Last block carries no Game Block line; Current/Next blocks do.
- Settings grid hides Overtime detail rows when Overtime is off; hides Pre-Sudden-Death when
  Sudden Death is off; hides Team Timeout Duration when timeouts = 0; single-period shows
  "Game Length" and hides Half-Time.
- Compact variant omits Water referees and Helper; full variant includes them (Helper only when
  the assignment is present).
- Referees omitted entirely when not using the portal.
- Scores: Current uses live snapshot scores; Last uses last-game scores; Next has none.

Plus `just check` (fmt, clippy `-D warnings`, tests) green on all platforms.

---

## 7. Surfaces / files touched (summary)

- `refbox/src/app/view_builders/game_info_table.rs` *(new)* — row model + renderer + builder.
- `refbox/src/app/view_builders/game_info.rs` — use full table + tap-to-Game-Options.
- `refbox/src/app/view_builders/main_view.rs` — use compact table in the centre panel.
- `refbox/src/app/view_builders/shared_elements.rs` — possibly relocate/retire `config_string`
  text path; keep `config_string_game_num` fallback.
- `refbox/src/app/message.rs` + `refbox/src/app/mod.rs` — navigation to `ConfigPage::Game`;
  thread last-game scores into the view.
- `refbox/translations/<15 locales>/refbox.ftl` — new/updated labels.

No `uwh-common` / wire-format change. No `overlay` / LED-panel change.

---

## 8. Portal follow-up (separate, non-refbox task)

Write up for the Portal team (does not block this work):

- **Time/Score Helper:** define and send a referee-assignment role for a second time/score keeper
  so the Helper row can populate. Until then the row simply never appears.
- **Game Type (future):** if desired later, send a game type with values *Round Robin / Crossover
  / Playoff / Final / Medal Game*; the refbox would then add a row showing it (Unknown when
  absent). Out of scope for this build.

---

## 9. Out of scope / explicitly not doing

- Merging, rebasing, or cherry-picking the legacy branch or any of its unrelated changes.
- Any `uwh-common` wire-format change, or any change to the `overlay` / LED-panel game-info
  rendering.
- Adding Game Type or a real Helper data source to the refbox (those are Portal-side, §8).
- Re-doing or reverting the recent game-info-consistency work already in master; the table is
  built on top of it, preserving its current content and rules.

---

## 10. Details to finalise during planning / walkthrough

- Exact message shape for tap-to-Game-Options (parameterise vs. new sibling message).
- Exact row-model enum shape and the renderer's column proportions (must fit both surfaces;
  verified live).
- Non-portal team rows: blank name cell vs. a generic "White"/"Black" label.
- Whether any labels need table-specific shortening to fit cells (and the resulting locale edits).
- Compact-variant fit on the main page vs. the warnings panel (verified live).
