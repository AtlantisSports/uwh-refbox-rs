# Audit Unit 4 — Manual Alarm Button: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit the 16 commits on master that ship the manual alarm button feature (`bc66e1e^..ff6018b`), confirm the shipped behaviour matches the authoritative spec at `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md`, flag any divergences for explicit operator decision, exercise what survives, and produce a fresh retroactive ADR 021 for the feature in its current form.

**Architecture:** Diff-led catalog grouped by spec section (Settings · Behaviour-by-state · Messages · State · View layout · Button appearance · Translations · Cross-cutting). Spec is the oracle: every B-entry is marked matches-spec / diverges-from-spec / not-in-spec. Operator-observable grain. Three test sessions cover the surface area without re-navigation; the companion delta plan's 11-item checklist is a backstop. Fresh ADR 021 (not an amendment) since ADR 006 describes a future successor, not what shipped. Page-batched review for Step 4 if entry count exceeds ~25.

**Tech Stack:** Rust 2024 / MSRV 1.85; iced 0.13 (refbox UI); `cargo test`, `cargo clippy`, `just check`; manual refbox launch (`WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox` with `dangerouslyDisableSandbox:true`); Gherkin scenarios in `refbox/tests/features/manual_alarm.feature` (pre-existing, will be aligned during Step 6).

---

## Acceptance criteria (Unit 4 "complete-pending-integration")

Unit 4 is complete-pending-integration when **all** of these hold on the audit branch `audit/refbox/manual-alarm-button`:

