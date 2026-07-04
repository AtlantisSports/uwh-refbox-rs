# Unit 8 Behaviour Catalog — Language UI chrome

**Working file for Task 3. Do NOT edit AUDIT-PLAN.md directly.**
**Audit commits:** `848138c` (11 new languages + CJK/Thai fonts + grid-selection page) and `ea151ac` (Turkish + UNVERIFIED marker + action-bar script-font fix)
**Date produced:** 2026-05-15
**Status:** @proposed (all entries)

---

## Out-of-scope acknowledgement

The following files appear in the diff for these two commits but are NOT cataloged below because
they are outside the audit boundary per the spec §1 scope decision and the 2026-05-12 scope
reduction:

- **15 `.ftl` translation files** in `refbox/translations/*/refbox.ftl` (en-US, es, fr, de-DE,
  id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, tl-PH, th-TH, zh-CN, tr-TR) — translation
  accuracy deferred indefinitely pending native-speaker review; file presence and key structure
  accepted as-is.
- **2 font binary files**: `refbox/resources/NotoSansCJK-Subset.otf`,
  `refbox/resources/NotoSansThai-Subset.ttf` — bundled subsets, accepted as-is.
- **2 regen scripts**: `scripts/regen-cjk-font.py`, `scripts/regen-thai-font.py` — out of scope
  per spec §1.
- **Justfile** recipe additions for the regen scripts — out of scope per spec §1.
- **`docs/superpowers/` files** committed in `ea151ac` (original design spec + plan for
  Turkish/UNVERIFIED) — acknowledged as the oracle for that commit's in-scope work.

---

## Section A — Behaviour Catalog

---

### Feature 1: Language selection page

---

##### B8.1 — Language enum extended to 14 languages

- **What it does (plain English):** The app now recognises 14 languages instead of 3. An
  operator launching the app on a system configured in Korean, Japanese, Mandarin, Italian,
  German, Tagalog, Indonesian, Dutch, Malay, Portuguese, or Thai will have that language
  automatically detected and loaded (or fall back to English if the detected locale is
  unrecognised). The language list visible on the Language selection page covers all 14.
- **Where in the diff:** `refbox/src/app/languages.rs` lines 6–21 (enum variants);
  lines 27–40 (`as_lang_id` arms); lines 48–80 (`from_lang_id` branches). Commit `848138c`.
- **Why it might be intentional:** The refbox is used at international tournaments. Expanding
  from 3 languages to 14 makes it accessible to referees and operators whose first language is
  not English, French, or Spanish.
- **Why it might be slop:** Not applicable — this is the core feature purpose of both commits.
  No slop pattern matches.
- **Linked scenario(s):** S8.1.1 (grid renders 14 buttons), S8.1.2 (navigate to Language page)
- **Recommendation:** keep — the core data model for the feature; everything else in this catalog
  depends on it.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.2 — Language serialization (serde derives added)

- **What it does (plain English):** The language the operator chose is now saved to disk between
  sessions. When the app shuts down and restarts, it opens in the same language that was last
  selected. Before this change, the language choice was lost on exit and the app would reset to
  whatever the system locale reported.
- **Where in the diff:** `refbox/src/app/languages.rs` line 6 (`serde::Serialize,
  serde::Deserialize` added to the derive list). `refbox/src/config.rs` line 101
  (`pub language: Option<Language>` added to the `Config` struct). Commit `848138c`.
- **Why it might be intentional:** Tournament operators configure language once; they should not
  need to re-select every session. Saving to `confy`'s config file is the standard persistence
  mechanism in this codebase for all other settings.
- **Why it might be slop:** `Option<Language>` could be a smell if `None` is never a meaningful
  state. However, `None` here correctly means "first launch / no language yet saved" — distinct
  from explicitly selecting English. This is NOT slop. See Section C for the full argument.
- **Linked scenario(s):** S8.1.7 (saved language persists across restart)
- **Recommendation:** keep — persistence is required for the feature to be usable.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.3 — Config migration handles absent `language` field

- **What it does (plain English):** If an operator has an existing saved configuration from
  before this feature was added, the app will still start cleanly and simply treat the missing
  language field as "no language saved" (English default). The operator's other settings
  (portal URL, sound settings, screen layout, etc.) are all preserved; nothing is lost.
- **Where in the diff:** `refbox/src/config.rs` lines 111–161 (`Config::from_toml_value`
  migration block), specifically the `language,` field added to the destructure-and-reconstruct
  pattern. The existing `..Default::default()` expansion handles the absent key via serde's
  `Default`. Commit `848138c`.
- **Why it might be intentional:** The codebase already uses a custom migration pattern for
  `Config` to survive forward/backward changes; `language` follows the established convention.
- **Why it might be slop:** Not applicable — follows the existing migration pattern exactly. No
  slop pattern matches.
- **Linked scenario(s):** none (backend-only; operator-facing outcome is the default-English
  behaviour on first launch, covered by S8.1.6)
- **Recommendation:** keep — standard migration behaviour; required for safe upgrade from
  existing installations.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.4 — Language page navigation entry point (App Options → Language page)

