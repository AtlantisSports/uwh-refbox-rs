# Audit Unit 7 — Portal Health Indicator: scope & shape design

**Date:** 2026-05-15
**Status:** approved (operator-confirmed section-by-section)
**Unit:** 7 in the AI Code Audit playbook (`AUDIT-PLAN.md`)
**Audit branch:** `audit/refbox/portal-health` (to be cut from `feat/refbox/portal-health-indicator`, NOT master)
**Oracle:** `docs/decisions/011-portal-health-indicator.md` + 4 amendments, with
`docs/superpowers/plans/2026-04-19-portal-health-indicator.md` as secondary

This document scopes Unit 7 of the AI Code Audit. It is the brainstorm output that
feeds `superpowers:writing-plans` for the granular per-unit plan at
`docs/superpowers/plans/2026-05-15-audit-unit-7-portal-health.md`.

---

## 1. Goal & scope boundary

**Goal.** Audit the 46 feature commits between `3ce6bdd` (portal_manager scaffolding)
and `0a5cc2e` (clean stale comments) on `feat/refbox/portal-health-indicator`. For
each distinct behaviour the commits introduce, decide with the operator whether to
keep, delete, or amend it. Produce a Unit 7 branch (`audit/refbox/portal-health`)
that holds locally until Final Integration, alongside an in-place-finalized ADR 011.

### In scope

- All file changes in those 46 commits inside `refbox/src/`, `refbox/translations/`,
  `refbox/assets/`, and `refbox/tests/`.
- The new `refbox/src/portal_manager/` module in full.
- View builder changes in `refbox/src/app/view_builders/` (time banner, detail page,
  attention action page, token-expired page).
- Theme additions in `refbox/src/app/theme/mod.rs`.
- Message variants added in `refbox/src/app/message.rs`.
- `app/mod.rs` rewiring of `handle_game_end`, `post_game_score`, `post_game_stats`.
- Test files added inside the audit window (whether unit tests or integration
  scaffolding).
- New translation keys added by the externalization commit `e6c3954` across all
  15 locales.

### Out of scope (in this section)

- Commit `089c98d` (Renovate rustls-webpki bump) — pure dependency maintenance,
  already on `origin/master`.
- Any change outside `refbox/`. ADR 011 states `uwh-common` is not modified; the
  Step 2 file-list check verifies this.
- The web refbox's portal health design — no web equivalent exists at this time.
- ADR 013 territory (cold-restart state recovery) which addressed the *separate*
  root cause of the 2026-04-18 missing-games incident.

**Scope-creep guard.** Per `.claude/rules/scope.md`, any defect or improvement
surfaced in code outside the 46-commit window goes to the Findings backlog with a
branch suggestion — never into the audit branch.

---

## 2. Acceptance criteria

Unit 7 is **complete-pending-integration** when *all* of:

1. **Catalog fully decided.** Every entry in Unit 7's Behaviour catalog is marked
   `@user_verified` (keep) or `@deleted` (delete). No `@proposed` entries remain.
   Scenario tags (where present) match their catalog entry's decision.
2. **Surgical pruning complete.** Every `@deleted` behaviour has been removed from
   the audit branch with a corresponding pruning commit. Branch history reads
   coherently.
3. **`just check` passes.** Format, lint, tests, audit — all green on Linux.
   Windows + macOS targets verified via `just lint` parity since the original
   branch already ran cross-platform CI.
4. **Walkthrough verified against local uwh-portal.** ADR 011 amendment
   2026-04-22's three scenarios (token-valid green path, mid-tournament network
   drop → yellow → red, induced 409 conflict → force/discard) all walk cleanly.
   Any divergence between observed behaviour and ADR 011 is either resolved on
   the audit branch or recorded as a deliberate divergence in the Verified-by-
   audit section.
5. **ADR 011 finalized in place.** Status flipped `proposed` → `accepted`. Four
   amendments preserved chronologically. New "Verified by Unit 7 audit
   (2026-05-15)" section added listing which catalog entries verified which ADR
   sections, plus any catalog entry that supersedes an amendment.
6. **Running state updated.** Unit 7 status in AUDIT-PLAN.md's catalog table
   flipped to `complete-pending-integration (YYYY-MM-DD)`. Unit 7's Completed
   audits summary entry added. Findings backlog updated with any out-of-scope
   discoveries.
7. **Branch held local.** No push to origin; no PR opened or updated. PR #761 on
   `feat/refbox/portal-health-indicator` remains untouched at its current state.
8. **Memory updated.** Any new feedback or process refinements from Unit 7
   recorded in memory and the playbook's Process refinements log.

---

## 3. Oracle

Source-of-truth priority for "what should this code do":