1. A behaviour catalog exists in `AUDIT-PLAN.md` under Unit 4, grouped by spec section, with every entry tagged `@user_verified` or `@deleted`. No `@proposed` remaining.
2. Every `@user_verified` operator-observable behaviour is captured as a Gherkin scenario in `refbox/tests/features/manual_alarm.feature` (aligned from the pre-existing file rather than parallel-created), tagged `@user_verified` and a test-state tag (`@tested_pass`, `@tested_fail`, or `@tested_inconclusive`).
3. Every `@user_verified` backend behaviour has its test status captured in the retroactive ADR's prose (since backend has no `.feature` row).
4. The operator has driven the refbox UI through Sessions 1, 2, and 3 (Sound Options page; main game screen across states; spacebar parity and inactive screens). Each session's scenarios carry a manual-walkthrough timestamp in the .feature file.
5. The companion delta plan's 11-item Manual Verification Checklist (in `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md` Task 3 Step 2) has been cross-checked against the sessions; gaps go to AUDIT-PLAN.md's Findings backlog.
6. A retroactive ADR exists at `docs/decisions/021-manual-alarm-button.md` (numbered against the expected post-merge state per Unit 1 refinement #8). The Decision section embeds `@user_verified @tested_pass` scenarios verbatim with one sentence of plain-English framing per scenario. ADR 006 is listed in References as the proposed successor.
7. `just check` passes on the audit branch (`fmt-check`, `clippy -D warnings`, all tests pass, `cargo audit` clean — except for the two pre-existing dependency vulnerabilities noted in Findings backlog #4).
8. The branch holds locally (no push, no PR) per playbook Step 8.
9. AUDIT-PLAN.md status flipped from "not started" to "complete-pending-integration"; summary pointer added to "Completed audits" section per playbook-amended Step 9.4.
10. Findings discovered out-of-scope are recorded in AUDIT-PLAN.md's Findings backlog with a suggested follow-up branch name. They are **not fixed** on this branch.

---

## Prerequisites

- Read `AUDIT-PLAN.md` Unit 4 section and the Process refinements log entries from Units 1, 2, and 3.
- Read `docs/superpowers/specs/2026-05-13-audit-unit-4-manual-alarm-design.md` (this unit's audit-design spec; the spec it audits *against* is the 2026-04-14 spec).
- Read `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md` (canonical spec — audit's oracle).
- Read `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md` (companion delta plan with the 11-item manual verification checklist in Task 3).
- Skim `docs/superpowers/plans/2026-04-14-manual-alarm-button.md` (815-line original plan, retconned mid-flight; reference only).
- Read `docs/decisions/006-multi-remote-alarm-buttons.md` (proposed successor ADR — out of scope, referenced in retroactive ADR only).
- Read `.claude/rules/scope.md`, `communication.md`, `workspace.md`, `rust.md`, `plan-execution.md`, `pr-review.md`, `embedded.md` (last is informational; Unit 4 does not touch wireless-remote).
- Confirm the local working tree is clean: `cd /home/estraily/projects/uwh-refbox-rs && git status --short`. Resolve any uncommitted changes before proceeding.

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:**
- Create: `.worktrees/audit-unit-4-manual-alarm-button/` (new worktree)
- Modify: `<main-repo>/.git/hooks/pre-commit` (only if hook is not already audit-aware from Unit 1)

- [ ] **Step 1.1: Cut the audit worktree from master**

The feature is fully merged on master, so the audit branch cuts from `origin/master`'s tip.

```bash
cd /home/estraily/projects/uwh-refbox-rs && git fetch origin && git worktree add -b audit/refbox/manual-alarm-button .worktrees/audit-unit-4-manual-alarm-button origin/master
```

Expected output: `Preparing worktree (new branch 'audit/refbox/manual-alarm-button')` then `HEAD is now at <master-tip-hash>`.

- [ ] **Step 1.2: Verify the audit branch starts at master tip and contains all 16 manual-alarm commits**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git rev-parse --abbrev-ref HEAD && git log --oneline ff6018b -1 && git log --oneline bc66e1e -1
```

Expected: branch name `audit/refbox/manual-alarm-button`; both commits reachable.

- [ ] **Step 1.3: Confirm pre-commit hook is audit-aware**

The hook was updated in Unit 1 (commit `2a8dcbc` on `audit/refbox/confirm-score-timing`) to allow the `audit` branch type. Until that lands on master, every fresh worktree's pre-commit hook (shared via `<main-repo>/.git/hooks/pre-commit`) must allow `audit`.

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
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && cargo check -p refbox 2>&1 | tail -5
```

Expected: clean compile. If this fails, master is broken — stop and check with user.

- [ ] **Step 1.5: Flip Unit 4's status to "in progress"**

In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
- Update the unit-catalog table row for Unit 4: `not started` → `in progress (started 2026-MM-DD)`
- Update the `### Unit 4 — Manual alarm button` section's `**Status:**` line the same way

This file is gitignored; no commit follows.

---

## Task 2: Generate the diff (AUDIT-PLAN.md Step 2)

**Files:**
- Create: `.audit/unit-4-commits-summary.txt` (not committed; local working artifact)
- Create: `.audit/unit-4-stat.txt` (not committed)
- Create: `.audit/unit-4-messages.txt` (not committed)
- Create: `.audit/unit-4-diff.txt` (not committed)

- [ ] **Step 2.1: List the 16 commits with full subject lines**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && mkdir -p .audit && git log bc66e1e^..ff6018b --reverse --oneline > .audit/unit-4-commits-summary.txt && cat .audit/unit-4-commits-summary.txt
```

Expected output (oldest first):

```
bc66e1e feat(refbox): add manual_alarm_enabled field to SoundSettings
38799da test(refbox): assert manual_alarm_enabled in migration test
b3fde3e feat(refbox): add AlarmPressed/Released/Fired messages and ManualAlarmEnabled parameter
40a857a feat(refbox): add translation strings for manual alarm button
5685a29 feat(refbox): add Alarm Button toggle to sound settings page
94b1c64 feat(refbox): handle alarm messages and ManualAlarmEnabled toggle in app update
8db9387 refactor(refbox): tighten alarm hold generation and add logging
36cde0e feat(refbox): subscribe to spacebar events for manual alarm when enabled
2a4294e feat(refbox): add alarm button layout to main view
babe3ca fix(refbox): use matches! for alarm_available to satisfy clippy
bc620ff feat(refbox): pass manual_alarm_enabled to build_main_view
d5f485a feat(refbox): add manual alarm button to main view
9cef2c4 docs(refbox): revise manual alarm scenarios, spec, and plan to uniform hold model
7f173ee feat(refbox): unify manual alarm press handlers under uniform hold model
c90348b feat(refbox): switch alarm button colour on active-play state, not BetweenGames
ff6018b fix(refbox): tune manual alarm active-play hold to 150ms
```

If the count is not 16, stop and reconcile with the user — the audit-design spec asserted 16 based on `git rev-list bc66e1e^..ff6018b --count`.

- [ ] **Step 2.2: Generate the full diff, stat, and per-commit messages**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git log bc66e1e^..ff6018b --reverse --stat > .audit/unit-4-stat.txt && git log bc66e1e^..ff6018b --reverse --format='COMMIT %h %s%n%n%b%n----' > .audit/unit-4-messages.txt && git diff bc66e1e^..ff6018b > .audit/unit-4-diff.txt && wc -l .audit/unit-4-*.txt
```

Expected: stat and messages files populated; diff is the union of 16 commits' changes.

- [ ] **Step 2.3: Confirm the diff scope is `refbox` only**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git diff --name-only bc66e1e^..ff6018b | awk -F'/' '{print $1}' | sort -u
```

Expected: only `refbox` and possibly `docs` (the doc-revision commit `9cef2c4` touches `docs/superpowers/...`). If anything else appears, flag to user before proceeding — Unit 4's scope is `refbox` only per `.claude/rules/scope.md`.

- [ ] **Step 2.4: Pull the spec, companion plan, and ADR 006 into the audit workspace for cross-reference**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && cp docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md .audit/oracle-spec.md && cp docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md .audit/companion-plan.md && cp docs/decisions/006-multi-remote-alarm-buttons.md .audit/adr-006.md && ls -la .audit/
```

Expected: three reference files copied. These are working artifacts the catalog-build step (Task 3) reads from without traversing the source tree repeatedly.

- [ ] **Step 2.5: Record the files-touched list under Unit 4's section in AUDIT-PLAN.md**

In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`, under the `### Unit 4 — Manual alarm button` section, add a `#### Files touched` subsection with the output of:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git diff --name-only bc66e1e^..ff6018b | sort
```

Group the output as: settings, messages, app, view-builders, translations, scenarios, docs. Add a one-line summary of why each file is in scope. This file is gitignored; no commit.

---

## Task 3: Build the behaviour catalog (AUDIT-PLAN.md Step 3)

**Files:**
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Unit 4 section near line 1571; gitignored)

Catalog entries follow the template in AUDIT-PLAN.md's "Templates" section. **Build order is chronological by commit** (so we don't miss anything in the diff). **Presentation order in the catalog file is spec-section-grouped** (so the user-review session in Task 4 flows naturally against the spec's structure).

Catalog grain: **operator-observable behaviour**. Each entry covers one fact the operator can see, feel, or trigger. Backend behaviours (migration default, BoolGameParameter wiring, helper-method extraction, token cancellation logic) appear under "Cross-cutting (backend)".

Each catalog entry MUST include:
- `What it does (plain English):` — what the operator observes (or, for backend, what the system internally does)
- `Where in the diff:` — file:line refs
- `Why it might be intentional:` — quote the spec line if it covers the behaviour; otherwise guess at intent
- `Why it might be slop:` — slop-catching-checklist hits, divergence-from-spec, or "not in spec at all"
- `Spec status:` — `matches-spec` | `diverges-from-spec` | `not-in-spec`
- `Linked scenario(s):` — S4.N for operator-observable; `none` for backend
- `Recommendation:` — keep | delete | clarify-with-operator — one line
- `Decision (Step 4):` — `@proposed` until Task 4 sets `@user_verified` or `@deleted`

- [ ] **Step 3.1: Open the catalog section and add the spec-section groupings**

In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`, locate the Unit 4 section (currently near line 1571). Under the "Behaviour catalog · Scenarios · Decisions · Pruning record · Retroactive ADR" subheading, replace the `*(filled during execution)*` placeholder with the spec-section scaffold:

```markdown
#### Files touched

*(filled in Task 2.5)*

#### Behaviour catalog

##### Spec section: Settings Integration
*(entries appended here)*

##### Spec section: Alarm Behavior by Game State
*(entries appended here)*

##### Spec section: Main Screen Layout
*(entries appended here)*

##### Spec section: Alarm button appearance
*(entries appended here)*

##### Implementation: Messages and BoolGameParameter
*(entries appended here)*

##### Implementation: App state and handlers
*(entries appended here)*

##### Implementation: Keyboard subscription
*(entries appended here)*

##### Implementation: Hold-duration helper
*(entries appended here)*

##### Translations
*(entries appended here)*

##### Cross-cutting (backend, refactors, doc-revision, slop)
*(entries appended here)*

#### Scenarios

*(entries appended here in Task 3.4)*

#### Decisions

*(filled in Task 4)*

#### Pruning record

*(filled in Task 5)*

#### Retroactive ADR

*(filled in Task 7)*
```

- [ ] **Step 3.2: Walk the 16 commits chronologically and append entries**

For each commit (oldest first: `bc66e1e` → ... → `ff6018b`):

1. Read `.audit/unit-4-messages.txt` for that commit's message body — body's `Changes:` list (if present) is the most reliable source of operator-observable behaviour facts.
2. Inspect the diff for that commit: `git show <hash> --stat` then `git show <hash> -- <suspect-file>` for each file the commit changed.
3. For each operator-observable fact, write a B-entry under the matching spec-section group. Quote the relevant spec line in the "Why it might be intentional" field.
4. For each fact that is purely internal (refactor with no operator-visible behaviour change — e.g., the `8db9387` "tighten alarm hold generation" refactor, the `babe3ca` `matches!` clippy fix), put it under "Cross-cutting (backend, refactors, doc-revision, slop)" with the appropriate slop-check note.
5. Mark `Spec status` for every entry: matches-spec, diverges-from-spec, or not-in-spec.

**Entry template:**

```markdown
##### B4.N — <short name>

- **What it does (plain English):** <one to two sentences; operator-observable for UI entries, system-observable for backend>
- **Where in the diff:** `<file>:<line>` (commit `<hash>`)
- **Why it might be intentional:** <spec line ref + quote if covered; otherwise "spec is silent on this">
- **Why it might be slop:** <slop-checklist hit if any; "spec says X but code does Y" for divergences; "not in spec" otherwise>
- **Spec status:** `matches-spec` | `diverges-from-spec` | `not-in-spec`
- **Linked scenario(s):** S4.N (UI) or `none` (backend)
- **Recommendation:** keep | delete | clarify-with-operator — <one-line reason>
- **Decision (Step 4):** [ ] @proposed [ ] @user_verified [ ] @deleted
- **Notes from review:** *(filled in Task 4)*
```

- [ ] **Step 3.3: Slop sweep against the catalog**

After the catalog scaffold is filled, run the slop-catching checklist from AUDIT-PLAN.md. For Unit 4 specifically:

- [ ] Are there `unwrap()` / `expect()` calls added in the 16 commits without justifying comments? `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git diff bc66e1e^..ff6018b -- '*.rs' | grep -E '^\+.*\.(unwrap|expect)\('` then audit each. Per Unit 3 refinement #5: distinguish "panic indicates programming bug" (acceptable with comment) from "panic indicates unhandled runtime condition" (flag as a Findings-backlog candidate).
- [ ] Are there bundled fixes inside feature commits (Unit 2 refinement #2)? Check each `feat(refbox)` commit message body — does the `Changes:` list span more than one concern? If yes, list each concern as a distinct B-entry and flag the bundling for the Process refinements log regardless of keep/delete outcome.
- [ ] Are there `#[allow(...)]` attributes added? Capture each as a B-entry under cross-cutting.
- [ ] Did any new dependency get added? `git diff bc66e1e^..ff6018b -- '**/Cargo.toml'`. If yes, flag.
- [ ] Hold-duration constant audit: spec line 72 names 150ms; companion plan names 250ms in three places; the final tuning commit `ff6018b` lands at 150ms. Build one B-entry recording the chain and the final landing point.
- [ ] State-naming audit: spec line 117 names `alarm_hold_generation`; code uses `alarm_delay_token`. B-entry to confirm code naming wins.
- [ ] Independent-flag design audit: spec describes a single generation counter; code uses two independent flags (`mouse_alarm_held`, `spacebar_held`) plus a token counter. B-entry to confirm the added complexity is intentional (handles simultaneous-press case).
- [ ] Helper-extraction audit: companion plan Task 1 Step 4 flagged `manual_alarm_hold_duration()` extraction as "Optional but recommended". Confirm whether it was extracted in `7f173ee` or later. B-entry under "Implementation: Hold-duration helper".
- [ ] `disabled_container` import audit (companion plan Task 2 Step 2): `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && grep -n disabled_container refbox/src/app/view_builders/main_view.rs`. Expected: no matches. If matches exist, the obsolete import is dead weight — B-entry under "Cross-cutting".
- [ ] `mouse_area` always-wrapped audit (companion plan Task 2 Step 1): confirm `mouse_area` wraps the alarm face unconditionally in `main_view.rs`. If it's still conditional, that's an operator-observable regression risk — B-entry under "Main Screen Layout".
- [ ] Translation-key audit: list the 5+ new keys (`alarm-button`, `alarm`, `or-press-spacebar`, `hold-to-test`, `or-hold-spacebar`, `game-info`) and confirm each is reachable from a kept UI element. For each key, list the languages it appears in: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && for k in alarm-button alarm or-press-spacebar hold-to-test or-hold-spacebar game-info; do echo "--- $k ---"; grep -l "^$k " refbox/translations/*/*.ftl 2>/dev/null | wc -l; done`. Expected: each key in roughly the same number of locale files (~13).
- [ ] Spacebar gating audit (spec line 75): confirm spacebar is gated at subscription time and/or handler time and does not fire `AlarmPressed` on non-main screens. Test plan covers it in Session 3.
- [ ] Sound-while-held continuous-mode audit (spec line 127): confirm the dispatch path here matches the regular automatic-buzzer continuous path; if it's a separate code path, the slop-checklist flags re-implementation.

- [ ] **Step 3.4: Draft Gherkin scenarios for operator-observable entries**

Under the `#### Scenarios` subsection of Unit 4 in AUDIT-PLAN.md, draft one scenario per operator-observable B-entry. Use the scenario entry template from AUDIT-PLAN.md's "Scenario entry template" section.

Expected scenario count: 10–18. Anticipated scenarios (sketch — actual phrasing emerges from the catalog):

1. S4.1 — Active play, First Half clock running, no timeout, mouse hold past 150ms fires
2. S4.2 — Active play, First Half clock running, no timeout, mouse tap under 150ms does not fire
3. S4.3 — Active play, mouse parity for Second Half / Overtime halves / Sudden Death
4. S4.4 — Active play, spacebar parity with mouse
5. S4.5 — Active play with timeout active, 1-second hold required
6. S4.6 — Break period (Between Games), 1-second hold required
7. S4.7 — Break period (Half Time, Pre-OT, OT Half Time, Pre-Sudden Death), 1-second hold required
8. S4.8 — Settings toggle: defaults to Off; on enables layout switch
9. S4.9 — Settings toggle: greyed when Sound Enabled is Off
10. S4.10 — Layout: fouls-on splits vertically (alarm left, warnings panel right)
11. S4.11 — Layout: fouls-off is full-width
12. S4.12 — Button appearance: red + "Alarm / Or press Spacebar" in active-play-no-timeout
13. S4.13 — Button appearance: blue + "Hold to Test / Or hold Spacebar" elsewhere
14. S4.14 — Button appearance: pressed-state container visible while held
15. S4.15 — Release behaviour: currently-playing tone finishes natural cycle; no further tones queued
16. S4.16 — Spacebar has no effect on screens other than the main game screen

Cross-link each scenario to its B-entry via `Linked catalog entry: B4.N`. Apply the playbook's concrete-phrasing rule: every scenario names the actual game-state period (e.g., "First Half clock running, no timeout active"), the actual button label, and the actual hold-duration threshold.

- [ ] **Step 3.5: Verify entry count is reasonable**

Expected: **25–40 catalog entries** based on the audit-design spec's Expected Catalog Size section. If below 20 or above 50, re-check granularity. Single commits shipping multiple operator-observable facts (e.g., `7f173ee` unifying both `AlarmPressed` and `SpacebarPressed` to uniform-hold + extracting the helper) should produce multiple entries — one per fact.

- [ ] **Step 3.6: Verify the working tree is still clean**

The catalog lives in the gitignored AUDIT-PLAN.md — no commit on the audit branch. Confirm:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git status --short
```

Expected: nothing tracked changed.

---

## Task 4: User review session (AUDIT-PLAN.md Step 4)

**Files:**
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Unit 4 catalog: flip `@proposed` to `@user_verified` or `@deleted`; populate Decisions subsection)

Walk through the catalog. **If the catalog exceeds ~25 entries, use page-batched review** per Unit 3 Process refinement #3 (one approval question per spec-section group, plus carve-outs for entries flagged ambiguous during the catalog draft). **If ≤25 entries, walk one entry at a time** per the strict playbook default.

For each entry (or each batch), follow the playbook's Step 4 protocol:

1. State the plain-English summary of the behaviour (read the scenario aloud for UI entries)
2. State Claude's keep/delete recommendation with reason, citing the Spec status (matches / diverges / not-in-spec)
3. For divergence entries, present the three resolutions: code wins (update spec), spec wins (fix code), or both tolerable (note as-is)
4. Ask the user — one question at a time, never bundled (per `feedback_one_question_at_a_time`)
5. Record the user's decision in the catalog: `Decision: @user_verified` or `Decision: @deleted` or `Decision: @redesign-followup`
6. For deletions and redesigns, capture the user's rationale in a one-line note appended to the entry's "Notes from review" field

- [ ] **Step 4.1: Confirm review pattern with the operator**

Before launching the session, ask:

> "I'll walk the catalog grouped by spec section: Settings → Behaviour-by-state → Main Screen Layout → Button appearance → Messages → App state → Subscription → Helper → Translations → Cross-cutting. The catalog has <N> entries, so I'll use <one-question-per-entry|page-batched> review. Carve-outs for any ambiguous entries get standalone questions. Ready to start, or do you want to reorder or change the pattern?"

- [ ] **Step 4.2: Walk the catalog**

For each entry or batch, use the `AskUserQuestion` tool with three options:

- "Keep as-is" (recommended for entries matching spec or where divergence is harmless)
- "Delete" (for entries with no operator value, or where spec wins and the code should be removed)
- "Discuss further" (for entries needing fresh design conversation — these become `@redesign-followup` and either resolve at end-of-session or land in Findings backlog)

For divergence entries, recommend explicitly: "code wins" if the divergence is harmless or the code is correct; "spec wins" if the code is incorrect. Spec is ground truth per the audit-design policy, but the operator has final say.

- [ ] **Step 4.3: Resolve any `@redesign-followup` entries at end-of-session**

Per the playbook ("If user is uncertain, leave the entry as @proposed and continue. Revisit all remaining @proposed entries together at the end of Step 4"). Sometimes seeing the whole picture clarifies a single item.

- [ ] **Step 4.4: Populate the Decisions subsection in AUDIT-PLAN.md**

Under the `#### Decisions` subsection of Unit 4, write one line per B-entry following the playbook's decision-log template:

```markdown
- B4.1 <short name> — **@user_verified** — <one-line operator rationale>
- B4.2 <short name> — **@deleted** — <one-line operator rationale>
...
```

- [ ] **Step 4.5: Findings backlog entries**

If the user surfaces concerns that are out-of-scope for Unit 4 (e.g., a UX wish that affects pages not in the spec's scope, or a related cleanup, or any input to ADR 006), record in AUDIT-PLAN.md's "Findings backlog" section with a suggested branch name. **Do not fix on this branch.** ADR 006 inputs specifically: collect them as a single Findings-backlog entry titled "ADR 006 inputs surfaced during Unit 4 audit" so the post-audit ADR-006 pass can find them in one place.

---

## Task 5: Surgical pruning (AUDIT-PLAN.md Step 5)

> **Skip Task 5 entirely if every catalog entry is `@user_verified`.** Document the skip with a one-line note: "No `@deleted` entries; Task 5 skipped." If skipped, proceed directly to Task 6.

**Files:**
- Modify: refbox source files containing deleted behaviours
- Modify: `refbox/translations/*.ftl` (if a deleted behaviour's strings are no longer reachable)
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Pruning record subsection)

- [ ] **Step 5.1: For each `@deleted` entry, remove the code that implements it**

One commit per deletion (or one commit per clean cluster — e.g., if three entries delete together because they share a code block, group them). Commits use conventional types:

- `refactor(refbox): remove <behaviour> (per Unit 4 audit B4.N)` for clean removals
- `fix(refbox): <fix-description> (per Unit 4 audit B4.N)` if removal also fixes a bug surfaced during audit
- `chore(refbox): drop unused <thing> (per Unit 4 audit B4.N)` for translation keys or dead helpers

Each commit message body should reference the B-entry and the operator's rationale from the Decisions table.

- [ ] **Step 5.2: Verify the audit branch still compiles after each prune commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && cargo check -p refbox 2>&1 | tail -3
```

Expected: clean compile. If a prune breaks the build, an `@user_verified` behaviour depends on the `@deleted` one — flip the `@user_verified` back to `@proposed` and discuss with the user per the playbook ("Pruning breaks something the user wanted to keep").

- [ ] **Step 5.3: Run `just fmt` and `just lint` after the pruning sequence**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && just fmt && just lint 2>&1 | tail -5
```

Expected: both clean. If clippy surfaces new warnings (e.g., dead imports, unused variables after a prune), address before moving on.

- [ ] **Step 5.4: Run `just check` end-to-end**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && just check 2>&1 | tail -10
```

Expected: fmt-check, clippy with `-D warnings`, all tests pass. `cargo audit` may surface the two pre-existing dependency vulnerabilities flagged in Findings backlog #4 (`RUSTSEC-2026-0002`, `RUSTSEC-2025-0035`) — those are not unit blockers.

- [ ] **Step 5.5: Populate the Pruning record subsection in AUDIT-PLAN.md**

Under `#### Pruning record`, list each prune commit with its B-entry reference, hash, and one-line summary. Mirror the format from Unit 3's Pruning record.

---

## Task 6: Test pass (AUDIT-PLAN.md Step 6)

**Files:**
- Modify: `refbox/tests/features/manual_alarm.feature` (align scenarios from `@user_verified` catalog entries; add test-state tags)

### Step 6.1 — Align the pre-existing `.feature` file

The file at `refbox/tests/features/manual_alarm.feature` already exists from doc-revision commit `9cef2c4`. Per Unit 1's settled convention (refinement #2), align this file rather than creating a parallel one in `docs/audit-scenarios/`.

- [ ] **Step 6.1.1: Read the existing file**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && wc -l refbox/tests/features/manual_alarm.feature && cat refbox/tests/features/manual_alarm.feature
```

Note: if the file already uses `Background:` blocks (per the existing project convention), the audit's no-`Background:` rule applies. The audit's aligned scenarios state preconditions inline. Document this in the file header as a comment.

- [ ] **Step 6.1.2: Replace the file content with the audit's aligned scenarios**

For each `@user_verified` scenario from AUDIT-PLAN.md's Unit 4 Scenarios subsection, copy verbatim into `manual_alarm.feature`. Tags: `@user_verified` only at this stage (test-state tags come later in Step 6.2). Order scenarios by their S4.N number. No `Background:` block.

File header comment to add at top:

```gherkin
# Manual alarm button — audit scenarios
#
# Source of truth: Audit Unit 4 (see AUDIT-PLAN.md). The audit deliberately
# omits `Background:` blocks so each scenario is reviewable in isolation
# during the per-unit review session.
```

- [ ] **Step 6.1.3: Commit the alignment**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git add refbox/tests/features/manual_alarm.feature && git commit -m "docs(refbox): align manual_alarm.feature with audit unit 4 scenarios"
```

Expected: pre-commit hook passes (no Rust changes).

### Step 6.2 — Execute the three test sessions

Each session group's scenarios share a precondition; the operator navigates to that state once and walks through each scenario from there. After each session, update `manual_alarm.feature` with `@tested_pass` / `@tested_fail` / `@tested_inconclusive` tags and a date-stamped session comment block at the top of the `Feature:`.

Claude launches the refbox (`WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox` with `dangerouslyDisableSandbox:true` and `run_in_background:true` per memory entry `feedback_user_drives_refbox_ui`); the operator drives the UI and reports observations; Claude watches the launched process's log output for unexpected side effects.

- [ ] **Step 6.2.1: Pre-session — launch refbox**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox
```

(Run in background with `dangerouslyDisableSandbox:true`. Wait for refbox to fully render before starting Session 1.)

- [ ] **Step 6.2.2: Session 1 — Sound Options page**

Scenarios in this session: S4.8 (settings toggle default + on), S4.9 (greyed when Sound Enabled is Off), plus any translation-rendering checks from S4.12 / S4.13 that can be verified statically.

Operator path: open Settings → Sound Options. Operator verifies:

1. Alarm Button row appears below the existing three rows of sound settings, on the left column, above the Manage Remotes section
2. Defaults to Off
3. Toggle to On — no immediate visible effect on this page
4. Toggle Sound Enabled to Off — Alarm Button row visibly greys (matches the other sound-dependent toggles)
5. Toggle Sound Enabled back to On — Alarm Button row returns to interactive

After the session, Claude updates `manual_alarm.feature` with `@tested_pass` (or other) for S4.8 and S4.9, and adds the session comment:

```gherkin
# Test session 1 — 2026-MM-DD — Sound Options page — S4.8 pass, S4.9 pass
```

Commit: `docs(refbox): record test session 1 (sound options) for manual alarm`.

- [ ] **Step 6.2.3: Session 2 — Main game screen with feature enabled**

Scenarios: S4.1, S4.2, S4.3 (active play mouse, 150ms band), S4.5 (active play with timeout, 1s band), S4.6, S4.7 (break periods, 1s band), S4.10, S4.11 (layout variants), S4.12, S4.13, S4.14 (button appearance), S4.15 (release natural cycle).

Operator path: from Sound Options with Alarm Button On, configure a fresh game with fouls-and-warnings tracking on, start the game, and walk:

1. **First Half clock running, no timeout, fouls-on layout:** verify split-vertical layout with alarm-left / warnings-right. Verify button is red with "Alarm / Or press Spacebar". Mouse-tap for ~100ms — no sound fires. Mouse-hold for ~300ms — sound fires after 150ms and continues. Release — sound stops at the end of the currently playing tone.
2. **Trigger a timeout (Team / Ref / Penalty Shot):** verify button switches to blue with "Hold to Test / Or hold Spacebar". Mouse-tap for ~500ms — no sound. Mouse-hold for ~1500ms — sound fires after 1s and continues. Release — sound stops at the end of the currently playing tone.
3. **End the timeout:** verify button returns to red.
4. **Advance to Half Time:** verify blue + "Hold to Test", 1s hold fires.
5. **Advance to Second Half:** verify red + "Alarm" returns; 150ms hold fires.
6. **Advance through Pre-OT, Overtime, OT Half Time, Pre-Sudden Death, Sudden Death:** verify each state's colour + label + hold duration matches the spec table.
7. **Advance to Between Games:** verify blue + "Hold to Test", 1s hold fires.
8. **Restart with fouls-and-warnings tracking off:** verify full-width single-container layout for the alarm. Repeat one or two state checks to confirm behaviour is identical.

After the session, update `manual_alarm.feature` with per-scenario test tags and add the session comment:

```gherkin
# Test session 2 — 2026-MM-DD — Main game screen across states — S4.1 pass, S4.2 pass, ...
```

Commit: `docs(refbox): record test session 2 (main game screen) for manual alarm`.

- [ ] **Step 6.2.4: Session 3 — Spacebar parity and inactive screens**

Scenarios: S4.4 (spacebar parity with mouse), S4.16 (spacebar has no effect on non-main screens).

Operator path: from the same running game, repeat the Session 2 state walk using only the spacebar:

1. First Half clock running: spacebar-tap under 150ms — no fire. Spacebar-hold past 150ms — fires. Release — stops at end of tone.
2. Trigger a timeout: 1s spacebar-hold fires; short tap does not.
3. Half Time / Between Games: 1s spacebar-hold fires.
4. Hold mouse and spacebar simultaneously during First Half: confirm the alarm does not double-fire on release of one input while the other is still held. (Tests the independent-flag design from spec divergence area.)

Then open non-main screens:

5. **Game Options page:** spacebar — no effect on alarm. Confirm any text-input field on this page accepts spacebar normally (i.e., the subscription does not consume the key globally).
6. **Penalties Page:** spacebar — no effect.
7. **Score Edit Page:** spacebar — no effect.

After the session, update `manual_alarm.feature` with per-scenario test tags and add the session comment:

```gherkin
# Test session 3 — 2026-MM-DD — Spacebar parity + inactive screens — S4.4 pass, S4.16 pass
```

Commit: `docs(refbox): record test session 3 (spacebar parity) for manual alarm`.

### Step 6.3 — Companion plan's 11-item checklist cross-check

- [ ] **Step 6.3.1: Read the checklist**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && sed -n '/Manual Verification/,/Step 3/p' .audit/companion-plan.md
```

The 11-item checklist lives in companion plan Task 3 Step 2 (`docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md`). It enumerates: feature off layout unchanged, feature on default off, feature on enables layout, active-play 150ms fires, active-play sub-150ms does not, timeout 1s, break 1s, fouls-on split layout, fouls-off full-width layout, spacebar parity, release natural-cycle.

- [ ] **Step 6.3.2: Cross-check each item against the test sessions**

For each of the 11 items, verify a corresponding scenario was exercised in Sessions 1–3. If any item is uncovered, add a Findings-backlog entry: "Companion plan checklist gap: <item> not covered in Unit 4 sessions" with a suggested follow-up branch name.

### Step 6.4 — Optional Rust regression tests

> **Skip Step 6.4 if no concerns surfaced during catalog review or test sessions.**

If catalog review (Task 4) or test sessions (Step 6.2) surfaced a state-machinery invariant worth locking in code, add a Rust test alongside the code it tests. Pre-flight estimate: 0–3 tests, focused on:

- Token-cancellation invariant: after `AlarmReleased`, a stale `AlarmDelayElapsed` with the prior token does not start the buzzer
- Mouse/spacebar independence: holding both and releasing one does not stop the buzzer while the other is still held

Each test follows TDD: write the failing test first, run to confirm it fails on a temporary revert, restore the code, confirm it passes. Commit one test per concern.

### Step 6.5 — Full validation pass

- [ ] **Step 6.5.1: Run `just check` end-to-end**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && just check 2>&1 | tail -10
```

Expected: fmt-check clean, clippy clean with `-D warnings`, all tests pass. `cargo audit` may flag the two pre-existing dependency vulnerabilities — note in Test status if so.

- [ ] **Step 6.5.2: Capture backend-behaviour test status in AUDIT-PLAN.md notes**

Backend behaviours (migration default, BoolGameParameter wiring, helper-method extraction, token cancellation, mouse/spacebar independent flags) have no `.feature` row. Under Unit 4's catalog, append a `#### Test status (backend behaviours)` subsection mirroring Unit 1's pattern:

```markdown
#### Test status (backend behaviours)

- **B4.N (migration default)** — covered by the existing migration test from commit `38799da`. Verified: `cargo test -p refbox migrate` passes.
- **B4.M (BoolGameParameter wiring)** — covered by configuration-page tests. Verified: `cargo test -p refbox configuration` passes.
...
```

These bullets feed into the retroactive ADR's prose in Task 7.

---

## Task 7: Retroactive ADR (AUDIT-PLAN.md Step 7)

**Files:**
- Create: `docs/decisions/021-manual-alarm-button.md`

- [ ] **Step 7.1: Confirm the ADR number**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && ls docs/decisions/ | sort
```

Expected: ADRs 001–005 visible (audit branch cut from master, before backlog-adrs / Unit 1 / Unit 2 land). The audit's ADR is numbered against the **expected post-merge state** per Unit 1 refinement #8. Use 021 — Unit 1 holds 019; Unit 2 holds 020; Unit 4 takes 021. (If a prior unit lands on master before Unit 4 closes, increment.)

- [ ] **Step 7.2: Draft the ADR from the retroactive template**

Use the template from AUDIT-PLAN.md's "Retroactive ADR template" section. Mandatory sections:

- **Title:** `# ADR 021: Manual Alarm Button (Retroactive)`
- **Status:** `Accepted (retroactive)`
- **Date:** today's date
- **Audit unit:** 4 — Manual alarm button
- **Audit PR:** none until Final Integration (Step 8 holds the branch local)

**Context** (2–3 sentences): The manual alarm button was added between 2026-04-14 and 2026-04-17 with AI assistance, ~16 commits, against a pre-existing spec and two plan documents. The audit (2026-05-MM) confirmed the shipped behaviour matches the spec; this ADR records the surviving feature.

**Decision** section embeds every `@user_verified @tested_pass` scenario from `refbox/tests/features/manual_alarm.feature` as Gherkin code blocks with one sentence of plain-English framing per scenario. Backend behaviours (migration default, BoolGameParameter wiring, helper-method extraction, token cancellation, mouse/spacebar independent flags) appear as a separate bulleted subsection.

**Consequences** section: what this enables for the operator (manual buzzer trigger without remote dependency, accidental-tap protection via the 150ms debounce); what we're committing to maintaining (the spec, the uniform-hold model, the colour/label state predicate); constraints on future changes (ADR 006 successor must preserve the operator-observable behaviour or document explicit deltas).

**What was removed during audit** section: bullet per `@deleted` entry with the reason. If none, write: "No behaviours were deleted during the audit; the shipped implementation matched the spec with only minor naming and helper-extraction divergences (see Notes on divergence below)."

**Notes on divergence from spec** (new section, Unit 4 specific): list each `diverges-from-spec` catalog entry with the operator's keep/delete/update decision. Examples:

- Hold duration: spec says 150ms; companion plan said 250ms (stale); code lands at 150ms. Spec and code agree; companion plan is acknowledged historical.
- State naming: spec uses `alarm_hold_generation`; code uses `alarm_delay_token`. Operator decision: code naming wins; future spec revision (if any) should align.

**What was not verified** section: any catalog entry ending `@tested_inconclusive`, plus any test-protocol gaps (e.g., long-duration tournament-style use). Likely empty.

**References**:

- Spec: `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md`
- Companion delta plan: `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md`
- Original plan: `docs/superpowers/plans/2026-04-14-manual-alarm-button.md` (retconned mid-flight)
- Audit-design spec: `docs/superpowers/specs/2026-05-13-audit-unit-4-manual-alarm-design.md`
- Per-unit audit plan: `docs/superpowers/plans/2026-05-13-audit-unit-4-manual-alarm-button.md`
- **ADR 006 (proposed successor)**: `docs/decisions/006-multi-remote-alarm-buttons.md` — gated on findings recorded here
- Audit branch: `audit/refbox/manual-alarm-button`
- Original commits (16): `bc66e1e`, `38799da`, `b3fde3e`, `40a857a`, `5685a29`, `94b1c64`, `8db9387`, `36cde0e`, `2a4294e`, `babe3ca`, `bc620ff`, `d5f485a`, `9cef2c4`, `7f173ee`, `c90348b`, `ff6018b`

- [ ] **Step 7.3: Spec-coverage sweep**

Read the spec section-by-section against the ADR's Decision section. Every spec claim should map to either a scenario embed or a backend bullet. List any gaps and fill before committing.

- [ ] **Step 7.4: Commit the ADR**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git add docs/decisions/021-manual-alarm-button.md && git commit -m "docs(refbox): add ADR 021 for manual alarm button (retroactive)"
```

Expected: pre-commit hook passes; commit lands on `audit/refbox/manual-alarm-button`.

- [ ] **Step 7.5: Final `just check`**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && just check 2>&1 | tail -10
```

Expected: fmt-check clean, clippy clean, all tests pass, `cargo audit` clean (modulo pre-existing CVEs).

---

## Task 8: Hold branch locally (AUDIT-PLAN.md Step 8)

**Files:** None.

- [ ] **Step 8.1: Confirm branch is local-only**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && git branch -vv | grep manual-alarm-button
```

Expected: branch tracks nothing remote (no `[origin/...]` indicator). Do **not** push.

- [ ] **Step 8.2: Optional backup**

Per the playbook, weekly backups are recommended during long audits:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-4-manual-alarm-button && mkdir -p ~/backups && git bundle create ~/backups/audit-unit-4-manual-alarm-button-2026-MM-DD.bundle audit/refbox/manual-alarm-button
```

Optional. Skip if the audit is being closed within the day.

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Step 9)

**Files:**
- Modify: `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (Unit 4 status flip; Completed audits summary)

- [ ] **Step 9.1: Operator reviews the decision log and test status**

Ask the operator to:

1. Read the Unit 4 Decisions subsection in AUDIT-PLAN.md
2. Read `refbox/tests/features/manual_alarm.feature` for test tag distribution
3. Read `docs/decisions/021-manual-alarm-button.md`
4. Confirm: "Unit 4 approved" or request changes

- [ ] **Step 9.2: Flip status**

In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
- Update the unit-catalog table row: `in progress` → `complete-pending-integration (2026-MM-DD)`
- Update the `### Unit 4 — Manual alarm button` section's `**Status:**` line the same way

- [ ] **Step 9.3: Add a summary entry to "Completed audits"**

Per Unit 1 refinement #3 (in-place flip + summary pointer, not destructive section-move), add an entry to the Completed audits section near the bottom of AUDIT-PLAN.md, between the existing Unit 3 and Unit 2 entries (newest first):

```markdown
#### Unit 4 — Manual alarm button — complete-pending-integration 2026-MM-DD

- **Branch:** `audit/refbox/manual-alarm-button` (local only; <N> commits ahead of `origin/master`; not pushed)
- **Per-unit plan:** `docs/superpowers/plans/2026-05-13-audit-unit-4-manual-alarm-button.md`
- **Audit-design spec:** `docs/superpowers/specs/2026-05-13-audit-unit-4-manual-alarm-design.md`
- **Retroactive ADR:** `docs/decisions/021-manual-alarm-button.md`
- **Scenarios:** `refbox/tests/features/manual_alarm.feature` — <N> scenarios; <X> @tested_pass, <Y> @tested_fail, <Z> @tested_inconclusive
- **Catalog outcome:** <N> entries; <X> @user_verified; <Y> @deleted
- **Tests added during audit:** <N> Rust regression tests (or "none — backend behaviours covered by existing tests")
- **Audit commits on branch:** <list of SHAs>
- **What was not verified:** <bullet list, or "nothing — all scenarios and backend behaviours verified">
- **Findings filed:** <N> new Findings backlog items
- **Full details section:** retained in "Unit-by-unit details" above with status flipped to complete-pending-integration.
```

- [ ] **Step 9.4: Process refinements (if any)**

If Unit 4 surfaced any improvements to the playbook (catalog template, slop-catching checklist, workflow tweaks), add an entry under "Process refinements log" → "From Unit 4 (2026-MM-DD)". Examples worth watching for:

- A spec-as-oracle pattern that future units with pre-existing specs should adopt
- Handling of stale companion plans (where one plan is acknowledged stale relative to spec + code)
- The doc-revision-commit-as-meta-behaviour pattern (`9cef2c4`)
- Three-session test grouping with checklist-backstop pattern

- [ ] **Step 9.5: Confirm the unit is closed**

Tell the operator:

> "Unit 4 complete-pending-integration. Branch `audit/refbox/manual-alarm-button` holds locally with <N> audit commits. Retroactive ADR 021 written. AUDIT-PLAN.md status flipped. Ready to move on to Unit 5 (Referee names display)?"

---

## Risks and known divergences

These are the audit's pre-flight risk areas, repeated here from the audit-design spec for the executing subagent's convenience. They are starting points for catalog questions, not pre-decisions.

1. **Hold-duration constant.** Spec line 72 says 150ms; companion plan says 250ms (3 places); code lands at 150ms via `ff6018b`. Verify spec line was updated to 150 in doc-revision `9cef2c4`. One B-entry records the chain.

2. **State-naming divergence.** Spec line 117 names `alarm_hold_generation`; code uses `alarm_delay_token`. B-entry to confirm code naming wins.

3. **Independent-flag design vs spec's generation counter.** Spec uses a single generation counter; code uses two flags + a token. B-entry confirms the added complexity handles the simultaneous-press case.

4. **`manual_alarm_hold_duration()` helper extraction.** Companion plan flagged as "Optional but recommended". B-entry confirms whether it was done.

5. **`disabled_container` import cleanup.** Quick grep confirms.

6. **`mouse_area` always-wrapped guarantee.** Verify no conditional wrap remains.

7. **Translation coverage.** 5+ new keys × ~13 languages. Plain-text grep per key.

8. **Doc-revision commit `9cef2c4` as a meta-behaviour.** B-entry records the design-correction event.

9. **Spacebar gating on non-main screens.** Verify subscription vs handler gating; ensure typing spacebar in a text field has no unintended effect.

10. **Sound-while-held continuous-mode dispatch.** Confirm the dispatch path matches the regular automatic-buzzer continuous path; flag re-implementation otherwise.

---

## Files Created or Modified by This Plan

- `.worktrees/audit-unit-4-manual-alarm-button/` (new worktree, lifecycle: removed at Final Integration)
- `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (gitignored; multiple edits)
- `.audit/unit-4-commits-summary.txt`, `.audit/unit-4-stat.txt`, `.audit/unit-4-messages.txt`, `.audit/unit-4-diff.txt`, `.audit/oracle-spec.md`, `.audit/companion-plan.md`, `.audit/adr-006.md` (local working artifacts; not committed)
- `refbox/tests/features/manual_alarm.feature` (aligned)
- `docs/decisions/021-manual-alarm-button.md` (created)
- Possibly: refbox source files for any `@deleted` entries
- Possibly: 0–3 new Rust regression tests in `refbox/src/app/` or `refbox/tests/`

---

## Estimated commits on the audit branch

- 0–N prune commits (Task 5; only if any catalog entries are `@deleted`)
- 1 scenario-alignment commit (Step 6.1.3)
- 3 test-session record commits (Step 6.2.2 / 6.2.3 / 6.2.4)
- 0–3 regression-test commits (Step 6.4; optional)
- 1 ADR commit (Step 7.4)
- **Total:** 5–10 commits on `audit/refbox/manual-alarm-button` at close.
