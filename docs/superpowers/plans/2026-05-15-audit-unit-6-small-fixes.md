# Audit Unit 6 — Sound / Keypad / Multi-Label Fixes: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` (subagent-driven execution is the default; principal dispatches a fresh subagent per task and ferries critical keep/delete decisions back to the user). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit the four post-rebase fix commits on master (`edb4b9c`, `8a8d018`, `7269c11`, `03d126c`), verify the shipped behaviour matches ADR 005 entries 7 (UI Text Clipping Fixes) and 11 (Sound Artifacts Fix), surface any scope that leaked in during the rebase (notably the `LARGE_TEXT` → `MEDIUM_TEXT` font-size simplification disclosed in `edb4b9c`'s commit message but absent from ADR 005), seed Gherkin scenarios for what survives, and finalize ADR 005 entries 7 and 11 with "Verified by Unit 6 audit" subsections.

**Architecture:** Diff-led catalog grouped into three feature blocks (timed-buzzer-playback · multi-label-button-text · keypad-player-number) per AUDIT-PLAN.md playbook line 379. Catalog decomposes the two bundled-fix commits (`7269c11` → 3 sub-behaviours; `edb4b9c` → 2 sub-behaviours) per Unit 5 refinement #4. ADR 005 already captured the design intent; this audit verifies the post-rebase implementation matches and amends entries 7 and 11 in place per Unit 3 refinement #4. Lean process per `.claude/rules/plan-execution.md` — refbox-only, lower-risk bug-fix work.

**Tech Stack:** Rust 2024 / MSRV 1.85; iced 0.13 (refbox UI for keypad + multi-label buttons); `web_audio_api` crate for sound playback; `cargo test`, `cargo clippy`, `just check`; manual refbox launch (`WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox` with `dangerouslyDisableSandbox:true`); Gherkin scenarios at three new `.feature` files in `refbox/tests/features/`.

**Testing approach:**

- **No new Rust unit tests expected.** Sound playback is async + Web Audio context dependent; `Sound::stop()` resists direct unit-testing. View builders are walkthrough-verified, not unit-tested. Matches the lean-process refbox-UI pattern from Units 3, 4, 5. If the audit surfaces something testable that isn't already covered, add a test — but don't force-add tests for behaviours that resist unit-testing.
- **Operator-driven UI/audio walkthrough** for Gherkin scenarios in Task 7, one session covering all three feature blocks. Walkthrough ordering: **UI-first (multi-label → keypad), then sound** — UI fixes are easier to verify and re-test; audio is reserved for last so the sound subsystem need only initialize once.
- **No cross-crate verification** — Unit 6 touches `refbox` only. `just check` at the audit branch tip is sufficient.

---

## Acceptance criteria (Unit 6 "complete-pending-integration")

Unit 6 is complete-pending-integration when **all** of these hold on the audit branch `audit/refbox/small-fixes-cluster`:

1. A behaviour catalog exists in `AUDIT-PLAN.md` under Unit 6, grouped into three feature blocks (Timed buzzer playback · Multi-label button text · Keypad player number display), with every entry tagged `@user_verified`, `@deleted`, `@redesign-followup`, or `@findings-backlog`. No `@open` remaining.
2. Every `@user_verified` operator-observable behaviour is captured as a Gherkin scenario in one of three `.feature` files:
   - `refbox/tests/features/timed-buzzer-playback.feature`
   - `refbox/tests/features/multi-label-button-text.feature`
   - `refbox/tests/features/keypad-player-number.feature`
