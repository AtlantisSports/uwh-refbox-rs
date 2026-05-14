# ADR 022 — Referee name display in game info

**Status:** Accepted (retroactive)
**Date:** 2026-05-14
**Audit unit:** 5 — Referee names display
**Audit branch:** `audit/refbox/referee-names`
**Audit PR:** (to be filled at Final Integration)

---

## Context

When the refbox shows a game's information — either on the main game screen or on the dedicated game-info page — operators need to see who the referees for that game are. Before the work covered here, the only data available was a system identifier code that wasn't human-readable. The portal exposes a public `/referees` endpoint per event that returns user-ID → display-name mappings, so the refbox can resolve those codes into real names.

The feature shipped across six commits between 2026-04-12 and 2026-04-18 (`353b476`, `8d4a667`, `996874a`, `d72d643`, `1bd4676`, `931d01d`) with AI assistance. It was audited 2026-05-13/14 on branch `audit/refbox/referee-names` (cut from `origin/master`). This ADR records the surviving behaviours, the design decisions, and what was removed.

---

## Decision

The audit retained four coherent behaviours and removed one. Each is summarised in plain English below; operator-observable behaviours embed their `@user_verified @tested_pass` Gherkin scenarios from `refbox/tests/features/referee_names.feature` verbatim.

### 1. Individual-ref display path

When the refbox fetches a schedule from the portal, it now also calls the public `/referees` endpoint for that event and builds a user-ID → display-name map. Each game in the schedule carries a list of referee assignments with role labels (Chief, Water1, Water2, Water3, TimeOrScoreKeeper). When the operator opens a game's info, the refbox renders a per-role grid showing the real referee names resolved through the name map.

This grid appears on both the main game screen's info panel and the dedicated Game Info page; both render through the same role-matching logic (commit `931d01d` brought the game-info page in sync with the main view after an earlier divergence).

```gherkin
Scenario: Real referee names appear when the portal returns a name map
  Given the refbox has fetched a schedule for an event with individual referee assignments
  And the portal's /referees endpoint returned display names for every assigned user_id
  When the operator navigates to the game-info page for a game with referees
  Then the page shows the per-role grid (Chief, Timer, Water 1, Water 2, Water 3)
  And each role displays the referee's resolved display_name
  And no role shows the '-' placeholder for a slot that has a resolved name
```

```gherkin
Scenario: Main view and game-info page agree on referee data
  Given the refbox has fetched a schedule with individual referee assignments
  And the portal's /referees endpoint returned a name map
  When the operator views the referee list on the main game screen
  And then navigates to the game-info page for the same game
  Then both views show the same referee names in the same role positions
  And both views use the same fallback chain (display_name then '-')
```

**Backend support:** Adds `RefereeAssignment` type (with `role`, `user_id`, and a transient `display_name` field marked `#[serde(skip)]`), adds `referee_assignments` to `Game`, adds `get_event_referee_name_map_from_referees` method on `UwhPortalClient`. Tested via the new serde roundtrip test in `uwh-common/src/uwhportal/schedule.rs` (commit `d8fa6db`).

### 2. PII handling

