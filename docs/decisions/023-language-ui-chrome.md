# ADR 023: Language UI chrome

**Status:** Accepted (retroactive)
**Date:** 2026-05-15
**Audit unit:** 8 — Grid-select page + UNVERIFIED marker
**Audit branch:** `audit/refbox/language-ui-chrome`

## Context

This ADR documents behaviour added to the refbox between 2026-04-18 (commit `848138c`) and 2026-04-18 (commit `ea151ac`) with AI assistance. The behaviour was audited 2026-05-15 (Unit 8 of the AI Code Audit per `AUDIT-PLAN.md`) and the surviving parts are recorded here.

Before this work the refbox supported three UI languages — English, French, Spanish — and only the Latin script. Language selection was a cycle-through-languages button on the App Options page that rotated through `English → French → Spanish → English` via the `CyclingParameter::Language` mechanism. The two commits audited under Unit 8 expand the supported set to 15 languages spanning Latin, CJK (Korean, Japanese, Mandarin), and Thai scripts; replace the cycle button with a dedicated grid-selection page; add an UNVERIFIED marker on each language whose translation has not been native-speaker reviewed; bundle Noto Sans CJK and Noto Sans Thai font subsets at build time; and apply an iced-0.13 damage-tracking workaround to every button helper in the app.

Translation file content for the 14 non-English locales and the bundled font binaries are explicitly **out of scope** of this audit. Their accuracy is deferred indefinitely pending native-speaker review, per the playbook's scope reduction of 2026-05-12.

## Decision

The refbox UI presents a dedicated **Language selection page** reachable from the App Options grid via a "language" button (the button that previously triggered cycling). The page is the only way for the operator to change the active UI language. The selection page applies preview-then-confirm semantics: tapping a language paints it blue without changing the running UI; Done commits; Cancel reverts. When the chosen language requires a different font family than the current app's default, the action bar's right button changes from green "DONE" to blue "RESTART TO APPLY" and tapping it persists the language, kills the simulator child, spawns a fresh copy of the executable, and exits.

The audit organised the decision into four coherent operator-facing concerns (one Gherkin Feature each, with scenarios in `refbox/tests/features/`).

### Feature 1 — Language selection page

The grid layout (4×4 with 14 buttons + 1 empty slot), preview-then-confirm interaction, alphabetical-by-romanized-name ordering, first-launch English default, persistence across restart, and the action-bar script-font fix that ensures Cancel/Done/Restart text renders in the *target* language's script font (so Latin text under a CJK app default does not render as tofu boxes).

```gherkin
Feature: Language selection page

  Scenario: Language selection grid shows all 14 languages in romanized alphabetical order
    Given the operator has the App Options settings page open
    When the operator taps the language button in the App Options page
    Then the Language selection page opens
    And row 1 shows BAHASA INDONESIA, BAHASA MELAYU, DEUTSCH, ENGLISH (left to right)
    And row 2 shows ESPAÑOL, FILIPINO, FRANÇAIS, 한국어 (left to right)
    And row 3 shows ITALIANO, NEDERLANDS, 日本語, PORTUGUÊS (left to right)
    And row 4 shows ภาษาไทย, TÜRKÇE, 中文, and one empty slot (left to right)

  Scenario: Language button in App Options opens the Language selection page
    Given the operator is on the App Options settings page
    When the operator taps the language button
    Then the Language selection page appears
    And the operator is NOT taken to any other settings page

  Scenario: Current language is pre-selected (blue) when Language page opens
    Given the operator has previously selected DEUTSCH and tapped Done
    When the operator opens the Language selection page again
    Then the DEUTSCH button is highlighted blue
    And all other language buttons are light gray

  Scenario: Tapping a language button previews the selection without changing the app
    Given the Language selection page is open
    And ENGLISH is currently highlighted blue
    When the operator taps the ITALIANO button
    Then the ITALIANO button turns blue
    And the ENGLISH button returns to light gray
    And the rest of the app UI still shows English text (not Italian)

  Scenario: Cancel returns to App Options without changing the active language
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped DEUTSCH (DEUTSCH is highlighted blue)
    When the operator taps the Cancel button (red, bottom-left)
    Then the App Options page appears
    And the app is still running in ENGLISH
    And no language change has been applied

  Scenario: Chosen language persists after app restart
    Given the operator selects FRANÇAIS and taps Done on the Language selection page
    And the app UI updates to French
    When the operator closes the app and opens it again
    Then the app opens in French (FRANÇAIS)
    And the Language selection page shows FRANÇAIS pre-selected

  Scenario: Action-bar buttons render text in the target language's script font
    Given the app is currently running in 한국어 (CJK font as default)
    And the Language selection page is open with 한국어 pre-selected
    When the operator taps the ENGLISH button
    Then the Cancel button shows "CANCEL" in readable Latin text (not tofu boxes)
    And the Done button shows "DONE" in readable Latin text (not tofu boxes)
```

