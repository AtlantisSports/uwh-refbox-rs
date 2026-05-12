# Audit Unit 3 — ADR 009 Settings Navigation: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit the 10 commits on `refactor/refbox/settings-navigation` that implement ADR 009 (settings navigation and layout), let the operator decide keep/delete on every behaviour they shipped, prune what's rejected, exercise what survives, and **finalize ADR 009 in place** (flip from `proposed` to `accepted`, fold approved deviations into its Decision section).

**Architecture:** Page-by-page catalog grouping (Main → User Options → Game Options → App Options → Display Options → Sound Options → Manage Remotes → Language → cross-cutting). Operator-observable grain. Mixed-weight verification: heavy (Rust regression tests) for state-machinery invariants — per-page snapshot/revert, `uwhportal_incomplete()` Apply-disable, mid-game Apply gates, picker-driven field clearing; lean (manual UI walkthrough) for chrome and navigation. ADR 009 amended in place; no new ADR number consumed. Side ADRs 017 and 018 fenced off (separate units).

**Tech Stack:** Rust 2024 / MSRV 1.85; iced 0.13 (refbox UI); `cargo test`, `cargo clippy`, `just check`; manual refbox launch (`WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox`); Gherkin scenarios in `refbox/tests/features/`.

---

## Acceptance criteria (Unit 3 "complete-pending-integration")

The unit is complete-pending-integration when **all** of these hold on the audit branch `audit/refbox/adr-009-settings`:

1. A behaviour catalog exists in `AUDIT-PLAN.md` under Unit 3, grouped by page, with every entry tagged `@user_verified` or `@deleted`. No `@proposed` remaining.
2. Every kept state-machinery behaviour is covered by a Rust regression test in `refbox/src/app/` (alongside the code it locks) or `refbox/tests/`.
3. Every kept operator-observable behaviour is documented as a Gherkin scenario in `refbox/tests/features/adr_009_settings.feature` with workflow tags reflecting verification status (`@user_verified` + `@tested_pass` or `@manual_walkthrough_only`).
4. The operator has driven the refbox UI through every kept behaviour at least once. Each scenario carries a manual-walkthrough timestamp.
5. ADR 009 at `docs/decisions/009-settings-navigation-layout.md` is amended in place: Decision section reflects shipped reality (approved deviations folded in); status flipped from `proposed` to `accepted`; "What is changing beyond navigation" section captures any behaviour shipped beyond original Decision; cross-reference to Unit 3's AUDIT-PLAN.md section added.
6. `just check` passes on the audit branch.
7. The branch holds locally (no push, no PR) per playbook Step 8.
8. AUDIT-PLAN.md status flipped from "not started" to "complete-pending-integration"; summary pointer added to "Completed audits" section per playbook-amended Step 9.4.
9. Findings discovered out-of-scope are recorded in AUDIT-PLAN.md's Findings Backlog with a suggested follow-up branch name. They are **not fixed** on this branch.

---

## Prerequisites

- Read `AUDIT-PLAN.md` Unit 3 section and the Process refinements log entries from Units 1 & 2.
- Read `docs/decisions/009-settings-navigation-layout.md` (ADR 009 itself).
- Read `docs/superpowers/plans/2026-04-20-adr-009-settings-navigation.md`, especially the Deviations log section starting at line 1118.
- Read `.claude/rules/scope.md`, `communication.md`, `workspace.md`, `rust.md`, `plan-execution.md`, `pr-review.md`.
- Confirm the in-flight worktree at `/home/estraily/projects/refbox-settings-worktree/` is paused (user already confirmed; latest commit `ce6cfeb` 2026-05-12; working tree clean).

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:**
- Create: `.worktrees/audit-unit-3-adr-009-settings/` (new worktree)
- Modify: `<main-repo>/.git/hooks/pre-commit` (if not already audit-aware from Unit 1)

- [ ] **Step 1.1: Cut the audit branch from `ce6cfeb`**

The audit branch starts at HEAD of the in-flight settings-navigation branch.

```bash
cd /home/estraily/projects/uwh-refbox-rs
git worktree add -b audit/refbox/adr-009-settings .worktrees/audit-unit-3-adr-009-settings refactor/refbox/settings-navigation
```

Expected output: `Preparing worktree (new branch 'audit/refbox/adr-009-settings')` then `HEAD is now at ce6cfeb refactor(refbox): unify settings page layouts and rearrange game options`.

