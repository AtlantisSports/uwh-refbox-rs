# Audit Unit 6 — Sound / Keypad / Multi-Label Fixes — Audit Design

**Date:** 2026-05-15
**Audit unit:** 6 — Sound / keypad / multi-label fixes
**Branch (to be cut):** `audit/refbox/small-fixes-cluster`, cut from master at `82a370d`
**Authoritative reference:** `docs/decisions/005-v040-feature-audit.md` entries 7 (UI Text Clipping Fixes) and 11 (Sound Artifacts Fix)
**Playbook:** `AUDIT-PLAN.md` (gitignored working document)

---

## Purpose

This document captures the brainstormed shape of Unit 6 of the AI Code Audit. Unit 6 is a cluster of four small bug-fix commits from the v0.4.0 release cycle. ADR 005 already documents these fixes at a feature level — Unit 6's job is to verify that the post-rebase commits faithfully implement what ADR 005 describes, surface any scope that leaked in during the rebase, and finalize ADR 005 entries 7 and 11 with "Verified by Unit 6 audit" subsections.

This spec is the input to the per-unit plan that will be written next via `superpowers:writing-plans`. The per-unit plan decomposes the playbook's 9-step workflow into Unit-6-specific tasks. Per `.claude/rules/plan-execution.md`, the lean process applies (refbox-only, lower-risk).

---

## Scope

### In scope

The four post-rebase fix commits on master between `edb4b9c` (2026-04-15) and `03d126c` (2026-04-17):

| Commit | Date | Subject |
|---|---|---|
| `edb4b9c` | 2026-04-15 | fix(refbox): fix keypad player number display using space widget |
| `8a8d018` | 2026-04-15 | fix(refbox): fix multi-label button text clipping on state transitions |
| `7269c11` | 2026-04-17 | fix(refbox): fix sound artifacts in timed buzzer playback |
| `03d126c` | 2026-04-17 | fix(refbox): use is_some_and instead of map_or(false, ...) in sound controller |

Files in scope:
- `refbox/src/sound_controller/mod.rs` — `Sound::stop()` method, `SOUND_LEN` constant, `already_silent` logic, `is_some_and` line.
- `refbox/resources/sounds/crazy.raw` — binary audio asset (replacement for the peak-amplitude-clipping original, per ADR 005 entry 11 bug 3). Verified by listening during walkthrough.
- `refbox/src/app/view_builders/shared_elements.rs` — `make_multi_label_button` and the `centered_text` helper insofar as the multi-label code path changes its use.
- `refbox/src/app/view_builders/keypad_pages/mod.rs` — `build_keypad_page` digit-display path, including the font-size simplification.

### Out of scope