### Feature 2 — Restart-required indicator and flow

Switching between Latin / CJK / Thai font families requires a restart because iced 0.13 picks the default font at startup and cannot change it at runtime. The action bar's right button reflects this: green "DONE" for same-family selections (which hot-swap via `request_language(..)`), blue "RESTART TO APPLY" with target-script text for cross-family selections. The restart sequence saves the chosen language to `config.toml`, kills the LED-panel simulator child process if present, spawns a fresh `current_exe()`, and exits.

```gherkin
Feature: Restart-required indicator and flow

  Scenario: Switching between two Latin-script languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps DEUTSCH
    Then the confirm button (bottom-right) shows "FERTIG" in green

  Scenario: Switching from a Latin language to a CJK language shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps 한국어
    Then the confirm button (bottom-right) shows "재시작하여 적용" in blue
    And the Done/green button is not visible

  Scenario: Switching between two CJK languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in 한국어
    When the operator taps 日本語
    Then the confirm button (bottom-right) shows a green Done button in Japanese ("完了")

  Scenario: Switching from a Latin language to Thai shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps ภาษาไทย
    Then the confirm button (bottom-right) shows a blue Restart button in Thai text

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

### Feature 3 — UNVERIFIED marker on language buttons

Every language button except English, Spanish, and French shows a small `(UNVERIFIED)`-equivalent note in its language's own script beneath the language name. The note is hardcoded per-language at the call site (not routed through `fl!`, because `fl!` renders in the current locale and would defeat the purpose of a per-language self-labeling button). Bahasa Indonesia and Bahasa Melayu use a single-small-text-line name shape because their names are too long for the default size when the UNVERIFIED note is shown below. English, Spanish, and French are exempt because their translations have been reviewed.

```gherkin
Feature: UNVERIFIED marker on language buttons

  Scenario: TÜRKÇE button appears in row 4 column 2 of the language grid
    Given the operator opens the Language selection page
    Then the TÜRKÇE button is visible in row 4, column 2 (between ภาษาไทย and 中文)

  Scenario: Language buttons for unverified translations show a small note in the button
    Given the operator opens the Language selection page
    Then the TÜRKÇE button shows "(DOĞRULANMAMIŞ)" in small text below "TÜRKÇE"
    And the 中文 button shows "(未验证)" in small text below "中文"
    And the 한국어 button shows "(검증되지 않음)" in small text below "한국어"
    And the DEUTSCH button shows "(NICHT VERIFIZIERT)" in small text below "DEUTSCH"

  Scenario: ENGLISH button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ENGLISH button shows only "ENGLISH" with no note below it

  Scenario: ESPAÑOL button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ESPAÑOL button shows only "ESPAÑOL" with no note below it

  Scenario: FRANÇAIS button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the FRANÇAIS button shows only "FRANÇAIS" with no note below it

  Scenario: Bahasa Indonesia and Bahasa Melayu buttons show name as one small-text line plus note
    Given the operator opens the Language selection page
    Then the Bahasa Indonesia button shows "BAHASA INDONESIA" as a single smaller-text line
    And below it shows "(BELUM DIVERIFIKASI)" in small text
    And the Bahasa Melayu button shows "BAHASA MELAYU" as a single smaller-text line
    And below it shows "(BELUM DISAHKAN)" in small text
