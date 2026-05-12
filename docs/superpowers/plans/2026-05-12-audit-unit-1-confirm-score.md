# AI Code Audit — Unit 1 — Confirm-Score Timing Fix

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (this unit runs inline by explicit decision — Unit 1 is the audit-playbook rehearsal and surfacing playbook kinks live is the point). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the confirm-score timing fix (commits `0d895ca` and `54973a8`) to the same level of intentional design as hand-coded work — by cataloging every behaviour the diff added, having the user decide keep/delete on each, surgically pruning anything unwanted, exercising what remains (including writing a plain Rust regression test), and capturing the result in a retroactive ADR.

**Architecture:** This plan decomposes Steps 1–9 of `AUDIT-PLAN.md`'s per-unit workflow into concrete tasks. The audit is performed in an isolated git worktree on a new `audit/refbox/confirm-score-timing` branch cut from `master`. The branch stays local through Step 9 and joins the other audit units at the future Final Integration phase. AUDIT-PLAN.md is gitignored — edits there are file-on-disk only, not commits.

**Tech Stack:** Rust 1.85 / edition 2024, `tournament_manager` state machine (the most critical code in the `refbox` crate), iced 0.13 GUI (manual reproduction only), `just` task runner.

**Testing approach:** Two layers. (1) A plain Rust regression test added to `tournament_manager::test` that reproduces the buggy state-machine path and verifies the fix. The cucumber-harness blocker mentioned in the spec doc commit does NOT apply to plain `#[test]` tests. (2) Manual reproduction in the live refbox with the user driving — confirm-score OFF, end the second half, verify the refbox remains responsive through the ~90-second window where the bug used to hit.

---

## Acceptance criteria (Unit 1 "complete-pending-integration")

Mapped to `AUDIT-PLAN.md` "Definition of audit-unit-done":

- [ ] Every catalog entry (B1.1–B1.5) has a final Decision tag — `@user_verified` or `@deleted`. No `@proposed` remaining.
- [ ] Any code marked `@deleted` is removed and committed on the audit branch.
- [ ] `just check` passes on the audit branch (fmt-check, clippy `-D warnings`, all tests pass, `cargo audit` clean).
- [ ] The plain Rust regression test exists, passes on the audit branch, and demonstrably fails when the fix is reverted.
- [ ] Scenario S1.1 has a test tag (`@tested_pass`, `@tested_fail`, or `@tested_inconclusive`) recorded in its `.feature` file in whichever location Task 6.1 settles on.
- [ ] Backend `@user_verified` behaviours (B1.2, B1.4, B1.5) have their test status captured in the retroactive ADR's prose.
- [ ] Any `@tested_fail` finding has been discussed with the user and either resolved or recorded as a known-defect in the ADR.
- [ ] Retroactive ADR `docs/decisions/019-confirm-score-timing.md` (or next-available number) is written and committed.
- [ ] Branch is **not pushed** to remote. No PR is opened.
- [ ] User has reviewed the decision log in AUDIT-PLAN.md and the test status in the `.feature` file, and signalled approval.
- [ ] Any playbook refinements surfaced during execution are logged in AUDIT-PLAN.md's "Process refinements" section.

---

## Prerequisites

- The user has approved this per-unit plan before any Task 1 step runs.
- Current working tree on `master` is clean OR uncommitted state is on `docs/workspace/backlog-adrs` (the worktree creation does not disturb it).
- `git fetch` is current.

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:** none (worktree creation + AUDIT-PLAN.md edit)

- [ ] **Step 1: Confirm working tree is clean on the current branch.**

  Run: `git status` — Expected: no uncommitted changes to tracked files (untracked `.claude/scheduled_tasks.lock` is fine).

  If unclean, stop and discuss with user before proceeding.

- [ ] **Step 2: Ask the user for explicit approval to cut the audit branch in a worktree.**

  Surface the exact branch name and worktree path:
  - Branch: `audit/refbox/confirm-score-timing`
  - Worktree: `.worktrees/audit-unit-1-confirm-score/`

- [ ] **Step 3: Create the worktree on a fresh `master`.**

  Run from repo root:
  ```bash
  git fetch origin master
  git worktree add -b audit/refbox/confirm-score-timing .worktrees/audit-unit-1-confirm-score origin/master
  ```
  Expected: `Preparing worktree (new branch 'audit/refbox/confirm-score-timing')` and the worktree directory exists.

