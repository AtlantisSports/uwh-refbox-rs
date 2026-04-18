# Turkish Language and "(UNVERIFIED)" Label — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land two commits on `feat/workspace/desktop-build`: (1) Turkish as a selectable language matching the Latin-script pattern, and (2) a small "(UNVERIFIED)" note in each button's own language on every language-select button except English / Spanish / French.

**Architecture:** Commit 1 extends the `Language` enum, adds a new `tr-TR/refbox.ftl`, and places a `TÜRKÇE` button between Thai and Mandarin on the language-select grid. Commit 2 adds one new helper in `shared_elements.rs` for the "native name + small note" button shape and rewires the 12 unverified buttons in `configuration.rs` to use it, with each note text hardcoded inline in that button's target language.

**Tech Stack:** Rust 2024 (MSRV 1.85), `iced` 0.13, Fluent (`fl!` macro via `i18n_embed_fl`), cargo workspace tools (`just check`, `just fmt`, `just lint`, `just test`).

**Source of truth:** See companion spec `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md`.

**Testing note (deviation from default TDD):** The spec explicitly calls out that no new unit tests are required — this matches the pattern of the prior language-addition commits (`199e1b9`, `15b9f18`, `44ec5f1`). Validation is via `just check` (fmt + clippy + existing tests + audit) plus a manual UI smoke check on the running app. If an obvious regression surface appears during implementation, flag it rather than adding speculative tests.

**Branch state:** Current branch is `feat/workspace/desktop-build`. Do not switch off it. HEAD at plan-write time: `2897046` (the spec doc commit).

---

## Task 0: Human Approval — Plan Review

**Files:** none (gate only)

- [ ] **Step 1: Present this plan to the human for review.**

    The human must approve this plan before any implementation begins. This is a CLAUDE.md approval gate.

- [ ] **Step 2: Wait for explicit approval.**

    Expected approval signal: "approved", "ok, proceed", or similar. If the human requests changes, update this plan and re-present.

---

## Task 1: Turkish UWH Glossary Research + Human Review

