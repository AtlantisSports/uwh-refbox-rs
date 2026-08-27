# Shared Picker Cells — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the language-picker and buzzer-picker cells impossible to drift apart, by moving
the four hand-kept copies into one definition each in `shared_elements.rs`.

**Architecture:** Follow the role pattern established by
`docs/superpowers/plans/2026-08-27-button-height-by-role.md`: a helper owns a cell's construction
— including its height — and no call site passes one. The two language pickers become callers of
a single `make_language_grid_rows`; the two buzzer pickers become callers of a single
`make_buzzer_grid_rows` that takes the message constructor as a parameter, so each picker keeps
sending its own message. Every moved block is lifted verbatim; nothing is redesigned.

**Tech Stack:** Rust 2024, iced 0.13, `refbox` crate only.

**Spec:** This plan is the spec. It implements the first bullet of the "Known follow-ups, not done
here" section of `docs/superpowers/plans/2026-08-27-button-height-by-role.md`, whose Deviations
are required reading before starting (particularly 6, 10, 11 and 15).

**Base:** `origin/master` at `188fce06` ("refactor(refbox): make a button's height follow its
role"). Note the local `master` in the shared checkout is 4 commits behind that.

## Global Constraints

- MSRV 1.85, edition 2024. Do not change either.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must stay clean. This
  constrains task boundaries: a helper added in one commit and called in the next would fail
  `dead_code` in between, so every task adds a helper **and** converts its call sites together.
- `refbox` only. No `uwh-common`, no `overlay`, no wire format, no new dependencies.
- **Zero visible change.** All four screens must render exactly as they do today.
- Do not change any height, style, padding, spacing, or the selected-state treatment.
- The two buzzer pickers keep their own distinct messages: `Message::SelectBuzzer` and
  `Message::BeepTestSelectBuzzer`. Do not unify the two pickers.
- No `unwrap()`/`expect()` in production code.
- **Do not touch `AUDIT-PLAN.md`** or any `.feature` file. `AUDIT-PLAN.md` (gitignored, local)
  describes past commits by their symbol names; it is a dated record, not live documentation.
  Deviation 14 of the button-height plan is the precedent — a rename sweep edited a dated record
  and thereby asserted human sign-off that was never given.

---

## Scope boundary

**In scope — the only files that change:**

- `refbox/src/app/view_builders/shared_elements.rs` — new helpers, new font constants, one new
  doc comment, two visibility reductions.
- `refbox/src/app/view_builders/configuration.rs` — `make_buzzer_select_page` and
  `make_language_select_page` become callers.
- `refbox/src/app/view_builders/beep_test_settings.rs` — `build_beep_test_buzzer_picker` and
  `build_beep_test_language_picker` become callers.

**Explicitly out of scope — do not touch:**

- `font_family_id`, which is duplicated three ways (`configuration.rs`, `beep_test_settings.rs`,
  `app/mod.rs`). `beep_test_settings.rs` carries a comment explaining why it is inline. It is a
  separate, already-recorded follow-up (`AUDIT-PLAN.md` B8.11) and is not part of this change.
- The `selected_font` match itself (`Korean|Japanese|Mandarin → CJK, Thai → Thai, _ → Latin`).
  It stays in both files. Only the three `Font` values it references move.
- The Cancel / Apply / Restart footers on all four pages, including the `make_label` closure
  duplicated between the two language pickers. Those buttons are **chrome** (fixed
  `MIN_BUTTON_SIZE`) and deviation 2 of the button-height plan records that a text-pattern sweep
  already wrongly caught them once. Leave them exactly as they are.
- The `top_row_tile` / `centered_text` behaviour, `beep_test.rs`, and every other page.

**In scope but not named in the original request — flag before proceeding:**

The three `iced_core::Font` values (`cjk_font`, `thai_font`, `latin_font`) are defined
identically in both files and are consumed by both the grid and the footer. Extracting the grid
forces a choice; leaving the locals alone would mean a *third* copy inside the shared helper.
This plan moves them to `pub(super) const CJK_FONT / THAI_FONT / LATIN_FONT` in
`shared_elements.rs` and deletes both local sets, so the count goes 2 → 1 rather than 2 → 3. The
values are copied verbatim, so this is invisible.

## Acceptance criteria

Observable without reading code:

1. **Settings → App → Language** renders exactly as on `master`: 15 language tiles in four rows,
   the same order, the same fonts and native-script notes, the selected one blue, the rest gray,
   all tiles the same height and filling the page, with the "next game" ribbon above them and the
   Cancel/Apply footer below.
2. **Settings → Sound → the buzzer picker** renders exactly as on `master`: 12 sounds in three
   rows of four, one filler row, the Cancel | TEST | Apply footer. Tapping a sound selects it and
   tapping TEST plays it.
3. **Beep test → Settings → LANGUAGE** renders exactly as on `master` (no ribbon, two filler
   rows) and selecting a language still changes only the beep-test settings.
4. **Beep test → Settings → Sound Settings → the buzzer picker** renders exactly as on `master`
   (three filler rows), and selecting a sound there and applying **does** change the buzzer sound
   shown on Settings → Sound. There is one `config.sound.buzzer_sound`; both pickers stage into
   the same `edited_settings.sound.buzzer_sound`, and `BeepTestSoundSettingsSave` commits it via
   the same `apply_sound_options()` + `persist_config()` path as the hockey-mode Sound page.
5. `just check` passes.

## Why there are no new tests

Neither the moved code nor its callers is testable in this repo: iced 0.13 offers no way to
assert a rendered widget's height, the existing test modules in all three files cover pure logic
functions only, and none touches a page builder. Fabricating a test that asserts on constructed
widget values would pass without proving anything — the button-height plan made the same call for
the same reason.

An exhaustiveness guard over `Language` was considered and rejected: `Language` does not derive
`enum_iterator::Sequence`, so it would require editing `app/languages.rs`, a file outside this
change.

What replaces tests here is stronger than usual, because this is a pure extraction: **Task 5
proves the moved text is character-identical to the text it replaced.** Plus `just check`, code
review, and Eric walking all four screens.

---

## Task 1: Share the buzzer cell

The smaller of the two extractions; do it first to establish the pattern.

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (add helper near the other button
  helpers, after `make_lang_button_with_note`)
- Modify: `refbox/src/app/view_builders/configuration.rs` (`make_buzzer_select_page`, ~line 2201)
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`
  (`build_beep_test_buzzer_picker`, ~line 750)

**Interfaces produced:**

```rust
pub(super) fn make_buzzer_grid_rows<'a>(
    selected: BuzzerSound,
    on_select: fn(BuzzerSound) -> Message,
) -> Vec<Element<'a, Message>>
```

`Message::SelectBuzzer` and `Message::BeepTestSelectBuzzer` are tuple-variant constructors and
are therefore usable directly as `fn(BuzzerSound) -> Message` — no closure needed at either call
site.

- [ ] **Step 1: Add `make_buzzer_grid_rows` to `shared_elements.rs`**

The `cell` closure body is lifted verbatim from `configuration.rs:2213-2226`; only
`Message::SelectBuzzer(s)` becomes `on_select(s)`. The loop is lifted verbatim too, including its
comments and the short-chunk padding.

```rust
/// The 12 buzzer sounds as three rows of four, for both buzzer pickers.
///
/// `on_select` is the message each cell sends: the Sound settings page passes
/// `Message::SelectBuzzer`, the beep-test picker passes
/// `Message::BeepTestSelectBuzzer`. The two pickers are deliberately separate
/// pages sending separate messages; only the cells are shared.
///
/// Height is not a parameter — see [`make_tile_button`]. Callers place these
/// rows in their own column and add their own filler rows and footer, which is
/// the only thing that differs between the two pages.
pub(super) fn make_buzzer_grid_rows<'a>(
    selected: BuzzerSound,
    on_select: fn(BuzzerSound) -> Message,
) -> Vec<Element<'a, Message>> {
    // Build each sound cell: blue when selected, gray otherwise.
    let cell = |s: BuzzerSound| -> Element<'a, Message> {
        let style = if s == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        button(centered_text(s.to_string().to_uppercase()))
            .padding(PADDING)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(style)
            .on_press(on_select(s))
            .into()
    };

    // 12 sounds laid out in 3 rows of 4. BuzzerSound::ALL is always exactly 12
    // elements.
    let mut rows = Vec::new();
    for chunk in BuzzerSound::ALL.chunks(4) {
        let mut r = Row::new().spacing(SPACING).height(Length::Fill);
        for &s in chunk {
            r = r.push(cell(s));
        }
        // Pad any short final chunk with spacers (chunks(4) on 12 items is always
        // exactly 3 full rows, but this keeps the layout robust).
        for _ in chunk.len()..4 {
            r = r.push(horizontal_space());
        }
        rows.push(r.into());
    }
    rows
}
```

`Row`, `horizontal_space`, `button`, `Length`, `PADDING`, `SPACING`, `blue_selected_button` and
`light_gray_button` are already in scope in `shared_elements.rs`. `BuzzerSound` should reach it
through the `use super::*` chain (`app/mod.rs` has `use sound_controller::*`), the same way
`Mode` already does at `portal_name_for_mode`. If it does not, add
`use crate::sound_controller::BuzzerSound;` — do not widen anything else.

- [ ] **Step 2: Convert `make_buzzer_select_page` in `configuration.rs`**

Delete the `cell` closure and the `for chunk in BuzzerSound::ALL.chunks(4)` loop. Keep the
column, the ribbon, the single filler row and the footer exactly as they are.

```rust
    let mut grid = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
        None
    )]
    .spacing(SPACING)
    .height(Length::Fill);

    for r in make_buzzer_grid_rows(selected, Message::SelectBuzzer) {
        grid = grid.push(r);
    }
