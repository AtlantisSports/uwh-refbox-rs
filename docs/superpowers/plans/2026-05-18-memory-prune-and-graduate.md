# Memory Prune and Graduate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prune stale entries from the auto-memory and graduate eight durable engineering/safety rules from memory into the project's `.claude/rules/` system, leaving memory focused on session/env/communication concerns.

**Architecture:** Two-pass workflow. Pass 1 prunes (6 deletes, 3 in-place refreshes) inside the memory directory only. Pass 2 graduates eight memory entries into existing or new `.claude/rules/*.md` files (and updates `CLAUDE.md`'s Rules Reference table), then deletes the source memory files. `MEMORY.md` is kept in sync after each pass. All `.claude/rules/` and `CLAUDE.md` changes go through git; the memory directory lives outside the repo.

**Tech Stack:** Markdown files only. No code changes. `git` for the in-repo changes. The `Read`, `Write`, `Edit`, and `Bash` tools for file operations.

**Reference docs:**
- Spec: `docs/superpowers/specs/2026-05-18-memory-prune-and-graduate-design.md`
- Communication rules (approval gates): `.claude/rules/communication.md`
- Plan-execution rules (lean vs heavy): `.claude/rules/plan-execution.md`

**Process choice:** Lean per `.claude/rules/plan-execution.md` — this is documentation/memory work, no code, no behaviour change, no wire format. Defer ceremony commits.

---

## Task 0: Pre-flight — branch approval and Unit 10 fact-check

**Files:** None modified yet.

- [ ] **Step 1: Verify whether `origin/uwh-refbox-game-info-layout` still exists**

Run:
```bash
git ls-remote origin 'refs/heads/uwh-refbox-game-info-layout' | head -1
git ls-remote origin 'refs/heads/*game-info-layout*' | head -5
```
Expected: a hash + ref line if the branch still exists; empty if it has been deleted.

Record the answer (exists / does not exist). This decides whether `project_unit_10_game_info_layout_pending.md` is **updated** (exists) or **deleted** (does not exist) in Pass 1.

- [ ] **Step 2: Confirm working tree is clean before starting**

Run:
```bash
git status --short
```
Expected: only `refbox-translations.csv` and `refbox-translations.tsv` (the pre-existing untracked files noted at session start), nothing else.

If anything else appears, stop and ask the operator about it.

- [ ] **Step 3: Ask the operator for branch creation approval**

Per `.claude/rules/communication.md`, do not create a branch without explicit operator approval. Surface the proposed branch name and intent:

> "About to create branch `chore/workspace/memory-prune-graduate` for the memory prune + graduation work. Proceed?"

Wait for approval before continuing.

- [ ] **Step 4: Create the branch**

Run:
```bash
git checkout -b chore/workspace/memory-prune-graduate
```
Expected: `Switched to a new branch 'chore/workspace/memory-prune-graduate'`.

- [ ] **Step 5: Commit the design spec onto the new branch**

The spec file `docs/superpowers/specs/2026-05-18-memory-prune-and-graduate-design.md` was written during brainstorming but not committed. Land it now so the plan and spec live together on the branch.

Run:
```bash
git add docs/superpowers/specs/2026-05-18-memory-prune-and-graduate-design.md docs/superpowers/plans/2026-05-18-memory-prune-and-graduate.md
git commit -m "$(cat <<'EOF'
docs(workspace): design and plan for memory prune and graduate

Records the two-pass approach (prune then graduate) for the auto-memory
refresh. Pass 1 deletes six stale tournament/audit memories and
refreshes three. Pass 2 graduates eight durable engineering/safety
rules into the existing .claude/rules/ system. The memory directory
itself is not under version control; only the rule-file edits land in
git.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```
Expected: commit created. `git status` shows clean working tree.

---

## Task 1: Pass 1 — delete six stale memory files

**Files (memory directory; outside the repo):**
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_prs_deferred_until_audit_done.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_audit_playbook.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_portal_health_session.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_portal_subsystem_dormancy_followup.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_unit_9_scope_narrow.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_v040_handover.md`

- [ ] **Step 1: Delete the six files**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_prs_deferred_until_audit_done.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_audit_playbook.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_portal_health_session.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_portal_subsystem_dormancy_followup.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_unit_9_scope_narrow.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_v040_handover.md
```
Expected: no output.

- [ ] **Step 2: Verify the deletes**

Run:
```bash
ls ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/ | grep -E 'prs_deferred|audit_playbook|portal_health_session|portal_subsystem_dormancy|unit_9_scope|v040_handover' || echo 'all gone'
```
Expected: `all gone`.

---

## Task 2: Pass 1 — refresh `project_adr_016_uwr_in_flight.md`

**Files:**
- Modify: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_adr_016_uwr_in_flight.md`

- [ ] **Step 1: Read current content**

Read the file to see current frontmatter and body.

- [ ] **Step 2: Rewrite body**

Replace the body so it reads (preserving the frontmatter and `name:`/`description:`/`metadata:` block):

```markdown
ADR 016 (UWR mode portal routing) implementation is complete and operator-walkthrough-verified;
the feature branch `feat/refbox/uwr-mode-portal-routing` is **not yet merged to master**.

**Why:** The audit's "Final Integration push" concept is retired (the audit closed at Unit 9).
The UWR-mode work was the last item still riding that workflow; it now stands alone as a
finished feature awaiting a merge decision.

**How to apply:** Treat the UWR-mode branch as a normal-cadence PR candidate. Do not assume
it has merged — verify with `git log master --oneline | grep -i uwr` or check the worktree at
`.worktrees/uwr-mode-portal-routing/` before acting on UWR-related assumptions.

Branch: `feat/refbox/uwr-mode-portal-routing`
Worktree (if present): `.worktrees/uwr-mode-portal-routing/`
Status as of 2026-05-18: implementation-complete, walkthrough-verified, awaiting merge.
```

Update `description:` in the frontmatter to:
`description: ADR 016 UWR mode portal routing — implementation complete, walkthrough-verified, branch not yet merged to master as of 2026-05-18`

- [ ] **Step 3: Verify the refresh**

Read the file and confirm:
- No "Final Integration push" awaited / pending phrasing
- Date 2026-05-18 appears
- Branch path is correct
- Frontmatter description matches

---

## Task 3: Pass 1 — refresh `project_unit_10_game_info_layout_pending.md` OR delete it

**Files:**
- Modify or delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_unit_10_game_info_layout_pending.md`

Decision tree based on Task 0 Step 1 result:

- [ ] **Step 1 (branch exists): Refresh body**

If `origin/uwh-refbox-game-info-layout` was found, rewrite the body:

```markdown
Unit 10 (game-info-layout audit) is deferred. The legacy branch `origin/uwh-refbox-game-info-layout`
remains on the remote, reclassified from "delete" to "audit-separately."

**Why:** When the audit playbook closed at Unit 9, this branch was still un-triaged. It contains
work that pre-dates the audit framework and needs its own design pass.

**How to apply:** Treat Unit 10 as queued work, not in-flight work. Before scheduling it, verify
the branch still exists on the remote (`git ls-remote origin 'refs/heads/uwh-refbox-game-info-layout'`)
and that nothing has overtaken it.

Status as of 2026-05-18: branch exists on origin, no active worktree, no scheduled session.
```

Update frontmatter `description:` to:
`description: Unit 10 game-info-layout audit deferred; branch origin/uwh-refbox-game-info-layout exists on remote as of 2026-05-18`

- [ ] **Step 2 (branch does NOT exist): Delete the file**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_unit_10_game_info_layout_pending.md
```

Only one of Step 1 or Step 2 executes — pick based on Task 0 Step 1.

---

## Task 4: Pass 1 — refresh `project_recent_prs_untested.md`

**Files:**
- Modify: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_recent_prs_untested.md`

- [ ] **Step 1: Rewrite as a generic principle**

Replace body with:

```markdown
Merged ≠ tested at a tournament. Code that landed during the post-v0.4.0 polish window and
during audit Unit verification has been **reviewed and walkthrough-verified**, but in most cases
has not been exercised at a live tournament with a real operator and real referee equipment.

**Why:** v0.4.0 shipped before all of the polish/bug-fix work had a chance to face a tournament.
The audit also produced behaviour adjustments that were verified by Gherkin scenarios and
walkthroughs, not by tournament use. There is a real gap between "CI green + walkthrough OK"
and "field-validated."

**How to apply:** When discussing recently-merged features (anything since v0.4.0), state that
they are tournament-untested by default. Do not claim a feature "works in practice" without
actual tournament evidence. The next tournament will produce that evidence; until then, treat
the field-validation state as Unknown.
```

Update frontmatter `description:` to:
`description: Post-v0.4.0 features and audit-verified behaviour have been reviewed/walkthrough-checked but not exercised at a tournament — field-validation state is Unknown until next event`

- [ ] **Step 2: Verify the rewrite**

Read the file and confirm:
- No specific PR numbers in the body (the principle is now generic)
- Date 2026-05-18 appears
- "Unknown until next event" framing is present

---

## Task 5: Pass 1 — MEMORY.md index sync

**Files:**
- Modify: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/MEMORY.md`

- [ ] **Step 1: Read current MEMORY.md**

- [ ] **Step 2: Edit the index**

Apply the following edits (use `Edit` per line for safety; do not rewrite the file wholesale):

Remove these lines:
- `- [PRs deferred until audit done — HISTORICAL](feedback_prs_deferred_until_audit_done.md) — Deferral ended 2026-05-15 with Final Integration push. Normal-cadence PRs are the practice again; file kept for historical context only.`
- `- [Audit playbook in progress](project_audit_playbook.md) — AI-code audit via gitignored AUDIT-PLAN.md; Gherkin scenarios for UI behaviour, two-document model (decisions in playbook, test status in \`docs/audit-scenarios/\`), per-unit plans via \`superpowers:writing-plans\``
- `- [Portal health indicator PR #761 open](project_portal_health_session.md) — ADR 011 PR still open at #761 but superseded by \`audit/refbox/portal-health\` (Unit 7 complete 2026-05-15) at Final Integration`
- `- [Portal subsystem dormancy follow-up](project_portal_subsystem_dormancy_followup.md) — After portal health PR merges, separate branch aligns whole subsystem with "dormant until Using UWH Portal enabled"`
- `- [Unit 9 cleanup scope is narrow](feedback_unit_9_scope_narrow.md) — Only audit branches + direct predecessors; leave schedule-processor, unfinished ADRs, partly-implemented features, and legacy branches alone`
- `- [Post-v0.4.0 backlog](project_v040_handover.md) — 2026-05-16 session: 9 polish/bug PRs (#803–#839) all CI-green awaiting operator review; #828/#838 share a toggle-handler arm and conflict (trivial resolution); v0.4.0 retag pending operator merges; audit-lesson graduation queued.`

Refresh these lines (replacement, not removal):

Replace:
`- [ADR 016 UWR complete](project_adr_016_uwr_in_flight.md) — feat/refbox/uwr-mode-portal-routing implementation-complete and walkthrough-verified 2026-05-15; awaiting Final Integration push`
With:
`- [ADR 016 UWR awaiting merge](project_adr_016_uwr_in_flight.md) — feat/refbox/uwr-mode-portal-routing implementation-complete and walkthrough-verified as of 2026-05-18; branch not merged to master`

Replace:
`- [Recent PRs untested](project_recent_prs_untested.md) — Merged ≠ tested; post-v0.4.0 PR'd code has only been reviewed, not exercised at a tournament`
With:
`- [Merged ≠ tournament-tested](project_recent_prs_untested.md) — Post-v0.4.0 and audit-verified features have been reviewed/walkthrough-checked but not field-validated; tournament-untested by default until next event`

For the Unit 10 line:
- If Task 3 took Step 1 (refresh): replace
  `- [Unit 10 game-info-layout pending](project_unit_10_game_info_layout_pending.md) — origin/uwh-refbox-game-info-layout reclassified from delete to audit-separately 2026-05-15; Unit 10 deferred to its own session after Final Integration`
  with
  `- [Unit 10 game-info-layout deferred](project_unit_10_game_info_layout_pending.md) — origin/uwh-refbox-game-info-layout exists on remote as of 2026-05-18; queued for its own audit session, no active worktree`
- If Task 3 took Step 2 (delete): remove the line entirely.

- [ ] **Step 3: Verify the index**

Read MEMORY.md and check:
- Six lines removed (or seven if Unit 10 was deleted)
- Three refreshed lines present with new descriptions
- No dangling references to deleted files

---

## Task 6: Pass 1 — commit (memory-only, no git effect)

**Note:** the memory directory is OUTSIDE the repo. The Pass 1 changes leave no trace in `git status`. There is nothing to commit for Pass 1 work.

- [ ] **Step 1: Confirm git is clean**

Run:
```bash
git status --short
```
Expected: same untracked files as Task 0 Step 2 (`refbox-translations.csv`, `refbox-translations.tsv`). No staged or modified files.

- [ ] **Step 2: Spot-check memory state**

Run:
```bash
ls ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/ | wc -l
```
Expected: a number between 22 and 25 (was 31 including MEMORY.md; deleted 6 in Task 1 + possibly 1 in Task 3 = 24 or 25 files remaining after Pass 1, before Pass 2 graduations begin).

---

## Task 7: Pass 2 — G1: Match existing patterns → new `.claude/rules/patterns.md`

**Files:**
- Create: `.claude/rules/patterns.md`
- Modify: `CLAUDE.md` (Rules Reference table)
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_match_existing_patterns.md`

- [ ] **Step 1: Read source memory file**

Read `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_match_existing_patterns.md` to preserve intent and exact wording.

- [ ] **Step 2: Create `.claude/rules/patterns.md`**

Write to `.claude/rules/patterns.md`:

```markdown
# Pattern Matching

Before writing new code, find and match the patterns already in use. This is the highest-priority
rule in this workspace: a small change that follows existing conventions is always better than a
"clean" change that diverges from them.

## The Rule

**Before writing any new function, view, or component, read 2–3 sibling implementations** in the
same crate/module/directory. Match their conventions on:

- Theme constants — `SPACING`, `PADDING`, `MIN_BUTTON_SIZE`, `LINE_HEIGHT`, etc. Never introduce
  a magic number where a theme constant already exists.
- Helper functions — if a helper exists for what you need (`make_*_button`, `confirm_pause`,
  `current_pool_color`), use it. Do not write a parallel implementation.
- Naming idioms — match the casing, abbreviation style, and word choice already in use. A new
  field named `confirm_score_pause_remaining` belongs next to `confirm_pause_remaining`, not as
  a free-standing `score_confirm_pause`.
- Layout idioms — column/row composition, spacing rules, button grouping. Mimic the layout of
  a comparable existing view, do not re-design it.

## Why

The codebase has accumulated taste over many tournament cycles. The operator (who runs the
sessions but does not write code) recognises and depends on consistency in the UI. Diverging
from established patterns introduces visible inconsistencies that take operator attention away
from the game.

This rule is what prevents the same problem from being solved three different ways across
the codebase, and what keeps the refbox feeling like one tool rather than a stack of
contributions.

## How to apply

1. Identify what you are about to build — a button, a view, a state machine, a field, a helper.
2. Find 2–3 sibling implementations of the same kind nearby. For UI work, that means other views
   in the same screen family. For state work, other places that mutate the same kind of state.
3. Read them. Note their conventions.
4. Write yours to match. If you find yourself reaching for a magic number, stop and search for
   the corresponding theme constant.

## Deviations require explicit approval

If you have a real reason to deviate from an established pattern, surface it before
implementation:

> "I'm about to deviate from the pattern in `<file>:<lines>`. The deviation is `<X>`. The reason
> is `<Y>`. Proceed?"

Wait for explicit operator confirmation. Do not silently diverge.
```

- [ ] **Step 3: Add row to CLAUDE.md Rules Reference table**

Edit `CLAUDE.md` Rules Reference table. After the existing rows, the new row goes in
alphabetical-by-filename order:

Locate:
```
| `.claude/rules/scope.md` | No scope creep — flag before acting |
| `.claude/rules/communication.md` | Plain English; approval gates |
```

Insert the `patterns.md` row at the top (since it's the highest-priority rule). The block becomes:

```
| `.claude/rules/patterns.md` | Match existing patterns before writing new code — ROOT RULE |
| `.claude/rules/scope.md` | No scope creep — flag before acting |
| `.claude/rules/communication.md` | Plain English; approval gates |
```

- [ ] **Step 4: Delete the source memory file**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_match_existing_patterns.md
```

- [ ] **Step 5: Verify**

- Read `.claude/rules/patterns.md` — confirm content.
- Read `CLAUDE.md` Rules Reference table — confirm new row present.
- `ls ~/.claude/.../memory/ | grep match_existing` should return nothing.

---

## Task 8: Pass 2 — G2 + G3: Plan-mode rules → `.claude/rules/plan-execution.md`

**Files:**
- Modify: `.claude/rules/plan-execution.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_plan_before_non_trivial_changes.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_plan_mode_for_ui_changes.md`

- [ ] **Step 1: Read source memory files**

Read both files for intent and exact wording.

- [ ] **Step 2: Add a new top section to `.claude/rules/plan-execution.md`**

Insert immediately after the title line (`# Plan Execution`) and before the existing
`## Default process (lean)` section:

```markdown
## When to enter plan mode

Before starting *any* of the following, enter plan mode (`EnterPlanMode`) and produce a written
plan via `superpowers:writing-plans`. Quick diagnosis does **not** equal trivial fix — diagnosing
the cause of a bug in five minutes does not mean fixing it is also a five-minute job.

**Always plan first:**

- Any change touching `uwh-common` (shared types, wire format, serialization).
- Any change to `wireless-remote` (embedded firmware — see `embedded.md`).
- Any change crossing two or more crates.
- Any change to a state machine (game clock, tournament manager, penalty tracking).
- Any UI change involving layout, button placement, sizing, spacing, or operator-visible
  behaviour. Operator-visible-behaviour hints in user requests (e.g., "for the time being")
  often carry design context that needs to be captured in writing.

**Skip planning only when:**

- The change is a one-line correction (typo, obvious bug) AND
- It is confined to a single file AND
- The diff is verifiable by reading.

When in doubt, plan. The cost of a quick plan is small; the cost of unplanned cross-crate work
is reverting half-finished change.

```

- [ ] **Step 3: Delete the source memory files**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_plan_before_non_trivial_changes.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_plan_mode_for_ui_changes.md
```

- [ ] **Step 4: Verify**

Read `.claude/rules/plan-execution.md` — confirm new section is present and the existing
`## Default process (lean)` follows below it.

---

## Task 9: Pass 2 — G4: Smoke test → `.claude/rules/pr-review.md`

**Files:**
- Modify: `.claude/rules/pr-review.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_smoke_test_before_remote_ops.md`

- [ ] **Step 1: Read source memory file**

- [ ] **Step 2: Add to "Quality gates" checklist**

In `.claude/rules/pr-review.md`, locate the "Quality gates" sub-list under "## Before Opening a PR":

```
**Quality gates:**
- [ ] `just check` passes locally (fmt, lint, tests, audit — all clean)
- [ ] No files changed outside the stated scope
- [ ] No `unwrap()` or `expect()` added without justification
- [ ] No new dependencies added without discussion
```

Add one bullet at the top of that list:

```
- [ ] **Smoke-tested locally** — refbox (or the affected artifact) was launched and the change exercised in a real session before any push/PR/merge/tag-push. CI green ≠ smoke-tested.
```

The block becomes:
```
**Quality gates:**
- [ ] **Smoke-tested locally** — refbox (or the affected artifact) was launched and the change exercised in a real session before any push/PR/merge/tag-push. CI green ≠ smoke-tested.
- [ ] `just check` passes locally (fmt, lint, tests, audit — all clean)
- [ ] No files changed outside the stated scope
- [ ] No `unwrap()` or `expect()` added without justification
- [ ] No new dependencies added without discussion
```

- [ ] **Step 3: Delete the source memory file**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_smoke_test_before_remote_ops.md
```

- [ ] **Step 4: Verify**

Read `.claude/rules/pr-review.md` — confirm the smoke-test bullet is at the top of Quality gates.

---

## Task 10: Pass 2 — G5 + G6: Workspace navigation rules → `.claude/rules/workspace.md`

**Files:**
- Modify: `.claude/rules/workspace.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_search_git_log_before_brainstorm.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_backport_web_is_standard.md`

- [ ] **Step 1: Read both source memory files**

- [ ] **Step 2: Add two new sections to `.claude/rules/workspace.md`**

Append at the end of the file (after the existing "## Multi-Crate Changes" section):

```markdown
## Before brainstorming cross-crate or wire-format work

Before brainstorming a fix or feature that touches `uwh-common`, the wire format, or any concept
that may already be under active development on another branch, **search the git log and
worktree list first.** Past lessons (notably audit Unit 5) cost ~30 minutes of redundant spec
work when parallel branch activity was not discovered up front.

Use:

```bash
git log --all -S '<symbol-or-concept>' --oneline | head -20
git worktree list
git branch -a --list 'feat/*' 'fix/*' 'audit/*' | head -30
```

If a similar concept is already in flight on another branch, pause and ask the operator how
the work should relate (continue that branch, supersede it, or fold the change in).

## Back-ports from `uwh-portal`

The web refbox at `/home/estraily/projects/uwh-portal/` is the standard for any refbox-related
feature that exists on both surfaces. When back-porting from the web refbox to this Rust refbox:

- **Match the web implementation faithfully.** Same data flow, same field names where possible,
  same operator-facing behaviour.
- **Flag every deviation explicitly.** If the Rust implementation needs to differ from the web
  reference (because of `iced` constraints, no-std requirements, or a refbox-specific behaviour
  the web version does not have), surface the deviation and the reason before implementing.
- **Cross-reference the source.** Quote the web file path (e.g.,
  `uwh-portal/js/@underwater-ui/refbox/.../X.tsx:nnn-mmm`) in the design spec or implementation
  plan so the back-port can be audited later.

See also the env-reference memory entry `reference_uwh_portal_source.md` for the canonical
locations of the web refbox source tree.
```

- [ ] **Step 3: Delete the source memory files**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_search_git_log_before_brainstorm.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_backport_web_is_standard.md
```

- [ ] **Step 4: Verify**

Read `.claude/rules/workspace.md` — confirm both new sections are present.

---

## Task 11: Pass 2 — G7 + G8: Safety rules → `.claude/rules/communication.md`

**Files:**
- Modify: `.claude/rules/communication.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_confirm_each_force_push.md`
- Delete: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_pause_on_batch_destructive_ops.md`

- [ ] **Step 1: Read both source memory files**

- [ ] **Step 2: Extend the Approval Gates section**

In `.claude/rules/communication.md`, locate the "## Approval Gates" section. After the existing
`**Always ask before:**` bullet list and the paragraph that follows it, append:

```markdown
### Per-operation confirmation within cascades

A blanket approval to "clean up this branch" or "handle this PR" does NOT roll up to permission
for every individual destructive operation along the way.

- **Force-push:** confirm each force-push individually, even within a cascade. Surface the exact
  branch and target ref, then wait for explicit go-ahead before pushing.
- **Batch destructive ops:** before applying the same destructive operation (branch delete,
  worktree remove, file delete) across multiple targets in a row, pause and confirm the batch
  as a whole — even if each individual operation was implicitly approved. State the list
  ("about to delete branches A, B, C") and wait for confirmation.

The reason: cascades feel approved because the parent task was approved, but each individual
write is its own change with its own blast radius. The operator may want to skip or alter one
mid-cascade.
```

- [ ] **Step 3: Delete the source memory files**

Run:
```bash
rm ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_confirm_each_force_push.md \
   ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_pause_on_batch_destructive_ops.md
```

- [ ] **Step 4: Verify**

Read `.claude/rules/communication.md` — confirm the new "Per-operation confirmation within
cascades" subsection appears under Approval Gates.

---

## Task 12: Pass 2 — MEMORY.md index sync for graduations

**Files:**
- Modify: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/MEMORY.md`

- [ ] **Step 1: Remove the eight graduation lines and the ROOT RULES section**

Apply these edits to `MEMORY.md`:

**1a.** Remove the entire `## ROOT RULES (never bypassed)` section, including its heading and the
single bullet underneath it referencing `feedback_match_existing_patterns.md`. After this edit,
the file should go from the `# Memory Index` title directly to `## All other memory`.

**1b.** Remove every index line that references a graduated source file. Identify by the
filename inside the link's parentheses; remove the whole bullet line. The filenames to remove are:
- `feedback_match_existing_patterns.md` (already removed with 1a if it was only in ROOT RULES — check)
- `feedback_plan_before_non_trivial_changes.md`
- `feedback_plan_mode_for_ui_changes.md`
- `feedback_smoke_test_before_remote_ops.md`
- `feedback_search_git_log_before_brainstorm.md`
- `feedback_backport_web_is_standard.md`
- `feedback_confirm_each_force_push.md`
- `feedback_pause_on_batch_destructive_ops.md`

A robust way to find them: `grep -n 'feedback_match_existing_patterns\|feedback_plan_before_non_trivial\|feedback_plan_mode_for_ui\|feedback_smoke_test_before_remote\|feedback_search_git_log_before_brainstorm\|feedback_backport_web_is_standard\|feedback_confirm_each_force_push\|feedback_pause_on_batch_destructive_ops' ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/MEMORY.md` — every line returned should be removed.

Note: the `feedback_pause_on_batch_destructive_ops.md` file exists on disk per the directory
listing in Task 0 evidence (memory ls showed it). If it does not currently appear in `MEMORY.md`,
no removal is needed for that filename — the grep will simply not match it.

- [ ] **Step 2: Verify index integrity**

Run:
```bash
grep -o '(\([^)]*\.md\))' ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/MEMORY.md | sort -u > /tmp/memory-index-refs.txt
ls ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/*.md | sed 's|.*/||' | sort -u > /tmp/memory-files.txt
diff /tmp/memory-index-refs.txt <(awk '{print "("$0")"}' /tmp/memory-files.txt) | head -30
```

Expected: the diff should show only `MEMORY.md` (which is not indexed by itself) — every other
referenced file should exist on disk, and every file on disk should appear in the index.

Investigate any other diff lines before proceeding.

---

## Task 13: Cross-file touch-up — `feedback_check_rule_applicability.md`

**Files:**
- Modify: `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/feedback_check_rule_applicability.md`

- [ ] **Step 1: Read the file**

- [ ] **Step 2: Update wording**

The file currently refers to "the memory rule's why." Replace that wording with "the relevant
rule's why" (because several rules are no longer in memory after Pass 2).

Specifically, change:
- "check the memory rule's why actually applies" → "check the relevant rule's why actually applies"
- any other instances of "memory rule" used in the same sense → "rule"

Leave the rest of the file untouched.

- [ ] **Step 3: Verify**

Read the file and confirm the wording is now generic.

---

## Task 14: Commit the Pass 2 in-repo changes

**Files staged for commit:**
- `CLAUDE.md`
- `.claude/rules/patterns.md` (new)
- `.claude/rules/plan-execution.md`
- `.claude/rules/pr-review.md`
- `.claude/rules/workspace.md`
- `.claude/rules/communication.md`

(The memory directory changes do not touch git.)

- [ ] **Step 1: Stage the files**

Run:
```bash
git add CLAUDE.md \
        .claude/rules/patterns.md \
        .claude/rules/plan-execution.md \
        .claude/rules/pr-review.md \
        .claude/rules/workspace.md \
        .claude/rules/communication.md
```

- [ ] **Step 2: Confirm the diff**

Run:
```bash
git diff --staged --stat
```
Expected: six files changed, one new file (`patterns.md`), modest line counts.

Also:
```bash
git diff --staged
```
Spot-check each hunk against the corresponding plan task.

- [ ] **Step 3: Ask the operator for commit approval**

Surface the diff summary and proposed commit message. Wait for approval before committing.

Proposed message:

```
chore(workspace): graduate durable rules from memory into .claude/rules/

Move eight long-running engineering and safety rules out of the auto-memory
and into the project's .claude/rules/ system, where they live alongside
other workspace conventions and are visible at the start of every session.

- New .claude/rules/patterns.md captures the "Match existing patterns"
  ROOT RULE that was previously memory-only.
- .claude/rules/plan-execution.md gains a new "When to enter plan mode"
  section consolidating the "plan before non-trivial changes" and "plan
  mode for UI changes" memories.
- .claude/rules/pr-review.md gains a smoke-test bullet under Quality gates.
- .claude/rules/workspace.md gains sections for "search the git log before
  brainstorming cross-crate work" and "back-ports from uwh-portal."
- .claude/rules/communication.md gains per-operation confirmation rules
  for force-pushes and batch destructive operations.
- CLAUDE.md Rules Reference table gains a row for the new patterns.md.

Memory now stays focused on session, environment, and communication
concerns. The portal-conventions alignment is queued for a separate
session.
```

- [ ] **Step 4: Commit (after approval)**

Run:
```bash
git commit -m "$(cat <<'EOF'
chore(workspace): graduate durable rules from memory into .claude/rules/

Move eight long-running engineering and safety rules out of the auto-memory
and into the project's .claude/rules/ system, where they live alongside
other workspace conventions and are visible at the start of every session.

- New .claude/rules/patterns.md captures the "Match existing patterns"
  ROOT RULE that was previously memory-only.
- .claude/rules/plan-execution.md gains a new "When to enter plan mode"
  section consolidating the "plan before non-trivial changes" and "plan
  mode for UI changes" memories.
- .claude/rules/pr-review.md gains a smoke-test bullet under Quality gates.
- .claude/rules/workspace.md gains sections for "search the git log before
  brainstorming cross-crate work" and "back-ports from uwh-portal."
- .claude/rules/communication.md gains per-operation confirmation rules
  for force-pushes and batch destructive operations.
- CLAUDE.md Rules Reference table gains a row for the new patterns.md.

Memory now stays focused on session, environment, and communication
concerns. The portal-conventions alignment is queued for a separate
session.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5: Verify the commit**

Run:
```bash
git log --oneline -3
git status
```
Expected: commit appears in the log; working tree clean.

---

## Task 15: Final verification

- [ ] **Step 1: Memory directory state**

Run:
```bash
ls ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/ | sort
```

Expected presence (the keep + updated set):
- `MEMORY.md`
- `backlog_exclusion_with_max_rank.md`
- `feedback_cd_worktree_before_cargo.md`
- `feedback_check_rule_applicability.md` (touched in Task 13)
- `feedback_does_that_make_sense.md`
- `feedback_explore_env_before_deferring.md`
- `feedback_how_about_now_means_status.md`
- `feedback_one_action_no_choice.md`
- `feedback_one_question_at_a_time.md`
- `feedback_options_with_recommendation.md`
- `feedback_predictable_ui_over_conditional.md`
- `feedback_refbox_wsl_wayland_unset.md`
- `feedback_retarget_then_reopen.md`
- `feedback_run_command.md`
- `feedback_single_recommendation_not_blanket.md`
- `feedback_user_drives_refbox_ui.md`
- `feedback_visual_companion_scope.md`
- `project_adr_016_uwr_in_flight.md` (refreshed)
- `project_recent_prs_untested.md` (refreshed)
- `reference_dev_portal_url.md`
- `reference_uwh_portal_source.md`
- `user_screenshot_path.md`
- Conditional: `project_unit_10_game_info_layout_pending.md` (only if Task 3 kept it)

Expected absence (the delete + graduate set, 14 files):
- `feedback_backport_web_is_standard.md`
- `feedback_confirm_each_force_push.md`
- `feedback_match_existing_patterns.md`
- `feedback_pause_on_batch_destructive_ops.md`
- `feedback_plan_before_non_trivial_changes.md`
- `feedback_plan_mode_for_ui_changes.md`
- `feedback_prs_deferred_until_audit_done.md`
- `feedback_search_git_log_before_brainstorm.md`
- `feedback_smoke_test_before_remote_ops.md`
- `feedback_unit_9_scope_narrow.md`
- `project_audit_playbook.md`
- `project_portal_health_session.md`
- `project_portal_subsystem_dormancy_followup.md`
- `project_v040_handover.md`

- [ ] **Step 2: Rules-file content spot-check**

Open and skim each modified rules file. Confirm the new sections read well and the existing
content is undisturbed:

- `CLAUDE.md` — Rules Reference table has a `patterns.md` row.
- `.claude/rules/patterns.md` — full content present.
- `.claude/rules/plan-execution.md` — new top section before "Default process (lean)".
- `.claude/rules/pr-review.md` — smoke-test bullet at top of Quality gates.
- `.claude/rules/workspace.md` — two new sections appended.
- `.claude/rules/communication.md` — Per-operation confirmation subsection present.

- [ ] **Step 3: Index integrity recheck**

Re-run the diff command from Task 12 Step 2. Expected: clean.

- [ ] **Step 4: Fresh-session spot test (optional but recommended)**

Open a new Claude session (or simulate by re-reading CLAUDE.md) and confirm that asking "what is
the rule for matching existing patterns?" surfaces `.claude/rules/patterns.md` content, not a
memory lookup.

This is a soft check — useful but not blocking.

---

## Done

After Task 15 passes:

- Memory contains ~22 entries focused on session/env/communication concerns.
- `.claude/rules/` contains the eight graduated rules in their natural homes.
- A single feature branch `chore/workspace/memory-prune-graduate` holds two commits (design+plan, then graduations).
- No code in any crate has been touched.

The "rules-and-CLAUDE.md alignment with portal patterns" work is queued for a separate session.
