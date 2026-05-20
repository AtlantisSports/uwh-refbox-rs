# Beep-Test Redesign — UI Polish — Design

**Date:** 2026-05-20
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (third pass on this branch — follows the original redesign and the panic-fix pass that's already landed and walkthrough-verified)

---

## Goal

Bring the beep-test mode's UI in line with the rest of the refbox — same row rhythm,
same button vocabulary, same standard helpers — and replace the bespoke top banner
on the main page with a three-cell row that surfaces the live time, level, and
within-level lap side by side.

Further Edit Levels page UI updates (beyond removing its time banner, which is
covered by item 7 below) are intentionally deferred to a separate session.

---

## Motivation

The original beep-test absorption pass and the subsequent panic-fix pass focused
on correctness: getting the cadence engine working inside refbox, keeping
`edited_settings` alive across sub-pages, and making the table render without
panicking. The page layout that resulted is functional but visibly diverges from
the rest of the refbox in three places:

- The top banner is a custom full-width time bar plus a separate row of
  `[LEVEL: N]` / `[LAPS: N]` info widgets, which doesn't match any other page.
- The levels-table cells aren't sized to the standard row height the operator
  reads on every other screen, so the table feels arbitrarily proportioned.
- Every sub-page in the BeepTest Settings hierarchy still shows the game-time
  banner, even though beep-test has no game and no use for that banner outside
  the main page.

The operator has asked for these polished out before any further Edit Levels
work goes in.

---

## Scope

### Crates touched

- `refbox` — view-builder changes and translation-key additions only.

### Files touched

- `refbox/src/app/view_builders/beep_test.rs` — main-page top row and levels table.
- `refbox/src/app/view_builders/beep_test_settings.rs` — remove time banner from
  Settings landing, Sound Settings, Edit Levels, and Language picker; tweak the
  RESTART TO APPLY button style on the landing.
- `refbox/translations/*.ftl` — new translation keys for the top-row labels
  (TIME / LEVEL / LAP). Fifteen locale files.

### Not touched

- `uwh-common` — no shared-type or wire-format changes.
- `matrix-drawing`, `overlay`, `led-panel-sim`, `wireless-modes`, `wireless-remote`.
- The cadence engine (`refbox/src/beep_test/cadence.rs`) and snapshot type
  (`refbox/src/beep_test/snapshot.rs`).
- Sound logic, hardware communication, or anything outside the BeepTest view
  builders and the translation files.
- The Edit Levels page's internals (the table + edit-panel layout, button
  arrangement, anything below the top banner). The operator has more changes
  in mind there and said they'll come in a separate session. Item 7's banner
  removal still applies to Edit Levels, but nothing else on that page is
  touched.
- The hockey-side mode-switch confirmation modal (`PortalTenantSwitch`) — beep-test
  keeps its existing inline restart-to-apply pattern, no modal involved.

---

## Design

The seven items below match the operator's numbered request one-to-one.

### 1. New three-cell top row on the main page

**Today:** a full-width light-gray time banner with large yellow time digits,
followed (below) by a row containing `[LEVEL: N]` and `[LAPS: N]` info widgets.
When the table needs more than two bands, the timer and info widgets collapse
into one row to free vertical space.

**Change:** replace both with a single row of three cells using the standard
`make_value_button` helper from `shared_elements.rs` — the same helper used
throughout the configuration pages.

Layout, left to right:

| Cell | First label  | Value             |
|------|--------------|-------------------|
| 1    | `TIME`       | mm:ss             |
| 2    | `LEVEL`      | current level number |
| 3    | `LAP`        | within-level lap number |

The row is fixed `MIN_BUTTON_SIZE` tall, separated from the table below by the
standard `SPACING`, exactly like the top row on any other page. The two-band
collapse heuristic in the existing code (`BAND_COUNT_COLLAPSE_THRESHOLD`)
becomes unnecessary and is removed.

The within-level lap derivation logic from the current `time_bar` /
`info_widget` block stays intact — only the rendering helpers change.

### 2. Green-fill level column headers

**Today:** the levels-table column headers (the "1", "2", "3" labels at the top
of each column) use `light_gray_container` when inactive and `yellow_button`
when their column is the active one.

**Change:** all column headers use `green_button` styling, regardless of which
column is active. The active-column signal moves entirely onto the active lap
cell inside that column (see item 3).

