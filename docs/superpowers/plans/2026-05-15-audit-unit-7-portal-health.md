# Audit Unit 7 — Portal Health Indicator: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` (subagent-driven execution is the default for Unit 7 because Task 3 catalog construction spans five subsystem groups that decompose cleanly into parallel subagent dispatches; the principal session walks the user through Task 5 reviews and Task 7 walkthrough). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit the 46 feature commits between `3ce6bdd` (portal_manager scaffolding) and `0a5cc2e` (clean stale comments + tokio feature) on `feat/refbox/portal-health-indicator`. Verify the shipped behaviour against ADR 011 + its 4 amendments, surgically remove anything that doesn't survive operator review, seed Gherkin scenarios for what does, and finalize ADR 011 in place with a "Verified by Unit 7 audit" section. PR #761 stays untouched throughout — the audit branch is local-only until Final Integration.

**Architecture:** Diff-led catalog grouped into five subsystem blocks per the approved spec at [docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md](docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md) Section 4. Expected catalog size 50–80 entries. Subagent-driven catalog construction (one subagent per subsystem group writing to its own `.audit/unit-7-<group>.md` working file, principal merges into AUDIT-PLAN.md). Page-batched review with ambiguity carve-outs per Unit 3 refinement #3. Heavy process per `.claude/rules/plan-execution.md` — refbox state machine, portal communication, UI surface, and `app/mod.rs` rewiring all trigger full ceremony. ADR 011 finalized in place (proposed → accepted + new Verified-by section) per Unit 3 refinement #4 since the ADR is still `proposed`.

**Tech Stack:** Rust 2024 / MSRV 1.85; iced 0.13 (refbox UI); tokio (async portal background task); confy (config persistence — colocated with portal queue file); reqwest (portal HTTP client in `uwh-common`); Fluent (translation keys); `cargo test`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `just check`; manual refbox launch (`UWH_PORTAL_URL_OVERRIDE=http://localhost:5000 WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox` with `dangerouslyDisableSandbox:true`); Gherkin scenarios at one new `.feature` file `refbox/tests/features/portal-health.feature`; local uwh-portal API + DB at `localhost:5000` (source at `/home/estraily/projects/uwh-portal/`).

**Testing approach:**