- **What it does (plain English):** On the App Options settings page, tapping the "language"
  button now opens a dedicated Language selection page instead of cycling through languages
  one at a time. The button label still reads "language" / "this-language" (in the current
  locale), but its action changed from cycle-through to open-page.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` lines 590–596 (the
  `make_app_config_page` language row button changed from `Some(Message::CycleParameter(
  CyclingParameter::Language,))` to `Some(Message::ChangeConfigPage(ConfigPage::Language))`).
  `refbox/src/app/message.rs` line 416 (`ConfigPage::Language` variant added). Commit `848138c`.
- **Why it might be intentional:** A grid-based page allows the operator to see and choose all
  14 languages at once, rather than cycling through them blindly; far more usable at a
  tournament with time pressure.
- **Why it might be slop:** Not applicable — this is the intended UX improvement.
- **Linked scenario(s):** S8.1.2 (navigate from App page to Language page)
- **Recommendation:** keep — the grid-selection page is the intended interaction model.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.5 — Language page initialises pending/original tracking on open

- **What it does (plain English):** When the operator opens the Language selection page, the
  app internally records the current active language as both the "original" (what to revert
  to if Cancel is pressed) and the "pending" selection (what to highlight blue on the grid).
  This happens before the page renders, so the correct language button is always highlighted
  blue the moment the page appears.
- **Where in the diff:** `refbox/src/app/mod.rs` lines 1316–1321 (the `ChangeConfigPage(Language)`
  arm initialises `settings.original_language` and `settings.pending_language`). Also
  `refbox/src/app/view_builders/configuration.rs` lines 37–38 (`pending_language: Option<Language>`
  and `original_language: Option<Language>` fields on `EditableSettings`). The `EditableSettings`
  constructor at `refbox/src/app/mod.rs` lines 1304–1305 initialises both to `None`. Commit
  `848138c`.
- **Why it might be intentional:** Preview-then-confirm interaction requires a staging field
  separate from the committed config value.
- **Why it might be slop:** The `.unwrap_or(Language::English)` fallback in
  `make_language_select_page` at line 1023–1024 is technically defensive — the page is only
  reachable when `ChangeConfigPage(Language)` runs, which always sets both fields to `Some`.
  However, the fallback is harmless, follows existing codebase convention for `Option<Language>`
  extraction, and the alternative (panicking on `None`) would be worse. Not flagged as slop.
- **Linked scenario(s):** S8.1.3 (current language pre-selected on page open), S8.1.6
  (first-launch defaults to English)
- **Recommendation:** keep — required for the preview-then-confirm interaction model.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.6 — Language button tap previews selection (blue highlight, no immediate switch)

- **What it does (plain English):** Tapping a language button on the Language selection page
  highlights that button blue but does NOT change the running app's language. The operator can
  tap around freely to see which option they want before committing. The UI text everywhere
  else in the app stays in the current language until the operator taps Done (or Restart, if
  a restart is required).
- **Where in the diff:** `refbox/src/app/mod.rs` lines 1743–1745 (`Message::SelectLanguage(lang)`
  handler sets `pending_language` only). `refbox/src/app/view_builders/configuration.rs`
  lines 1060–1080 (`lang_btn` closure applies `blue_selected_button` style when `lang == selected`,
  `light_gray_button` otherwise). Commit `848138c`.
- **Why it might be intentional:** Preview-before-commit is standard UX for settings pages with
  side effects. For CJK/Thai languages, the side effect is an app restart, so seeing the choice
  before committing is especially important.
- **Why it might be slop:** Not applicable — this is the central UX goal of the page.
- **Linked scenario(s):** S8.1.4 (tap language → blue highlight without UI switch)
- **Recommendation:** keep — the preview model is the intended interaction.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.7 — Cancel returns to App Options without changing language

- **What it does (plain English):** Tapping the Cancel button (red, bottom-left) on the
  Language selection page discards the pending selection and navigates back to the App Options
  page. The running language and all other settings remain unchanged.
- **Where in the diff:** `refbox/src/app/mod.rs` lines 1747–1775 (`Message::LanguageSelectComplete
  { canceled: true }` path — clears `pending_language` and `original_language`, sets page back
  to `ConfigPage::App`). `refbox/src/app/view_builders/configuration.rs` lines 1222–1226
  (Cancel button sends `LanguageSelectComplete { canceled: true }`). Commit `848138c`.
- **Why it might be intentional:** All settings pages in refbox have Cancel. Consistent
  navigation is important for operators under pressure.
- **Why it might be slop:** Not applicable.
- **Linked scenario(s):** S8.1.5 (Cancel returns to App page, language unchanged)
- **Recommendation:** keep — standard settings-page cancel behaviour.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.8 — Grid layout: 14 language buttons in alphabetical-by-romanized-name order

- **What it does (plain English):** The language selection grid shows all 14 languages arranged
  in four rows of four, alphabetically by romanized name: Row 1 = BAHASA INDONESIA, BAHASA
  MELAYU, DEUTSCH, ENGLISH; Row 2 = ESPAÑOL, FILIPINO, FRANÇAIS, 한국어; Row 3 = ITALIANO,
  NEDERLANDS, 日本語, PORTUGUÊS; Row 4 = ภาษาไทย, TÜRKÇE, 中文, (empty). The final slot is
  intentionally blank.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` lines 1100–1210 (the
  four `row![...]` blocks in `make_language_select_page`). Commit `848138c` established the 14
  slots; commit `ea151ac` added TÜRKÇE in the correct alphabetical slot.
- **Why it might be intentional:** Alphabetical order by romanized name is specified in the
  commit body; it allows operators to scan predictably. The one empty cell at position [4,4] is
  the result of 15 total entries with one having its own slot — this is by design, not an error.

  **Note on BAHASA change (cross-entry link to B8.22):** In `848138c`, Bahasa Indonesia and
  Bahasa Melayu used the two-line `make_multi_label_button` helper (showing "BAHASA" on line 1
  and "INDONESIA"/"MELAYU" on line 2, at default text size). In `ea151ac`, these were replaced
  with the new `lang_btn_note` helper using `NameLines::OneLineSmall` (showing "BAHASA INDONESIA"
  or "BAHASA MELAYU" as one small-text line above the UNVERIFIED note). This is an operator-visible
  shape change cataloged separately as B8.22.
- **Why it might be slop:** The empty cell in row 4 is not slop; it reflects the count of
  supported languages. The alphabetical ordering comment in the source is a "comment explaining
  what the code does" (slop checklist item), but it also explains the *why* (romanized-name
  sort order), so it serves a documentation purpose beyond what the code makes obvious.
- **Linked scenario(s):** S8.1.1 (grid renders 14 buttons in correct order)
- **Recommendation:** keep — correct, specified layout.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.9 — Action-bar Cancel/Done/Restart buttons render in target language's script