```

### Feature 4 — Button-text damage-tracking workaround

Every button helper in the app (`make_button`, `make_smaller_button`, `make_small_button`, `make_multi_label_button`, and the period-text container inside `make_game_time_button`) was rewritten to wrap a `width(Shrink)` text widget inside a centering container. This is the iced-0.13 damage-tracking workaround for old glyph pixels bleeding through when text content changes script (e.g. when the operator changes language and the time bar period name changes from `FIRST HALF` to its Korean, Japanese, Mandarin, or Thai equivalent). Scenarios here are regression-coverage rather than new-behaviour: they verify the sweep didn't break existing screens.

```gherkin
Feature: Button-text damage-tracking workaround

  Scenario: Period name in game-time button does not show ghost pixels from the previous name
    Given the operator is on any screen that shows the game-time button
    And the current period is displayed as "FIRST HALF"
    When the game advances to "SECOND HALF"
    Then the game-time button shows "SECOND HALF" cleanly with no remnant pixels from "FIRST HALF"

  Scenario: Existing config pages still display button text correctly after the button helper changes
    Given the operator is on the Main Config page
    When the operator navigates through the Game Options, App Options, Display, and Sound settings pages
    Then all button labels on each page render centered and readable
    And no button shows truncated, overlapping, or bleeding text

  Scenario: Two-line buttons on existing screens still render with both lines centered
    Given the operator navigates to any config page that shows a two-line button
    When the operator views the button
    Then both lines of text appear centered within the button
    And neither line is clipped or misaligned