- **Existing tests retained as oracle.** The audit window adds substantial unit tests under `refbox/src/portal_manager/` (queue round-trip, retry-eligibility, indicator-state, health-check cadence). The audit treats these as part of the catalog (each test gets a B-entry where it encodes behavioural intent beyond mechanical assertion) and as pruning support (a deleted behaviour's tests get deleted alongside).
- **No new Rust unit tests expected.** Portal communication is HTTP + tokio-task driven and is walkthrough-verified, not unit-tested. View builders are walkthrough-verified.
- **Operator-driven walkthrough on local uwh-portal** for Gherkin scenarios in Task 7, one extended session covering ADR 011 amendment 2026-04-22's three primary scenarios (token-valid green path, mid-tournament network drop → yellow → red, induced 409 conflict → force/discard) plus the four amendments' acceptance proofs (dormant-until-event-linked, no-green-checkmark, translation coverage, paragraph-cache pattern).
- **`just check` on the audit branch tip** before close. Cross-platform parity inherited from feat-branch CI history.

---

## Acceptance criteria (Unit 7 "complete-pending-integration")

Unit 7 is complete-pending-integration when **all** of these hold on the audit branch `audit/refbox/portal-health`:

1. A behaviour catalog exists in `AUDIT-PLAN.md` under Unit 7, grouped into five subsystem blocks (portal_manager module · app/mod.rs rewiring · UI surface · tests + assets · env-var hooks), with every entry tagged `@user_verified`, `@deleted`, `@findings-backlog`, or `@redesign-followup`. No `@open` or `@proposed` remaining.
2. Every `@user_verified` operator-observable behaviour is captured as a Gherkin scenario in `refbox/tests/features/portal-health.feature`.
3. Each scenario carries `@user_verified` plus a test-state tag (`@tested_pass`, `@tested_fail`, or `@tested_inconclusive`) and a manual-walkthrough timestamp in a session-notes comment.
4. The operator has driven the refbox UI through one Session covering ADR 011 amendment 2026-04-22's three primary scenarios on local uwh-portal at `localhost:5000`, plus the four amendments' acceptance proofs.
5. `just check` passes on the audit branch (`fmt-check`, `clippy -D warnings`, all tests pass, `cargo audit` clean — the two pre-existing dependency vulnerabilities noted in Unit 3's Findings backlog #4 are expected and not regressions).
6. ADR 011 finalized in place on `docs/workspace/backlog-adrs` (per Unit 3 refinement #6 — ADR commit lands on the ADR's branch, not the audit branch). Status flipped `proposed` → `accepted (verified by Unit 7 audit YYYY-MM-DD)`. Four amendments preserved verbatim. New "Verified by Unit 7 audit (YYYY-MM-DD)" section appended listing audit scope, verified decisions per ADR section + amendment, supersessions (if any), what was removed during audit, and what was not verified.
7. The branch holds locally — no push, no PR opened or updated. PR #761 on `feat/refbox/portal-health-indicator` remains at its current state, untouched. Per `prs-deferred-until-audit-done` memory.
8. `AUDIT-PLAN.md` status flipped from "not started" to "complete-pending-integration (YYYY-MM-DD)" in both the catalog table line 64 and the unit section heading; summary pointer added to "Completed audits" section per playbook-amended Step 9.4.
9. Findings discovered out-of-scope are recorded in `AUDIT-PLAN.md`'s Findings backlog with a suggested follow-up branch name. They are **not fixed** on this branch.
10. Process refinements surfaced during execution are logged in `AUDIT-PLAN.md`'s "Process refinements log → From Unit 7".
11. Claude's memory files updated: `project_v040_handover.md` records Unit 7 complete-pending-integration; audit progress count incremented; Unit 8 noted as next.

---

## Prerequisites

- The user has approved this per-unit plan before any Task 1 step runs.
- Working tree on the current branch (`docs/workspace/backlog-adrs`) is clean except for the gitignored `.claude/scheduled_tasks.lock`.
- Read the approved design spec: [docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md](docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md) (commit `39225ce`).
- Read `AUDIT-PLAN.md` Unit 7 section + the playbook's Per-unit workflow + Templates + the Process refinements log entries from Units 1–6. Particularly relevant to Unit 7:
  - Unit 1 refinement #2 (`.feature` files live in `refbox/tests/features/`)
  - Unit 1 refinement #6 (Bash cwd doesn't persist between calls — always `cd` into worktree)
  - Unit 1 refinement #7 (refbox launch needs `WAYLAND_DISPLAY= RUST_LOG=info`)
  - Unit 2 refinement #2 (bundled-fix decomposition into separate B-entries)
  - Unit 2 refinement #4 (composite-agent cross-crate verification — Unit 7 only verifies one crate so not directly applicable, but the pattern guides Task 7's batch ordering)
  - Unit 3 refinement #3 (page-batched + ambiguity carve-outs for catalogs ≥25 entries — Unit 7 fits)
  - Unit 3 refinement #4 (ADR finalize-in-place pattern — applies to ADR 011 because it's `proposed`)
  - Unit 3 refinement #5 (`confy::store(..).unwrap()` audit pattern — relevant for the portal queue file's persistence layer)
  - Unit 3 refinement #6 (ADR amendment commit on its own branch, not the audit branch)
  - Unit 4 refinement #2 (commit-fan-out check — Unit 7's commit list has at least one heavy-fan-out commit per the spec's risk register)
  - Unit 4 refinement #3 (pre-existing pattern-consistent debt → Findings backlog, not audit-branch fix)
  - Unit 4 refinement #4 (diff-led catalog construction even when a spec exists — ADR 011 is oracle, not source)
  - Unit 5 refinement #1 (search `git log --all -S '<symbol>'` before brainstorming any uwh-common fix — Unit 7 expects no uwh-common changes; if any are surfaced, this rule fires)
  - Unit 5 refinement #3 (cross-branch dependency detection)
  - Unit 5 refinement #4 (bundled-fix decomposition for behaviour changes, not just bug fixes)
  - Unit 6 refinement #3 (cross-unit code evolution awareness — `make_game_time_button` in `shared_elements.rs` and `app/mod.rs` are touched across multiple units)
  - Unit 6 refinement #4 (don't kill-loop a working process — check the memory rule's `why`)
- Read `docs/decisions/011-portal-health-indicator.md` (the audit's primary oracle) + its 4 amendments.
- Read `docs/superpowers/plans/2026-04-19-portal-health-indicator.md` (the audit's secondary oracle — original implementation plan with 22 tasks).
- Read `.claude/rules/scope.md`, `communication.md`, `workspace.md`, `rust.md`, `embedded.md` (informational — Unit 7 does not touch wireless-remote), `pr-review.md`, `plan-execution.md`.
- Pre-commit hook at `<main-repo>/.git/hooks/pre-commit` must allow `audit/` branch type (fixed by Unit 1's `2a8dcbc`, inherited locally via the audit branches). Verify in Task 1 Step 3.
- Memory `feedback_prs_deferred_until_audit_done` is in force: do not propose, suggest, or execute any PR/merge during this unit. PR #761 stays untouched.
- Memory `feedback_refbox_wsl_wayland_unset` is in force: native `cargo run -p refbox` under WSLg requires `WAYLAND_DISPLAY=` prefix.
- Memory `feedback_user_drives_refbox_ui` is in force: Claude launches refbox in background with `dangerouslyDisableSandbox:true`; operator drives the UI.
- Memory `feedback_check_rule_applicability` is in force: before killing a process or retrying a tool call, read its output and check whether the memory rule's `why` actually applies.
- Local uwh-portal source at `/home/estraily/projects/uwh-portal/` (per memory `reference_uwh_portal_source`); operator boots the API + DB before Task 7's walkthrough.

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:**
- Create: `.worktrees/audit-unit-7-portal-health/` (new worktree)
- Edit: `AUDIT-PLAN.md` (gitignored, no commit)

- [ ] **Step 1.1: Confirm working tree is clean.**

  Run: `git -C /home/estraily/projects/uwh-refbox-rs status --short`
  Expected: empty output, or only `?? .claude/scheduled_tasks.lock` (gitignored harness file).

- [ ] **Step 1.2: Ask the user for explicit approval to cut the audit branch in a worktree.**

  Surface to the user:
  - Branch: `audit/refbox/portal-health`
  - Worktree: `.worktrees/audit-unit-7-portal-health/`
  - Cut from: `feat/refbox/portal-health-indicator` HEAD at `0a5cc2e` — **NOT** from master. This is the explicit playbook deviation for Unit 7 so PR #761 stays untouched during the audit.

  Wait for approval.

- [ ] **Step 1.3: Verify the pre-commit hook allows the `audit` branch type.**

  Run from main repo root: `grep -c '\baudit\b' .git/hooks/pre-commit`
  Expected: non-zero count. If zero, copy the audit-aware version from Unit 1's branch:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs && git show audit/refbox/confirm-score-timing:scripts/pre-commit > .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
  ```

- [ ] **Step 1.4: Create the worktree at the feature-branch tip.**

  Run from main repo root:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs && git worktree add -b audit/refbox/portal-health .worktrees/audit-unit-7-portal-health feat/refbox/portal-health-indicator
  ```
  Expected output: `Preparing worktree (new branch 'audit/refbox/portal-health')`.

- [ ] **Step 1.5: Verify the worktree HEAD matches the feature-branch tip.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health && git rev-parse HEAD`
  Expected: `0a5cc2eb...` (full SHA starts with `0a5cc2e`).

  Run: `git log --oneline -1`
  Expected: `0a5cc2e chore(refbox): clean stale comments and dev-only tokio feature`.

- [ ] **Step 1.6: Sanity-check the worktree builds.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health && cargo build -p refbox 2>&1 | tail -5`
  Expected: clean build (or only the workspace's pre-existing warnings if any). Cargo will fetch and compile dependencies; first build may take 5–10 minutes.

- [ ] **Step 1.7: Update the Unit 7 status in AUDIT-PLAN.md to "in progress".**

  In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
  - In the unit-catalog table (around line 64), change `not started` → `in progress (started YYYY-MM-DD)` for Unit 7.
  - In the `### Unit 7 — PR #761 portal health indicator` section (around line 3136), change `**Status:** not started` → `**Status:** in progress (started YYYY-MM-DD)`.

  No commit — AUDIT-PLAN.md is gitignored.

- [ ] **Step 1.8: Pre-flight local uwh-portal availability check (deferrable to Task 7).**

  Surface to the user: "Local uwh-portal pre-flight check — can be done now or deferred to Task 7. Recommended: defer to Task 7 so the catalog work can proceed independently of portal availability."

  If the operator wants to pre-flight now:
  - Boot uwh-portal API + DB from `/home/estraily/projects/uwh-portal/` (operator action).
  - Run: `curl -fsS http://localhost:5000/api/health 2>&1` (or whichever health endpoint uwh-portal exposes).
  - Expected: 200 OK response.

  Otherwise, mark as deferred and move on.

---

## Task 2: History reconstruction (AUDIT-PLAN.md Step 2)

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 7 section (gitignored, no commit)
- Optional: `.audit/unit-7-*.txt` working artifacts (local-only, never committed)

- [ ] **Step 2.1: Capture the audit-window commit list.**

  From the worktree:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health \
    && mkdir -p .audit \
    && git log --oneline master..HEAD > .audit/unit-7-commits-raw.txt
  ```

  Verify line count: `wc -l .audit/unit-7-commits-raw.txt` should report **47**. The 47th line is `089c98d` (Renovate rustls-webpki — out of scope per the spec).

  Extract the in-scope 46 commits (everything except `089c98d`):
  ```bash
  grep -v '^089c98d' .audit/unit-7-commits-raw.txt > .audit/unit-7-commits.txt && wc -l .audit/unit-7-commits.txt
  ```
  Expected: `46 .audit/unit-7-commits.txt`.

- [ ] **Step 2.2: Capture commit metadata (chronological).**

  Per Unit 6 refinement #2, do NOT use `git log --no-walk --reverse` — that doesn't actually reverse. Use a for-loop:

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health \
    && hashes=$(git log --reverse --format=%H master..HEAD | grep -v '^089c98d') \
    && for h in $hashes; do git log -1 --format='%H%n%ai%n%s%n%n%b%n---END---%n' $h; done > .audit/unit-7-commit-messages.txt
  ```

  Verify: `grep -c '^---END---$' .audit/unit-7-commit-messages.txt` should report `46`.

- [ ] **Step 2.3: Capture file-touch list across the audit window.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health \
    && git diff --name-only $(git merge-base master HEAD)..HEAD | grep -v '^$' | sort > .audit/unit-7-files-touched.txt
  ```

  Inspect: `wc -l .audit/unit-7-files-touched.txt` and `cat .audit/unit-7-files-touched.txt`.

  Expected files (high-confidence prediction; verify against actual):
  - `refbox/src/portal_manager/*` (new module: `mod.rs`, `queue.rs`, `health.rs`, `state.rs`, possibly `task.rs`)
  - `refbox/src/app/mod.rs`
  - `refbox/src/app/message.rs`
  - `refbox/src/app/view_builders/shared_elements.rs` (time-banner integration)
  - `refbox/src/app/view_builders/portal_detail.rs` (new file, detail page)
  - `refbox/src/app/view_builders/portal_attention.rs` (new file, attention page)
  - `refbox/src/app/view_builders/portal_token_expired.rs` (new file then later refactored — see commit `bd4f2ca`)
  - `refbox/src/app/theme/mod.rs`
  - `refbox/translations/*/refbox.ftl` (15 locale files)
  - `refbox/assets/uwh-portal-logo-compact.svg` (or similar)
  - `refbox/Cargo.toml` (tempfile dev-dep at `d4b5504`)
  - `Cargo.lock`
  - Possibly tests under `refbox/src/portal_manager/*` (inline `#[cfg(test)]` modules)

  Flag anything outside this prediction for explicit attention in the catalog.

- [ ] **Step 2.4: Verify NO touches outside refbox.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health \
    && git diff --name-only $(git merge-base master HEAD)..HEAD | grep -v '^refbox/' | grep -v '^Cargo.lock$' | grep -v '^089c98d'
  ```
  Expected: empty output (only `refbox/...` and `Cargo.lock` should appear).

  If anything else appears (notably `uwh-common/`, `overlay/`, `wireless-remote/`, `schedule-processor/`), record it as a high-priority slop candidate — ADR 011 explicitly states no changes outside refbox.

- [ ] **Step 2.5: Group commits by subsystem.**

  Read the commit list and bucket each of the 46 commits into one of the five subsystem groups defined in the spec:

  - **A: portal_manager module** — commits that touch only `refbox/src/portal_manager/*` (scaffold types, queue, health, state, retry rules, task, public API).
  - **B: app/mod.rs rewiring** — commits that touch `refbox/src/app/mod.rs` to route through portal_manager (game-end routing, SelectedEventId writer, login-keypad-panic fix, snapshot push, dormant gate, advisory).
  - **C: UI surface** — commits that touch view builders, theme, message variants, assets, translations (tile, detail page, attention page, token-expired page, advisory, mode-aware logo, externalized strings, layout passes).
  - **D: Tests + assets** — commits whose primary content is test code or non-code assets (the compact logo SVG, removed `check_circle.svg`, tempfile dev-dep).
  - **E: Env-var hooks** — commits that add `UWH_PORTAL_URL_OVERRIDE`, `UWH_PORTAL_SCRAMBLE_TOKEN`, dev-only tokio feature.

  Write the grouping to `.audit/unit-7-grouping.md`. Allow commits to belong to multiple groups (e.g., `e6c3954` "externalize strings to translations" is mostly C but touches almost every UI file).

- [ ] **Step 2.6: Map commits to ADR 011 sections and amendments.**

  For each commit, identify which ADR 011 section it realizes or amendment it implements:
  - Decision section → Status indicator
  - Decision section → Detail page
  - Decision section → Per-item action pages
  - Decision section → portal_manager module
  - Decision section → Conflict resolution
  - Amendment 2026-04-21 (conflict refinement — single attention page, Yellow replaces Orange)
  - Amendment 2026-04-22 (translation coverage + local-portal verification env)
  - Amendment 2026-04-23a (dormant until event linked)
  - Amendment 2026-04-23b (remove 10-second green-checkmark overlay)
  - **Unmapped** (commit doesn't trace to any ADR section or amendment — primary slop candidate)

  Append the mapping to `.audit/unit-7-grouping.md`.

- [ ] **Step 2.7: Append a history-trace subsection to AUDIT-PLAN.md Unit 7.**

  Under `### Unit 7 — PR #761 portal health indicator`, add `#### History trace` with:
  - One paragraph per subsystem group (A–E) summarizing what changed across the group's commits.
  - The unmapped commits (if any) flagged individually with a one-line "Why this might be slop" hypothesis.
  - The total commit-count check (46 in scope; 1 out of scope = `089c98d`).

  No commit — AUDIT-PLAN.md is gitignored.

---

## Task 3: Build behaviour catalog (AUDIT-PLAN.md Step 3) — subagent-driven

This task uses **five subagent dispatches**, one per subsystem group. Each subagent reads its group's commit diffs, applies the slop-catching checklist, and writes its B-entries to a per-group working file. The principal then merges the five files into a single ordered catalog section in AUDIT-PLAN.md.

**Parallel vs sequential:** Subagents read the worktree read-only and write to disjoint working files (`.audit/unit-7-group-{A,B,C,D,E}.md`) — no shared state. Per `dispatching-parallel-agents`, all five dispatches can be issued in a single message for speed (~5–10 min wall clock vs ~25 min sequential), provided the principal is comfortable interpreting five concurrent reports. If the principal prefers checkpoint-able pacing, run them sequentially (one Agent tool call per Step 3.2–3.6). Either is acceptable; choose at dispatch time.

**Files:**
- Create: `.audit/unit-7-group-A-portal-manager.md`
- Create: `.audit/unit-7-group-B-app-mod.md`
- Create: `.audit/unit-7-group-C-ui-surface.md`
- Create: `.audit/unit-7-group-D-tests-assets.md`
- Create: `.audit/unit-7-group-E-env-vars.md`
- Edit: `AUDIT-PLAN.md` Unit 7 section (gitignored, no commit)

- [ ] **Step 3.1: Create the behaviour catalog subsection scaffold in AUDIT-PLAN.md.**

  Under `### Unit 7 — PR #761 portal health indicator` (after the history trace from Task 2.7), add `#### Behaviour catalog` with five sub-headings — one per group:

  ```markdown
  #### Behaviour catalog

  ##### Group A: portal_manager module

  (filled by subagent dispatch in Step 3.2)

  ##### Group B: app/mod.rs rewiring

  (filled by subagent dispatch in Step 3.3)

  ##### Group C: UI surface

  (filled by subagent dispatch in Step 3.4)

  ##### Group D: Tests + assets

  (filled by subagent dispatch in Step 3.5)

  ##### Group E: Env-var hooks

  (filled by subagent dispatch in Step 3.6)
  ```

- [ ] **Step 3.2: Dispatch subagent for Group A (portal_manager module).**

  Use `Agent({subagent_type: "general-purpose", description: "Build Group A catalog", prompt: <full task brief>})`. The prompt must include:

  - **Context:** Audit Unit 7 of the AI Code Audit. Working in `/home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health/`. Oracle is `docs/decisions/011-portal-health-indicator.md` + 4 amendments + `docs/superpowers/plans/2026-04-19-portal-health-indicator.md`.
  - **Scope:** Commits in `.audit/unit-7-grouping.md` under Group A (portal_manager module). Read each commit's diff from `git show <hash>` and build one B-entry per distinct behaviour.
  - **Expected entries:** 15–25 covering scaffold types, queue file format + atomic load/save, retry-eligibility rules, health-check cadence decision (5min green / 15sec yellow-red), indicator-state computation, PortalTaskIo trait, public API + event channel, background task spawn, queue-snapshot wiring, error handling on startup I/O failure, dead_code-allow removal.
  - **Slop-catching checklist:** Apply playbook's slop-catching section. Flag bundled fixes (Unit 2 #2 / Unit 5 #4), new unwraps without justification (Unit 3 #5 — special attention to `confy::store` pattern and any disk-I/O unwrap), internal helpers not called from real code, defaults/clamping, side effects.
  - **Per-entry shape:** Use the template at `AUDIT-PLAN.md` "Behaviour catalog entry" section. Each entry has: name, source commit, files+lines, behaviour summary in plain English, **Spec status:** `matches-spec | diverges-from-spec | not-in-spec | realises-amendment`, why-it-might-be-slop annotation, recommendation, Status: `@open`.
  - **Output path:** Write to `.audit/unit-7-group-A-portal-manager.md` in the worktree.
  - **Do NOT:** edit `AUDIT-PLAN.md` directly, do not write to `docs/decisions/`, do not modify any code, do not run cargo or git mutating commands beyond `git show <hash>` and `git log --no-pager`.
  - **Return:** a short report listing the number of entries written, the count of `matches-spec` / `diverges-from-spec` / `not-in-spec` / `realises-amendment`, and a one-line note for each `diverges-from-spec` or `not-in-spec` entry.

  Run in foreground (the principal waits for the result before proceeding).

- [ ] **Step 3.3: Dispatch subagent for Group B (app/mod.rs rewiring).**

  Same shape as Step 3.2 but:
  - **Scope:** Group B commits from `.audit/unit-7-grouping.md`.
  - **Expected entries:** 5–10.
  - **Special attention:** Cross-unit code in `app/mod.rs`. Per Unit 6 refinement #3, every entry on `app/mod.rs` must include a cross-unit note. Run `git log <hash>..HEAD -- refbox/src/app/mod.rs` for each entry to detect post-audit-window commits on the same region. Cross-units to watch: Unit 1 (confirm-score), Unit 3 (settings), Unit 4 (manual alarm).
  - **Output path:** `.audit/unit-7-group-B-app-mod.md`.

  Run in foreground.

- [ ] **Step 3.4: Dispatch subagent for Group C (UI surface).**

  Same shape as Step 3.2 but:
  - **Scope:** Group C commits from `.audit/unit-7-grouping.md`.
  - **Expected entries:** 15–25.
  - **Special attention:**
    - Cross-unit code in `shared_elements.rs` and `make_game_time_button`. Per Unit 6 refinement #3, every entry on these regions must include a cross-unit note. Cross-units to watch: Unit 5 (referee names — already merged into shared_elements.rs region), Unit 8 (CJK/Thai font support — affects per-line container wrap).
    - Translation key adds in commit `e6c3954` across 15 locale files. Catalog the keys themselves (the keys list, not per-locale duplication), the parametrization pattern (e.g. `portal-summary-issues` simple form per amendment 2026-04-22), and any key that's now orphaned (no longer referenced after a later commit deleted its call site).
    - The token-expired action page added by `3cfb137` was later retired/refactored by `bd4f2ca` (drop intermediate token-expired action page). Catalog BOTH the add and the retirement as separate B-entries; the retirement is itself a behaviour change.
    - The 10-second green-checkmark overlay added in an earlier commit was removed by `b6b095b` per amendment 2026-04-23b. Catalog the add as `realises-amendment` (was: planned then removed) and the removal as `realises-amendment` (matches the amendment).
    - The `check_circle.svg` asset was added then removed in the same window. Catalog only the surviving net (asset doesn't exist on HEAD).
  - **Output path:** `.audit/unit-7-group-C-ui-surface.md`.

  Run in foreground.

- [ ] **Step 3.5: Dispatch subagent for Group D (tests + assets).**

  Same shape as Step 3.2 but:
  - **Scope:** Group D commits from `.audit/unit-7-grouping.md`.
  - **Expected entries:** 5–10.
  - **Test entries:** For each `#[cfg(test)]` module or `#[test]` function added in the audit window, catalog one B-entry per *distinct* behavioural assertion (not per `assert!` line). Mechanical assertions (e.g. "this struct round-trips through JSON") are one entry. Behavioural assertions (e.g. "the retry rule returns `Ready` exactly when delay has elapsed and not before") are individual entries. If a test encodes a behaviour the operator may not have intended (e.g., a specific clamping bound), flag as ambiguous-by-design.
  - **Asset entries:** Compact UWH Portal logo (`3ce6bdd`); any other assets if present. The `check_circle.svg` net-zero is already covered in Group C; don't double-catalog.
  - **Output path:** `.audit/unit-7-group-D-tests-assets.md`.

  Run in foreground.

- [ ] **Step 3.6: Dispatch subagent for Group E (env-var hooks).**

  Same shape as Step 3.2 but:
  - **Scope:** Group E commits from `.audit/unit-7-grouping.md` (`UWH_PORTAL_URL_OVERRIDE`, `UWH_PORTAL_SCRAMBLE_TOKEN`, dev-only tokio feature).
  - **Expected entries:** 2–4.
  - **Special attention:** Env vars are an unusual behaviour surface — they create a hidden test-only mode that ships in production binaries. For each env var: catalog what it changes when set, what it does when unset (default behaviour), and whether the audit-window code gates its effect behind a debug-only/dev-only feature flag.
  - **Output path:** `.audit/unit-7-group-E-env-vars.md`.

  Run in foreground.

- [ ] **Step 3.7: Merge the five group files into AUDIT-PLAN.md Unit 7 catalog.**

  Renumber entries to a single B-series per the spec's expected size (B7.1 through B7.N, where N is 50–80). Group A entries get the lowest numbers (B7.1 onward), then B, then C, then D, then E. Inside each group, preserve the order the subagent produced.

  Cross-reference any inter-group dependency in a one-line note (e.g., "B7.43 depends on B7.5's queue file shape — both must keep/delete together").

- [ ] **Step 3.8: Sanity-check total catalog size.**

  Count B-entries: `grep -c '^##### B7\.' /home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`.
  Expected: 50–80.
  - If **< 50**: subagents may have under-decomposed bundled fixes. Re-read one of the heavier commits (`d5f485a`-class) and check whether the subagent caught its multi-behaviour content.
  - If **> 80**: per the spec's Risk R4, pause and re-batch before Step 4. Re-group entries by tighter subsystem boundaries (e.g., split UI surface into "indicator tile + sizing" / "detail page" / "attention page" / "advisory + per-game") before moving to slop-catching.

- [ ] **Step 3.9: Surface the catalog size and shape to the operator.**

  Report:
  - Total entry count
  - Distribution by group (A: N, B: N, C: N, D: N, E: N)
  - Distribution by Spec status (matches-spec / diverges-from-spec / not-in-spec / realises-amendment)
  - List of `diverges-from-spec` and `not-in-spec` entries (these become carve-out review questions in Task 5)

  Wait for operator acknowledgement before proceeding to Task 4.

---

## Task 4: Slop-catching pass (AUDIT-PLAN.md Step 4 prep — by the principal)

The five subagents already applied the slop-catching checklist per-group. Task 4 is the principal's cross-group consolidation pass.

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 7 section (gitignored, no commit)

- [ ] **Step 4.1: Apply the cross-group slop-catching checklist.**

  Read the consolidated catalog and look for cross-group patterns that the per-group subagents may have missed:

  - **Commit-fan-out check (Unit 4 refinement #2).** For each commit that contributed entries to ≥3 groups, flag the commit message vs. its actual fan-out. Example to watch: `e6c3954` (translation externalization) likely spans Groups C, D. Heavier fan-out commits warrant a meta-entry under "Cross-cutting findings".
  - **Bundled-fix decomposition (Unit 5 refinement #4).** For each commit whose subject line is one concern but whose body or diff reveals additional concerns, confirm the catalog has separate B-entries for each. Particularly watch the `fix(refbox)` commits in the window — `fix` titles often hide a behaviour shift.
  - **Cross-branch dependency detection (Unit 5 refinement #3).** Run `git log --all -S '<symbol>'` for each catalog entry that references a symbol whose origin is unclear (e.g., a type imported from `uwh-common` whose definition pre-dates the audit window). If any catalog entry depends on code added on a different branch, record it as a cross-branch dep.
  - **Cross-unit code awareness (Unit 6 refinement #3).** Verify Group B and Group C subagents added cross-unit notes for all `app/mod.rs` and `shared_elements.rs` entries. If any are missing, add them.
  - **Unwrap audit (Unit 3 refinement #5).** Compile a list of all new `.unwrap()` / `.expect()` calls in the audit window. Classify each as: safe-by-construction / new-latent-debt / pattern-consistent-with-module-debt / I/O-failure-risk. The I/O-failure-risk category (notably anything on the portal queue file save path) becomes a carve-out review question in Task 5.
  - **Unmapped commits.** Any commit from Task 2.6 that didn't map to an ADR section or amendment is a primary slop candidate. Confirm each one's catalog entries are flagged `not-in-spec` and have a "Why it might be slop" annotation.

- [ ] **Step 4.2: Add a "Cross-cutting findings" subsection.**

  Under the Behaviour catalog, add:

  ```markdown
  ##### Cross-cutting findings

  - **CF7.1 — Heavy-fan-out commit `<hash>`** — touches groups <A,B,C>; per-entry B-entries are <B7.X, B7.Y, B7.Z>. Recommendation: confirm with operator that this fan-out was intended and not the bundled-feature anti-pattern from Unit 4 #2.
  - **CF7.2 — I/O-failure unwraps in portal queue save** — list of lines. Recommendation: present to operator as a single carve-out for module-wide error-handling review (Findings backlog) vs. surgical fix on audit branch.
  - **CF7.3 — Unmapped commits** — list with one-line hypothesis each.
  ```

  Fill in real hashes and entries at execution time.

- [ ] **Step 4.3: Sanity-check coverage against ADR 011.**

  For each ADR 011 section and amendment, list the catalog entries that realize it. Any section with **zero** realising entries is a coverage gap — the code may not actually implement that part of the ADR. Add a note under "Cross-cutting findings" for any such gap (`CF7.K — ADR 011 section <name> has no realising catalog entry — verify with operator whether the code is missing or the catalog under-cataloged`).

  Common gaps to watch:
  - Conflict resolution (ADR 011 Decision section "Conflict resolution") — was substantially refined by amendment 2026-04-21. The "Conflict" name doesn't survive; the surviving shape is the single attention action page. Coverage should be via the attention-page B-entries plus the retry-eligibility B-entry.
  - Token expired flow — was added then refactored. Coverage should be via the token-expired-state-on-indicator B-entry + the login-keypad-panic-fix B-entry (which is the surviving navigation hook).

---

## Task 5: Per-entry operator decisions (AUDIT-PLAN.md Step 4 review — page-batched)

This task uses **page-batched review with ambiguity carve-outs** per Unit 3 refinement #3, not per-entry. Total expected questions: 12–20.

**Files:**
- Edit: `AUDIT-PLAN.md` Unit 7 section (gitignored, no commit)

- [ ] **Step 5.1: Present the catalog summary to the operator.**

  Read aloud (in plain English):
  - Total entry count
  - Distribution by group (A–E) with one-sentence summary per group of what it covers
  - Distribution by Spec status
  - List of carve-out candidates (every `diverges-from-spec`, every `not-in-spec`, plus any entry the slop-catching pass annotated with a non-trivial "Why it might be slop")

  Wait for operator acknowledgement.

- [ ] **Step 5.2: Page-batched approval — Group A (portal_manager module).**

  Present:
  - Plain-English summary of what Group A covers (the new module's responsibilities: persistent retry queue, dual-cadence health check, indicator-state computation, retry-eligibility rules, background task with PortalTaskIo trait).
  - One-line per entry: `B7.N — <short name> — <Spec status> — <recommendation>`.
  - Recommendation per entry: keep / delete / move-to-findings-backlog.

  Ask: "Approve this group as recommended, or carve out specific entries for individual review?"

  Operator responds with either "approve all as recommended" OR "approve all except B7.X, B7.Y" (which become carve-out questions in Step 5.7).

- [ ] **Step 5.3: Page-batched approval — Group B (app/mod.rs rewiring).**

  Same shape as Step 5.2, scoped to Group B. Plain-English summary: how the existing game-end path was rerouted through portal_manager, where the SelectedEventId writer was wired in, what the dormant-until-event-linked gate gates, how the confirm-score advisory appears when red.

- [ ] **Step 5.4: Page-batched approval — Group C (UI surface).**

  Same shape, scoped to Group C. Plain-English summary: the indicator tile on the time banner, the detail page (scrollable list + back), the attention action page (force / discard), the per-game advisory banner on confirm-score, mode-aware portal logo + sport prefix in strings, paragraph-cache pattern for portal row text, externalized translations.

  Because Group C is the largest, the operator may want the carve-out list expanded in this group specifically. The token-expired action page (add at `3cfb137` then retirement at `bd4f2ca`) is a near-certain carve-out — present its B-entry pair as a single decision: "The token-expired action page was added and then refactored away within the audit window. Both add and retirement entries are kept (they document the design evolution); the *surviving* state is no separate page — the indicator goes red and tapping the row routes through the portal-login flow. Operator: approve this surviving shape as the verified design, or revisit?"

- [ ] **Step 5.5: Page-batched approval — Group D (tests + assets).**

  Same shape, scoped to Group D. Plain-English summary: unit tests under `portal_manager/` covering queue round-trip, retry-eligibility, indicator state computation, health cadence; the compact UWH Portal logo asset.

- [ ] **Step 5.6: Page-batched approval — Group E (env-var hooks).**

  Same shape, scoped to Group E. Plain-English summary: the three env-var hooks for dev/test (`UWH_PORTAL_URL_OVERRIDE` for pointing at local portal, `UWH_PORTAL_SCRAMBLE_TOKEN` for inducing token rejection in test, dev-only tokio feature).

  Particular question for the operator: "These env-var hooks ship in production binaries. Approve them as audit-friendly debug knobs, or carve them out for a separate decision about gating them behind a `debug_assertions` cfg?"

- [ ] **Step 5.7: Carve-out questions — one per ambiguous-by-design entry.**

  For each entry the operator carved out from Steps 5.2–5.6, plus each `not-in-spec` entry, plus each entry CF7.* identified as cross-cutting concern: present a standalone question.

  Standard form per carve-out:
  - Behaviour summary in plain English
  - Spec status + why it's ambiguous
  - Recommendation with reason
  - Three options: keep / delete (surgical pruning in Task 6) / move-to-findings-backlog
  - **Special handling for any I/O-failure unwrap entry:** present the three options with a fourth: surgical fix on the audit branch (replace the unwrap with a graceful error path) — only if the operator wants the audit to absorb the fix rather than punt it.

- [ ] **Step 5.8: Update each B-entry's `Status:` line with the operator decision.**

  Each `Status:` line becomes one of: `@user_verified`, `@deleted`, `@findings-backlog`, or `@redesign-followup`. No `@open` may remain.

- [ ] **Step 5.9: File any Findings-Backlog items inline.**

  If any B-entry became `@findings-backlog`, add a `#### From Unit 7 (YYYY-MM-DD)` subsection to `AUDIT-PLAN.md`'s `### Findings backlog` section listing each finding with a suggested branch name. Cross-reference the B-entry ID.

- [ ] **Step 5.10: Surface the post-decision catalog state to the operator.**

  Report:
  - Final count by Status: `<X> @user_verified`, `<Y> @deleted`, `<Z> @findings-backlog`, `<W> @redesign-followup`.
  - If any `@deleted` exists, list them — Task 6 will perform surgical pruning.
  - If zero `@deleted`, Task 6 is a no-op except for the final `just check` confirmation.

---

## Task 6: Surgical pruning (AUDIT-PLAN.md Step 5)

**Files:**
- Modify: whichever audit-branch files the `@deleted` behaviours live in (varies per Task 5 outcome)
- Edit: `AUDIT-PLAN.md` Unit 7 section (gitignored, no commit per pruning step; AUDIT-PLAN.md is gitignored throughout)

- [ ] **Step 6.1: For each `@deleted` behaviour, identify the prune set.**

  For each B-entry marked `@deleted`:
  - Read the entry's `Files / lines:` field.
  - Identify any test that asserts the deleted behaviour (search via `grep` for symbol names, function names, or assertion targets).
  - Identify any translation key, asset, or message variant that's exclusively used by the deleted behaviour.
  - Identify any downstream consumer in the audit window (e.g., a different B-entry that depends on this one).

  Produce a per-deleted-entry checklist before any edit.

- [ ] **Step 6.2: Prune behaviours one at a time.**

  For each `@deleted` behaviour, in dependency order (consumers before producers — so the producer can be cleanly removed once its consumers are gone):

  - **Step 6.2.a: Edit the audit-branch source.** Use the Edit tool with the worktree path prefix `/home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health/`. Remove the behaviour's code surgically. Do NOT reflow surrounding code unless required to maintain compilation.
  - **Step 6.2.b: Remove any test that exclusively asserted the deleted behaviour.** Tests that assert *related* behaviour (where the deleted assertion is one of many) are amended, not removed.
  - **Step 6.2.c: Remove any translation key, asset, or message variant uniquely tied to the deleted behaviour.** Confirm via `grep` that no other code references the removed identifier.
  - **Step 6.2.d: Run `just fmt-check` in the worktree.** Expected: clean.
  - **Step 6.2.e: Run `just lint` in the worktree.** Expected: clippy `-D warnings` clean.
  - **Step 6.2.f: Run `just test` in the worktree.** Expected: all surviving tests pass (the ones whose deleted-behaviour assertions were removed should no longer exist; everything else passes).
  - **Step 6.2.g: Commit the prune.** Branch-type `audit`. Subject: `audit(refbox): remove <B7.N short name>`. Body references the catalog entry and lists files touched.

- [ ] **Step 6.3: Surface the pruning summary to the operator.**

  Report:
  - Number of prune commits
  - Each prune commit's subject line
  - Net lines removed
  - Confirm `just check` (full) passes on the audit-branch tip.

  Skip Step 6.2 entirely if zero `@deleted` entries; jump straight here with "No surgical pruning required — catalog had zero `@deleted` entries."

---

## Task 7: Test pass + walkthrough verification (AUDIT-PLAN.md Step 6)

**Files:**
- Create: `refbox/tests/features/portal-health.feature` (on audit branch)
- Edit (later, after walkthrough): `refbox/tests/features/portal-health.feature` (session notes)

- [ ] **Step 7.1: Run `just check` on the audit-branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health && just check 2>&1 | tail -40
  ```

  Expected: `fmt-check` clean, `clippy -D warnings` clean, all tests pass, `cargo audit` reports the two pre-existing CVEs from Unit 3's Findings backlog #4 (not regressions).

  If anything fails: stop, diagnose, fix on the audit branch, re-run. Per Unit 6 refinement #4, don't retry-loop a process whose output you haven't read.

- [ ] **Step 7.2: Write `refbox/tests/features/portal-health.feature`.**

  Create the file with one `Feature:` block and one `Scenario:` per `@user_verified` operator-observable behaviour. Use the spec's behaviour descriptions; do not invent scenarios beyond what catalog entries cover.

  Skeleton structure (fill in scenarios from the Task 5 outcome):

  ```gherkin
  Feature: Portal health indicator
    A clickable tile on the left end of the time banner shows whether
    the refbox is successfully communicating with the UWH Portal. The
    tile appears only when a portal event is linked. When red, it
    signals that operator attention is needed.

    Background:
      Given the refbox is launched with UWH_PORTAL_URL_OVERRIDE=http://localhost:5000
      And a test event exists on the local uwh-portal instance
      And the operator has logged in and linked the test event

    @user_verified
    Scenario: Green path — successful submission lands silently
      Given the portal indicator is showing green
      When the operator ends a game and confirms the score
      Then the score is submitted to the portal
      And the portal indicator stays green
      And no operator-facing dialog interrupts the flow

      # Session notes (filled by Step 7.5):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Network drop — indicator escalates from green to yellow to red
      Given the portal indicator is showing green
      When the network connection to the portal is interrupted
      Then the next health check fails
      And the indicator transitions to yellow within one health-check cycle
      And if failures persist for 30 minutes, the indicator escalates to red

      # Session notes (filled by Step 7.5):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Induced 409 conflict — operator chooses force or discard
      Given the portal indicator is showing red because a queued submission has been stuck for 30 minutes
      When the operator taps the indicator
      Then the detail page opens listing the stuck submission
      And tapping the stuck row opens the attention action page
      And the operator can choose FORCE THIS GAME RESULT or DISCARD THIS SUBMISSION
      And choosing FORCE resubmits with force=true and clears the queue entry on success
      And choosing DISCARD removes the queue entry without resubmission

      # Session notes (filled by Step 7.5):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Dormant — indicator is hidden when no event is linked
      Given no portal event is currently linked
      Then the portal indicator tile is not rendered on the time banner
      And the confirm-score advisory banner does not appear when ending a game
      And no background health check produces a 404 in the log

      # Session notes (filled by Step 7.5):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM

    @user_verified
    Scenario: Confirm-score advisory — red indicator surfaces a warning before submitting
      Given the portal indicator is showing red
      When the operator reaches the confirm-score screen at the end of a game
      Then a red advisory banner appears warning that submissions are not landing
      And the operator can still confirm the score (the submission queues for retry)

      # Session notes (filled by Step 7.5):
      # @tested_pass | @tested_fail | @tested_inconclusive
      # walkthrough: YYYY-MM-DD HH:MM
  ```

  **Add more scenarios** as the Task 5 outcome dictates — every `@user_verified` operator-observable behaviour gets a scenario. Backend-only behaviours (queue file format, retry-eligibility rules in isolation) do NOT get scenarios; they're verified by the unit tests already in the catalog.

- [ ] **Step 7.3: Commit the `.feature` file.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health \
    && git add refbox/tests/features/portal-health.feature \
    && git commit -m "audit(refbox): seed Gherkin scenarios for Unit 7 portal health audit"
  ```

- [ ] **Step 7.4: Pre-flight local uwh-portal (if not done in Task 1.8).**

  Surface to the operator: "Time to boot the local uwh-portal API + DB. Source at `/home/estraily/projects/uwh-portal/`. Per the project's dev guide, this is typically `npm run dev` or equivalent. Need API listening on port 5000 and a test event created in the local DB."

  Wait for operator confirmation that local portal is up.

  Quick check (Claude runs):
  ```bash
  curl -fsS http://localhost:5000/api/health 2>&1
  ```
  If this fails or hangs: present the spec's Risk R5 fallback to dev.uwhportal.com to the operator. Decision happens here, not mid-walkthrough.

- [ ] **Step 7.5: Launch refbox from the audit worktree.**

  Claude runs (background, with `dangerouslyDisableSandbox: true` per memory `feedback_run_command`):

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health && UWH_PORTAL_URL_OVERRIDE=http://localhost:5000 WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox
  ```

  Wait for the refbox window to come up. Confirm the operator can see the window and the time-banner area.

- [ ] **Step 7.6: Walkthrough Session — Scenario 1 (Green path).**

  Ask the operator to:
  - Confirm the portal indicator tile is visible on the time banner (green dot, UWH Portal logo above)
  - Start a game on the test event, advance to game-end, confirm the score
  - Verify the score appears in the local uwh-portal's web UI for that event
  - Confirm the indicator stayed green throughout

  Mark the Scenario 1 entry in `portal-health.feature` as `@tested_pass` / `@tested_fail` / `@tested_inconclusive` with timestamp.

- [ ] **Step 7.7: Walkthrough Session — Scenario 2 (Network drop).**

  Ask the operator to:
  - Stop the local uwh-portal process (or block port 5000 via firewall) while a game is in progress
  - Watch the indicator: should transition green → yellow within one health-check cycle (~5 min by default, or ~15 sec if cadence-decision already short)
  - Leave the failure in place; the indicator should escalate to red after 30 min of continuous queue failures
  - For walkthrough efficiency, the operator may temporarily reduce the 30-min threshold in code or accept a `@tested_inconclusive` on the 30-min escalation specifically while marking the yellow-transition `@tested_pass`

  Mark Scenario 2 with the operator's outcome.

- [ ] **Step 7.8: Walkthrough Session — Scenario 3 (Induced 409 conflict).**

  Ask the operator to:
  - Restart the local uwh-portal
  - Submit a game's score successfully (this puts a record on the portal)
  - In the local portal's web UI, edit that game's score to a different value (forces a 409 on the next attempt)
  - In the refbox, force a resubmission of the same game (operator-known path; or wait for a queued retry)
  - Observe the queued submission stuck-state — does it transition to a red indicator after 30 min, or does the audit-window's logic surface it sooner?
  - Tap the indicator → detail page → tap the stuck row → attention action page
  - Choose `FORCE THIS GAME RESULT`: confirm the resubmission lands and the row clears
  - Reproduce the 409 once more and choose `DISCARD THIS SUBMISSION`: confirm the row clears without resubmission

  Mark Scenario 3 with the operator's outcome.

- [ ] **Step 7.9: Walkthrough Session — Scenario 4 (Dormant).**

  Ask the operator to:
  - Unlink the portal event in the refbox (via Portal Login / Select Event flow)
  - Confirm the indicator tile is no longer rendered on the time banner
  - Reach the confirm-score screen at the end of a game; confirm no advisory banner
  - Check the refbox log: no `404` errors from `verify_token` calls

  Mark Scenario 4 with the operator's outcome.

- [ ] **Step 7.10: Walkthrough Session — Scenario 5 (Confirm-score advisory).**

  Ask the operator to:
  - Relink the portal event
  - Re-induce a red state (the simplest is to stop the local portal and wait)
  - Start and end a game; reach the confirm-score screen
  - Confirm the red advisory banner appears warning that submissions are not landing
  - Confirm the operator can still confirm the score (queues for retry)

  Mark Scenario 5 with the operator's outcome.

- [ ] **Step 7.11: Walkthrough Session — any additional scenarios from Task 7.2.**

  For each additional scenario seeded in `portal-health.feature` beyond the five core scenarios above: drive the operator through it and mark the outcome.

- [ ] **Step 7.12: Stop the refbox process and commit the session notes.**

  Kill the background refbox run.

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health \
    && git add refbox/tests/features/portal-health.feature \
    && git commit -m "audit(refbox): record Unit 7 walkthrough session notes"
  ```

- [ ] **Step 7.13: Final `just check` on the audit-branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health && just check 2>&1 | tail -40
  ```

  Expected: still green. If any test fails *after* the prune commits and the walkthrough commits, something regressed during the audit — investigate before proceeding.

- [ ] **Step 7.14: Update any catalog entries that the walkthrough resolved.**

  If any B-entry was marked `@open — pending Task 7 walkthrough` (analogous to Unit 6's B6.6 listening-verification), update its `Status:` line with the verified outcome.

---

## Task 8: ADR 011 finalization (AUDIT-PLAN.md Step 7)

**Files:**
- Modify: `docs/decisions/011-portal-health-indicator.md` (on branch `docs/workspace/backlog-adrs`, **not** the audit branch)

Per Unit 3 refinement #6: ADR finalization commits live on the ADR's own branch.

- [ ] **Step 8.1: Switch the main repo to `docs/workspace/backlog-adrs`.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs && git checkout docs/workspace/backlog-adrs`
  Confirm: `git rev-parse --abbrev-ref HEAD` returns `docs/workspace/backlog-adrs`.

- [ ] **Step 8.2: Flip the ADR 011 status line.**

  In `docs/decisions/011-portal-health-indicator.md`, line 4:
  - Before: `**Status:** proposed`
  - After: `**Status:** accepted (verified by Unit 7 audit YYYY-MM-DD)`

  Fill in the actual close date.

- [ ] **Step 8.3: Confirm the four amendments are preserved.**

  Do NOT edit the four amendment subsections. Read each to confirm they remain verbatim:
  - `### 2026-04-21 — Conflict handling refined after API verification`
  - `### 2026-04-22 — Translation coverage and verification environment`
  - `### 2026-04-23 — Dormant until event linked`
  - `### 2026-04-23 — Remove the 10-second green-checkmark overlay`

- [ ] **Step 8.4: Append the "Verified by Unit 7 audit" section.**

  After the last amendment, append:

  ```markdown
  ## Verified by Unit 7 audit (YYYY-MM-DD)

  ### Audit scope

  - **AUDIT-PLAN.md section:** Unit 7 — PR #761 portal health indicator
  - **Audit branch:** `audit/refbox/portal-health` (local-only; <N> commits ahead of `feat/refbox/portal-health-indicator`)
  - **Commit range audited:** `3ce6bdd..0a5cc2e` (46 commits; `089c98d` Renovate excluded)
  - **Walkthrough date:** YYYY-MM-DD
  - **Walkthrough environment:** local uwh-portal API + DB at `localhost:5000`

  ### Verified decisions

  - **Status indicator (Decision section):** verified by entries <B7.X> (tile rendering), <B7.Y> (indicator state computation), <B7.Z> (dormant-until-event-linked gate).
  - **Detail page (Decision section):** verified by entries <B7.A> (detail page scaffolding), <B7.B> (row ordering), <B7.C> (paragraph-cache pattern).
  - **Per-item action pages (Decision section, refined by amendment 2026-04-21):** verified by entries <B7.D> (single attention action page), <B7.E> (force / discard actions).
  - **portal_manager module (Decision section):** verified by entries <B7.F> through <B7.G> (Group A entries).
  - **Conflict resolution (Decision section, refined by amendment 2026-04-21):** verified by entries <B7.H> (retry-eligibility), <B7.I> (no dedicated Conflict state — all non-success → Pending).
  - **Amendment 2026-04-21:** verified by entries <B7.J>, <B7.K> (single attention page, Yellow replaces Orange).
  - **Amendment 2026-04-22 (translation coverage):** verified by entries <B7.L> through <B7.M> (translation keys added across 15 locales).
  - **Amendment 2026-04-22 (verification environment):** verified by Task 7 walkthrough running against local uwh-portal at `localhost:5000`.
  - **Amendment 2026-04-23 (dormant until event linked):** verified by entries <B7.N>, <B7.O> (tile-visibility gate, advisory suppression).
  - **Amendment 2026-04-23 (remove 10-second green-checkmark overlay):** verified by entries <B7.P>, <B7.Q> (overlay removal, retained-only red-exclamation overlay).

  ### Supersessions

  - <List any catalog entry whose `@user_verified` decision contradicts an ADR section or amendment. Format: "Entry B7.X supersedes amendment YYYY-MM-DD paragraph N — operator decided in walkthrough to <change>". Empty section if none.>

  ### What was removed during audit

  - <One bullet per `@deleted` catalog entry with one-line reason. Empty section if zero deletions.>

  ### What was not verified

  - <Explicit gaps. Examples: "Real-portal-data shape against dev.uwhportal.com was not exercised; only local portal walkthrough." "The 30-minute escalation timer was verified at a reduced threshold during walkthrough; full-cadence verification deferred to next tournament.">

  ### Audit reference

  - **Per-unit plan:** `docs/superpowers/plans/2026-05-15-audit-unit-7-portal-health.md`
  - **Audit-design spec:** `docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md`
  - **Gherkin scenarios:** `refbox/tests/features/portal-health.feature` on audit branch
  ```

  Fill in real entry numbers and dates at execution time. The placeholder `<B7.X>` markers are filled with the actual catalog IDs from Task 3.

- [ ] **Step 8.5: Commit the ADR amendment on `docs/workspace/backlog-adrs`.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs \
    && git add docs/decisions/011-portal-health-indicator.md \
    && git commit -m "docs(refbox): finalize ADR 011 and record Unit 7 audit verification"
  ```

- [ ] **Step 8.6: Stay on `docs/workspace/backlog-adrs` for Task 9.**

  Task 9 also operates from this branch, but since `AUDIT-PLAN.md` is gitignored throughout, Task 9's edits to it produce no commits. The only Task 9 actions that touch the git tree are the memory-related file writes in Step 9.7, which live outside the repo. The audit worktree at `.worktrees/audit-unit-7-portal-health/` is separate and unaffected by the current branch on the main repo.

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Steps 8 + 9)

**Files:**
- Modify: `AUDIT-PLAN.md` (gitignored; status flip + Completed audits summary + Process refinements + Findings backlog)
- Modify: memory files at `/home/estraily/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/`

- [ ] **Step 9.1: Operator reviews the decision log, test status, and ADR amendment.**

  Ask the operator to:
  1. Read the Unit 7 catalog (Status column) in `AUDIT-PLAN.md`
  2. Read `refbox/tests/features/portal-health.feature` for test-tag distribution
  3. Read the ADR 011 "Verified by Unit 7 audit" section on `docs/workspace/backlog-adrs`
  4. Confirm: "Unit 7 approved" or request changes

  Iterate on changes until the operator confirms approval.

- [ ] **Step 9.2: Flip Unit 7 status in AUDIT-PLAN.md.**

  In `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md`:
  - Update the unit-catalog table row (line 64): `in progress (started YYYY-MM-DD)` → `complete-pending-integration (YYYY-MM-DD)`
  - Update the `### Unit 7 — PR #761 portal health indicator` section's `**Status:**` line the same way

- [ ] **Step 9.3: Add a summary entry to "Completed audits".**

  Per Unit 1 refinement #3 (in-place flip + summary pointer, not destructive section-move), add an entry to the Completed audits section near the bottom of `AUDIT-PLAN.md`, between the existing Unit 6 and Unit 5 entries (newest first):

  ```markdown
  #### Unit 7 — PR #761 portal health indicator — complete-pending-integration YYYY-MM-DD

  - **Branch:** `audit/refbox/portal-health` (local only; <N> commits ahead of `feat/refbox/portal-health-indicator`; not pushed)
  - **Per-unit plan:** `docs/superpowers/plans/2026-05-15-audit-unit-7-portal-health.md`
  - **Audit-design spec:** `docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md`
  - **ADR (finalized in place):** `docs/decisions/011-portal-health-indicator.md` (status flipped proposed → accepted; 4 amendments preserved; new "Verified by Unit 7 audit" section appended); finalization commit on `docs/workspace/backlog-adrs`
  - **Scenarios:** `refbox/tests/features/portal-health.feature` — <N> scenarios total; <X> @tested_pass, <Y> @tested_fail, <Z> @tested_inconclusive
  - **Catalog outcome:** <N> entries (target was 50–80); <X> @user_verified; <Y> @deleted; <Z> @findings-backlog; <W> @redesign-followup
  - **Tests added during audit:** none — portal communication and view-builder behaviour are walkthrough-verified per the lean-process refbox-UI convention; existing unit tests under `refbox/src/portal_manager/` were retained
  - **Audit commits on branch:** <list of SHAs — Gherkin seed commit, walkthrough session-notes commit, plus any prune commits>
  - **What was not verified:** <bullet list from ADR 011's Verified-by section>
  - **Findings filed:** <N> new Findings backlog items
  - **Process refinements:** <N> new refinements logged
  - **Cross-branch dependencies:** <list, or "none">
  - **Full details section:** retained in "Unit-by-unit details" above with status flipped to complete-pending-integration.
  ```

  Fill in real values at execution time.

- [ ] **Step 9.4: Add Findings backlog entries (if any).**

  Per Step 5.9, any `@findings-backlog` decisions or any out-of-scope discoveries from Tasks 2–7 are listed under `### Findings backlog → #### From Unit 7 (YYYY-MM-DD)`. Examples to watch for:
  - Pre-existing `confy::store(..).unwrap()` pattern in portal_manager queue persistence (parallels Unit 3 finding #2 but on a new module)
  - Env-var hooks shipping in production binaries (if Step 5.6 carved them out)
  - Any `uwh-common` follow-up implied by Group A's error-handling decisions
  - The retired token-expired action page asset / message variant if any orphans survive

- [ ] **Step 9.5: Add Process refinements log entries (if any).**

  Under `### Process refinements log → #### From Unit 7 (YYYY-MM-DD)`. Examples to watch:
  - The five-subagent parallel-dispatch pattern for catalog construction (first use; record what worked and what didn't)
  - The page-batched + ambiguity-carve-outs pattern at 50+ entries (re-validation of Unit 3 refinement #3 at the upper end of its range)
  - Any new memory-of-rule-applicability lesson (Unit 6 refinement #4 territory)
  - Local-portal walkthrough setup time and gotchas
  - Cross-branch dependency detection efficacy

- [ ] **Step 9.6: Run `just check` once more on the audit branch tip.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-7-portal-health && just check 2>&1 | tail -30
  ```

  Expected: green (matches Step 7.13).

- [ ] **Step 9.7: (Principal-only follow-up) Update Claude's memory files.**

  This step is performed by the principal Claude session, not an executing subagent — memory at `~/.claude/projects/.../memory/` is principal-only territory. After Step 9.6 reports green, the principal:

  - Updates `project_v040_handover.md`: mark Unit 7 as complete-pending-integration; note that PR #761 remains untouched; add a one-line "audit branch supersedes PR #761 at Final Integration" reminder.
  - Updates audit progress count: "6 of 9 units complete" → "7 of 9 units complete".
  - Notes Unit 8 (Grid-select page + UNVERIFIED marker) as next.
  - Updates `MEMORY.md` index if the handover entry's hook sentence needs to change.
  - If any new feedback memory was earned during Unit 7 (e.g., a subagent-dispatch lesson, a local-portal-setup gotcha), write it as its own memory file and add the index pointer.

- [ ] **Step 9.8: Confirm the unit is closed.**

  Tell the operator:

  > "Unit 7 complete-pending-integration. Branch `audit/refbox/portal-health` holds locally with <N> audit commits. ADR 011 finalized on `docs/workspace/backlog-adrs` (status flipped, amendments preserved, Verified-by-Unit-7 section appended). PR #761 untouched on `feat/refbox/portal-health-indicator`. `AUDIT-PLAN.md` status flipped. Memory updated. Ready to move on to Unit 8 (Grid-select page + UNVERIFIED marker)?"

---

## Risks and known divergences

Starting points for catalog questions, not pre-decisions. (See the spec's Risk Register for the canonical list; the items below are the per-task pickup of those risks.)

1. **PR #761 must stay untouched.** Per spec R1: audit on a fresh branch (Task 1.4), never push during the audit, branch is local-only until Final Integration. Per memory `feedback_prs_deferred_until_audit_done`, do NOT propose, suggest, or execute any PR/merge during this unit.
2. **Cross-unit shared files** (`app/mod.rs`, `shared_elements.rs`) get cross-unit notes per Unit 6 refinement #3 in Tasks 3.3 and 3.4. Watch for collisions with Unit 1 (confirm-score), Unit 3 (settings), Unit 4 (manual alarm), Unit 5 (referee names), Unit 8 (CJK/Thai).
3. **Cross-branch dependencies** are detected in Task 4.1 via `git log --all -S '<symbol>'`. If found, hand-apply on the audit branch per Unit 5 ADR 022 pattern and record in the ADR Verified-by section's audit-reference paragraph.
4. **Catalog size > 80** triggers re-batching per Task 3.8. Don't push through with an oversized catalog on the original batching.
5. **Local portal setup might block Task 7.** Pre-flight handled in Task 7.4; fallback to dev.uwhportal.com per ADR 011 amendment 2026-04-22 if local proves impractical. Decision happens at pre-flight, not mid-walkthrough.
6. **Unrecorded behaviour shifts** become `not-in-spec` carve-outs in Task 5.7. Watch in particular: commit `5de76ed` "detail and attention pages UX pass" — may have shifted behaviour without an amendment record.
7. **The token-expired action page lifecycle.** Added at `3cfb137`, refactored away at `bd4f2ca`. Both ends get catalog entries; the surviving design is verified by walkthrough Scenario 3 (no separate token-expired page; red row routes through portal-login).
8. **30-minute escalation timer.** Walkthrough Scenario 2 may verify this only at a reduced threshold; full-cadence verification may be deferred. Record any such deferral in the ADR Verified-by section's "What was not verified" subsection.
9. **Env-var hooks in production binaries** (Step 5.6 carve-out). If the operator carves them to Findings, the audit doesn't surgically gate them — that's deferred to a separate branch.

---

## Deviations

> Filled in during execution. Per heavy-process discipline, deviations are recorded here as a running section AND each significant deviation gets its own commit when it lands on a branch. Lean-process per-task deviation commits are NOT used for Unit 7 (heavy process).

(empty at plan-write time; populated during execution)

---

## Files Created or Modified by This Plan

- `.worktrees/audit-unit-7-portal-health/` (new worktree, lifecycle: removed at Final Integration)
- `/home/estraily/projects/uwh-refbox-rs/AUDIT-PLAN.md` (gitignored; multiple edits — history trace, behaviour catalog with five subgroups + cross-cutting findings, Findings backlog from-Unit-7 subsection, Process refinements from-Unit-7 subsection, status flips, Completed audits summary)
- `.audit/unit-7-commits-raw.txt` (local working artifact)
- `.audit/unit-7-commits.txt` (local working artifact)
- `.audit/unit-7-commit-messages.txt` (local working artifact)
- `.audit/unit-7-files-touched.txt` (local working artifact)
- `.audit/unit-7-grouping.md` (local working artifact)
- `.audit/unit-7-group-A-portal-manager.md` (local working artifact)
- `.audit/unit-7-group-B-app-mod.md` (local working artifact)
- `.audit/unit-7-group-C-ui-surface.md` (local working artifact)
- `.audit/unit-7-group-D-tests-assets.md` (local working artifact)
- `.audit/unit-7-group-E-env-vars.md` (local working artifact)
- `refbox/tests/features/portal-health.feature` (created on audit branch)
- Surgical pruning edits to whichever audit-branch files contain `@deleted` behaviours (varies per Task 5 outcome)
- `docs/decisions/011-portal-health-indicator.md` (amended on `docs/workspace/backlog-adrs`, NOT on the audit branch — status flip + new Verified-by-Unit-7 section)
- Memory `project_v040_handover.md` and `MEMORY.md` (updated at close); any new memory files written by the principal in Step 9.7

---

## Estimated commits on the audit branch

- 0–N prune commits (one per `@deleted` behaviour, dependency-ordered) — Step 6.2.g
- 1 scenario-seeding commit (Step 7.3)
- 1 walkthrough session-notes commit (Step 7.12)
- **Total on `audit/refbox/portal-health`:** 2 + N commits at close (N is the number of `@deleted` entries; expected range 0–5 given the spec's prediction that most entries are realises-amendment or matches-spec).

Plus, on `docs/workspace/backlog-adrs`:
- 1 ADR 011 finalization commit (Step 8.5)
- Possibly memory writes (Step 9.7) — these aren't git commits in this repo; they're file writes to `~/.claude/projects/.../memory/`.