- **What it does (plain English):** The Cancel, Done, and Restart buttons at the bottom of the
  Language selection page always display their text in the font that matches the *previewed*
  language, not the *current* app language. For example, if the app is currently in Korean (CJK
  font) and the operator taps ENGLISH on the grid, the Cancel button immediately shows "CANCEL"
  rendered in the Latin (Roboto) font — not a tofu box. Conversely, if the app is in English and
  the operator taps 한국어, the Cancel button shows "취소" in the CJK font.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` lines 1039–1056.
  Commit `848138c` introduced `selected_font` with `_ => None` as the default (no explicit font
  override for Latin selections). Commit `ea151ac` added `latin_font` and changed the match arm
  to `_ => Some(latin_font)`, fixing the tofu-box regression for Latin text under a CJK app
  default. The final state is lines 1039–1056 in the post-`ea151ac` file.

  **Cross-unit note:** The `selected_font` mechanism is introduced in `848138c` and refined in
  `ea151ac`. Both contributions are in scope for Unit 8. The interim state (where Latin text
  would show as tofu under CJK locale) existed only between the two commits; the audit inherits
  only the final state.
- **Why it might be intentional:** Without explicit font overrides, iced renders text using the
  app's default font. When the app is running in a CJK locale, the default font is the CJK
  subset, which does not contain Latin letters. The `latin_font` explicit override ensures Latin
  text always renders correctly regardless of the app's current default font.
- **Why it might be slop:** Not applicable — this is a necessary correctness fix. The tofu-box
  failure mode was real (confirmed in the `ea151ac` commit body).
- **Linked scenario(s):** S8.1.8 (action-bar text renders in target language's script font)
- **Recommendation:** keep — the tofu fix is required for correct rendering.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.10 — Hardcoded action-bar strings (cancel_text / done_text / restart_text)

- **What it does (plain English):** The Cancel, Done, and Restart-to-Apply button labels on
  the Language selection page are written directly into the code for each language (e.g. "TAMAM"
  for Turkish Done, "취소" for Korean Cancel) rather than being loaded through the translation
  file system. This means those three strings are not part of the `.ftl` files that can be
  updated without recompiling.
- **Where in the diff:** `refbox/src/app/languages.rs` lines 53–138 (`cancel_text()`,
  `done_text()`, `restart_text()` methods, each with a 14-arm match). Turkish arms added in
  commit `ea151ac` (lines 96, 116, 136). All others in `848138c`.
- **Why it might be intentional:** The Language page is the one screen that must display text in
  the *target* language's script — not the current app locale. The `fl!()` macro always renders
  in the operator's current locale. Hardcoding these three short strings per language is the
  correct workaround: the same reason the button labels ("DEUTSCH", "中文", etc.) are hardcoded.
  The oracle (`2026-04-17-turkish-language-and-unverified-label-design.md` §"Why the notes are
  hardcoded") explicitly endorses this approach.
- **Why it might be slop:** "String literals that aren't in translations" is on the slop-catching
  checklist. These strings do bypass the translation system — intentionally. The justification
  (fl! cannot render in an arbitrary target language) is technically sound and matches the oracle.
  This is NOT slop in the structural sense. **Accuracy of the translations is deferred** — the
  three strings per language have not been reviewed by native speakers; they are placeholders
  that may need correction. This is noted here and should be flagged to the operator for awareness.
- **Linked scenario(s):** S8.1.8 (action-bar text renders correctly; indirectly covers presence
  of these strings)
- **Recommendation:** keep (presence and structure) — accuracy deferred pending native-speaker
  review; the structural approach is correct and matches the oracle.
- **Decision (Step 4):** @proposed
- **Notes from review:** Accuracy deferred. Operator should be informed that the Cancel/Done/
  Restart labels for non-English/Spanish/French languages are placeholders, same as the
  UNVERIFIED note strings.

---

### Feature 2: Restart-required indicator and flow

---

##### B8.11 — font_family_id classifier (backend type plumbing)

- **What it does (plain English):** Behind the scenes, every language is assigned to one of
  three font groups: CJK (Korean, Japanese, Mandarin), Thai, or Latin (everything else).
  Switching within the same group can happen instantly; switching across groups requires the
  app to close and reopen because the app's default font is set once at launch and cannot
  change while running.
- **Where in the diff:** `refbox/src/app/mod.rs` lines 2446–2451 (module-level `font_family_id`
  function); `refbox/src/app/view_builders/configuration.rs` lines 1009–1015 (private duplicate
  in the `configuration` module). Commit `848138c`.

  **Note (possible duplication):** `font_family_id` is defined twice — once in `mod.rs` (used
  in the `LanguageSelectComplete` handler) and once in `configuration.rs` (used in
  `make_language_select_page` to decide whether to show DONE vs. RESTART). These are identical
  functions at different visibility levels. This is worth a slop flag — see Section C.
- **Why it might be intentional:** The view-builder module needs to know the restart requirement
  to render the right button, and the update handler needs to know it to trigger the restart.
  Two usages at different abstraction layers.
- **Why it might be slop:** **Duplicate function definition.** The slop pattern "re-implementations
  of existing utilities" partially applies: both copies of `font_family_id` are identical. A
  single shared helper would be cleaner. However, the private duplicate in `configuration.rs`
  exists because Rust's module visibility prevented easy sharing at the time of writing. This is
  a minor code-quality issue; it does not affect operator-facing behaviour. Flag as
  `findings-backlog` candidate — low priority, can be refactored when touching these files next.
- **Linked scenario(s):** none (backend classifier; operator-visible outcome is the DONE vs.
  RESTART button, covered by B8.12/B8.13 and S8.2.1–S8.2.3)
- **Recommendation:** keep (functionally correct) — minor duplication noted in Section C as
  findings-backlog candidate.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.12 — Same-family language selection shows green DONE button

- **What it does (plain English):** When the operator previews a language that uses the same
  font family as the currently running app (e.g. switching between any two Latin-script languages,
  or between Korean and Japanese), the confirm button shows "DONE" in green. Tapping it applies
  the new language immediately without restarting.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` lines 1226–1241 (the
  `needs_restart` conditional that chooses between blue RESTART-TO-APPLY and green DONE buttons).
  `refbox/src/app/mod.rs` lines 1747–1775 (the `LanguageSelectComplete { canceled: false }` path
  when `needs_restart` is false — calls `crate::request_language(...)` to hot-swap in place).
  Commit `848138c`.
- **Why it might be intentional:** Hot-swap within the same font family requires no restart and
  provides a better operator experience — the change is immediate.
- **Why it might be slop:** Not applicable.
- **Linked scenario(s):** S8.2.1 (Latin → Latin shows green DONE), S8.2.3 (CJK → CJK shows
  green DONE)
- **Recommendation:** keep — correct and deliberate.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.13 — Cross-family language selection shows blue RESTART TO APPLY button

