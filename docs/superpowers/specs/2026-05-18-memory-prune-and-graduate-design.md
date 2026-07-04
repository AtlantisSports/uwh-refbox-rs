# Design Spec — Memory prune-and-graduate (2026-05-18)

## Purpose

Refresh the auto-memory at `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/` so that it
reflects post-v0.4.0 reality: prune entries whose underlying state has resolved, refresh entries
whose facts have shifted, and graduate durable engineering/safety rules out of memory into the
project's `.claude/rules/` system where they belong. This work is preparation for a later session
that aligns `.claude/rules/` and `CLAUDE.md` with patterns observed in the
`uwh-portal/PROJECT-KNOWLEDGE-BASE.md` reference workspace.

## Scope

**In scope this session:**

- The 30 files at `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/`, including the
  `MEMORY.md` index.
- Targeted additions to `.claude/rules/*.md` and the creation of `.claude/rules/patterns.md` as the
  destinations for graduated rules.

**Out of scope this session:**

- Full alignment of `.claude/rules/` and `CLAUDE.md` with portal conventions (Scope Card pattern,
  Absolutes section, After-Edits checklist, Two-Strike, etc.). This is a deliberate follow-up
  session with its own design.
- `docs/decisions/` ADRs and `docs/superpowers/specs|plans/` documents.
- `docs/conventions.md`, `docs/development.md`, `docs/workspace-map.md`, `docs/review-checklist.md`,
  `docs/domain.md`, `docs/scoresheet-styles.md`.
- The duplication of branch/commit-format content between `CLAUDE.md` and `docs/conventions.md`
  (also queued for the rules-alignment session).

## Approach

Two passes:

1. **Prune pass** — delete clearly-stale memory entries; update three entries whose underlying
   facts have shifted but whose principles remain useful.
2. **Graduate pass** — move durable engineering and safety rules out of memory and into
   appropriate `.claude/rules/*.md` files (or create new ones), then delete the corresponding
   memory files.

Each pass closes with a `MEMORY.md` index sync. No code in the main crates is touched.

## Pass 1 — Prune

### Delete (6 files)

| File | Reason |
|------|--------|
| `feedback_prs_deferred_until_audit_done.md` | Self-labelled HISTORICAL; deferral ended 2026-05-15. |
| `project_audit_playbook.md` | Audit complete through Unit 9; ADRs 005, 009, 011 finalized in master. |
| `project_portal_health_session.md` | Unit 7 finalized via commit `e6d5be1`. PR #761 superseded by audit branch. |
| `project_portal_subsystem_dormancy_followup.md` | Merged in commit `170d88f`. |
| `feedback_unit_9_scope_narrow.md` | Unit 9 plan landed in commit `8df6401`; behavioural lesson is too narrow to keep as a session-rule. |
| `project_v040_handover.md` | v0.4.0 is tagged; PR list #803–#839 is stale and authoritative state lives in git. |

### Update (3 files — keep file, refresh body)

| File | Update |
|------|--------|
| `project_adr_016_uwr_in_flight.md` | Branch `feat/refbox/uwr-mode-portal-routing` still un-merged. Refresh wording: remove "Final Integration push" framing (concept retired); state current status as "implementation-complete, walkthrough-verified, awaiting merge decision." Update date to 2026-05-18. |
| `project_unit_10_game_info_layout_pending.md` | Verify `origin/uwh-refbox-game-info-layout` still exists before deciding. If branch exists, refresh wording and date. If branch is gone, move to delete list. |
| `project_recent_prs_untested.md` | Rewrite from a v0.4.0-specific PR list into a generic principle: "Merged ≠ tested at a tournament — back-ported and post-v0.4.0 features have only been verified by review/walkthrough, not real operator use." |

## Pass 2 — Graduate

Each graduation: write the rule into its destination file (preserving intent and "Why/How to apply"
framing), then delete the memory file. After all graduations, prune the corresponding lines from
`MEMORY.md`.

### G1 — Match existing patterns (ROOT RULE)

- **Source:** `feedback_match_existing_patterns.md`
- **Destination:** new file `.claude/rules/patterns.md`
- **Why a new file:** the rule is large and self-contained (read 2–3 siblings before writing new
  code; match theme constants, helpers, naming, layout idioms; deviations require explicit
  operator confirmation). Mirrors portal section 4.20 "Pattern search before implementing,"
  which is also its own concern.
- **Cross-link:** `CLAUDE.md` "Rules Reference" table gets a new row pointing at
  `.claude/rules/patterns.md`.

### G2 — Plan before non-trivial changes

- **Source:** `feedback_plan_before_non_trivial_changes.md`
- **Destination:** `.claude/rules/plan-execution.md` (appended as a new top section
  "When to enter plan mode")
- **Rationale:** the file already discusses lean vs heavy process; "when to plan at all" is the
  same topic at a higher level.

### G3 — Plan mode for UI changes

- **Source:** `feedback_plan_mode_for_ui_changes.md`
- **Destination:** `.claude/rules/plan-execution.md` (same new section as G2, listed as a
  specific case)
- **Rationale:** UI-change rule is a specialization of G2.

### G4 — Smoke test before remote ops

- **Source:** `feedback_smoke_test_before_remote_ops.md`
- **Destination:** `.claude/rules/pr-review.md` ("Before Opening a PR" checklist)
- **Form:** new bullet under "Quality gates" — "Smoke-tested locally: refbox (or affected
  artifact) launched and the change exercised in a browser/app session before any
  push/PR/merge/tag-push; CI green ≠ smoke-tested."

### G5 — Search git log before brainstorming