3. Each scenario carries `@user_verified` plus a test-state tag (`@tested_pass`, `@tested_fail`, or `@tested_inconclusive`) and a manual-walkthrough timestamp in a session-notes comment.
4. The operator has driven the refbox UI/audio through one Session covering all three feature blocks (multi-label state transitions → keypad short-string display → timed Crazy buzzer playback).
5. `just check` passes on the audit branch (`fmt-check`, `clippy -D warnings`, all tests pass, `cargo audit` clean — the two pre-existing dependency vulnerabilities noted in Unit 3's Findings backlog #4 are expected and not regressions).
6. ADR 005 entries 7 and 11 each gain a "Verified by Unit 6 audit (YYYY-MM-DD)" subsection on branch `docs/workspace/backlog-adrs` (per Unit 3 refinement #6 — ADR finalization commits land on the ADR's own branch, not the audit branch). The subsection lists post-rebase commit hashes, references catalog entries, and references the seeded `.feature` files.
7. The branch holds locally (no push, no PR — all PRs deferred to Final Integration per the post-v0.4.0 convention).
8. AUDIT-PLAN.md status flipped from "not started" to "complete-pending-integration (YYYY-MM-DD)" in both the catalog table and the unit section heading; summary pointer added to "Completed audits" section per playbook-amended Step 9.4.
9. Findings discovered out-of-scope are recorded in AUDIT-PLAN.md's Findings backlog with a suggested follow-up branch name. They are **not fixed** on this branch.
10. Process refinements surfaced during execution are logged in AUDIT-PLAN.md's "Process refinements log → From Unit 6".

---

## Prerequisites

- The user has approved this per-unit plan before any Task 1 step runs.
- Working tree on the current branch (`docs/workspace/backlog-adrs`) is clean except for the gitignored `.claude/scheduled_tasks.lock`.
- `git fetch origin master` is current; master is at `82a370d`.
- Read `AUDIT-PLAN.md` Unit 6 section and the Process refinements log entries from Units 1–5. Particularly relevant to Unit 6:
  - Unit 1 refinement #2 (`.feature` files live in `refbox/tests/features/`)
  - Unit 1 refinement #6 (Bash cwd doesn't persist between calls — always `cd` into worktree)
  - Unit 1 refinement #7 (refbox launch needs `WAYLAND_DISPLAY= RUST_LOG=info`)
  - Unit 3 refinement #3 (per-entry review for catalogs ≤15 entries — Unit 6 fits)
  - Unit 3 refinement #4 (ADR finalize-in-place pattern)
  - Unit 3 refinement #6 (ADR amendment commit on its own branch, not the audit branch)
  - Unit 4 refinement #3 (pre-existing pattern-consistent debt → Findings backlog, not audit-branch fix)
  - Unit 5 refinement #4 (decompose bundled-fix commits into separate B-entries)
- Read `docs/superpowers/specs/2026-05-15-audit-unit-6-small-fixes-design.md` (this plan's authoritative spec).
- Read `docs/decisions/005-v040-feature-audit.md` entries 7 and 11 (the audit's oracle).
- Read `.claude/rules/scope.md`, `communication.md`, `workspace.md`, `rust.md`, `plan-execution.md`, `pr-review.md`. `embedded.md` is informational only — Unit 6 does not touch `wireless-remote`.
- Pre-commit hook at `<main-repo>/.git/hooks/pre-commit` must allow `audit/` branch type (fixed by Unit 1's `2a8dcbc`). Verify in Task 1 Step 3.
- Memory `feedback_prs_deferred_until_audit_done` is in force: do not propose, suggest, or execute any PR/merge during this unit.

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:**
- Create: `.worktrees/audit-unit-6-small-fixes/` (new worktree)
- Edit: `AUDIT-PLAN.md` (gitignored, no commit)

- [ ] **Step 1.1: Confirm working tree is clean.**

  Run: `git -C /home/estraily/projects/uwh-refbox-rs status --short`
  Expected: empty output (or only `?? .claude/scheduled_tasks.lock` which is untracked-and-fine).

- [ ] **Step 1.2: Ask the user for explicit approval to cut the audit branch in a worktree.**

  Surface to the user:
  - Branch: `audit/refbox/small-fixes-cluster`
  - Worktree: `.worktrees/audit-unit-6-small-fixes/`
  - Cut from: `origin/master` at `82a370d`

  Wait for approval.

- [ ] **Step 1.3: Verify the pre-commit hook allows the `audit` branch type.**

  Run from main repo root: `grep -c '\baudit\b' .git/hooks/pre-commit`
  Expected: non-zero count. If zero, copy the audit-aware version from Unit 1's branch:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs && git show audit/refbox/confirm-score-timing:scripts/pre-commit > .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
  ```

- [ ] **Step 1.4: Create the worktree on a fresh `master`.**

  Run from main repo root:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs && git fetch origin master && git worktree add -b audit/refbox/small-fixes-cluster .worktrees/audit-unit-6-small-fixes origin/master
  ```
  Expected output: `Preparing worktree (new branch 'audit/refbox/small-fixes-cluster')`.

- [ ] **Step 1.5: Verify the worktree HEAD contains the four audit commits.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes && git log --oneline edb4b9c 8a8d018 7269c11 03d126c -4`
  Expected: all four commits appear (they merged to master before this audit).

- [ ] **Step 1.6: Sanity-check the worktree builds.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes && cargo build -p refbox 2>&1 | tail -5`
  Expected: clean build (or only the workspace's pre-existing warnings if any).

---

## Task 2: History reconstruction (AUDIT-PLAN.md Step 2)

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 6 section (gitignored, no commit)
- Optional: `.audit/unit-6-*.txt` working artifacts (local-only, never committed)

- [ ] **Step 2.1: Capture commit metadata for the four audit commits.**

  Run (from anywhere — `--no-walk` enumerates only the listed hashes):

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes \
    && mkdir -p .audit \
    && git log --no-walk --reverse --pretty='format:%H%n%ad%n%s%n%b%n---' edb4b9c 8a8d018 7269c11 03d126c > .audit/unit-6-commit-messages.txt
  ```

  Walk the four commits chronologically: `edb4b9c` (2026-04-15) → `8a8d018` (2026-04-15) → `7269c11` (2026-04-17) → `03d126c` (2026-04-17). Note that `03d126c` patches a line introduced by `7269c11`.

- [ ] **Step 2.2: Capture per-commit diffs.**

  For each of `edb4b9c`, `8a8d018`, `7269c11`, `03d126c`: run `git show <hash>` and read the full diff.

  For `7269c11`, the binary asset diff (`refbox/resources/sounds/crazy.raw`) will show as `Binary files ... differ` — that's expected. Note the byte count (117404 → 117720, +316).

- [ ] **Step 2.3: Map post-rebase commits to ADR 005 pre-rebase references.**

  ADR 005 entry 7 references pre-rebase `cd577b2` (keypad) and `2749104` (multi-label). ADR 005 entry 11 references pre-rebase `701d12d` (sound).

  Confirm:
  - `edb4b9c` is the post-rebase counterpart of `cd577b2` (same subject line; both touch `keypad_pages/mod.rs`).
  - `8a8d018` is the post-rebase counterpart of `2749104` (same subject line; both touch `shared_elements.rs`'s `make_multi_label_button`).
  - `7269c11` is the post-rebase counterpart of `701d12d` (same subject line; both touch `sound_controller/mod.rs` + `crazy.raw`).
  - `03d126c` is a post-spec clippy follow-up; not referenced by ADR 005.

- [ ] **Step 2.4: Append a short history-trace section to AUDIT-PLAN.md Unit 6.**

  Under `### Unit 6 — Sound / keypad / multi-label fixes`, add a subsection `#### History trace` with one paragraph per commit summarizing (a) what changed, (b) which ADR 005 entry it corresponds to, (c) any divergence-from-spec observation.

  No commit yet — AUDIT-PLAN.md is gitignored.

---

## Task 3: Build behaviour catalog (AUDIT-PLAN.md Step 3)

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 6 section (gitignored, no commit)

- [ ] **Step 3.1: Create the behaviour catalog subsection.**

  Under `### Unit 6 — Sound / keypad / multi-label fixes` (after the history trace), add `#### Behaviour catalog` with three sub-headings:
  - `##### Feature: Timed buzzer playback`
  - `##### Feature: Multi-label button text`
  - `##### Feature: Keypad player number display`

- [ ] **Step 3.2: Populate the seven expected B-entries.**

  Under each feature block, add the B-entries in this exact shape:

  ```markdown
  ##### B6.1 — Space-widget swap fixes short-string display on keypad digit
  - **Source commit:** `edb4b9c`
  - **Files / lines:** `refbox/src/app/view_builders/keypad_pages/mod.rs` lines ~100–106 (post-rebase)
  - **Behaviour:** The digit text widget previously combined `align_x(Horizontal::Right)` with `width(Length::Fill)`, which iced 0.13 fails to render for short strings. The fix replaces that combination with a `Space::with_width(Length::Fill)` widget that pushes the digit to the right, with `align_x(Horizontal::Left)` on the label text.
  - **Spec status:** matches ADR 005 entry 7 bug 1
  - **Status:** @open
  ```

  Repeat for B6.2 through B6.7 with the behaviour summaries from the spec. Key entries:

  - **B6.1** — Space-widget swap (matches ADR 005 entry 7 bug 1)
  - **B6.2** — Font-size simplification: `LARGE_TEXT` removed; all keypad pages now `MEDIUM_TEXT` (**not-in-spec** — flag for explicit operator decision)
  - **B6.3** — Multi-label button container wrap (matches ADR 005 entry 7 bug 2)
  - **B6.4** — `SOUND_LEN` 2.0 → 2.15s (matches ADR 005 entry 11 bug 1)
  - **B6.5** — `Sound::stop()` `already_silent` early-exit (matches ADR 005 entry 11 bug 2)
  - **B6.6** — `crazy.raw` binary replacement (matches ADR 005 entry 11 bug 3)
  - **B6.7** — `is_some_and` clippy refactor on B6.5's added line (not-in-spec — post-spec stylistic; folded into B6.5's verification)

  Group B6.1 + B6.2 under "Keypad player number display"; B6.3 alone under "Multi-label button text"; B6.4–B6.7 under "Timed buzzer playback".

---

## Task 4: Slop-catching pass (AUDIT-PLAN.md Step 4)

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 6 section (gitignored, no commit)

- [ ] **Step 4.1: Apply the playbook checklist.**

  For each B-entry, scan for:
  - Bundled fixes inside the commit (Unit 2 refinement #2 / Unit 5 refinement #4). B6.2, B6.4, B6.5, B6.6, B6.7 are already the bundled-decomposition outputs — confirm each one's parent commit message honestly disclosed the change.
  - New `unwrap()` / `expect()` without justification. Search `git show 7269c11 -- refbox/src/sound_controller/mod.rs | grep -E '\.(unwrap|expect)\('` — if hits exist on lines added by the audit window, distinguish "new latent debt" from "pattern-consistent with module debt" (Unit 4 refinement #3).
  - Commit-ordering anomalies. The four commits are topologically sequential on master; no risk here (Unit 5 refinement #5 false-positive avoided).
  - Pre-existing pattern-consistent debt — flag for whole-module Findings-Backlog, not audit-branch fix.

- [ ] **Step 4.2: Annotate each B-entry with `Why it might be slop` / `Recommendation` pairs.**

  Append to each B-entry as needed:

  ```markdown
  - **Why it might be slop:** [observation]
  - **Recommendation:** keep | flag for Findings-Backlog | propose @deleted
  ```

  Expected observations:
  - **B6.2** (font-size): bundled inside a clipping-fix commit; behaviour change not in ADR 005's text. Recommendation: present to operator with both keep-and-record-in-ADR and revert-as-separate-fix paths.
  - **B6.6** (`crazy.raw`): asset audit needs listening verification; defer recommendation until Task 7.
  - Others: likely "keep" with no slop concern.

---

## Task 5: Per-entry operator decisions (AUDIT-PLAN.md Step 5)

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 6 section (gitignored, no commit)

- [ ] **Step 5.1: Walk the seven B-entries with the user, one at a time.**

  With 7 entries (≤15 threshold per Unit 3 refinement #3), use **per-entry approval**, not page-batched. For each entry, present:
  - Behaviour summary (one sentence in plain English)
  - Spec status (matches-spec / not-in-spec)
  - Slop annotation if any
  - Recommendation (keep / @deleted / @redesign-followup / @findings-backlog)

  Wait for operator decision. Then move to the next entry.

  **Special handling for B6.2 (font-size change, not in ADR 005):** present three options and recommend the first:
  1. **Keep the change and record it in the ADR 005 amendment.** Recommended because the operator wrote the commit message that disclosed it; this is the simplest path.
  2. Revert the font-size simplification on the audit branch (B6.2 becomes @deleted). Surgical edit to `keypad_pages/mod.rs` restoring the `LARGE_TEXT` branch.
  3. Carve out the font-size change to a Findings-Backlog branch for separate decision later. B6.2 becomes @findings-backlog; the change stays on master but the audit defers the decision.

  **Special handling for B6.6 (`crazy.raw`):** mark as `@open — pending Task 7 walkthrough listening verification`. Do not finalize until the operator has listened.

- [ ] **Step 5.2: Update each B-entry's `Status:` line with the operator decision.**

  Each `Status:` line becomes one of: `@user_verified`, `@deleted`, `@redesign-followup`, `@findings-backlog`, or (for B6.6) `@open — pending Task 7`.

- [ ] **Step 5.3: File any Findings-Backlog items inline.**

  If any B-entry becomes `@findings-backlog`, add a `#### From Unit 6 (YYYY-MM-DD)` subsection to AUDIT-PLAN.md's `### Findings backlog` section listing each finding with a suggested branch name.

---

## Task 6: Write Gherkin `.feature` files (AUDIT-PLAN.md Step 6)

**Files:**
- Create: `refbox/tests/features/timed-buzzer-playback.feature`
- Create: `refbox/tests/features/multi-label-button-text.feature`
- Create: `refbox/tests/features/keypad-player-number.feature`

- [ ] **Step 6.1: Write `multi-label-button-text.feature`.**

  In the worktree at `.worktrees/audit-unit-6-small-fixes/`, create the file with one `Feature:` block and one `Scenario:` per `@user_verified` operator-observable behaviour in B6.3:

  ```gherkin
  Feature: Multi-label button text
    The multi-label button (used for two-line button labels on state-transition
    screens) must keep its text within the button's clip bounds across every
    iced re-render triggered by a game-state change.

    @user_verified
    Scenario: Two-line button text survives a state transition
      Given the refbox is on the main game screen
      And a multi-label button is visible (e.g. the start-clock / score buttons)
      When the game state changes (e.g. start clock, end half, score confirm)
      Then both lines of button text remain fully visible
      And no character is clipped against the button's edge

      # Session notes (filled by Task 7):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM
  ```

  Use the spec's behaviour description; do not invent scenarios beyond what B6.3 covers.

- [ ] **Step 6.2: Write `keypad-player-number.feature`.**

  One `Feature:` block, scenarios for B6.1 and (if B6.2 was kept) B6.2:

  ```gherkin
  Feature: Keypad player number display
    The keypad displays a player number, foul number, penalty number, or
    portal-login digit string to the right of a label. The digit string
    must render correctly regardless of length.

    @user_verified
    Scenario: Short digit string renders correctly
      Given the refbox is on a keypad page (player-number, foul, penalty,
        or portal-login)
      When the operator types one or two digits
      Then the digit string renders fully and is right-aligned in its row
      And the digit string does not vanish (the rendering bug pre-fix
        manifested as an empty render for short strings)

      # Session notes (filled by Task 7):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Digit text size is consistent across keypad variants
      Given the refbox is on a keypad page
      When the operator views any of the four keypad variants (player-number,
        foul, penalty, portal-login)
      Then the digit text renders at MEDIUM_TEXT size in all variants

      # Session notes (filled by Task 7):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM
  ```

  Omit the second scenario entirely if B6.2 became `@deleted` or `@findings-backlog`.

- [ ] **Step 6.3: Write `timed-buzzer-playback.feature`.**

  One `Feature:` block, scenarios for B6.4, B6.5, and B6.6:

  ```gherkin
  Feature: Timed buzzer playback
    The refbox plays a timed buzzer (~2.15 seconds + fade) when configured
    sound events fire. The buzzer must end cleanly (no click, no clipping
    distortion) and the fade-out must land in a neutral part of the waveform.

    @user_verified
    Scenario: Timed buzzer ends without an audible click
      Given the refbox is configured with a sound buzzer enabled
      When the operator triggers an event that fires a timed buzzer
      Then the buzzer plays for approximately 2.15 seconds
      And the fade-out at the end is smooth
      And no audible click or tap is heard at the moment of stop

      # Session notes (filled by Task 7):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Timed buzzer fade-out is not aligned with the buzzer's natural cycle
      Given the buzzer sound (Buzz / Whoop / Crazy) has a natural loop cycle
      When the timed buzzer's software fade-out runs
      Then the fade-out lands in a full-amplitude region of the buzzer's waveform
      And not at the start of a new loop cycle (which would re-attack
        as the gain ramps to zero)

      # Session notes (filled by Task 7):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Crazy buzzer body has no peak-clipping distortion
      Given the Crazy buzzer asset has been replaced (pre-fix peak amplitude 2.03)
      When the operator plays the Crazy buzzer at the default system volume
      Then no peak-clipping distortion is audible during the buzz body
      And the buzz character matches the operator's expectation of the
        Crazy sound

      # Session notes (filled by Task 7):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM
  ```

- [ ] **Step 6.4: Commit the three `.feature` files together.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes \
    && git add refbox/tests/features/timed-buzzer-playback.feature refbox/tests/features/multi-label-button-text.feature refbox/tests/features/keypad-player-number.feature \
    && git commit -m "docs(refbox): seed Gherkin scenarios for Unit 6 audit (sound / keypad / multi-label)"
  ```

---

## Task 7: Walkthrough verification (AUDIT-PLAN.md Step 7)

**Files:**
- Edit: `refbox/tests/features/multi-label-button-text.feature` (session notes)
- Edit: `refbox/tests/features/keypad-player-number.feature` (session notes)
- Edit: `refbox/tests/features/timed-buzzer-playback.feature` (session notes)

- [ ] **Step 7.1: Launch refbox from the audit worktree.**

  Claude runs (background, with `dangerouslyDisableSandbox: true` per memory `feedback_run_command`):

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes && WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox
  ```

  Wait for the refbox window to come up.

- [ ] **Step 7.2: Walkthrough Session — Multi-label buttons (UI-first per the spec's ordering decision).**

  Ask the operator to drive the refbox through state transitions that surface multi-label buttons. For each transition:
  - Confirm both lines of button text remain visible
  - Confirm no character is clipped against the button's edge
  - Note any state transition where the bug appears

  Mark each scenario in `multi-label-button-text.feature` as `@tested_pass`, `@tested_fail`, or `@tested_inconclusive` and record the walkthrough timestamp.

- [ ] **Step 7.3: Walkthrough Session — Keypad digit display.**

  Ask the operator to enter values on each of the four keypad variants (player-number, foul, penalty, portal-login). For each:
  - Type one digit, observe rendering — confirm digit appears right-aligned
  - Type three digits, observe rendering — confirm string appears right-aligned
  - Confirm text size is consistent (MEDIUM_TEXT) across all four variants (only if B6.2 was kept; otherwise skip the second sub-step)

  Mark each scenario in `keypad-player-number.feature` accordingly.

- [ ] **Step 7.4: Walkthrough Session — Timed buzzer playback.**

  Ask the operator to trigger a timed buzzer for each of the bundled sounds (Buzz, Whoop, Crazy). For each:
  - Listen for the 2.15s buzz body
  - Listen for an audible click at the moment of stop — there should be none
  - Listen for peak-clipping distortion during the Crazy buzzer body — there should be none

  Mark each scenario in `timed-buzzer-playback.feature` accordingly. If the Crazy buzzer still has distortion, flag B6.6 as `@redesign-followup` and file a Findings-Backlog entry.

- [ ] **Step 7.5: Finalize B6.6's status.**

  Update B6.6's `Status:` line in AUDIT-PLAN.md based on the listening verification outcome (was `@open — pending Task 7`).

- [ ] **Step 7.6: Stop the refbox process and commit the session notes.**

  Kill the background refbox run.

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes \
    && git add refbox/tests/features/multi-label-button-text.feature refbox/tests/features/keypad-player-number.feature refbox/tests/features/timed-buzzer-playback.feature \
    && git commit -m "docs(refbox): record Unit 6 walkthrough session notes"
  ```

---

## Task 8: ADR 005 amendment (AUDIT-PLAN.md Step 8)

**Files:**
- Modify: `docs/decisions/005-v040-feature-audit.md` (on branch `docs/workspace/backlog-adrs`, **not** the audit branch)

Per Unit 3 refinement #6: ADR finalization commits live on the ADR's own branch.

- [ ] **Step 8.1: Switch the main repo to `docs/workspace/backlog-adrs`.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs && git checkout docs/workspace/backlog-adrs`
  Confirm: `git rev-parse --abbrev-ref HEAD` returns `docs/workspace/backlog-adrs`.

- [ ] **Step 8.2: Append a "Verified by Unit 6 audit" subsection to ADR 005 entry 7.**

  In `docs/decisions/005-v040-feature-audit.md`, locate the `### 7. UI Text Clipping Fixes` section. Append after its `**Concern level: LOW**` paragraph:

  ```markdown
  **Verified by Unit 6 audit (YYYY-MM-DD):**
  Post-rebase commits on master are `edb4b9c` (keypad) and `8a8d018` (multi-label). The pre-rebase commit hashes referenced in the header (`cd577b2`, `2749104`) are the originals on `feat/workspace/desktop-build`; both branches reach the same surviving state.

  Catalog entries (in `AUDIT-PLAN.md` Unit 6):
  - B6.1 — Space-widget swap on keypad digit
  - B6.2 — Font-size simplification: `LARGE_TEXT` branch removed; all keypad pages render at `MEDIUM_TEXT`. **Disclosed in `edb4b9c`'s commit message but not described in this ADR's original text.** Operator decision recorded in AUDIT-PLAN.md Unit 6 Decisions: [keep | revert | findings-backlog].
  - B6.3 — Multi-label button container wrap

  Gherkin scenarios:
  - `refbox/tests/features/keypad-player-number.feature` — Scenarios for B6.1 and (conditionally) B6.2.
  - `refbox/tests/features/multi-label-button-text.feature` — Scenario for B6.3.

  Walkthrough verified YYYY-MM-DD on `audit/refbox/small-fixes-cluster`.
  ```

  Fill in the operator decision and the YYYY-MM-DD at execution time.

- [ ] **Step 8.3: Append a "Verified by Unit 6 audit" subsection to ADR 005 entry 11.**

  Similarly, in entry 11 (`### 11. Sound Artifacts Fix`), append after its `**Concern level: LOW**` paragraph:

  ```markdown
  **Verified by Unit 6 audit (YYYY-MM-DD):**
  Post-rebase commits on master are `7269c11` (sound artifacts) and `03d126c` (`is_some_and` clippy refactor of the line `7269c11` added). The pre-rebase commit hash referenced in the header (`701d12d`) is the original on `feat/workspace/desktop-build`; both branches reach the same surviving state.

  Catalog entries (in `AUDIT-PLAN.md` Unit 6):
  - B6.4 — `SOUND_LEN` 2.0 → 2.15s with cycle-alignment rationale comment
  - B6.5 — `Sound::stop()` `already_silent` early-exit
  - B6.6 — `crazy.raw` binary asset replaced (peak amplitude 2.03 → 1.0)
  - B6.7 — `is_some_and` clippy refactor on B6.5's added line (folded into B6.5's verification)

  Gherkin scenarios:
  - `refbox/tests/features/timed-buzzer-playback.feature` — Scenarios for B6.4, B6.5, B6.6.

  Walkthrough verified YYYY-MM-DD on `audit/refbox/small-fixes-cluster`. Operator confirmed the Crazy asset replacement audibly fixed the peak-clipping distortion described in bug 3.
  ```

- [ ] **Step 8.4: Commit the amendment.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs \
    && git add docs/decisions/005-v040-feature-audit.md \
    && git commit -m "docs(refbox): record Unit 6 audit verification on ADR 005 entries 7 and 11"
  ```

- [ ] **Step 8.5: Switch back to the audit worktree for Task 9.**

  No `git checkout` needed — Task 9 happens in the audit worktree at `.worktrees/audit-unit-6-small-fixes/`, which is separate from the main repo's current branch.

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Step 9)

**Files:**
- Modify: `AUDIT-PLAN.md` (gitignored; status flip + Completed audits summary)

- [ ] **Step 9.1: Operator reviews the decision log and test status.**

  Ask the operator to:
  1. Read the Unit 6 catalog (Decisions column) in AUDIT-PLAN.md
  2. Read the three `.feature` files for test-tag distribution
  3. Read the ADR 005 amendment subsections on `docs/workspace/backlog-adrs`
  4. Confirm: "Unit 6 approved" or request changes

- [ ] **Step 9.2: Flip status.**

  In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
  - Update the unit-catalog table row: `not started` → `complete-pending-integration (YYYY-MM-DD)`
  - Update the `### Unit 6 — Sound / keypad / multi-label fixes` section's `**Status:**` line the same way

- [ ] **Step 9.3: Add a summary entry to "Completed audits".**

  Per Unit 1 refinement #3 (in-place flip + summary pointer, not destructive section-move), add an entry to the Completed audits section near the bottom of AUDIT-PLAN.md, between the existing Unit 5 and Unit 4 entries (newest first):

  ```markdown
  #### Unit 6 — Sound / keypad / multi-label fixes — complete-pending-integration YYYY-MM-DD

  - **Branch:** `audit/refbox/small-fixes-cluster` (local only; <N> commits ahead of `origin/master`; not pushed)
  - **Per-unit plan:** `docs/superpowers/plans/2026-05-15-audit-unit-6-small-fixes.md`
  - **Audit-design spec:** `docs/superpowers/specs/2026-05-15-audit-unit-6-small-fixes-design.md`
  - **ADR (amended in place):** `docs/decisions/005-v040-feature-audit.md` entries 7 and 11; amendment commit on `docs/workspace/backlog-adrs`
  - **Scenarios:** Three `.feature` files: `timed-buzzer-playback.feature`, `multi-label-button-text.feature`, `keypad-player-number.feature` — <N> scenarios total; <X> @tested_pass, <Y> @tested_fail, <Z> @tested_inconclusive
  - **Catalog outcome:** 7 entries; <X> @user_verified; <Y> @deleted / @findings-backlog
  - **Tests added during audit:** none — sound + view-builder behaviour is walkthrough-verified per lean-process refbox-UI convention
  - **Audit commits on branch:** <list of SHAs>
  - **What was not verified:** <bullet list, or "nothing — all scenarios walkthrough-verified">
  - **Findings filed:** <N> new Findings backlog items
  - **Full details section:** retained in "Unit-by-unit details" above with status flipped to complete-pending-integration.
  ```

- [ ] **Step 9.4: Process refinements (if any).**

  If Unit 6 surfaced any playbook improvements, add an entry under "Process refinements log" → "From Unit 6 (YYYY-MM-DD)". Examples worth watching for:

  - First-time amendment-of-not-yet-accepted ADR (ADR 005 is "Phase 2 complete — scope confirmed", not the standard proposed/accepted lifecycle)
  - Listening-during-walkthrough as a verification mode for binary asset audits
  - Bundled-fix decomposition for visual-UI commits (the `LARGE_TEXT` → `MEDIUM_TEXT` pattern)

- [ ] **Step 9.5: Run `just check` on the audit branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-6-small-fixes && just check 2>&1 | tail -30
  ```

  Expected: green (fmt-check, clippy -D warnings, tests all pass; `cargo audit` reports the two pre-existing CVEs noted in Unit 3's Findings backlog #4 — not regressions).

- [ ] **Step 9.6: (Principal-only follow-up) Update Claude's memory files.**

  This step is performed by the principal Claude session, not an executing subagent — memory at `~/.claude/projects/.../memory/` is principal-only territory. After Step 9.5 reports green, the principal:

  - Marks Unit 6 as complete-pending-integration in `project_v040_handover.md`'s "Audit branches" list.
  - Updates audit progress count ("5 of 9 units complete" → "6 of 9 units complete").
  - Notes Unit 7 (PR #761 portal health) as next.
  - Updates `MEMORY.md` index if the handover entry's hook sentence needs to change.

- [ ] **Step 9.7: Confirm the unit is closed.**

  Tell the operator:

  > "Unit 6 complete-pending-integration. Branch `audit/refbox/small-fixes-cluster` holds locally with <N> audit commits. ADR 005 entries 7 and 11 amended on `docs/workspace/backlog-adrs`. AUDIT-PLAN.md status flipped. Ready to move on to Unit 7 (PR #761 portal health indicator)?"

---

## Risks and known divergences

Starting points for catalog questions, not pre-decisions:

1. **B6.2 font-size change is not in ADR 005's original text.** The commit message disclosed it ("Also simplify digit font size to MEDIUM_TEXT for all cases") but ADR 005 entry 7 doesn't mention it. The audit must elicit an explicit operator decision (keep / revert / findings-backlog) — Task 5 Step 5.1 handles this.
2. **`crazy.raw` listening verification is operator-dependent.** If the operator's audio hardware can't reliably distinguish peak-clipping distortion, the audit closes B6.6 as `@tested_inconclusive` and files a Findings-Backlog item recommending a separate listening session on better hardware.
3. **Pre-existing `sound_controller` unwrap debt.** If `7269c11` added new unwraps that match the surrounding module's debt pattern, file as Findings-Backlog whole-module candidate per Unit 4 refinement #3; do not flag on this audit branch.
4. **Post-rebase commit-hash chain.** The pre-rebase commits (`cd577b2`, `2749104`, `701d12d`) referenced in ADR 005's headers are on `feat/workspace/desktop-build` and not on master. The amendment subsection clarifies the pre/post correspondence — Unit 9 (stale branches) will resolve the legacy branches.
5. **ADR 005 amendment lifecycle.** ADR 005's `**Status:** Phase 2 complete — scope confirmed by human review 2026-04-17` is not the standard proposed/accepted lifecycle. The Unit 6 amendment adds verification subsections but does NOT change the Status line. If a Status update is needed, raise it with the operator in Task 8.

---

## Deviations

> Filled in during execution. Per lean process, deviations live here rather than as standalone commits.

**1. Baseline tip — plan said `82a370d`, actual cut at `089c98d` (2026-05-15, Task 1).**

The plan's references to `origin/master at 82a370d` reflected the principal's stale local master view. The actual `origin/master` tip when Task 1 ran was `089c98d` (a Renovate `chore(deps): bump rustls-webpki from 0.103.12 to 0.103.13` commit landed on 2026-04-24, three weeks before this audit). The audit worktree at `.worktrees/audit-unit-6-small-fixes/` is correctly anchored to the current origin/master tip. All four Unit 6 audit commits (`edb4b9c`, `8a8d018`, `7269c11`, `03d126c`) are reachable on the worktree HEAD. Audit scope is unaffected — Unit 6 audits four specific commits, not the tip.

---

## Files Created or Modified by This Plan

- `.worktrees/audit-unit-6-small-fixes/` (new worktree, lifecycle: removed at Final Integration)
- `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (gitignored; multiple edits)
- `.audit/unit-6-*.txt` (local working artifacts; not committed)
- `refbox/tests/features/timed-buzzer-playback.feature` (created on audit branch)
- `refbox/tests/features/multi-label-button-text.feature` (created on audit branch)
- `refbox/tests/features/keypad-player-number.feature` (created on audit branch)
- `docs/decisions/005-v040-feature-audit.md` (amended on `docs/workspace/backlog-adrs`, NOT on the audit branch)
- Memory `project_v040_handover.md` and `MEMORY.md` (updated at close)

---

## Estimated commits on the audit branch

- 1 scenario-seeding commit (Step 6.4)
- 1 walkthrough session-notes commit (Step 7.6)
- 0–1 prune commit (only if B6.2 becomes `@deleted` and the revert needs its own commit)
- **Total on `audit/refbox/small-fixes-cluster`:** 2–3 commits at close.

Plus, on `docs/workspace/backlog-adrs`:
- 1 ADR 005 amendment commit (Step 8.4).
