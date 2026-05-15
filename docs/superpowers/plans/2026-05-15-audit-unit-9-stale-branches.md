# Audit Unit 9 — Stale Branches Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Triage all local and remote branches into delete / keep / audit-separately, completing the final audit prerequisite for Final Integration.

**Architecture:** Branch hygiene, not code audit. No diff to read, no behaviour catalog, no Gherkin scenarios, no ADR. A single audit branch `audit/workspace/stale-branches` is cut from master to satisfy the playbook's Step 1, but it will hold zero code commits — the audit record lives in `AUDIT-PLAN.md`'s "Per-branch resolution log" section (gitignored, so no commit). All branch deletions are gated by a user-confirmation step; local-safe-deletes (`git branch -d`) batch automatically, while local-force-deletes (`-D`) and remote-deletes (`git push origin --delete`) require per-branch typed approval.

**Tech Stack:** `git` CLI only. No code changes. No tests. No build.

---

## Context

After audit Units 1–8 closed, the workspace contains 41 local and 41 remote-only branches (counted on 2026-05-15). Many predate the audit; some are predecessors of audit-Unit branches that will be superseded at Final Integration; some are operator-owned WIP whose status only the operator knows; some are bot-managed (Renovate / Dependabot / GitHub merge-queue) and should not be touched manually.

Per `AUDIT-PLAN.md` Unit 9 section at line 4623: "This is branch hygiene, not a code audit. For each branch in the list below: check whether its commits are already on master. If yes, delete the branch. If no, decide: (a) audit it as a separate unit, (b) abandon and delete, or (c) keep and revisit. Save for last — by then most ambiguity should be resolved by earlier audits."

