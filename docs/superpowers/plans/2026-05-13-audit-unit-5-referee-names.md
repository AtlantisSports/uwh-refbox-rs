# Audit Unit 5 — Referee Names Display: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` (subagent-driven execution is the default; Unit 1's rehearsal of inline execution is complete). Each task is dispatched to a fresh subagent by the principal; critical keep/delete decisions come back to the user via the principal. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit the six commits that ship referee-name display in game info (`996874a`, `d72d643`, `353b476`, `8d4a667`, `1bd4676`, `931d01d`), cataloging every distinct behaviour the diff added across `uwh-common` and `refbox` in **four feature groups** (individual-ref display path · team-ref fallback path · PII handling · cross-cutting & slop), running operator-driven user review per group, surgically pruning anything the user rejects, exercising the surviving feature across every `uwh-common` consumer crate, and producing a fresh retroactive ADR 022 for what shipped.

**Architecture:** Diff-led catalog grouped by feature (per the just-completed brainstorm). Commits don't map 1:1 to features — PII handling spans two commits, individual-ref display spans four — so feature-grouping minimizes mental re-stitching during Step 4 review and aligns the retroactive ADR's sections to the catalog. **Heavy process** because the diff touches `uwh-common` (shared types, full-project blast radius). Audit performed in an isolated git worktree at `.worktrees/audit-unit-5-referee-names/` on branch `audit/refbox/referee-names` cut from `origin/master`. Branch stays local through Step 9. AUDIT-PLAN.md is gitignored — edits there are file-on-disk only, never commits. Per-task deviations get their own `docs(workspace): record Task N deviations` commits (heavy-process discipline).

**Tech Stack:** Rust 2024 / MSRV 1.85; `serde` / `serde_json` for the portal wire format (new `RefereeAssignment` types and `/referees` endpoint response shape); iced 0.13 (refbox UI rendering of the referee list); `cargo test`, `cargo clippy`, `just check`; manual refbox launch (`WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox` with `dangerouslyDisableSandbox:true`); Gherkin scenarios at `refbox/tests/features/referee_names.feature` (new file, seeded in Step 6 from `@user_verified` scenarios).

**Testing approach:**

