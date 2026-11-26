# Fitted Text Height Check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `fit_text` accepts a two-line split only when the lines' letters fit the box's height, so "1/HALF" on the Settings TEAM TIMEOUTS tile is no longer cut off.

**Architecture:** One file, `refbox/src/app/view_builders/fit_text.rs`. The pure decision functions (`fit_layout`, `shared_start`, `shared_width`) gain a `max_height` and reject a multi-line arrangement that is too tall. `layout` passes `limits.max().height` in, keys its cache on it, and logs at debug level when the height check changed the answer.

**Tech Stack:** Rust 2024, iced 0.13, `log`.

**Spec:** `docs/superpowers/specs/2026-11-26-fit-text-height-check-design.md`

## Global Constraints

- Change only `refbox/src/app/view_builders/fit_text.rs`.
- MSRV 1.85: no let-chains (`if let … && …`). The pinned toolchain accepts them, but MSRV does not.
- No `unwrap()`/`expect()` in non-test code; clippy `-D warnings`.
- A single-line arrangement is NEVER rejected for height.
- Multi-line height needed: `(lines - 1) × LineHeight::default().to_absolute(Pixels(size)).0 + size`.
- An infinite `max_height` (unbounded container) never rejects anything.
- Every cargo command runs with `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4` and only after the refbox coordinator's GO. Use `CARGO_TARGET_DIR=~/projects/uwh-refbox-rs-zero-gap/target` (the warm cache the coordinator holds for this lane).

## Review Focus

1. Two-line labels at the default 29px in 89px buttons (BLACK / TIMEOUT): must still split. Pinned by `default_two_line_button_label_still_splits`.
2. An unbounded height (`f32::INFINITY`) must behave exactly as before. Pinned by `an_unbounded_height_changes_nothing`.
3. A reserving `Shrink` box (GameNumber keypad) must not reserve a split width while the text is drawn whole. Pinned by `shared_width_does_not_reserve_a_split_that_is_too_tall`.
4. Window resize changes only height (portrait or landscape), so the cache must re-measure. `available_height` must be in `CacheKey`; reviewed by reading.
5. Japanese or Chinese labels that split between characters follow the same rule. Covered by the generic tests; the walk includes one CJK language.

---

### Task 1: Height-aware fitting

**Files:**
- Modify: `refbox/src/app/view_builders/fit_text.rs` (functions `fit_layout`, `shared_start`, `shared_width`, `CacheKey`, `Widget::layout`, and the `tests` module, including the 17 existing `fit_layout(` call sites)

**Interfaces:**
- Produces:
  - `fn text_height(lines: usize, size: f32) -> f32`: `0.0` for 0 lines; otherwise `(lines - 1) * line height at size + size`.
  - `fn fit_layout(measure: impl Fn(&str, f32) -> f32, max_width: f32, max_height: f32, candidates: &[f32], line_sets: &[Vec<String>]) -> (usize, f32)`
  - `fn shared_start(measure, max_width: f32, max_height: f32, candidates, shared, wrap) -> usize`
  - `fn shared_width(measure, size, shared, wrap, max_width: f32, max_height: f32) -> f32`

