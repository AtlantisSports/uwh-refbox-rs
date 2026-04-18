# Post-v0.4.0 UI Backlog — Implementation Roadmap

> **For agentic workers:** This is a roadmap across five ADRs (006–010). Each phase below becomes its own branch + PR. Detailed TDD plans are written immediately before each phase begins (using superpowers:writing-plans) and saved alongside this file as `2026-XX-XX-<feature-slug>.md`. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship ADRs 006, 007, 008, 009, and 010 as five independent PRs, sequenced to minimise churn, respect UI-layer dependencies, and land the highest-value fix first.

**Strategy:** One ADR = one branch off `master` = one PR. Each PR is reviewable, mergeable, and shippable on its own. Back-port ADRs (009, 010) follow the web refbox verbatim per the back-port rule. No work lands on `docs/workspace/backlog-adrs` — that branch stays local for the ADR documents themselves.

**Tech stack:** Rust 1.85 / edition 2024, iced 0.13 GUI, cargo workspace, `just` task runner, fluent translation system.

---

## Recommended order

| Phase | ADR | Effort | Depends on | Why this slot |
|---|---|---|---|---|
| 1 | **007** — help expand page | 1–2 days | — | Real bug (Cancel/Done buttons pushed off-screen when help text wraps many lines — worsens in German/Italian). Self-contained refbox-only UI work. Revised 2026-04-19: the original Part A "add Length::Fill" fix was a no-op (already in code); the preview + expand-page design is now the sole fix. |
| 2 | **006** — multi-remote alarm buttons | 1–2 days | — | Self-contained refbox-only UI work. No cross-crate or portal coordination. Ships diagnostic value (visible confirmation that remote presses arrived) before the next tournament. |
| 3 | **009** — settings navigation | ~2 days | — | Prerequisite for 010 (creates the User Options page that hosts the View Mode button). Visually noticeable but functionally inert — safe to merge on its own. |
| 4 | **010** — display modes (dark / high-contrast) | 2–3 days | 009 | Adds the `VIEW MODE` button onto the page 009 creates. Requires every inline colour across view_builders to route through the theme — easier once 009's settings refactor is in place, so the two refactors don't race over the same files. |
| 5 | **008** — Game Block | 3–5 days + portal coordination | — | Largest. Touches `uwh-common`, `tournament_manager`, config migration, portal decoder, config screen, main-screen indicator. Has a cross-system prerequisite: the portal must add a `game_block` field before portal-driven refboxes ever leave old-world mode. Parking at the end lets portal coordination start in parallel with small wins shipping. |

**Total wall-clock estimate:** ~10–14 development days across all five PRs.

**Alternative orderings considered and rejected:**

- *Start with 008 because it's the largest.* Rejected — longest feedback loop, blocks on portal, portal coordination is better started in parallel while smaller PRs ship.
- *Bundle 009 and 010 into one PR.* Rejected — two separable concerns, and 009 is useful on its own (referees who use both web and native refbox get matching muscle memory immediately).
- *Swap phases 1 and 2 (do the multi-remote alarms first).* Reasonable alternative. Chose help-expand first because it's a fix for a reproducible overflow bug, which ranks ahead of a feature addition.

---

## PR groupings (five PRs)

| # | Branch | ADR | Scope crate | Notes |
|---|---|---|---|---|
| 1 | `feat/refbox/help-expand-page` | 007 | refbox | Adds FTL keys across 15 languages |
| 2 | `feat/refbox/multi-remote-alarms` | 006 | refbox | UI-only, no `uwh-common` touch |
| 3 | `refactor/refbox/settings-navigation` | 009 | refbox | Type = `refactor` (no behaviour change, layout-only) |
| 4 | `feat/refbox/display-modes` | 010 | refbox | Adds `AppConfig` field; config defaults handle missing value |
| 5 | `feat/uwh-common/game-block` | 008 | uwh-common (broadest) | Per workspace rule, scope = broadest crate |

All branch off `master`. Each merges via the non-programmer review checklist. The ADR files themselves stay on the local-only `docs/workspace/backlog-adrs` branch — they are the spec, not the change, so they are **not** copied into the feature branches.