```

### Code-only behaviours (no Gherkin scenarios)

- **Window-position changes in `main.rs` (B8.C1).** The simulator window is pinned to `Position::Specific((0.0, 40.0))` and the main refbox window opens `Position::Centered`. Neither change was mentioned in the `848138c` commit body — this was scope-creep that rode along with the language work. The walkthrough confirmed both placements are improvements (centered main window aids first-impression UX; the pinned simulator stays out of the way during dev), so both are kept.
- **`Language` enum gains 12 new variants** with serde Serialize/Deserialize derives for persistence, plus `as_lang_id` / `from_lang_id` matching and hardcoded action-bar strings (`cancel_text` / `done_text` / `restart_text`) per language. Same-family hot-swap uses `request_language(..)` to apply Fluent locale changes in-place.
- **`Config.language: Option<Language>`** persists the chosen language via `confy`. `None` distinctly means "first launch, no saved language" — the app falls back to the system locale or English.
- **`font_family_id` classifier** maps each Language to one of three font families (Latin = 0, CJK = 1, Thai = 2). Used by `needs_restart` detection in `LanguageSelectComplete` and by the action-bar's `selected_font` selection in `make_language_select_page`.
- **`NameLines` enum and `make_lang_button_with_note` helper** in `shared_elements.rs` render the two-line UNVERIFIED-marker button shape (name on top, note below). `NameLines::OneLineSmall` is used for the long Bahasa names; `NameLines::OneLine` for the rest.

## Consequences

- The operator can pick a UI language by name from a visible grid rather than cycling blindly through three options.
- Switching between Latin and non-Latin scripts requires a restart and the operator sees this clearly on the action-bar button before committing.
- Translations not yet reviewed by a native speaker are clearly marked with a per-language `(UNVERIFIED)` note in the language's own script, so operators know which translations to trust.
- Every button in the app is now resistant to glyph-bleed when its text content changes script — a small overhead in widget structure (a centering container wrapping a `width(Shrink)` text) for a fix to a hard-to-spot visual bug.
- The 15-locale `.ftl` translation files are committed but their accuracy is **not warranted** by this audit. Future work will review each locale per language.
- The bundled Noto Sans CJK and Noto Sans Thai font subsets add ~157 KB to the binary size but eliminate the prior requirement that operators install system fonts on Raspberry Pi OS for Mandarin / CJK / Thai rendering.
- The simulator child process and the language-select restart flow now require the executable path to be discoverable via `std::env::current_exe()`; if the executable is moved or deleted at runtime, the restart leaves the app closed with no recovery path. This is a low-probability but high-severity failure mode flagged in Findings backlog.

## What was removed during audit

- **Dead `impl Cyclable for Language`** (catalog entry B8.C2). The `CyclingParameter::Language` caller was removed in `848138c` when the grid-select page replaced cycle-through-languages, but the `Cyclable` impl was extended in both audited commits anyway (adding all 11 new languages plus Turkish to the cycle chain) despite having no remaining callers. Removed in audit commit `cfe3204` (`refbox/src/app/languages.rs`: the `impl Cyclable for Language { fn next(..) {...} }` block and the orphan `use super::Cyclable;` import). No operator-facing behaviour change.

The window-position changes (B8.C1) were considered for revert but the operator confirmed during walkthrough that both placements are improvements. No revert; no follow-up branch needed.

## What was not verified

- **Translation accuracy** for the 14 non-English locales is deferred to native-speaker review (explicitly out of scope per the playbook's 2026-05-12 scope reduction). The `.ftl` files are accepted as-is for now.
- **Bundled font subset completeness** for the CJK and Thai font binaries is accepted as-is from the audited commits. The regen scripts (`scripts/regen-cjk-font.py`, `scripts/regen-thai-font.py`) are out of scope; their correctness has not been audited.
- **First-launch behaviour from a freshly-deleted config (S8.1.6)** is `@tested_inconclusive` in `refbox/tests/features/language-selection-page.feature`. The test would require deleting `~/.config/refbox/default-config.toml` and relaunching, which was out of scope for the walkthrough. The destructure-and-reconstruct pattern in `Config::from_toml_value` was inspected at Task 4 Step 4.2 and correctly handles the absent `language` field via serde's `Default` for `Option<Language>` (which is `None`).
- **Multi-tournament regression of the damage-tracking sweep.** The walkthrough exercised the five config sub-pages (Main, Game Options, App Options, Display, Sound) plus the in-game time-bar across multiple language-script transitions. Production use over the next tournament will be the final acceptance check for any subtle visual regressions on screens not exercised here (e.g. overlay, scoresheet generation, multi-remote management).

## Audit reference

- **Audit branch:** `audit/refbox/language-ui-chrome` (local-only until Final Integration)
- **Audit design spec:** `docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md` (commit `34f4bef` on `docs/workspace/backlog-adrs`)
- **Audit per-unit plan:** `docs/superpowers/plans/2026-05-15-audit-unit-8-language-ui-chrome.md` (commit `0b4997d` on `docs/workspace/backlog-adrs`)
- **Original commits audited:** `848138c` (11 new languages + CJK/Thai fonts + grid-selection page) and `ea151ac` (Turkish + UNVERIFIED marker + action-bar script-font fix)
- **Original design oracle for `ea151ac` only:** `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md`
- **Original implementation plan for `ea151ac` only:** `docs/superpowers/plans/2026-04-17-turkish-language-and-unverified-label.md`
- **No prior design oracle for `848138c`** — this ADR retroactively becomes its design record.
- **Walkthrough verification:** `refbox/tests/features/language-selection-page.feature`, `language-restart-flow.feature`, `language-unverified-marker.feature`, `button-damage-tracking.feature` (all committed on the audit branch with 21 `@user_verified @tested_pass` scenarios and 1 `@user_verified @tested_inconclusive` for S8.1.6).
- **Findings filed in `AUDIT-PLAN.md` From-Unit-8:**
  - `confy::store(..).unwrap()` pattern on language save → branch suggestion `chore/refbox/config-save-error-handling`
  - Duplicate `font_family_id` function in `mod.rs` and `configuration.rs` → branch suggestion `refactor/refbox/font-family-id-shared`
  - Silent `current_exe()` failure in restart path → roll into `chore/refbox/config-save-error-handling`
- **Cross-unit reconcile (filed for separate-branch resolution at Final Integration):**
  - `team-ref-list` orphan key in 15 locales (Unit 5's note) → `chore/refbox/remove-unused-team-ref-list-keys`
  - `portal-row-attempt-suffix` 14-locale population (Unit 7's note) → `chore/refbox/portal-row-attempt-suffix-14-locales` or Final-Integration absorption
  - `make_multi_label_button` per-line wrap (Unit 6's "Unit 8 territory" punt) — **closed** by this audit as catalog entry B8.26