- [ ] **Step 1: Write the failing tests** (in `mod tests`, using the existing `ruler` and `candidates()`; a two-line split is `vec![vec!["1/HALF".into()], vec!["1/".into(), "HALF".into()]]`)

  - `two_lines_too_tall_fall_back_to_one_smaller_line`: `fit_layout(ruler, 60.0, 40.0, &candidates(), &split_sets)`. One line "1/HALF" is 3×size wide and fits 60 at 20. Two lines fit the width at 29, but `text_height(2, s)` = 2.3×s > 40 for every s ≥ 19, so they never fit. Expect `(0, 20.0)`. Today's code returns `(1, 29.0)`.
  - `two_lines_that_fit_the_height_still_win`: same sets and width, `max_height` 100 → `(1, 29.0)`.
  - `a_single_line_is_never_rejected_for_height`: `fit_layout(ruler, 100.0, 1.0, &candidates(), &one("ABCD"))` → `(0, 29.0)`.
  - `an_unbounded_height_changes_nothing`: `prefers_two_lines_at_a_bigger_size_over_one_shrunken_line`'s inputs with `f32::INFINITY` → `(1, 28.0)`.
  - `nothing_fits_and_two_lines_are_too_tall_gives_one_line_at_the_floor`: width 10, height 10, split sets → `(0, MIN_FIT_TEXT)`.
  - `nothing_fits_but_two_lines_fit_the_height_keeps_two_lines`: width 10, height 1000, split sets → `(1, MIN_FIT_TEXT)` (today's fallback, unchanged).
  - `default_two_line_button_label_still_splits`: the inside of an 89px button less 2×`PADDING` is 73. "BLACK TIMEOUT" split into "BLACK"/"TIMEOUT", width 110, height 73 → `(1, 29.0)`.
  - `the_team_timeouts_tile_from_the_screenshot_is_not_split_at_38`: `fit_layout(ruler, 80.0, 64.0, &size_ladder(38.0, MIN_FIT_TEXT), &split_sets)` → `(1, 27.0)`. One line needs 3×s ≤ 80, so s ≤ 26. Two lines need 2.3×s ≤ 64, so s ≤ 27. Today's code returns `(1, 38.0)`.
  - `text_height_of_one_line_is_its_size` and `text_height_of_two_lines_adds_one_line_height`: 29 → 29.0; 2 lines at 29 → 29×1.3 + 29.
  - `shared_start_does_not_count_a_split_that_is_too_tall` and `shared_width_does_not_reserve_a_split_that_is_too_tall`: with a height that rejects the split, the start index and the reserved width match the one-line answer.

- [ ] **Step 2: Run, and confirm the new tests fail** (compile errors on the new arity count as failing; after Step 3's signature-only change, `two_lines_too_tall_fall_back_to_one_smaller_line` must FAIL with `(1, 29.0)` before the logic lands)

  Run: `cargo test -p refbox --lib fit_text`

- [ ] **Step 3: Implement**
  - `text_height` as specified above.
  - `fit_layout`: an arrangement fits at `size` when every line fits `max_width` AND (`lines.len() <= 1` OR `text_height(lines.len(), size) <= max_height`). Fallback: the last (most-lines) arrangement at the floor if its `text_height` at the floor is ≤ `max_height`, otherwise index 0 at the floor.
  - `shared_start` / `shared_width`: treat a split as unavailable at `size` when `text_height(2, size) > max_height`.
  - `CacheKey`: add `available_height: f32`, and set it from `available.height`.
  - `layout`: pass `available.height` to all three. On a cache miss, when the answer differs from `fit_layout(…, f32::INFINITY, …)`, call `log::debug!` naming the lines, the chosen index and size, and the box (`available.width` × `available.height`). This line stays in the code.
  - Update the 17 existing `fit_layout(` call sites and the `shared_*` test calls to pass `f32::INFINITY`, so their expectations are unchanged.
  - Update the comment above `let top = …` in `layout` so it says why a couple of pixels of overflow is still allowed (blank line spacing, not letters).

- [ ] **Step 4: Run, and confirm everything passes**

  Run: `cargo test -p refbox --lib fit_text`. Expected: all pass, with every existing test unchanged.

- [ ] **Step 5: Commit** `fix(refbox): only split fitted text onto two lines when they fit the height`

## Verification (main session, after Task 1)

1. `just check` (GO first). Then clippy at the MSRV toolchain on refbox if available, else a grep for `if let .* &&` in the diff.
2. All languages, offline: a fontTools replay (`refbox/resources/` faces, chosen per `default_font_for`) of "1/<half>" and "1/<game>" for all 16 locales against the TEAM TIMEOUTS value box, whose size is read from the debug line in step 3. Table: locale → one line or two, and the size. Any locale at the 19 floor that is still too wide is reported to the PO, not fixed here.
3. Walk build (GO first): refbox with `-v`, its own `XDG_CONFIG_HOME`, booked with the coordinator. The PO walks Settings (portal off) in English, German, and Japanese, then the Main, Game Info, Timeouts and Warnings pages. The log's `fit_text` debug lines list every place the height check acted. Each is either the intended fix or reported.

## Deviations and status (2026-11-26)

- Status: Task 1 done (35b0dc2b); code review (high) findings applied in 29d1b20e; `just check` green; PO walk PASSED.
- refbox has no lib target: the test command is `cargo test -p refbox --bins fit_text`, not `--lib`.
- The debug line re-takes the whole answer without the height (shared start and reserved width too), and runs only when debug logging is on (review finding).
- Text a caller or translator breaks onto two lines is checked too; the spec's non-goal wording was corrected (review finding).
- Walk log, 945 x 691 window: TEAM TIMEOUTS value box is 104 x 64. Acted on 1/HALF (2 lines @38 -> 1 line @30), 1/GAME (2 @37 -> 1 @27), 1/ハーフ (2 @34 -> 2 @27), and ja いいえ on the OVERTIME / SUDDEN DEATH tiles (2 @38 -> 1 @33). Nothing on the Main, Game Info or warnings pages.
- Offline replay, all 16 languages: id-ID "1/PERTANDINGAN" and ms-MY "1/PERLAWANAN" (per GAME) are still too WIDE at the 19 floor on this window and are trimmed at the sides. Same as before this change (width, not height); reported to the PO, not fixed here.
