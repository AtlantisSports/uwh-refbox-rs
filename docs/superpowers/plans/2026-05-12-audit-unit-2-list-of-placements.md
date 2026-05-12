# AI Code Audit — Unit 2 — ListOfPlacements + SeededBy

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` (subagent-driven execution is the default now that Unit 1's rehearsal is complete). Each task is dispatched to a fresh subagent by the principal; critical keep/delete decisions come back to the user via the principal. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the ListOfPlacements + SeededBy.group type change (commits `6907ef8` and `803d985`) to the same level of intentional design as hand-coded work — by cataloging every distinct behaviour the diff added across three crates, having the user decide keep/delete on each, surgically pruning anything unwanted, exercising the kept behaviour across every consumer of `uwh-common`, and capturing the result in a retroactive ADR.

**Architecture:** This plan decomposes Steps 1–9 of `AUDIT-PLAN.md`'s per-unit workflow for Unit 2's specific scope. **Unit 2 is heavy-process** because the diff touches `uwh-common` — the shared types crate with full-project blast radius. Audit is performed in an isolated git worktree on a new `audit/uwh-common/list-of-placements` branch cut from `master`. Branch stays local through Step 9. AUDIT-PLAN.md is gitignored — edits there are file-on-disk only, not commits. Per-task deviations from this plan get their own `docs(workspace): record Task N deviations` commits (heavy-process discipline, NOT lean).

**Tech Stack:** Rust 1.85 / edition 2024. Affected crates: `uwh-common` (the shared types library), `schedule-processor` (CLI consumer of `uwh-common`), `refbox` (GUI consumer). Indirect consumers to verify via `just check`: `overlay`, `led-panel-sim`, `matrix-drawing`. `serde` / `serde_json` for the wire format. `wireless-remote` is on a separate toolchain and out of scope (the changed types are not used there).

**Testing approach:**
- Unit-level: existing `uwh-common::uwhportal::schedule` tests (in `uwh-common/src/uwhportal/schedule.rs` `tests` module — see `test_serialize_seeded_by`, `test_deserialize_seeded_by`, `test_serialize_list_of_games_final_results`, etc.) must pass with the new types. The audit considers adding a serde round-trip test for `ListOfPlacements` if one is missing (it is — `test_serialize_list_of_placements_final_results` / `test_deserialize_list_of_placements_final_results` do not exist on master).
- Cross-crate: `just check` runs every crate's tests in one sweep, but for heavy-process discipline the plan also has explicit per-consumer verification tasks so any cross-crate issue is isolated, not papered over.
- End-to-end: `schedule-processor` exercised against a real placements-using schedule (e.g. a portal export that contains a `ListOfPlacements` block). The audit's headline downstream use is `schedule-processor`'s scoresheet generation, so this is the primary integration test.
- Regression: a focused test for the `single_half` detection fix (B2.5) — the original commit fixed a real bug but did NOT add a regression test for it. If kept, the audit should add one.

---

## Acceptance criteria (Unit 2 "complete-pending-integration")

Mapped to `AUDIT-PLAN.md` "Definition of audit-unit-done":

- [ ] Every catalog entry (anticipated B2.1–B2.8, possibly more from the slop sweep) has a final Decision tag — `@user_verified` or `@deleted`. No `@proposed` remaining.
- [ ] Any code marked `@deleted` is removed and committed on the audit branch.
- [ ] `just check` passes on the audit branch (fmt-check, clippy `-D warnings`, all tests pass, `cargo audit` clean).
- [ ] Every consumer crate of `uwh-common` has been built and its tests run cleanly **as a separate verification step** (not just via `just check`). Crates: `refbox`, `schedule-processor`, `overlay`, `led-panel-sim`, `matrix-drawing`.
- [ ] `schedule-processor` has been exercised against a real placements-using schedule, and the output looked at by the user — recorded in the unit notes.
- [ ] If B2.5 (`single_half` fix) is kept, a focused regression test for it has been added in the appropriate test module and passes; demonstrably fails when the fix is reverted.
- [ ] Backend behaviours' test status is captured in the retroactive ADR's prose. Unit 2 has NO scenarios (pure backend type change per AUDIT-PLAN.md applicability table), so no `.feature` file is seeded.
- [ ] Retroactive ADR `docs/decisions/020-list-of-placements.md` (or next-available number) is written and committed.
- [ ] Branch is **not pushed** to remote. No PR is opened.
- [ ] User has reviewed the decision log in AUDIT-PLAN.md, signalled approval.
- [ ] Any playbook refinements surfaced during execution are logged in AUDIT-PLAN.md's "Process refinements log" → "From Unit 2".

---

## Prerequisites

- The user has approved this per-unit plan before any Task 1 step runs.
- Working tree on `docs/workspace/backlog-adrs` (or whichever branch the principal currently sits on) is clean — uncommitted state stays untouched by the worktree creation.
- `git fetch origin master` is current.
- Unit 1's audit branch (`audit/refbox/confirm-score-timing`) remains local in its own worktree at `.worktrees/audit-unit-1-confirm-score/`. Unit 2's worktree is independent.
- Pre-commit hook at `<main-repo>/.git/hooks/pre-commit` allows `audit/` branch type (fixed by Unit 1's commit `2a8dcbc`). If a fresh checkout, run `cp scripts/pre-commit .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit` from the main repo first.

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:** worktree creation + AUDIT-PLAN.md edit (gitignored, no commit).

- [ ] **Step 1: Confirm working tree is clean.**

  Run: `git -C /home/estraily/projects/uwh-refbox-rs status`
  Expected: no uncommitted changes to tracked files (untracked `.claude/scheduled_tasks.lock` is fine).

- [ ] **Step 2: Ask the user for explicit approval to cut the audit branch in a worktree.**

  Surface the branch name and worktree path:
  - Branch: `audit/uwh-common/list-of-placements`
  - Worktree: `.worktrees/audit-unit-2-list-of-placements/`

- [ ] **Step 3: Create the worktree on a fresh `master`.**

  Run from main repo root:
  ```bash
  git fetch origin master
  git worktree add -b audit/uwh-common/list-of-placements .worktrees/audit-unit-2-list-of-placements origin/master
  ```
  Expected: `Preparing worktree (new branch 'audit/uwh-common/list-of-placements')`.

- [ ] **Step 4: Verify the worktree HEAD contains both audit commits.**

  Run: `cd .worktrees/audit-unit-2-list-of-placements && git log --oneline 6907ef8 803d985 -2`
  Expected: both commits appear (they merged to master before this audit started).

- [ ] **Step 5: Update Unit 2 status in `AUDIT-PLAN.md`.**

  Edit Unit 2's row in the "Audit unit catalog" table AND the section heading near the bottom: change status from `not started` to `in progress (started YYYY-MM-DD)`. Add `**Per-unit plan:** docs/superpowers/plans/2026-05-12-audit-unit-2-list-of-placements.md` and the worktree path to the unit detail section. File edit only; no commit.

---

## Task 2: Generate the diff (AUDIT-PLAN.md Step 2)

**Files:** AUDIT-PLAN.md edit only.

- [ ] **Step 1: Identify the merge point.**

  Run (from worktree): `git log --oneline 6907ef8~1 -1`
  Record this SHA as `<base>`.

- [ ] **Step 2: File list for both audited commits.**

  Run: `git diff --name-only <base>..803d985`
  Expected: four files —
  - `uwh-common/src/uwhportal/schedule.rs`
  - `schedule-processor/src/csv_parser.rs`
  - `schedule-processor/src/schedule_checks.rs`
  - `refbox/src/app/view_builders/shared_elements.rs`

- [ ] **Step 3: Save the full diff for catalog reference.**

  Run: `git diff <base>..803d985 > /tmp/audit-unit-2-diff.patch`
  This is reference material for catalog drafting; do not commit.

- [ ] **Step 4: Record file list and SHA range in `AUDIT-PLAN.md` under Unit 2's "Files touched" subsection.**

  Note the cross-crate blast radius explicitly: every consumer of `uwh-common` is potentially affected. Full list: `refbox`, `schedule-processor`, `overlay`, `led-panel-sim`, `matrix-drawing`, partially `wireless-remote`. AUDIT-PLAN.md edit only.

---

## Task 3: Build the behaviour catalog (AUDIT-PLAN.md Step 3)

**Files:** AUDIT-PLAN.md edit only — no code, no commit. No scenarios (pure backend per per-unit applicability table).

Anticipated entries are listed below. The executing subagent MUST verify each by reading the diff before drafting, and add any additional entries surfaced by the slop-catching sweep.

- [ ] **Step 1: Draft B2.1 — Add `FinalPlacingSource` struct.**

  Plain English: "A new data type that describes how a final tournament placing is determined. Each placing can come from either a game result (winner/loser of game N) OR from a seeded position in a group (seed X of group Y, where Y is optional)." Slop-check: nested `Option<...>` types may be over-flexible — does the portal API actually emit `FinalPlacingSource` records with BOTH `result_of` and `seeded_by` set, or NEITHER? If yes, the type permits an invalid state.

- [ ] **Step 2: Draft B2.2 — Add `FinalResults::ListOfPlacements` variant.**

  Plain English: "A new way the tournament rules can describe how final standings are determined: a list of individual placings, each pointing to a game result or a group seed." Slop-check: how does this differ from the existing `FinalResults::ListOfGames` and `FinalResults::Standings`? Is the new variant actually used by current portal data, or anticipatory?

- [ ] **Step 3: Draft B2.3 — Change `SeededBy.group` from `String` to `Option<String>`.**

  Plain English: "Group name on a seed reference is now optional. Previously a seed had to belong to a named group; now it can belong to 'no group' (used for placements that span groups in a playoff)." Slop-check: this is a **wire format / serialization change** — the portal API contract for SeededBy has changed shape. Verify: does the portal send `"group": null` or omit the field entirely? Does the `skip_serializing_if = "Option::is_none"` interact correctly with existing portal payloads that always set group? Backward-compatibility impact for clients reading old schedule snapshots.

- [ ] **Step 4: Draft B2.4 — New `option_item_name` serde helper module.**

  Plain English: "A new piece of plumbing that lets the optional group be serialized/deserialized correctly: when the group is set, it's wrapped in a JSON object `{ \"name\": \"...\" }` like the existing convention; when absent, it's omitted from the JSON entirely." Slop-check: 28-line custom serde helper — could this be done with `#[serde(default, with = "...")]` and the existing `item_name` module via a wrapper, or is the duplication justified?

- [ ] **Step 5: Draft B2.5 — Fix `single_half` detection bug in `TimingRule::into()`.**

  Plain English: "Bug fix in how single-half-game rules are detected. The previous check looked at whether the play time was zero — which is never true, so single-half games were never recognized. The fix checks whether the halftime break is zero, which is the actual signal." **Scope flag:** this fix is bundled inside a feature commit. Per `.claude/rules/scope.md`, that's a scope violation. Catalog this as a distinct behaviour with its own decision, and note the scope violation in the unit notes for the Process refinements log. The fix itself looks correct; the question is whether to keep it AND whether to add a regression test (the original commit shipped without one).

- [ ] **Step 6: Draft B2.6 — `schedule-processor` `csv_parser` handles optional group.**

  Plain English: "When the schedule processor reads a CSV and remaps group names, it now only remaps groups that have a name set. Seeds with no group are left alone." Slop-check: this is a mechanical downstream fix for B2.3. Trivial; almost certainly keep.

- [ ] **Step 7: Draft B2.7 — `schedule-processor` `schedule_checks` handles optional group.**

  Plain English: "When schedule validation walks seed references, it now only validates seeds that name a group; cross-group seeds are skipped with a comment explaining that cross-group validation isn't performed." Slop-check: the `// Placements reference seeds and game results from other groups; cross-group validation is not performed here` comment — is this an intentional design limit, or a hole in the validation that the audit should call out? Verify: what kinds of validation are skipped, and is that the right call?

- [ ] **Step 8: Draft B2.8 — `refbox` `shared_elements::get_team_name` falls back to "Unknown".**

  Plain English: "When the refbox displays a seed-based team reference and the group is missing, it shows `Seed N of Unknown` instead of crashing or showing a confusing literal." Slop-check: this commit was AI-authored (Co-Authored-By: Claude Opus 4.7) per its commit message. Verify: does the `"Unknown"` string match the pattern in `uwh-common`'s `ScheduledTeam::Display` impl exactly (which also uses "Unknown" — verified on master at `uwh-common/src/uwhportal/schedule.rs:59`)? Is "Unknown" the right user-facing string, or should this be translated through the refbox's translation system per `refbox/CLAUDE.md`?

- [ ] **Step 9: Apply the slop-catching checklist sweep.**

  Walk every item in `AUDIT-PLAN.md`'s slop-catching checklist against this diff. Note specifically:
  - Fallback paths for "impossible" cases — the `"Unknown"` fallbacks (B2.8 and the uwh-common Display impl that was edited in B2.3's diff).
  - Logging at unusual levels — n/a, no new log lines in this diff.
  - Comments explaining what code does — the `cross-group validation is not performed here` comment in B2.7.
  - Magic numbers — n/a.
  - "Just in case" retries / waits / sleeps — n/a.

  Add any additional B-entries (B2.9+) for anything the sweep surfaces.

---

## Task 4: User review session (AUDIT-PLAN.md Step 4)

**Files:** AUDIT-PLAN.md edit only — no code, no commit.

- [ ] **Step 1: Walk the user through each catalog entry one at a time, in numerical order.**

  For each:
  1. Read the "What it does (plain English)" line aloud.
  2. State recommendation with reason. The recommendation should engage with the slop-check question raised in Task 3.
  3. Ask the user: keep, delete, or not-sure.
  4. Update the entry's Decision tag in AUDIT-PLAN.md.

  **One question per message. No bundling.**

  Specific entries that warrant extra care during review:
  - **B2.5 (single_half fix):** This is a scope-violation finding. After the keep/delete decision, ask the user separately: "Should the audit add a regression test for this fix?" Default recommendation: yes if kept, since the original commit didn't add one and the bug was real.
  - **B2.4 (option_item_name helper):** Ask the user to confirm they're OK with the duplication vs. consolidating with the existing `item_name` module. If they want consolidation, that's a separate refactor branch — not in scope for Unit 2.
  - **B2.8 ("Unknown" string):** Ask the user whether `"Unknown"` should go through the translation system. Default recommendation: out of scope for this audit — the pattern matches `uwh-common`'s Display impl which is also "Unknown"-as-literal. Translation would be a separate cross-crate refactor. Note as a Findings Backlog item.

- [ ] **Step 2: Revisit any `@proposed` entries at the end of the walkthrough.**

- [ ] **Step 3: Record the consolidated decision log in AUDIT-PLAN.md's Unit 2 "Decisions" subsection.**

  One line per entry per the decision-log template. AUDIT-PLAN.md edit only; no commit.

---

## Task 5: Surgical pruning (AUDIT-PLAN.md Step 5)

**Files:** depends on which entries are `@deleted`. Most likely no deletions (the AI-authored content looks generally clean), but any `@deleted` entry triggers a commit.

- [ ] **Step 1: For each `@deleted` entry, remove the relevant code surgically.**

  Use exact file paths and line ranges from the diff. Do NOT opportunistically refactor adjacent code.

- [ ] **Step 2: Run `cd .worktrees/audit-unit-2-list-of-placements && just fmt && just lint`.**

  Expected: both exit cleanly.

- [ ] **Step 3: Run `cd .worktrees/audit-unit-2-list-of-placements && just check`.**

  Expected: all jobs succeed.

- [ ] **Step 4: Ask the user for approval to commit, then commit per `@deleted` entry.**

  Suggested message shape: `refactor(<scope>): remove <behaviour name> per audit`. One commit per behaviour or per logical group.

If no entries are `@deleted`, this task is a no-op — proceed to Task 6.

---

## Task 6: Cross-crate verification (heavy-process)

**Files:** verification only — no production-code changes here unless a test is added.

This is the heavy-process core of Unit 2. Every consumer of `uwh-common` is verified independently, AND `schedule-processor` is exercised end-to-end. Each verification step is dispatched as its own subagent task with the principal recording the outcome in AUDIT-PLAN.md unit notes.

### Step 6.1: `uwh-common` itself

- [ ] Run (from worktree): `cargo test -p uwh-common --lib`
  Expected: all tests pass. Existing tests `test_serialize_seeded_by` and `test_deserialize_seeded_by` are particularly relevant — they exercise the changed `SeededBy.group` type. Confirm they were updated to the new `Option<String>` shape (they should have been, since the original commit had to make them compile).
- [ ] Record in unit notes: which existing tests cover the changed behaviour, and which kept behaviours have NO direct test (gap list).

### Step 6.2: New serde round-trip test for `ListOfPlacements` (gap fill, optional)

- [ ] Check existing tests: there are `test_serialize_list_of_games_final_results` and `test_deserialize_list_of_games_final_results` for the `ListOfGames` variant. The new `ListOfPlacements` variant has NO equivalent test on master.
- [ ] If the user agrees (ask during Task 4 follow-up or here), add `test_serialize_list_of_placements_final_results` and `test_deserialize_list_of_placements_final_results` in `uwh-common/src/uwhportal/schedule.rs`'s test module, mirroring the pattern. The test exercises `serde_json::to_string` → `serde_json::from_str` round-trip on a `FinalResults::ListOfPlacements` with at least one entry that has `result_of: Some(...)` and at least one that has `seeded_by: Some(...)`. Bonus: include one entry with `group: None` to cover the cross-group case.
- [ ] Commit: `test(uwh-common): add serde round-trip tests for ListOfPlacements variant`.

### Step 6.3: `schedule-processor` build, tests, and end-to-end exercise

- [ ] Run (from worktree): `cargo build -p schedule-processor && cargo test -p schedule-processor`
  Expected: clean.
- [ ] **End-to-end exercise:** Run `schedule-processor` against a real placements-using schedule from a UWH portal export. The user has prior tournament data — ask them to provide (or point at) a portal JSON dump that contains a `ListOfPlacements` block. If no real data is available, generate a synthetic schedule by hand-editing a known-good export to add a `ListOfPlacements` group, and run `schedule-processor check <path>` against it.
- [ ] Record in unit notes: command run, output observed by the user, any unexpected warnings or errors.
- [ ] If the exercise reveals a regression (e.g. validation skips a real issue, scoresheet generation breaks), surface it as a `@tested_fail` for the relevant catalog entries and STOP — discuss with user before proceeding.

### Step 6.4: `refbox` build and tests

- [ ] Run (from worktree): `cargo build -p refbox && cargo test -p refbox --bin refbox`
  Expected: clean. The downstream B2.8 fix is exercised indirectly by any `refbox` UI rendering test that touches `shared_elements::get_team_name` — note if any exist.
- [ ] Record in unit notes.

### Step 6.5: `overlay` build and tests

- [ ] Run (from worktree): `cargo build -p overlay && cargo test -p overlay`
  Expected: clean. `overlay` consumes `uwh-common` types via the wire format, so the `SeededBy.group` change must not break overlay's parsing of game data sent by refbox.
- [ ] Record in unit notes.

### Step 6.6: `led-panel-sim` and `matrix-drawing` build and tests

- [ ] Run (from worktree): `cargo build -p led-panel-sim && cargo test -p led-panel-sim`
- [ ] Run (from worktree): `cargo build -p matrix-drawing && cargo test -p matrix-drawing`
  Expected: clean for both.
- [ ] Record in unit notes.

### Step 6.7: Regression test for the single_half fix (B2.5), if kept

- [ ] If B2.5 was kept in Task 4, add a focused test in `uwh-common/src/uwhportal/schedule.rs`'s test module that constructs a `TimingRule` with `half_time_duration: Duration::ZERO` and a non-zero `half_play_duration`, runs `Into::<GameConfig>::into()`, and asserts `single_half == true`. Add a paired test with `half_time_duration > 0` asserting `single_half == false`.
- [ ] Verify the test fails when the fix is reverted (i.e. when `half_time_duration == 0` is changed back to `half_play_duration == 0`). Per Unit 1 refinement #4, use a surgical Edit on just the `single_half:` line to revert + re-restore, NOT `git checkout` (which would wipe the new test).
- [ ] Commit: `test(uwh-common): add regression test for single_half detection fix`.

### Step 6.8: Full `just check` sweep

- [ ] Run (from worktree): `just check`
  Expected: exit 0; all jobs succeed; the existing 5 RUSTSEC "allowed warnings" surface and are not failures.
- [ ] Record in unit notes the final `just check` outcome.

---

## Task 7: Retroactive ADR (AUDIT-PLAN.md Step 7)

**Files:**
- Create: `docs/decisions/020-list-of-placements.md` in the worktree (verify number with `ls docs/decisions/` — the worktree was cut from `origin/master`, which only has 001-005; the eventual post-merge state will have 019 from Unit 1, so 020 is anticipated for Unit 2. Per Unit 1 refinement #8, the gap on the audit branch is expected and closes at Final Integration).

- [ ] **Step 1: Confirm the next ADR number.**

  Run (from worktree): `ls docs/decisions/ | sort | tail -3`
  If Unit 1's audit branch's 019 file isn't visible here (it won't be — Unit 1 is on a different branch), the next free integer on this branch is 006. Use **020** anyway, anticipating the post-merge state. Document this in the ADR's prose if it could confuse a reviewer.

- [ ] **Step 2: Write the retroactive ADR using the template in AUDIT-PLAN.md.**

  Required structure:
  - **Title:** `# ADR 020 — ListOfPlacements + SeededBy.group`
  - **Status:** `Accepted (retroactive)`
  - **Date:** the audit date (YYYY-MM-DD)
  - **Audit unit:** `2 — ListOfPlacements + SeededBy`
  - **Audit PR:** `(to be filled at Final Integration)`
  - **Context:** 2–3 sentences. The portal API gained a `ListOfPlacements` tournament format for seeded group playoffs; this required new types and a wire-format change to `SeededBy.group`. The fix shipped in commit `6907ef8` (2026-04-11) with a downstream refbox display fix in `803d985` (2026-04-18). This ADR records what survived after the AI Code Audit.
  - **Decision:** numbered list of kept behaviours. Each backend behaviour gets one paragraph in plain English describing what it does and how the audit verified it (which test, which crate, which exercise).
  - **Consequences:** what this enables (placements-format tournaments), what wire-format changes downstream tools must absorb, what cross-group validation is intentionally not performed.
  - **What was removed during audit:** list of `@deleted` entries. Likely empty.
  - **Audit reference:** branch name, commits (`6907ef8`, `803d985`), and the SHAs of all audit-branch commits made in this unit.

- [ ] **Step 3: Run `just check` to verify nothing in the ADR broke the build (e.g. markdown link checking).**

- [ ] **Step 4: Ask the user for approval to commit, then commit the ADR.**

  Suggested message: `docs(uwh-common): add ADR 020 for ListOfPlacements + SeededBy (retroactive)`.

---

## Task 8: Hold branch locally (AUDIT-PLAN.md Step 8)

**Files:** none.

- [ ] **Step 1: Confirm the branch is NOT pushed.**

  Run (from worktree): `git push --dry-run` — expected: git refuses or prints "Everything up-to-date" with no upstream branch matching. Do not push.

- [ ] **Step 2: Optional — offer the user a bundle backup if 4–5 audit units are now accumulated locally.**

  Suggested command if accepted: `git bundle create ~/backups/audit-list-of-placements-YYYY-MM-DD.bundle audit/uwh-common/list-of-placements`. The user owns the backup decision.

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Step 9)

**Files:** AUDIT-PLAN.md edits (gitignored, no commit).

- [ ] **Step 1: Present the unit's decision log, cross-crate verification results, and ADR summary for user review.**

  Plain English summary tailored for a non-programmer: which behaviours were kept, what the slop-check resolution was for each, which consumer crates passed verification, what the schedule-processor exercise showed, whether any regression test was added.

- [ ] **Step 2: Wait for explicit "Unit 2 approved" or revision request.**

- [ ] **Step 3: On approval, flip Unit 2's status to `complete-pending-integration (YYYY-MM-DD)` in AUDIT-PLAN.md** — both in the audit-unit-catalog table AND in the unit's section heading.

- [ ] **Step 4: Add a summary entry to "Completed audits"** (NOT a full-section move — per Unit 1 refinement #3 and the playbook amendment to Step 9.4): branch name, plan path, ADR path, catalog outcome, audit-branch commit SHAs.

- [ ] **Step 5: Log playbook refinements discovered during this unit** under AUDIT-PLAN.md's "Process refinements log" → "From Unit 2".

  Anticipated entries:
  - Whether the scope-violation finding (B2.5 bundled in B2.1's feature commit) should change how future audits flag bundled work.
  - Whether the `schedule-processor` exercise step needed real portal data and what to do if such data isn't available.
  - Any cross-crate verification gotchas (e.g. did `overlay` or `led-panel-sim` reveal something `just check` didn't catch?).
  - Whether the gap-fill test in Task 6.2 (round-trip for `ListOfPlacements`) should be a default playbook step for any new variant added to an enum that already has serde tests for sibling variants.

- [ ] **Step 6: Confirm Task 1 → Task 9 are all complete and the acceptance criteria at the top of this plan are satisfied.**

---

## Out-of-scope guardrails

- Findings outside Unit 2 (e.g. an unrelated bug noticed while reading `schedule-processor`) go to AUDIT-PLAN.md's "Findings backlog" section. **Do not fix on this branch.**
- The `single_half` fix (B2.5) is in-scope DESPITE being a scope violation in the original commit — the audit's job is to catalog and decide on every behaviour the diff added, including bundled ones. Note the scope violation in unit notes; do not re-bundle in the audit's own commits.
- Translation of operator-facing strings like `"Unknown"` is OUT of scope for this audit. If the user wants translation, log it as a Findings Backlog entry pointing at the broader translation-coverage question that affects `uwh-common`, `refbox`, and `schedule-processor` together.
- The `option_item_name` serde helper duplication with `item_name` is OUT of scope to refactor (Task 4 Step 1 recommendation). The audit's decision is keep-as-is; consolidation would be a separate `refactor/uwh-common/consolidate-item-name-serde` branch later.