- **Source:** `feedback_search_git_log_before_brainstorm.md`
- **Destination:** `.claude/rules/workspace.md` (new short section "Before brainstorming
  cross-crate or wire-format work")
- **Form:** retain the example command (`git log --all -S '<symbol>'`) and the worktree-check
  reminder.

### G6 — Back-ports: web is standard

- **Source:** `feedback_backport_web_is_standard.md`
- **Destination:** `.claude/rules/workspace.md` (new short section "Back-ports from `uwh-portal`")
- **Form:** "When back-porting refbox-related code from `uwh-portal`, match the web implementation
  faithfully. Flag any deviation for explicit operator confirmation before applying." Cross-link to
  `reference_uwh_portal_source.md` (which stays in memory as an env reference).

### G7 — Confirm each force-push

- **Source:** `feedback_confirm_each_force_push.md`
- **Destination:** `.claude/rules/communication.md` (Approval Gates section)
- **Form:** new bullet — "Confirm each force-push individually, even within a 'handle it'
  cascade. A blanket approval to clean up a branch does NOT roll up to permission for every
  force-push along the way."

### G8 — Pause on batch destructive ops

- **Source:** `feedback_pause_on_batch_destructive_ops.md`
- **Destination:** `.claude/rules/communication.md` (Approval Gates section)
- **Form:** new bullet — "Before applying a destructive operation (branch delete, worktree
  remove, file delete) across multiple targets in a row, pause and confirm the batch as a whole
  even if each individual operation was implicitly approved."

### Borderlines — stay in memory

- **G9 — `feedback_predictable_ui_over_conditional.md`** stays in memory as a UX preference. Not a
  rule.
- **G10 — `feedback_retarget_then_reopen.md`** stays in memory as an env quirk for the stacked-PR
  workflow on this specific repo's CI.

## Keep list (no change)

Environment / setup:
- `user_screenshot_path.md`
- `feedback_run_command.md`
- `feedback_refbox_wsl_wayland_unset.md`
- `feedback_user_drives_refbox_ui.md`
- `feedback_cd_worktree_before_cargo.md`
- `reference_uwh_portal_source.md`
- `reference_dev_portal_url.md`

User-communication conventions:
- `feedback_does_that_make_sense.md`
- `feedback_one_question_at_a_time.md`
- `feedback_options_with_recommendation.md`
- `feedback_one_action_no_choice.md`
- `feedback_single_recommendation_not_blanket.md`
- `feedback_how_about_now_means_status.md`
- `feedback_visual_companion_scope.md`

Workflow meta:
- `feedback_check_rule_applicability.md`
- `feedback_explore_env_before_deferring.md`

Backlog and borderline (kept per Section 2/3 decisions):
- `backlog_exclusion_with_max_rank.md`
- `feedback_predictable_ui_over_conditional.md` (G9)
- `feedback_retarget_then_reopen.md` (G10)

Updated content (kept file, body refreshed per Pass 1):
- `project_adr_016_uwr_in_flight.md`
- `project_unit_10_game_info_layout_pending.md` (subject to branch-existence verification)
- `project_recent_prs_untested.md`

## MEMORY.md index updates

- Remove the "ROOT RULES" header section entirely (its only entry, Match existing patterns,
  graduates out of memory in G1).
- Remove lines for the six deleted files (Pass 1).
- Remove lines for the eight graduated files (G1–G8).
- Refresh the description lines for the three updated files (Pass 1 updates).
- Expected final index length: ~20 entries, all under "All other memory."

## Cross-file touch-up

After graduations, `feedback_check_rule_applicability.md` references "checking the memory rule's
why." Since several rules are no longer in memory, reword this entry to "checking the relevant
rule's why" (a small wording tweak that keeps the memory itself coherent).

## Acceptance criteria

1. Every file listed in "Delete" no longer exists on disk.
2. Every file listed in "Graduate" no longer exists on disk, AND its content is reflected in the
   designated destination file with intent preserved.
3. `.claude/rules/patterns.md` exists as a new file with the "Match existing patterns" rule.
4. `CLAUDE.md` "Rules Reference" table includes a new row pointing at `.claude/rules/patterns.md`
   with a one-line purpose description.
5. `MEMORY.md` index lists no entries pointing at deleted files; entries pointing at updated files
   have refreshed descriptions; new entries (if any) are present for updated files.
6. `feedback_check_rule_applicability.md` body refers to "rule's why" generically, not "memory
   rule's why."
7. The three updated files have refreshed bodies and current dates (2026-05-18).
8. Work lands on a new branch `chore/workspace/memory-prune-graduate` (after operator approval
   per `.claude/rules/communication.md`) as a small commit series — one commit for Pass 1
   (memory prune), one for Pass 2 (graduations + rules-file changes), and a final commit for
   `MEMORY.md` index sync if it isn't already clean.

## Verification

The work is verifiable by:

1. `ls ~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/` showing the expected file
   set (no deleted/graduated entries, all keep + update entries present).
2. Reading the new and modified `.claude/rules/*.md` files to confirm the graduations landed
   intact.
3. Reading `MEMORY.md` to confirm it accurately indexes the on-disk memory.
4. Spot-check: starting a fresh Claude session and asking "what are the project's rules for
   pattern matching / planning UI work / smoke-testing before remote ops?" — Claude should find
   the answer in the rules system, not in memory.

## Out-of-scope / explicitly deferred

- Aligning the broader `.claude/rules/` and `CLAUDE.md` structure with portal patterns (Scope
  Card, Absolutes, After-Edits, Two-Strike, etc.) — separate session with its own design.
- Eliminating duplication between `CLAUDE.md` and `docs/conventions.md` (branch/commit format
  appears in both) — same separate session.
- Reviewing `docs/decisions/` ADRs for stale "in-flight" status.
- Reviewing `docs/superpowers/specs|plans/` for superseded items.