---

## Per-ADR phase breakdowns

Each phase below is a file-level sketch. The detailed TDD plan for a phase is written **immediately before that phase's branch is cut** — after reading the relevant code fresh — and saved as its own plan file.

### Phase 1 — ADR 007: help expand page

**Branch:** `feat/refbox/help-expand-page`
**Plan file (to be written):** `2026-04-XX-help-expand-page.md`

**Files to modify:**
- `refbox/src/app/view_builders/configuration.rs` — specifically `build_game_parameter_editor` at line 950. Swap in-place help block for preview + "More..." affordance.
- `refbox/src/app/message.rs` — `ShowHelp(LengthParameter)` / `ReturnFromHelp` message pair (or similar naming).
- `refbox/src/app/mod.rs` — state-machine support for the new `AppState::HelpPage` variant and for preserving the editor's in-progress value on return.
- New view builder: the full-screen help page. Can live alongside the editor in `configuration.rs` or be factored out depending on size.
- `refbox/translations/*/refbox.ftl` — one new page-title key per help-bearing parameter (roughly 8 new keys × 15 languages). Existing help strings are reused verbatim.

**Open design questions to resolve before writing the plan:**

1. Preview line count — 2 or 3? Test across laptop and smallest touchscreen aspect ratios.
2. Expand affordance form — text "More..." link, or an icon?
3. Back-target state preservation — confirm the existing `AppState::ParameterEditor(param, dur)` tuple carries enough state to round-trip the editor's in-progress value through a help-page visit.

**Phase structure (the plan will expand each into TDD steps):**

1. Failing test: help page opens from the editor, back button returns to the editor.
2. Failing test: in-progress value in the editor survives the round-trip.
3. Implement the new `AppState` variant plus the message pair.
4. Implement the preview + "More..." affordance in `build_game_parameter_editor`.
5. Implement the full-screen help view.
6. Add FTL keys — English first, then all other 14 languages (follow the existing translation pattern established during v0.4.0).
7. Visual QA: confirm Cancel/Done are always visible in every supported language on the nominal-break and other help-bearing screens.
8. `just check` green.
9. Commit(s). Open PR.

**Risk:** Medium. New navigation level + state preservation + 15 languages. Visual QA is the critical gate — this is the fix for the real bug.

---

### Phase 2 — ADR 006: multi-remote alarm buttons

**Branch:** `feat/refbox/multi-remote-alarms`
**Plan file (to be written):** `2026-04-XX-multi-remote-alarms.md`

**Files to modify:**
- `refbox/src/app/view_builders/main_view.rs` — renders the alarm tile(s); replace single tile with a dynamic row.
- `refbox/src/app/message.rs` — new message variant(s) per remote, and/or a `RemoteId`-keyed variant.
- `refbox/src/app/mod.rs` — `update()` handler for the new messages; hook for incoming radio packet flash.
- `refbox/src/sound_controller/button_handler/mod.rs` — surface incoming remote ID to the app (flash source).
- `refbox/src/app/theme/` — pressed-state visual already exists; reuse.
- Translations — new FTL keys if labels change.

**Open design questions to resolve before writing the detailed plan** (per ADR 006):

1. Spacebar behaviour — retire, keep as "primary remote", or rebind to "all remotes"?
2. Label strategy — sound-name default, or operator-set pairing label?
3. Keyboard `1`–`4` — confirm no existing bindings collide.

These must be answered with the human (likely a brief pre-plan discussion) before the plan is written.

**Phase structure:**

1. Failing test: with N paired remotes, main view renders N tiles.
2. Implement dynamic tile row sized by `sound_settings.remotes.len()`.
3. Test: tapping tile `i` dispatches the configured sound for remote `i`.
4. Test: incoming radio packet from remote `i` visually flashes tile `i`.
5. Keyboard binding: digits `1`–`4` trigger tiles.
6. Resolve spacebar behaviour (per design-question answer).
7. `just check` green. Commit. PR.

**Risk:** Low–medium. Main-view layout has real estate constraints; verify on small touchscreen dimensions.