1. **ADR 011 + 4 amendments** (`docs/decisions/011-portal-health-indicator.md`)
   — primary oracle. Decision section captures the original design; the four
   amendments capture deliberate refinements made during implementation.
2. **Prior implementation plan**
   (`docs/superpowers/plans/2026-04-19-portal-health-indicator.md`) — secondary
   oracle. Useful for resolving ambiguity in ADR 011 (file paths, struct names)
   and for cross-checking commit-to-task mapping at close-out. Where the plan
   and ADR 011 conflict, **ADR 011 wins**.
3. **Operator review** — final tiebreaker. When ADR 011 is silent or the catalog
   entry describes behaviour the operator never weighed in on, the Step 4
   decision is the source of truth.

### Spec status per catalog entry

Each entry carries one of:

- `matches-spec` — behaviour matches ADR 011 (including amendments) directly.
- `diverges-from-spec` — behaviour exists but contradicts the ADR or an
  amendment.
- `not-in-spec` — behaviour exists but the ADR is silent on it.
- `realises-amendment` — behaviour exists *because of* an amendment that landed
  mid-implementation (e.g., dormant-until-event-linked code, removed-green-
  checkmark code).

The `diverges-from-spec` and `not-in-spec` entries are the audit's primary slop
candidates. `realises-amendment` entries are by-design and typically
`@user_verified`.

### Cross-unit code awareness

Per Unit 6 process refinement #3: every catalog entry on files touched by other
audits — notably `make_game_time_button` in
`refbox/src/app/view_builders/shared_elements.rs` and `refbox/src/app/mod.rs` —
runs `git log <hash>..HEAD -- <file>` during construction and includes a one-line
cross-unit note if overlapping changes exist.

---

## 4. Catalog construction & review pattern

**Construction.** Diff-led per Unit 4 process refinement #4. For each of the 46
commits, read the diff and write one catalog entry per distinct behaviour. Apply
the slop-catching checklist (commit fan-out, bundled fixes, internal helpers
without callers, defaults/clamping, side effects, new strings, unwrap audits) as
each entry is built. Each entry follows the template at AUDIT-PLAN.md
"Behaviour catalog entry".

**Expected catalog size: 50–80 entries**, distributed roughly as:

| Subsystem | Approx. entries | Examples |
|---|---|---|
| `portal_manager` module | 15–25 | scaffold types, queue file format + atomic load/save, retry-eligibility rules, health-check cadence decision, indicator-state computation, PortalTaskIo trait, public API, background task spawn, error handling |
| `app/mod.rs` rewiring | 5–10 | game-end routing, SelectedEventId writer, login-keypad-panic fix, snapshot-push-on-immediate-retry, dormant-until-event-linked gate, advisory-on-confirm-score |
| UI surface | 15–25 | time-banner tile + sizing, detail page scaffolding + row ordering + paragraph-cache, attention action page, token-expired action page (later retired), per-game confirm advisory, mode-aware logo, externalized strings, layout passes |
| Tests + assets | 5–10 | unit tests under portal_manager/, queue round-trip, retry-eligibility tests, indicator-state tests, the compact UWH Portal logo asset, the removed `check_circle.svg` |
| Env-var hooks | 2–4 | `UWH_PORTAL_URL_OVERRIDE`, `UWH_PORTAL_SCRAMBLE_TOKEN`, dev-only tokio feature |

**Review pattern.** Page-batched + ambiguity carve-outs per Unit 3 process
refinement #3:

- Group entries by subsystem (the 5 rows above).
- Present each group to the operator as a single approval question with a per-
  entry recommendation list.
- Carve out entries flagged as **ambiguous-by-design** (entries where the auditor
  sees a real keep/delete trade-off) and ask them as standalone questions.
- Expected total: 12–20 review questions, walking 50–80 entries in roughly
  30–45 minutes.

**Scenarios.** UI-facing entries get linked `.feature` scenarios stored at
`refbox/tests/features/portal_health.feature` per Unit 1 refinement #2. Backend-
only entries (queue file format, health-check decision logic) do not get
scenarios.

---

## 5. Setup & branch

### Branch creation

The audit branch `audit/refbox/portal-health` is cut from
`feat/refbox/portal-health-indicator` HEAD (`0a5cc2e`), **not** from master.
This is the explicit playbook deviation for Unit 7 (see AUDIT-PLAN.md): we audit
from the feature branch's tip so we can rewrite it without disturbing the public
PR #761 until decision time. Local-only branch; no push.

### Worktree

A fresh worktree is cut for the audit branch at
`.worktrees/audit-unit-7-portal-health/`:

```
git worktree add .worktrees/audit-unit-7-portal-health \
  -b audit/refbox/portal-health feat/refbox/portal-health-indicator
```

