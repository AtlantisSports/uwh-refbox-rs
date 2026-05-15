# Audit Unit 8 — Language UI chrome: scope & shape design

**Date:** 2026-05-15
**Status:** proposed (awaiting operator approval)
**Unit:** 8 in the AI Code Audit playbook (`AUDIT-PLAN.md`)
**Audit branch:** `audit/refbox/language-ui-chrome` (to be cut from `origin/master` at `089c98d`)
**Oracle:** asymmetric — see §3
**Retroactive ADR target:** `docs/decisions/023-language-ui-chrome.md` (next free; 012, 019–022 are taken by prior in-flight or audit branches)

This document scopes Unit 8 of the AI Code Audit. It is the brainstorm output that
feeds `superpowers:writing-plans` for the granular per-unit plan at
`docs/superpowers/plans/2026-05-15-audit-unit-8-language-ui-chrome.md`.

---

## 1. Goal & scope boundary

**Goal.** Audit the operator-facing UI changes introduced by commits `848138c` (11
languages + CJK/Thai fonts + grid-selection page) and `ea151ac` (Turkish + UNVERIFIED
marker on language buttons) against the playbook's scope reduction of
2026-05-12: language *content* (the contents of the 15 `.ftl` files plus the bundled
font binaries) is out of scope; the *UI chrome* — the page that lets the operator pick
a language, the restart-required flow, the UNVERIFIED marker, and the
damage-tracking layout sweep that rode along with these commits — is in scope.

Produce a Unit 8 branch (`audit/refbox/language-ui-chrome`) that holds locally
until Final Integration, alongside a new retroactive ADR.

### In scope

- **Grid-selection page rendering and navigation** in
  `refbox/src/app/view_builders/configuration.rs` (the `make_language_select_page`
  function and its call site from the App page).
- **Language state plumbing** in `refbox/src/app/mod.rs` (`EditableSettings`
  pending/original tracking, `ChangeConfigPage(Language)` initialization,
  `SelectLanguage` and `LanguageSelectComplete` handlers including the restart
  flow that kills the sim child and spawns a fresh exe).
- **`Message` and `ConfigPage` additions** in `refbox/src/app/message.rs`
  (`SelectLanguage`, `LanguageSelectComplete`, `ConfigPage::Language`, plus the
  removal of `CyclingParameter::Language`).
- **The new `Language` enum variants** (12 added across the two commits) in
  `refbox/src/app/languages.rs` and the new `as_lang_id` / `from_lang_id` arms.
- **Hardcoded action-bar translations** in `languages.rs`
  (`cancel_text()` / `done_text()` / `restart_text()`) — **presence and structure
  only**; translation accuracy is deferred (same treatment as `.ftl` content
  awaiting native-speaker review).
- **Hardcoded `(UNVERIFIED)` note strings** at the grid page call sites
  (e.g. `(DOĞRULANMAMIŞ)`, `(未验证)`, `(검증되지 않음)`) — **presence and
  structure only**; accuracy deferred.
- **Two-line button rendering for the UNVERIFIED marker**:
  `make_lang_button_with_note` and the `NameLines` enum in
  `refbox/src/app/view_builders/shared_elements.rs`.
- **Action-bar script-font fix**: the `selected_font` selection in
  `make_language_select_page` that renders Cancel/Done/Restart in the *target*
  language's script (the tofu-box fix from `ea151ac`).
- **Restart-required detection and flow**: `font_family_id()` comparison plus
  the spawn-fresh-exe / kill-sim-child / `std::process::exit(0)` sequence.
- **Damage-tracking layout sweep** across all button helpers in
  `shared_elements.rs`: `make_button`, `make_smaller_button`, `make_small_button`,
  `make_multi_label_button`, and the period-text container inside
  `make_game_time_button`. These were all switched from `width(Fill)` text to
  `width(Shrink)` text wrapped in a centering container as one coherent
  workaround for an iced-0.13 damage-tracking bug.