```

Everything from the "One trailing filler row" comment onward is untouched.

- [ ] **Step 3: Convert `build_beep_test_buzzer_picker` in `beep_test_settings.rs`**

Same shape. Delete the `cell` closure and the chunk loop; keep the three filler rows and footer.

```rust
    let mut col = Column::new().spacing(SPACING).height(Length::Fill);
    for r in make_buzzer_grid_rows(selected, Message::BeepTestSelectBuzzer) {
        col = col.push(r);
    }
```

The function's local `use crate::sound_controller::BuzzerSound;` and `use iced::widget::Row;` are
now unused if nothing else in the function needs them — check and remove only what became
unused. `BuzzerSound` is still needed for the `selected` binding's type inference? It is not
named there, so verify with the compiler rather than by eye, and let `-D warnings` decide.

- [ ] **Step 4: Build and lint**

Run: `cd <worktree> && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: clean. An `unused_imports` warning here is the compiler telling you Step 3's cleanup was
incomplete — fix it rather than allowing it.

- [ ] **Step 5: Commit** (ask Eric first — this repo requires approval before every commit)

```bash
git add refbox/src/app/view_builders/shared_elements.rs \
        refbox/src/app/view_builders/configuration.rs \
        refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "refactor(refbox): share the buzzer picker cells"
```

