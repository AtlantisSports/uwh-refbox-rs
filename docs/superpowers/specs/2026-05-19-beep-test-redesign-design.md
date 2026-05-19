# Beep-Test Redesign — Design Spec

**Date:** 2026-05-19
**Branch:** `feat/refbox/beep-test-redesign`
**Builds on:** `feat/refbox/beep-test-mode` (absorption — held PR)
**Process:** Lean (refbox-only, no `uwh-common`, no wire-format changes)

---

## Goal

Replace the placeholder beep-test UI inherited from the standalone crate with a refbox-native
design: a top time bar like the rest of the app, a transposed levels table that doubles as a
visual progress indicator, and a proper Settings sub-page hierarchy so the operator can change
sounds, edit the level schedule, switch modes, and change language without leaving BeepTest mode.

Also remove the redundant `Pre` state from the cadence engine (the standalone had both a `Pre`
period and a `Level(0)` period of identical 10-second duration — there is no need for both).

---

## Out of Scope

These were considered and explicitly deferred to follow-up branches:

| Item | Branch |
|------|--------|
| LED panel renders "LEAVE IN" instead of "NEXT GAME IN" | Branch 3 (`feat/uwh-common/beep-test-led-display`) |
| LED panel leaves black-side score panel blank | Branch 3 |
| New `GamePeriod::BeepTest` variant in `uwh-common` | Branch 3 |
| Anything in `uwh-common`, `wireless-remote`, or `matrix-drawing` | Branch 3 or later |
| Wireless remote support in BeepTest mode | Out of scope entirely (it's silently ignored, as today) |
| Tournament/portal integration in BeepTest mode | Out of scope (BeepTest has no portal interaction) |

---

## Acceptance Criteria

When this branch lands, all of the following are true:

1. The cadence engine no longer has a `Pre` period. Pressing **Start** transitions directly
   from stopped → `Level(0)`. The cadence engine tests reflect this.
2. The `pre` field is removed from the `BeepTest` config struct. The config migration handles
   old configs that still contain `pre` by ignoring the field.
3. The **Reset** button is visibly disabled (grayed out, non-interactive) when no Start has
   yet been pressed in the current session. It becomes enabled after the first Start press
   and stays enabled afterward, including when the cadence is paused via Stop.
4. The main beep-test view layout matches the design in this spec: a refbox-standard timer
   bar at the top, a `[LEVEL: N]` `[LAPS: N]` widget row below it, the transposed levels
   table below that, and the `[RESET] [SETTINGS] [START/STOP]` bottom row.
5. The transposed levels table shows level numbers as column headers and time values stacked
   vertically below, where the number of stacked time cells equals the lap count for that
   level. The currently active level/lap is visually highlighted.
6. Tapping **Settings** opens the new BeepTest Settings landing page (not the game-mode
   Configuration page). The landing page has four half-width buttons in a 2×2 grid: Sound
   Settings, Edit Levels, App Mode, Language.
7. Tapping **Sound Settings** opens a 3×2 grid of six controls with the disabled-gating
   described in this spec.
8. Tapping **Edit Levels** opens an interactive version of the transposed levels table where
   the operator can add, remove, and edit levels with `[Cancel] [Save]` semantics.
9. Tapping **App Mode** opens a page with the same cycle-button pattern used elsewhere in
   refbox to switch modes (Hockey 6v6 / Hockey 3v3 / Rugby / Beep Test).
10. Tapping **Language** opens the existing refbox Language Selection page.
11. `just check` passes cleanly (fmt, clippy `-D warnings` on Linux/Windows/macOS, all tests,
    audit).

---

## Main view

```
+--------------------------------------------+
|              TIMER BAR  0:36               |   row 1: refbox-standard time bar (just timer)
+--------------------------------------------+
|     [LEVEL: 1]           [LAPS: 5]         |   row 2: two info widgets
+--------------------------------------------+
| [1] [2] [3] [4] [5] [6] [7] [8] [9] [10]   |   header row (level numbers)
| [36][34][32][30][28][26][24][22][20][18]   |   time cells; column height = count
| [36][34][32][30][28][26][24][22][20][18]   |
| [36][34][32][30][28][26][24][22][20][18]   |
|         [30][28][26][24][22][20][18]       |   (overflow rows where count > 3)
|                     [22][20]               |
+--------------------------------------------+
|  [RESET]    [SETTINGS]    [START/STOP]     |   bottom row (from Branch 1)
+--------------------------------------------+
```

**Timer bar (row 1):** uses the same iced widget pattern as the existing game-mode time
banner. Just the timer text (e.g. `0:36`). No period name, no scores. Refbox-standard
yellow countdown styling.

**Widget row (row 2):** two boxes side-by-side, each about half-width. `LEVEL: N` on the
left, `LAPS: N` on the right. `LAPS` shows the **lap-within-level** counter (1..count),
not a cumulative total. (Cumulative laps across levels are not displayed.)

**Levels table:**
- Column header per level (1..N). 1-indexed for the operator.
- Each column has `count[i]` cells stacked vertically, each showing the duration in seconds.
- The active column and the active cell within it are visually highlighted as laps progress.
- If there are more than 10 levels, the table wraps: the first 10 levels appear in the first
  band of header+cell rows; levels 11–20 appear in a second band below; and so on. Each
  band is `1 (header) + max(count) (cells)` rows tall.

**Cell highlight:**
- Active level column header: bolder/brighter background to make the column stand out.
- Active cell (current lap within current level): solid background using refbox's
  existing "active period" highlight color (likely the existing yellow/highlighted tone
  used for the active quarter/half in game mode — the implementer picks the closest match).
- Completed cells within the active level: muted/dim to show "already done."
- Future cells: default cell style.

**Fallback if vertical space is tight (e.g. when the table wraps to >2 bands):** collapse
rows 1 and 2 into a single horizontal row containing `[Time 0:36] [LEVEL: 1] [LAPS: 5]`.
The implementer adopts this fallback layout when the band count exceeds a threshold the
implementer chooses (e.g. >2 bands).

---

## Behaviour

### Drop the `Pre` state

The cadence engine in `refbox/src/beep_test/cadence.rs` currently has a `BeepTestPeriod::Pre`
variant that fires for 10 seconds before `Level(0)` starts. Both `Pre` and `Level(0)` had
identical duration (10s) and 1 lap, so there were effectively two 10-second prep periods at
the start of every run.

After this change:
- `BeepTestPeriod` has only the `Level(usize)` variant.
- `TournamentManager::start_beep_test_now(now)` sets the period directly to `Level(0)` and
  starts its countdown.
- `start_next_lap` no longer returns to `Pre` after the last level — instead it stops the
  clock and resets to `Level(0)` with the clock stopped.
- The `BeepTest::pre` config field is removed. `BeepTest::migrate` no longer reads it
  (silently ignores any `pre = N` in old configs).
- The cadence engine's tests are updated to reflect the new flow.
- The sound-trigger function `maybe_play_beep_test_sound` in `refbox/src/app/mod.rs` no
  longer references `BeepTestPeriod::Pre`.
- The view_builder no longer renders a "PRE" label.

### Reset disabled until first Start

The `RefBoxApp` struct gains a `bool` field `beep_test_has_run` (or similar). It is `false`
when refbox starts, set to `true` the first time `Message::BeepTestStart` fires, and stays
`true` for the lifetime of the process (it does NOT reset to `false` when Stop or Reset
fires).

The view_builder reads this flag and renders the Reset button in a disabled style (gray
background, non-interactive — same pattern refbox uses for other disabled buttons) when
the flag is `false`. After the first Start, the button is enabled (red, interactive) as it
is today.

---

## Settings sub-page hierarchy

### Settings landing page (4 buttons, half-width 2×2)

```
+----------------------------+----------------------------+
|     SOUND SETTINGS         |       EDIT LEVELS          |
+----------------------------+----------------------------+
|        APP MODE            |        LANGUAGE            |
+----------------------------+----------------------------+
|   [BACK]                                       [DONE]   |
+---------------------------------------------------------+
```

**Back:** returns to the main beep-test view (no state changes). 

**Done:** same as Back — there is nothing on this landing page to save.

The landing page is reachable only when `config.mode == Mode::BeepTest`. It is a new
`AppState` variant: `AppState::BeepTestSettings(BeepTestConfigPage::Main)`.

`BeepTestConfigPage` is a new enum (in `refbox/src/app/mod.rs` near the existing `ConfigPage`
enum) with these variants:

```rust
pub enum BeepTestConfigPage {
    Main,         // The 2x2 landing
    Sound,        // 3x2 sound settings
    EditLevels,   // Interactive level editor
    AppMode,      // Mode cycle button
}
```

(Language re-uses the existing `AppState::EditGameConfig(ConfigPage::Language)` flow; no
new variant needed.)

### Sound Settings page (3 columns × 2 rows)

```
+----------------+--------------------+-----------------+
|  SOUND ENABLED | ABOVE WATER VOL    | WHISTLE ENABLED |
+----------------+--------------------+-----------------+
|  BUZZER SOUND  | BELOW WATER VOL    |  WHISTLE VOL    |
+----------------+--------------------+-----------------+
|   [CANCEL]                                  [SAVE]    |
+-------------------------------------------------------+
```

**Controls:**
- **SOUND ENABLED:** toggle. Reads/writes `config.sound.sound_enabled`.
- **BUZZER SOUND:** selector cycling through the available buzzer sounds. Reads/writes
  `config.sound.buzzer_sound`.
- **ABOVE WATER VOL:** volume control. Reads/writes `config.sound.above_water_vol`.
- **BELOW WATER VOL:** volume control. Reads/writes `config.sound.below_water_vol`.
- **WHISTLE ENABLED:** toggle. Reads/writes `config.sound.whistle_enabled`.
- **WHISTLE VOL:** volume control. Reads/writes `config.sound.whistle_vol`.

**Disabled gating (visual + interaction):**
- When **SOUND ENABLED** is OFF, the five other controls render disabled (grayed out,
  non-interactive). Tapping does nothing.
- When **WHISTLE ENABLED** is OFF, **WHISTLE VOL** renders disabled (regardless of Sound
  Enabled state — both gates apply).

**Save semantics:**
- The page operates on an `EditableSettings`-style staged edit (same pattern as refbox's
  existing Configuration page).
- **Cancel** discards the staged edits and returns to the Settings landing.
- **Save** commits the staged edits to `self.config.sound` and persists via `confy::store`,
  then returns to the Settings landing.

### Edit Levels page

Same transposed table from the main view, but interactive:

```
+--------------------------------------------+
|              EDIT LEVELS                   |
+--------------------------------------------+
| [1] [2] [3] [4]  ...  [10]  [+NEW]         |
| [36][34][32][30]      [18]                 |
| [36][34][32][30]      [18]                 |
| [36][34][32][30]      [18]                 |
|         [30]      ...  [18]                |
|                                            |
|  Selected: Level 4                         |
|  Time:  [30s]   [-][+]                     |
|  Count: [4]     [-][+]                     |
|  [REMOVE LEVEL]                            |
+--------------------------------------------+
|  [CANCEL]                       [SAVE]     |
+--------------------------------------------+
```

**Interactions:**
- Tap a column header (or any cell in the column) to **select** that level. The selected
  level is visually highlighted (different style from the main view's "active lap"
  highlight, but reusable patterns are fine).
- The bottom edit panel shows the selected level's time and count, each with `[-]` `[+]`
  buttons to decrement/increment.
- **+NEW** adds a new level at the end with default values (count=4, duration=20s — same
  as the last default level). The new level becomes the selected one.
- **REMOVE LEVEL** removes the selected level. If only 1 level remains, this button is
  disabled (need at least 1 level).
- **CANCEL** discards the staged edits and returns to the Settings landing.
- **SAVE** commits the staged edits to `self.config.beep_test.levels` and persists via
  `confy::store`, then returns to the Settings landing.

**Validation:**
- `levels.len() >= 1` enforced (Remove disabled when len == 1).
- `count >= 1` per level (decrement disabled at 1).
- `duration >= 1 second` per level (decrement disabled at 1s).
- No upper bound enforced (the table wraps; that's the only practical concern).

**Initial state:** when the page opens, the first level is selected by default.

### App Mode page

Single screen with the existing refbox cycle-button widget (same pattern as the game-mode
Configuration page's `Cyclable<Mode>` button). Tapping cycles Hockey 6v6 → Hockey 3v3 →
Rugby → Beep Test → Hockey 6v6.

```
+--------------------------------------------+
|              APP MODE                      |
+--------------------------------------------+
|                                            |
|         [   Beep Test   ]                  |   (cycle button)
|                                            |
+--------------------------------------------+
|  [CANCEL]                       [APPLY]    |
+--------------------------------------------+
```

**Cancel:** discard staged mode change, return to Settings landing.

**Apply:** commit the staged mode change. If the new mode differs from `self.config.mode`,
trigger the existing mode-change-restart flow (the same one that runs today when changing
mode in the game-mode Configuration page). Refbox re-execs and lands in the new mode.

If the mode is unchanged, Apply simply returns to the landing.

### Language page (reused)

Tapping Language on the Settings landing navigates to the existing
`AppState::EditGameConfig(ConfigPage::Language)`. The existing UI and message flow are
reused entirely. When the operator hits Save/Back from that page, the existing flow
returns them to... whichever existing return path is configured. Verify that after
returning from the Language page in BeepTest mode, the operator lands back on the
BeepTest Settings landing (not the game-mode Configuration landing).

If the existing return path doesn't handle this correctly, add a small adjustment to
`ConfigEditComplete` or equivalent so it routes back to `AppState::BeepTestSettings(Main)`
when `config.mode == Mode::BeepTest` and the previous page was the language page.

---

## Files

### Created

| Path | Responsibility |
|------|----------------|
| `refbox/src/app/view_builders/beep_test_settings.rs` | Settings landing (2×2), Sound Settings (3×2), Edit Levels, App Mode pages |
| `docs/superpowers/specs/2026-05-19-beep-test-redesign-design.md` | This spec |
| `docs/superpowers/plans/2026-05-19-beep-test-redesign.md` | The plan |

### Modified

| Path | Why |
|------|-----|
| `refbox/src/beep_test/snapshot.rs` | Remove `Pre` variant from `BeepTestPeriod`; update Display impl |
| `refbox/src/beep_test/cadence.rs` | Remove Pre transitions; `start_beep_test_now` goes to Level(0); update tests |
| `refbox/src/config.rs` | Remove `pre: Duration` field from `BeepTest`; update Default, migrate, and tests |
| `refbox/src/app/mod.rs` | Add `beep_test_has_run` field; add `AppState::BeepTestSettings(BeepTestConfigPage)` variant; dispatch the new sub-pages in `view()`; add Message variants for the new pages; handle them in `update()`; update `maybe_play_beep_test_sound` to drop Pre references |
| `refbox/src/app/message.rs` | New Message variants for Settings navigation, Sound Settings controls, Edit Levels actions, App Mode toggle |
| `refbox/src/app/view_builders/beep_test.rs` | Complete rewrite of the main-view layout per this spec |
| `refbox/src/app/view_builders/mod.rs` | Register `pub mod beep_test_settings;` |
| `refbox/translations/<locale>/refbox.ftl` × 15 | New keys for Settings labels, Sound page controls, Edit Levels buttons, App Mode title; reused English placeholders for the 12 non-fr/es locales |

### Deleted

None. The absorption Branch 1's `beep_test.rs` is rewritten, not deleted.

---

## Design rationale

**Why drop the `Pre` state?** Inspection of the cadence engine shows `Pre` and `Level(0)`
both fire for 10 seconds with 1 lap. There is no operator-visible distinction. Dropping `Pre`
removes a redundant state and simplifies the cadence engine, tests, and view.

**Why a transposed table?** The original vertical table (Level/Count/Duration as columns,
one row per level) wasted horizontal space and gave no visual indication of progress. The
transposed table fits naturally across the refbox's wide aspect ratio and the column
height encodes the count — making the schedule legible at a glance. Cell-by-cell highlighting
as laps progress turns the schedule into a live progress indicator.

**Why a dedicated Settings landing page (not reuse Configuration)?** The game-mode
Configuration page exposes Game / App / User / Language tabs, most of which (timeouts, half
durations, etc.) are irrelevant in BeepTest mode. Reusing it would confuse the operator with
inapplicable options. A BeepTest-specific landing keeps the options focused.

**Why is Language a separate option (not under Sound or App Mode)?** Language is a global
preference, not specific to sound or mode. Treating it as a peer of the other three options
matches its scope.

**Why is the cycle-button pattern reused for App Mode?** Refbox already uses this pattern
for the Mode cycler in the game-mode Configuration. Reusing it is one less new widget to
build and gives the operator a familiar interaction.

---

## Walkthrough scenarios (to verify when implementing Task 11 / equivalent)

| # | Scenario | Expected outcome |
|---|----------|------------------|
| A | Open Settings, observe landing page | Four half-width buttons in a 2×2 grid: Sound Settings, Edit Levels, App Mode, Language. Back/Done at the bottom. |
| B | Open Sound Settings, toggle Sound Enabled off | The other five controls visibly gray out. Toggling Sound Enabled on re-enables them. |
| C | With Sound Enabled on, toggle Whistle Enabled off | Whistle Vol grays out; the other four controls stay enabled. |
| D | Cancel from Sound Settings after making changes | Returns to Settings landing; the config is unchanged. |
| E | Save from Sound Settings after making changes | Returns to Settings landing; the config is updated and persisted. |
| F | Open Edit Levels, change Level 3's time and count, Save | Returns to Settings landing; the main view's table reflects the new values; next Start uses the new schedule. |
| G | Open Edit Levels, Add a new level, Save | Main view's table shows the new column. |
| H | Open Edit Levels, Remove Level 5, Save | Main view's table no longer has Level 5; what was Level 6 is now Level 5. |
| I | Open App Mode, cycle to Hockey 6v6, Apply | Refbox restarts, lands in Hockey 6v6 mode. |
| J | Open Language, change to French, Save | Returns to BeepTest Settings landing in French; all BeepTest UI translated. |
| K | On the main view, observe Reset state with no Start press | Reset button rendered disabled. |
| L | Press Start, observe Reset state | Reset button rendered enabled. |
| M | Press Start; observe cadence display | Timer counts down through Level 0 → Level 1 → ... directly (no "PRE" state). Active cell highlights cell-by-cell. |

---

## Spec coverage (self-check)

| Requirement | Covered by |
|-------------|-----------|
| Drop PRE state | "Drop the `Pre` state" section + Task 1 in plan |
| Reset disabled until first Start | "Reset disabled until first Start" section + Task 2 in plan |
| New main view layout | "Main view" section + Task 3 in plan |
| Settings landing (2×2) | "Settings sub-page hierarchy / Settings landing page" + Task 4 |
| Sound Settings (3×2) | "Sound Settings page" + Task 5 |
| Edit Levels | "Edit Levels page" + Task 6 |
| App Mode | "App Mode page" + Task 7 |
| Language reuse | "Language page (reused)" + Task 8 |
| 15-locale translations | "Files / Modified" row + Task 9 |
| `just check` clean | Verification step on every task |