This costs a fresh cargo build (~5 min) but preserves the existing
`.worktrees/portal-health/` on `feat/refbox/portal-health-indicator` so the
audit can `git diff` between the audit branch and the PR head during pruning.
Naming follows the Unit 1–6 convention.

### Pre-commit hook

Per Unit 1 refinements #1 + #5: copy `scripts/pre-commit` to the main repo's
`.git/hooks/` (which all worktrees share). The hook now accepts `audit` as a
branch type — fixed on Unit 1's branch and inherited locally.

### Local uwh-portal setup

Operator boots the uwh-portal API + DB locally before Step 6 walkthrough. Source
at `/home/estraily/projects/uwh-portal/`. Refbox launched from the audit worktree
with:

```
UWH_PORTAL_URL_OVERRIDE=http://localhost:5000 \
WAYLAND_DISPLAY= RUST_LOG=info \
cargo run -p refbox
```

per `feedback_refbox_wsl_wayland_unset` (forces X11 via XWayland to dodge the
WSLg Wayland surface crash) and `feedback_user_drives_refbox_ui` (operator drives
the UI; Claude launches it in the background with `dangerouslyDisableSandbox`).

A test event is created in the local DB. Walkthrough scenarios reference this
setup.

### Sandbox

Cargo invocations that launch the refbox GUI use `dangerouslyDisableSandbox:true`
per `feedback_run_command` (Wayland socket access required).

---

## 6. Risk register

| ID | Risk | Mitigation |
|---|---|---|
| R1 | PR #761 accidental update | Audit on a fresh branch (Section 5). Never push during the audit. Audit branch is local-only until Final Integration. |
| R2 | Cross-unit shared files mis-attributed | Per Unit 6 refinement #3: every catalog entry on `shared_elements.rs` / `app/mod.rs` runs `git log <hash>..HEAD -- <file>` during construction; one-line cross-unit note where overlapping changes exist. |
| R3 | Cross-branch dependencies | Step 2 lists every file touched; `git log --all -S '<key-symbol>'` per Unit 5 refinement #1 runs for any catalog entry that references symbols whose origin isn't obvious. If detected, hand-apply on the audit branch and record per Unit 5 ADR 022 pattern. |
| R4 | Catalog size pushes past 80 | If catalog crosses 80, pause Step 3 and re-batch by subsystem before continuing to Step 4. Don't try to walk an oversized catalog with the original batching. |
| R5 | Local portal setup blocks Step 6 | Pre-flight check at end of Step 5: operator boots local portal, refbox connects, test event reachable, one trivial submission lands. If pre-flight fails, fall back to `dev.uwhportal.com` per ADR 011 amendment 2026-04-22's fallback note. Decision happens at pre-flight, not mid-walkthrough. |
| R6 | Catalog reveals an unrecorded behaviour shift | Standard audit process catches this — entries get `not-in-spec` or `diverges-from-spec` status, become carve-out review questions, operator decides keep + amend-ADR or delete. Watch in particular: commit `5de76ed` (detail/attention UX pass) for unrecorded shifts. |

---

## 7. Close-out & ADR 011 finalization