- **uwh-common unit tests** for the new types: serde roundtrips for `RefereeAssignment` (with and without `display_name` — `#[serde(skip)]` means it must not appear in JSON either direction), `GameReferees` accepting `hybrid` as both object and array (per `353b476`'s commit-message claim), `Schedule.referees_by_game_number` Optional roundtrip, URL-construction test for `get_event_referee_name_map_from_referees` (non-network), and `last_2_min_stop_time` default-value test (gated on Step 4 keeping the field).
- **Cross-crate verification** per Unit 2 refinement #4: a single composite subagent runs `cargo build` + `cargo test` for `schedule-processor`, `overlay`, `led-panel-sim`, `matrix-drawing` against the audited `uwh-common` shape. `wireless-remote` is on a separate toolchain and unaffected (the new types are not used there) — explicitly noted in unit notes and ADR.
- **Operator-driven UI walkthrough** for Gherkin scenarios in Step 6, one session covering the four-to-six `@user_verified` scenarios in `refbox/tests/features/referee_names.feature`.
- **End-to-end consumer exercise** (real portal data emitting `referee_assignments` / `refereesByGameNumber`) is **skippable** per Unit 2 refinement #1 if real-shape data isn't reachable. Gap documented in ADR's "What was not verified" section.

---

## Acceptance criteria (Unit 5 "complete-pending-integration")

Unit 5 is complete-pending-integration when **all** of these hold on the audit branch `audit/refbox/referee-names`:

1. A behaviour catalog exists in `AUDIT-PLAN.md` under Unit 5, grouped into four feature sections (Individual-ref display path · Team-ref fallback path · PII handling · Cross-cutting & slop), with every entry tagged `@user_verified` or `@deleted`. No `@proposed` remaining.
2. Every `@user_verified` operator-observable behaviour is captured as a Gherkin scenario in `refbox/tests/features/referee_names.feature`, tagged `@user_verified` and a test-state tag (`@tested_pass`, `@tested_fail`, or `@tested_inconclusive`).
3. Every `@user_verified` backend behaviour has its test status captured in the retroactive ADR's prose (no `.feature` row for backend).
4. The operator has driven the refbox UI through one Session covering scenarios S5.1–S5.6 (presence of S5.4 conditional on Group 2 surviving Step 4). Each scenario carries a manual-walkthrough timestamp in the .feature file.
5. `just check` passes on the audit branch (`fmt-check`, `clippy -D warnings`, all tests pass, `cargo audit` clean — the two pre-existing dependency vulnerabilities noted in Unit 3's Findings backlog #4 are expected and not regressions).
6. Every consumer crate of `uwh-common` (`schedule-processor`, `overlay`, `led-panel-sim`, `matrix-drawing`) has been built and its tests run cleanly **as a separate verification step**, not just via `just check`. `wireless-remote` is documented as out-of-scope (separate toolchain; types not consumed there).
7. A retroactive ADR exists at `docs/decisions/022-referee-names-display.md` (numbered against the expected post-merge state per Unit 1 refinement #8; Unit 4 holds 021). Its Decision section embeds `@user_verified @tested_pass` scenarios verbatim with one sentence of plain-English framing per scenario, plus plain-English bullets for backend behaviours that have no scenario.
8. The branch holds locally (no push, no PR) per playbook Step 8.
9. AUDIT-PLAN.md status flipped from "not started" to "complete-pending-integration (YYYY-MM-DD)" in both the catalog table and the unit section heading; summary pointer added to "Completed audits" section per playbook-amended Step 9.4.
10. Findings discovered out-of-scope are recorded in AUDIT-PLAN.md's Findings backlog with a suggested follow-up branch name. They are **not fixed** on this branch.
11. Process refinements surfaced during execution are logged in AUDIT-PLAN.md's "Process refinements log → From Unit 5".

---

## Prerequisites

- The user has approved this per-unit plan before any Task 1 step runs.
- Working tree on the current branch (whichever the principal sits on) is clean — uncommitted state stays untouched by the worktree creation.
- `git fetch origin master` is current.
- Read `AUDIT-PLAN.md` Unit 5 section (line 2522) and the Process refinements log entries from Units 1, 2, 3, and 4. Particularly relevant to Unit 5:
  - Unit 1 refinement #2 (.feature location: `refbox/tests/features/`)
  - Unit 1 refinement #6 (Bash cwd doesn't persist between calls)
  - Unit 1 refinement #7 (refbox launch needs `WAYLAND_DISPLAY= RUST_LOG=info`)
  - Unit 1 refinement #8 (ADR numbering gap on audit branches is expected)
  - Unit 2 refinement #1 (consumer-end-to-end exercise is skippable when real data unavailable)
  - Unit 2 refinement #2 (catalog bundled-fix-in-feature-commit as its own B-entry; flag for Process refinements)
  - Unit 2 refinement #4 (single composite subagent for cross-crate verification)
  - Unit 2 refinement #5 (first-try serde wire-format match is a confidence signal)
  - Unit 3 refinement #3 (page-batched / group-batched review for catalogs of 25+ entries; one-by-one for ≤15)
  - Unit 3 refinement #5 (`unwrap()` on I/O is real risk; classify carefully)
  - Unit 4 refinement #3 (pattern-consistent pre-existing debt → Findings-backlog whole-module candidate, not in-audit fix)
- Read `.claude/rules/scope.md`, `communication.md`, `workspace.md`, `rust.md`, `plan-execution.md`, `pr-review.md`. `embedded.md` is informational only — Unit 5 does not touch `wireless-remote`.
- Pre-commit hook at `<main-repo>/.git/hooks/pre-commit` must allow `audit/` branch type (fixed by Unit 1's commit `2a8dcbc`). Verify presence in Task 1 Step 3.

---

## Task 1: Setup (AUDIT-PLAN.md Step 1)

**Files:**
- Create: `.worktrees/audit-unit-5-referee-names/` (new worktree)
- Edit: `AUDIT-PLAN.md` (gitignored, no commit)

- [ ] **Step 1.1: Confirm working tree is clean.**

  Run: `git -C /home/estraily/projects/uwh-refbox-rs status --short`
  Expected: empty output (or only `?? .claude/scheduled_tasks.lock` which is untracked-and-fine).

- [ ] **Step 1.2: Ask the user for explicit approval to cut the audit branch in a worktree.**

  Surface the branch name and worktree path:
  - Branch: `audit/refbox/referee-names`
  - Worktree: `.worktrees/audit-unit-5-referee-names/`
  - Cut from: `origin/master` (Unit 5's commits are all merged on master per the v0.4.0 ship per memory).

- [ ] **Step 1.3: Verify the pre-commit hook allows the `audit` branch type.**

  Run from main repo root: `grep -c '\baudit\b' .git/hooks/pre-commit`
  Expected: non-zero count. If zero, copy the audit-aware version from Unit 1's branch:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs && git show audit/refbox/confirm-score-timing:scripts/pre-commit > .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
  ```

- [ ] **Step 1.4: Create the worktree on a fresh `master`.**

  Run from main repo root:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs && git fetch origin master && git worktree add -b audit/refbox/referee-names .worktrees/audit-unit-5-referee-names origin/master
  ```
  Expected output: `Preparing worktree (new branch 'audit/refbox/referee-names')`.

- [ ] **Step 1.5: Verify the worktree HEAD contains all six audit commits.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names && git log --oneline 996874a d72d643 353b476 8d4a667 1bd4676 931d01d -6`
  Expected: all six commits appear (they merged to master before this audit).

- [ ] **Step 1.6: Sanity-check the worktree builds.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names && cargo check -p uwh-common -p refbox 2>&1 | tail -5`
  Expected: clean compile. If this fails, master is broken — stop and check with user.

- [ ] **Step 1.7: Update Unit 5 status in `AUDIT-PLAN.md`.**

  Edit (file edit, no commit — AUDIT-PLAN.md is gitignored):
  - Unit 5 row in the "Audit unit catalog" table: change status from `not started` to `in progress (started YYYY-MM-DD)`.
  - Unit 5 section heading near the bottom: same status update.
  - Add to the Unit 5 section header block: `**Per-unit plan:** docs/superpowers/plans/2026-05-13-audit-unit-5-referee-names.md` and `**Worktree:** .worktrees/audit-unit-5-referee-names/`.

---

## Task 2: Generate the diff (AUDIT-PLAN.md Step 2)

**Files:** AUDIT-PLAN.md edit only.

- [ ] **Step 2.1: Identify the merge point.**

  Run (from worktree): `git log --oneline 996874a~1 -1`
  Record this SHA as `<base>`. This is the commit immediately preceding the unit's first commit.

- [ ] **Step 2.2: List the unit's six commits in chronological order.**

  Run (from worktree): `git log --oneline 996874a^..931d01d --reverse`
  Expected:
  ```
  996874a fix(refbox): correct referee role strings in game info display
  d72d643 feat(refbox): display real referee names in game info
  353b476 feat(uwh-common): add /referees endpoint for referee name resolution
  8d4a667 feat(refbox): display team or individual referee assignments in game info
  1bd4676 fix(refbox): show "Unknown" when a referee has no display name
  931d01d fix(refbox): display referee names on game-info page
  ```

- [ ] **Step 2.3: File list across all six commits.**

  Run (from worktree): `git diff --name-only 996874a~1..931d01d`
  Expected (anticipated, verify):
  - `uwh-common/src/uwhportal/mod.rs`
  - `uwh-common/src/uwhportal/schedule.rs`
  - `refbox/src/app/mod.rs`
  - `refbox/src/app/view_builders/game_info.rs`
  - `refbox/src/app/view_builders/main_view.rs`
  - `refbox/src/app/view_builders/shared_elements.rs`
  - `refbox/translations/en-US/refbox.ftl`
  - `refbox/translations/es/refbox.ftl`
  - `refbox/translations/fr/refbox.ftl`
  - `schedule-processor/src/csv_parser.rs` (2-line touch per `8d4a667`)

- [ ] **Step 2.4: Save the full diff for catalog reference.**

  Run (from worktree): `git diff 996874a~1..931d01d > .audit/unit-5-diff.patch` (create `.audit/` directory first if needed; not committed).

- [ ] **Step 2.5: Record file list and SHA range in `AUDIT-PLAN.md` under Unit 5's "Files touched" subsection.**

  Note the cross-crate blast radius explicitly: every consumer of `uwh-common` is potentially affected. Full list: `refbox`, `schedule-processor`, `overlay`, `led-panel-sim`, `matrix-drawing`. `wireless-remote` is unaffected (separate toolchain; new types not consumed). AUDIT-PLAN.md edit only; no commit.

- [ ] **Step 2.6: Resolve the commit-ordering anomaly before cataloging.**

  Commit `d72d643` (2026-04-12) calls `client.get_event_referee_name_map_from_referees(&event_id)` in `request_schedule`, but commit `353b476` (2026-04-18) is the commit that adds that method to `UwhPortalClient`. Either (a) `d72d643` originally added a stub or earlier-named function that `353b476` replaced, (b) the work shipped on a non-master branch and was rebased, or (c) the commit messages misrepresent the history. Trace the history before drafting catalog entries.

  Run:
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names
  git log -p -S "get_event_referee_name_map" --all -- uwh-common/src/uwhportal/ | head -150
  git show d72d643 -- uwh-common/src/uwhportal/mod.rs | head -100
  git show 996874a^..d72d643 -- uwh-common/src/uwhportal/mod.rs
  ```

  Record the finding in `.audit/unit-5-commit-ordering-note.md` (not committed) and reference it from the eventual `B5.15` catalog entry. The finding becomes one of two things:
  - Stub-and-replace: `d72d643` added a stub or different-shape function that `353b476` superseded. → Catalog as a slop entry: the audit branch's first compileable state has `353b476` and the in-between commit `d72d643` isn't standalone-buildable.
  - Off-master development: the commits were made on a feature branch and rebased onto master in their current order. → Catalog as a Process refinements observation only; the cumulative end-state is what shipped.

---

## Task 3: Build the behaviour catalog — feature-grouped (AUDIT-PLAN.md Step 3)

**Files:** AUDIT-PLAN.md edit only — no code, no commit.

Per the brainstorm, the catalog is grouped into **four feature sections**. Anticipated entries are listed below. The executing subagent MUST verify each by reading the diff at `.audit/unit-5-diff.patch` before drafting, and add any additional entries surfaced by the slop-catching sweep (Step 3.6).

The slop-catching sweep is performed AS the catalog is built, with each slop hit becoming its own B-entry placed in the appropriate group (or Group 4 if cross-cutting).

### Group 1 — Individual-ref display path

- [ ] **Step 3.1.1: Draft B5.1 — `RefereeAssignment` type + `display_name: Option<String>` field.**

  Plain English: "A new data type for one referee's assignment to a role in one game (their portal-side identifier, their role string, and an optional user ID and team ID). The `display_name` field holds the resolved human-readable name once the `/referees` endpoint has been called; it's marked `#[serde(skip)]` so it never enters or leaves the portal JSON contract." Where: `uwh-common/src/uwhportal/schedule.rs` — type added in `8d4a667`, `display_name` added in `d72d643`. Why intentional: enables per-referee name display. Slop-check: `comments: Option<String>` field — is it ever read by refbox or schedule-processor? Verify or flag as unused-field slop.

- [ ] **Step 3.1.2: Draft B5.2 — `Game.referee_assignments: Option<Vec<RefereeAssignment>>` field.**

  Plain English: "Each game in the schedule now carries an optional list of referee assignments. None means no referees are assigned for this game (or the portal didn't send any)." Where: `uwh-common/src/uwhportal/schedule.rs` `Game` struct, added in `8d4a667`. Slop-check: is `None` distinguished from `Some(vec![])` anywhere in the consumer code, or are they treated identically? If identical, the inner Option is over-flexible.

- [ ] **Step 3.1.3: Draft B5.3 — `get_event_referee_name_map_from_referees` method on `UwhPortalClient`.**

  Plain English: "A new HTTP GET against `/api/events/{partial_id}/referees` that builds a user-ID → display-name map from the portal's response. Tolerates the `hybrid` referees category as either an object or an array (portal shape variant)." Where: `uwh-common/src/uwhportal/mod.rs`, added in `353b476`. Slop-check: error path returns `Default::default()` (an empty map) via `unwrap_or_default()` at the call site (`refbox/src/app/mod.rs` `request_schedule`) — silent degradation rather than surfacing the error. Is this the right error handling, or should a failed `/referees` call be visible to the operator?

- [ ] **Step 3.1.4: Draft B5.4 — Name-map fetch and `display_name` population in `request_schedule`.**

  Plain English: "When refbox fetches a schedule, it also calls `/referees` for the same event and populates each referee assignment's `display_name` from the resulting name map. If the lookup fails, `display_name` is left as `None` and the display falls back through the rest of the chain." Where: `refbox/src/app/mod.rs` `request_schedule`, added in `d72d643`. Slop-check: error handling on `names_req.await.unwrap_or_default()` — silent failure mode (see B5.3 slop-check).

- [ ] **Step 3.1.5: Draft B5.5 — Role-string corrections (Chief Ref → Chief, Water Ref 1 → Water1, etc.).**

  Plain English: "The strings the role-matching code tests against are corrected to match the portal's actual role values (`Chief`, `Water1`, `Water2`, `Water3`, `TimeOrScoreKeeper`). Previously every role match fell through to the wildcard, showing 'Unknown' for every referee." Where: `refbox/src/app/view_builders/shared_elements.rs`, corrected in `996874a`. **Verify the corrected strings are used consistently in BOTH `main_view` and `game_info` (`config_string`)** — `8d4a667` later introduced a NEW copy of the role-matching block in `config_string` using the OLD strings, and `931d01d` brought them back into sync.

- [ ] **Step 3.1.6: Draft B5.6 — Individual-ref rendering in `config_string` (game-info page) and `main_view`.**

  Plain English: "When a game has individual referee assignments with `user_id` set, the game-info page and main view display them in a per-role grid (Chief / Timer / Water 1 / Water 2 / Water 3). The displayed text is `display_name` if populated, otherwise the localized 'Unknown' string (B5.14)." Where: `refbox/src/app/view_builders/shared_elements.rs` `config_string` and the `main_view.rs` rendering path, added/modified across `d72d643`, `8d4a667`, `1bd4676`, `931d01d`. Linked scenarios: **S5.1**, **S5.5**. Slop-check: this is the entry where the duplicate-display-logic finding lands — main_view and config_string have parallel copies of the role-matching block. **931d01d** synced them; they remain two copies. The dedupe itself is Findings-Backlog material per Unit 4 refinement #3.

- [ ] **Step 3.1.7: Draft B5.7 — game_info.rs sync to match main_view (commit `931d01d`).**

  Plain English: "The game-info page's referee-display logic, which had its own copy of the role-matching code, is brought back in sync with the main view's version (correct role strings, display-name fallback chain). Without this, the game-info page showed 'Unknown' for every referee slot regardless of portal data." Where: `refbox/src/app/view_builders/game_info.rs` and adjacent in `refbox/src/app/mod.rs`, in `931d01d`. Linked scenario: **S5.5**. Slop-check: the existence of two parallel copies in the first place is debt; the sync fixes the immediate symptom. Findings-Backlog candidate (per Unit 4 refinement #3) for whole-file dedupe.

### Group 2 — Team-ref fallback path

All Group 2 entries are catalogued `@proposed`; Step 4 review decides keep/delete for the entire group together (see Task 4 Step 4.2).

- [ ] **Step 3.2.1: Draft B5.8 — `RefereesByGameNumber` type alias + `Schedule.referees_by_game_number: Option<RefereesByGameNumber>` field.**

  Plain English: "The schedule gains an optional schedule-wide map of game-number → team-referee assignments. Distinct from each game's own `referee_assignments` (Group 1) — this is a separate, schedule-level data source for the team-ref fallback path." Where: `uwh-common/src/uwhportal/schedule.rs`, added in `8d4a667`. Slop-check: two parallel data sources (`Game.referee_assignments` AND `Schedule.referees_by_game_number`) for what could be one. Verify: does the portal actually emit both shapes in real data, or is the schedule-level one anticipatory?

- [ ] **Step 3.2.2: Draft B5.9 — `GameReferees`, `TeamRefAssignment`, `TeamRefInfo` types.**

  Plain English: "Three new types that describe team-level referee assignments: `GameReferees` holds three slots (`time_or_score_keeper`, `time_or_score_helper`, `referees`); `TeamRefAssignment` wraps a single team; `TeamRefInfo` is the team's ID and name." Where: `uwh-common/src/uwhportal/schedule.rs`, added in `8d4a667`. Linked scenario: **S5.4** (only if Group 2 kept). Slop-check: `time_or_score_helper` slot is never read by the rendering code — see B5.12.

- [ ] **Step 3.2.3: Draft B5.10 — Team-ref fallback branch in `config_string`.**

  Plain English: "When a game has no individual referees with `user_id` set, the game-info page falls through to a different display: 'Refs: <team name>' and 'Time/Score Keeper: <team name>', sourced from `Schedule.referees_by_game_number`. If this fallback path activates and a team name is present, it early-returns before the per-role grid would have been rendered." Where: `refbox/src/app/view_builders/shared_elements.rs` `config_string`, added in `8d4a667`. Linked scenario: **S5.4**. Slop-check: only one display mode is shown at a time (early return); is this what the operator expects, or could both be shown side-by-side? Step 4 territory.

- [ ] **Step 3.2.4: Draft B5.11 — `team-ref-list` translation key.**

  Plain English: "A new fluent translation key with placeholders `ref_team` and `ts_keeper_team`, used to render the team-ref fallback line." Where: `refbox/translations/en-US/refbox.ftl`, `es/refbox.ftl`, `fr/refbox.ftl`, added in `8d4a667`. Slop-check: **translation gap** — added in en/es/fr only (3 of 15 locales). If Group 2 is kept, the gap needs in-audit fixing or a Findings-Backlog entry (see B5.18).

- [ ] **Step 3.2.5: Draft B5.12 — `GameReferees.time_or_score_helper` defined-but-unread field.**

  Plain English: "A third slot on `GameReferees` for the time/score helper referee. The type defines it but the rendering code in `config_string` never reads it — only `referees` and `time_or_score_keeper` reach the display." Where: `uwh-common/src/uwhportal/schedule.rs`, defined in `8d4a667`. Slop-check: classic AI slop pattern — defining a field for future use that no current consumer reads. Recommendation: **delete** if Group 2 is kept, OR delete with the whole Group 2 if rejected. Step 4 decides.

### Group 3 — PII handling

- [ ] **Step 3.3.1: Draft B5.13 — PII boundary: `rosterName → username` preference, intentional skip of `user.name`.**

  Plain English: "The `/referees` endpoint parsing reads `rosterName` (if non-empty) as the primary display name, falling back to `user.username`. The `user.name` field — which the portal exposes as the user's real account-profile name — is **deliberately not read** and never enters the refbox. Treated as PII the refbox UI should not surface; the chosen handle is the appropriate fallback." Where: `uwh-common/src/uwhportal/mod.rs` `get_event_referee_name_map_from_referees`, in `353b476`. Linked scenario: **S5.6**. Slop-check: verify the inline comment is present in the code (per the commit message's claim); if absent, the design intent is git-archaeology-only — surfaces as a Process refinements observation.

- [ ] **Step 3.3.2: Draft B5.14 — Display fallback chain ends at localized "Unknown".**

  Plain English: "When the resolved `display_name` is None for a referee slot, the displayed text is the localized 'Unknown' string (via `fl!('unknown')`) — NOT the portal-assigned `identifier` (a system code, not a human name). This change replaces an earlier behaviour where the system code was shown to the operator." Where: `refbox/src/app/view_builders/shared_elements.rs`, in `1bd4676`. Linked scenarios: **S5.2**, **S5.3**. Slop-check: NONE — this is the *correctly localized* fallback pattern (cross-cuts inversely to Unit 2 Finding #1's *bypass* pattern in the team-name path). **Note for Step 9:** edit Unit 2 Finding #1 to point at this entry as the reference implementation when team-name "Unknown" gets a fix branch.

### Group 4 — Cross-cutting & slop

- [ ] **Step 3.4.1: Draft B5.15 — Commit-ordering anomaly.**

  Plain English: "Commit `d72d643` (2026-04-12) calls `client.get_event_referee_name_map_from_referees(&event_id)` in `request_schedule`. Commit `353b476` (2026-04-18) is the commit that *adds* that method to `UwhPortalClient`. Either the intervening 6 days had a stub-and-replace pattern, or the work was done on a feature branch and rebased." Source: Task 2 Step 2.6 history-trace finding. Slop-check: this is the "method defined later than its first caller" pattern — candidate for a new slop-catching checklist item. Process refinements log entry.

- [ ] **Step 3.4.2: Draft B5.16 — `last_2_min_stop_time: bool` field added to `TimingRule`.**

  Plain English: "A new boolean field on `TimingRule` indicating whether the clock stops in the last two minutes of play. Read by `config_string` to display the stop-clock setting on the game-info page (replacing a previous 'Unknown' literal). Defined and read by 8d4a667, but unrelated to the headline 'referee names' feature." Where: `uwh-common/src/uwhportal/schedule.rs` and `refbox/src/app/view_builders/shared_elements.rs`, in `8d4a667`. **Scope flag:** bundled-fix-in-feature-commit pattern, per Unit 2 refinement #2. Catalog as its own entry; flag in Process refinements log regardless of the keep/delete outcome on the field itself. Recommendation: **keep** if the operator confirms the displayed stop-clock setting is useful; flag the bundling separately.

- [ ] **Step 3.4.3: Draft B5.17 — `TimingRule::into()` destructure of `last_2_min_stop_time: _`.**

  Plain English: "The `Into<GameConfig>` impl on `TimingRule` destructures the new `last_2_min_stop_time` field but discards it with `_`. The field is read only by `config_string`'s display logic, never by the game state machine." Where: `uwh-common/src/uwhportal/schedule.rs` `TimingRule` Into impl, in `8d4a667`. Slop-check: the discard pattern is correct given the field's display-only purpose, but it does mean the value flows into a display path that never reaches the game clock's actual stop-clock logic. Worth surfacing for the operator — is this asymmetry intentional?

- [ ] **Step 3.4.4: Draft B5.18 — Translation coverage gap for `team-ref-list` key (en/es/fr of 15).**

  Plain English: "The `team-ref-list` translation key is added in en-US, es, and fr only; the project ships 15 locales. Operators on the other 12 locales see a fallback (English text or the literal key, depending on fluent's fallback chain) for the team-ref fallback line." Where: `refbox/translations/*/refbox.ftl`, in `8d4a667`. **Conditional:** only material if Group 2 (team-ref fallback) survives Step 4. Slop-check: same pattern Unit 4 caught (B4.30 / B4.31) — translation rollout incomplete. Recommendation: **in-audit fix** (add the key in the remaining 12 locales) if Group 2 is kept; **delete with Group 2** if the path is removed; otherwise Findings-Backlog candidate.

### Step 3.5: Slop-catching checklist sweep

- [ ] **Step 3.5.1: Walk every item in `AUDIT-PLAN.md`'s slop-catching checklist against the unit's diff.** For each match not already captured above, add a new B-entry in Group 4. Specifically check:

  - Fallback paths for "impossible" cases — `unwrap_or_default()` on `names_req.await` (already B5.3 slop-check); any `match` arms for impossible portal shapes.
  - Defensive validation at internal boundaries — n/a anticipated.
  - Error-mapping that just re-wraps — n/a anticipated.
  - Helper functions never called from real code — verify no orphan helpers in `mod.rs`'s referees code.
  - Configuration knobs not exposed in UI — n/a anticipated (no new settings).
  - Logging at unusual levels — verify `info!("Got schedule")` log line moved in `request_schedule` is appropriate.
  - Comments explaining what the code does — survey new code for what-not-why comments.
  - "Future-proofing" abstractions — the four-types-for-team-refs is on the edge of this; already B5.9.
  - Re-implementations of existing utilities — n/a anticipated.
  - Tests that don't actually assert anything — n/a (Unit 5 ships no new tests on master).
  - String literals that aren't in translations — verify all new operator-facing strings go through `fl!()`. The "Unknown" pattern in 1bd4676 already does (B5.14).
  - Magic numbers — n/a anticipated.
  - "Just in case" retries / waits / sleeps — n/a anticipated.

### Step 3.6: Draft Gherkin scenarios for UI-facing entries

- [ ] **Step 3.6.1: Draft S5.1 — Real names appear when /referees returns a name map.** Links to B5.6.

  ```gherkin
  Feature: Referee names in game info

    Scenario: Real referee names appear when the portal returns a name map
      Given the refbox has fetched a schedule for an event with individual referee assignments
      And the portal's /referees endpoint returned display names for every assigned user_id
      When the operator navigates to the game-info page for a game with referees
      Then the page shows the per-role grid (Chief, Timer, Water 1, Water 2, Water 3)
      And each role displays the referee's resolved display_name
      And no role shows the localized "Unknown" placeholder
  ```

- [ ] **Step 3.6.2: Draft S5.2 — Localized "Unknown" appears when name map has no entry.** Links to B5.14.

  ```gherkin
    Scenario: Localized Unknown appears when a referee has no display name
      Given the refbox has fetched a schedule with individual referee assignments
      And the portal's /referees endpoint returned a name map missing entries for one or more user_ids
      When the operator navigates to the game-info page for that game
      Then the unresolved roles display the localized "Unknown" string
      And the localized "Unknown" matches the system locale (not a hardcoded English string)
  ```

- [ ] **Step 3.6.3: Draft S5.3 — Silent degradation when /referees fails.** Links to B5.4 error path.

  ```gherkin
    Scenario: Silent degradation when the /referees endpoint fails
      Given the refbox has fetched a schedule with individual referee assignments
      And the portal's /referees endpoint call failed (network error, 404, malformed response)
      When the operator navigates to the game-info page for a game with referees
      Then every role displays the localized "Unknown" string
      And the schedule loads and displays successfully despite the failure
      And no error message is shown to the operator about the missing names
  ```

- [ ] **Step 3.6.4: Draft S5.4 — Team-ref fallback shows team names.** Links to B5.10. **Conditional on Group 2 surviving Step 4.**

  ```gherkin
    Scenario: Team-referee fallback when no individual referees are assigned
      Given the refbox has fetched a schedule with no individual referee assignments for a given game
      And the schedule's referees_by_game_number entry for that game has team-level assignments
      When the operator navigates to the game-info page for that game
      Then the page shows the team-referee fallback line
      And it displays "Refs: <team name>" and "Time/Score Keeper: <team name>"
      And the per-role grid (Chief, Timer, Water 1-3) is not shown for this game
  ```

- [ ] **Step 3.6.5: Draft S5.5 — main_view and game_info display identical referee data.** Links to B5.6, B5.7.

  ```gherkin
    Scenario: Main view and game-info page agree on referee data
      Given the refbox has fetched a schedule with individual referee assignments
      And the portal's /referees endpoint returned a name map
      When the operator views the referee list on the main game screen
      And then navigates to the game-info page for the same game
      Then both views show the same referee names in the same role positions
      And both views use the same fallback chain (display_name -> localized "Unknown")
  ```

- [ ] **Step 3.6.6: Draft S5.6 — PII: user.name is never displayed.** Links to B5.13.

  ```gherkin
    Scenario: Account-profile user.name is never displayed
      Given the portal's /referees endpoint response contains a user.name field for one or more referees
      And the same user has a rosterName or username distinct from user.name
      When the refbox builds the name map and renders the referee list
      Then the displayed name is the rosterName (preferred) or username (fallback), never user.name
      And user.name does not appear anywhere in the refbox UI
  ```

  **Verification note:** this scenario requires either real portal data containing `user.name`, or a fixture/manual JSON tweak to inject a `user.name` value. If unreachable in the test session, document the gap in unit notes and flip to `@tested_inconclusive` per the playbook.

### Step 3.7: Cross-check anticipated catalog size

- [ ] **Step 3.7.1: Count drafted B-entries.** Anticipated 18 entries (B5.1–B5.18). If slop-sweep adds more, that's fine — page-batched review handles larger counts per Unit 3 refinement #3. If the count exceeds 25, switch Step 4 from per-entry questions within each group to a single approval question per group with carve-out questions for ambiguous entries (Unit 3's pattern).

---

## Task 4: User review session — feature-group-batched (AUDIT-PLAN.md Step 4)

**Files:** AUDIT-PLAN.md edit only — no code, no commit.

Per Unit 3 refinement #3 and the brainstorm: feature-group-batched review. One approval batch per group, four batches total, with standalone questions for entries the cataloger flagged ambiguous in Step 3.

- [ ] **Step 4.1: Walk Group 1 (Individual-ref display path).**

  Present the seven entries (B5.1–B5.7) as a coherent feature: real referee names on the game-info page, sourced from `/referees`, fallback chain ending at localized "Unknown", main_view and game_info kept in sync. One approval question per group: "Keep the individual-ref display path as cataloged?"

  Carve-out questions to ask separately (only if the entries are present):
  - B5.1 `comments` field — keep or delete the unused field on `RefereeAssignment`?
  - B5.2 `Some(vec![])` vs `None` distinction — does any consumer need to distinguish, or collapse to `Vec<RefereeAssignment>` with default empty?
  - B5.3 / B5.4 silent error path — is silent degradation right, or should `/referees` failure be visible to the operator?

  **One question per message. No bundling.**

  Update each entry's Decision tag in AUDIT-PLAN.md based on the answers. The group-level "keep" answer marks all entries `@user_verified` except any explicitly carved out as `@deleted` by the carve-out questions.

- [ ] **Step 4.2: Walk Group 2 (Team-ref fallback path).**

  Present the five entries (B5.8–B5.12) as a coherent feature: team-ref fallback display when no individual referees exist, sourced from `Schedule.referees_by_game_number`. **Per the brainstorm, no pre-decision — the operator decides keep/delete for the whole group here.**

  One approval question: "Keep the team-ref fallback display path?"

  Carve-out questions:
  - If keep: B5.11 translation rollout — fix in-audit (add team-ref-list in remaining 12 locales) or defer to Findings-Backlog?
  - If keep: B5.12 unread `time_or_score_helper` field — delete this field separately (it's slop regardless of whether the rest of the group is kept)?
  - If delete: confirm cascade — `RefereesByGameNumber`, `Schedule.referees_by_game_number`, the four types, the fallback branch in `config_string`, the `team-ref-list` key in all three locale files, and the conditional B5.18 entry all go together.

  **One question per message.**

  Update Decision tags. If deleted, S5.4 becomes `@deleted` automatically.

- [ ] **Step 4.3: Walk Group 3 (PII handling).**

  Present the two entries (B5.13, B5.14) as a coherent design decision: explicit PII boundary at the `/referees` parser, plus localized "Unknown" fallback. Recommendation: keep both — they are the audited-and-intentional shape.

  One approval question: "Keep the PII boundary (skip user.name, prefer rosterName → username → 'Unknown') as cataloged?"

  Carve-out question: confirm B5.14's inverted-pointer note for Unit 2 Finding #1 (the team-name "Unknown" path is the bypass version; the referee-name path is the reference implementation when Finding #1 gets fixed).

- [ ] **Step 4.4: Walk Group 4 (Cross-cutting & slop).**

  Present the cross-cutting and slop entries (B5.15–B5.18 and any slop-sweep additions). Each is its own decision because they're not a coherent feature:

  - B5.15 commit-ordering anomaly: this is a Process refinements observation, not a code keep/delete. Confirm "keep as record" — leave the catalog entry and add to Process refinements log at Step 9.
  - B5.16 `last_2_min_stop_time` on `TimingRule`: keep the field (it's a legitimate display feature) but flag the bundling as a Process refinements log entry per Unit 2 refinement #2.
  - B5.17 `last_2_min_stop_time: _` destructure: keep (correct given the field is display-only).
  - B5.18 translation gap: conditional on Group 2 outcome. If Group 2 deleted, B5.18 is moot. If Group 2 kept, decision was already made in Step 4.2's carve-out.

  Plus any additional slop-sweep entries from Step 3.5.

- [ ] **Step 4.5: Revisit any `@proposed` entries.**

  At the end of all four batches, any entry still tagged `@proposed` is revisited together. Per the playbook: "sometimes seeing the whole picture clarifies a single item."

- [ ] **Step 4.6: Record the consolidated decision log in AUDIT-PLAN.md's Unit 5 "Decisions" subsection.**

  One line per entry per the decision-log template (`AUDIT-PLAN.md` line 207). AUDIT-PLAN.md edit only; no commit.

---

## Task 5: Surgical pruning (AUDIT-PLAN.md Step 5)

**Files:** depends on which entries are `@deleted`. The most likely deletions cluster in Group 2 (team-ref fallback) if that group is rejected; otherwise B5.12 alone (the unread field) is the most likely single deletion.

- [ ] **Step 5.1: For each `@deleted` entry, remove the relevant code surgically.**

  Use exact file paths and line ranges from `.audit/unit-5-diff.patch`. Do NOT opportunistically refactor adjacent code (per `.claude/rules/scope.md`).

  **Common deletion plays (depending on Step 4 outcome):**

  - **Group 2 deleted in full:** remove `RefereesByGameNumber` alias, `GameReferees`/`TeamRefAssignment`/`TeamRefInfo` types, `Schedule.referees_by_game_number` field, the team-ref fallback branch in `config_string` (the `if !has_individual_refs { ... }` block in `shared_elements.rs`), the `team-ref-list` translation key in `refbox/translations/en-US/refbox.ftl`, `es/refbox.ftl`, and `fr/refbox.ftl`, and the corresponding scenario S5.4 stays tagged `@deleted` but the code is gone.
  - **B5.12 deleted only (`time_or_score_helper` unused field):** remove the field from `GameReferees`; verify no consumer reads it (should be none).
  - **B5.1 `comments` field deleted:** remove the field from `RefereeAssignment`; verify no consumer reads it.

- [ ] **Step 5.2: Run `just fmt` and `just lint` in the worktree.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names && just fmt && just lint`
  Expected: both exit cleanly; no formatting changes; no clippy warnings.

- [ ] **Step 5.3: Run `just check` in the worktree.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names && just check`
  Expected: all jobs succeed — `fmt-check`, `clippy -D warnings`, all tests pass, `cargo audit` clean (modulo the two pre-existing dep advisories per Unit 3 Findings #4).

- [ ] **Step 5.4: Ask the user for approval to commit, then commit per `@deleted` entry or per logical group.**

  Suggested message shapes:
  - `refactor(uwh-common): remove team-ref fallback types and Schedule.referees_by_game_number per audit`
  - `refactor(refbox): remove team-ref fallback branch in config_string per audit`
  - `refactor(refbox): remove team-ref-list translation key per audit`
  - `refactor(uwh-common): remove unused time_or_score_helper field per audit`
  - `refactor(uwh-common): remove unused comments field on RefereeAssignment per audit`

  One commit per behaviour or per logical group. Each commit should compile and pass `just check` independently.

If no entries are `@deleted`, this task is a no-op — proceed to Task 6.

---

## Task 6: Cross-crate verification + test additions (heavy-process)

This is the heavy-process core. Every consumer of `uwh-common` is verified, AND Gherkin scenarios are tested via operator-driven UI walkthrough.

Per Unit 2 refinement #4, Steps 6.1–6.4 are dispatched as a **single composite subagent** running sequential cargo invocations against the shared worktree.

### Step 6.1: New uwh-common unit tests for the audited shape

**Files:**
- Modify: `uwh-common/src/uwhportal/schedule.rs` (extend `mod tests` block)
- Modify: `uwh-common/src/uwhportal/mod.rs` (extend tests for the referees-name-map helper, if a test module exists; otherwise add one)

- [ ] **Step 6.1.1: Add `RefereeAssignment` serde roundtrip test (with and without `display_name`).**

  Add `#[test] fn test_serialize_referee_assignment_skips_display_name()` and `#[test] fn test_deserialize_referee_assignment_ignores_display_name_if_present()` in `uwh-common/src/uwhportal/schedule.rs` `tests` module. Test that:
  - Serializing a `RefereeAssignment { display_name: Some(...), ... }` produces JSON with **no** `displayName` field (because `#[serde(skip)]`).
  - Deserializing JSON that contains a `displayName` field works and the field is set to `None` (skip means "skip both directions").

- [ ] **Step 6.1.2: Add `GameReferees` serde roundtrip — `hybrid` field accepts both object and array.** **Only if Group 2 was kept in Step 4.**

  Test two input shapes:
  - `{"timeOrScoreKeeper": ..., "referees": {"team": {...}}}` (object)
  - `{"timeOrScoreKeeper": ..., "referees": [{"team": {...}}]}` (array — verify the parser tolerates both per `353b476`'s commit message)
  
  If the parser does NOT accept both, this surfaces a divergence between commit message and implementation — log as Process refinements and discuss with operator.

- [ ] **Step 6.1.3: Add `Schedule.referees_by_game_number` Optional roundtrip.** **Only if Group 2 kept.**

  Test: `Some(IndexMap of two GameReferees entries)` and `None` both roundtrip cleanly.

- [ ] **Step 6.1.4: Add URL-construction test for `get_event_referee_name_map_from_referees`.**

  Non-network: instantiate `UwhPortalClient` with a dummy base URL, call the method's URL-building helper (or capture the URL via a mock HTTP transport if the design allows), assert the URL is `<base>/api/events/<partial_id>/referees`. If the method doesn't expose a testable URL-builder, document the gap in unit notes and ADR's "What was not verified" — per Unit 3 refinement #5, don't fake-test by mocking the whole client.

- [ ] **Step 6.1.5: Add `last_2_min_stop_time` default-value test.** **Only if B5.16 was kept.**

  Test that a `TimingRule` JSON missing the `last2minStopTime` field deserializes with `last_2_min_stop_time == false` (the serde default for bool). Test that an explicit `"last2minStopTime": true` deserializes correctly. Test that `TimingRule::into::<GameConfig>()` ignores the field (the field is display-only per B5.17).

- [ ] **Step 6.1.6: Run the new tests.**

  Run (from worktree): `cargo test -p uwh-common --lib uwhportal::schedule::tests::test_serialize_referee_assignment` (or the broader `cargo test -p uwh-common --lib`).
  Expected: all new tests pass.

- [ ] **Step 6.1.7: Commit the new tests.**

  Suggested message: `test(uwh-common): add serde roundtrip tests for RefereeAssignment, GameReferees, and referees-name-map URL`. One commit covering all new tests is fine.

### Step 6.2: Cross-crate consumer verification (single composite subagent)

The same subagent runs Steps 6.2.1 through 6.2.4 sequentially in the worktree, recording each crate's outcome in unit notes.

- [ ] **Step 6.2.1: `schedule-processor` build + tests + `referee_assignments` field exercise.**

  Run: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names && cargo build -p schedule-processor && cargo test -p schedule-processor`
  Expected: clean. Verify in unit notes that `csv_parser.rs`'s 2-line change for `referee_assignments` compiles (it should — `8d4a667` shipped on master with this change).

- [ ] **Step 6.2.2: `overlay` build + tests.**

  Run: `cargo build -p overlay && cargo test -p overlay`
  Expected: clean. The wire format change in `Game` (new `referee_assignments` field) must not break overlay's parsing of game data over the wire.

- [ ] **Step 6.2.3: `led-panel-sim` build + tests.**

  Run: `cargo build -p led-panel-sim && cargo test -p led-panel-sim`
  Expected: clean.

- [ ] **Step 6.2.4: `matrix-drawing` build + tests.**

  Run: `cargo build -p matrix-drawing && cargo test -p matrix-drawing`
  Expected: clean.

- [ ] **Step 6.2.5: Record `wireless-remote` as out-of-scope (separate toolchain).**

  Confirm in unit notes: the new types (`RefereeAssignment`, `GameReferees`, etc.) are not consumed by `wireless-remote/`. Spot-check: `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names/wireless-remote && grep -r "RefereeAssignment\|GameReferees\|referee" src/ 2>/dev/null | head -5` — expected: no matches. Out-of-scope finding recorded; no toolchain switch needed.

### Step 6.3: Seed and align `refbox/tests/features/referee_names.feature`

- [ ] **Step 6.3.1: Create the .feature file with all `@user_verified` scenarios.**

  Per Unit 1 refinement #2, default location is `refbox/tests/features/`. Create `refbox/tests/features/referee_names.feature` in the worktree. Copy every `@user_verified` scenario from AUDIT-PLAN.md's Unit 5 Scenarios subsection verbatim. Scenarios still tagged `@proposed` after Step 4 → carry the tag forward; `@deleted` scenarios stay in AUDIT-PLAN.md only.

  Single `Feature: Referee names in game info` block; all scenarios live inside it.

- [ ] **Step 6.3.2: Commit the seeded .feature file.**

  Suggested message: `docs(refbox): seed referee_names.feature with user-verified scenarios from Unit 5 audit`.

### Step 6.4: Operator-driven UI walkthrough — Session 1

**One session covering all `@user_verified` scenarios.** The operator drives the refbox UI; the principal observes logs and confirms each scenario's Then-lines.

- [ ] **Step 6.4.1: Launch refbox in the worktree.**

  Per memory `feedback_cd_worktree_before_cargo` and `feedback_user_drives_refbox_ui` (updated 2026-05-12 — Claude launches refbox when ready):
  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/audit-unit-5-referee-names && WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox
  ```
  Run in background with `dangerouslyDisableSandbox: true`. The operator drives; the principal monitors logs via the Monitor tool.

- [ ] **Step 6.4.2: Walk S5.1 — Real names appear when /referees returns a name map.**

  Operator: load a real event with individual referee assignments (provide the event ID; the schedule and `/referees` calls run in `request_schedule`). Navigate to a game-info page for a game with referees.
  Principal: confirm logs show both `request_schedule` and a successful `/referees` fetch; confirm the operator sees real names in the per-role grid (Chief, Timer, Water 1–3); no "Unknown" placeholders.
  Tag in .feature file: `@tested_pass`, `@tested_fail`, or `@tested_inconclusive` with a date-stamped comment.

- [ ] **Step 6.4.3: Walk S5.2 — Localized "Unknown" appears when name map has no entry.**

  Operator: same event, but for a game whose referees lack `/referees` entries (or load an event where some assigned `user_id`s are not in the public `/referees` response). Navigate to its game-info page.
  Principal: confirm the localized "Unknown" string appears for unresolved roles. If the system locale is en-US, "Unknown" is the literal English; for other locales, the corresponding translation. The operator can switch locale if needed to confirm localization.

- [ ] **Step 6.4.4: Walk S5.3 — Silent degradation when /referees fails.**

  Operator: simulate `/referees` failure. The cleanest realistic path is to load a schedule while offline or to use an event ID where `/referees` returns 404. If neither is reachable, document as `@tested_inconclusive` and note the gap in unit notes.
  Principal: confirm the schedule still loads, every role shows localized "Unknown", and no operator-visible error appears.

- [ ] **Step 6.4.5: Walk S5.4 — Team-ref fallback shows team names.** **Only if Group 2 survived Step 4.**

  Operator: load a schedule whose games have no individual referee assignments and whose `referees_by_game_number` map has team-level assignments. If real data isn't reachable, the path is harder to exercise — document `@tested_inconclusive` and note in unit notes.
  Principal: confirm the team-referee fallback line appears with team names; the per-role grid is not shown for that game.

- [ ] **Step 6.4.6: Walk S5.5 — main_view and game_info display identical referee data.**

  Operator: view the referee list on the main game screen for a game that has resolved referees (from S5.1). Then navigate to that game's game-info page. Compare role-by-role.
  Principal: confirm both views show the same names in the same role positions, same fallback chain. The 931d01d regression test target.

- [ ] **Step 6.4.7: Walk S5.6 — PII: user.name is never displayed.**

  This requires either (a) real portal data where `/referees` returns `user.name` with values, or (b) a manual fixture. If neither is reachable, document `@tested_inconclusive` and note the gap explicitly. The PII boundary is verifiable in code (B5.13) even without runtime exercise — the audit can mark this scenario `@tested_inconclusive` and still ship the ADR with the design decision documented.

- [ ] **Step 6.4.8: Commit the .feature file with test tags applied.**

  Suggested message: `test(refbox): record Session 1 walkthrough results for referee-names scenarios`.

### Step 6.5: Full `just check` sweep on the audit branch

- [ ] **Step 6.5.1: Run `just check` and record outcome.**

  Run (from worktree): `just check`
  Expected: all jobs succeed; the two pre-existing dep advisories per Unit 3 Findings #4 surface as expected and are not regressions.

  Record final outcome in unit notes.

---

## Task 7: Retroactive ADR 022 (AUDIT-PLAN.md Step 7)

**Files:**
- Create: `docs/decisions/022-referee-names-display.md` in the worktree.

- [ ] **Step 7.1: Confirm the next ADR number.**

  Run (from worktree): `ls docs/decisions/ | sort | tail -5`
  Expected: top numbers visible on this branch are 017, 018 (and possibly 011, 013, etc. from the backlog branch — depends on what merged). Per Unit 1 refinement #8, **use 022** anyway, anticipating the post-merge state (Unit 1 = 019, Unit 2 = 020, Unit 4 = 021). The gap on this branch is expected and closes at Final Integration.

- [ ] **Step 7.2: Write the retroactive ADR using the template in AUDIT-PLAN.md (line 222).**

  Required structure:
  - **Title:** `# ADR 022 — Referee name display in game info`
  - **Status:** `Accepted (retroactive)`
  - **Date:** the audit date (YYYY-MM-DD — convert from `date +%Y-%m-%d`)
  - **Audit unit:** `5 — Referee names display`
  - **Audit PR:** `(to be filled at Final Integration)`
  - **Context:** 2–3 sentences. Refbox needed to display human-readable referee names in game info instead of portal-assigned system codes. The portal exposes referee assignments per game and a public `/referees` endpoint per event that returns user → display-name mappings. The feature shipped across six commits from 2026-04-12 to 2026-04-18 (`996874a`, `d72d643`, `353b476`, `8d4a667`, `1bd4676`, `931d01d`) with AI assistance and was audited 2026-05-13.
  - **Decision:** four sub-sections mirroring the feature groups:
    1. **Individual-ref display path** — plain-English summary of B5.1–B5.7 kept behaviours, plus embedded Gherkin scenarios S5.1, S5.5 (if `@tested_pass`).
    2. **Team-ref fallback path** — present-or-absent depending on Step 4 outcome. If kept: summary of B5.8–B5.11 (less B5.12), plus embedded S5.4. If deleted: brief note that the path was removed; see "What was removed during audit".
    3. **PII handling** — plain-English statement of the rosterName → username → "Unknown" preference with rationale (account-profile real name is not surfaced). Embed S5.6 (if `@tested_pass`); if `@tested_inconclusive`, prose-only with the limit acknowledged.
    4. **Cross-cutting** — note the commit-ordering observation (B5.15) and the `last_2_min_stop_time` bundling (B5.16) as recorded slop, both kept-as-shipped.
  - **Consequences:** what this enables (real referee names on the game-info page); what the refbox now depends on (the public `/referees` endpoint, the per-game `referee_assignments` shape); the silent-degradation contract on `/referees` failure; the PII commitment.
  - **What was removed during audit:** list every `@deleted` entry from Step 5, with the deletion's commit SHA on the audit branch.
  - **What was not verified:** explicit list of skipped exercises (S5.3 if not reachable; S5.4 if Group 2 kept but no real data; S5.6 if no user.name fixture available; any consumer end-to-end exercise that wasn't run). Per Unit 2 refinement #1.
  - **Audit reference:** audit branch `audit/refbox/referee-names`, original commits (the six SHAs), audit-branch commits (from deletion + new-test commits in this unit), worktree path, this plan's path.
  - **Verified by Unit 5 audit (footer):** date, branch, .feature file path.

- [ ] **Step 7.3: Run `just check` to verify the ADR doesn't break the build.**

  Markdown-only doc; expected: clean.

- [ ] **Step 7.4: Ask the user for approval to commit, then commit the ADR.**

  Suggested message: `docs(refbox): add ADR 022 for referee name display in game info (retroactive)`.

---

## Task 8: Hold branch locally (AUDIT-PLAN.md Step 8)

**Files:** none.

- [ ] **Step 8.1: Confirm the branch is NOT pushed.**

  Run (from worktree): `git rev-parse --abbrev-ref HEAD && git config --get branch.$(git rev-parse --abbrev-ref HEAD).remote 2>&1`
  Expected: branch name printed; the second command outputs nothing or `error: key does not exist` (no remote tracking configured). Do not push.

- [ ] **Step 8.2: Optional — offer a bundle backup if 4+ audit units now accumulated locally.**

  Suggested command if accepted: `git bundle create ~/backups/audit-referee-names-YYYY-MM-DD.bundle audit/refbox/referee-names`. The user owns the backup decision.

---

## Task 9: Close the audit unit (AUDIT-PLAN.md Step 9)

**Files:** AUDIT-PLAN.md edits (gitignored, no commit).

- [ ] **Step 9.1: Present the unit's decision log, cross-crate verification results, and ADR summary for user review.**

  Plain-English summary tailored for a non-programmer per `.claude/rules/communication.md`: which feature groups were kept, what carve-out questions surfaced, which consumer crates passed verification, what the UI walkthrough showed for each scenario, whether any test was inconclusive and why, what's recorded in the ADR.

- [ ] **Step 9.2: Wait for explicit "Unit 5 approved" or revision request.**

- [ ] **Step 9.3: On approval, flip Unit 5's status to `complete-pending-integration (YYYY-MM-DD)` in AUDIT-PLAN.md** — both in the audit-unit-catalog table AND in the unit's section heading.

- [ ] **Step 9.4: Add a summary entry to "Completed audits"** per Unit 1 refinement #3 (status flip in place + summary pointer; not a full section move). Include: branch name, plan path, ADR path, catalog outcome, audit-branch commit SHAs.

- [ ] **Step 9.5: Add Findings-Backlog entries** per the brainstorm:

  - **Whole-file dedupe of `main_view.rs` ↔ `game_info.rs` referee-display logic.** Per Unit 4 refinement #3. Branch suggestion: `refactor/refbox/dedupe-referee-display-logic`. Note that `931d01d` synced the two copies, but the duplication itself persists as pattern-consistent debt.
  - **Team-ref-list translation gap** — **only if Group 2 was kept in Step 4 and the gap wasn't closed in-audit.** Branch suggestion: `chore/refbox/team-ref-list-translation-coverage`, or roll into a project-wide translation-coverage sweep.
  - **Edit Unit 2 Finding #1 with a pointer to Unit 5's `fl!("unknown")` pattern.** Add a sentence to Unit 2 Finding #1: "Unit 5's `1bd4676` (referee display) implements the same fallback correctly via `fl!('unknown')` — use that as the reference when fixing this Finding." Edit AUDIT-PLAN.md in place.

- [ ] **Step 9.6: Log Process refinements** under AUDIT-PLAN.md's "Process refinements log → From Unit 5":

  1. **Bundled-fix-in-feature-commit pattern reinforced** (`last_2_min_stop_time` in `8d4a667`, unrelated to the headline referee-names feature). Reinforces Unit 2 refinement #2; no new playbook amendment needed unless the pattern keeps recurring.
  2. **New slop-catching candidate: "method or type defined later than its first caller".** Caught by B5.15's commit-ordering anomaly (`d72d643` calls `353b476`'s method 6 days before the method is committed). Recommend adding to the slop-catching checklist as a separate item — distinct from Unit 4 refinement #2's bundled-commit fan-out, since this pattern is about commit *ordering* not commit *contents*.
  3. **Feature-grouped catalog construction worked cleanly for a multi-feature unit.** Unit 5's six commits don't map 1:1 to features (individual-ref display spans four commits, PII handling spans two). Feature-grouping in Step 3 + group-batched review in Step 4 produced lower review friction than commit-by-commit would have. Add as a recommended pattern for any audit unit where commits don't align with features.

  Note: if any of the anticipated refinements turn out to be moot (e.g. the commit-ordering finding has a benign explanation), omit them — only log what actually came out of the audit.

- [ ] **Step 9.7: Confirm Task 1 → Task 9 are all complete and the acceptance criteria at the top of this plan are satisfied.**

---

## Out-of-scope guardrails

- Findings outside Unit 5 (e.g. an unrelated bug noticed while reading `shared_elements.rs`) go to AUDIT-PLAN.md's "Findings backlog" section. **Do not fix on this branch.**
- The `last_2_min_stop_time` field (B5.16) is in-scope for catalog despite being a bundled scope creep in `8d4a667`. The audit's job is to catalog every behaviour the diff added, including bundled ones. Flag the bundling in Process refinements; the catalog Decision is independent (likely keep, per the brainstorm recommendation).
- The `main_view.rs` ↔ `game_info.rs` whole-file dedupe is OUT of scope for this audit — pre-existing pattern-consistent debt per Unit 4 refinement #3. Findings-Backlog entry; separate branch later.
- The project-wide translation-coverage sweep (across all 15 locales for all keys, not just `team-ref-list`) is OUT of scope. Findings-Backlog candidate; possibly a project-wide chore branch.
- Broader PII review of refbox (beyond the explicit `/referees` boundary in B5.13) is OUT of scope. If the operator wants a sweep — e.g. confirming no other code path surfaces account-profile real names — log as a Findings-Backlog item pointing at the `/referees` boundary as the existing reference implementation.
- `wireless-remote` is explicitly OUT of scope. The new types are not consumed there; no toolchain switch needed. Document in unit notes and ADR.
