# Audit Unit 8 — Language UI chrome: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit the operator-facing UI chrome introduced by commits `848138c` (11 languages + CJK/Thai fonts + grid-selection page) and `ea151ac` (Turkish + UNVERIFIED marker on language buttons). Catalog every distinct behaviour, walk each one with the operator, surgically prune anything that's not wanted, write a retroactive ADR 023 capturing what survives, and hold the branch local until Final Integration. Translation file content and font binaries are out of scope per the playbook's 2026-05-12 scope reduction.

**Architecture:** Diff-led catalog of two commits, organized around the four Gherkin Feature blocks defined in the approved spec at [docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md](docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md) Section 4 (Language selection page · Restart-required indicator and flow · UNVERIFIED marker · Button-text damage-tracking workaround). Plus a Code-only changes subsection (window-position B8.C1 · dead-code Cyclable B8.C2) and a Cross-unit reconcile subsection. Expected catalog size ~20–30 entries. Single-pass catalog construction (no parallel subagent dispatch — scope is small enough to handle in-line per the playbook's "subagent-driven optional below 30 entries" guidance from Unit 6's process refinement). Per-Feature batched review with ambiguity carve-outs per Unit 3 refinement #3. Heavy process per `.claude/rules/plan-execution.md` and the playbook's audit norm. New retroactive ADR 023 authored on the audit branch (since no prior ADR exists for these commits).

**Tech Stack:** Rust 2024 / MSRV 1.85; iced 0.13 (refbox UI, including the new font-registration + default-font selection at startup); confy (config persistence for the saved language); Fluent (translation key system for hot-swap path); embedded font subsets Noto Sans CJK and Noto Sans Thai (binary; out of scope for content); `cargo test`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `just check`; manual refbox launch (`WAYLAND_DISPLAY= cargo run -p refbox` with `dangerouslyDisableSandbox:true`); Gherkin scenarios at one new `.feature` file `refbox/tests/features/language-ui-chrome.feature` (single file with four `Feature:` blocks — the playbook permits multi-Feature single files when the features cohere around one unit).

**Testing approach:**

- **No new Rust unit tests expected.** The grid-select page, UNVERIFIED marker, action-bar font selection, restart flow, and damage-tracking workaround are all walkthrough-verified — they're operator-facing UI behaviour, not testable in isolation.
- **Operator-driven walkthrough on the native refbox** for the four Gherkin Feature blocks. Single session covering all four Features, ~30–45 minutes wall clock. The walkthrough script lives in spec §6 (9 numbered steps).
- **Existing tests retained as-is.** The audit window adds zero new tests; the existing test suite continues to pass.
- **`just check` on the audit-branch tip** before close. Cross-platform lint parity inherited from existing master CI history (both in-scope commits are already in master and have passed cross-platform CI).
- **Out-of-scope sanity check.** Confirm that `.ftl` content edits and font binaries land in the diff but are **not** cataloged as audit entries — they're noted as "out of scope, accepted as-is" in the catalog's header.

---

## Acceptance criteria (Unit 8 "complete-pending-integration")

Unit 8 is complete-pending-integration when **all** of these hold on the audit branch `audit/refbox/language-ui-chrome`:

1. A behaviour catalog exists in `AUDIT-PLAN.md` under Unit 8, organized into four Feature blocks plus a Code-only changes subsection plus a Cross-unit reconcile subsection. Every catalog entry tagged `@user_verified`, `@deleted`, `@findings-backlog`, or `@redesign-followup`. No `@open` or `@proposed` entries remain.
2. Every `@user_verified` operator-observable behaviour is captured as a Gherkin scenario in `refbox/tests/features/language-ui-chrome.feature` (single file, four `Feature:` blocks). Backend-only or code-only behaviour stays plain-English in the catalog and the retroactive ADR.
3. Each scenario carries `@user_verified` plus a test-state tag (`@tested_pass`, `@tested_fail`, or `@tested_inconclusive`) and a manual-walkthrough timestamp in a session-notes comment.
4. The operator has driven the refbox UI through one Session covering the nine-step walkthrough script in spec §6, exercising all four Feature blocks including a Latin↔CJK restart round-trip (English → 한국어 → English) and a non-language-page sweep for damage-tracking regressions.
5. The window-position decision is recorded: catalog entry B8.C1 marked `@user_verified` (keep) with a brief justification, OR sent to `@findings-backlog` with a revert commit landing on the audit branch.
6. The dead-code `impl Cyclable for Language` is reverted on the audit branch as catalog entry B8.C2's surgical pruning.
7. `just check` passes on the audit branch tip (`fmt-check`, `clippy -D warnings`, all tests pass, `cargo audit` reports only the two pre-existing CVEs from Unit 3's Findings backlog #4 — not regressions).
8. A new retroactive ADR 023 exists at `docs/decisions/023-language-ui-chrome.md`, committed on the audit branch. Status `Accepted (retroactive)`. Decision section embeds every `@user_verified @tested_pass` scenario verbatim plus plain-English bullets for the code-only behaviours. References `848138c` and `ea151ac` as original commits.
9. The branch holds locally — no push, no PR opened. Per `prs-deferred-until-audit-done` memory.
10. `AUDIT-PLAN.md` status flipped from "not started" to "complete-pending-integration (YYYY-MM-DD)" in both the catalog table line 65 and the unit section heading; summary pointer added to "Completed audits" section per Unit 1 refinement #3.
11. Findings discovered out-of-scope are recorded in `AUDIT-PLAN.md`'s Findings backlog with a suggested follow-up branch name. They are **not fixed** on this branch.
12. Process refinements surfaced during execution are logged in `AUDIT-PLAN.md`'s "Process refinements log → From Unit 8".
13. Claude's memory files updated: `project_v040_handover.md` records Unit 8 complete-pending-integration; audit progress count incremented from "7 of 9 units complete" to "8 of 9 units complete"; Unit 9 (stale branches cleanup) noted as next.

---

## Prerequisites

- The user has approved this per-unit plan before any Task 1 step runs.
- Working tree on the current branch (`docs/workspace/backlog-adrs`) is clean except for the gitignored `.claude/scheduled_tasks.lock`.
- Read the approved design spec: [docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md](docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md) (commit `34f4bef`).
- Read `AUDIT-PLAN.md` Unit 8 section + the playbook's Per-unit workflow + Templates + the Process refinements log entries from Units 1–7. Particularly relevant to Unit 8:
  - Unit 1 refinement #2 (`.feature` files live in `refbox/tests/features/`)
  - Unit 1 refinement #6 (Bash cwd doesn't persist between calls — always `cd` into worktree; matches memory `feedback_cd_worktree_before_cargo`)
  - Unit 1 refinement #7 (refbox launch needs `WAYLAND_DISPLAY=`)
  - Unit 3 refinement #3 (per-Feature batched + ambiguity carve-outs for catalogs ≥20 entries — Unit 8 fits at the low end)
  - Unit 6 refinement #3 (cross-unit code evolution — `shared_elements.rs` is touched here AND in Units 5, 6, 7; `make_multi_label_button` per-line wrap was explicitly Unit-6-noted as Unit 8 territory)
  - Unit 6 refinement #4 (don't kill-loop a working process — check the memory rule's `why`)
  - Unit 7 refinement (TBD at Unit 7 close) on dev-portal URL clarity is informational only — Unit 8 does not require portal access at all.
- Read the two oracle docs for `ea151ac`: `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md` and `docs/superpowers/plans/2026-04-17-turkish-language-and-unverified-label.md`. **There is no oracle doc for `848138c`** — the audit retroactively becomes its design record via ADR 023.
- Read `.claude/rules/scope.md`, `communication.md`, `workspace.md`, `rust.md`, `embedded.md` (informational — Unit 8 does not touch wireless-remote), `pr-review.md`, `plan-execution.md`.
- Pre-commit hook at `<main-repo>/.git/hooks/pre-commit` must allow `audit/` branch type (fixed by Unit 1's `2a8dcbc`, inherited locally via the audit branches). Verify in Task 1.
- Memory `feedback_prs_deferred_until_audit_done` is in force: do not propose, suggest, or execute any PR/merge during this unit.
- Memory `feedback_refbox_wsl_wayland_unset` is in force: native `cargo run -p refbox` under WSLg requires `WAYLAND_DISPLAY=` prefix.
- Memory `feedback_user_drives_refbox_ui` is in force: Claude launches refbox in background with `dangerouslyDisableSandbox:true`; operator drives the UI.
- Memory `feedback_cd_worktree_before_cargo` is in force: every `cargo` or `git` command runs from a working directory `cd`-ed into the worktree at the start of the same shell call; do not rely on shell state across Bash calls.
- Memory `feedback_check_rule_applicability` is in force: before killing a process or retrying a tool call, read its output and check whether the memory rule's `why` actually applies.
- Memory `feedback_one_question_at_a_time` is in force: ask one question per message during Task 5 reviews.
- Memory `feedback_options_with_recommendation` is in force: every multi-option question to the operator includes Claude's recommended option first with a one-line "why".

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:**
- Create: `.worktrees/audit-unit-8-language-ui-chrome/` (new worktree)
- Edit: `AUDIT-PLAN.md` (gitignored, no commit)

- [ ] **Step 1.1: Confirm working tree is clean.**

  Run: `git -C /home/estraily/projects/uwh-refbox-rs status --short`
  Expected: empty output, or only `?? .claude/scheduled_tasks.lock` (gitignored harness file).

- [ ] **Step 1.2: Ask the user for explicit approval to cut the audit branch in a worktree.**

  Surface to the user:
  - Branch: `audit/refbox/language-ui-chrome`
  - Worktree: `.worktrees/audit-unit-8-language-ui-chrome/`
  - Cut from: `origin/master` at `089c98d` (both in-scope commits `848138c` and `ea151ac` are already ancestors of `origin/master`, so the branch inherits them naturally — no cherry-pick).

  Wait for operator approval before proceeding.

- [ ] **Step 1.3: Verify the pre-commit hook accepts `audit/` branch type.**

  Run from the main repo root:
  ```bash
  git -C /home/estraily/projects/uwh-refbox-rs show HEAD:.git/hooks/pre-commit 2>/dev/null | grep -E '^audit\|^valid_types' | head -5
  ```
  Or inspect `/home/estraily/projects/uwh-refbox-rs/.git/hooks/pre-commit` directly via Read tool. Expected: the `audit` branch-type prefix is in the allowed-prefixes list (Unit 1 commit `2a8dcbc` added it).

  If `audit` is missing: stop, surface the gap to the operator, and do not proceed — the prior audit units (1–7) had the hook in place, so this should not happen on a clean repo.

- [ ] **Step 1.4: Cut the audit branch in a new worktree.**

  From the main repo root:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs \
    && git fetch origin master \
    && git worktree add -b audit/refbox/language-ui-chrome \
       .worktrees/audit-unit-8-language-ui-chrome origin/master
  ```

  Verify:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git rev-parse --abbrev-ref HEAD \
    && git rev-parse HEAD
  ```
  Expected: `audit/refbox/language-ui-chrome` on branch, `089c98d…` as HEAD (or whatever the current `origin/master` is at execution time).

  Confirm both in-scope commits are ancestors:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git merge-base --is-ancestor 848138c HEAD && echo "848138c ✓" \
    && git merge-base --is-ancestor ea151ac HEAD && echo "ea151ac ✓"
  ```
  Expected: both lines print the ✓ confirmation.

- [ ] **Step 1.5: Update Unit 8 status in AUDIT-PLAN.md.**

  In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
  - Update the unit-catalog table row for Unit 8 (around line 65): `not started` → `in progress (started YYYY-MM-DD)` (use today's date).
  - Update the `### Unit 8 — Grid-select page + UNVERIFIED marker` section's heading line to add `**Status:** in progress (started YYYY-MM-DD)`.

  AUDIT-PLAN.md is gitignored — no commit.

- [ ] **Step 1.6: Verify worktree builds before touching anything.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && cargo build -p refbox 2>&1 | tail -10
  ```
  Expected: clean build (master at `089c98d` already builds; this is a sanity check that the worktree is functional).

  If a build error appears: this is unexpected, stop and diagnose. The branch is cut from a known-good master; an immediate compile failure means the worktree creation went wrong.

---

## Task 2: History reconstruction (AUDIT-PLAN.md Step 2)

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 8 section (gitignored, no commit)
- Optional: `.audit/unit-8-*.txt` working artifacts (local-only, never committed)

- [ ] **Step 2.1: Capture the in-scope commit list.**

  From the worktree:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && mkdir -p .audit \
    && git log --oneline 848138c^..ea151ac > .audit/unit-8-commits-raw.txt \
    && wc -l .audit/unit-8-commits-raw.txt
  ```

  Expected: `2 .audit/unit-8-commits-raw.txt`. The two lines are `848138c` and `ea151ac`. The audit window is exactly these two commits — no Renovate or other interleaved commits between them (`ea151ac` is `848138c`'s direct child per the commit timestamps in the spec).

  Verify the parent-child relationship:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git log --oneline 848138c..ea151ac \
    && echo "---" \
    && git rev-parse ea151ac^
  ```
  The first `git log` should print exactly one line (`ea151ac`); the parent of `ea151ac` should equal `848138c`. If anything else appears between them on master's history, treat the intervening commits as out-of-scope and record them under "Files touched but out of scope" in Step 2.4.

- [ ] **Step 2.2: Capture commit metadata.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && for h in 848138c ea151ac; do \
         git log -1 --format='%H%n%ai%n%s%n%n%b%n---END---%n' $h; \
       done > .audit/unit-8-commit-messages.txt \
    && grep -c '^---END---$' .audit/unit-8-commit-messages.txt
  ```
  Expected: `2`.

- [ ] **Step 2.3: Capture file-touch list across the audit window.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git diff --name-only 848138c^..ea151ac | grep -v '^$' | sort > .audit/unit-8-files-touched.txt \
    && wc -l .audit/unit-8-files-touched.txt \
    && cat .audit/unit-8-files-touched.txt
  ```

  Expected files (high-confidence prediction; the audit branch must contain exactly these):
  - **In scope (UI chrome):**
    - `refbox/src/app/languages.rs`
    - `refbox/src/app/message.rs`
    - `refbox/src/app/mod.rs`
    - `refbox/src/app/view_builders/configuration.rs`
    - `refbox/src/app/view_builders/shared_elements.rs`
    - `refbox/src/config.rs`
    - `refbox/src/main.rs`
  - **Out of scope (translation content / font binaries / scripts / README cleanup):**
    - `refbox/resources/NotoSansCJK-Subset.otf` (binary)
    - `refbox/resources/NotoSansThai-Subset.ttf` (binary)
    - `refbox/translations/de-DE/refbox.ftl` (and 13 other locale files: en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN)
    - `scripts/regen-cjk-font.py`
    - `scripts/regen-thai-font.py`
    - `Justfile` (recipe additions for the regen scripts)
    - `README.md` (Raspberry Pi section removal from `848138c`)
    - `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md` (from `ea151ac` — landed on master with the commit; out of scope as a doc, just acknowledge presence)
    - `docs/superpowers/plans/2026-04-17-turkish-language-and-unverified-label.md` (same)

  Flag anything in the actual file-touch list that's NOT in this prediction for explicit attention in the catalog.

- [ ] **Step 2.4: Verify NO touches outside `refbox/` and the documented out-of-scope paths.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git diff --name-only 848138c^..ea151ac \
       | grep -vE '^(refbox/|scripts/|docs/superpowers/|Justfile$|README\.md$|Cargo\.lock$)'
  ```
  Expected: empty output. If anything appears (notably `uwh-common/`, `overlay/`, `wireless-remote/`, `schedule-processor/`, `matrix-drawing/`), record it as a high-priority slop candidate — both commits are described in their bodies as `refbox/`-only.

- [ ] **Step 2.5: Map each commit to its in-scope file touches.**

  For each of the two commits, build a list of in-scope files it touched (excluding the out-of-scope paths from Step 2.3):

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && echo "=== 848138c (in scope) ===" \
    && git diff --name-only 848138c^..848138c \
       | grep -E '^refbox/(src/|tests/)' > .audit/unit-8-files-848138c.txt \
    && cat .audit/unit-8-files-848138c.txt \
    && echo "=== ea151ac (in scope) ===" \
    && git diff --name-only ea151ac^..ea151ac \
       | grep -E '^refbox/(src/|tests/)' > .audit/unit-8-files-ea151ac.txt \
    && cat .audit/unit-8-files-ea151ac.txt
  ```

  Expected (from the spec § In scope):
  - 848138c in-scope: `refbox/src/app/languages.rs`, `refbox/src/app/message.rs`, `refbox/src/app/mod.rs`, `refbox/src/app/view_builders/configuration.rs`, `refbox/src/app/view_builders/shared_elements.rs`, `refbox/src/config.rs`, `refbox/src/main.rs`.
  - ea151ac in-scope: `refbox/src/app/languages.rs`, `refbox/src/app/view_builders/configuration.rs`, `refbox/src/app/view_builders/shared_elements.rs`.

- [ ] **Step 2.6: Append a history-trace subsection to AUDIT-PLAN.md Unit 8.**

  Under `### Unit 8 — Grid-select page + UNVERIFIED marker`, add `#### History trace` with:
  - One-line summary of `848138c`: "Adds Language grid-select page + 11 new languages + CJK/Thai font registration + restart flow when crossing font-family boundaries."
  - One-line summary of `ea151ac`: "Adds Turkish (15th language) + per-language UNVERIFIED note + action-bar script-font fix (Cancel/Done/Restart in target language's font)."
  - The in-scope file list grouped by commit (from Step 2.5).
  - A "Cross-unit reconcile reminders" note pointing at the three items from spec §4 (team-ref-list orphan, portal-row-attempt-suffix 14-locale population, multi-label per-line wrap was Unit-6-flagged as Unit 8 territory).

  No commit — AUDIT-PLAN.md is gitignored.

---

## Task 3: Build behaviour catalog (AUDIT-PLAN.md Step 3)

This task is single-pass (no parallel subagent dispatch) because the catalog scope is ~20–30 entries — small enough to handle in the principal session without dispatch overhead. The four Feature blocks structure the catalog; entries are numbered B8.1, B8.2, … per Feature, with Code-only entries numbered B8.C1, B8.C2.

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 8 section (gitignored, no commit)

- [ ] **Step 3.1: Re-read the diffs for both commits.**

  Use Read or Bash (`git show`) to load the full diffs for `848138c` and `ea151ac` in the worktree. Focus on the seven in-scope files from Task 2.5. Take notes (mental model only — no working file needed at this size) on:
  - Every operator-observable behaviour the diff introduces
  - Every non-operator-observable internal state change
  - Every new translation key, message variant, or config field
  - Every defensive default or fallback branch
  - Every cross-unit conflict potential (multi-label per-line wrap; period-text damage-tracking on `make_game_time_button`)

- [ ] **Step 3.2: Write the catalog Feature 1 — Language selection page.**

  Open `AUDIT-PLAN.md` Unit 8 section. Add `#### Behaviour catalog → Feature 1: Language selection page` subsection. For each distinct operator-observable behaviour introduced by the grid-select page work, write a catalog entry using the playbook template (Templates → Behaviour catalog entry).

  Anticipated entries (the actual list is constructed from the diff, but these are high-confidence predictions for the spec's first Feature block):
  - B8.1 — `ConfigPage::Language` page added; reached via the "language" button on App Options page (which previously was a CyclingParameter trigger).
  - B8.2 — Page renders a 4×4 grid (rows 1–3 full + row 4 with two buttons + two `horizontal_space()` slots), with the time bar at top and Cancel + Done/Restart action bar at bottom.
  - B8.3 — Languages sorted alphabetically by romanized native name (per spec §4 Feature 1 and the inline comment in `make_language_select_page`).
  - B8.4 — Each language button label uses the language's native name in the language's own script (e.g. ENGLISH, ESPAÑOL, FRANÇAIS, DEUTSCH, 한국어, 日本語, 中文, ภาษาไทย, TÜRKÇE).
  - B8.5 — Tapping a language button paints it blue (preview) but does not switch the running app's locale until Done is tapped.
  - B8.6 — Tapping Cancel returns to App Options page without changing language (both `pending_language` and `original_language` are cleared from EditableSettings).
  - B8.7 — Tapping Done with a same-font-family selection commits the choice via `request_language(&LANGUAGE_LOADER, &[lang.as_lang_id()])` (hot-swap) and persists to config.
  - B8.8 — `EditableSettings.pending_language` and `EditableSettings.original_language` are `Option<Language>` initialized to `None`; populated from `LANGUAGE_LOADER.current_languages()[0]` when ChangeConfigPage(Language) fires.
  - B8.9 — First-launch behaviour: `config.language` is `None` until the operator confirms a Done; default font family is Roboto; locale is the system locale or English fallback (via the existing `unic-langid` flow).
  - B8.10 — Cancel/Done/Restart action-bar text renders in the *target* language's script font (the ea151ac tofu fix): selected_font picks CJK for Korean/Japanese/Mandarin, Thai for Thai, Roboto-Medium (Latin) for every other case including Turkish.

  Build the actual list from the diff. Drop predictions that don't match what the code actually does. Add anything the diff has that the predictions miss.

  For each entry, fill in: What it does (plain English) · Where in the diff (file:line refs) · Why intentional · Why slop · Linked scenario (S8.1.N for Feature 1) · Recommendation · Decision (`@proposed` default).

- [ ] **Step 3.3: Write the catalog Feature 2 — Restart-required indicator and flow.**

  Add `#### Behaviour catalog → Feature 2: Restart-required indicator and flow` subsection. Anticipated entries:

  - B8.11 — `font_family_id()` function classifies each Language as Latin (0), CJK (1), or Thai (2). Used both in `mod.rs` (LanguageSelectComplete handler) and `view_builders/configuration.rs` (make_language_select_page).
  - B8.12 — `needs_restart = font_family_id(original) != font_family_id(selected)` drives the action-bar right button: green DONE if same family, blue RESTART TO APPLY if different.
  - B8.13 — RESTART TO APPLY button text uses `selected.restart_text()` (the hardcoded per-language translation in `languages.rs`).
  - B8.14 — Tapping RESTART TO APPLY: saves `config.language = Some(lang)`, calls `confy::store(crate::APP_NAME, None, &self.config).unwrap()`, kills `sim_child` if present, spawns a fresh `std::env::current_exe()`, then `std::process::exit(0)`.
  - B8.15 — The fresh exe instance reads the saved `config.language` and chooses the matching default font family at startup (`Roboto` for Latin, `Noto Sans CJK KR` for CJK, `Noto Sans Thai` for Thai).
  - B8.16 — `confy::store` is called with `.unwrap()` (panic on failure). Slop candidate: same pattern as Unit 3's finding #2 — silent unwrap on config persistence.
  - B8.17 — Same-family Done bypasses restart entirely: just `request_language(..)` for hot-swap and persists config.

- [ ] **Step 3.4: Write the catalog Feature 3 — UNVERIFIED marker on language buttons.**

  Add `#### Behaviour catalog → Feature 3: UNVERIFIED marker on language buttons` subsection. Anticipated entries:

  - B8.18 — Every language button except English, Spanish, and French shows a small `(UNVERIFIED)`-equivalent note in the language's own script, below the language name.
  - B8.19 — The note text is hardcoded per language at the call site in `make_language_select_page` (NOT routed through `fl!`, because `fl!` always renders in the operator's current locale — each button must label itself).
  - B8.20 — `make_lang_button_with_note` helper in `shared_elements.rs` renders a two-line button: main name on top (`NameLines::OneLine` or `NameLines::OneLineSmall`) + the note in small text below.
  - B8.21 — `NameLines` enum has two variants: `OneLine(T)` for short names rendered at default text size (used for TÜRKÇE, ITALIANO, FILIPINO, etc.) and `OneLineSmall(T)` for long names rendered at SMALL_TEXT (used only for the two Bahasa buttons: "BAHASA INDONESIA" and "BAHASA MELAYU").
  - B8.22 — The Bahasa buttons changed shape from `848138c`'s 3-line stack ("BAHASA / INDONESIA / [no note]") to `ea151ac`'s 2-line shape ("BAHASA INDONESIA (small) / (note)"). Walkthrough must confirm the new shape is operator-acceptable.
  - B8.23 — English, Spanish, and French buttons stay as the simpler single-line `lang_btn` helper (no NameLines wrapper, no note).

- [ ] **Step 3.5: Write the catalog Feature 4 — Button-text damage-tracking workaround.**

  Add `#### Behaviour catalog → Feature 4: Button-text damage-tracking workaround` subsection. Anticipated entries:

  - B8.24 — `make_button`, `make_smaller_button`, `make_small_button` all changed from `button(centered_text(label))` to `button(container(text(label).align_x(Left).align_y(Center).width(Shrink)).center(Length::Fill))`. Stated reason in code comments: iced 0.13 damage-tracking bug where width(Fill) text leaves stale glyph pixels when content changes script.
  - B8.25 — `make_multi_label_button` per-line container wrap: the two-line layout was changed from `text(labels.0).align_x(Center).width(Fill)` to `container(text(labels.0).align_x(Left).width(Shrink)).center_x(Length::Fill)`, same per-line for the second label. (Unit 6 explicitly flagged this as Unit 8 territory in its cross-unit note.)
  - B8.26 — `make_game_time_button` period-text wrap: the period text inside the game-time button was changed from `text(period_text).style(style).width(Fill).align_y(Center).align_x(Right)` to a `container(...)` wrapping a `text(...).width(Shrink)` for the same damage-tracking reason.
  - B8.27 — Generic-parameter bound was tightened on all four button helpers from `Message: Clone` to `Message: 'a + Clone`. Cause: the new inner `container(text(...))` widget requires a lifetime bound the prior `centered_text` flow didn't expose.

- [ ] **Step 3.6: Write the catalog Code-only changes subsection.**

  Add `#### Behaviour catalog → Code-only changes` subsection. Two entries:

  - B8.C1 — Window-position changes in `main.rs`: simulator window pinned to `(0.0, 40.0)` via `window::Position::Specific(iced::Point::new(0.0, 40.0))`; main window centered via `window::Position::Centered`. Both unrelated to language UI. Recommendation: catalog in scope, decide during walkthrough whether to keep or revert.
  - B8.C2 — Dead-code `impl Cyclable for Language` in `languages.rs` plus the orphan `use super::Cyclable` import. The caller `CyclingParameter::Language` was removed in `848138c` but the impl was extended in both commits anyway. Recommendation: surgical revert in Task 6.

- [ ] **Step 3.7: Write the catalog Cross-unit reconcile subsection.**

  Add `#### Behaviour catalog → Cross-unit reconcile (not in scope here)` subsection. Three entries, each pointing at a separate-branch resolution path:

  - B8.X1 — `team-ref-list` orphan key in 15 locales. From Unit 5's cross-unit note. Resolution: branch suggestion `chore/refbox/remove-unused-team-ref-list-keys`.
  - B8.X2 — `portal-row-attempt-suffix` translation key present only in `en-US` (added by Unit 7's commit `38482fd`). Resolution: branch suggestion `chore/refbox/portal-row-attempt-suffix-14-locales`, OR absorb at Final Integration.
  - B8.X3 — Multi-label per-line wrap (B8.25) is confirmed in scope here, closing Unit 6's cross-unit punt. No follow-up needed.

  Each X-entry has no Decision tag — they are notes about deferrals, not catalog decisions.

- [ ] **Step 3.8: Draft Gherkin scenarios for every `@user_verified`-candidate Feature entry.**

  In the same `#### Behaviour catalog` section, for each Feature 1–4 entry that's operator-observable (which is most of them — Feature 4 entries are observable as "no glyph bleed during language switch"), draft a `@proposed` Gherkin scenario in the unit's `#### Scenarios` subsection of AUDIT-PLAN.md.

  Use the format from the playbook (Scenario format section):
  ```gherkin
  @proposed
  Scenario: <one-line behaviour>
    Given <starting operator-visible state>
    When <operator action>
    Then <observable outcome>
  ```

  Group scenarios under one of four `Feature:` blocks matching the spec §4 names. Backend-only entries (B8.8 EditableSettings field plumbing, B8.11 font_family_id classifier, B8.27 generic bound tightening) do NOT get scenarios; they're prose-only in the catalog.

- [ ] **Step 3.9: Apply the slop-catching checklist.**

  Walk the playbook's slop-catching list (AUDIT-PLAN.md "Slop-catching checklist" section, line ~456). For each item that matches in the Unit 8 diff, ensure it has its own catalog entry. Particular watch-fors:

  - **Defensive default for impossible case:** B8.16's `confy::store(..).unwrap()` is the closest match — the unwrap silences a real failure mode (disk full, permission error, config-path missing) rather than a defensive-against-impossible case. Note in catalog.
  - **Helper never called from real code:** B8.C2 (dead Cyclable impl). Catalog as `@deleted` candidate.
  - **Method or type defined later than its first caller:** check that `font_family_id` is defined before its first call in both `mod.rs` and `configuration.rs`.
  - **Persistence file format slop:** `Option<Language>` in `Config` (B8 — number TBD based on catalog) — the field is `Option` rather than defaulting to English. Slop hypothesis: an absent saved value is genuinely meaningful (first launch) so `Option` is correct. Validate during review.
  - **String-content slop:** the hardcoded `cancel_text`/`done_text`/`restart_text` and per-language UNVERIFIED notes are NOT slop in the structural sense — they're operator-facing UI chrome whose accuracy is deferred. The catalog flags them as "presence + structure in scope, accuracy deferred."

- [ ] **Step 3.10: Surface the catalog snapshot to the operator.**

  Report:
  - Total entry count (Feature 1 / 2 / 3 / 4 / Code-only / Cross-unit reconcile breakdown)
  - One-sentence summary per Feature block
  - List of slop-flagged entries (entries with non-trivial "Why it might be slop")
  - List of carve-out candidates (any entry that needs individual operator decision — at minimum B8.C1 window-position, B8.16 confy unwrap pattern, B8.22 Bahasa shape change)
  - Total scenario count

  Wait for operator acknowledgement before Task 4.

---

## Task 4: Slop-catching pass (AUDIT-PLAN.md Step 4 prep — by the principal)

Before the operator walks the catalog in Task 5, the principal does one more pass focused on cross-unit risk and consistency.

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 8 section (gitignored, no commit)

- [ ] **Step 4.1: Search for cross-branch dependencies.**

  Per Unit 5 refinement #1, before approving any Feature 4 entry that touches a generic helper, run:

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git log --all -S 'make_button' --oneline | head -20 \
    && echo "---" \
    && git log --all -S 'make_multi_label_button' --oneline | head -20 \
    && echo "---" \
    && git log --all -S 'make_game_time_button' --oneline | head -20
  ```

  Identify any commit on another local branch (notably the other audit branches `audit/refbox/confirm-score-timing`, `audit/refbox/manual-alarm-button`, `audit/refbox/referee-names`, `audit/refbox/small-fixes-cluster`, `audit/refbox/portal-health`) that touches one of these helpers. If a cross-branch touch exists:
  - Note it in the catalog entry's `Cross-unit note:` line
  - If the touch is a *content* change to the helper that this audit branch doesn't have, that's a Final-Integration merge concern, not an audit blocker
  - If the touch is a *callers-of-the-helper* change, Feature 4's damage-tracking sweep affects them — note in walkthrough plan

- [ ] **Step 4.2: Check for behaviour shifts hidden inside the diff.**

  Per Unit 4 refinement #2 (commit-fan-out check) and Unit 5 refinement #4 (bundled-fix decomposition): inspect the largest single-commit hunks in `848138c` for behaviour changes that the commit body doesn't explicitly call out.

  Particular watch-fors:
  - **Window-position lines** in `main.rs` (already cataloged as B8.C1, but confirm there are no *other* unmentioned non-language changes ride-along).
  - **`config.rs` migration:** check that the `Config::sanitize_old(..)` migration path correctly handles old configs that don't have a `language` field. Per the diff, `language` is destructured with no explicit default — `Default::default()` for `Option<Language>` is `None`, which matches the spec's first-launch behaviour. Confirm.
  - **Removed CyclingParameter::Language arm** — confirm no caller leaks (the match in `mod.rs` updates to no longer handle Language).

- [ ] **Step 4.3: Verify all Feature 1–3 anticipated cross-references resolve.**

  Each catalog entry has a `Linked scenario:` field. Walk the entries and confirm every scenario reference resolves to an actual scenario draft in §3.8 — and vice versa. No orphan scenarios; no entry-without-scenario for operator-observable items.

- [ ] **Step 4.4: Stamp `Recommendation:` lines.**

  For each catalog entry, write a one-line recommendation: `keep` (default for behaviour the spec endorses), `delete` (default for B8.C2 dead Cyclable and any newly-surfaced slop), `walkthrough-decides` (default for B8.C1 window-position and B8.22 Bahasa shape change), `findings-backlog` (default for B8.16 confy unwrap pattern — parallels Unit 3's finding #2).

- [ ] **Step 4.5: Surface the slop-catching summary to the operator.**

  Report the recommendation distribution: `<N> keep`, `<N> delete`, `<N> walkthrough-decides`, `<N> findings-backlog`. Particular call-outs:
  - B8.C1 → walkthrough decides
  - B8.C2 → delete (dead code, no callers)
  - B8.16 → findings-backlog (consistent with Unit 3 #2 confy pattern)
  - B8.22 → walkthrough decides (Bahasa shape change)

  Wait for operator acknowledgement before Task 5.

---

## Task 5: Per-entry operator decisions (AUDIT-PLAN.md Step 4 review — per-Feature batched)

This task uses **per-Feature batched review with ambiguity carve-outs** per Unit 3 refinement #3. Total expected questions: 6–10 (one per Feature block + one each for the carve-outs).

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 8 section (gitignored, no commit)

- [ ] **Step 5.1: Present the catalog summary to the operator.**

  Read aloud (in plain English):
  - Total entry count by Feature block + Code-only + Cross-unit reconcile
  - One-sentence summary per Feature block of what it covers
  - The list of walkthrough-decides entries (carve-outs)
  - The list of findings-backlog candidates

  Wait for operator acknowledgement.

- [ ] **Step 5.2: Per-Feature batched approval — Feature 1 (Language selection page).**

  Present:
  - Plain-English summary of Feature 1's scope (the grid layout, the preview-then-confirm interaction, the alphabetical ordering, the action-bar script-font fix).
  - One-line per entry: `B8.N — <short name> — <recommendation>`.
  - For each Feature 1 entry, recommendation is `keep` (default).

  Ask: "Approve Feature 1 as recommended, or carve out specific entries for individual review?"

  Operator responds with either "approve all as recommended" OR "approve all except B8.X" (which becomes a carve-out question in Step 5.6).

- [ ] **Step 5.3: Per-Feature batched approval — Feature 2 (Restart-required indicator and flow).**

  Same shape as Step 5.2, scoped to Feature 2. Particular question to surface: "B8.16 (`confy::store(..).unwrap()` on language save) — recommendation is to file as findings-backlog (consistent with Unit 3 finding #2). Approve filing, or want a different decision?"

- [ ] **Step 5.4: Per-Feature batched approval — Feature 3 (UNVERIFIED marker).**

  Same shape, scoped to Feature 3. Particular question to surface: "B8.22 (Bahasa buttons changed from 3-line stack to 2-line small-text+note) — recommendation is to walkthrough-decide. You'll see the shape at Task 7 and choose then. OK?"

- [ ] **Step 5.5: Per-Feature batched approval — Feature 4 (Button-text damage-tracking workaround).**

  Same shape, scoped to Feature 4. Particular question to surface: "Feature 4 affects every button helper in the app, not just the language page. The walkthrough plan (Task 7) includes a non-language-page sweep — Main, Game Options, App Options, Display, Sound — to spot any regression. If a regression is found, recommendation is to file as findings-backlog and keep the Feature 4 sweep on this audit branch (we don't want to revert the workaround mid-audit; the regression would itself be the new follow-up). OK with that plan?"

- [ ] **Step 5.6: Carve-out questions — one per ambiguous-by-design entry.**

  For each entry the operator carved out from Steps 5.2–5.5, plus B8.C1 (window-position walkthrough-decides), present a standalone question.

  Standard form per carve-out:
  - Behaviour summary in plain English
  - Recommendation with reason
  - Three options: keep / delete (surgical pruning in Task 6) / move-to-findings-backlog
  - **Special handling for B8.C1:** "Both window-position lines (simulator pinned + main centered) are walkthrough-decided. We'll mark them `@proposed` for now and revisit at Task 7 Step 7 once you've seen the current behaviour."

- [ ] **Step 5.7: Code-only batched approval — Code-only changes.**

  Present:
  - B8.C1 — walkthrough decides (deferred to Task 7).
  - B8.C2 — dead Cyclable impl, surgical revert in Task 6. Confirm.

  Ask: "Approve B8.C2 surgical revert?"

- [ ] **Step 5.8: Update each catalog entry's Decision: line with the operator decision.**

  Each Decision line becomes one of: `@user_verified`, `@deleted`, `@findings-backlog`, `@redesign-followup`, or `@proposed` (only for B8.C1 if walkthrough-deferred).

  For each operator-observable entry, also update its linked scenario's tag from `@proposed` to `@user_verified` (or `@deleted`) to match.

- [ ] **Step 5.9: File any Findings-backlog items inline.**

  If any entry became `@findings-backlog`, add a `#### From Unit 8 (YYYY-MM-DD)` subsection to `AUDIT-PLAN.md`'s `### Findings backlog` section listing each finding with a suggested branch name. Cross-reference the catalog entry ID. Expected entries:
  - B8.16 confy unwrap on language save → branch suggestion `chore/refbox/confy-error-paths` (or absorb into Unit 3's similar finding's eventual branch).
  - Anything else the operator carved into findings-backlog during Steps 5.6 / 5.7.

- [ ] **Step 5.10: Surface the post-decision catalog state to the operator.**

  Report:
  - Final count by Decision: `<X> @user_verified`, `<Y> @deleted`, `<Z> @findings-backlog`, `<W> @redesign-followup`, `<V> @proposed` (for walkthrough-deferred B8.C1).
  - If any `@deleted` entries exist, list them — Task 6 will perform surgical pruning.
  - If only B8.C2 is `@deleted`, that's the expected default — Task 6 has one prune to do.

---

## Task 6: Surgical pruning (AUDIT-PLAN.md Step 5)

**Files:**
- Modify: `refbox/src/app/languages.rs` (for B8.C2 dead Cyclable revert; very high confidence this is the only prune)
- Edit: `AUDIT-PLAN.md` Unit 8 section (gitignored, no commit per pruning step)

- [ ] **Step 6.1: For each `@deleted` entry, identify the prune set.**

  Expected list at this point: B8.C2 only (dead `impl Cyclable for Language` + the orphan `use super::Cyclable` import in `refbox/src/app/languages.rs`).

  If Task 5 produced additional `@deleted` entries, list them with file:line refs and walk them in dependency order in Steps 6.2.

- [ ] **Step 6.2: Prune B8.C2 — dead `impl Cyclable for Language`.**

  - **Step 6.2.a: Edit `refbox/src/app/languages.rs` in the worktree.** Remove the `impl Cyclable for Language { … }` block (the 17-line impl ending with `Self::Turkish => Self::English`). Remove the `use super::Cyclable;` line at the top of the file. Do not reflow other code.

  - **Step 6.2.b: No tests assert on this dead code** (it had no callers). Skip test-removal.

  - **Step 6.2.c: Confirm `Cyclable` is still needed elsewhere.** It is — the trait itself lives in `refbox/src/app/view_builders/configuration.rs` and is implemented for `BlackWhiteBleachers`, `Mode`, `Brightness`, `UnderWaterVol`, etc. The trait stays; only `Language`'s impl + the unused import in `languages.rs` go.

  - **Step 6.2.d: Run `just fmt-check` in the worktree.**
    ```bash
    cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just fmt-check 2>&1 | tail -5
    ```
    Expected: clean.

  - **Step 6.2.e: Run `just lint` in the worktree.**
    ```bash
    cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just lint 2>&1 | tail -20
    ```
    Expected: clippy `-D warnings` clean. The removed `use` shouldn't trigger an unused-import warning because we removed it; if a "trait imported but not used" warning surfaces elsewhere, treat as a follow-up to the surgical edit (not a separate bug).

  - **Step 6.2.f: Run `just test` in the worktree.**
    ```bash
    cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just test 2>&1 | tail -10
    ```
    Expected: all tests pass.

  - **Step 6.2.g: Commit the prune.**
    ```bash
    cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
      && git add refbox/src/app/languages.rs \
      && git commit -m "audit(refbox): remove dead impl Cyclable for Language

CyclingParameter::Language was removed in 848138c when the
grid-select page replaced cycle-through-languages, but the
Cyclable impl was extended in both 848138c and ea151ac
anyway with no remaining callers. Unit 8 catalog entry B8.C2.

No callers in the workspace; verified by:
  grep -rn 'Cyclable\\|\\.cycle()' refbox/src/ | grep -i 'lang'

Removed: impl Cyclable for Language block (lines ~144–162 in
refbox/src/app/languages.rs) and the orphan
\`use super::Cyclable;\` import."
    ```

- [ ] **Step 6.3: Surface the pruning summary to the operator.**

  Report:
  - 1 prune commit (B8.C2) — or N if additional carve-outs landed in Task 5.
  - Net lines removed: ~17 + 1 (impl block + import).
  - Confirm `just check` passes on the audit-branch tip:
    ```bash
    cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just check 2>&1 | tail -10
    ```

  If zero `@deleted` entries somehow survived (unlikely given B8.C2 is recommended-delete by default), skip Step 6.2 entirely and jump straight here with "No surgical pruning required."

---

## Task 7: Test pass + walkthrough verification (AUDIT-PLAN.md Step 6)

**Files:**
- Create: `refbox/tests/features/language-ui-chrome.feature` (on audit branch)
- Edit (later, after walkthrough): `refbox/tests/features/language-ui-chrome.feature` (session notes appended)
- Conditionally edit (if walkthrough sends B8.C1 to findings-backlog): `refbox/src/main.rs` (window-position revert)

- [ ] **Step 7.1: Run `just check` on the audit-branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just check 2>&1 | tail -40
  ```

  Expected: `fmt-check` clean, `clippy -D warnings` clean, all tests pass, `cargo audit` reports the two pre-existing CVEs from Unit 3's Findings backlog #4 (not regressions).

  If anything fails: stop, diagnose, fix on the audit branch, re-run. Per Unit 6 refinement #4, don't retry-loop a process whose output you haven't read.

- [ ] **Step 7.2: Write `refbox/tests/features/language-ui-chrome.feature`.**

  Create the file with four `Feature:` blocks. Each Feature contains the `@user_verified` scenarios from Task 5. The file is committed once with `@user_verified` tags and session-note placeholder comments; tags are upgraded to `@tested_*` during the walkthrough.

  Skeleton (fill in scenario list from Task 5 outcome — the predictions below are starting points):

  ```gherkin
  Feature: Language selection page
    The operator can pick a UI language from a grid-select page reached
    from App Options. Selection is preview-then-confirm: tapping a
    language paints it blue without changing the running UI; Done commits;
    Cancel reverts. Languages render in their own native script via
    bundled font subsets for CJK and Thai.

    @user_verified
    Scenario: Navigate to the Language page from App Options
      Given the operator is on the App Options page in any language
      When the operator taps the "language" button (top-right of the grid)
      Then the Language page appears with the time bar at top and four rows of language buttons
      And the currently active language's button is painted blue (selected)
      And the action bar shows Cancel on the left and Done (green) on the right
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Preview a language without committing
      Given the operator is on the Language page with English currently active
      When the operator taps "ESPAÑOL"
      Then the ESPAÑOL button becomes blue and ENGLISH becomes light-gray
      And the rest of the page (including the time bar) stays in English
      And no language change is persisted to config
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Cancel reverts to the prior language
      Given the operator is on the Language page with ESPAÑOL preview-selected
      When the operator taps Cancel
      Then the App Options page appears with the UI still in the prior language (English)
      And no change is persisted to config
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Done commits a same-family language hot-swap
      Given the operator is on the Language page with ESPAÑOL preview-selected and English currently active
      When the operator taps Done
      Then the App Options page appears with the UI rendered in Spanish
      And the config persists `language: Some(Spanish)`
      And no app restart is needed
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Action-bar text renders in the target language's script font
      Given the operator is on the Language page with the app running in English
      When the operator taps "한국어"
      Then the Cancel button text reads "취소" rendered in the CJK font
      And the right action button (RESTART TO APPLY) reads "재시작하여 적용" rendered in the CJK font
      And no glyph renders as a tofu box
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

  Feature: Restart-required indicator and flow
    Switching between Latin / CJK / Thai font families requires a restart
    (iced picks the default font once at startup). The action bar's right
    button flips from green DONE to blue RESTART TO APPLY when the
    preview-selected language crosses a font-family boundary. Tapping
    RESTART persists the language, kills the simulator child, and respawns
    a fresh copy of the exe.

    @user_verified
    Scenario: Selecting a CJK language from a Latin start shows RESTART TO APPLY
      Given the operator is on the Language page with ENGLISH currently active
      When the operator taps "한국어"
      Then the right action button reads "재시작하여 적용" with a blue background
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Switching within Latin shows DONE not RESTART
      Given the operator is on the Language page with ENGLISH currently active
      When the operator taps "DEUTSCH"
      Then the right action button reads "FERTIG" with a green background
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Switching within CJK shows DONE not RESTART
      Given the operator is on the Language page with 한국어 currently active
      When the operator taps "日本語"
      Then the right action button reads "完了" with a green background
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: RESTART TO APPLY persists language and respawns the exe
      Given the operator is on the Language page with ENGLISH currently active and 한국어 preview-selected
      When the operator taps "재시작하여 적용"
      Then the current refbox process exits
      And a fresh refbox window appears with all UI rendered in Korean
      And the time bar period names and game labels use the CJK font
      And the persisted config now reads `language: Some(Korean)`
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Restart from CJK back to Latin renders cleanly with no tofu
      Given the operator is on the Language page after the Korean restart, with 한국어 currently active
      When the operator taps "ENGLISH" then "RESTART TO APPLY"
      Then the current refbox process exits
      And a fresh refbox window appears with all UI rendered in English using Roboto-Medium
      And no language-page action-bar glyph renders as a tofu box during the brief preview pause before the restart
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

  Feature: UNVERIFIED marker on language buttons
    Every language button except English, Spanish, and French shows a
    small "(UNVERIFIED)"-equivalent note in that language's own script
    beneath the language name, signalling that a native speaker has not
    yet reviewed the translation.

    @user_verified
    Scenario: Turkish button shows the UNVERIFIED note in Turkish
      Given the operator is on the Language page
      Then the "TÜRKÇE" button shows "(DOĞRULANMAMIŞ)" in small text below the name
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Mandarin button shows the UNVERIFIED note in Mandarin
      Given the operator is on the Language page
      Then the "中文" button shows "(未验证)" in small text below the name
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: English button shows no UNVERIFIED note
      Given the operator is on the Language page
      Then the "ENGLISH" button shows the single word "ENGLISH" with no note below
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Spanish button shows no UNVERIFIED note
      Given the operator is on the Language page
      Then the "ESPAÑOL" button shows the single word "ESPAÑOL" with no note below
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: French button shows no UNVERIFIED note
      Given the operator is on the Language page
      Then the "FRANÇAIS" button shows the single word "FRANÇAIS" with no note below
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Bahasa Indonesia renders as one small-text line with note below
      Given the operator is on the Language page
      Then the BAHASA INDONESIA button renders as a single small-text line "BAHASA INDONESIA"
      And the note "(BELUM DIVERIFIKASI)" appears in small text below
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

  Feature: Button-text damage-tracking workaround
    Every button helper in the app (make_button, make_smaller_button,
    make_small_button, make_multi_label_button, and the period text
    inside make_game_time_button) was rewritten to wrap a width(Shrink)
    text widget inside a centering container. This is the iced-0.13
    damage-tracking workaround for old glyph pixels bleeding through
    when text content changes script.

    @user_verified
    Scenario: Time bar period text re-renders cleanly when language changes script
      Given the operator is on any page with the time bar visible and the app running in English
      When the operator changes language to 한국어 (with a restart) and returns to the same page
      Then the time bar period name renders in Korean using the CJK font
      And no Latin glyph pixels bleed through from the previous rendering
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Existing config-page buttons render centered after the workaround
      Given the operator navigates to Main, Game Options, App Options, Display, Sound config pages
      Then every button on these pages renders its text centered within the button bounds
      And no button-text alignment regression is observed relative to the prior visual
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Multi-label buttons render their two labels centered after the workaround
      Given the operator opens a page that uses make_multi_label_button (e.g. wherever it still appears in the app outside the Language page)
      Then both lines of every multi-label button render center-aligned
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM
  ```

  **Add more scenarios** as the Task 3.8 / Task 5 outcome dictates. Drop any scenario whose linked entry was `@deleted` in Task 5.

  Backend-only or code-only behaviours (B8.8 EditableSettings field plumbing, B8.27 generic bound tightening, B8.C2 dead Cyclable revert) do NOT get scenarios.

- [ ] **Step 7.3: Commit the `.feature` file.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git add refbox/tests/features/language-ui-chrome.feature \
    && git commit -m "audit(refbox): seed Gherkin scenarios for Unit 8 language UI chrome audit"
  ```

- [ ] **Step 7.4: Launch refbox from the audit worktree.**

  Claude runs (background, with `dangerouslyDisableSandbox: true` per memory `feedback_run_command`):

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox
  ```

  Wait for the refbox window to come up. Confirm the operator can see the window and the time-banner area. Per memory `feedback_user_drives_refbox_ui`, the operator drives the UI from here; Claude reports observations and asks per-scenario confirmations.

- [ ] **Step 7.5: Walkthrough Session — Step 1 (Navigate to Language page).**

  Ask the operator to:
  - Navigate from the main game screen to App Options (Cog → App Options).
  - Tap the "language" button (top-right of the App Options grid; previously labelled with the current language's "this language" string).

  Confirm: the Language page opens with the current language pre-selected (blue button).

  Mark Feature 1 Scenario "Navigate to the Language page from App Options" in `language-ui-chrome.feature` as `@tested_pass` / `@tested_fail` / `@tested_inconclusive` with timestamp.

- [ ] **Step 7.6: Walkthrough Session — Step 2 (Preview each language).**

  Ask the operator to tap several language buttons in turn (at minimum ENGLISH, ESPAÑOL, 한국어, 日本語, 中文, ภาษาไทย, TÜRKÇE). Confirm:
  - Tapped button paints blue; previously-selected button reverts to light-gray.
  - Time bar at top stays in the originally-active language (preview hasn't committed).
  - For non-Latin selections, the action bar's Cancel + RESTART TO APPLY text re-renders in the target script.

  Mark Feature 1 "Preview" scenario + Feature 1 "Action-bar script font" scenario + Feature 2 CJK/Latin/Thai scenarios + Feature 3 UNVERIFIED-marker scenarios per the operator's observations.

- [ ] **Step 7.7: Walkthrough Session — Step 3 (Cancel reverts).**

  Ask the operator to:
  - Re-enter the Language page if needed.
  - Tap a same-family language (e.g. DEUTSCH from an ENGLISH start), confirm green DONE appears.
  - Tap Cancel.

  Confirm: returns to App Options; UI still in English; no restart; config not changed.

  Mark Feature 1 "Cancel reverts" scenario.

- [ ] **Step 7.8: Walkthrough Session — Step 4 (Same-family hot-swap).**

  Ask the operator to:
  - Re-enter the Language page.
  - Tap ESPAÑOL (or another same-family Latin language).
  - Tap DONE (green button).

  Confirm: returns to App Options; UI renders in Spanish (or chosen language); time bar period text changes; no restart; config persists `language: Some(Spanish)`.

  Verify by reading `~/.config/refbox/<config-file>` (or wherever confy stores it) — Claude runs:
  ```bash
  find ~/.config -name '*refbox*' 2>/dev/null | head -3
  cat $(find ~/.config -name '*refbox*' 2>/dev/null | head -1) | grep -A1 language
  ```
  Expected: `language = "Spanish"` (or the toml equivalent of the chosen variant).

  Mark Feature 1 "Same-family Done" scenario.

- [ ] **Step 7.9: Walkthrough Session — Step 5 (Latin → CJK restart).**

  Ask the operator to:
  - Re-enter the Language page (now showing Spanish active).
  - Tap "한국어" (preview-select Korean).
  - Confirm the right action button now reads "재시작하여 적용" in blue (Korean text in CJK font).
  - Tap "재시작하여 적용".

  Watch: the current refbox process should exit; a fresh one should appear (Claude monitors logs in the background-launched cargo run).

  Confirm with the operator: a new refbox window appears with all UI in Korean, time bar period names in CJK font.

  Mark Feature 2 "RESTART TO APPLY persists" scenario.

- [ ] **Step 7.10: Walkthrough Session — Step 6 (CJK → Latin restart round-trip).**

  Ask the operator to:
  - From the now-Korean app: navigate to App Options → tap the "language" button.
  - Confirm the Language page opens with action-bar text in CJK font (Cancel = "취소", right button = "재시작하여 적용" because the current-language preview-selection makes the same-family check pass — wait, the active language IS Korean now, so preview-selecting another CJK language shows DONE, preview-selecting a Latin language shows RESTART TO APPLY).
  - Tap "ENGLISH".
  - Confirm Cancel now reads "CANCEL" in Roboto, right button reads "RESTART TO APPLY" in Roboto (the tofu fix means Latin text renders in Roboto-Medium even though the app's current default font is the CJK family).
  - Tap "RESTART TO APPLY".

  Confirm: process restarts; new refbox window in English; no tofu observed during the brief preview pause before the restart.

  Mark Feature 1 "Action-bar script font" scenario + Feature 2 "CJK→Latin restart" scenario.

- [ ] **Step 7.11: Walkthrough Session — Step 7 (UNVERIFIED markers visual check).**

  Ask the operator to:
  - Re-enter the Language page from the now-English app.
  - Visually confirm every language button:
    - ENGLISH: no note.
    - ESPAÑOL: no note.
    - FRANÇAIS: no note.
    - All 12 other buttons: a note in small text below the language name, in the language's own script.
  - Specifically confirm Bahasa Indonesia button: single small-text line "BAHASA INDONESIA" with "(BELUM DIVERIFIKASI)" below.
  - Decide: is the Bahasa shape acceptable? (Catalog entry B8.22 was walkthrough-decided.)

  Mark Feature 3 all scenarios. If operator decides B8.22's Bahasa shape is NOT acceptable, mark it `@findings-backlog` in AUDIT-PLAN.md and add to the Findings backlog From-Unit-8 subsection with a follow-up branch suggestion.

- [ ] **Step 7.12: Walkthrough Session — Step 8 (Damage-tracking regression sweep).**

  Ask the operator to navigate to each config sub-page and visually check button text alignment + lack of glyph bleed:
  - Main config page
  - Game Options
  - App Options (the language button on this page is itself a damage-tracking workaround call site)
  - Display
  - Sound

  At each, the operator confirms: no button-text alignment regression vs. prior memory of how these pages looked. No glyph bleed (Latin text on a Latin app should never show glyph bleed; this check is mostly a baseline).

  Mark Feature 4 "Existing config-page buttons" scenario per observations.

  If any regression is found: per Task 5 Step 5.5's agreement, file as findings-backlog and keep the Feature 4 sweep on the audit branch. The regression itself goes to Findings backlog From-Unit-8 with a follow-up branch suggestion.

- [ ] **Step 7.13: Walkthrough Session — Step 9 (Window-position B8.C1 decision).**

  Ask the operator to look at the current refbox window placement:
  - Where is the main refbox window? (Currently `Position::Centered`.)
  - Where is the simulator window if visible? (Currently `Position::Specific((0.0, 40.0))`.)

  Ask: "Are these placements acceptable, or should the audit revert them?"

  - If acceptable: update B8.C1 catalog entry from `@proposed` (walkthrough-deferred) to `@user_verified` with the operator's brief justification noted.
  - If revert: keep B8.C1 as `@proposed` for now and immediately do the surgical revert in Step 7.14.

- [ ] **Step 7.14: (Conditional) Surgical revert of window-position changes (B8.C1).**

  ONLY if Step 7.13 chose revert.

  Edit `refbox/src/main.rs` in the worktree:
  - Remove the line `position: window::Position::Specific(iced::Point::new(0.0, 40.0)),` from the simulator window settings (around line 268 in current master).
  - Remove the line `position: window::Position::Centered,` from the main window settings (around line 444 in current master).

  Run `just check`:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just check 2>&1 | tail -10
  ```
  Expected: clean.

  Commit:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git add refbox/src/main.rs \
    && git commit -m "audit(refbox): revert unrelated window-position changes from language commit

848138c added two window-position lines that are unrelated to
language UI chrome — simulator window pinned to (0.0, 40.0)
and main window Centered. The commit body offered no
positioning rationale. Walkthrough decision (Unit 8 catalog
entry B8.C1) sent the change to findings-backlog for a
deliberate window-placement design pass later.

Restores prior \`..Default::default()\` behaviour for both
window settings."
  ```

  Update B8.C1 catalog entry to `@findings-backlog`. Add to Findings backlog From-Unit-8 with branch suggestion `feat/refbox/window-placement-design`.

- [ ] **Step 7.15: Stop the refbox process and commit the session notes.**

  Kill the background refbox run (the harness will notify when the cargo process exits; if it's still running because the operator is mid-walkthrough, ask the operator to close the refbox window first, then the cargo run terminates naturally).

  Update `refbox/tests/features/language-ui-chrome.feature` with the final `@tested_*` tags and the `walkthrough: YYYY-MM-DD HH:MM` lines populated for each scenario.

  Add a session-summary comment block at the top of the file:
  ```gherkin
  # Test session 1 — YYYY-MM-DD — language UI chrome audit walkthrough
  # Environment: native refbox launch in worktree (WAYLAND_DISPLAY= cargo run -p refbox)
  # Scenarios passed: <N> / Total: <N>
  # Operator observations: <one-liner per Feature block>
  ```

  Commit:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git add refbox/tests/features/language-ui-chrome.feature \
    && git commit -m "audit(refbox): record Unit 8 walkthrough session notes"
  ```

- [ ] **Step 7.16: Final `just check` on the audit-branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just check 2>&1 | tail -40
  ```

  Expected: still green. If any test fails *after* the prune + walkthrough commits, something regressed during the audit — investigate before proceeding.

- [ ] **Step 7.17: Update any catalog entries that the walkthrough resolved.**

  - B8.C1: now `@user_verified` (keep) or `@findings-backlog` (revert).
  - B8.22: now `@user_verified` (Bahasa shape acceptable) or `@findings-backlog` (re-shape request).
  - Any `@tested_fail` scenario: flag in catalog as the linked entry needs operator follow-up (do NOT silently flip Decision to `@deleted` — a wanted-but-broken behaviour is a separate finding).

---

## Task 8: Write retroactive ADR 023 (AUDIT-PLAN.md Step 7)

**Files:**
- Create: `docs/decisions/023-language-ui-chrome.md` (on audit branch)

- [ ] **Step 8.1: Confirm 023 is still the next free ADR number.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && ls docs/decisions/ | grep -E '^02[0-9]-' | sort
  ```

  Expected outcome at execution time: file numbers 019–022 visible (from the other audit branches' ADRs that have been merged into this audit branch via origin/master — wait, those branches are local-only and have NOT been merged). Actually, at the time of writing, the audit branch is cut directly from `origin/master` `089c98d` which does NOT include ADRs 019–022 (they live only on the other audit branches). So the directory listing on this audit branch will show:

  ```
  013-cold-restart-state-recovery.md
  014-live-settings-preview.md
  015-refbox-stats-endpoint-handling.md
  016-uwr-mode-portal-routing.md
  017-portal-data-lifecycle.md
  018-event-picker-sort-order.md
  ```

  (no 019–022 because those audit branches are local-only). ADR 023 is still the correct number for Unit 8 because Final Integration will merge all audit ADRs together, and the audit-design spec already committed to 023 for Unit 8.

  Verify no `023-` file exists on this branch:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && ls docs/decisions/023-* 2>/dev/null && echo "EXISTS" || echo "OK — 023 is free"
  ```
  Expected: `OK — 023 is free`.

- [ ] **Step 8.2: Draft the retroactive ADR.**

  Create `docs/decisions/023-language-ui-chrome.md` using the playbook's retroactive ADR template (AUDIT-PLAN.md → Templates → Retroactive ADR template) plus the playbook's "Decision section embeds @user_verified @tested_pass scenarios verbatim" rule (Gherkin Scenario plan section).

  Structure:

  ```markdown
  # ADR 023: Language UI chrome

  **Status:** Accepted (retroactive)
  **Date:** YYYY-MM-DD
  **Audit unit:** 8 — Grid-select page + UNVERIFIED marker
  **Audit branch:** `audit/refbox/language-ui-chrome`

  ## Context

  This ADR documents behaviour added to the refbox between 2026-04-18 (commit
  `848138c`) and 2026-04-18 (commit `ea151ac`) with AI assistance. The behaviour
  was audited 2026-05-15 (Unit 8 of the AI Code Audit per AUDIT-PLAN.md) and
  the surviving parts are recorded here.

  The previous language selection mechanism was a cycle-through-languages button
  on the App Options page (CyclingParameter::Language) that rotated through
  English → French → Spanish → English. The refbox supported only those three
  languages and only Latin script. The two commits audited under Unit 8 expand
  the supported set to 15 languages spanning Latin, CJK (Korean, Japanese,
  Mandarin), and Thai scripts, with bundled font subsets for the non-Latin
  families; replace the cycle-button with a dedicated grid-selection page; add
  an UNVERIFIED marker on each language whose translation has not yet been
  native-speaker reviewed; and apply an iced-0.13 damage-tracking workaround
  to every button helper in the app.

  Translation file content for the 15 locales and the bundled font binaries are
  out of scope of this audit — their accuracy is deferred indefinitely pending
  native-speaker review per the playbook's scope reduction of 2026-05-12.

  ## Decision

  The refbox UI presents a dedicated **Language selection page** reachable from
  the App Options grid via a "language" button. The page is the only way for
  the operator to change the active UI language.

  ### Feature 1 — Language selection page

  <Plain-English wrap: ~2 sentences for the Feature> Then embed every
  @user_verified @tested_pass scenario from language-ui-chrome.feature
  Feature 1 block verbatim as Gherkin code blocks.

  ### Feature 2 — Restart-required indicator and flow

  <Plain-English wrap, then embed @user_verified @tested_pass scenarios>

  ### Feature 3 — UNVERIFIED marker on language buttons

  <Plain-English wrap, then embed @user_verified @tested_pass scenarios>

  ### Feature 4 — Button-text damage-tracking workaround

  <Plain-English wrap, then embed @user_verified @tested_pass scenarios.
  Note: this Feature affects every button in the app, not just the language
  page — the audit treats it as a cross-cutting consequence of the language
  work because it was triggered by the language-switch script-change.>

  ### Code-only behaviours (no Gherkin scenarios)

  - **Window-position changes in `main.rs`:** <kept | reverted — per B8.C1
    outcome>.
  - **Dead-code `impl Cyclable for Language`:** removed during audit.

  ## Consequences

  - The operator can pick a language by name from a visible grid rather than
    cycling blindly.
  - Switching between Latin and non-Latin scripts requires a restart and the
    operator sees this clearly via the action-bar button label.
  - Translations not yet reviewed by a native speaker are clearly marked.
  - Every button in the app is now resistant to glyph-bleed when its text
    content changes script — a small overhead in widget structure for a
    fix to a hard-to-spot visual bug.
  - The 15-locale `.ftl` translation files are committed but their accuracy
    is not warranted by this audit. Future work will review them per language.

  ## What was removed during audit

  - **Dead `impl Cyclable for Language`** (B8.C2). The `CyclingParameter::Language`
    caller was removed in `848138c` but the impl was extended in both audited
    commits anyway. Removed in audit commit <SHA>.
  - **Window-position changes in `main.rs`** (B8.C1) — only if the walkthrough
    chose revert. Removed in audit commit <SHA>. If kept, this bullet reads
    "none in this category."

  ## What was not verified

  - **Translation accuracy** for the 14 non-English locales is deferred to
    native-speaker review (out of scope).
  - **Font subset completeness** for the bundled CJK and Thai font binaries
    is accepted as-is from the audited commits.
  - **First-launch behaviour from a freshly-deleted config** — partially
    verified via inspection of the destructuring path in `Config::sanitize_old`
    but not tested by deleting and re-launching.
  - **Multi-tournament regression** of the damage-tracking sweep — the
    walkthrough exercised the five config sub-pages but not all in-game and
    overlay surfaces. Production use over the next tournament will be the
    final acceptance check.

  ## Audit reference

  - Audit branch: `audit/refbox/language-ui-chrome` (held local until Final
    Integration)
  - Audit per-unit plan: `docs/superpowers/plans/2026-05-15-audit-unit-8-language-ui-chrome.md`
  - Audit design spec: `docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md`
  - Original commits: `848138c`, `ea151ac`
  - Original design (for ea151ac only): `docs/superpowers/specs/2026-04-17-turkish-language-and-unverified-label-design.md`
  - Original plan (for ea151ac only): `docs/superpowers/plans/2026-04-17-turkish-language-and-unverified-label.md`
  - 848138c had no pre-existing design spec; this ADR retroactively becomes
    its design record.
  ```

  Fill in real scenario blocks (the verbatim contents of `language-ui-chrome.feature`'s `@user_verified @tested_pass` scenarios) and real audit commit SHAs at execution time.

- [ ] **Step 8.3: Commit the ADR.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome \
    && git add docs/decisions/023-language-ui-chrome.md \
    && git commit -m "docs(refbox): add ADR 023 for language UI chrome (retroactive)

Retroactive ADR captures the operator-confirmed shape of the
language selection page (4-row grid, preview-then-confirm),
the restart-required flow (when crossing font-family boundaries),
the UNVERIFIED marker rule (English/Spanish/French exempt),
and the cross-cutting button-text damage-tracking workaround.

References commits 848138c (grid page + 11 languages + CJK/Thai
fonts) and ea151ac (Turkish + UNVERIFIED marker). Embeds the
@user_verified @tested_pass Gherkin scenarios from
refbox/tests/features/language-ui-chrome.feature verbatim.

Translation accuracy for the 14 non-English locales is
explicitly NOT verified by this audit — deferred to native-
speaker review per the playbook's 2026-05-12 scope reduction."
  ```

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Steps 8 + 9)

**Files:**
- Modify: `AUDIT-PLAN.md` (gitignored; status flip + Completed audits summary + Process refinements + Findings backlog)
- Modify: memory files at `/home/estraily/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/`

- [ ] **Step 9.1: Operator reviews the decision log, test status, and ADR.**

  Ask the operator to:
  1. Read the Unit 8 catalog (Decision column) in `AUDIT-PLAN.md`
  2. Read `refbox/tests/features/language-ui-chrome.feature` for test-tag distribution
  3. Read `docs/decisions/023-language-ui-chrome.md` on the audit branch
  4. Confirm: "Unit 8 approved" or request changes

  Iterate on changes until the operator confirms approval.

- [ ] **Step 9.2: Flip Unit 8 status in AUDIT-PLAN.md.**

  In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
  - Update the unit-catalog table row for Unit 8: `in progress (started YYYY-MM-DD)` → `complete-pending-integration (YYYY-MM-DD)`.
  - Update the `### Unit 8 — Grid-select page + UNVERIFIED marker` section's `**Status:**` line the same way.

- [ ] **Step 9.3: Add a summary entry to "Completed audits".**

  Per Unit 1 refinement #3 (in-place flip + summary pointer, not destructive section-move), add an entry to the Completed audits section near the bottom of `AUDIT-PLAN.md`, between the existing Unit 7 entry and any earlier entries (newest first):

  ```markdown
  #### Unit 8 — Grid-select page + UNVERIFIED marker — complete-pending-integration YYYY-MM-DD

  - **Branch:** `audit/refbox/language-ui-chrome` (local only; <N> commits ahead of `origin/master` `089c98d`; not pushed)
  - **Per-unit plan:** `docs/superpowers/plans/2026-05-15-audit-unit-8-language-ui-chrome.md`
  - **Audit-design spec:** `docs/superpowers/specs/2026-05-15-audit-unit-8-language-ui-chrome-design.md`
  - **ADR (new, retroactive):** `docs/decisions/023-language-ui-chrome.md` (Accepted retroactive)
  - **Scenarios:** `refbox/tests/features/language-ui-chrome.feature` — <N> scenarios across 4 Features; <X> @tested_pass, <Y> @tested_fail, <Z> @tested_inconclusive
  - **Catalog outcome:** <N> entries; <X> @user_verified; <Y> @deleted; <Z> @findings-backlog; <W> @redesign-followup
  - **Tests added during audit:** none — language UI chrome is walkthrough-verified per the lean-process refbox-UI convention
  - **Audit commits on branch:** <list of SHAs — Cyclable revert (B8.C2), optional window-position revert (B8.C1 conditional), Gherkin seed commit, walkthrough session-notes commit, ADR 023 commit>
  - **What was not verified:** translation accuracy for 14 non-English locales; full first-launch-from-deleted-config walkthrough; multi-tournament regression of damage-tracking sweep
  - **Findings filed:** <N> new Findings backlog items (typically: B8.16 confy unwrap on language save; optionally B8.C1 window-position; optionally B8.22 Bahasa shape; cross-unit reconcile entries from Step 3.7)
  - **Process refinements:** <N> new refinements logged
  - **Cross-branch dependencies:** none observed (the in-scope files are touched by other audit branches but the changes don't conflict at content level)
  - **Full details section:** retained in "Unit-by-unit details" above with status flipped to complete-pending-integration.
  ```

  Fill in real values at execution time.

- [ ] **Step 9.4: Add Findings backlog entries.**

  Per Step 5.9, any `@findings-backlog` decisions or any out-of-scope discoveries from Tasks 2–7 are listed under `### Findings backlog → #### From Unit 8 (YYYY-MM-DD)`. Expected entries:
  - B8.16 — `confy::store(..).unwrap()` on language save in `mod.rs` LanguageSelectComplete handler. Branch suggestion: `chore/refbox/confy-error-paths` (or roll into the eventual fix for Unit 3 finding #2).
  - Cross-unit reconcile entries from Step 3.7:
    - B8.X1 — `team-ref-list` orphan key sweep → `chore/refbox/remove-unused-team-ref-list-keys`.
    - B8.X2 — `portal-row-attempt-suffix` 14-locale population → `chore/refbox/portal-row-attempt-suffix-14-locales` or absorb at Final Integration.
  - Conditional from Task 7:
    - B8.C1 — window-position revert if walkthrough chose `@findings-backlog`. Branch suggestion: `feat/refbox/window-placement-design`.
    - B8.22 — Bahasa button shape revert if walkthrough chose `@findings-backlog`. Branch suggestion: `chore/refbox/bahasa-3-line-button-shape`.

- [ ] **Step 9.5: Add Process refinements log entries (if any).**

  Under `### Process refinements log → #### From Unit 8 (YYYY-MM-DD)`. Examples to watch:
  - Confirmation that single-pass (no-subagent-dispatch) catalog construction works cleanly at the 20–30-entry range (revalidation of Unit 6 process refinement).
  - Per-Feature batched review at 4 Features confirmed efficient — operator handled all four block-approvals + a small number of carve-outs in under N questions.
  - Walkthrough script's Latin↔CJK restart round-trip executed cleanly via `WAYLAND_DISPLAY=` prefix on WSLg (revalidation of Unit 1 refinement #7).
  - Window-position B8.C1 decision flow worked as designed (catalog-then-walkthrough-decide).

- [ ] **Step 9.6: Run `just check` once more on the audit branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-8-language-ui-chrome && just check 2>&1 | tail -30
  ```

  Expected: green (matches Step 7.16).

- [ ] **Step 9.7: (Principal-only follow-up) Update Claude's memory files.**

  This step is performed by the principal Claude session, not an executing subagent — memory at `~/.claude/projects/.../memory/` is principal-only territory. After Step 9.6 reports green, the principal:

  - Updates `project_v040_handover.md`: mark Unit 8 as complete-pending-integration; add Unit 8's audit branch to the "Audit branches" list with its commit count and date; note that the cross-unit reconcile items B8.X1 and B8.X2 are now filed in Findings backlog.
  - Updates audit progress count: "7 of 9 units complete" → "8 of 9 units complete".
  - Notes Unit 9 (stale branches cleanup) as next.
  - Updates `MEMORY.md` index if the handover entry's hook sentence needs to change.
  - If any new feedback memory was earned during Unit 8 (e.g., a walkthrough-script lesson, a per-Feature batched-review lesson), write it as its own memory file and add the index pointer.

- [ ] **Step 9.8: Confirm the unit is closed.**

  Tell the operator:

  > "Unit 8 complete-pending-integration. Branch `audit/refbox/language-ui-chrome` holds locally with <N> audit commits. ADR 023 added on the audit branch (new retroactive ADR). `AUDIT-PLAN.md` status flipped. Memory updated. 8 of 9 units complete; Unit 9 (stale branches cleanup) is next, and after that Final Integration."

---

## Risks and known divergences

Starting points for catalog questions, not pre-decisions. (See the spec's Risk section for the canonical list; the items below are the per-task pickup of those risks.)

1. **Process-restart path is hard to test mechanically.** Spawning `std::env::current_exe()` is WSL-vs-native-Linux sensitive. Task 7 Steps 7.9 and 7.10 are the only meaningful verifications. If the restart hangs or spawns a non-functional child on WSLg, file as `@tested_fail` and stop-and-discuss per Step 6 protocol.
2. **Damage-tracking sweep may have subtle regressions on non-language pages.** Task 7 Step 7.12 exercises the five config sub-pages. If a regression is found, file as findings-backlog and keep the Feature 4 sweep on the audit branch (Step 5.5 agreement).
3. **iced 0.13 damage-tracking bug is the stated justification for the layout sweep.** The audit is not equipped to verify the bug exists without the workaround. Treat as "documented workaround that operates correctly" — if the walkthrough finds a regression, the regression itself is the new follow-up, not a revert of the workaround.
4. **Action-bar font fallback for Latin under CJK locale.** Task 7 Step 7.10 covers this (the tofu fix). Requires actually being in a CJK locale when opening the Language page.
5. **First-launch behaviour with corrupt or absent `config.language`.** Defaults to `None` → English at runtime. Task 4.2 partially validates this via inspection; full walkthrough verification by deleting the config and re-launching is deferred to "What was not verified."
6. **Cross-unit shared files** (`shared_elements.rs`, `mod.rs`, `configuration.rs`) get cross-unit notes per Unit 6 refinement #3 in Task 4.1. Watch for collisions with Units 1 (confirm-score), 3 (settings), 4 (manual alarm), 5 (referee names), 6 (small fixes), 7 (portal health) at Final Integration.
7. **Cross-branch dependencies.** Task 4.1 runs `git log --all -S '<symbol>'` for the touched helpers. None expected (Unit 8's diff is well-contained in the language UI chrome surface); if any found, hand-apply and record per Unit 5 pattern.
8. **The walkthrough script touches the simulator window.** If the sim window's `Position::Specific(0.0, 40.0)` placement obscures the main window on the operator's display, surface immediately at Task 7 Step 7.4. The simulator window can be closed entirely if it's in the way; the main walkthrough doesn't depend on it.

---

## Deviations

> Filled in during execution. Per heavy-process discipline, deviations are recorded here as a running section AND each significant deviation gets its own commit when it lands on a branch. Lean-process per-task deviation commits are NOT used for Unit 8 (heavy process).

(none yet)

---

## Files Created or Modified by This Plan

- `.worktrees/audit-unit-8-language-ui-chrome/` (new worktree, lifecycle: removed at Final Integration)
- `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (gitignored; multiple edits — history trace, behaviour catalog with four Features + Code-only + Cross-unit reconcile, Findings backlog from-Unit-8 subsection, Process refinements from-Unit-8 subsection, status flips, Completed audits summary)
- `.audit/unit-8-commits-raw.txt` (local working artifact)
- `.audit/unit-8-commit-messages.txt` (local working artifact)
- `.audit/unit-8-files-touched.txt` (local working artifact)
- `.audit/unit-8-files-848138c.txt` (local working artifact)
- `.audit/unit-8-files-ea151ac.txt` (local working artifact)
- `refbox/tests/features/language-ui-chrome.feature` (created on audit branch with 4 `Feature:` blocks)
- `refbox/src/app/languages.rs` (B8.C2 surgical revert on audit branch — remove dead `impl Cyclable for Language` + orphan `use super::Cyclable;` import)
- `refbox/src/main.rs` (conditional B8.C1 revert on audit branch — only if walkthrough chose revert)
- `docs/decisions/023-language-ui-chrome.md` (new retroactive ADR on audit branch)
- Memory `project_v040_handover.md` and `MEMORY.md` (updated at close); any new memory files written by the principal in Step 9.7

---

## Estimated commits on the audit branch

- 1 prune commit (B8.C2 dead Cyclable revert) — Step 6.2.g
- 1 scenario-seeding commit — Step 7.3
- 0–1 conditional B8.C1 window-position revert commit — Step 7.14
- 1 walkthrough session-notes commit — Step 7.15
- 1 ADR 023 commit — Step 8.3
- **Total on `audit/refbox/language-ui-chrome`:** **4–5 audit commits** at close (4 if B8.C1 kept, 5 if reverted). Matches the spec §5 estimate of 3–5.