- **Language persistence** in `refbox/src/config.rs` (`Config.language:
  Option<Language>`).
- **Font registration + default-font selection at startup** in
  `refbox/src/main.rs` — the part that loads `Roboto-Medium.ttf`,
  `NotoSansCJK-Subset.otf`, and `NotoSansThai-Subset.ttf` and chooses the default
  font family based on `config.language`.
- **Window-position changes in `main.rs`** (`window::Position::Specific(0.0,
  40.0)` for the simulator window and `window::Position::Centered` for the main
  window). Catalog as in-scope; walkthrough decides keep vs. send to Findings-Backlog.
- **Dead-code `impl Cyclable for Language`** plus the now-orphan
  `use super::Cyclable` import. Surgical revert anticipated.

### Out of scope

- **All 15 locale `.ftl` files' contents** — translation accuracy is deferred
  indefinitely pending native-speaker review.
- **Font binary files** (`refbox/resources/NotoSansCJK-Subset.otf`,
  `refbox/resources/NotoSansThai-Subset.ttf`) — bundled subsets, accepted as-is.
- **The regen-font scripts** (`scripts/regen-cjk-font.py`,
  `scripts/regen-thai-font.py`) and their `Justfile` recipes.
- **The Raspberry Pi README section removed in `848138c`** — pure docs cleanup,
  outside the language UI chrome.
- **`team-ref-list` orphan key sweep** (Unit 5 cross-unit reconcile) — filed for
  separate cleanup branch `chore/refbox/remove-unused-team-ref-list-keys`.
- **`portal-row-attempt-suffix` 14-locale population** (Unit 7 cross-unit
  reconcile) — filed for separate branch or absorption at Final Integration.

**Scope-creep guard.** Per `.claude/rules/scope.md`, any defect or improvement
surfaced in code outside this audit window goes to the Findings backlog with a
branch suggestion — never into the audit branch.

---

## 2. Acceptance criteria

Unit 8 is **complete-pending-integration** when *all* of:

1. **Catalog fully decided.** Every entry in Unit 8's Behaviour catalog is
   marked `@user_verified` (keep) or `@deleted` (delete). No `@proposed`
   entries remain. Scenario tags match their catalog entry's decision.
2. **Surgical pruning complete.** Every `@deleted` behaviour has been removed
   from the audit branch with a corresponding pruning commit. Branch history
   reads coherently.
3. **`just check` passes.** Format, lint, tests, audit — all green on Linux.
   Windows + macOS lint parity assumed unchanged.
4. **Walkthrough verified on the real refbox app.** All four Feature blocks
   exercised (see §6). The action-bar script-font behaviour is verified by
   actually switching the running app into a CJK locale and then opening the
   language page — confirming Latin action-button text doesn't render as tofu.
   The restart flow is exercised at least once (Latin ↔ CJK) and confirmed to
   restart cleanly and apply the new default font.
5. **Window-position decision recorded.** Walkthrough determines whether the
   simulator-window pin and the main-window centering are kept (catalog entries
   marked `@user_verified` with a brief justification) or sent to Findings-Backlog
   for a separate revert branch.
6. **Retroactive ADR 023 authored.** Captures the operator-confirmed shape of
   the language selection page, the UNVERIFIED marker rule (English / Spanish /
   French exempt; rest tagged), the restart-required flow, and the damage-tracking
   workaround rationale. References `848138c` and `ea151ac` as origin commits.
7. **Running state updated.** Unit 8 status in `AUDIT-PLAN.md`'s catalog table
   flipped to `complete-pending-integration (YYYY-MM-DD)`. Unit 8's Completed
   audits summary entry added. Findings backlog updated with any out-of-scope
   discoveries.
8. **Branch held local.** No push to origin; no PR opened.
9. **Memory updated.** Any new feedback or process refinements recorded.

---