- [ ] **Step 1.2: Verify the audit branch starting point**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git log --oneline -1 && git rev-parse --abbrev-ref HEAD
```

Expected output:
```
ce6cfeb refactor(refbox): unify settings page layouts and rearrange game options
audit/refbox/adr-009-settings
```

- [ ] **Step 1.3: Confirm pre-commit hook is audit-aware**

The hook was updated in Unit 1 (commit `2a8dcbc` on `audit/refbox/confirm-score-timing`) to allow `audit` branch type. Until that lands on master, every fresh worktree's pre-commit hook (shared via `<main-repo>/.git/hooks/pre-commit`) must allow `audit`.

```bash
cd /home/estraily/projects/uwh-refbox-rs && grep -c '\baudit\b' .git/hooks/pre-commit
```

Expected: non-zero count (hook recognizes `audit` branch type).

If zero, copy the audit-aware version from Unit 1's branch:
```bash
cd /home/estraily/projects/uwh-refbox-rs && git show audit/refbox/confirm-score-timing:scripts/pre-commit > .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
```

- [ ] **Step 1.4: Sanity-check the worktree builds**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && cargo check -p refbox 2>&1 | tail -5
```

Expected: clean compile (the worktree starts at the in-flight HEAD; if this fails, the in-flight branch was not in a clean state, which contradicts the user's "Pause and audit now" decision — stop and check with user).

---

## Task 2: Generate the diff (AUDIT-PLAN.md Step 2)

**Files:**
- Create: `.audit/unit-3-diff.txt` (not committed; local working artifact)
- Create: `.audit/unit-3-commit-summaries.md` (not committed; local working artifact)

- [ ] **Step 2.1: List the 10 commits with full subject lines**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && mkdir -p .audit && git log origin/master..HEAD --oneline > .audit/unit-3-commits-summary.txt && cat .audit/unit-3-commits-summary.txt
```

Expected: 10 commits, oldest at bottom, newest at top, matching:
```
ce6cfeb refactor(refbox): unify settings page layouts and rearrange game options
f3aafbf feat(refbox): roll out Cancel/Apply chrome to remaining settings pages
65d7f97 feat(refbox): Game Options gains Cancel/Apply chrome
4ba2753 feat(refbox): move game-number picker to Game Options, restructure Main as 2x2 grid
0686efa feat(refbox): add User Options page
6a79dbf feat(refbox): add per-page Apply/Cancel messages and handlers
7dc7a2c feat(refbox): add per-page entry snapshot and change detection
32db7e8 refactor(refbox): split apply_settings_change into per-page slice functions
f41cc30 feat(refbox): add ConfigPage::User variant with placeholder routing
15320f3 chore(refbox): add apply and user-options translation keys
```

- [ ] **Step 2.2: Generate the full diff and the per-commit messages**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git log origin/master..HEAD --reverse --stat > .audit/unit-3-stat.txt && git log origin/master..HEAD --reverse --format='COMMIT %h %s%n%n%b%n----' > .audit/unit-3-messages.txt && git diff origin/master..HEAD > .audit/unit-3-diff.txt && wc -l .audit/unit-3-*.txt
```

Expected: messages and stat files populated; diff size shown.

- [ ] **Step 2.3: Pull the original plan's deviations log into the audit workspace**

The deviations log at `docs/superpowers/plans/2026-04-20-adr-009-settings-navigation.md` lines 1118–1162 captures behaviours added during execution beyond ADR 009's Decision section. These are catalog candidates.

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && sed -n '1110,1162p' docs/superpowers/plans/2026-04-20-adr-009-settings-navigation.md > .audit/unit-3-deviations.md && wc -l .audit/unit-3-deviations.md
```

Expected: ~53 lines of deviations log content.

- [ ] **Step 2.4: Confirm the diff scope is `refbox` only**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git diff --name-only origin/master..HEAD | awk -F'/' '{print $1}' | sort -u
```

Expected: only `refbox` and `translations`. If anything else appears, flag to user before proceeding — that's out-of-stated-scope per the catalog note (`refbox` is the unit's scope).

---

## Task 3: Build the behaviour catalog (AUDIT-PLAN.md Step 3)

**Files:**
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Unit 3 section near line 990; this is the main repo's working file, gitignored)

Catalog entries follow the template in AUDIT-PLAN.md's "Templates" section. **Build order is chronological by commit** (so we don't miss anything in the diff). **Presentation order in the catalog file is page-grouped** (so the user-review session in Task 4 flows naturally).