**Files:**
- Create: `docs/superpowers/specs/2026-04-17-turkish-glossary.md` (temporary reference — not committed long-term unless the human asks; it's a review artefact)

**Purpose:** Build a ~25-30 term glossary of UWH-specific Turkish terms with source tags, so the full `tr-TR/refbox.ftl` (Task 3) uses vetted domain terminology rather than machine-translated guesses. Generic UI strings ("Start", "Cancel", "Options") don't belong here — just the referee/UWH-specific vocabulary.

- [ ] **Step 1: Research Turkish UWH resources.**

    Prioritise in this order:
    1. TSSF (Türkiye Sualtı Sporları Federasyonu) — specifically the "Sualtı Hokeyi" rulebook if published in Turkish. Search terms: `"sualtı hokeyi" kural`, `TSSF sualtı hokeyi yönetmelik`.
    2. Turkish UWH club / team pages (e.g. university UWH teams' Turkish materials).
    3. Turkish Wikipedia article on underwater hockey (`tr.wikipedia.org/wiki/Sualtı_hokeyi`) as a secondary source.
    4. CMAS-Turkey materials if accessible.

    Capture each term's source URL / citation. Use WebFetch / WebSearch tools.

- [ ] **Step 2: Produce the glossary doc.**

    Create `docs/superpowers/specs/2026-04-17-turkish-glossary.md` with this structure:

    ```markdown
    # Turkish UWH Glossary — First Pass

    **Date:** 2026-04-17
    **Purpose:** Vet UWH-specific Turkish terms before generating tr-TR/refbox.ftl.
    **Review status:** Pending human review. Native-speaker review is a separate, later step.

    ## Source tags

    - `[FED-RULEBOOK]` — from the TSSF rulebook (cite URL/section)
    - `[WEB-UWH-CLUB]` — from a Turkish UWH club/team page (cite URL)
    - `[WIKIPEDIA-TR]` — from Turkish Wikipedia
    - `[BEST-GUESS]` — no direct source found, proposed from general Turkish sports vocabulary

    ## Glossary (~25-30 terms)

    | English (from en-US/refbox.ftl) | Turkish | Source | Notes |
    |---|---|---|---|
    | Underwater hockey | Sualtı hokeyi | [WIKIPEDIA-TR] | |
    | Penalty (30s / 1m / 2m / 4m / 5m) | ... | ... | |
    | Total dismissal | ... | ... | |
    | Team timeout | ... | ... | |
    | Half | ... | ... | |
    | Between games | ... | ... | |
    | Sudden death / Overtime | ... | ... | |
    | Half time | ... | ... | |
    | Goal | ... | ... | |
    | Foul | ... | ... | |
    | Warning | ... | ... | |
    | Cap number | ... | ... | |
    | Black team / White team | ... | ... | |
    | Referee | ... | ... | |
    | Penalty shot | ... | ... | |
    ... (continue to ~25-30 rows)
    ```

    Every row must have an actual Turkish translation and a source tag. `[BEST-GUESS]` is fine but must be honestly flagged.

- [ ] **Step 3: Present glossary to human for review.**

    Summarise in chat: the term count, the breakdown by source tag (e.g. "18 FED-RULEBOOK, 7 WIKIPEDIA-TR, 5 BEST-GUESS"), and any terms where the best guess felt particularly uncertain.

- [ ] **Step 4: Incorporate human corrections.**

    If the human changes any term, update the glossary doc. Iterate until approval.

- [ ] **Step 5: Human approval gate.**

    Wait for explicit glossary sign-off before moving to Task 2. This is a CLAUDE.md approval gate — Task 2 onwards depends on the glossary being stable.

---

## Task 2: Add Turkish to the `Language` enum (`languages.rs`)

**Files:**
- Modify: `refbox/src/app/languages.rs`

- [ ] **Step 1: Add `Turkish` to the enum.**

    In `refbox/src/app/languages.rs`, add `Turkish` as the new last variant (after `Thai`):

    ```rust
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum Language {
        English,
        French,
        Spanish,
        Mandarin,
        Korean,
        Italian,
        German,
        Tagalog,
        Indonesian,
        Dutch,
        Japanese,
        Malay,
        Portuguese,
        Thai,
        Turkish,
    }
    ```

- [ ] **Step 2: Add Turkish to `as_lang_id`.**

    Add this arm to the `match self` block in `as_lang_id` (after the `Thai` arm):

    ```rust
    Self::Turkish => LanguageIdentifier::from_bytes(b"tr-TR").unwrap(),
    ```

- [ ] **Step 3: Add Turkish to `from_lang_id`.**

    Add this `else if` branch before the final `else` (after the Thai branch):

    ```rust
    } else if lang_id.matches(&"tr".parse::<LanguageIdentifier>().unwrap(), false, true) {
        Self::Turkish
    ```

- [ ] **Step 4: Add Turkish short strings.**

    Add one arm per text function (placeholder values; replace with the glossary-approved Turkish if the glossary changes any of these):

    ```rust
    // In cancel_text:
    Self::Turkish => "İPTAL",

    // In done_text:
    Self::Turkish => "TAMAM",

    // In restart_text:
    Self::Turkish => "UYGULAMAK İÇİN YENİDEN BAŞLAT",
    ```

    Cross-check these three strings against the glossary from Task 1; if the glossary has a specific preferred Turkish for "CANCEL" / "DONE" / "RESTART TO APPLY", use the glossary's value.

- [ ] **Step 5: Update `Cyclable::next`.**

    Change the last arm of the `impl Cyclable for Language { fn next(&self) -> Self { match self { ... } } }` block so that `Thai` → `Turkish` → `English`:

    ```rust
    Self::Portuguese => Self::Thai,
    Self::Thai => Self::Turkish,
    Self::Turkish => Self::English,
    ```

- [ ] **Step 6: Verify it compiles.**

    Run: `cargo check -p refbox`
    Expected: compiles cleanly (warnings are OK at this point, fixed in Task 5).

---

## Task 3: Create `tr-TR/refbox.ftl`

**Files:**
- Create: `refbox/translations/tr-TR/refbox.ftl`

- [ ] **Step 1: Read `refbox/translations/en-US/refbox.ftl` in full.**

    Use the Read tool on `refbox/translations/en-US/refbox.ftl`. This is the authoritative key set. Every key in this file must appear in the Turkish file.

- [ ] **Step 2: Read a reference non-English locale for formatting comparison.**

    Read `refbox/translations/nl-NL/refbox.ftl` (Dutch) — another Latin-script addition with the same structure. This shows the expected shape (same keys, translated values, same `{$placeholder}` syntax preserved).

- [ ] **Step 3: Generate the Turkish `refbox.ftl`.**

    Create `refbox/translations/tr-TR/refbox.ftl` with every key from `en-US`, in the same order, with Turkish values. Rules:
    - Use the Task 1 glossary for UWH-specific terminology.
    - Preserve Fluent placeholders (`{$name}`, `{$count}`) unchanged.
    - Preserve the `penalty-kind = {$kind -> ... }` selector structure; translate the inner branches to Turkish (e.g. `[thirty-seconds] 30s` stays as an opaque key; only user-visible outputs change).
    - Preserve comments (`# Penalty Edit`) — translate the comment text as well.
    - Preserve multi-line values that use the indented continuation format (`key = LINE1\n    LINE2`).
    - Preserve the `-dark-team-name` / `-light-team-name` private-message keys that reference team colours.

- [ ] **Step 4: Verify key parity.**

    Run:

    ```bash
    grep -E '^[a-zA-Z-]+ *=' refbox/translations/en-US/refbox.ftl | cut -d= -f1 | sort -u > /tmp/en-keys.txt
    grep -E '^[a-zA-Z-]+ *=' refbox/translations/tr-TR/refbox.ftl | cut -d= -f1 | sort -u > /tmp/tr-keys.txt
    diff /tmp/en-keys.txt /tmp/tr-keys.txt
    ```

    Expected: no diff output. If there is a diff, add the missing keys to `tr-TR/refbox.ftl` until parity holds.

- [ ] **Step 5: Verify it builds and Fluent parses it.**

    Run: `cargo build -p refbox`
    Expected: builds cleanly. If Fluent parse errors appear, fix them in `tr-TR/refbox.ftl`.

---

## Task 4: Add `TÜRKÇE` button to the language-select grid

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (specifically the `make_language_select_page` function, around lines 1017-1130)

- [ ] **Step 1: Update the alphabetical-order comment.**

    In `make_language_select_page`, find the comment block starting `// Languages sorted alphabetically by romanized native name:` (around lines 1072-1075). Change the last line from:

    ```rust
    // Nederlands(N), Nihongo/日本語(N), Português(P), Thai/ภาษาไทย(T), Zhōngwén/中文(Z)
    ```

    to:

    ```rust
    // Nederlands(N), Nihongo/日本語(N), Português(P), Thai/ภาษาไทย(T),
    // Türkçe(T), Zhōngwén/中文(Z)
    ```

- [ ] **Step 2: Insert the `TÜRKÇE` button into row 4.**

    Find the row 4 block (around lines 1118-1123):

    ```rust
    row![
        lang_btn(Language::Thai, "ภาษาไทย", Some(thai_font)),
        lang_btn(Language::Mandarin, "中文", Some(cjk_font)),
        horizontal_space(),
        horizontal_space(),
    ]
    ```

    Change it to:

    ```rust
    row![
        lang_btn(Language::Thai, "ภาษาไทย", Some(thai_font)),
        lang_btn(Language::Turkish, "TÜRKÇE", None),
        lang_btn(Language::Mandarin, "中文", Some(cjk_font)),
        horizontal_space(),
    ]
    ```

- [ ] **Step 3: Run `just check`.**

    Run: `just check`
    Expected: fmt clean, clippy zero warnings, tests pass, audit clean.

    If clippy complains about a `match` that is missing `Turkish` anywhere else in the crate, add the missing arm (most likely an `_ => ...` fallthrough is already present in `font_family_id`, but verify by searching: `Grep "Language::" in refbox/src`).

- [ ] **Step 4: Manual UI smoke test.**

    Run the app (Wayland requires `dangerouslyDisableSandbox:true`):

    ```bash
    just refbox
    ```

    Or the equivalent command the human uses. Open the configuration → language-select screen. Confirm:
    - A `TÜRKÇE` button appears between Thai and Mandarin on row 4.
    - Clicking `TÜRKÇE` selects Turkish; the selection highlight (blue) moves to that button.
    - Clicking Done applies Turkish; the rest of the refbox UI now renders in Turkish from `tr-TR/refbox.ftl`.
    - Cancel/Done/Restart short text on the language-select screen renders the Turkish values from `languages.rs`.
    - Selecting English from Turkish, then clicking through every other language, still works.

    Close the app.

---

## Task 5: Commit 1 — Turkish language

**Files:** the three touched so far plus the new `.ftl`.

- [ ] **Step 1: Verify `git status` shows only the expected files.**

    Run: `git status --short`
    Expected to show (in the modified/untracked set for this work):
    - `M  refbox/src/app/languages.rs`
    - `M  refbox/src/app/view_builders/configuration.rs`
    - `?? refbox/translations/tr-TR/refbox.ftl`

    Plus pre-existing unrelated modifications (`refbox/tests/features/...`, `uwh-common/tests/features/...`, `docs/decisions/004-...`, etc. — these must NOT be staged).

- [ ] **Step 2: Human approval gate.**

    Present a plain-language summary of Commit 1 to the human:
    - **What changed:** Turkish added as a selectable language; new translation file; new button on the language-select screen between Thai and Mandarin.
    - **Why:** Requested feature — expand the app's language support to include Turkish for tournaments in Turkey.
    - **How to verify:** Open Configuration → Language; select TÜRKÇE; confirm UI text switches to Turkish; confirm no regression in other 13 languages.

    Wait for explicit commit approval.

- [ ] **Step 3: Stage only the three files.**

    Run:

    ```bash
    git add refbox/src/app/languages.rs \
            refbox/src/app/view_builders/configuration.rs \
            refbox/translations/tr-TR/refbox.ftl
    ```

- [ ] **Step 4: Create the commit.**

    Use this HEREDOC template (adjust wording to match the state of the code, but keep the `feat(refbox):` prefix):

    ```bash
    git commit -m "$(cat <<'EOF'
    feat(refbox): add Turkish translation and language button

    Extends the Language enum with tr-TR, adds a full Turkish translation
    file, and inserts a TÜRKÇE button on the language-select screen
    between the Thai and Mandarin buttons. Turkish uses the Latin script
    with diacritics — no new font bundling required. Translations are
    first-pass, sourced from the TSSF rulebook and Turkish UWH club
    materials for domain terms; awaiting native-speaker verification.

    Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
    EOF
    )"
    ```

- [ ] **Step 5: Verify the commit.**

    Run: `git log --oneline -1 && git status --short`
    Expected: new commit on `feat/workspace/desktop-build`; unrelated dirty files still untouched.

---

## Task 6: Add "native name + note" button helper (`shared_elements.rs`)

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (add new helper; do NOT modify `make_multi_label_button`)

- [ ] **Step 1: Pick the small-text size.**

    Read `refbox/src/app/theme/` to find existing text-size constants (likely `SMALL_TEXT`, `SMALL_PLUS_TEXT`, etc.). Choose the smallest constant that is reasonably legible. Record the chosen constant name in the task output.

- [ ] **Step 2: Add the helper.**

    Add this helper to `shared_elements.rs`, alongside `make_multi_label_button`. Use the small-text constant chosen in Step 1 wherever `<SMALL_TEXT_CONSTANT>` appears below.

    ```rust
    /// Language-select button with one or two lines of native name text
    /// and an optional smaller "(UNVERIFIED)" note below. Used only on
    /// the language-select grid. See make_multi_label_button for general
    /// multi-label buttons without a smaller note.
    pub(super) fn make_lang_button_with_note<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
        main_line_1: T,
        main_line_2: Option<T>,
        note: T,
    ) -> Button<'a, Message> {
        let m1 = text(main_line_1)
            .align_x(Horizontal::Left)
            .width(Length::Shrink);
        let n = text(note)
            .size(<SMALL_TEXT_CONSTANT>)
            .align_x(Horizontal::Left)
            .width(Length::Shrink);

        let mut col = column![container(m1).center_x(Length::Fill)];
        if let Some(m2_text) = main_line_2 {
            let m2 = text(m2_text)
                .align_x(Horizontal::Left)
                .width(Length::Shrink);
            col = col.push(container(m2).center_x(Length::Fill));
        }
        col = col.push(container(n).center_x(Length::Fill));

        button(container(col.width(Length::Fill)).center(Length::Fill))
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .width(Length::Fill)
    }
    ```

    If the compiler complains about type inference for the conditional `push`, move the `col` out of `column!` into a plain `Column::new().push(...)` and rebuild. Keep it idiomatic to the rest of `shared_elements.rs`.

- [ ] **Step 3: Verify it compiles.**

    Run: `cargo check -p refbox`
    Expected: clean.

---

## Task 7: Wire up UNVERIFIED notes in the language grid (`configuration.rs`)

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (only `make_language_select_page`)

- [ ] **Step 1: Import the new helper.**

    Ensure `make_lang_button_with_note` is visible in `configuration.rs` — follow the same `use super::...` path as `make_multi_label_button` already uses in that file.

- [ ] **Step 2: Replace the 10 single-line unverified language buttons.**

    For each of these 10 buttons, replace the current `lang_btn(...)` call with a styled button using `make_lang_button_with_note(native_name, None, note_text)`, wrapped in the same styling (selected vs. unselected), fonts, and `on_press(Message::SelectLanguage(...))` as the existing `lang_btn` closure applies.

    Concretely, each replacement follows this shape (example for German):

    ```rust
    {
        let style = if selected == Language::German { blue_selected_button } else { light_gray_button };
        make_lang_button_with_note("DEUTSCH", None, "(NICHT VERIFIZIERT)")
            .style(style)
            .on_press(Message::SelectLanguage(Language::German))
    }
    ```

    Apply the same pattern for the other 9:

    | Language | Native name (line 1) | Note |
    |----------|----------------------|------|
    | Italian | `"ITALIANO"` | `"(NON VERIFICATO)"` |
    | Dutch | `"NEDERLANDS"` | `"(NIET GEVERIFIEERD)"` |
    | Portuguese | `"PORTUGUÊS"` | `"(NÃO VERIFICADO)"` |
    | Tagalog | `"FILIPINO"` | `"(HINDI PA NA-VERIFY)"` |
    | Korean | `"한국어"` | `"(검증되지 않음)"` |
    | Japanese | `"日本語"` | `"(未検証)"` |
    | Mandarin | `"中文"` | `"(未验证)"` |
    | Thai | `"ภาษาไทย"` | `"(ยังไม่ได้ตรวจสอบ)"` |
    | Turkish | `"TÜRKÇE"` | `"(DOĞRULANMAMIŞ)"` |

    For the Korean / Japanese / Mandarin buttons, also apply the existing `cjk_font` via `.style` or per the current pattern. For Thai, apply `thai_font`. You may need to extend `make_lang_button_with_note` to take an optional `Font` for the main text lines — if so, add that parameter now, defaulting to `None`, and pass the font through to the `text(...)` calls in the helper.

- [ ] **Step 3: Replace the two Bahasa buttons with two-line + note form.**

    The existing buttons use `make_multi_label_button(("BAHASA", "INDONESIA"))`. Replace each with `make_lang_button_with_note("BAHASA", Some("INDONESIA"), "(BELUM DIVERIFIKASI)")` (and `(BELUM DISAHKAN)` for Malay), wrapped in the existing `style` / `on_press` chain.

- [ ] **Step 4: Leave English / Spanish / French unchanged.**

    Those three `lang_btn(...)` calls remain as they are. Do not touch them.

- [ ] **Step 5: Run `just check`.**

    Run: `just check`
    Expected: fmt, clippy, tests, audit all clean. If the `lang_btn` closure is now only used for three languages, that's fine — do not remove or refactor it unless clippy requires it.

- [ ] **Step 6: Manual UI smoke test.**

    Run the app. On the language-select screen, confirm:
    - English, Spanish, French buttons look identical to before (single line, no note).
    - The 10 single-line unverified buttons show the native name on line 1 and `(UNVERIFIED)` in that language's script on a smaller line 2.
    - The two Bahasa buttons now have three lines: `BAHASA` / `INDONESIA` (or `MELAYU`) / `(BELUM ...)`.
    - All buttons remain the same height as before (still `MIN_BUTTON_SIZE`).
    - The small text is legible at the app's default DPI.
    - Selecting and confirming each of the 12 unverified languages still switches the UI correctly; no layout break.

    If the small-text size is too small to read or too large to fit three lines in the Bahasa buttons, go back to Task 6 Step 1 and pick a different constant, re-run steps 5-6.

---

## Task 8: Commit 2 — UNVERIFIED labels

**Files:** `shared_elements.rs` and `configuration.rs`.

- [ ] **Step 1: Verify `git status` shows only the expected files.**

    Run: `git status --short`
    Expected to show:
    - `M  refbox/src/app/view_builders/configuration.rs`
    - `M  refbox/src/app/view_builders/shared_elements.rs`

    Plus the pre-existing unrelated dirty files. No FTL files should appear.

- [ ] **Step 2: Human approval gate.**

    Present a plain-language summary to the human:
    - **What changed:** A small "(UNVERIFIED)" note in the target language now appears under every language button except English / Spanish / French, so operators know those translations are awaiting native-speaker review.
    - **Why:** Eleven of the existing translations (now twelve with Turkish) are first-pass and not yet verified by native speakers. This note surfaces that status honestly.
    - **How to verify:** Open Configuration → Language; scan the grid; confirm the small note appears under each of the 12 unverified languages in their own script; confirm English/Spanish/French are unchanged.

    Wait for explicit commit approval.

- [ ] **Step 3: Stage only the two files.**

    Run:

    ```bash
    git add refbox/src/app/view_builders/configuration.rs \
            refbox/src/app/view_builders/shared_elements.rs
    ```

- [ ] **Step 4: Create the commit.**

    ```bash
    git commit -m "$(cat <<'EOF'
    feat(refbox): show "(UNVERIFIED)" note under unreviewed language buttons

    Adds a small note in each button's own target language under every
    language-select button except English, Spanish, and French, so
    operators see at a glance which translations are still awaiting
    native-speaker review. Note strings are hardcoded inline alongside
    each button's native name because fl!() resolves only against the
    operator's current language, not each button's target language.

    Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
    EOF
    )"
    ```

- [ ] **Step 5: Verify.**

    Run: `git log --oneline -2 && git status --short`
    Expected: both commits on branch; pre-existing dirty files still untouched.

---

## Task 9: End-of-branch options

**Files:** none (workflow gate only)

- [ ] **Step 1: Invoke the finishing-a-development-branch skill.**

    Per CLAUDE.md and the initial task prompt, `feat/workspace/desktop-build` is the v0.4.0 integration branch and accumulates work. The expected outcome is "keep as-is", but present the four options (merge, PR, keep, cleanup) via the skill and let the human choose.

- [ ] **Step 2: Respect the human's choice.**

    Do not push, merge, or open a PR without explicit approval — CLAUDE.md rule.

---

## Self-Review Log

**Spec coverage check:** Every spec section maps to at least one task:
- Overview → covered by goal + architecture above.
- Commit 1 § Enum and identifier wiring → Task 2.
- Commit 1 § Translation file → Task 3.
- Commit 1 § Language-select grid → Task 4.
- Commit 2 § What the user sees → Task 7 step 6 (visual verification).
- Commit 2 § Files + why hardcoded → Task 6 and Task 7.
- Commit 2 § Candidate translations → Task 7 step 2 (table is duplicated inline in the task).
- Commit 2 § Button helpers → Task 6.
- Commit 2 § Button-construction pattern → Task 7.
- Testing and verification → Task 4 step 3-4, Task 7 steps 5-6.
- Explicit non-scope → enforced by the narrow file lists in Tasks 2-8; Task 5 step 1 explicitly checks git status for unrelated files before staging.
- Plain-language summary → Tasks 5 and 8 each reuse the spec's summary text.

**Placeholder scan:** The `<SMALL_TEXT_CONSTANT>` in Task 6 is a deliberate placeholder the implementer must resolve in Task 6 Step 1 against existing theme constants. The three Turkish short strings in Task 2 Step 4 are flagged as placeholders to cross-check against the glossary from Task 1, not left undefined.

**Type/name consistency:** The helper is consistently named `make_lang_button_with_note` across Tasks 6 and 7. The `Turkish` enum variant, the `"tr-TR"` language identifier, and the `"TÜRKÇE"` button label are used consistently across Tasks 2, 3, 4, and 7.