Final Integration ([AUDIT-PLAN.md:557](../../../AUDIT-PLAN.md#L557)) explicitly gates on all 9 audit units complete-pending-integration. Unit 9 closes that gate.

## Bucketing rubric

For each branch, the executor applies this decision tree **in order** (first match wins):

1. **Audit-Unit-1-8 branch or `feat/schedule-processor/csv-display-order`** → **(c) KEEP**. Needed for Final Integration; deleted there, not here.
2. **Active working branch with a checked-out worktree** (per `git worktree list`) → **(c) KEEP**. Worktree presence = active use.
3. **Backlog/ADR working branch** (`docs/workspace/backlog-adrs`, `docs/workspace/adr-012-time-calc-log-level`, `chore/refbox/time-calc-log-level`) → **(c) KEEP**. Carries unmerged ADR work.
4. **Predecessor of an audit-Unit branch** (e.g., `feat/refbox/portal-health-indicator` is the predecessor of `audit/refbox/portal-health`) → **(c) KEEP**. Delete-at-Final-Integration, not here, in case audit-branch rebase needs the predecessor as fallback.
5. **All commits already on master** (verified via `git rev-list --count master..<branch>` = 0 OR `git cherry master <branch>` shows only `-` lines) → **(b) DELETE** with `git branch -d` (safe; git refuses if not merged). Batch-safe.
6. **Renovate/Dependabot/GitHub merge-queue auto-managed branch** (`origin/renovate/*`, `origin/dependabot/*`, `origin/gh-readonly-queue/*`, `origin/staging`) → **(c) KEEP — bot-managed**. Out of scope; bots own their lifecycle.
7. **>6-months-old non-conventional branch with clear AI-era marker** (e.g., `Scoresheets`, `high-contrast-ui`, `origin/Attempt_at_Chrome_OS`, `origin/Button-layout-changes`, `origin/Eric-First-Try-Edits`, `origin/Cha-Test`) → **(b) DELETE** but only after per-branch user confirmation (force-delete `-D` for locals; `--delete` push for remotes).
8. **Operator-owned WIP whose status is unclear** → **(c) KEEP for now**, surface a question to the operator. Default to keeping; deletion is irreversible without remote re-fetch.

## Initial per-branch classification

The executor enters Task 3 with this table as a starting point. The user is the source of truth and can override any line.

### Local branches (41)

| Branch | Bucket | Reason |
|---|---|---|
| `audit/refbox/adr-009-settings` | c | Unit 3 audit branch (rule 1) |
| `audit/refbox/confirm-score-timing` | c | Unit 1 audit branch (rule 1) |
| `audit/refbox/language-ui-chrome` | c | Unit 8 audit branch (rule 1) |
| `audit/refbox/manual-alarm-button` | c | Unit 4 audit branch (rule 1) |
| `audit/refbox/portal-health` | c | Unit 7 audit branch (rule 1) |
| `audit/refbox/referee-names` | c | Unit 5 audit branch (rule 1) |
| `audit/refbox/small-fixes-cluster` | c | Unit 6 audit branch (rule 1) |
| `audit/uwh-common/list-of-placements` | c | Unit 2 audit branch (rule 1) |
| `feat/schedule-processor/csv-display-order` | c | Held-local production fix (rule 1 covers it explicitly) |
| `docs/workspace/backlog-adrs` | c | Current branch; holds ADRs 006–018 (rule 3) |
| `refactor/refbox/settings-navigation` | c | Active worktree at `~/projects/refbox-settings-worktree`; ADR 009 finalized here (rule 2) |
| `docs/workspace/adr-012-time-calc-log-level` | c | Paired with chore branch for ADR 012 (rule 3) |
| `chore/refbox/time-calc-log-level` | c | Has worktree `adr-012-time-calc-log-level`; paired with ADR 012 (rule 2) |
| `fix/uwh-common/schedule-order-deserialize-compat` | c | Has worktree `scoresheet-portal-fix`; needs operator review on status (rule 2 / rule 8) |
| `feat/refbox/portal-health-indicator` | c | Predecessor of `audit/refbox/portal-health` (rule 4) |
| `feat/uwh-common/list-of-placements` | c | Predecessor of `audit/uwh-common/list-of-placements` (rule 4) |
| `feat/uwh-common/listofplacements-support` | c | Predecessor / sibling of Unit 2 source material (rule 4) |
| `pr/schedule-processor-list-of-placements` | c | Legacy `pr/` grandfathered; likely covered by Unit 2 (rule 4) |
| `feat/refbox/manual-alarm-button` | c | Predecessor of Unit 4 audit branch (rule 4) |
| `feat/refbox/referee-display-names` | c | Predecessor of Unit 5 audit branch (rule 4) |
| `feat/refbox/team-referee-assignments` | c | Source material for Unit 5 (rule 4) |
| `feat/refbox/referee-display` | c | Source material for Unit 5 (rule 4) |
| `feat/refbox/language-support` | c | Predecessor of Unit 8 audit branch (rule 4) |
| `fix/refbox/confirm-score-false-panic` | c | Predecessor of Unit 1 audit branch (rule 4) |
| `fix/refbox/language-select-font-fallback` | c | Sibling of Unit 8 source material (rule 4) |
| `feat/schedule-processor/scoresheet-generation` | c | Has worktree `scoresheet-generation`; active (rule 2) |
| `chore/ci/release-pipeline` | c-pending-operator | In-flight CI work, last 2026-04-16; needs operator status (rule 8) |
| `chore/ci/rpi-cross-compile-fix` | c-pending-operator | Possibly superseded by `origin/chore/ci/rpi-cross-compile`; needs operator status (rule 8) |
| `chore/deps/fix-audit-cves` | c-pending-operator | Old (2026-04-10), may be superseded by Renovate (rule 8) |
| `chore/refbox/clean-unused-imports` | c-pending-operator | Small chore, status unclear (rule 8) |
| `chore/workspace/assert-style-cleanup` | c-pending-operator | Non-trivial (ahead=11) but old; needs operator status (rule 8) |
| `chore/workspace/bump-v0-4-0` | b-pending-cherry-check | v0.4.0 shipped 2026-04-15; commits likely already on master via different SHAs — verify with `git cherry` (rule 5) |
| `docs/workspace/web-app-audit` | c-pending-operator | Small (ahead=3), status unclear (rule 8) |
| `feat/refbox/help-expand-page` | c-pending-operator | Tiny (ahead=1, behind=1); needs operator status (rule 8) |
| `feat/workspace/desktop-build` | c-pending-operator | Large (ahead=74), status unclear (rule 8) |
| `fix/refbox/centered-text-clipping` | c-pending-operator | Non-trivial fix, status unclear (rule 8) |
| `fix/schedule-processor/single-half-detection` | c-pending-operator | Old (2026-04-10), small (ahead=3) (rule 8) |
| `refactor/schedule-processor/deferred-csv-loading` | c-pending-operator | Non-trivial refactor, status unclear (rule 8) |
| `Scoresheets` | b-pending-confirm | AI-era 2025-11-08, non-conventional name (rule 7) |
| `archive/language-support-first-attempt` | b-pending-confirm | Explicit `archive/` prefix (rule 7) |
| `high-contrast-ui` | b-pending-confirm | AI-era 2025-10-17, non-conventional name (rule 7) |

### Remote-only branches (41)

| Branch | Bucket | Reason |
|---|---|---|
| `origin/Attempt_at_Chrome_OS` | b-pending-confirm | AI Sept 2025, called out in playbook (rule 7) |
| `origin/Button-layout-changes` | b-pending-confirm | AI Sept 2025, called out in playbook (rule 7) |
| `origin/Eric-First-Try-Edits` | b-pending-confirm | AI Sept 2025, called out in playbook (rule 7) |
| `origin/Cha-Test` | b-pending-confirm | 2025-10-08, non-conventional name (rule 7) |
| `origin/trying` | b-pending-confirm | 2022, presumably dead (rule 7) |
| `origin/staging` | c | Bot/queue branch (rule 6) |
| `origin/gh-readonly-queue/master/...` | c | GitHub merge queue auto-managed (rule 6) |
| `origin/feat/translations` | b-pending-confirm | 2024-12, very old AI-era (rule 7) |
| `origin/uwh-refbox-game-info-layout` | b-pending-confirm | 2025-09-06 AI-era, non-conventional (rule 7) |
| `origin/revert-455-add_infraction_name` | b-pending-confirm | 2024-12, stale revert-PR branch (rule 7) |
| `origin/pr/1-schedule-uwh-common` | b-pending-confirm | 2025-12-09, legacy PR submission slot (rule 7) |
| `origin/pr/2-refbox` | b-pending-confirm | 2025-12-09, legacy PR submission slot (rule 7) |
| `origin/pr/3-misc` | b-pending-confirm | 2025-12-09, legacy PR submission slot (rule 7) |
| `origin/pr/4-workspace-fmt` | b-pending-confirm | 2025-12-09, legacy PR submission slot (rule 7) |
| `origin/chore/ci/rpi-cross-compile` | c-pending-operator | Possibly merged or paired with local variant (rule 8) |
| `origin/chore/deps/audit-unblock` | c-pending-operator | Operator-owned chore (rule 8) |
| `origin/chore/workspace/decision-records` | c-pending-operator | Likely superseded by `docs/workspace/backlog-adrs` (rule 8) |
| `origin/chore/workspace/gitignore-local-artifacts` | c-pending-operator | Small chore (rule 8) |
| `origin/fix/refbox/confirm-score-timing` | c-pending-operator | Predecessor of Unit 1 audit branch; remote-only (rule 4 by analogy) |
| `origin/fix/refbox/sound-artifacts` | c-pending-operator | Possibly covered by Unit 6 (rule 4 by analogy) |
| `origin/fix/refbox/ui-clipping` | c-pending-operator | Possibly covered by Unit 8 or local `fix/refbox/centered-text-clipping` (rule 4) |
| `origin/dependabot/*` (8 branches) | c | Bot-managed (rule 6) |
| `origin/renovate/*` (12 branches) | c | Bot-managed (rule 6) |

---

## Tasks

### Task 1: Setup

**Files:**
- Read: `AUDIT-PLAN.md` (Unit 9 section near line 4623; "Per-branch resolution log" placeholder at line 4664)
- Modify: `AUDIT-PLAN.md` (status flip in catalog table at line 66)

- [ ] **Step 1.1: Verify master is clean from main repo.**

```bash
cd /home/estraily/projects/uwh-refbox-rs && git status
```
Expected: branch is `docs/workspace/backlog-adrs`, only untracked file is `.claude/scheduled_tasks.lock` (harness lock — ignore).

- [ ] **Step 1.2: Refresh master.**

```bash
cd /home/estraily/projects/uwh-refbox-rs && git fetch origin && git log --oneline master..origin/master | head
```
Expected: zero or a small number of new commits. If new commits, ask operator before fast-forwarding master.

- [ ] **Step 1.3: Cut the audit branch (from current master, not the working branch).**

```bash
cd /home/estraily/projects/uwh-refbox-rs && git checkout master && git checkout -b audit/workspace/stale-branches
```
Expected: branch created. Note that this branch will hold zero code commits — it exists only to satisfy the playbook's per-unit branch convention. Switch back to `docs/workspace/backlog-adrs` immediately after Task 1.5.

Then return to the working branch for AUDIT-PLAN.md edits:

```bash
git checkout docs/workspace/backlog-adrs
```

- [ ] **Step 1.4: Flip Unit 9 status in AUDIT-PLAN.md.**

Edit `AUDIT-PLAN.md` line 66 from `| 9 | Stale branches cleanup | workspace | not started | ...` to `| 9 | Stale branches cleanup | workspace | in progress (started 2026-05-15) | ...`.

Also update the Unit 9 detail section heading at line 4623 from `**Status:** not started` to `**Status:** in progress (started 2026-05-15)`.

(No commit — AUDIT-PLAN.md is gitignored.)

- [ ] **Step 1.5: Decide audit-branch retention strategy.**

Two options for the empty `audit/workspace/stale-branches` branch:
- **A — Keep as marker:** Leave the empty branch in place. At Final Integration it appears in the sequencing list as a no-op (no PR, no merge). Provides a paper trail that Unit 9 was executed.
- **B — Discard:** Delete the empty branch at end of unit. Recorded only in AUDIT-PLAN.md.

**Recommendation: A.** Cost is one extra ref; benefit is visible-in-`git branch` evidence that the unit ran. Document this choice in the Process refinements log if A is chosen.

### Task 2: Cherry-check the "already on master" candidates

For each branch in row "b-pending-cherry-check" of the table above:

- [ ] **Step 2.1: Run `git cherry master <branch>` for `chore/workspace/bump-v0-4-0`.**

```bash
cd /home/estraily/projects/uwh-refbox-rs && git cherry -v master chore/workspace/bump-v0-4-0
```
Expected output legend:
- Lines starting with `-` → commit is already on master (different SHA, same patch). Safe to delete.
- Lines starting with `+` → commit is NOT on master. Cannot safe-delete; reclassify under operator-input bucket.

If all lines are `-` (or output is empty AND `git rev-list --count master..chore/workspace/bump-v0-4-0` is `0`), confirm bucket b for batch delete. Otherwise flip to c-pending-operator.

- [ ] **Step 2.2: Spot-check 5 other branches** with high `behind` count (49+) to see if any are also fully on master via different SHAs.

```bash
for b in chore/ci/rpi-cross-compile-fix chore/deps/fix-audit-cves chore/refbox/clean-unused-imports feat/refbox/team-referee-assignments fix/refbox/confirm-score-false-panic; do
  echo "=== $b ===" && git cherry -v master $b
done
```
For any branch where every line starts with `-`, reclassify to bucket b.

### Task 3: User confirmation gate

- [ ] **Step 3.1: Present the classification table to the operator.**

Show the operator the local-branches table and remote-only table (verbatim, with any Task-2 reclassifications applied) plus a list of every branch tagged `c-pending-operator` or `b-pending-confirm` with a one-line ask.

Use AskUserQuestion in **batches**, not 41 separate questions. Group as:
1. **Batch A — operator-owned chore/feat/fix branches** (~12 branches): one multi-select asking "Which of these still represent work-in-progress you want to preserve?". Unselected branches become candidate (b) deletes.
2. **Batch B — AI-era / non-conventional branches confirmed for deletion** (~8 local + remote): one multi-select asking "Confirm deletion of these old branches"; unselected stays (c).
3. **Batch C — `fix/uwh-common/schedule-order-deserialize-compat`** (single question): "Is the `scoresheet-portal-fix` worktree still active work?" — its uwh-common scope makes this higher-stakes.

**Honor [[one-question-at-a-time]]** for the high-stakes single questions; the multi-select batches are acceptable because they ask one logical question per batch.

- [ ] **Step 3.2: Update AUDIT-PLAN.md's Per-branch resolution log (line 4664).**

For each branch, append:

```
- <branch> — **(a|b|c)** — <reason from operator or default rule>
```

(No commit — AUDIT-PLAN.md is gitignored.)

### Task 4: Execute bucket-(b) deletions in safety-tiered groups

**Group order:** safest first (local-merged), then locally-force-delete with per-branch confirm, then remote-delete with per-branch confirm.

- [ ] **Step 4.1: Group 1 — local safe-delete (commits already on master via different SHAs).**

For each branch confirmed bucket-(b) by Task 2 cherry-check:

```bash
git branch -d <branch>
```
Expected: `Deleted branch <branch> (was <sha>).` If git refuses with "not fully merged", do not force — escalate to operator and reclassify.

- [ ] **Step 4.2: Group 2 — local force-delete (AI-era confirmed by operator in Task 3 Batch B).**

For each branch:

```bash
git branch -D <branch>
```
Expected: `Deleted branch <branch> (was <sha>).` Run one at a time with a status-update line per branch ("deleting `Scoresheets`...") so the operator can interrupt if needed.

- [ ] **Step 4.3: Group 3 — remote-only deletions (operator-confirmed in Task 3 Batch B).**

For each branch:

```bash
git push origin --delete <branch-name-without-origin-prefix>
```
Expected: `- [deleted]   <branch>`. **One at a time with operator-visible status line.** Per CLAUDE.md and `.claude/rules/communication.md`: remote operations affect shared state and need explicit per-action surface.

If any `push --delete` fails (branch protected, network issue), stop and report — do not retry blindly.

### Task 5: Document keepers and audit-separately classifications

- [ ] **Step 5.1: For each bucket-(c) branch**, ensure its line in the Per-branch resolution log notes the keep reason (rule number from the rubric).

- [ ] **Step 5.2: For each bucket-(a) branch** (audit separately): add a stub in the AUDIT-PLAN.md audit unit catalog at line 54 as `| 10 | <descriptive name> | <scope> | not started | <branch-name> |`. If zero bucket-(a) branches emerge (likely outcome — audit Units 1–8 absorbed everything substantive), record that explicitly in the Per-branch resolution log: `No branches required separate audit; Units 1–8 covered the AI-era surface area.`

### Task 6: Step 7 — ADR not required

Per `AUDIT-PLAN.md` Step 7 rule at line 168: "If a unit ends with zero `@user_verified` behaviours (every entry was `@deleted`), no ADR is needed." Unit 9 has zero behaviours (no code audit), so this rule applies.

- [ ] **Step 6.1: Record the no-ADR decision** in the Per-branch resolution log header: `ADR: none (per Step 7 zero-@user_verified rule; Unit 9 audits refs, not behaviour).`

### Task 7: Step 8 — Hold branch locally

- [ ] **Step 7.1: If Task 1.5 chose Option A**, the empty `audit/workspace/stale-branches` branch already exists with no commits. Leave it alone. No push, no PR, no further action.

- [ ] **Step 7.2: Optional backup.** Skip — there are zero commits on the audit branch worth bundling. Unit 9's record lives in AUDIT-PLAN.md (which the operator can back up separately).

### Task 8: Step 9 — Close the unit

- [ ] **Step 8.1: Operator review.** Operator reads:
  1. AUDIT-PLAN.md Unit 9 detail section (Per-branch resolution log).
  2. `git branch` output (post-deletion) and confirms the branch list looks right.
  3. `git branch -r` output (post-deletion) and confirms remote list looks right.

  Operator signals "Unit 9 approved" or requests changes.

- [ ] **Step 8.2: Update Unit 9 status in two places.**

In `AUDIT-PLAN.md` catalog table line 66: change to `complete-pending-integration (2026-05-15)`.

In `AUDIT-PLAN.md` Unit 9 detail section heading at line 4623 area: change to `**Status:** complete-pending-integration (2026-05-15)`.

- [ ] **Step 8.3: Add summary entry to "Completed audits" section.**

Find the "Completed audits" section (likely after the Per-unit details section; locate via `grep -n "^## Completed audits" AUDIT-PLAN.md`). Append:

```markdown
### Unit 9 — Stale branches cleanup (complete-pending-integration 2026-05-15)

- **Branch:** `audit/workspace/stale-branches` (empty — zero code commits; reference-only)
- **Plan:** `docs/superpowers/plans/2026-05-15-audit-unit-9-stale-branches.md`
- **ADR:** none (per Step 7 zero-@user_verified rule)
- **Decisions:** see "Per-branch resolution log" in Unit 9 detail section
- **Outcome:** <N> branches deleted (local) + <M> branches deleted (remote); <K> branches kept (active WIP / predecessor-of-audit-branch / bot-managed); 0 branches reclassified to a separate audit unit.
```

Fill in N, M, K from the resolution log.

- [ ] **Step 8.4: Log Process refinements (if any).**

Locate the "Process refinements" section (`grep -n "Process refinements" AUDIT-PLAN.md`) and add any lessons under "From-Unit-9". Candidate observations:
- Whether the empty-audit-branch (Task 1.5 Option A) made sense or felt like ceremony bloat.
- Whether the batched multi-select user-confirmation gate (Task 3 Batch A/B) worked or fragmented attention.
- Whether `git cherry` consistently identified safe-deletes that `git branch -d` then accepted (validates the rubric's rule-5 path).

If nothing notable surfaced, write: `Unit 9: no new process refinements — playbook's hygiene-unit guidance held up.`

## Deviations

Record execution drift here (lean-process convention from `.claude/rules/plan-execution.md`):

- *(empty until execution begins)*

## Out of scope

- **Cleaning up `.worktrees/` directories.** Worktree pruning happens during Final Integration per-branch cleanup step ([AUDIT-PLAN.md:581](../../../AUDIT-PLAN.md#L581)), not here.
- **Closing PR #761.** Explicitly deferred to Final Integration with operator approval gate ([AUDIT-PLAN.md:586](../../../AUDIT-PLAN.md#L586)).
- **Force-pushing or rewriting history on any branch.** Out of scope. Unit 9 only deletes whole branches; per-commit hygiene is not part of this unit.
- **Touching `master`.** Unit 9 does not modify master. Master may be fast-forwarded in Step 1.2 if origin has newer commits, but that's standard sync, not Unit 9 work.