### 3. Active lap stays yellow; whole-level disabled look once a level is done

**Today's cell-state logic** (in `build_levels_table`) only marks cells
`Completed` (disabled look) within the *currently active* column. As soon as
the runner advances to the next level, the previous level's cells revert to the
`Default` look — losing the "this level is done" signal.

**Change:** extend the cell-state logic so it covers three column states:

- **Future column** (its 1-based index is greater than `active_level`):
  header = green, cells = default light-gray.
- **Active column** (index equals `active_level`):
  header = green, completed laps inside it = disabled look, the within-level
  active lap = yellow, future laps inside it = default light-gray.
- **Past column** (index is less than `active_level`, i.e. the operator has
  moved on to a later level — so every lap in this column completed):
  header = disabled look, all cells = disabled look.

The yellow active-lap cell is the *only* signal of which column is currently
running, which is unambiguous because exactly one lap cell across the whole
table is yellow at any one time.

During the warmup (`BeepTestPeriod::Level(0)`) no column is active, so all
headers are green and all cells are default.

### 4. Two-layer-per-standard-row band sizing

**Today:** the table region is `Length::Fill` and consumes whatever vertical
space remains. Cell sizes vary with screen height, with no relationship to
`MIN_BUTTON_SIZE`.

**Change:** each cell in the table is sized such that two stacked button layers
plus the standard inter-row `SPACING` equal one standard row height
(`MIN_BUTTON_SIZE`). That is:

```
cell_height = (MIN_BUTTON_SIZE - SPACING) / 2
```

A "layer" is either the header row or any single lap row. A band with `L`
layers (1 header + `L-1` lap rows) takes `L / 2` standard rows of vertical
space when `L` is even, plus one cell-and-spacing pair of slack when `L` is
odd.

To keep each band resolving to a whole number of standard rows, **if a band's
total layer count is odd, append one blank row** (a `Space` cell-row of the
same height) so the band ends on an even number of layers. The blank row sits
below the last lap row in the band; it preserves column-width alignment with
adjacent bands but is otherwise empty.

Bands are separated by the standard `SPACING`. The table region as a whole no
longer uses `Length::Fill` for its height — it sizes to its content (sum of
band heights plus inter-band spacing). Vertical slack on the main page goes
into a `horizontal_space` filler row between the table and the bottom action
row, matching how other pages absorb leftover height.

`TABLE_CELL_SPACING` (the tight intra-band spacing, currently `2.0`) is
retained for horizontal spacing between columns inside a band. Only vertical
spacing changes — between rows it now uses the standard `SPACING`.

The padding inside each cell stays the same as on the rest of the page (the
standard `PADDING`).

### 5. Main-page row rhythm matches the standard

The main BeepTest page now follows the same vertical structure every other
refbox page uses:

- Top row: the three-cell time/level/lap row from item 1 — `MIN_BUTTON_SIZE` tall.
- Middle: the levels table sized per item 4, plus a flexible
  `horizontal_space` filler row to consume leftover vertical space.
- Bottom row: the existing `[RESET] [SETTINGS] [START/STOP]` action row —
  unchanged.

All rows are separated by the standard `SPACING`. Outer page padding stays
`PADDING`. No fixed-pixel layout math beyond what `MIN_BUTTON_SIZE` and
`SPACING` already define.

### 6. Blue-fill restart-to-apply button on Settings landing

**Today:** when the staged App Mode differs from the live mode, the BeepTest
Settings landing's bottom row shows an inline `green_button` "RESTART TO APPLY"
button on the right side. Pressing it commits the new mode and restarts the
app immediately, without any confirmation step.

**Change:** the button's style changes from `green_button` to `blue_button`.
Trigger condition, label, and behaviour are unchanged.

The hockey-side `PortalTenantSwitch` confirmation modal continues to fire when
switching *into* BeepTest from the hockey configuration page — that path is
unchanged. BeepTest's own landing keeps the inline-button-only pattern; no new
confirmation modal is introduced from this side.

### 7. Remove the time banner from beep-test sub-pages

**Today:** all four BeepTest Settings sub-pages start with a
`make_game_time_button` row at the top.

**Change:** remove that row from:

- BeepTest Settings landing (`build_beep_test_settings_landing`)
- Sound Settings (`build_beep_test_sound_settings_page`)
- Edit Levels (`build_beep_test_edit_levels_page`)
- Language picker (`build_beep_test_language_picker`)