---

### Phase 3 — ADR 009: settings navigation

**Branch:** `refactor/refbox/settings-navigation`
**Plan file (to be written):** `2026-04-XX-settings-navigation.md`

**Files to modify:**
- `refbox/src/app/message.rs` — add `ConfigPage::User` variant.
- `refbox/src/app/view_builders/configuration.rs` — rework Main page to 2×2 grid; add new `make_user_options_page`; verify uniform chrome on all sub-pages (timer bar / CANCEL-DONE / timeout ribbon).
- `refbox/src/app/mod.rs` — `update()` handler for the new variant; routing from User Options to Display / View Mode / Sound leaves nav-stack intact.
- `translations/*/main.ftl` — `user-options` label; any uppercase relabelling to match the web convention.

**Authoritative reference (back-port rule):**
- `uwh-portal/components/refbox/pages/SettingsMainPage.tsx` — 2×2 grid layout.
- `uwh-portal/components/refbox/pages/UserOptionsPage.tsx` — User Options sub-layout.

**Accepted deviation:** Language stays a list page (not cycle). No other deviations.

**Phase structure:**

1. Read web source for the two pages and pull exact tile order, label casing, spacing.
2. Failing test: tapping `USER OPTIONS` on Main navigates to the new User page.
3. Failing test: tapping `DISPLAY OPTIONS` from User navigates to existing `ConfigPage::Display`.
4. Add `ConfigPage::User`, router logic, view builder.
5. Rework Main page to 2×2 grid (4 tiles).
6. Audit uniform chrome — any sub-page missing timer bar / footer / ribbon gets it.
7. Add / adjust FTL keys across all 15 languages.
8. Snapshot or manual screenshot comparison vs web source.
9. `just check` green. Commit. PR.

**Risk:** Medium. Central message enum change has small blast radius across the app. No behaviour change reduces risk.

---

### Phase 4 — ADR 010: display modes

**Branch:** `feat/refbox/display-modes`
**Plan file (to be written):** `2026-04-XX-display-modes.md`

**Depends on Phase 3 landing on master first.**

**Files to modify:**
- `refbox/src/app/theme/` — add `DisplayMode` enum (`Default`, `Dark`, `HighContrast`); every style resolves its colour through the active mode.
- `refbox/src/config.rs` (or wherever `AppConfig` lives) — new `display_mode: DisplayMode` field; serde default = `Default`.
- `refbox/src/app/message.rs` — `CycleDisplayMode` message.
- `refbox/src/app/view_builders/configuration.rs` — add `VIEW MODE` cycle button on the User Options page (new in Phase 3).
- `refbox/src/app/mod.rs` — `CycleDisplayMode` handler writes config immediately (live-apply, no DONE step).
- Every `view_builders/*.rs` file — any inline colour becomes a theme lookup.
- `translations/*/main.ftl` — `display-mode-default`, `display-mode-dark`, `display-mode-high-contrast`, `view-mode` labels.

**Authoritative palette source:**
- `uwh-portal/styles/refbox-theme.scss` — all three mode palettes. Rust copies values verbatim.
- `uwh-portal/contexts/RefboxThemeContext.tsx` — live-swap behaviour model.

**Phase structure:**

1. Read web SCSS palette file; produce a colour-table doc under the plan.
2. Add `DisplayMode` enum, wire through theme layer (Default-only first to prove the routing works with no visible change).
3. `just check` green; visually verify `Default` looks identical to today.
4. Add `Dark` palette; `VIEW MODE` button cycles; verify live repaint.
5. Add `HighContrast` palette; verify.
6. Persist to `AppConfig`; verify startup picks saved mode.
7. Sweep view_builders for any remaining inline colours; route through theme. Each sweep file gets its own commit to keep the diff reviewable.
8. Add FTL keys across 15 languages.
9. `just check` green. Commit-stack clean. PR.

**Risk:** Medium–high. The inline-colour sweep is the riskiest part — easy to miss one, and a missed inline colour is visibly wrong in Dark/HC mode. Budget extra time for visual QA in all three modes.