---

## Task 2: Share the language grid

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs`
- Modify: `refbox/src/app/view_builders/configuration.rs` (`make_language_select_page`, ~2377)
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`
  (`build_beep_test_language_picker`, ~837)

**Interfaces produced:**

```rust
pub(super) const CJK_FONT: iced_core::Font;
pub(super) const THAI_FONT: iced_core::Font;
pub(super) const LATIN_FONT: iced_core::Font;

pub(super) fn make_language_grid_rows<'a>(selected: Language) -> [Element<'a, Message>; 4];
```

**Interfaces withdrawn:** `make_lang_button_with_note` and `NameLines` become private to
`shared_elements.rs` (drop `pub(super)`). After this task their only caller is
`make_language_grid_rows`, in the same file. This is the point of the change: the plain and the
note variants finally sit side by side at the same visibility.

- [ ] **Step 1: Add the three font constants to `shared_elements.rs`**

Copy the field values verbatim from `configuration.rs:2389-2409`. Written as full struct literals
rather than `Font::with_name` so the equivalence needs no reasoning — `LATIN_FONT` uses
`Weight::Medium`, which `with_name` would not give it.

```rust
/// Fonts for the language picker. A language tile and the Cancel/Apply footer
/// beside it must use the same face, so both pickers and the shared grid read
/// them from here.
pub(super) const CJK_FONT: iced_core::Font = iced_core::Font {
    family: iced_core::font::Family::Name("WenQuanYi Zen Hei"),
    weight: iced_core::font::Weight::Normal,
    stretch: iced_core::font::Stretch::Normal,
    style: iced_core::font::Style::Normal,
};

pub(super) const THAI_FONT: iced_core::Font = iced_core::Font {
    family: iced_core::font::Family::Name("Noto Sans Thai"),
    weight: iced_core::font::Weight::Normal,
    stretch: iced_core::font::Stretch::Normal,
    style: iced_core::font::Style::Normal,
};

pub(super) const LATIN_FONT: iced_core::Font = iced_core::Font {
    family: iced_core::font::Family::Name("Roboto"),
    weight: iced_core::font::Weight::Medium,
    stretch: iced_core::font::Stretch::Normal,
    style: iced_core::font::Style::Normal,
};
```

- [ ] **Step 2: Add `make_lang_button` beside `make_lang_button_with_note`**

The plain variant, so the two shapes that sit in the same row are built by two helpers in the
same place. Body lifted from the `lang_btn` closure at `configuration.rs:2431-2444`, minus the
`.style()` and `.on_press()` the grid adds.