- [ ] **Step 4: Verify the worktree HEAD contains commit `54973a8` (the spec doc commit).**

  Run: `cd .worktrees/audit-unit-1-confirm-score && git log --oneline -5`
  Expected: commit `54973a8` appears in the log (it merged to master before this audit started).

- [ ] **Step 5: Update Unit 1 status in `AUDIT-PLAN.md`.**

  Edit the Unit 1 row in the "Audit unit catalog" table and the unit's detail section at the bottom of `AUDIT-PLAN.md`. Change status from `not started` to `in progress (started 2026-05-12)`. AUDIT-PLAN.md is gitignored — this is a file edit only, no commit.

---

## Task 2: Generate the diff (AUDIT-PLAN.md Step 2)

**Files:** AUDIT-PLAN.md edit only

- [ ] **Step 1: Identify the merge point.**

  Run (from the worktree): `git log --oneline 0d895ca~1 -1`
  Expected: one line showing the parent of `0d895ca`. Record this SHA as `<base>`.

- [ ] **Step 2: Get the file list for both audited commits.**

  Run: `git diff --name-only <base>..54973a8`
  Expected: four files —
  - `refbox/src/app/mod.rs`
  - `refbox/src/tournament_manager/mod.rs`
  - `refbox/tests/features/README.md` (new)
  - `refbox/tests/features/confirm_score_timing.feature` (new)

- [ ] **Step 3: Get the full diff text for the audit.**

  Run: `git diff <base>..54973a8 > /tmp/audit-unit-1-diff.patch`
  This is reference material for catalog drafting in Task 3 — do not commit it.

- [ ] **Step 4: Record the file list and the two SHAs in `AUDIT-PLAN.md` under Unit 1's "Files touched" subsection.**

  AUDIT-PLAN.md edit only; no commit.

---

## Task 3: Build the behaviour catalog and scenarios (AUDIT-PLAN.md Step 3)

**Files:** AUDIT-PLAN.md edit only — no code, no commit

The five anticipated entries from the brainstorm. Draft each in AUDIT-PLAN.md's Unit 1 "Behaviour catalog" subsection using the template under "Templates" in the playbook. Start each entry with the Decision tag `@proposed`.

- [ ] **Step 1: Draft B1.1 — Primary fix: clear pause state before starting the clock.**

  UI-facing → also draft scenario `S1.1` in the Scenarios subsection using the scenario entry template. Suggested wording:
  ```gherkin
  Feature: Confirm-score timing fix

    Scenario: Clock starts cleanly after the second half ends with confirm-score off
      Given the operator has "Confirm Score Required" set to OFF in Game Settings
      And a game has been configured and started
      And the second half has ended
      When the operator dismisses the score-confirmation prompt
      Then the refbox moves to the between-games period
      And the refbox remains fully responsive for at least 120 seconds afterwards
  ```
  Tag `@proposed` until Step 4.

- [ ] **Step 2: Draft B1.2 — Defensive fix in `end_confirm_pause`.**

  Backend-only; no scenario. The "What it does" line should be plain English: *"If `end_confirm_pause` is ever called while the game is not in a state where score confirmation makes sense, the refbox now logs a warning and clears the pause state silently instead of crashing the process."* Slop-flag note: looks like defensive code at an internal boundary, but the original bug demonstrates the path was actually reached — recommend keep, discuss in Step 4.