- **What it does (plain English):** When the operator previews a language that uses a different
  font family from the current app (e.g. English → Korean, or Korean → English), the confirm
  button changes from green "DONE" to blue "RESTART TO APPLY" (displayed in the target language's
  script — e.g. "재시작하여 적용" when 한국어 is selected). This tells the operator that tapping
  the button will close and reopen the app.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` lines 1055–1057
  (`needs_restart` calculation) and lines 1228–1237 (blue RESTART-TO-APPLY button rendering
  when `needs_restart` is true). Commit `848138c`.
- **Why it might be intentional:** iced's default font is fixed at startup. A switch between
  font families requires a restart to load the correct default font. Showing this in advance
  prevents the operator from being surprised.
- **Why it might be slop:** Not applicable — necessary disclosure.
- **Linked scenario(s):** S8.2.2 (Latin → CJK shows blue RESTART TO APPLY), S8.2.4
  (Latin → Thai shows blue RESTART TO APPLY)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.14 — Restart flow: config saved, sim child killed, fresh exe spawned, process exits

- **What it does (plain English):** When the operator taps "RESTART TO APPLY" (cross-family
  language change), the app: (1) saves the new language to the config file on disk, (2) stops
  the LED-panel simulator process if it is running, (3) launches a fresh copy of itself as a
  new process, and (4) closes itself. The fresh copy starts with the new language loaded from
  the config file and with the correct font family as its default. To the operator, the app
  appears to restart in the new language.
- **Where in the diff:** `refbox/src/app/mod.rs` lines 1750–1766 (the `needs_restart = true`
  branch inside `LanguageSelectComplete { canceled: false }`). Commit `848138c`.
- **Why it might be intentional:** The entire restart sequence is necessary because iced 0.13
  does not support changing the default font at runtime.
- **Why it might be slop:** `confy::store(crate::APP_NAME, None, &self.config).unwrap()` at
  line 1754 uses `.unwrap()` on an I/O operation. This is a real failure mode: if the config
  file cannot be written (e.g. disk full, permissions error), the process exits without
  completing the save, and the fresh instance starts with the old language. This parallels
  Unit 3 finding #2. Flagged as `findings-backlog` — see Section C (B8.16 is the dedicated
  entry for the unwrap pattern).

  `std::env::current_exe()` failure is silently ignored with `if let Ok(exe) = ...`. This means
  if the executable path cannot be determined (rare but possible), the current process still
  exits (line 1765) but no new process is spawned — leaving the app closed with no recovery
  path for the operator. This is a low-probability but high-severity failure mode. Flagged.
- **Linked scenario(s):** S8.2.5 (RESTART TO APPLY saves, kills sim, spawns fresh, exits)
- **Recommendation:** keep (functional) — the unwrap and the silent-failure on `current_exe`
  are filed separately in findings-backlog.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.15 — Fresh exe reads saved language and starts with correct font

- **What it does (plain English):** When the app starts up, it checks whether a language was
  previously saved to disk. If so, it loads the saved language and selects the matching font
  family as the app's default before rendering any UI. This is what makes the restart actually
  work: the fresh instance does not use the system locale but the operator's explicit choice.
- **Where in the diff:** `refbox/src/main.rs` lines 389–407 (reads `config.language`, applies
  it to `LANGUAGE_OVERRIDE`, then selects `default_font_family`/`default_font_weight` based on
  `saved_language`). `refbox/src/main.rs` lines 421–427 (font registration: bundles all three
  font families). Commit `848138c`.
- **Why it might be intentional:** Required for the restart to produce the correct font.
- **Why it might be slop:** Not applicable.
- **Linked scenario(s):** S8.2.5 (covered — the fresh exe's startup behaviour is part of the
  restart scenario)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.16 — confy::store unwrap on language save (findings-backlog candidate)

- **What it does (plain English):** When the operator confirms a language change (both the
  hot-swap path and the restart path), the app saves the new setting to disk using a call that
  will crash the entire application if the disk is full or the config file is unwritable. No
  error is shown to the operator — the app simply exits.
- **Where in the diff:** `refbox/src/app/mod.rs` line 1754:
  `confy::store(crate::APP_NAME, None, &self.config).unwrap()`. Commit `848138c`.
- **Why it might be intentional:** This pattern (`confy::store(...).unwrap()`) is used in
  five other places in `mod.rs` (lines 1405, 1442, 1450, 1888, 1901) for other settings saves.
  It is the established codebase convention — not unique to the language save.
- **Why it might be slop:** **Fallback path for a real error case.** The `.unwrap()` on an I/O
  operation means a disk-full or permission error produces a panic with an obscure message rather
  than a graceful failure. For the restart path, the failure is particularly bad: the process
  exits (`std::process::exit(0)`) after the unwrap, so the fresh instance starts but with the
  *old* language in config. Parallels Unit 3 finding #2 exactly. The pattern appears widely, so
  fixing it here without fixing all instances would be inconsistent; the correct fix is a
  workspace-wide error-handling improvement. **Recommend findings-backlog** with a suggested
  branch `chore/refbox/config-save-error-handling`.
- **Linked scenario(s):** none (edge-case failure mode, not walkthrough-verifiable without
  deliberately corrupting disk state)
- **Recommendation:** findings-backlog — unwrap on confy::store is a pre-existing pattern in
  the codebase; fix as a separate workspace-wide improvement.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.17 — Hot-swap language apply (same-family Done path)

- **What it does (plain English):** When the operator taps Done for a same-family language
  (green button), the app switches the running UI to the new language immediately — all button
  labels, page titles, and status text update in place without any restart. The app page
  returns to App Options and the new language is immediately visible.
- **Where in the diff:** `refbox/src/app/mod.rs` lines 1768–1773 (the `needs_restart = false`
  path: calls `crate::request_language(...)` then clears pending/original and navigates back to
  `ConfigPage::App`). Commit `848138c`.
- **Why it might be intentional:** Instant switch without restart is the expected behaviour for
  same-family languages — far better UX than an unnecessary restart.
- **Why it might be slop:** Not applicable.
- **Linked scenario(s):** S8.2.1 (Latin→Latin Done path applies instantly)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

### Feature 3: UNVERIFIED marker on language buttons

---

##### B8.18 — Turkish added as 15th language

- **What it does (plain English):** Turkish ("TÜRKÇE") appears on the language selection grid
  in the correct alphabetical position (row 4, column 2 — between Thai and Mandarin). Turkish
  uses the Latin script with diacritics (ı, İ, ö, ü, ğ, ş, ç), which the existing Roboto font
  covers; no additional font file is needed. Turkish restarts are not required for Latin-to-Latin
  switches.
- **Where in the diff:** `refbox/src/app/languages.rs` lines 22, 39, 72–73, 96, 116, 136,
  157–158 (variant + wiring across all methods). `refbox/src/app/view_builders/configuration.rs`
  Turkish button in row 4. Commit `ea151ac`.
- **Why it might be intentional:** Adds Turkish UWH community reach; designed and spec'd in
  oracle `2026-04-17-turkish-language-and-unverified-label-design.md`.
- **Why it might be slop:** Not applicable — fully specified in the oracle.
- **Linked scenario(s):** S8.3.1 (TÜRKÇE button appears in correct grid position)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.19 — NameLines enum and make_lang_button_with_note helper

- **What it does (plain English):** A new button-building helper was added to support the
  two-line layout needed for the UNVERIFIED marker: a language name on one line and a small
  "(UNVERIFIED)" note in that language's script on the line below. A companion enum
  (`NameLines`) lets call sites specify whether the name is displayed at full size
  (`OneLine`) or small size (`OneLineSmall`, used for long names like "BAHASA INDONESIA").
- **Where in the diff:** `refbox/src/app/view_builders/shared_elements.rs` lines 927–978
  (`NameLines` enum + `make_lang_button_with_note` function). Commit `ea151ac`.
- **Why it might be intentional:** The oracle specifies a new helper for this button shape. The
  existing `make_multi_label_button` handles two lines at the same size; the new helper handles
  a name at full/small size plus a smaller note line.
- **Why it might be slop:** The `with_font` closure at lines 956–957 could be seen as a
  micro-abstraction, but it serves a real purpose (applying the optional font consistently to
  both widgets) and is not speculative. `NameLines` has exactly two variants and both are used.
  Not slop.

  **Cross-unit note (Unit 6 reconcile):** `make_multi_label_button` in `shared_elements.rs`
  was the subject of B6.3 in Unit 6's audit. Unit 6 confirmed the outer container-wrap as
  intentional; it explicitly noted that `848138c`'s per-line container refinements were "Unit 8
  territory." The new `make_lang_button_with_note` is a separate helper — not a modification of
  `make_multi_label_button`. The Unit 6 reconcile is closed by B8.25 (the per-line
  `container(text).center_x` refinement in `make_multi_label_button` itself).
- **Linked scenario(s):** S8.3.2 (language buttons with UNVERIFIED note show two-line layout)
- **Recommendation:** keep — purposeful helper, not speculative abstraction.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.20 — UNVERIFIED note on 12 language buttons (presence and structure)

- **What it does (plain English):** Every language button on the grid except English, Spanish,
  and French shows a small note below the language name indicating the translation has not been
  reviewed by a native speaker. Each note is written in that language's own script (e.g. the
  Korean button shows "한국어" above "(검증되지 않음)" in Korean script; the German button shows
  "DEUTSCH" above "(NICHT VERIFIZIERT)" in Latin). The note text is the same across all app
  locales — it is hardcoded per language, not translated through the locale system.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` — the 12 `lang_btn_note`
  call sites in `make_language_select_page` (replacing the 12 `lang_btn` calls from `848138c`).
  Commit `ea151ac`.
- **Why it might be intentional:** Honest signalling to operators that a translation has not
  been native-speaker reviewed. The oracle specifies this feature explicitly. Hardcoding in
  each button's target language is the correct approach (same reason as B8.10 — fl! renders
  in the current locale, not the button's target language).
- **Why it might be slop:** "String literals that aren't in translations" — same deliberate
  exception as B8.10. **Accuracy of the 12 UNVERIFIED strings is deferred**, same treatment
  as the action-bar strings. NOT slop in the structural sense.
- **Linked scenario(s):** S8.3.2 (UNVERIFIED note appears on non-verified buttons),
  S8.3.3 (ENGLISH shows no UNVERIFIED note), S8.3.4 (ESPAÑOL shows no UNVERIFIED note),
  S8.3.5 (FRANÇAIS shows no UNVERIFIED note)
- **Recommendation:** keep — presence and structure correct per oracle; accuracy deferred.
- **Decision (Step 4):** @proposed
- **Notes from review:** Accuracy of note strings deferred pending native-speaker review.