```rust
/// A plain language tile: the native name, centred, in that language's script.
///
/// The sibling of [`make_lang_button_with_note`] — the two shapes sit next to
/// each other in the same row, so they are defined next to each other and
/// neither takes a height. Callers add the style and the message.
fn make_lang_button<'a, Message: 'a + Clone>(
    label: &'static str,
    font: Option<iced_core::Font>,
) -> Button<'a, Message> {
    let label_widget = {
        let t = centered_text(label);
        if let Some(f) = font { t.font(f) } else { t }
    };
    button(label_widget)
        .padding(PADDING)
        .height(Length::Fill)
        .width(Length::Fill)
}
```

Also update `make_lang_button_with_note`'s doc comment: it currently warns that it must "stay in
step with the plain `lang_btn` closures in `configuration.rs` and `beep_test_settings.rs`". Those
closures no longer exist — point it at `make_lang_button` instead, and say the two are kept in
step by sitting in the same file rather than by hand.

- [ ] **Step 3: Add `make_language_grid_rows`**

The two closures move in unchanged (each keeps its own `let style = if …` so no fn-pointer
coercion changes), and the four `row![…]` blocks are lifted **character for character** from
`configuration.rs:2484-2575`, with `latin_font`/`cjk_font`/`thai_font` renamed to the new
constants. The languages-sorted comment block above them (`configuration.rs:2465-2473`) moves too.

**Copy this text mechanically — do not retype it.** The note strings contain Thai, Korean,
Japanese and Chinese text; a transcription error there is invisible in review and would ship a
corrupted label. Extract the range with `git show`/`sed` and paste it, then apply only the three
font renames. Returning an array of exactly four rows lets both call sites keep their existing
`column![…]` literal instead of switching to `.push()`, so the container's height policy is
established the same way it is today.

```rust
/// The 15 language tiles as four rows, for both language pickers.
///
/// Both pickers send `Message::SelectLanguage`; what differs between them is
/// only what surrounds the grid — the main settings page puts the "next game"
/// ribbon above it and one footer below, the beep-test page has no ribbon and
/// two filler rows. So the rows are returned, not the page.
///
/// Returned as exactly four rows because the grid is hand-written, not
/// generated: a sixteenth language needs a considered place in the alphabetical
/// order, not an automatic append.
pub(super) fn make_language_grid_rows<'a>(selected: Language) -> [Element<'a, Message>; 4] {
    let lang_btn = |lang: Language,
                    label: &'static str,
                    font: Option<iced_core::Font>|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_lang_button(label, font)
            .style(style)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Button variant for unverified translations: shows native name plus a small
    // "(UNVERIFIED)" note in that language's own script. The note text is hardcoded
    // in each target language, not routed through fl!, because fl! always renders
    // in the operator's current locale — but each button must label itself.
    let lang_btn_note = |lang: Language,
                         main: NameLines<&'static str>,
                         note: &'static str,
                         font: Option<iced_core::Font>|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_lang_button_with_note(main, note, font)
            .style(style)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Languages sorted alphabetically by romanized native name: … (keep the
    // full existing comment block from configuration.rs verbatim)
    [
        row![ /* row 1 verbatim: Indonesian, Malay, German, English */ ]
            .spacing(SPACING)
            .height(Length::Fill)
            .into(),
        row![ /* row 2 verbatim */ ].spacing(SPACING).height(Length::Fill).into(),
        row![ /* row 3 verbatim */ ].spacing(SPACING).height(Length::Fill).into(),
        row![ /* row 4 verbatim, ending in horizontal_space() */ ]
            .spacing(SPACING)
            .height(Length::Fill)
            .into(),
    ]
}
```

Two deliberate no-ops while lifting, both invisible:
- Drop the `.width(Length::Fill)` that each call site currently restates — the helper already
  sets it. This is the same cleanup deviation 12 of the button-height plan applied to a
  `make_chrome_button` call site restating its own height.
- `shared_elements.rs` has its own local `column!`/`row!` macros (defined near the top of the
  file) which expand to `Column/Row::with_children(vec![Element::from(x), …])`. iced's macros,
  which the two call-site files import, expand the same way. The moved `row![…]` blocks will now
  use the local ones. Confirm the file compiles; do not "fix" this by importing iced's macros.

- [ ] **Step 4: Convert `make_language_select_page` in `configuration.rs`**

Delete: the three `let cjk_font/thai_font/latin_font` bindings, the `lang_btn` closure, the
`lang_btn_note` closure, the languages-sorted comment block, and the four `row![…]` blocks.

Keep: `selected`, `original`, `apply_enabled`, `selected_font`, `needs_restart`, the ribbon, and
the entire footer block. `selected_font` now reads the constants:

```rust
    let selected_font: Option<iced_core::Font> = match selected {
        Language::Korean | Language::Japanese | Language::Mandarin => Some(CJK_FONT),
        Language::Thai => Some(THAI_FONT),
        _ => Some(LATIN_FONT),
    };
```