For each page, redistribute the freed vertical space among the existing
`horizontal_space` filler rows so the content rows keep the same standard row
height and the bottom footer (Cancel/Apply or Back/Restart-to-Apply) stays
anchored at the bottom of the screen. No content row above the footer changes;
only the count and sizing of filler rows in between.

The BeepTest main page keeps its top row (now the three-cell row from item 1).

---

## Translation work

Three new keys are added to all 15 locale files in `refbox/translations/`:

- `beep-test-top-time-label` → "TIME" (English)
- `beep-test-top-level-label` → "LEVEL" (English)
- `beep-test-top-lap-label` → "LAP" (English)

The existing inline-label keys `beep-test-level` ("LEVEL: {level}") and
`beep-test-laps` ("LAPS: {laps}") become unused after item 1 lands. They will
be removed in the same change, so all 15 locale files lose those two keys.

Non-English translations for the three new keys reuse the same vocabulary the
locale already uses for "TIME" / "LEVEL" / "LAP" elsewhere — they are not new
concepts. No translation amendment / unverified-tag work is needed.

---

## Acceptance criteria

The operator can confirm each item by walking through these scenarios in the
running refbox:

1. **Top row.** On the BeepTest main page, the very top row shows three
   side-by-side cells labelled TIME, LEVEL, LAP. Each cell is the same height
   as the time button on a hockey page. The current locale's translations
   appear correctly when the language is switched.
2. **Green level headers.** With the cadence engine stopped, every column
   header in the levels table is green. The headers stay green while the
   engine runs through level 1, level 2, and so on.
3. **Yellow active lap; level disable.** While running, exactly one lap cell
   across the whole table is yellow — the within-level lap that's currently
   ticking. As that level's last lap completes and the engine advances to the
   next level, the previous column's header and all its cells turn to the
   disabled look and stay that way until Reset is pressed.
4. **Two-layer-per-row sizing.** The table cells are visibly about half the
   height of a standard refbox button. Two stacked cells (with the inter-row
   gap between them) fit in the same vertical space as one row of buttons on
   a hockey page. A band with an odd layer count shows a small blank row at
   the bottom of the band so the band's bottom aligns to the standard row
   grid.
5. **Standard row rhythm.** Comparing the BeepTest main page to a hockey page
   side-by-side, the top row, the bottom row, and the vertical spacing
   between rows look identical. The table region floats inside the middle
   without distorting the rest of the page.
6. **Blue RESTART TO APPLY.** Opening BeepTest Settings, cycling the App Mode
   tile to a different mode, and looking at the bottom row: the RESTART TO
   APPLY button on the right is blue, not green. Pressing it still commits
   the mode change and restarts the app.
7. **No time banner on sub-pages.** Opening each of the four sub-pages
   (Settings landing, Sound Settings, Edit Levels, Language picker), there is
   no time banner at the top. The first row on each sub-page is the first
   content row (e.g. SOUND SETTINGS / EDIT LEVELS on the landing). Bottom
   footers (Cancel/Apply, Back) remain anchored at the bottom; the page is
   not visibly cramped.

A full walkthrough on the running refbox (`cargo run -p refbox` with
`WAYLAND_DISPLAY=` on WSL) is the verification bar. `just check` must pass.

---

## Out of scope (intentionally deferred)

- **Edit Levels page internals** — apart from removing its time banner (item 7),
  the operator has more changes planned for the Edit Levels page once this
  lands.
- A confirmation modal for BeepTest → another-mode switching (item 6 stays an
  inline button only).
- Removing or renaming the now-unused `BAND_COUNT_COLLAPSE_THRESHOLD` constant
  is in scope as cleanup once the new top row no longer needs it. No other
  refactoring is undertaken.

---

## Open implementation notes

Two small implementation choices will be made by the executing subagent inside
the plan; neither changes the design above:

- Whether the new cell height comes from `Length::Fixed((MIN_BUTTON_SIZE -
  SPACING) / 2.0)` or from a `FillPortion`-based scheme. `Length::Fixed` is
  simpler and matches how `MIN_BUTTON_SIZE` is used everywhere else; the
  executing pass should default to it.
- Whether the new translation keys are added in one commit alongside the view
  changes or in a separate translation-only commit first. Per
  `.claude/rules/plan-execution.md` (lean process for refbox UI work), one
  bundled commit is fine.