## 3. Oracle (source-of-truth for "what should this code do")

The oracle is asymmetric across the two in-scope commits:

1. **For `ea151ac` (Turkish + UNVERIFIED marker):**
   `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md`
   (227 lines, authored on the original branch) is the **primary oracle**.
   `docs/superpowers/plans/2026-04-17-turkish-language-and-unverified-label.md`
   (570 lines, the implementation plan) is the **secondary oracle** for resolving
   ambiguity in file paths and struct names. Where the plan and the design spec
   conflict, the design spec wins.
2. **For `848138c` (11 languages + CJK/Thai fonts + grid-selection page):**
   **no design oracle exists.** This commit was authored as a feature commit
   without an accompanying ADR or design doc. The oracle is therefore:
   - The commit body itself (statement of intent).
   - Operator-confirmation during the walkthrough session.
   - This design spec, once approved, becomes the retroactive oracle for the
     grid-page UI chrome.

**Where the two commits overlap** — e.g. `ea151ac` evolved the `selected_font`
selection added in `848138c` from `_ => None` to `_ => Some(latin_font)` — the
later (`ea151ac`) shape is the oracle, with the earlier shape documented in the
catalog as a transitional state.

---

## 4. Behaviour catalog plan

Per the playbook's two-document model: the **catalog with decision tags**
(`@proposed`, `@user_verified`, `@deleted`, `@findings-backlog`) lives in
`AUDIT-PLAN.md` Unit 8 section. The **Gherkin scenarios with test tags**
(`@tested_pass`, `@tested_fail`, `@tested_inconclusive`) live in
`refbox/tests/features/language-ui-chrome.feature` on the audit branch.

### Gherkin Features (four blocks in `language-ui-chrome.feature`)

**Feature 1 — Language selection page.** Operator concern: "I can pick a
language from a grid." Scenarios cover:

- Navigate from App page to Language page via the "language" button.
- The grid renders 14 language buttons in alphabetical-by-romanized-name order
  across rows 1–3 and the first two slots of row 4; two empty `horizontal_space()`
  slots fill the rest of row 4.
- Tapping a language button previews the selection (blue highlight) but does
  not switch the running UI.
- Tapping Cancel returns to the App page without changing language.
- Tapping Done with a same-family selection commits the choice and the UI
  re-renders in the new language (hot-swap).
- Cancel and Done text on the action bar render in the *target* language's
  font (the tofu fix).
- First-launch behaviour: with no saved language in config, the page opens
  with English pre-selected and styled as the blue selection.

**Feature 2 — Restart-required indicator and flow.** Operator concern:
"Some languages require a restart and I see that before I commit." Scenarios
cover:

- Selecting a CJK-family language from a Latin-family app changes the right
  action button from green "DONE" to blue "RESTART TO APPLY".
- Selecting a Thai-family language from a Latin-family app does the same.
- Switching within Latin (e.g. ENGLISH → DEUTSCH) shows green DONE.
- Switching within CJK (e.g. 한국어 → 日本語) shows green DONE.
- Tapping RESTART TO APPLY saves the language to config, kills the simulator
  child process, spawns a fresh `current_exe()`, and the current process exits.
  The fresh instance reads the saved language and starts with the matching
  default font family.

**Feature 3 — UNVERIFIED marker on language buttons.** Operator concern:
"Some languages are marked unverified so I can choose informed." Scenarios
cover:

- TÜRKÇE button shows `(DOĞRULANMAMIŞ)` in small text below the name.
- 中文 button shows `(未验证)` in small text below.
- ENGLISH button shows no UNVERIFIED note.
- ESPAÑOL button shows no UNVERIFIED note.
- FRANÇAIS button shows no UNVERIFIED note.
- BAHASA INDONESIA renders as one small-text line ("BAHASA INDONESIA") with
  the note `(BELUM DIVERIFIKASI)` below — i.e. the new two-line shape, not the
  pre-`ea151ac` three-line stack of `BAHASA / INDONESIA / (note)`.