Catalog grain: **operator-observable behaviour**. Each entry covers one fact the operator can see, feel, or trigger.

Each catalog entry MUST include:
- `Behaviour:` — plain English description of what the operator observes
- `Type:` — `pure-UI` | `state-machine` | `translation` | `internal-refactor`
- `Page(s):` — where the operator encounters this behaviour
- `Originating commit(s):` — short hash(es) from the 10
- `ADR 009 reference:` — bullet from Decision section, or `added beyond ADR 009`
- `Deviations-log link:` — line range in the original plan's deviations log, if applicable
- `Status:` — `@proposed` until the user-review session in Task 4 sets `@user_verified` or `@deleted`

- [ ] **Step 3.1: Open the catalog section and add the page groupings**

In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`, locate the Unit 3 section (currently near line 990, header `### Unit 3 — ADR 009 in-flight work`). Under the "Behaviour catalog · Scenarios · Decisions · Pruning record · Retroactive ADR" subheading, replace the `*(filled during execution)*` placeholder with the page-group scaffold:

```markdown
#### Behaviour catalog

##### Page: Main (settings entry)
*(entries appended here)*

##### Page: User Options
*(entries appended here)*

##### Page: Game Options
*(entries appended here)*

##### Page: App Options
*(entries appended here)*

##### Page: Display Options
*(entries appended here)*

##### Page: Sound Options
*(entries appended here)*

##### Page: Manage Remotes
*(entries appended here)*

##### Page: Language
*(entries appended here)*

##### Cross-cutting (snapshot, change detection, translation keys, internal refactors)
*(entries appended here)*
```

- [ ] **Step 3.2: Walk the 10 commits chronologically and append entries**

For each commit (oldest first: `15320f3` → ... → `ce6cfeb`):

1. Read `.audit/unit-3-messages.txt` for that commit's message body — the body's `Changes:` list is the most reliable source of operator-observable behaviour facts.
2. Inspect the diff for that commit: `git show <hash> --stat` then `git show <hash> -- <suspect-file>`.
3. For each operator-observable fact, write a B-entry under the matching page group. Cross-reference ADR 009 and the deviations log.
4. For each fact that is purely internal (refactor with no operator-visible behaviour change — e.g., `apply_settings_change()` split into slices), put it under "Cross-cutting" with `Type: internal-refactor`. These still need user approval to keep (per playbook principle: "every behaviour has been explicitly approved").

**Entry template:**

```markdown
##### B3.N — <short name>

- **Behaviour:** <plain English; one to two sentences>
- **Type:** <pure-UI | state-machine | translation | internal-refactor>
- **Page(s):** <Main | User Options | Game Options | ... | Cross-cutting>
- **Originating commit(s):** `<hash>` (`<commit subject>`)
- **ADR 009 reference:** <Decision-section bullet text, or "added beyond ADR 009">
- **Deviations-log link:** `2026-04-20-adr-009-settings-navigation.md:<line-range>` (omit if not in deviations log)
- **Status:** `@proposed`

<Optional: 1–2 lines on why this might be worth questioning, especially for "added beyond ADR 009" entries.>
```

- [ ] **Step 3.3: Slop sweep against the catalog**

After the catalog scaffold is filled, run the slop-catching checklist from AUDIT-PLAN.md. For Unit 3 specifically:

- [ ] Are there `unwrap()` / `expect()` calls added in the 10 commits without justifying comments? `git diff origin/master..HEAD -- '*.rs' | grep -E '^\+.*\.(unwrap|expect)\('` then audit each.
- [ ] Are there bundled fixes inside feature commits (Unit 2 refinement #2)? Check each `feat(refbox)` commit message body — does the `Changes:` list span more than one concern? If yes, list each concern as a distinct B-entry and flag the bundling for the Process refinements log regardless of keep/delete outcome.
- [ ] Are there `#[allow(...)]` attributes added with `clippy::large_enum_variant` or similar? The deviations log already flags one on `PageEntrySnapshot` (Task 8 deviation). Capture as a B-entry under cross-cutting.
- [ ] Did any new dependency get added? `git diff origin/master..HEAD -- '**/Cargo.toml'`. If yes, flag.
- [ ] Translation-key audit: every new key in `translations/` should be reachable from a kept UI element. List unreachable keys for the user to confirm.

- [ ] **Step 3.4: Verify entry count is reasonable**

Expected: **15–25 catalog entries** based on the design brainstorm. If the count is below 12 or above 30, re-check granularity. Single-commit shipping multiple operator-observable facts (e.g., `f3aafbf` rolls out Cancel/Apply chrome to four pages) should produce multiple entries — one per page or one per chrome variant, not one combined entry.

- [ ] **Step 3.5: Commit the catalog scaffold**

The catalog lives in the gitignored AUDIT-PLAN.md — no commit on the audit branch. Verify the working tree is still clean:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git status --short
```

Expected: nothing tracked changed (audit-side state lives in the main repo's AUDIT-PLAN.md and the `.audit/` artifacts).

---

## Task 4: User review session (AUDIT-PLAN.md Step 4)

**Files:**
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Unit 3 catalog: flip `@proposed` to `@user_verified` or `@deleted`; add Decisions subsection)

Walk page-by-page through the catalog. For each page section, for each B-entry within it:

1. State the plain-English summary of the behaviour
2. State Claude's keep/delete/redesign recommendation with reason
3. Ask the user — one question at a time, never bundled (per `feedback_one_question_at_a_time`)
4. Record the user's decision in the catalog: `Status: @user_verified` or `Status: @deleted` or `Status: @redesign-followup`
5. For deletions and redesigns, capture the user's rationale in a one-line note appended to the entry

- [ ] **Step 4.1: Confirm review order with the user**

Before launching the session, confirm with the user:
> "I'll walk the catalog page-by-page in this order: Main → User Options → Game Options → App Options → Display Options → Sound Options → Manage Remotes → Language → Cross-cutting. Each entry is one question. Ready to start, or do you want to reorder?"

- [ ] **Step 4.2: Walk the catalog one entry at a time**

For each entry, use the `AskUserQuestion` tool with three options:
- "Keep as-is" (recommended for entries matching ADR 009 intent)
- "Delete" (for entries with no operator value)
- "Redesign / not sure" (for entries needing fresh design conversation)

For "added beyond ADR 009" entries, default the recommendation more carefully — these are the operator's chance to roll back execution-time choices.

- [ ] **Step 4.3: Append a Decisions subsection to Unit 3 in AUDIT-PLAN.md**

After the catalog, add:

```markdown
#### Decisions

| B-entry | Decision | Rationale |
|---------|----------|-----------|
| B3.1 — <name> | keep / delete / redesign | <one-line operator rationale> |
| B3.2 — <name> | keep / delete / redesign | <one-line operator rationale> |
...
```

- [ ] **Step 4.4: Findings backlog entries**

If the user surfaces concerns that are out-of-scope for Unit 3 (e.g., a UX wish that affects pages not in ADR 009's scope, or a related cleanup), record in AUDIT-PLAN.md's "Findings backlog" section with a suggested branch name. **Do not fix on this branch.**

---

## Task 5: Surgical pruning (AUDIT-PLAN.md Step 5)

**Files:**
- Modify: refbox source files containing deleted behaviours
- Modify: translation files (if a deleted behaviour's strings are no longer reachable)
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Pruning record subsection)

- [ ] **Step 5.1: For each `@deleted` entry, remove the code that implements it**

One commit per deletion (or one commit per clean cluster — e.g., if three entries delete together because they share a code block, group them). Commits use conventional types:

- `refactor(refbox): remove <behaviour> (per Unit 3 audit B3.N)` for clean removals
- `fix(refbox): <fix-description> (per Unit 3 audit B3.N)` if removal also fixes a bug surfaced during audit
- `chore(refbox): drop unused <thing> (per Unit 3 audit B3.N)` for translation keys or dead helpers

Each commit message body should reference the B-entry and the operator's rationale from the Decisions table.

- [ ] **Step 5.2: Verify the audit branch still compiles after each prune commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && cargo check -p refbox 2>&1 | tail -3
```

Expected: clean compile after each prune commit. If compilation breaks, the deletion was incomplete — restore and try again with a wider scope.

- [ ] **Step 5.3: Record the pruning record in AUDIT-PLAN.md**

Append a Pruning record subsection to Unit 3:

```markdown
#### Pruning record

| Commit | B-entry | What was removed | Why |
|--------|---------|------------------|-----|
| `<hash>` | B3.N | <one-line summary> | <operator rationale> |
```

If no behaviours were deleted, write `*(no deletions — all catalog entries `@user_verified`)*` and continue.

---

## Task 6: Test pass on what remains (AUDIT-PLAN.md Step 6) — mixed weight

**Files:**
- Create / Modify: regression tests for state-machinery invariants — typically in `refbox/src/app/mod.rs` or new files under `refbox/src/app/` with `#[cfg(test)]` modules; or in `refbox/tests/`
- Create: `refbox/tests/features/adr_009_settings.feature` (Gherkin scenarios for manual-walkthrough coverage)
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Test status subsection per Unit 1 refinement #9)

Verification weight is **heavy for state-machinery, lean for chrome**. Heavy means: write a regression test that would fail if the invariant breaks. Lean means: launch refbox, operator demonstrates the behaviour, record a manual-walkthrough timestamp.

### Step 6.1: Identify state-machinery invariants to lock with Rust tests

From the design brainstorm, the heavy-process state-machinery surface is:

1. **Per-page snapshot capture-and-revert.** Cancel on a page reverts only that page's slice. Other pages' edits survive across the Cancel.
2. **`uwhportal_incomplete()` Apply-disable rule.** Apply on Game Options is disabled when portal state is incomplete.
3. **Mid-game Apply gates.** `GameConfigChangedFromApply`, `GameNumberChangedFromApply`, `UwhPortalIncompleteFromApply` confirmation variants fire from per-page Apply (not global Done) when game state is mid-game.
4. **Picker-driven field clearing.** Picking a new event clears `current_court`, `game_number`, `schedule`. Picking a new court clears `game_number`.

For each invariant, only add a regression test **if the underlying behaviour was kept** (i.e., the relevant catalog entry is `@user_verified`). If a state-machinery behaviour was `@deleted`, skip the test for it (the prune commit removed the code).

- [ ] **Step 6.1a: Locate existing test files for `refbox/src/app/`**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && grep -rl '#\[cfg(test)\]' refbox/src/app/ | head -10
```

Existing `#[cfg(test)]` modules tell us where to add new tests. If none exist near the code under test, add a new `#[cfg(test)] mod tests { ... }` block at the end of the relevant source file.

- [ ] **Step 6.1b: Write the per-page snapshot/revert regression test (if B3.X for snapshot is `@user_verified`)**

Locate `PageEntrySnapshot` in `refbox/src/app/mod.rs` (line discoverable via `grep -n 'PageEntrySnapshot' refbox/src/app/mod.rs`). The test exercises `capture_snapshot_for(page)`, modifies the in-memory `edited_settings`, calls `revert_from_snapshot()`, and asserts only the targeted page's fields reverted while other pages' edits survived.

Test name: `test_cancel_reverts_only_page_slice` or similar.

Run pattern:
```bash
cargo test -p refbox --lib test_cancel_reverts_only_page_slice -- --nocapture
```

Expected: PASS after writing the test against current code (the behaviour shipped; the test locks it).

- [ ] **Step 6.1c: Write the `uwhportal_incomplete()` Apply-disable test (if kept)**

Construct an `EditableSettings` instance with `using_uwhportal=true` but `current_event_id=None`. Assert `editable_settings.uwhportal_incomplete()` returns `true`. Construct one with `using_uwhportal=false`. Assert `uwhportal_incomplete()` returns `false` (per Task 8 deviation #5).

- [ ] **Step 6.1d: Write the mid-game Apply gate tests (if kept)**

For each of `GameConfigChangedFromApply`, `GameNumberChangedFromApply`, `UwhPortalIncompleteFromApply`: construct a `RefBoxApp` in a mid-game state, simulate `Message::ApplyConfigPage(Game)` with the relevant precondition (changed config / changed game number / incomplete portal). Assert the confirmation variant is dispatched (e.g., via the `confirmation_dialog` field on `RefBoxApp`).

This is the most state-heavy of the tests; if direct `RefBoxApp` construction is impractical, use an `apply_game_options()` unit test that returns the post-apply action enum and assert on that.

- [ ] **Step 6.1e: Write the picker-driven field-clearing test (if kept)**

Test `Message::ParameterSelected` with a new event ID; assert `current_court`, `game_number`, `schedule` are cleared on the resulting `edited_settings`. Test with a new court ID; assert `game_number` is cleared but `current_event_id` survives.

- [ ] **Step 6.1f: Run all new tests and `just check`**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && cargo test -p refbox --lib 2>&1 | tail -20 && just check 2>&1 | tail -10
```

Expected: all tests pass; clippy clean; format clean; audit clean.

- [ ] **Step 6.1g: Commit the regression tests**

One commit, suggested message:
```
test(refbox): lock per-page Apply state invariants (Unit 3 audit)

Adds regression tests for the state-machinery behaviours kept in
Unit 3's catalog:
- Per-page snapshot capture/revert
- uwhportal_incomplete() Apply-disable
- Mid-game Apply gate confirmation variants
- Picker-driven field clearing on event/court change

Refs: AUDIT-PLAN.md Unit 3, ADR 009.
```

### Step 6.2: Write Gherkin scenarios for manual-walkthrough coverage

**Files:**
- Create: `refbox/tests/features/adr_009_settings.feature`

Per Unit 1 refinement #2, `.feature` files live at `refbox/tests/features/`. The file uses the Gherkin Scenario format from AUDIT-PLAN.md's "Scenario format" section, with workflow tags per AUDIT-PLAN.md's "Workflow tags" section.

- [ ] **Step 6.2a: Create the feature file with a header**

```gherkin
@unit-3 @adr-009
Feature: Settings navigation and per-page save model

  Background: The operator opens the refbox application
    Given the refbox is launched
    And the operator is on the main game screen
```

- [ ] **Step 6.2b: Add one Scenario per kept B-entry that is operator-observable**

Each scenario uses the `S<unit>.<n>` numbering. Example scenario shape:

```gherkin
  @user_verified @manual_walkthrough_only
  Scenario: S3.N — Main settings page shows the 2×2 grid
    When the operator taps Settings from the main screen
    Then the Main settings page shows four tiles labeled
      | GAME OPTIONS |
      | APP OPTIONS  |
      | USER OPTIONS |
      | LANGUAGE     |
    And a single BACK button is shown at the bottom
```

Workflow tags to apply per scenario:
- `@user_verified` — the operator approved this behaviour in Task 4
- `@tested_pass` — if a Rust regression test from Step 6.1 covers this scenario's invariant
- `@manual_walkthrough_only` — if verification is operator-driven only (chrome and navigation entries)
- `@tested_fail` — if the manual walkthrough surfaced a regression (this triggers a fix decision)

- [ ] **Step 6.2c: Commit the scenario file**

```
test(refbox): add ADR 009 settings Gherkin scenarios (Unit 3 audit)

Captures operator-observable behaviour from Unit 3's catalog as Gherkin
scenarios under refbox/tests/features/adr_009_settings.feature for
manual-walkthrough verification.

Refs: AUDIT-PLAN.md Unit 3.
```

### Step 6.3: Manual reproduction with the user driving

- [ ] **Step 6.3a: Launch refbox in the background**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox 2>&1
```

Use `run_in_background: true` and `dangerouslyDisableSandbox: true`. Per `feedback_user_drives_refbox_ui` (updated 2026-05-12), Claude launches; user drives.

- [ ] **Step 6.3b: Walk every scenario page-by-page**

For each scenario, in operator-workflow order (Main → User Options → ...):

1. State the scenario steps to the user
2. Wait for the user to perform the steps and report what they observed
3. If observed matches expected: append `@tested_pass` to the scenario tags + a one-line timestamped note `# verified 2026-05-13 by operator`
4. If observed diverges: append `@tested_fail` + capture the divergence in a follow-up entry under AUDIT-PLAN.md's Findings backlog if out-of-scope, or fix on the audit branch if in-scope per the audit's discretion (consult user)

- [ ] **Step 6.3c: Commit any per-scenario tag updates**

Updates to the `.feature` file go in commits like:
```
docs(refbox): record manual-walkthrough results for ADR 009 scenarios
```

- [ ] **Step 6.3d: Stop the refbox process**

When all scenarios are walked through, terminate the background process via the user's preferred mechanism.

### Step 6.4: Final validation sweep

- [ ] **Step 6.4a: Run `just check` on the audit branch**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && just check 2>&1 | tail -15
```

Expected: clean fmt, clippy, tests, audit. If anything fails, explain in plain English per `.claude/rules/communication.md` and propose a fix.

- [ ] **Step 6.4b: Record test status summary in AUDIT-PLAN.md**

Under Unit 3, after the Pruning record, append:

```markdown
#### Test status

**State-machinery invariants (Rust tests):**
- B3.X — per-page snapshot/revert: ✅ test added (test name)
- B3.Y — uwhportal_incomplete() Apply-disable: ✅ test added
- B3.Z — mid-game Apply gates: ✅ test added
- B3.W — picker-driven field clearing: ✅ test added

**Operator-observable scenarios (Gherkin + manual walkthrough):**
See `refbox/tests/features/adr_009_settings.feature` — all `@user_verified` scenarios carry `@tested_pass` or `@manual_walkthrough_only` tags.

**`just check`:** clean on the audit branch at HEAD.
```

---

## Task 7: Finalize ADR 009 in place (AUDIT-PLAN.md Step 7)

**Files:**
- Modify: `docs/decisions/009-settings-navigation-layout.md` (amend in place; flip status; fold deviations into Decision section)
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (cross-reference summary appended)

Per Unit 3's design (user-confirmed: "Finalize ADR 009 in place"), ADR 009 is amended rather than superseded.

- [ ] **Step 7.1: Flip status from `proposed` to `accepted`**

In `docs/decisions/009-settings-navigation-layout.md` line 4: change `**Status:** proposed` to `**Status:** accepted`.

- [ ] **Step 7.2: Fold approved deviations into the Decision section**

For each `@user_verified` catalog entry tagged `added beyond ADR 009`, add an explicit bullet to ADR 009's Decision section so future readers see the full intent. Examples likely needed (subject to user-review outcomes in Task 4):

- "Apply on Game Options is disabled when portal state is incomplete (`uwhportal_incomplete()` returns true)." (from Task 8 deviation #5)
- "Picking a new event on Game Options clears `current_court`, `game_number`, and `schedule`. Picking a new court clears `game_number`." (from Task 8 deviation #4)
- "Game Options' `PageEntrySnapshot` covers both `config`, `game_number`, and portal-related App-slice fields (`using_uwhportal`, `current_event_id`, `current_court`, `schedule`)." (from Task 8 deviation #3 / 2026-04-29 side plan)
- "Per-page Apply for Game Options shares confirmation UI with global Done but commits only the Game slice via `apply_game_confirmation`. New confirmation variants: `GameConfigChangedFromApply`, `GameNumberChangedFromApply`, `UwhPortalIncompleteFromApply`." (from Task 8 deviation #3)

For each `@deleted` catalog entry, **do not** add it to ADR 009. The prune commits already removed it from the code; ADR 009 should reflect what's shipped.

- [ ] **Step 7.3: Add a "Verified by Unit 3 audit" subsection**

At the end of ADR 009, before the References section, add:

```markdown
## Verified by Unit 3 audit

This ADR's design was implemented in the 10 commits ending at `ce6cfeb` on `refactor/refbox/settings-navigation` and audited under Unit 3 of the AI Code Audit (`AUDIT-PLAN.md`). The catalog of operator-observable behaviours, the keep/delete decisions, and the manual-walkthrough record are in AUDIT-PLAN.md's Unit 3 section. Status flipped from `proposed` to `accepted` on 2026-05-13 once all catalog entries were `@user_verified` and the manual walkthrough completed.

### What was not verified

*(Append entries here for any kept behaviour where the audit could not exercise the full code path — e.g., real-portal-data flows that require live portal connectivity. Empty if everything was exercised.)*
```

- [ ] **Step 7.4: Update ADR 009 References section if needed**

If new file paths are referenced in the folded-in deviations (e.g., new helper methods, new confirmation variants), append them to the References section using the same format as existing references.

- [ ] **Step 7.5: Commit the ADR finalization**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git add docs/decisions/009-settings-navigation-layout.md
```

Commit message (use HEREDOC for formatting):
```
docs(refbox): finalize ADR 009 (settings navigation) — Unit 3 audit

Flip ADR 009 status from proposed to accepted. Fold execution-time
deviations (approved via Unit 3 audit) into the Decision section so
the ADR reflects shipped reality. Add Verified-by-Unit-3-audit section
with the audit's cross-reference and "What was not verified" header.

Refs: AUDIT-PLAN.md Unit 3.
```

**Approval gate per `.claude/rules/communication.md`:** confirm with the user before this commit. Show the diff of `docs/decisions/009-settings-navigation-layout.md` so they can read the amended ADR top-to-bottom.

---

## Task 8: Hold branch locally (AUDIT-PLAN.md Step 8)

**Files:** none

- [ ] **Step 8.1: Confirm the audit branch is local-only**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git status -sb && git log origin/master..HEAD --oneline | wc -l
```

Expected: `## audit/refbox/adr-009-settings` (no `[ahead/behind]` upstream tracking line); commit count from the 10 starting commits plus any audit-side commits (regression tests, scenario file, prune commits if any, ADR finalization).

- [ ] **Step 8.2: Do not push**

No `git push`. No PR. The branch is held locally until the Final Integration phase (per AUDIT-PLAN.md "Final integration: PR phase" section).

- [ ] **Step 8.3: Verify the in-flight branch was not disturbed**

```bash
cd /home/estraily/projects/refbox-settings-worktree && git log --oneline -1
```

Expected: still at `ce6cfeb` (the in-flight worktree's branch is untouched; the audit branch was cut from there but the in-flight branch itself remains at the same point).

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Step 9)

**Files:**
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (status flip + summary pointer per playbook-amended Step 9.4)

- [ ] **Step 9.1: Flip Unit 3's status in the catalog table**

In AUDIT-PLAN.md, update the catalog table at the top: change Unit 3's `Status` cell from `not started` to `complete-pending-integration (2026-05-13)`.

- [ ] **Step 9.2: Flip Unit 3's status in the per-unit details section**

In the `### Unit 3 — ADR 009 in-flight work` section near line 990, update the `**Status:**` line to:
```markdown
**Status:** complete-pending-integration (2026-05-13)
```

- [ ] **Step 9.3: Add a summary entry to "Completed audits"**

Per Unit 1 refinement #3 (in-place flip + summary pointer; not a destructive section move), add a new entry to "Completed audits" in AUDIT-PLAN.md following the Unit 1 and Unit 2 templates. Include:

- Branch name and "local only; N commits ahead of origin/master; not pushed"
- Per-unit plan path (this file)
- ADR(s) updated: `docs/decisions/009-settings-navigation-layout.md` (amended in place; flipped from `proposed` to `accepted`)
- Scenarios file: `refbox/tests/features/adr_009_settings.feature`
- Catalog outcome: N entries, all `@user_verified` (or list deletions if any)
- Tests added during audit: count + brief description
- Audit commits on branch: hashes from Tasks 5, 6, 7
- What was not verified: anything from the ADR's "What was not verified" subsection
- Full details section: retained in "Unit-by-unit details" above with status flipped (per Unit 1 refinement #3)

- [ ] **Step 9.4: Add any process refinements learned during Unit 3 to the log**

In AUDIT-PLAN.md's "Process refinements log" section, add a `#### From Unit 3 (2026-05-13)` block if Unit 3 surfaced playbook gaps or new patterns. Likely candidates (subject to actual execution experience):

- How auditing an in-flight ADR (with a pre-existing proposed status and a deviations log) differs from auditing post-merge work
- Whether the mixed-weight verification pattern was right-sized
- Any new slop-catching patterns surfaced by the chrome rollout commits

- [ ] **Step 9.5: Final sanity check**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-3-adr-009-settings && git status -sb && cd /home/estraily/projects/uwh-refbox-rs && grep -n 'Unit 3' AUDIT-PLAN.md | head -10
```

Expected: audit branch clean; AUDIT-PLAN.md shows Unit 3 marked complete-pending-integration in both the catalog table and the per-unit details section.

---

## Out-of-scope guardrails

Throughout the unit, the following are **out of scope** and must not be touched on the audit branch. They are recorded in the Findings backlog instead:

- **ADR 017 — Portal data lifecycle.** "Unknown vs Unknown" placeholder and wasteful startup event-list fetch when `using_uwhportal=false`. Has its own ADR (`proposed`, awaiting design input). Cross-reference in ADR 009 stays; no implementation change here.
- **ADR 018 — Event picker sort order.** Has its own ADR (`proposed`). Same disposition.
- **ADR 014 — Live preview.** Explicitly deferred in ADR 009. Sound, brightness, and starting-sides live-preview behaviour stays out of scope.
- **ADR 013 — Cold-restart state recovery.** Settings-done trigger path stays untouched per the original plan's out-of-scope guardrail.
- **`uwh-common`, `overlay`, `schedule-processor`, `wireless-remote`, `wireless-modes`.** No changes. ADR 009 itself constrains scope to `refbox`; the audit honors that.
- **Translations beyond keys added/used by the 10 commits.** Unit 3 audits what was shipped, not a translation-coverage sweep.
- **Refactoring nearby code "while we're in there."** Per `.claude/rules/scope.md` — no opportunistic refactoring.

If any of these are touched by an in-flight prune commit or test (e.g., a snapshot test happens to reach into `uwh-common`), stop and consult the user before proceeding.