- The broader `sound_controller` codebase (queue logic, alarm dispatch, settings serialization) — touched only as needed to read context around `Sound::stop()`.
- The full keypad-page module (all keypad variant builders, callbacks) — only the digit-display path is in scope.
- Any `shared_elements.rs` helpers other than `make_multi_label_button` (and `centered_text` insofar as it's used by `make_multi_label_button`).
- The pre-rebase commits on `feat/workspace/desktop-build` and `fix/refbox/centered-text-clipping` (`cd577b2`, `2749104`, `701d12d`). Unit 9 (stale branches cleanup) will resolve those.
- ADR 005 entries other than 7 and 11.

### Why the post-rebase hashes differ from ADR 005's referenced hashes

ADR 005 was written 2026-04-17 against the v0.4.0 consolidation branch `feat/workspace/desktop-build`. Its entries 7 and 11 reference the pre-rebase commit hashes (`cd577b2`, `2749104`, `701d12d`). When the fixes were later landed on master, the rebase produced new hashes (`edb4b9c`, `8a8d018`, `7269c11`). Unit 6 audits the master versions. The "Verified by Unit 6 audit" subsections on ADR 005 record the post-rebase hashes so future readers can trace either way.

---

## Acceptance criteria

The unit is complete-pending-integration when all of the following are true:

1. **Catalog complete.** The Unit 6 behaviour catalog (in AUDIT-PLAN.md) covers all post-rebase changes in the four commits, including bundled sub-behaviours in `7269c11` and `edb4b9c`. Each entry's `Spec status` line references ADR 005 entry 7 or 11, or is marked `not-in-spec` if the audit surfaces something ADR 005 didn't capture.
2. **All catalog entries reach a terminal status.** `@user_verified`, `@deleted`, `@redesign-followup`, or `@findings-backlog`. No `@open` entries remain.
3. **Three Gherkin `.feature` files seeded** in `refbox/tests/features/` per playbook line 379 — one per `Feature:` block.
4. **Walkthrough verified — three observable outcomes:**
   - **Sound.** Operator triggers a 2.15s timed Crazy buzzer; confirms (a) no audible click at the end, (b) no peak-clipping distortion during the buzz body, (c) the buzz duration feels correct (~2 seconds + fade).
   - **Multi-label buttons.** Operator runs through state transitions that surface multi-label buttons (start-clock, end-half, time-out, score-confirm); confirms text never clips or vanishes after a transition.
   - **Keypad.** Operator enters single-digit and multi-digit values on the player-number, foul, penalty, and portal-login keypads; confirms the digit displays correctly right-aligned and the text size is consistent (`MEDIUM_TEXT` across all variants).
5. **ADR 005 amended in place.** On branch `docs/workspace/backlog-adrs`: entries 7 and 11 each gain a "Verified by Unit 6 audit (2026-MM-DD)" subsection listing the post-rebase commit hashes plus pointers to the catalog and `.feature` files.
6. **AUDIT-PLAN.md updated.** Unit 6 status flipped to `complete-pending-integration`; summary entry added to "Completed audits" section. Findings-Backlog entries filed if any.
7. **`just check` passes** on the audit branch tip with zero clippy/test failures.

---

## Architectural sketch

### Expected behaviour catalog (~7 entries)

Following Unit 5's B-entry pattern, decomposing the bundled commits per Unit 5 refinement #4:

| # | Source commit | Behaviour | Spec status |
|---|---|---|---|
| B6.1 | `edb4b9c` | Space-widget swap fixes short-string display on keypad digit | matches ADR 005 entry 7 bug 1 |
| B6.2 | `edb4b9c` | Font-size simplification: `LARGE_TEXT` branch removed; all keypad digit displays render at `MEDIUM_TEXT` | **not-in-spec** (ADR 005 entry 7 doesn't mention the font change) |
| B6.3 | `8a8d018` | Multi-label button wrap-in-container + `centered_text` → `text(...).align_x(Horizontal::Center)` swap | matches ADR 005 entry 7 bug 2 |
| B6.4 | `7269c11` | `SOUND_LEN` 2.0 → 2.15s with cycle-alignment rationale comment | matches ADR 005 entry 11 bug 1 |
| B6.5 | `7269c11` | `Sound::stop()` adds `already_silent` early-exit so the stop-fade doesn't burst the gain back to full after a scheduled end | matches ADR 005 entry 11 bug 2 |
| B6.6 | `7269c11` | `crazy.raw` binary asset replaced (peak amplitude 2.03 → 1.0) | matches ADR 005 entry 11 bug 3 |
| B6.7 | `03d126c` | `map_or(false, …)` → `is_some_and(…)` clippy refactor on B6.5's added line | not-in-spec (post-spec stylistic cleanup; folded into B6.5's verification) |

**B6.2 is the heads-up entry.** It's a behaviour change honestly disclosed in `edb4b9c`'s commit message ("Also simplify digit font size to MEDIUM_TEXT for all cases") but absent from ADR 005 entry 7's description. The audit needs an explicit operator decision: keep the font change as desirable, revert it, or flag as Findings-Backlog. Per Unit 5 refinement #4, this is the expected outcome of decomposing a bundled-fix commit.

### Gherkin `.feature` file structure

Three files in `refbox/tests/features/`, per playbook line 379:

| File | Feature block | Covers |
|---|---|---|
| `timed-buzzer-playback.feature` | `Feature: Timed buzzer playback` | B6.4, B6.5, B6.6 |
| `multi-label-button-text.feature` | `Feature: Multi-label button text` | B6.3 |
| `keypad-player-number.feature` | `Feature: Keypad player number display` | B6.1, B6.2 |

Each `@user_verified` catalog entry becomes one or more `Scenario:` blocks in its file. Once a scenario is copied into the `.feature` file, all further test status — tags and session notes — lives in the `.feature` file, not in AUDIT-PLAN.md (per playbook Step 6).

### Slop-catching focus

Apply the playbook's standard checklist with two unit-specific focuses:

1. **Pre-existing `sound_controller` unwrap debt (Unit 4 refinement #3).** If `7269c11` introduced new unwraps, distinguish "new latent debt" from "pattern-consistent with module debt". The latter goes to Findings-Backlog for the module as a whole, not flagged on this audit branch.
2. **B6.2 / B6.7 are the bundled-fix surface area.** Per Unit 5 refinement #4, they get separate B-entries even though the parent commit subject lines don't fully describe them.

### Test coverage plan

No new Rust unit tests expected:
- Sound playback is async + Web Audio context dependent; `Sound::stop()` resists direct unit-testing.
- View builders are conventionally walkthrough-verified, not unit-tested.
- This matches the lean-process refbox-UI pattern from Units 3, 4, 5.

If the audit surfaces something testable that's not already covered, add a test — but don't force-add tests for behaviours that resist unit-testing.

### ADR 005 amendment plan

A single commit on `docs/workspace/backlog-adrs` (per Unit 3 refinement #6) adds two subsections to `docs/decisions/005-v040-feature-audit.md`:

- **Entry 7 — "Verified by Unit 6 audit (2026-MM-DD)" subsection.** Lists post-rebase commits `edb4b9c` and `8a8d018`. Notes that the pre-rebase hashes (`cd577b2`, `2749104`) referenced in the entry header are the originals on `feat/workspace/desktop-build`. References catalog B6.1, B6.2, B6.3 and `.feature` files `keypad-player-number.feature`, `multi-label-button-text.feature`. **Calls out B6.2's font-size change as a behaviour-shift disclosed in the commit message but absent from the original ADR text** — records the operator's keep/revert decision.
- **Entry 11 — "Verified by Unit 6 audit (2026-MM-DD)" subsection.** Lists post-rebase commits `7269c11` and `03d126c`. Notes the pre-rebase hash (`701d12d`). References catalog B6.4–B6.7 and `.feature` file `timed-buzzer-playback.feature`. Notes the walkthrough confirmed the Crazy asset replacement audibly fixed the clipping distortion described in bug 3.

Commit message format: `docs(refbox): record Unit 6 audit verification on ADR 005 entries 7 and 11`.

### Files & branches summary

| File / artefact | Lives on branch | Action |
|---|---|---|
| `AUDIT-PLAN.md` (root, gitignored) | working tree | Unit 6 catalog + decisions + scenarios filled; Completed audits + Findings-Backlog updated |
| `refbox/tests/features/timed-buzzer-playback.feature` | `audit/refbox/small-fixes-cluster` | Created |
| `refbox/tests/features/multi-label-button-text.feature` | `audit/refbox/small-fixes-cluster` | Created |
| `refbox/tests/features/keypad-player-number.feature` | `audit/refbox/small-fixes-cluster` | Created |
| `docs/decisions/005-v040-feature-audit.md` | `docs/workspace/backlog-adrs` | Amended (two "Verified by Unit 6 audit" subsections) |

---

## Task list (rough seams)

Per `.claude/rules/plan-execution.md` lean process — these are seams, not step-by-step scripts.

| # | Task | Outcome |
|---|---|---|
| 1 | Worktree + branch setup. Cut `audit/refbox/small-fixes-cluster` from master `82a370d`. Create worktree at `.worktrees/audit-unit-6-small-fixes/`. | Audit branch and isolated worktree ready. |
| 2 | History reconstruction. Walk each of the four commits' diffs end-to-end; cross-reference against ADR 005 entries 7 and 11 to map post-rebase hashes back to pre-rebase intent. Note any divergence. | Short history-trace section in AUDIT-PLAN.md Unit 6. |
| 3 | Build behaviour catalog (B6.1–B6.7). Fill Unit 6's `### Behaviour catalog` subsection with the seven expected entries. Each entry: source commit, behaviour summary, files/lines, `Spec status` line, `Status: @open`. | Catalog complete and ready for per-entry review. |
| 4 | Slop-catching pass. Apply playbook checklist with the two unit-specific focuses. Annotate entries with "Why it might be slop" / "Recommendation" pairs where applicable. | Catalog enriched with slop analysis; tentative Findings-Backlog items identified. |
| 5 | Per-entry operator decisions. Walk the 7 entries one at a time. Special attention to B6.2 (font-size change not in ADR 005) and B6.6 (Crazy asset listening verification deferred to Task 7 walkthrough). | All entries reach terminal status (or carry an explicit "pending Task 7 walkthrough" note). Findings-Backlog items filed in AUDIT-PLAN.md. |
| 6 | Write Gherkin `.feature` files. Three files in `refbox/tests/features/`. Each `@user_verified` catalog entry becomes `Scenario:` blocks. | Three executable spec files committed on audit branch. |
| 7 | Walkthrough verification. Claude launches `cargo run -p refbox` (background, `dangerouslyDisableSandbox: true`); operator drives all three observable outcomes from Acceptance Criteria. | Each catalog entry's status finalized; walkthrough-surfaced issues filed to Findings-Backlog. |
| 8 | ADR 005 amendment. Switch to `docs/workspace/backlog-adrs`; add the two "Verified by Unit 6 audit (date)" subsections. Single commit. | ADR 005 finalization commit on backlog-adrs branch. |
| 9 | AUDIT-PLAN.md close-out. Flip Unit 6 status to `complete-pending-integration`. Add summary entry to `### Completed audits`. Update memory if needed. | Unit 6 closed; ready for Final Integration alongside Units 1–5. |
| 10 | `just check` on audit branch tip. | Zero clippy/test failures. |

### What's deliberately not in the plan

Per `.claude/rules/plan-execution.md` lean process:
- **No per-task code review.** One review at the end via `superpowers:requesting-code-review` if needed before Final Integration.
- **No per-task deviation commits.** Track deviations in the per-unit plan's `## Deviations` section, not as standalone commits.
- **No per-task verification ceremony for mechanical bits.** Compilation + `just check` is enough for catalog edits, AUDIT-PLAN.md edits, and `.feature` file creation.

---

## Why these choices

- **One branch (not split).** The four commits don't share code dependencies that would block individual cherry-picking during Final Integration. Splitting into 2 or 4 branches would quadruple audit-process overhead for trivial gain.
- **Verify-against-ADR-005 (not from-scratch).** ADR 005 already captured the design intent. Unit 3 refinement #4 establishes the finalize-in-place pattern for ADRs being audited; Unit 6 applies it here.
- **Listen-during-walkthrough for `crazy.raw`.** The asset is co-designed with the `SOUND_LEN` tuning (the comment in `7269c11` references the Crazy 0.667s cycle). Auditing one without the other leaves the audit incomplete. Per the operator's decision 2026-05-15.
- **ADR amendment on `docs/workspace/backlog-adrs`, not the audit branch.** Unit 3 refinement #6 — ADR finalization commits live on the ADR's existing branch.
- **Lean process.** Per `.claude/rules/plan-execution.md`, refbox-only bug-fix work uses lean ceremony. No per-task review, no per-task deviation commits.

---

## Open questions for the per-unit plan

The per-unit plan should resolve these tactical specifics that this spec deliberately leaves open:

1. **Catalog review rhythm.** With 7 entries (≤15 threshold per Unit 3 refinement #3), per-entry approval is the default. The per-unit plan can confirm whether to bundle B6.4–B6.7 (all sound-related, sharing one Feature block) into a single approval question or keep them strictly per-entry.
2. **B6.2 decision pre-walkthrough vs. post-walkthrough.** The font-size simplification can be evaluated by reading code alone, but the operator may want to see it on the running app before deciding keep/revert. The per-unit plan should pick one.
3. **Walkthrough ordering.** Sound → multi-label → keypad vs. UI-first → sound-last. Per-unit plan should set a default; the operator can vary at session time.

---

## References

- **Playbook:** `AUDIT-PLAN.md` Section "Audit unit catalog" (Unit 6) and Section "Steps for each unit"
- **Authoritative ADR:** `docs/decisions/005-v040-feature-audit.md` entries 7 and 11
- **Prior unit precedent for ADR finalize-in-place:** `docs/decisions/009-settings-navigation-layout.md` (Unit 3 audit finalization)
- **Prior unit precedent for retroactive ADR with cross-branch dependencies:** `docs/decisions/022-referee-name-display.md` (Unit 5, on branch `audit/refbox/referee-names`)
- **Process refinements applied:** Unit 3 refinements #4 (ADR finalize-in-place) and #6 (ADR amendment on its own branch); Unit 4 refinement #3 (pre-existing latent debt distinction); Unit 5 refinement #4 (decompose bundled-fix commits into multiple B-entries)
- **Lean process:** `.claude/rules/plan-execution.md`
- **Communication & approval gates:** `.claude/rules/communication.md`, `.claude/rules/scope.md`