**Feature 4 — Button-text damage-tracking workaround (cross-cutting).**
Regression-coverage rather than new-behavior. Scenarios cover:

- Game-time button period text renders without prior-glyph bleed when the app
  language changes script (Latin ↔ CJK).
- `make_multi_label_button` callers elsewhere in the app (e.g. wherever this
  helper is still used) still render centered.
- `make_button` / `make_smaller_button` / `make_small_button` callers on
  existing screens still match their prior visual within reasonable tolerance.
  (Walkthrough exercises Main config page, Game Options, App Options, Display,
  Sound at a minimum to spot regressions.)

### Code-only changes subsection (no Gherkin scenarios; cataloged in `AUDIT-PLAN.md`)

- **B8.C1 — Window-position changes in `main.rs`.** Two lines added to window
  settings: simulator window pinned to `(0.0, 40.0)`; main window `Centered`.
  Catalog as in-scope. Walkthrough records keep vs. send-to-Findings-Backlog.
- **B8.C2 — Dead-code `impl Cyclable for Language`.** The `CyclingParameter::Language`
  caller was removed in `848138c` but the impl was extended in both commits
  anyway. No callers remain. Surgical revert anticipated: delete the impl block
  and the `use super::Cyclable` import in `languages.rs`. Verification:
  `cargo build` still passes (no callers); no operator-facing change.

### Cross-unit reconcile subsection

- **`team-ref-list` orphan key** in 15 locales — filed for separate cleanup
  branch (Unit 5 reconcile).
- **`portal-row-attempt-suffix` 14-locale population** — filed for separate
  branch or Final Integration absorption (Unit 7 reconcile).
- **Multi-label button per-line container wrap** — confirmed in scope here
  (was the Unit 6 punt to "Unit 8 territory").

---

## 5. Anticipated audit commits on `audit/refbox/language-ui-chrome`

The branch is cut directly from `origin/master` at `089c98d`. Both in-scope
commits `848138c` and `ea151ac` are already ancestors of `origin/master`, so
the audit branch inherits them as part of its history without any cherry-pick.
The Unit 8 diff for audit purposes is therefore the union of those two commits
as observed in master; the audit-only commits land on top:

1. Audit commit: add `refbox/tests/features/language-ui-chrome.feature` with
   the four Feature blocks (tags filled per walkthrough outcome).
2. Audit commit: dead-code `impl Cyclable for Language` revert (B8.C2).
3. Audit commit (conditional on walkthrough): window-position revert if
   walkthrough sends B8.C1 to Findings-Backlog. If kept, no commit; catalog
   note only.
4. Audit commit: walkthrough session notes (markdown comment block in the
   `.feature` file or a separate notes file, per Unit 7 precedent).
5. Audit commit: any surgical fixes surfaced during walkthrough (e.g. if a
   damage-tracking regression is found, the fix lands here).

Expected total: **3–5 audit commits** on top of `origin/master`. Smaller than
Unit 7 (5 audit commits beyond its base) is plausible given the reduced scope.

---

## 6. Walkthrough plan

**Environment.** Native refbox launch via
`WAYLAND_DISPLAY= cargo run -p refbox` in the worktree
`.worktrees/audit-unit-8-language-ui-chrome/` (per
[[refbox-wsl-launch-needs-wayland-display-unset]]).

**Steps.**

1. From App page, tap the "language" button → confirm Language page opens
   with current language pre-selected.
2. Tap each language button in turn → confirm preview blue highlight without
   running UI changing.
3. Tap Cancel → confirm App page returns and language unchanged.
4. Re-enter Language page, tap a same-family language (e.g. ENGLISH →
   DEUTSCH), tap DONE → confirm hot-swap (UI renders in German immediately,
   no restart).