- [ ] **Step 3: Draft B1.3 — New `.unwrap()` in [refbox/src/app/mod.rs:1918](refbox/src/app/mod.rs#L1918).**

  This is a code-hygiene finding, not an operator-observable behaviour. Note in the entry that it mirrors the existing pattern at [refbox/src/app/mod.rs:1934](refbox/src/app/mod.rs#L1934) (also unwrap, also in a ConfirmScores path) — so the project-rule resolution could reasonably be either:
  - **Option A:** add a justifying comment to both, or
  - **Option B:** replace both with proper error handling.

  Surface both options in the entry so Step 4 can decide.

- [ ] **Step 4: Draft B1.4 — `refbox/tests/features/README.md` workspace convention.**

  Backend-only. Note the location tension: this README documents `refbox/tests/features/` as the project's `.feature` location, while the revised audit playbook (Step 6.1) says audit `.feature` files live in `docs/audit-scenarios/`. Reference the deferred Task 6.1 decision.

- [ ] **Step 5: Draft B1.5 — `confirm_score_timing.feature` content.**

  Backend-only at the catalog level (the file's content is referenced from S1.1, but the file's existence/location is a separate decision). Note the same location tension as B1.4.

- [ ] **Step 6: Sanity-check the catalog against the slop-catching checklist in AUDIT-PLAN.md.**

  Walk down the list. If anything else in the diff matches (e.g. a magic number, a log at an unusual level, a re-implementation of an existing utility), add a B1.6+ entry now. Otherwise proceed.

---

## Task 4: User review session (AUDIT-PLAN.md Step 4)

**Files:** AUDIT-PLAN.md edit only — no code, no commit

- [ ] **Step 1: Walk the user through each catalog entry one at a time, in B1.1 → B1.5 order.**

  For each:
  1. Read the "What it does (plain English)" line aloud.
  2. For UI-facing entries (B1.1 only), read the linked scenario S1.1 aloud verbatim.
  3. State the recommendation and the reason.
  4. Ask the user: keep, delete, or not-sure.
  5. Update the entry's Decision tag in AUDIT-PLAN.md (`@proposed` → `@user_verified` or `@deleted`). On not-sure, leave as `@proposed`.

  **Never bundle multiple entries into one question.** One at a time.

  For B1.3 specifically, present Option A (justifying comment) and Option B (replace with error handling) as a sub-question after the keep decision.

- [ ] **Step 2: At the end of the walkthrough, revisit any `@proposed` entries.**

  Sometimes seeing the whole picture clarifies a single item. Convert each remaining `@proposed` to a final decision before Task 5 begins.

- [ ] **Step 3: Record the full decision log in AUDIT-PLAN.md's Unit 1 "Decisions" subsection.**

  One line per entry per the decision-log template in the playbook. AUDIT-PLAN.md edit only; no commit.

---

## Task 5: Surgical pruning (AUDIT-PLAN.md Step 5)

**Files:** depends on which entries are `@deleted` and (for B1.3) which option the user picked.

This task only fires if at least one entry is `@deleted`, or if B1.3 was kept-with-Option-A (comment) or kept-with-Option-B (error handling). Otherwise skip directly to Task 6.

- [ ] **Step 1: For each `@deleted` entry, remove the relevant code.**

  Use exact file paths from the diff. Keep edits surgical — do not opportunistically refactor adjacent code (project scope rule).

- [ ] **Step 2: For B1.3 Option A (justifying comment), add a comment above each `.unwrap()` explaining why it can't fail.**

  Apply consistently to both [refbox/src/app/mod.rs:1918](refbox/src/app/mod.rs#L1918) (introduced by the fix) and [refbox/src/app/mod.rs:1934](refbox/src/app/mod.rs#L1934) (pre-existing, same pattern). Mention in commit message that the second one is in-scope by pattern consistency.

- [ ] **Step 3: For B1.3 Option B (replace with error handling), propagate the `Result` upward.**

  This is more invasive; pause and discuss with user before proceeding. Likely requires changing the `Message::ConfirmScores` handler to log-and-recover on error rather than `.unwrap()` panicking.

- [ ] **Step 4: Run `just fmt` and `just lint`.**

  Run (from worktree): `just fmt && just lint`
  Expected: both exit cleanly.

- [ ] **Step 5: Run `just check`.**

  Run (from worktree): `just check`
  Expected: all jobs succeed.

- [ ] **Step 6: Ask the user for approval to commit, then commit the pruning.**

  One commit per behaviour or per logical group. Suggested message shapes:
  - For B1.3 Option A: `fix(refbox): document why end_confirm_pause unwrap is safe in ConfirmScores`
  - For B1.3 Option B: `fix(refbox): propagate end_confirm_pause errors in ConfirmScores`
  - For any `@deleted` entry: `refactor(refbox): remove <behaviour name> per audit`

  Use the project commit format `type(scope): description` per CLAUDE.md.

---

## Task 6: Test pass on what remains (AUDIT-PLAN.md Step 6)

**Files:**
- Decide in Step 1: `docs/audit-scenarios/confirm-score-timing.feature` (new) OR keep the existing `refbox/tests/features/confirm_score_timing.feature` as authoritative.
- Add: a new `#[test] fn` inside the existing `mod test` block in `refbox/src/tournament_manager/mod.rs` (the test module starts at line 2366 in current master; verify location at execution time).

### Step 6.1: Resolve the `.feature` file location wrinkle (deferred decision)

- [ ] **Step 6.1.1: Surface the decision to the user with options.**

  The audit playbook (Step 6.1) says: *"For each `Feature:` block in the unit's Scenarios subsection that contains at least one `@user_verified` scenario, create a corresponding `.feature` file in `docs/audit-scenarios/`."* But commit `54973a8` already put a `.feature` file at `refbox/tests/features/confirm_score_timing.feature`, with a README documenting that location as the project convention for the future cucumber harness.

  Options to present to the user, with recommendation:
  - **A. Adapt the playbook to the existing convention.** Future audit `.feature` files live in `refbox/tests/features/`, not `docs/audit-scenarios/`. Amend the playbook at end of Unit 1. *Recommended* — the project convention pre-dates the playbook revision and has a real cucumber-harness future.
  - **B. Adapt the project to the playbook.** Move `refbox/tests/features/confirm_score_timing.feature` to `docs/audit-scenarios/confirm-score-timing.feature` as part of this audit; delete or relocate the README; future audits use `docs/audit-scenarios/`.
  - **C. Both files exist (one in each location), with one designated authoritative.** Likely the worst option — diverges over time.

- [ ] **Step 6.1.2: Apply the user's decision.**

  - If A: leave existing file in place. Update the existing `refbox/tests/features/confirm_score_timing.feature` to add the audit's `@user_verified` tag to its Scenario block (the existing scenario in the file is a slightly different phrasing of S1.1 — replace or align with the version from Task 3 Step 1, with the user's say). Add a comment block at the top of the `Feature:` for the upcoming test session.
  - If B: `git mv refbox/tests/features/confirm_score_timing.feature docs/audit-scenarios/confirm-score-timing.feature`; revise or delete `refbox/tests/features/README.md`; copy S1.1 verbatim with `@user_verified`.
  - If C: discuss before committing — this option has known risks.

- [ ] **Step 6.1.3: Ask the user for approval to commit, then commit the seeded file.**

  Suggested message: `docs(refbox): align confirm-score-timing scenario with audit unit 1` (Option A) or `docs(refbox): move confirm-score-timing scenario to docs/audit-scenarios` (Option B).

### Step 6.2: Write the plain Rust regression test (TDD)

- [ ] **Step 6.2.1: Locate the test module.**

  Open `refbox/src/tournament_manager/mod.rs`. The test module `mod test` starts around line 2366 (verify with grep). Find an existing test that sets up a SecondHalf scenario with `pause_for_confirm` followed by `end_confirm_pause` (e.g. `test_pause_score_confirm_with_only_sd_score_changed_to_tie` near line 6293) to use as a pattern reference.

- [ ] **Step 6.2.2: Write the new test — name it `test_confirm_score_off_clears_pause_before_clock_start`.**

  The test must mirror the production code path: `pause_for_confirm` at end of SecondHalf, then call `end_confirm_pause` followed by `start_clock` in the same sequence as `Message::ConfirmScores` does at [refbox/src/app/mod.rs:1916-1920](refbox/src/app/mod.rs#L1916-L1920). Then verify:
  - `tm.time_pause_confirmation == None` (pause state was cleared)
  - The game has transitioned to `GamePeriod::BetweenGames`
  - The clock is running
  - **No panic occurs** when the background pause timer would fire ~90s later (advance the clock past the pause-confirmation duration and call `update`)

  Pattern to follow (adapt to project style):
  ```rust
  #[test]
  fn test_confirm_score_off_clears_pause_before_clock_start() {
      initialize();
      let config = GameConfig::default();
      let mut tm = TournamentManager::new(config);
      let start = Instant::now();

      // Reach end of SecondHalf with a score gap (no overtime needed)
      tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(1));
      tm.set_game_start(start);
      tm.set_scores(BlackWhiteBundle { black: 2, white: 1 }, start);
      tm.start_game_clock(start);

      // Game ends → operator hits pause_for_confirm (Message::ConfirmScoresPause path)
      let game_end = start + Duration::from_secs(1);
      tm.pause_for_confirm(game_end).unwrap();
      assert!(tm.in_score_confirm_pause());

      // Operator dismisses confirm-score (confirm_score == false branch):
      // end_confirm_pause + start_clock, mirroring refbox/src/app/mod.rs Message::ConfirmScores.
      let dismiss = game_end + Duration::from_secs(1);
      tm.end_confirm_pause(dismiss).unwrap();
      tm.start_clock(dismiss);

      // Post-condition checks
      assert_eq!(tm.time_pause_confirmation, None);
      assert_eq!(tm.current_period, GamePeriod::BetweenGames);
      assert!(tm.clock_is_running());

      // Advance ~90 seconds — must not panic when the background pause timer would have fired.
      let later = dismiss + Duration::from_secs(95);
      tm.update(later).unwrap();
      assert_eq!(tm.time_pause_confirmation, None);
  }
  ```
  Verify field/method names against the current `TournamentManager` API while writing (e.g. `set_scores`, `BlackWhiteBundle`, `Color::Black/White` if needed).

- [ ] **Step 6.2.3: Run the test to confirm it passes on the audit branch (fix is present).**

  Run (from worktree): `cargo test -p refbox --lib test_confirm_score_off_clears_pause_before_clock_start`
  Expected: 1 passed.

- [ ] **Step 6.2.4: Verify the test would have caught the bug — temporarily revert the fix.**

  The new test commit is not yet in HEAD (we are about to commit it in Step 6.2.6), so the new test file is still in the working tree. Point the two production files at their pre-fix state without touching the test:

  ```bash
  git checkout 0d895ca~1 -- refbox/src/app/mod.rs refbox/src/tournament_manager/mod.rs
  cargo test -p refbox --lib test_confirm_score_off_clears_pause_before_clock_start
  ```
  Expected: test **FAILS** — typically a panic on `unreachable!()` inside `end_confirm_pause` during the `tm.update(later)` call.

  Then restore the fix:
  ```bash
  git checkout HEAD -- refbox/src/app/mod.rs refbox/src/tournament_manager/mod.rs
  cargo test -p refbox --lib test_confirm_score_off_clears_pause_before_clock_start
  ```
  Expected: test **PASSES** again.

  If the pre-fix run does not fail, the test is not actually exercising the bug — refine and repeat before declaring this step done.

- [ ] **Step 6.2.5: Run `just check` to verify nothing else regressed.**

  Run (from worktree): `just check`
  Expected: all jobs succeed.

- [ ] **Step 6.2.6: Ask the user for approval to commit, then commit the test.**

  Suggested message: `test(refbox): add regression test for confirm-score timing fix`

### Step 6.3: Manual reproduction with the user driving

- [ ] **Step 6.3.1: Set the stage for the user.**

  The user will launch the refbox per the run-command memory (`cargo run -p refbox` with `dangerouslyDisableSandbox:true`). Claude does not auto-launch. Provide the user with the test protocol in plain English:
  1. Go to Settings → confirm "Confirm Score Required" is OFF.
  2. Start a game (any team config; the bug is independent of teams).
  3. Run the game clock down to the end of the second half.
  4. When the score-confirm prompt appears, dismiss it.
  5. Observe the refbox for at least 120 seconds after dismissal.

- [ ] **Step 6.3.2: User reports observations.**

  Expected: the refbox transitions cleanly to between-games, the clock starts, and the app remains responsive throughout the 120-second window. No freeze, no panic, no "mutex poisoned" log line.

- [ ] **Step 6.3.3: Record the result.**

  In whichever `.feature` file Step 6.1 settled on, add the test tag to scenario S1.1:
  - `@user_verified @tested_pass` if everything matches
  - `@user_verified @tested_fail` if any Then-line failed → STOP, discuss with user before proceeding
  - `@user_verified @tested_inconclusive` if ambiguous → revisit at end of unit

  Add a date-stamped comment block above the `Feature:` summarising the test session, per playbook Step 6.2.

- [ ] **Step 6.3.4: Ask the user for approval to commit the test-session update.**

  Suggested message: `docs(refbox): record test session 1 for confirm-score timing`

### Step 6.4: Record backend-test status for B1.2, B1.4, B1.5

- [ ] **Step 6.4.1: For B1.2 (defensive fix), note in unit notes that the regression test from Step 6.2 implicitly covers it.**

  The `tm.update(later)` line at the end of the regression test exercises the post-90-second path — that is the path that used to hit `unreachable!()` and now hits the warn-and-recover branch. Note this in AUDIT-PLAN.md unit notes for the ADR to cite.

- [ ] **Step 6.4.2: For B1.4 and B1.5, the backend test is the file's continued existence at the location Step 6.1 settled on. Note this in unit notes.**

---

## Task 7: Retroactive ADR (AUDIT-PLAN.md Step 7)

**Files:**
- Create: `docs/decisions/019-confirm-score-timing.md` (verify number with `ls docs/decisions/` — likely 019 since 012 is reserved for a different in-flight ADR).

- [ ] **Step 1: Confirm the next ADR number.**

  Run (from worktree): `ls docs/decisions/ | sort | tail -5`
  Pick the next free integer. Treat the result as authoritative even if it differs from 019.

- [ ] **Step 2: Write the retroactive ADR using the template in AUDIT-PLAN.md "Templates" → "Retroactive ADR template".**

  Required sections:
  - **Title:** `# ADR NNN: Confirm-Score Timing Fix`
  - **Status:** `Accepted (retroactive)`
  - **Date:** 2026-05-12
  - **Audit unit:** `1 — Confirm-score timing fix`
  - **Audit PR:** none yet (filled at Final Integration)
  - **Context:** 2–3 sentences. The bug was observed six times across Jan 13, Jan 19, and Feb 24 2026 deployments. The fix shipped April 2026; this audit confirms what survived.
  - **Decision:** numbered list of kept behaviours. For S1.1, embed the scenario verbatim as a Gherkin code block. For B1.2 / B1.4 / B1.5 (backend), one paragraph each in plain English. If B1.3 was kept (with comment or with proper error handling), note which option and why.
  - **Consequences:** what this enables, what we now commit to maintaining, any constraints.
  - **What was removed during audit:** list of `@deleted` entries (likely empty for this unit). If empty, write "Nothing was removed during this audit; every behaviour in the diff was kept."
  - **Audit reference:** branch name, commits (`0d895ca` and `54973a8`).

- [ ] **Step 3: Run `just check` to verify nothing in the ADR broke (e.g. lint catching trailing whitespace in markdown).**

  Run (from worktree): `just check`
  Expected: all jobs succeed.

- [ ] **Step 4: Ask the user for approval to commit, then commit the ADR.**

  Suggested message: `docs(refbox): add ADR NNN for confirm-score timing fix (retroactive)`

---

## Task 8: Hold branch locally (AUDIT-PLAN.md Step 8)

**Files:** none

- [ ] **Step 1: Confirm the branch is NOT pushed.**

  Run (from worktree): `git push --dry-run` should reveal nothing has been pushed for this branch. Do not push.

- [ ] **Step 2: Optional — offer the user a bundle backup.**

  Suggested command (if user wants it):
  ```bash
  git bundle create ~/backups/audit-confirm-score-timing-2026-05-12.bundle audit/refbox/confirm-score-timing
  ```
  The user owns the decision to back up.

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Step 9)

**Files:**
- AUDIT-PLAN.md edits (gitignored, no commit).

- [ ] **Step 1: Present the unit's decision log and test status for user review.**

  Show the user (in plain English):
  - The five catalog entries and their final decisions
  - The S1.1 test status from the `.feature` file
  - The new ADR's file path and a short summary of what it says

- [ ] **Step 2: Wait for the user's explicit "Unit 1 approved" or revision request.**

- [ ] **Step 3: On approval, update Unit 1's status to `complete-pending-integration (2026-05-12)` in AUDIT-PLAN.md.**

- [ ] **Step 4: Move Unit 1's full details section to "Completed audits" near the bottom of AUDIT-PLAN.md.**

- [ ] **Step 5: Log any playbook refinements discovered during this unit under AUDIT-PLAN.md's "Process refinements log".**

  Almost certain to include the `.feature` file location resolution from Task 6.1. Also log anything else that came up — overly-rigid steps, ambiguous templates, missing checks.

- [ ] **Step 6: Confirm Task 1 → Task 9 are all complete and the acceptance criteria at the top of this plan are satisfied.**

  If any criterion is unmet, surface it now rather than declaring done.

---

## Out-of-scope guardrails

- Findings outside Unit 1's scope (e.g. a separate bug noticed in tournament_manager while reading the diff) go to AUDIT-PLAN.md's "Findings backlog" section. **Do not fix on this branch.**
- The pre-existing `.unwrap()` at [refbox/src/app/mod.rs:1934](refbox/src/app/mod.rs#L1934) is in-scope **only as a pattern-consistency consideration for B1.3** — if Option A or B applies to the new unwrap, it applies to this one too. Do not separately audit any other pre-existing unwraps in the file.
- No `uwh-common` changes are expected. If pruning surfaces a need to touch `uwh-common`, stop and escalate — that would flip the unit's process from lean to heavy.