---

##### B8.21 — English, Spanish, French exempt from UNVERIFIED marker

- **What it does (plain English):** Three languages — English, Spanish, and French — do NOT
  show any UNVERIFIED note. Their buttons display only the language name with no second line.
  These are the three languages considered verified (translated and reviewed).
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` — the three `lang_btn`
  calls for English, Spanish, and French remain unchanged from `848138c`. Comment in
  `ea151ac` diff explicitly notes "English, Spanish, and French are considered verified."
  Commit `ea151ac`.
- **Why it might be intentional:** These three translations were produced with more rigour or
  are considered accurate enough for unqualified use. Treating them differently from the others
  is an explicit operator-facing signal.
- **Why it might be slop:** Not applicable — deliberate exemption per oracle.
- **Linked scenario(s):** S8.3.3 (ENGLISH no note), S8.3.4 (ESPAÑOL no note), S8.3.5
  (FRANÇAIS no note)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.22 — Bahasa buttons reshaped: two-line → one small-text line + note

- **What it does (plain English):** Before commit `ea151ac`, the Bahasa Indonesia and Bahasa
  Melayu buttons each showed their name in two lines at default size ("BAHASA" on top, "INDONESIA"
  or "MELAYU" below). After `ea151ac`, each of these buttons shows the full name as a single
  small-text line ("BAHASA INDONESIA" or "BAHASA MELAYU") above the UNVERIFIED note. The visual
  result is a two-line button (name small + note small) rather than a two-line name (no note).
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` — the two Bahasa
  `make_multi_label_button` calls from `848138c` are replaced in `ea151ac` by `lang_btn_note`
  calls with `NameLines::OneLineSmall`. Commit `ea151ac`.
- **Why it might be intentional:** The oracle spec (`2026-04-17-turkish-language-and-unverified-label-design.md`
  §"Button helpers") mentions the Bahasa buttons as needing special treatment because the name
  is long. The spec anticipates a `TwoLine(T, T)` variant for the name; the implementation
  chose `OneLineSmall(T)` instead, achieving the same visual goal (fitting the full name in one
  button without overflow) via a different mechanism.
- **Why it might be slop:** The spec described the Bahasa buttons as "Two-line native name +
  small unverified note" (§"What the user sees": "BAHASA / INDONESIA / note" as three lines).
  The implementation produces a different shape: "BAHASA INDONESIA" (one small-text line) /
  "(BELUM DIVERIFIKASI)" (one small-text line). This is an intentional deviation from the spec
  that simplifies the layout. **Recommend walkthrough-decides** — the operator should confirm
  whether the one-small-text-line shape is preferable to the original three-line shape. Flag
  for Task 5 carve-out.
- **Linked scenario(s):** S8.3.6 (Bahasa buttons show single-line name + note shape)
- **Recommendation:** walkthrough-decides — shape change from oracle spec; confirm with operator.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.23 — UNVERIFIED note font matches button's target language script

- **What it does (plain English):** The UNVERIFIED note text on each button is rendered in the
  correct font for that language's script. CJK language notes use the CJK font, Thai uses the
  Thai font, and all Latin-script notes use the Latin font. This ensures the note text is
  readable in any operating environment, regardless of which language the app is currently
  running in.
- **Where in the diff:** `refbox/src/app/view_builders/configuration.rs` — the `font` parameter
  passed to each `lang_btn_note` call site: CJK buttons pass `Some(cjk_font)`, Thai buttons
  pass `Some(thai_font)`, Latin buttons pass `Some(latin_font)`. `refbox/src/app/view_builders/
  shared_elements.rs` lines 955–957 (`with_font` closure applies the font to both the name text
  and the note text). Commit `ea151ac`.
- **Why it might be intentional:** Same rationale as B8.9 — the note must render in the target
  language's script, not the app's current default font.
- **Why it might be slop:** Not applicable — consistent with B8.9.
- **Linked scenario(s):** S8.3.2 (note text renders in correct script)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

### Feature 4: Button-text damage-tracking workaround

---

##### B8.24 — Period-text container wrap in make_game_time_button