5. Re-enter Language page, tap a different-family language (e.g. → 한국어
   from a Latin start), confirm green DONE flips to blue RESTART TO APPLY in
   target script, tap RESTART → confirm process exits and a fresh instance
   starts in the new font family.
6. Repeat from CJK start: open Language page (action-bar text now in CJK
   font), tap a Latin language → confirm Cancel/RESTART text renders cleanly
   (no tofu), tap RESTART → confirm process restart back to Latin font family.
7. Verify UNVERIFIED markers: each non-English/Spanish/French button shows
   its language's UNVERIFIED note in its own script.
8. Walk through Main, Game Options, App Options, Display, Sound config pages
   in the new language to spot any layout regression from the
   damage-tracking sweep.
9. Open the time bar across multiple states (warmup, half-time, between-game
   break) and switch language during each → confirm period text doesn't
   bleed glyphs.

**Outcome recording.** Per Unit 7 precedent: append a "Walkthrough session
notes" block at the bottom of `language-ui-chrome.feature` as Gherkin comments
(or in a separate sibling `.md` file), recording date, environment, observed
behaviours, divergences from the spec.

---

## 7. Process choice

**Heavy process** per `.claude/rules/plan-execution.md` and the playbook's
audit norm:

- Per-task verification (`superpowers:verification-before-completion` before
  marking each task done).
- Per-task code review for any commit that adds code beyond the cherry-picks
  (`superpowers:requesting-code-review` for the dead-code revert and any
  walkthrough-surfaced fixes).
- Strict deviation tracking in a Deviations section at the bottom of the
  per-unit plan.

Rationale: this audit's blast radius spans translation flow + iced font
defaults + a state-machine on `EditableSettings.pending_language/original_language`
+ a process-restart path that could break the running app. The heavy norm
that worked for Units 4–7 applies.

---

## 8. Risks and known unknowns

1. **Process-restart path is hard to test mechanically.** Spawning a fresh
   `current_exe()` is platform-sensitive and could behave differently in WSL
   vs. native Linux. Walkthrough is the only meaningful verification.
2. **Damage-tracking sweep may have subtle regressions on screens outside the
   language page.** The sweep affects every button helper. Walkthrough must
   exercise non-language config pages to spot regressions.
3. **iced 0.13 damage-tracking bug is the stated justification for the
   layout sweep**, but the audit is not equipped to verify the bug exists
   without the workaround. Treat as "documented workaround that operates
   correctly" — if a regression is found, file as Findings-Backlog with a
   suggested alternate fix.
4. **Action-bar font fallback for Latin under CJK locale** is the tofu fix.
   Verification requires actually being in a CJK locale when opening the
   language page. The walkthrough script (step 6) covers this.
5. **First-launch behaviour with corrupt or absent `config.language`** is
   currently `None` → defaults to English. Worth confirming during walkthrough
   that deleting the persisted config and re-launching produces English
   default with English-styled grid selection.

---

## 9. References

- `AUDIT-PLAN.md` Unit 8 section (catalog, decision log, cross-unit reconcile).
- `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md`
  — primary oracle for `ea151ac`.
- `docs/superpowers/plans/2026-04-17-turkish-language-and-unverified-label.md`
  — secondary oracle for `ea151ac`.
- Commits `848138c` and `ea151ac` on `origin/master`.
- Prior audit-unit design specs:
  `docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md`,
  `docs/superpowers/specs/2026-05-15-audit-unit-6-small-fixes-design.md`,
  `docs/superpowers/specs/2026-05-13-audit-unit-4-manual-alarm-design.md`.
- Unit 5 audit branch `audit/refbox/referee-names` — `team-ref-list` orphan
  key reconcile note.
- Unit 6 audit branch `audit/refbox/small-fixes-cluster` — per-line container
  wrap reconcile note.
- Unit 7 audit branch `audit/refbox/portal-health` — `portal-row-attempt-suffix`
  reconcile note.