The portal exposes three different fields for each referee that could each serve as a "name": `rosterName` (the referee's chosen tournament handle), `user.username` (the account login), and `user.name` (the account-profile real name, which may include middle names or other personal detail). The refbox **deliberately skips `user.name`** and uses `rosterName → username` as its preference chain. The chosen handle is the appropriate value for an operator-facing screen visible at poolside; the account-profile real name is treated as PII the refbox UI should not surface.

When all three sources of name resolution are empty, the slot is rendered as the literal `-` character (one of the audit's substantive amendments — see Section 5 below for the rationale).

```gherkin
Scenario: Account-profile user.name is never displayed
  Given the portal's /referees endpoint response contains a user.name field for one or more referees
  And the same user has a rosterName or username distinct from user.name
  When the refbox builds the name map and renders the referee list
  Then the displayed name is the rosterName (preferred) or username (fallback), never user.name
  And user.name does not appear anywhere in the refbox UI
```

```gherkin
Scenario: "-" placeholder appears when a referee has no display name
  Given the refbox has fetched a schedule with individual referee assignments
  And the portal's /referees endpoint returned a name map missing entries for one or more user_ids
  When the operator navigates to the game-info page for that game
  Then the unresolved roles display the literal '-' placeholder
  And the '-' placeholder is shown identically regardless of locale
  And the portal-assigned identifier code is never displayed in place of the name
```

**Concrete verification from the audit walkthrough (2026-05-14):** the Canadian Nationals event includes a referee whose `rosterName` is `"Lewis Saleem"` and whose `user.name` is `"Lewis Adam Saleem"`. The refbox displays `"Lewis Saleem"` — the `user.name` middle-name variant never appears in the UI. This is direct evidence the PII boundary works as designed.

### 3. Stop-clock display

A secondary behaviour, bundled into the same audit window without explicit mention in the original commit message: the game-info display now shows the actual stop-clock-in-last-2-minutes setting for the game (`Yes` or `No`), sourced from a new `last_2_min_stop_time` field on `TimingRule`. Previously this row always showed the literal "Unknown". The field is read by `config_string` and `details_strings` for display only — the game clock's actual stop-clock behaviour is governed elsewhere in the tournament manager and is not coupled to this display field. **The bundling itself was flagged in the audit's Process refinements log** — bundling unrelated behavioural changes into feature commits is the slop pattern Unit 2 refinement #2 first identified. The behaviour was kept because the operator confirmed stop-clock data is always Yes/No in practice and the display is useful for game preparation; the bundling concern is recorded for future practice.

### 4. Silent degradation when `/referees` fails

If the `/referees` HTTP call fails for any reason — network down, 404, malformed response, server error — the schedule still loads, every referee slot shows the literal `-`, and the refbox **logs a warning** (`Failed to fetch referee names: …`) without surfacing an error to the operator. This silent-but-logged degradation was decided during the audit: the alternative of blocking schedule fetch on name resolution would mean a name-server outage prevents the operator from running games, which is the wrong trade-off. The warn-level log line gives a diagnostic breadcrumb without affecting operator workflow. (The original AI-authored code degraded silently with no log at all; the audit's amendment added the log.)

This behaviour was **not** exercised at runtime during the audit walkthrough (the `/referees` endpoint succeeded reliably against production throughout the session). The code-level guarantee was verified from the diff. See "What was not verified" below.

---

## Consequences

**This enables:**
- Tournament operators see real referee names instead of opaque system identifier codes.
- The game-info page also surfaces the stop-clock setting, removing one source of "Unknown" guesswork during game preparation.

**The refbox now depends on:**
- The portal's public `/referees` endpoint shape (an object with `tournamentReferee` + `referees.{dedicated, hybrid, timeOrScoreKeeper}` categories; `hybrid` always an array per the portal's TypeScript type definition).
- The portal continuing to expose `rosterName` and `user.username` on referee records. If the portal ever stops emitting these, the refbox falls through to the `-` placeholder — degraded but not broken.

**Privacy commitment:**
- The refbox will not display `user.name` from the portal API anywhere in the UI. This is a load-bearing design decision; future code touching the `/referees` parser should preserve it.

---

## What was removed during audit

### Team-ref fallback path (Group 2 cascade)

The original AI-authored code included a separate "team-referee fallback" display path: when a game had no individual referee assignments, the game-info page would substitute team names (e.g., `Refs: <team A>`, `Time/Score Keeper: <team B>`) sourced from a schedule-wide `referees_by_game_number` map. The operator decided in Step 4 review that this is the **wrong fallback** — missing referee data should show `-` (no data), not substitute team-level data which is semantically different. Removed:

- Four `uwh-common` types: `RefereesByGameNumber`, `GameReferees`, `TeamRefAssignment`, `TeamRefInfo` (commit `978464f`).
- `Schedule.referees_by_game_number` field (commit `978464f`).
- ~30-line fallback branch in `config_string` and `details_strings` (commit `0c1e779`).
- The `simple_game_number` lookup variable used only by the fallback branch (commit `0c1e779`).
- The unread `time_or_score_helper` slot on `GameReferees` (subsumed into the type deletion above).

The `team-ref-list` translation key remains defined in all 15 locale files because it was added by commits outside this audit window (`44ec5f1` and Unit 8's locale-rollout commits). Those keys are now unreferenced; cleanup is a Findings-Backlog candidate.

### Unused fields on `RefereeAssignment`

The original `RefereeAssignment` struct carried `identifier: String`, `team_id: Option<TeamId>`, and `comments: Option<String>` fields that no consumer ever read. Removed in commit `4a9291f`. Serde tolerates extra fields on input, so the portal can continue to send them without breaking parsing.

---

## Audit amendments (changes the audit made beyond pure deletion)

### B5.14 — `"-"` placeholder instead of localized "Unknown"

The original AI-authored code, after the Group 2 deletion above, would have shown localized "Unknown" (via `fl!("unknown")`) for empty referee slots. The audit amended this to render the literal `"-"` character instead (commit `97d867c`). Operator rationale:

- Language-neutral: `-` reads the same regardless of locale.
- Visually distinct from a real name: harder to mistake for an actual referee.
- Stronger pattern than the localized fallback — when Unit 2's Finding #1 (team-name "Unknown" handling that bypasses translation) eventually gets a fix branch, this `-` pattern is the reference implementation, not a localized fluent macro.

The stop-clock display's separate `fl!("unknown")` fallback was left unchanged — the operator confirmed stop-clock data is always Yes/No in practice, so that fallback rarely fires and amending it isn't material.

### B5.18 — Warn-level log on `/referees` failure

The original AI-authored code swallowed `/referees` fetch errors silently (`unwrap_or_default()`). The audit added a `warn!("Failed to fetch referee names: {e}")` log line before the fallback (commit `4f7427a`). UX behaviour is unchanged (silent degradation in the UI); the log gives operators a diagnostic breadcrumb if they ever investigate all-`-` referees.

### B5.19 — `hybrid` doc-comment correction

The doc-comment on `get_event_referee_name_map_from_referees` claimed `hybrid` could be either an object or an array. The implementation only handles arrays. Cross-checked the portal's TypeScript type at `js/@underwater-base/types/EventReferee.ts:155` (`hybrid: EventRefereeModelWithPhotos[] | null`) — the portal definitively sends arrays. The audit fixed the doc to match the code (commit `17cc49d`).

### ed94287 — Schedule wire-format compatibility (cross-branch dependency)

During the Task 6.4 walkthrough against production, refbox crashed deserialising the schedule because the portal evolved `standingsOrder` and `finalResultsOrder` from `usize[]` to `GroupReference[]`. The audit hand-applied a counterpart of `feat/schedule-processor/csv-display-order`'s commit `165a803` — adding a `GroupReference { name: String }` struct and changing four field types. This was a precondition for finishing the walkthrough; logged as a Process refinements observation because cross-branch dependencies are not a pattern the audit playbook anticipates.

---

## What was not verified

Three gaps in this audit's verification, all documented per Unit 2 refinement #1:

1. **S5.3 — Silent `/referees` failure** — the endpoint succeeded reliably against production throughout the walkthrough session. The silent-degradation path was not exercised at runtime; only verified by code reading. Tagged `@tested_inconclusive` in `refbox/tests/features/referee_names.feature`.
2. **URL-construction unit test for `/referees`** — skipped per Unit 3 refinement #5 (don't fake-test by mocking the whole HTTP client). The URL is built inline inside `get_event_referee_name_map_from_referees`'s async future with no isolated URL-builder function.
3. **`hybrid` category as object shape** — the portal's authoritative TypeScript type says array-only and the production data confirms; tested only for arrays. If a future portal change emits object-shape `hybrid`, the code returns an empty name-map for that category (silent partial failure → operator sees `-` for hybrid referees). The Unit 5 audit accepted this trade-off; future-proofing was out of scope.

---

## Audit reference

- **Audit branch:** `audit/refbox/referee-names`
- **Worktree:** `.worktrees/audit-unit-5-referee-names/`
- **Per-unit plan:** `docs/superpowers/plans/2026-05-13-audit-unit-5-referee-names.md`
- **Original commits (audit window, on master):** `353b476`, `8d4a667`, `996874a`, `d72d643`, `1bd4676`, `931d01d`
- **Audit-branch commits beyond master tip (`089c98d`):**
  - `978464f` — remove team-ref fallback types and `Schedule.referees_by_game_number`
  - `0c1e779` — remove team-ref fallback branch from `config_string` and `details_strings`
  - `4a9291f` — remove unused fields from `RefereeAssignment`
  - `97d867c` — render `-` instead of localized "Unknown" for empty referee slots
  - `4f7427a` — add warn-level log on `/referees` fetch failure
  - `17cc49d` — correct `hybrid` field type in doc-comment
  - `d8fa6db` — add serde roundtrip tests for `RefereeAssignment` and `TimingRule.last_2_min_stop_time`
  - `5615af4` — seed `referee_names.feature` with `@user_verified` scenarios
  - `ed94287` — `GroupReference` type for schedule order fields (cross-branch dep)
  - `81f2167` — Session 1 walkthrough results (`@tested_pass` tags + Gherkin amendments)
- **Test artifact:** `refbox/tests/features/referee_names.feature` — five scenarios, four `@tested_pass`, one `@tested_inconclusive`

---

## Verified by Unit 5 audit (2026-05-14)

- Walkthrough run from `.worktrees/audit-unit-5-referee-names/` against production portal (`https://api.uwhportal.com`), event `ca-2026-canadian-underwater-hockey-nationals`, Next Game: 1 (RFISH vs Cornwall).
- Per-role grid rendered with real names: Chief Ref Lewis Saleem, Water Ref 1 Ayman, Water Ref 2 Darryl Brambilla, Water Ref 3 John Kulsa, Timer `-`.
- Main view and Game Info page showed identical referee data.
- PII check: Lewis Saleem's `rosterName` ("Lewis Saleem") displayed; his `user.name` ("Lewis Adam Saleem") absent from the UI.
- `just check` clean on the audit branch, modulo the two pre-existing dep advisories tracked in Unit 3 Findings #4.