and the page becomes:

```rust
    let [lang_row_1, lang_row_2, lang_row_3, lang_row_4] = make_language_grid_rows(selected);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        lang_row_1,
        lang_row_2,
        lang_row_3,
        lang_row_4,
        {
            // … the existing footer block, entirely unchanged …
        }
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
```

The comment above `selected_font` ("Font to apply to Cancel/Apply/Restart text …") stays.

- [ ] **Step 5: Convert `build_beep_test_language_picker` in `beep_test_settings.rs`**

The same deletions and the same `selected_font` change. The page becomes:

```rust
    let [lang_row_1, lang_row_2, lang_row_3, lang_row_4] = make_language_grid_rows(selected);

    column![
        lang_row_1,
        lang_row_2,
        lang_row_3,
        lang_row_4,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![cancel_btn, horizontal_space(), confirm_btn].spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
```

`cancel_btn` and `confirm_btn` keep their `Length::Fixed(MIN_BUTTON_SIZE)`. They are chrome and
deviation 2 records that a previous sweep wrongly flipped exactly these two. Do not touch them.

Then remove whatever imports became unused (`NameLines` at minimum, likely `text`,
`Horizontal`/`Vertical` if the footer no longer needs them — let the compiler say).

- [ ] **Step 6: Build and lint**

Run: `cd <worktree> && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit** (ask Eric first)

```bash
git add refbox/src/app/view_builders/shared_elements.rs \
        refbox/src/app/view_builders/configuration.rs \
        refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "refactor(refbox): share the language picker grid"
```

---

## Task 3: Document the `centered_text` trap

Comment only. No behaviour change. Requested by Eric during planning.

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`centered_text`, ~line 1315)

- [ ] **Step 1: Write the doc comment**

`centered_text` pairs `align_y(Center)` with `height(Fill)` — the iced 0.13 stale-paragraph
pattern that `make_multi_label_button` and the other button helpers were rewritten to avoid
(`AUDIT-PLAN.md` B6.3). It is safe today only because no label built with it ever changes in
place.

Eric described this as safe "at all 7 current call sites". That count is right on `master` —
2 buzzer cells, 2 `lang_btn` closures, the beep-test preset button, the custom-site URL title,
and the `=` key on the foul keypad. **This branch collapses 4 of those 7 into 2**, so write the
comment against the state it lands in, and name the sites instead of leaning on a number that
goes stale:

```rust
/// Text centred in both axes, filling its parent.
///
/// **Do not use this for a label that changes in place.** Pairing
/// `align_y(Center)` with `height(Fill)` is the iced 0.13 stale-paragraph
/// pattern: the widget's cached paragraph position survives a content change,
/// so a shorter new label leaves half the old glyphs on screen. It is the
/// reason `make_multi_label_button` and the other button helpers were rewritten
/// to centre a container around a `Shrink`-width text instead.
///
/// Every current call site is safe only because none of their labels ever
/// changes: the buzzer cells, the language tiles, the beep-test preset button
/// and the custom-site title are fixed per cell — only the *style* changes when
/// the selection moves — and the foul keypad's `=` is a literal. Making any of
/// those labels dynamic means moving it off this helper first.
pub fn centered_text<'a, T: IntoFragment<'a>>(label: T) -> Text<'a> {
```

- [ ] **Step 2: Confirm nothing but the comment changed**

Run: `cd <worktree> && git diff -U0 -- refbox/src/app/view_builders/shared_elements.rs`
Expected: added lines are all `///`.

- [ ] **Step 3: Commit** (ask Eric first)

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "docs(refbox): warn that centered_text cannot hold a changing label"
```

---

## Task 4: Verify

- [ ] **Step 1: Full gate**

Run: `cd <worktree> && just check`
(A backgrounded command does not inherit a `cd`, so the `cd` must be part of the command.)
Expected: fmt, clippy, tests and audit all clean. Confirm from the log that the compiled paths
are the worktree's, not the shared checkout's.

- [ ] **Step 2: Prove the moved text is identical**

This is what stands in for tests. For each moved block, compare the text on `origin/master`
against the text now in `shared_elements.rs`, ignoring leading whitespace:

```bash
cd <worktree>
git show origin/master:refbox/src/app/view_builders/configuration.rs \
  | sed -n '2484,2575p' | sed 's/^[[:space:]]*//' \
  | sed 's/\blatin_font\b/LATIN_FONT/g; s/\bcjk_font\b/CJK_FONT/g; s/\bthai_font\b/THAI_FONT/g' \
  > /tmp/old-grid.txt