- **What it does (plain English):** The period name displayed in the game-time button (e.g.
  "FIRST HALF", "HALF TIME") is now wrapped in a container that constrains the text to only
  as wide as its content. Previously the text widget occupied the full available width. This
  change prevents a rendering bug in iced 0.13: when the period name changes (e.g. from "FIRST
  HALF" to "SECOND HALF"), old glyph pixels from the longer name would bleed through in the
  space to the right of the shorter name. The fix ensures the rendering engine only redraws
  the exact area the text occupies.
- **Where in the diff:** `refbox/src/app/view_builders/shared_elements.rs` lines 368–386
  (the `make_time_view_row` closure's period-text changed from a plain right-aligned `text`
  widget to a `container(text(...).width(Shrink))...`). Commit `848138c`.

  **Cross-unit note (Unit 5/7):** `make_game_time_button` is the same function that B7.C1
  (portal health indicator threading) notes as a shared-file concern. The current post-`ea151ac`
  state of the function is what Unit 8 audits; the Unit 7 and Unit 5 contributions are already
  in master below this commit.
- **Why it might be intentional:** The commit body and inline comment explicitly name the
  iced 0.13 damage-tracking bug as the justification. This is not defensive code; it is a
  known workaround for a framework defect.
- **Why it might be slop:** Not applicable — documented workaround for a confirmed framework bug.
- **Linked scenario(s):** S8.4.1 (period text does not bleed after language/period change)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.25 — make_button / make_smaller_button / make_small_button text shrink-wrap

- **What it does (plain English):** Three button-building helpers used throughout the app
  (`make_button`, `make_smaller_button`, `make_small_button`) were updated so their label text
  widget is only as wide as the text itself, wrapped in a centering container. The visual result
  to the operator is identical — buttons still look centered. The change prevents the same
  glyph-bleed rendering bug as B8.24 from occurring on any button that uses these helpers when
  its label changes.
- **Where in the diff:** `refbox/src/app/view_builders/shared_elements.rs` lines 877–900
  (`make_button`), lines 890–900 (`make_smaller_button`), lines 980–992 (`make_small_button`).
  Also: the lifetime bound tightening `Message: Clone` → `Message: 'a + Clone` on all three
  helper signatures. Commit `848138c`.

  **Cross-unit note (Unit 6 reconcile — closed):** Unit 6 explicitly noted B6.3's
  `make_multi_label_button` per-line container refinements as "Unit 8 territory." This entry
  closes that reconcile: the per-line `container(t).center_x(Length::Fill)` pattern applied
  to `make_multi_label_button`'s lines (below) is the same sweep; the three single-label
  helpers here are the broader part of the same workaround.
- **Why it might be intentional:** Systematic sweep to apply the damage-tracking workaround
  across all button helpers before adding the language page (which would introduce heavy
  label-switching due to per-language text content).
- **Why it might be slop:** Not applicable — documented workaround, applied consistently.
- **Linked scenario(s):** S8.4.2 (existing config-page buttons still render correctly after
  the sweep)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.26 — make_multi_label_button per-line container wrap

- **What it does (plain English):** The helper used to render two-line buttons (e.g. "BAHASA /
  INDONESIA", "END / TIMEOUT") was updated so each text line is wrapped in its own centering
  container, with each text widget sized to fit its content exactly. The visual result is
  identical. Same motivation as B8.25.
- **Where in the diff:** `refbox/src/app/view_builders/shared_elements.rs` lines 903–923
  (`make_multi_label_button` — the inner `column!` changed from `text(...).align_x(Center).
  width(Fill)` per line to `container(text(...).width(Shrink)).center_x(Fill)` per line).
  Commit `848138c`.

  **Cross-unit note (Unit 6 — closes reconcile):** This is the specific contribution that
  Unit 6 B6.3 noted as Unit 8 territory. The outer container-wrap was `8a8d018`'s contribution
  (Unit 6 scope); the per-line shrink-wrap is `848138c`'s contribution (Unit 8 scope). Both
  coexist in the current code. This audit closes the reconcile.
- **Why it might be intentional:** Consistent with B8.25; applies the same workaround to the
  two-label button helper.
- **Why it might be slop:** Not applicable.
- **Linked scenario(s):** S8.4.3 (multi-label buttons on existing screens still render centered)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.27 — Message: 'a + Clone lifetime bound tightening

- **What it does (plain English):** An internal type constraint on several button-building
  helpers was made slightly more precise. This has no operator-visible effect but is required
  for the Rust compiler to accept the new button patterns introduced by the damage-tracking
  sweep. Operators see no change.
- **Where in the diff:** `refbox/src/app/view_builders/shared_elements.rs` — the `Message: Clone`
  generic bound on `make_button`, `make_smaller_button`, `make_small_button` changed to
  `Message: 'a + Clone`. Commit `848138c`.
- **Why it might be intentional:** The `'a` lifetime bound is required when a captured `&'a str`
  is stored inside a container that must outlive the function call — a Rust correctness
  requirement, not a design choice.
- **Why it might be slop:** Not applicable — technically required change.
- **Linked scenario(s):** none (backend-only; no operator-visible effect)
- **Recommendation:** keep.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

### Code-only changes

---

##### B8.C1 — Window-position changes in main.rs

- **What it does (plain English):** Two window-position settings were added or changed in the
  app startup code: (1) the LED-panel simulator window is pinned to a fixed position on screen
  (top-left corner, 40 pixels down), and (2) the main refbox window is set to open centered on
  whatever screen it appears on. Before this change, both windows opened at the OS default
  position.
- **Where in the diff:** `refbox/src/main.rs` line 268 (`position: window::Position::Specific(
  iced::Point::new(0.0, 40.0))` — simulator window); line 444 (`position: window::Position::
  Centered` — main window). Commit `848138c`.
- **Why it might be intentional:** When developing with the simulator, having the simulator
  window at a predictable position avoids it covering the main window. Centering the main window
  improves first-impression UX on new installations.
- **Why it might be slop:** Neither change is mentioned in the commit body. The commit body
  describes languages, fonts, the grid page, scripts, and README removal — no mention of
  window positioning. This matches the slop-catching checklist pattern of "changes that weren't
  mentioned in the commit description." The simulator pin is a developer-workflow convenience
  baked into the shipped binary; whether it belongs there is a question for the operator.
  **Recommend walkthrough-decides** — flag for Task 5 carve-out.
- **Linked scenario(s):** none (window position is observable but not a Gherkin-testable
  user-facing behaviour)
- **Recommendation:** walkthrough-decides — operator confirms whether both positions are
  wanted in the shipped binary.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

##### B8.C2 — Dead-code impl Cyclable for Language

- **What it does (plain English):** The code contains a full set of cycle-through logic for
  the Language type (the sequence English → French → Spanish → ... → Turkish → English), but
  nothing in the running app ever uses it. The button that previously triggered language cycling
  was changed in `848138c` to open the language selection page instead, removing the only
  caller. Despite having no caller, the `impl Cyclable for Language` block was kept and even
  extended in both commits (adding Mandarin/Korean/... in `848138c`, adding Turkish in
  `ea151ac`).
- **Where in the diff:** `refbox/src/app/languages.rs` lines 4 (`use super::Cyclable` import)
  and lines 144–163 (`impl Cyclable for Language { fn next(...) {...} }`). `refbox/src/app/
  message.rs` — `CyclingParameter::Language` variant removed in `848138c` (the caller side
  confirmed gone). No code anywhere calls `.cycle()` or `CycleParameter(CyclingParameter::Language)`.
- **Why it might be intentional:** None visible. The impl was kept and maintained across both
  commits even though the caller was removed in the same commit (`848138c`) that extended it.
  This suggests the AI generated the extension as part of a mechanical pattern (add new language
  → add to all match arms) without noticing the impl had become dead.
- **Why it might be slop:** **Helper function never called from real code.** This is a direct
  match of the slop-catching checklist item. The `impl Cyclable for Language` and the
  `use super::Cyclable` import in `languages.rs` serve no purpose in the current codebase.
  Clippy would flag the unused import; the impl itself is only not flagged because `Cyclable`
  has external implementors. **Surgical revert: delete the `impl` block and the `use super::
  Cyclable` import in `languages.rs`.** Compiler and tests confirm no callers.
- **Linked scenario(s):** none (dead code; no operator-visible behaviour)
- **Recommendation:** delete — dead code, no callers, no operator-facing effect.
- **Decision (Step 4):** @proposed
- **Notes from review:**

---

### Cross-unit reconcile (not in scope here — noted for Final Integration)

These are not catalog entries with decision tags. They are deferral notes pointing to known
open items that are out of scope for this audit unit but must be addressed before Final Integration.

**B8.X1 — `team-ref-list` orphan key in 15 locale files**
The `team-ref-list` translation key exists in all 15 `.ftl` files (added as a latent key in
`848138c`) but is not referenced by any running code (the feature that would use it has not
shipped yet). Unit 5 flagged this for cleanup. Suggested branch:
`chore/refbox/remove-unused-team-ref-list-keys`. Low priority — unused translation keys are
inert at runtime. Not a catalog entry for Unit 8; filed at Unit 5.

**B8.X2 — `portal-row-attempt-suffix` key absent from 14 non-English locales**
Unit 7 added `portal-row-attempt-suffix` to the English locale only. The other 14 locales
(including all those added in `848138c`) do not have this key. At runtime, Fluent falls back
to en-US for missing keys, so the portal display still works but in English regardless of
selected language. Unit 7 noted this for a separate cleanup branch or Final Integration
absorption. Suggested branch: `chore/refbox/portal-row-attempt-suffix-14-locales`.

**B8.X3 — `make_multi_label_button` Unit 6 cross-unit reconcile (CLOSED by B8.26)**
Unit 6 B6.3 explicitly flagged `848138c`'s per-line container refinements to
`make_multi_label_button` as "Unit 8 territory." B8.26 above catalogs and closes this
reconcile. No further action needed.

---

## Section B — Scenarios

All scenarios carry `@proposed` pending Step 4 review.

---

### Feature: Language selection page

```gherkin
Feature: Language selection page

  # S8.1.1
  @proposed
  Scenario: Language selection grid shows all 14 languages in romanized alphabetical order
    Given the operator has the App Options settings page open
    When the operator taps the language button in the App Options page
    Then the Language selection page opens
    And row 1 shows BAHASA INDONESIA, BAHASA MELAYU, DEUTSCH, ENGLISH (left to right)
    And row 2 shows ESPAÑOL, FILIPINO, FRANÇAIS, 한국어 (left to right)
    And row 3 shows ITALIANO, NEDERLANDS, 日本語, PORTUGUÊS (left to right)
    And row 4 shows ภาษาไทย, TÜRKÇE, 中文, and one empty slot (left to right)

  # S8.1.2
  @proposed
  Scenario: Language button in App Options opens the Language selection page
    Given the operator is on the App Options settings page
    When the operator taps the language button
    Then the Language selection page appears
    And the operator is NOT taken to any other settings page

  # S8.1.3
  @proposed
  Scenario: Current language is pre-selected (blue) when Language page opens
    Given the operator has previously selected DEUTSCH and tapped Done
    When the operator opens the Language selection page again
    Then the DEUTSCH button is highlighted blue
    And all other language buttons are light gray

  # S8.1.4
  @proposed
  Scenario: Tapping a language button previews the selection without changing the app
    Given the Language selection page is open
    And ENGLISH is currently highlighted blue
    When the operator taps the ITALIANO button
    Then the ITALIANO button turns blue
    And the ENGLISH button returns to light gray
    And the rest of the app UI still shows English text (not Italian)

  # S8.1.5
  @proposed
  Scenario: Cancel returns to App Options without changing the active language
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped DEUTSCH (DEUTSCH is highlighted blue)
    When the operator taps the Cancel button (red, bottom-left)
    Then the App Options page appears
    And the app is still running in ENGLISH
    And no language change has been applied

  # S8.1.6
  @proposed
  Scenario: First launch with no saved language pre-selects English
    Given no language has ever been saved in the app config
    When the operator opens the Language selection page
    Then the ENGLISH button is highlighted blue
    And all other language buttons are light gray

  # S8.1.7
  @proposed
  Scenario: Chosen language persists after app restart
    Given the operator selects FRANÇAIS and taps Done on the Language selection page
    And the app UI updates to French
    When the operator closes the app and opens it again
    Then the app opens in French (FRANÇAIS)
    And the Language selection page shows FRANÇAIS pre-selected

  # S8.1.8
  @proposed
  Scenario: Action-bar buttons render text in the target language's script font
    Given the app is currently running in 한국어 (CJK font as default)
    And the Language selection page is open with 한국어 pre-selected
    When the operator taps the ENGLISH button
    Then the Cancel button shows "CANCEL" in readable Latin text (not tofu boxes)
    And the Done button shows "DONE" in readable Latin text (not tofu boxes)
```

---

### Feature: Restart-required indicator and flow

```gherkin
Feature: Restart-required indicator and flow

  # S8.2.1
  @proposed
  Scenario: Switching between two Latin-script languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps DEUTSCH
    Then the confirm button (bottom-right) shows "FERTIG" in green

  # S8.2.2
  @proposed
  Scenario: Switching from a Latin language to a CJK language shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps 한국어
    Then the confirm button (bottom-right) shows "재시작하여 적용" in blue
    And the Done/green button is not visible

  # S8.2.3
  @proposed
  Scenario: Switching between two CJK languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in 한국어
    When the operator taps 日本語
    Then the confirm button (bottom-right) shows a green Done button in Japanese ("完了")

  # S8.2.4
  @proposed
  Scenario: Switching from a Latin language to Thai shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps ภาษาไทย
    Then the confirm button (bottom-right) shows a blue Restart button in Thai text

  # S8.2.5
  @proposed
  Scenario: Tapping Restart saves the language, closes the app, and opens a fresh instance
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped 한국어 (blue Restart button is visible)
    When the operator taps the blue Restart button
    Then the app closes
    And a new instance of the app opens
    And the new instance is running in 한국어 with Korean as the UI language
    And the Language selection page shows 한국어 pre-selected if reopened
```

---

### Feature: UNVERIFIED marker on language buttons

```gherkin
Feature: UNVERIFIED marker on language buttons

  # S8.3.1
  @proposed
  Scenario: TÜRKÇE button appears in row 4 column 2 of the language grid
    Given the operator opens the Language selection page
    Then the TÜRKÇE button is visible in row 4, column 2 (between ภาษาไทย and 中文)

  # S8.3.2
  @proposed
  Scenario: Language buttons for unverified translations show a small note in the button
    Given the operator opens the Language selection page
    Then the TÜRKÇE button shows "(DOĞRULANMAMIŞ)" in small text below "TÜRKÇE"
    And the 中文 button shows "(未验证)" in small text below "中文"
    And the 한국어 button shows "(검증되지 않음)" in small text below "한국어"
    And the DEUTSCH button shows "(NICHT VERIFIZIERT)" in small text below "DEUTSCH"

  # S8.3.3
  @proposed
  Scenario: ENGLISH button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ENGLISH button shows only "ENGLISH" with no note below it

  # S8.3.4
  @proposed
  Scenario: ESPAÑOL button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ESPAÑOL button shows only "ESPAÑOL" with no note below it

  # S8.3.5
  @proposed
  Scenario: FRANÇAIS button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the FRANÇAIS button shows only "FRANÇAIS" with no note below it

  # S8.3.6
  @proposed
  Scenario: Bahasa Indonesia and Bahasa Melayu buttons show name as one small-text line plus note
    Given the operator opens the Language selection page
    Then the Bahasa Indonesia button shows "BAHASA INDONESIA" as a single smaller-text line
    And below it shows "(BELUM DIVERIFIKASI)" in small text
    And the Bahasa Melayu button shows "BAHASA MELAYU" as a single smaller-text line
    And below it shows "(BELUM DISAHKAN)" in small text
```

---

### Feature: Button-text damage-tracking workaround

```gherkin
Feature: Button-text damage-tracking workaround

  # S8.4.1
  @proposed
  Scenario: Period name in game-time button does not show ghost pixels from the previous name
    Given the operator is on any screen that shows the game-time button
    And the current period is displayed as "FIRST HALF"
    When the game advances to "SECOND HALF"
    Then the game-time button shows "SECOND HALF" cleanly with no remnant pixels from "FIRST HALF"

  # S8.4.2
  @proposed
  Scenario: Existing config pages still display button text correctly after the button helper changes
    Given the operator is on the Main Config page
    When the operator navigates through the Game Options, App Options, Display, and Sound settings pages
    Then all button labels on each page render centered and readable
    And no button shows truncated, overlapping, or bleeding text

  # S8.4.3
  @proposed
  Scenario: Two-line buttons on existing screens still render with both lines centered
    Given the operator navigates to any config page that shows a two-line button
    When the operator views the button
    Then both lines of text appear centered within the button
    And neither line is clipped or misaligned
```

---

## Section C — Slop-catching pass notes

### Entries that match slop-catching checklist items

**B8.C2 (dead `impl Cyclable for Language`)** — Direct match: "helper functions never called
from real code." The `CyclingParameter::Language` caller was removed in `848138c` in the same
commit that extended the `impl Cyclable for Language` block to cover 11 new variants. The
mechanical extension of `Language::next()` (to keep the exhaustive match compiling) ran without
anyone noticing the whole impl had become dead. Both commits extended the dead impl further.
The `use super::Cyclable` import in `languages.rs` is also orphaned. **Verdict: delete.**

**B8.C1 (window-position changes)** — Match: changes not mentioned in the commit body. The
`848138c` commit body is detailed and covers 8 distinct topics — it does not mention window
positioning. The simulator-window pin (`Specific(0.0, 40.0)`) and main-window center
(`Centered`) are both present in the diff but absent from the stated intent. **Verdict:
walkthrough-decides.** Not definitively slop — both could be intentional developer-workflow
improvements — but require operator confirmation.

**B8.16 (confy::store unwrap)** — Match: "fallback path for an error case that causes silent
failure." The `.unwrap()` on disk I/O is a pre-existing pattern (five other call sites in
`mod.rs`), so this is not unique slop introduced by these commits. However, the language-save
unwrap is at a particularly bad location: the restart path exits the process after the save,
so a save failure leaves the config in the wrong state with no recovery. **Verdict:
findings-backlog.** Workspace-wide fix needed; suggest branch `chore/refbox/config-save-error-handling`.

**B8.14 (silent `current_exe` failure)** — Borderline match: "fallback path for an error
case." The `if let Ok(exe) = std::env::current_exe()` silently swallows the failure. If the
exe path cannot be determined, `std::process::exit(0)` still runs (line 1765), leaving the
operator with a closed app and no new instance. Not flagged as a separate B-entry (it is
already noted in B8.14's "Why it might be slop" field). Filed as findings-backlog via B8.14.

**B8.11 (duplicate `font_family_id`)** — Partial match: "re-implementations of existing
utilities." The function is defined identically in two modules. Not flagged as a standalone
B-entry because the functional behaviour is correct and the duplication is module-visibility-driven.
Noted in B8.11; findings-backlog candidate.

**Commit-body inaccuracy of `848138c`** — The commit body claims to remove "the obsolete
'Raspberry Pi deployment' README section." The actual diff does not touch `README.md`. The
section was already absent before this commit. This is a commit-message inaccuracy (the author
described something that did not happen), not a code behaviour issue. No catalog entry
warranted. Noted here for the record; the principal may wish to amend the commit body note in
the retroactive ADR (ADR 023).

### Entries that look borderline but are NOT slop

**B8.10 / B8.20 (hardcoded action-bar and UNVERIFIED strings)** — These strings bypass the
translation system. The slop checklist flags "string literals that aren't in translations."
However, the oracle explicitly documents and endorses this approach: `fl!()` renders in the
current locale, which would defeat the purpose of a per-language self-labeling button. The
bypass is deliberate and correct. NOT slop in the structural sense. Accuracy of the strings is
deferred (separate concern from structure).

**B8.2 (`Option<Language>` in Config)** — `Option` in a config field can suggest "defensive
code at an internal boundary" (the checklist's first item). However, `None` here genuinely
means "first launch / no saved language" — a state distinct from `Some(Language::English)`.
The `Option` carries semantic meaning. NOT slop.

**B8.5 (`.unwrap_or(Language::English)` in make_language_select_page)** — The view function
uses `unwrap_or` because both fields are `Option`. The page is only reachable via
`ChangeConfigPage(Language)`, which always sets both to `Some`. So the `None` branch is
technically unreachable in practice. This is "defensive code at an internal boundary" on the
checklist. However, the defensive pattern protects against future refactoring that might allow
the page to be reached with `None` fields, and the fallback (English default) is safe. Not
flagged as a primary slop finding — noted in B8.5 as "follows existing convention."

**Font registration in main.rs (B8.15)** — All three font families are always bundled and
registered at startup, even if only one will be used. This is "configuration that the user has
no way to change" in a narrow sense. However, all three must be registered at startup because
the language page renders buttons in all three scripts simultaneously (the grid shows CJK,
Thai, and Latin buttons at once). Pre-loading all three is required. NOT slop.

### Translation-file and font-binary acknowledgement

The 15 `.ftl` files and 2 font binaries are present in the diff and are out of scope per the
spec §1 scope decision. They are noted here as "acknowledged; accepted as-is pending native-speaker
review." No catalog entries were created for them.

---

## Section D — Summary for the principal

```
Total entries: 29
By Feature:
  Feature 1 (Language selection page):       B8.1–B8.10    = 10 entries
  Feature 2 (Restart-required indicator):    B8.11–B8.17   =  7 entries
  Feature 3 (UNVERIFIED marker):             B8.18–B8.23   =  6 entries
  Feature 4 (Damage-tracking workaround):    B8.24–B8.27   =  4 entries
  Code-only changes:                         B8.C1–B8.C2   =  2 entries
  Cross-unit reconcile:                      B8.X1–B8.X3   =  3 notes (no decision tag)

By Recommendation:
  keep             = 21 (B8.1–B8.10 all keep; B8.12–B8.15; B8.17–B8.21; B8.23–B8.27)
  delete           =  1 (B8.C2 — dead Cyclable impl)
  walkthrough-decides = 2 (B8.22 — Bahasa shape; B8.C1 — window position)
  findings-backlog =  2 (B8.11 — duplicate font_family_id note; B8.16 — confy unwrap)

Scenarios: 22 across 4 Feature blocks
  Feature 1: S8.1.1–S8.1.8  (8 scenarios)
  Feature 2: S8.2.1–S8.2.5  (5 scenarios)
  Feature 3: S8.3.1–S8.3.6  (6 scenarios)
  Feature 4: S8.4.1–S8.4.3  (3 scenarios — regression coverage, not new behaviour)

Carve-out candidates (need individual operator decision in Task 5):
  B8.16  — confy::store unwrap: findings-backlog (consistent with Unit 3 finding #2 pattern)
  B8.22  — Bahasa button shape: walkthrough-decides (changed from oracle spec; operator confirms)
  B8.C1  — Window-position: walkthrough-decides (not mentioned in commit body; keep vs. revert)
  B8.C2  — Dead Cyclable impl: delete (no callers; surgical revert recommended before walkthrough)

Cross-unit notes added:
  B8.9   — selected_font: `848138c` interim state (Latin → tofu under CJK) noted; `ea151ac`
           final state is the oracle. Both commits in scope; audit inherits final state.
  B8.11  — font_family_id: defined twice (mod.rs + configuration.rs), identical logic.
           Minor duplication; findings-backlog candidate.
  B8.19  — make_lang_button_with_note: Unit 6 B6.3 cross-unit reconcile noted.
           New helper is distinct from make_multi_label_button.
  B8.24  — make_game_time_button: Unit 5/7 shared-file cross-unit note recorded.
  B8.25  — make_button/smaller/small sweep: Unit 6 reconcile partially closed here.
  B8.26  — make_multi_label_button per-line wrap: Unit 6 B6.3 reconcile CLOSED by this entry.

Unexpected findings beyond the spec's anticipated breakdown:
  1. Duplicate `font_family_id` function (B8.11 note) — spec did not anticipate this;
     filed as findings-backlog candidate.
  2. Silent `current_exe()` failure in restart path (B8.14 note) — spec did not call this out;
     filed as findings-backlog candidate alongside B8.16's unwrap.
  3. Bahasa button shape changed from oracle spec (B8.22) — spec anticipated "three-line" shape
     (BAHASA / INDONESIA / note) per `ea151ac` oracle §"What the user sees"; implementation
     chose `OneLineSmall` instead. This is the most significant deviation from the design oracle
     found during catalog construction.
  4. Commit-body inaccuracy of `848138c` (README claim) — already noted in AUDIT-PLAN.md
     history trace; confirmed during diff reading. No catalog entry; noted in Section C.
```