---

### Phase 5 — ADR 008: Game Block

**Branch:** `feat/uwh-common/game-block`
**Plan file (to be written):** `2026-04-XX-game-block.md`

**Prerequisite (cross-system):** The portal must ship a schema that carries a `game_block` field in its `TimingRule` shape **before** any portal-driven refbox can enter new-world mode. This is coordinated outside this workspace but the refbox-side work can proceed (it accepts both shapes; until the portal ships new-shape rules, all portal-driven refboxes stay in old-world mode).

**Files to modify:**

*uwh-common:*
- `uwh-common/src/config.rs` — `GameConfig` gains `game_block: Duration`. `Config::migrate` auto-computes `game_block` from `2·half + half_time + nominal_break` (or `period + nominal_break` for single-period) on first read.
- `uwh-common/src/uwhportal/schedule.rs` (around line 314–338) — timing-rule decoder accepts both shapes.

*refbox:*
- `refbox/src/tournament_manager/mod.rs` — lines ~1040–1044 and ~1970–1974: replace `2·half + half_time + nominal_break` with `game_block` when in new-world mode.
- `refbox/src/app/view_builders/configuration.rs` — lines ~436–441: replace always-visible "Nominal Break" button with mode-dependent button ("Nominal Break" old-world, "Game Block" new-world). Add the warning-only colour validation (red/yellow/gray) on the Game Block button and its editor.
- `refbox/src/app/view_builders/main_view.rs` — right side of time bar, silent-by-default overrun indicator.
- `refbox/src/app/mod.rs` — state for accumulated overrun; mode selection based on portal data + standalone-mode.
- `translations/*/main.ftl` — `game-block`, `game-block-help`, validation-error strings across 15 languages.

**Design questions to resolve before writing plan** (per ADR 008):

1. Exact yellow-threshold formula (timeout absorption).
2. Portal/rule disagreement handling — refuse / warn / cap?
3. Runtime indicator format — `-M:SS` or rounded `-M`?

**Phase structure (high level — this plan will be large):**

1. `uwh-common`: add field + migration + portal decoder (both shapes). Unit tests.
2. `refbox`: tournament_manager switched to read `game_block` in new-world mode.
3. Configuration screen: mode-aware button + editor with colour validation.
4. Main screen: overrun indicator.
5. Translations across 15 languages.
6. Downstream crate check (`refbox`, `schedule-processor`, `overlay`, `led-panel-sim`, `matrix-drawing` all still compile).
7. `just check` green. Separate commits for uwh-common vs refbox layers. PR.

**Risk:** High. Largest surface area, cross-crate, cross-system. Strong candidate for an interim mid-phase review before the PR is opened.

---

## Execution mechanics

For each phase, the flow is:

1. **Pre-plan discussion** — resolve any open design questions the ADR flagged.
2. **Write detailed plan** — using superpowers:writing-plans, by reading the relevant code fresh, producing TDD-level steps with actual code and commands. Save the plan file.
3. **Approval gate** — human reads the plan, approves (or adjusts).
4. **Branch + execute** — cut the branch from `master`, work through the plan's tasks, commit as specified.
5. **`just check` must pass before PR open.**
6. **PR open** — body uses the `What changed / Why / Scope / How to verify` structure from `.claude/rules/pr-review.md`.
7. **Human reviews** via `docs/review-checklist.md`.
8. **Merge** — back to step 1 for the next phase.

ADRs are **not** copied into the feature branches. When a phase ships, the corresponding ADR is updated on the local backlog branch (`docs/workspace/backlog-adrs`) from status `proposed` to status `accepted`, and its "References" section is amended with the merged PR number.

---

## Open items that are not phases

These are tracked separately, not in this roadmap:

- Publish the v0.4.0 GitHub draft release (human action).
- Portal-side `game_block` schema work (external coordination — prereq for ADR 008 new-world mode).
- Native-speaker translation review for each UNVERIFIED language.
- Cucumber harness PR (feature files present, test runner not wired).
- Full scoresheet feature (deferred on `feat/workspace/desktop-build`).