# extract the same four row! blocks from the new make_language_grid_rows and diff
diff /tmp/old-grid.txt /tmp/new-grid.txt
```

Expected: the only differences are the `.into()` calls the array literal needs, the dropped
`.width(Length::Fill)` restatements, and the font renames. Do the same for the buzzer `cell`
body, where the only expected difference is `Message::SelectBuzzer(s)` → `on_select(s)`.

Also diff the *deleted* text in both call-site files against the beep-test copies, to confirm the
two copies really were identical and nothing beep-test-specific was dropped:

```bash
git diff origin/master -- refbox/src/app/view_builders/beep_test_settings.rs
```

Per deviation 10 of the button-height plan: a clean result here means "found nothing", not "there
is nothing". Review and Eric's eyes remain the gate.

- [ ] **Step 3: Rebuild the app binary**

`cargo check` and `just check` do **not** refresh `target/debug/refbox` — `just check` builds test
binaries. Without this the walkthrough tests the wrong build, which has already wasted a
walkthrough once (see the button-height plan's "One process failure worth keeping").

```bash
cd <worktree> && cargo build --bin refbox
```

Then launch it and confirm `/proc/<pid>/exe` points at the worktree's binary. Note the config
directory is shared — only one refbox at a time.

- [ ] **Step 4: Compare all four screens against `master` by eye**

Settings → App → Language; Settings → Sound → buzzer picker; Beep test → Settings → LANGUAGE;
Beep test → Settings → Sound Settings → buzzer picker. For each: tile heights even, order and
fonts unchanged, the selected tile blue, the footer unchanged in size.

Also select a sound in the beep-test picker, apply it, and confirm Settings → Sound now shows it —
the two pickers edit one setting, so that is the correct outcome.

Note what this does *not* prove. `Message::SelectBuzzer` and `Message::BeepTestSelectBuzzer` have
character-identical handlers (`edited.sound.buzzer_sound = sound`), so passing the wrong
constructor to `make_buzzer_grid_rows` would be unobservable. The distinction that does carry
weight is the Cancel/Apply pair on each page, and this change does not touch those.

---

## Pre-PR gate for this repo

All four, every time, before anything is proposed as PR-ready:

1. `just check` — green.
2. The `.claude/rules/pr-review.md` checklist.
3. The built-in `code-review` skill against the current diff. (There is no `citizen-review` and no
   project review skill in this repo.)
4. A manual walkthrough by Eric, following numbered steps written for him.

Ask before branching, committing, pushing, or opening a PR.

## Deviations

**1. The plan's lint command is wrong for this repo, and the strict form fails on `master`.**
Tasks 1 and 2 said to run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
That form reports two errors that have nothing to do with this branch — `items_after_test_module`
in `keypad_pages/player_grid.rs` and `field_reassign_with_default` in `app/mod.rs` — because it
lints test targets that CI does not. The project's own `just lint` (`--all` and
`--all --no-default-features`) is the gate, and it is clean. Used that instead.

**2. The first insertion of the language grid omitted the array brackets.** The four rows went in
without the enclosing `[` … `]`, so the function body was a syntax error. Caught immediately by
`cargo fmt` refusing to parse the file. No consequence beyond one extra edit.

**3. A line-range replacement silently deleted a comment line, and only reading the result caught
it.** Converting `make_language_select_page`, the range chosen for the `selected_font` block
started one line too early and swallowed the third line of the comment above it — the one
explaining that without an explicit Latin arm, Turkish text like "İPTAL" or "BAŞLAT" renders as
tofu in a CJK/Thai locale. The code compiled and linted clean without it. Restored. The general
lesson matches deviation 10 of the button-height plan: a deletion inside a comment block is
invisible to every automated gate, so the deleted side of the diff has to be read line by line.
That is what the audit in the next deviation exists for.

**4. Every deleted line was audited individually, not by count.** All 50 deletions from the two
call-site files that do not reappear verbatim in `shared_elements.rs` were listed and accounted
for: the font locals (renamed to the shared constants), the two `on_press` lines (now the
`on_select` parameter), two imports that became unused (`Row` in `configuration.rs`, and the
function-local `BuzzerSound`/`Row` in `beep_test_settings.rs`), and the beep-test file's *shorter*
duplicates of three comments whose fuller `configuration.rs` versions were kept in the shared
helper.

**5. The identity proof came out as intended.** Normalised for indentation, against
`origin/master`:

- the buzzer cell is **identical** to both former copies (14 lines), the only change being
  `Message::SelectBuzzer(s)` / `Message::BeepTestSelectBuzzer(s)` becoming `on_select(s)`;
- the two former language grids were **identical to each other** (92 lines), and the moved version
  differs only in the four row terminators, where the trailing comma moves from
  `.height(Length::Fill),` to after the `.into()` the array literal requires;
- all 18 lines of the three font constants match their originals in both files.

**6. The beep-test buzzer picker's doc comment was wrong before this branch, and is corrected.**
It said "two trailing filler rows"; the code pushes three. The extraction made that same sentence
stale anyway (it named `BuzzerSound::ALL.chunks(4)`, which moved), so the count was fixed while
rewriting it rather than left knowingly false.

**7. The `centered_text` doc names its call sites instead of counting them.** Eric described the
helper as safe "at all 7 current call sites", which is exactly right on `master` — 2 buzzer cells,
2 `lang_btn` closures, the beep-test preset button, the custom-site title, and the foul keypad's
`=`. This branch collapses 4 of those into 2, leaving 5, so a literal "7" would have been false on
landing. The comment enumerates the sites instead. It also records two Eric's summary did not
cover: the foul keypad's `=` is a literal, and the custom-site title is an `fl!` string that
changes only on a language switch, which rebuilds the page.

**8. Dropped the redundant `.width(Length::Fill)` at the note-button call sites.**
`make_lang_button_with_note` already sets it; both call sites restated it. Invisible, and the same
cleanup deviation 12 of the button-height plan applied to a `make_chrome_button` call site
restating its own height.

**9. `make_lang_button_with_note` and `NameLines` became private.** After the grid moved, their
only caller is in the same file. That the crate still compiles is itself the evidence that nothing
else used them. This is what resolves the asymmetry the task named: the plain and note variants
are now two private siblings, side by side, at the same visibility.

**10. Not done, deliberately:** `font_family_id` is still defined three times
(`configuration.rs`, `beep_test_settings.rs`, `app/mod.rs`), and the `selected_font` match is
still written out in both pickers. Both are outside the stated scope; the first is already
recorded as `AUDIT-PLAN.md` B8.11.

**11. Two review passes ran; both found only comment defects, and every code claim survived.**
`superpowers:requesting-code-review` (repo-mandated by `.claude/rules/plan-execution.md`) went
first, then the built-in `code-review` as a second pass — because acting on a review is itself a
change that needs reviewing, which is how the predecessor branch's third pass caught a "fix" that
was wrong.

Pass 1 re-derived the identity proof independently rather than trusting it, and strengthened it:
the comparison was byte-exact on every line but leading whitespace, so the Thai, Korean, Japanese
and Chinese note strings are proven identical to `master`'s bytes — the one transcription risk the
plan flagged, closed. It also traced the layout question through iced's own source rather than
reasoning about it: `Row::with_children` routes to the same constructor as `Row::new()` and then
runs `Length::enclose` per child, the local `row!`/`column!` macros expand identically to iced's,
and the child's `size_hint()` is unchanged by where the `Element::from` boxing happens — so
`enclose` sees the same value, and both callers still set `.height(Length::Fill)` on the outer
column afterwards regardless. Given deviations 11 and 15 of the button-height plan, having that
mechanism checked against the source instead of asserted is the point.

**Five findings, all comment text, all fixed:** the `centered_text` doc contradicted itself
(it said `make_custom_site_page` avoids the helper, then listed that page's title as a call site —
the *rejection message* avoids it, the *title* uses it); it named `align_y(Center)` + `height(Fill)`
but illustrated a horizontal effect, when the helper in fact centres on both axes; the font-constant
doc claimed a tile and "the footer beside it" share a face, when the footer is below and uses the
*selected* language's face while each tile uses its own; the shared grid doc hard-coded its callers'
filler-row counts; and `configuration.rs` still said the beep-test picker has "two filler rows"
after deviation 6 corrected that same claim's twin in the other file — the identical error, four
lines from code this branch edited, missed on the first pass.

**12. Pass 2's finding was answered with documentation, not a type change, and here is why.**
It observed that `make_buzzer_grid_rows` returns `Vec<Element>` while `make_language_grid_rows`
returns `[Element; 4]` precisely so a count change becomes a compile error, and that both buzzer
callers choose their filler-row count assuming exactly three rows — so a 13th `BuzzerSound` would
silently add a fourth row and unbalance both pages.

The observation is correct and the asymmetry is deliberate: the language grid is hand-written, so
its length is a fact of the source; the buzzer grid is derived from `BuzzerSound::ALL` by
`chunks(4)`, so its length is a fact about a slice. Returning `[Element; 3]` needs a fallible
conversion from a slice-derived `Vec`, and this function has no way to fail — that means an
`expect()`, which `.claude/rules/rust.md` forbids without justification, or `collect_array`'s error
type propagated into a view builder. `slice::as_chunks` would give it cleanly but was stabilised
after MSRV 1.85.

The coupling is also **pre-existing**: both pages hard-coded their filler counts against a
three-row grid before this branch, and nothing here made that worse. So it is documented on
`make_buzzer_grid_rows` — at the place someone adding a sound would actually look — rather than
converted. A latent hazard turned into a stated one, which is the honest trade at this MSRV.

**13. One sub-threshold note from pass 2 was worth acting on anyway.** The `centered_text` doc
grouped the beep-test preset button with the fixed literals, but its label is
`format!("{} {}M", fl!("beep-test-preset-ref"), preset.metres())`. It is still fixed *per cell*, so
the conclusion held, but it belongs with the `fl!` carve-out rather than with the foul keypad's
`=`. Moved. The other note — that the `chunks(4)` short-chunk padding loop is dead at 12 sounds —
is deliberate defensiveness moved verbatim from `master`, and was left alone.

**14. Which review skill is mandatory here was corrected mid-session, and I had it backwards.**
Eric's brief named the built-in `code-review` skill; I told him that conflicted with the repo and
ran `superpowers:requesting-code-review` instead, citing `.claude/rules/plan-execution.md`. The
repo was right and my memory of it was stale: since `8dd700ca`, `.claude/rules/pr-review.md`
defines "The Three Pre-PR Checks", and check 1 is the **built-in `code-review` skill**, with
`requesting-code-review` "an optional extra pass during execution, never a substitute for it".
Both were run in the end and they found different things — the extra pass found six comment
defects the mandatory one did not — but the correction is the point: re-read
`.claude/rules/pr-review.md` at the start of any refbox PR rather than trusting a summary of it.
Note that section did not exist on this branch's original base, which is exactly why it had to be
read from `origin/master`.

**15. The walkthrough corrected a false claim in this plan, and it was load-bearing.**
Acceptance criterion 4 and Task 4's verification step both said the beep-test buzzer picker
"stages a beep-test change and does **not** alter the main Sound settings", and called a mis-wired
`on_select` "the one behavioural risk in this change". Both were wrong. `Message::SelectBuzzer` and
`Message::BeepTestSelectBuzzer` have character-identical handlers —

```rust
if let Some(edited) = self.edited_settings.as_mut() {
    edited.sound.buzzer_sound = sound;
}
```

— and `BeepTestSoundSettingsSave` commits via the same `apply_sound_options()` +
`persist_config()` path as the hockey-mode Sound page, its own comment saying so. There is **one**
`config.sound.buzzer_sound`; changing it in either picker and applying changes it in both. So a
swapped constructor would be entirely unobservable, and the step as written would have had Eric
report a defect that does not exist — or worse, had us "fix" correct behaviour into the revert he
objected to. He caught it from reading the step, before walking it.

The error was procedural, not incidental: I verified which message each picker *sends* and never
read what the handlers *do*. Half of "ground behaviour claims in code" is the engine side, and I
skipped it while stating the conclusion with confidence twice. Criterion 4 and the Task 4 step are
corrected above.

**16. Rebased onto `855011ac` after both checks had already passed, at Eric's call.**
`origin/master` moved twice during execution — `8dd700ca` (the pre-PR-checks doc) and `855011ac`
(source-button fit-text). By `.claude/rules/pr-review.md`, a rebase stales checks 1 and 2 with no
carry-over, so the choice was between opening the PR from the older base (both checks intact, the
merge queue rebasing later) or rebasing now and paying for both again. Eric chose to rebase.

The rebase applied cleanly, but that was verified rather than trusted: the two changes sitting
closest were `855011ac` adding `use super::fit_text::fit_text;` at line 1 of `configuration.rs`
and this branch removing `Row` from the `iced` widget import at line 13. Both survived — checked by
reading both regions, confirming `fit_text` still has its four usages, and running `just check`
green. The button-height plan records a rebase that resolved perfectly and would not have
compiled, which is why this is checked and not assumed.

**17. Both mandatory checks were re-run on the rebased tree.** The built-in `code-review` skill
reported no correctness findings and confirmed the extraction is a pure de-duplication (the
Fixed→Fill height change having landed in `188fce06`, not here). One caveat on its scope: it
resolved `master...HEAD` to **seven** commits, because this worktree's local `master` ref is four
behind `origin/master`, so most of that report covers already-merged work. It did assess this
commit explicitly.

Eric re-walked all four screens on the rebased build and confirmed them, covering both app modes —
the beep-test pages in beep-test mode, the main Settings pages in 6v6, switching mode between them,
which restarts the app by design. The clean `exit 0` mid-session was that restart, not a crash.