ADR 011 finalization commit lands on `docs/workspace/backlog-adrs` (the branch
where ADR 011 lives), not on the audit branch — same pattern as Unit 3 used for
ADR 009 (per Unit 3 refinement #6).

### Step 7 edits to `docs/decisions/011-portal-health-indicator.md`

1. **Flip status.** Line 4: `**Status:** proposed` →
   `**Status:** accepted (verified by Unit 7 audit 2026-05-15)`.

2. **Preserve amendments.** The four amendments (2026-04-21, 2026-04-22,
   2026-04-23a, 2026-04-23b) stay verbatim. No edits to amendment text.

3. **Add new section** at the bottom of the file, after the four amendments.
   Template below — the `<...>` placeholders and the `B7.X`/`B7.Y`/`B7.Z`
   examples are filled in at close-out with the actual catalog entry IDs
   produced by the audit. The structure itself is fixed:

   ```
   ## Verified by Unit 7 audit (2026-05-15)

   ### Audit scope
   <link to AUDIT-PLAN.md Unit 7 section + audit branch name; commit range
   3ce6bdd..0a5cc2e; date>

   ### Verified decisions
   - Status indicator (Decision section) — verified by entries B7.X (...),
     B7.Y (...), B7.Z (...)
   - Detail page (Decision section) — verified by entries ...
   - Per-item action pages (Decision section, refined by amendment
     2026-04-21) — verified by entries ...
   - portal_manager module (Decision section) — verified by entries ...
   - Conflict resolution (Decision section, refined by amendment 2026-04-21)
     — verified by entries ...
   - Amendment 2026-04-22 (translation coverage + verification env) —
     verified by entries ...
   - Amendment 2026-04-23 (dormant until event linked) — verified by
     entries ...
   - Amendment 2026-04-23 (remove 10-second green-checkmark overlay) —
     verified by entries ...

   ### Supersessions
   <any catalog entry whose @user_verified decision contradicts an ADR
   section or amendment; empty if none>

   ### What was removed during audit
   <list of @deleted behaviours, one-line reason per delete>

   ### What was not verified
   <explicit gaps; e.g. "Real-portal-data shape against dev.uwhportal.com
   was not exercised; only local portal walkthrough"; empty if no gaps>
   ```

4. **Commit on `docs/workspace/backlog-adrs`.** One commit, ADR-only:
   `docs(refbox): finalize ADR 011 and record Unit 7 audit verification`.
   No push.

### Step 8 (Hold local)

Audit branch and ADR-amendment commit both stay local. PR #761 untouched.

### Step 9 (Close)

Done as a *separate* commit on `docs/workspace/backlog-adrs` (so the ADR commit
in Step 7 stays focused on the ADR; the AUDIT-PLAN.md edits don't bleed in):

- Update AUDIT-PLAN.md catalog table line 64: `not started` →
  `complete-pending-integration (YYYY-MM-DD)`.
- Add Unit 7 summary entry to the "Completed audits" section.
- Add any Unit 7 Findings backlog items + Process refinements.
- Memory updates for any new feedback rules (separate file writes, not part
  of this commit).
- Commit: `docs(workspace): close audit unit 7 (portal health indicator)`.

---

## 8. Out of scope (explicit)

Logged to Findings backlog with branch suggestions if surfaced, never fixed on
the audit branch:

- **The web refbox's portal health story.** No web equivalent at this time. If
  one ships later, that's a separate revisit per ADR 011's Reference section.
- **ADR 013 territory** (cold-restart state recovery for missing games 10–12 at
  the 2026-04-18 tournament). ADR 011 explicitly defers this. Unit 7 audits
  only the proactive portal-health hardening.
- **Pre-existing unwraps outside the audit window**, including any `confy::store`
  patterns flagged at Unit 3 finding #2. Per Unit 4 refinement #3, pattern-
  consistent module debt becomes a Findings entry for the whole module, not an
  audit blocker.
- **The compact UWH Portal logo asset's licensing or visual design.** Treated as
  fixed; only its usage is audited.
- **Future portal client API changes in `uwh-common`.** ADR 011 commits to "no
  change to `uwh-common`'s portal client API." If the audit surfaces a place
  where `uwh-common` *should* expose more (e.g., a structured error type so the
  refbox can distinguish 409 from other failures), it becomes a Findings entry
  for a separate `feat/uwh-common/...` branch.
- **LED panel, overlay, wireless-remote, schedule-processor** — all explicitly
  not affected by ADR 011 and not in any audit-window commit. Verify untouched
  in Step 2 by checking the diff path list. Any accidental touch becomes a slop
  entry.
- **Translation accuracy.** The audit catalogs that translation keys are added
  in 15 locales (per `e6c3954`), but does not audit the linguistic correctness
  of the strings — that's deferred to Unit 8 per its scope reduction.
- **Performance.** No catalog entries are created for performance-only concerns
  (allocation patterns, file I/O frequency, render cost) unless they manifest
  as operator-observable behaviour.

**What this section is not.** Not a closed list of forbidden topics — anything
genuinely necessary to verify the in-scope behaviour can pull in adjacent
context as needed (per `.claude/rules/scope.md`, "if the fix demonstrably
requires it"). Just the default decision-rule.

---

## References

- `AUDIT-PLAN.md` — playbook, audit unit catalog, per-unit workflow, slop-catching
  checklist, Findings backlog, Process refinements log, Completed audits.
- `docs/decisions/011-portal-health-indicator.md` — primary oracle.
- `docs/superpowers/plans/2026-04-19-portal-health-indicator.md` — secondary
  oracle.
- `.claude/rules/scope.md` — scope enforcement.
- `.claude/rules/communication.md` — approval gates, plain English.
- `.claude/rules/rust.md` — MSRV, clippy zero-warnings, no `unwrap()` without
  justification, `no_std` boundary.
- `.claude/rules/plan-execution.md` — heavy process triggers (state machine,
  portal comms, refbox UI).
- `feedback_refbox_wsl_wayland_unset` — `WAYLAND_DISPLAY=` for native refbox
  launch under WSLg.
- `feedback_user_drives_refbox_ui` — operator drives the UI; Claude launches in
  background with sandbox disabled.
- `feedback_check_rule_applicability` — read process output, check the rule's
  *why*, don't kill-loop.
- `prs-deferred-until-audit-done` — all PRs/merges deferred until every audit
  unit is complete-pending-integration.
- Process refinements from Units 1–6 in AUDIT-PLAN.md's running state.
