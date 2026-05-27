# schedule-processor JSON Input Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `schedule-processor` read a portal-format `.json` schedule file in addition to the existing `.csv` path, then run all the existing validation, team-mapping, and upload steps unchanged.

**Architecture:** Add a small `From<(SendableSchedule, EventId)> for Schedule` impl in `uwh-common` (symmetric to the inverse impl that already lives in the same file). In `schedule-processor`, add `parse_json` that deserializes via the existing `Deserialize` on `SendableSchedule` and converts via the new `From` impl. Switch the main `FileDialog` to accept both extensions and branch on the file's extension.

**Tech Stack:** Rust 2024, MSRV 1.85; `serde` / `serde_json`; `indexmap`; `rfd` (file dialog); `time` (offsets).

**Branch:** `feat/schedule-processor/json-input` (off `origin/master` @ `ca3254f3`).

**Worktree:** `.worktrees/feat-schedule-processor-json-input/` — all paths in this plan are relative to that worktree root unless stated otherwise.

**Spec:** [`docs/superpowers/specs/2026-05-27-schedule-processor-json-input-design.md`](../specs/2026-05-27-schedule-processor-json-input-design.md)

---

## Task 1: Add `From<(SendableSchedule, EventId)>` impl in `uwh-common`

**Process:** Heavy (per `.claude/rules/plan-execution.md`). The downstream compile check is part of this task.

**Files:**
- Modify: `uwh-common/src/uwhportal/schedule.rs` (add ~11-line `From` impl right after the existing `impl From<Schedule> for SendableSchedule` at line 505; add one unit test inside the existing `#[cfg(test)] mod tests`).

- [ ] **Step 1: Write the failing round-trip test**

Locate the existing `#[cfg(test)] mod tests { ... }` in `uwh-common/src/uwhportal/schedule.rs` (around the end of the file — find it by grepping for `mod tests` or for the existing `test_serialize_referee_assignment_skips_display_name` test). Add this test inside that module:

```rust
    #[test]
    fn schedule_roundtrips_through_sendable_via_from_tuple() {
        use std::time::Duration;
        let event_id = EventId::from_partial("test-event");
        let original = Schedule {
            event_id: event_id.clone(),
            games: IndexMap::new(),
            non_game_entries: vec![],
            groups: vec![],
            timing_rules: vec![TimingRule {
                name: "RR".to_string(),
                team_timeout_count: 1,
                team_timeouts_counted_per_half: true,
                overtime_allowed: false,
                sudden_death_allowed: false,
                last_2_min_stop_time: false,
                half_play_duration: Duration::from_secs(600),
                half_time_duration: Duration::from_secs(120),
                team_timeout_duration: Duration::from_secs(60),
                ot_half_play_duration: Duration::from_secs(0),
                ot_half_time_duration: Duration::from_secs(0),
                pre_overtime_break: Duration::from_secs(0),
                pre_sudden_death_duration: Duration::from_secs(0),
                minimum_break: Duration::from_secs(180),
            }],
            standings_order: None,
            final_results_order: None,
        };
        let sendable: SendableSchedule = original.clone().into();
        let round_tripped: Schedule = (sendable, event_id).into();
        assert_eq!(original, round_tripped);
    }
```

Notes:
- `EventId::from_partial` is the existing constructor at [schedule.rs:678](../../../uwh-common/src/uwhportal/schedule.rs#L678).
- `IndexMap` is already in scope at the top of `schedule.rs`; if the test module needs its own `use`, copy the existing patterns in the test module.
- If `TimingRule`'s field names differ from what's shown above, look at the existing test around line 1290 for the canonical shape — copy it.

- [ ] **Step 2: Run the test and verify it fails (no `From` impl yet)**

```
cargo test -p uwh-common --lib schedule_roundtrips_through_sendable_via_from_tuple
```

Expected: compile error along the lines of *"the trait `From<(SendableSchedule, EventId)>` is not implemented for `Schedule`"*.

- [ ] **Step 3: Add the `From` impl**

In `uwh-common/src/uwhportal/schedule.rs`, immediately below the existing `impl From<Schedule> for SendableSchedule { ... }` block (lines 505–516), add:

```rust
impl From<(SendableSchedule, EventId)> for Schedule {
    fn from((sendable, event_id): (SendableSchedule, EventId)) -> Self {
        let games = sendable
            .games
            .into_iter()
            .map(|g| (g.number.clone(), g))
            .collect();
        Schedule {
            event_id,
            games,
            non_game_entries: sendable.non_game_entries,
            groups: sendable.groups,
            timing_rules: sendable.timing_rules,
            standings_order: sendable.standings_order,
            final_results_order: sendable.final_results_order,
        }
    }
}
```

- [ ] **Step 4: Run the test and verify it passes**

```
cargo test -p uwh-common --lib schedule_roundtrips_through_sendable_via_from_tuple
```

Expected: `1 passed; 0 failed`.

- [ ] **Step 5: Downstream compile check** (heavy-process requirement)

From the worktree root, run:

```
cargo check -p refbox
cargo check -p schedule-processor
cargo check -p overlay
cargo check -p led-panel-sim
cargo build -p uwh-common --no-default-features
```

Expected: all five complete without errors. (`--no-default-features` confirms `no_std` still builds.) If any of these fail, STOP — the failure means the addition was not purely additive; investigate before continuing.

- [ ] **Step 6: Run `just check` to confirm nothing else regressed**

```
just check
```

Expected: all green (fmt, lint, tests, audit).

- [ ] **Step 7: Commit**

```
git add uwh-common/src/uwhportal/schedule.rs
git commit -m "$(cat <<'EOF'
feat(uwh-common): add From<(SendableSchedule, EventId)> for Schedule

Symmetric to the existing From<Schedule> for SendableSchedule. Lets
schedule-processor's JSON-input path round-trip from the portal wire
format back to the internal Schedule type without a private mirror
struct. Purely additive: no new public types, no wire-format change,
no breaking changes. Covered by a round-trip unit test.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Add the Australian Nationals mock JSON fixture

**Process:** Lean.

**Files:**
- Create: `schedule-processor/Mock Schedules for testing/2026 Australian Nationals - portal export.json`

- [ ] **Step 1: Save the user-provided JSON to the mock-schedules directory**

The JSON content is the file the tournament organizer provided (the document attached to the original prompt of this session). Save it verbatim at the path above.

If implementing this from a fresh subagent without the original prompt: ask the user for the file. Do NOT invent placeholder JSON.

- [ ] **Step 2: Verify the file is well-formed JSON**

```
python3 -c "import json; json.load(open('schedule-processor/Mock Schedules for testing/2026 Australian Nationals - portal export.json'))" && echo "OK"
```

Expected output: `OK`.

- [ ] **Step 3: Commit**

```
git add "schedule-processor/Mock Schedules for testing/2026 Australian Nationals - portal export.json"
git commit -m "$(cat <<'EOF'
test(schedule-processor): add 2026 Australian Nationals portal-export fixture

Real-world JSON schedule supplied by the tournament organizer, used by
the upcoming JSON-input integration test. 71 games across 4 divisions
(A/B/C Grade Open + Women), pod-based finals with cross-pod seedings,
populated standingsOrder and finalResultsOrder.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Create `json_loader.rs` with `parse_json` and unit tests

**Process:** Lean.

**Files:**
- Create: `schedule-processor/src/json_loader.rs`
- Modify: `schedule-processor/src/main.rs` (add one line: `mod json_loader;` near the top, alongside the existing `mod csv_parser;` and `mod schedule_checks;`).

- [ ] **Step 1: Create `schedule-processor/src/json_loader.rs` with the three tests + a `todo!()` stub**

```rust
use time::UtcOffset;
use uwh_common::uwhportal::schedule::*;

pub fn parse_json(
    json: &str,
    _offset: UtcOffset,
    event_id: EventId,
) -> Result<Schedule, Box<dyn std::error::Error>> {
    todo!("implement parse_json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::UtcOffset;

    const HAPPY_PATH_JSON: &str = r#"{
        "games": [
            {
                "number": "1",
                "court": "1",
                "startTime": "2026-06-26T23:30:00Z",
                "timingRule": "RR",
                "light": {"pendingAssignmentName": "Team A"},
                "dark": {"pendingAssignmentName": "Team B"}
            }
        ],
        "nonGameEntries": [],
        "groups": [
            {
                "name": "Test Group",
                "shortName": "TG",
                "type": "Division",
                "gameNumbers": ["1"],
                "standingsCalculation": {"type": "Standard"},
                "finalResultsCalculation": null
            }
        ],
        "timingRules": [
            {
                "name": "RR",
                "teamTimeoutCount": 1,
                "teamTimeoutsCountedPerHalf": true,
                "overtimeAllowed": false,
                "suddenDeathAllowed": false,
                "last2minStopTime": false,
                "halfPlayDuration": 600,
                "halfTimeDuration": 120,
                "teamTimeoutDuration": 60,
                "overtimeHalfPlayDuration": 0,
                "overtimeHalfTimeDuration": 0,
                "preOvertimeBreak": 0,
                "preSuddenDeathDuration": 0,
                "minimumBreak": 180
            }
        ],
        "standingsOrder": [{"name": "Test Group"}],
        "finalResultsOrder": null
    }"#;

    #[test]
    fn parses_happy_path_json() {
        let event_id = EventId::from_partial("test-event");
        let schedule = parse_json(HAPPY_PATH_JSON, UtcOffset::UTC, event_id).unwrap();
        assert_eq!(schedule.games.len(), 1);
        assert_eq!(schedule.groups.len(), 1);
        assert_eq!(schedule.timing_rules.len(), 1);
        assert_eq!(schedule.standings_order.as_ref().unwrap().len(), 1);
        assert!(schedule.final_results_order.is_none());
    }

    #[test]
    fn injects_event_id() {
        let event_id = EventId::from_partial("my-event");
        let schedule = parse_json(HAPPY_PATH_JSON, UtcOffset::UTC, event_id.clone()).unwrap();
        assert_eq!(schedule.event_id, event_id);
    }

    #[test]
    fn missing_required_field_surfaces_serde_error() {
        // games omitted
        let bad_json = r#"{
            "nonGameEntries": [],
            "groups": [],
            "timingRules": []
        }"#;
        let event_id = EventId::from_partial("test-event");
        let err = parse_json(bad_json, UtcOffset::UTC, event_id)
            .expect_err("expected parse to fail with missing field");
        assert!(
            err.to_string().contains("games"),
            "expected error to mention `games`, got: {err}"
        );
    }
}
```

Notes:
- The `startTime`, `timingRule`, `light`, `dark`, `number`, and `court` keys / field names on `Game` come from the existing `Game` struct's `serde` attributes. If field names in this fixture turn out to be wrong when you compile, look at the `Game` definition in `uwh-common/src/uwhportal/schedule.rs` (search for `pub struct Game`) and adjust the JSON literal to match.
- `EventId` and the rest are re-exported by `uwh-common::uwhportal::schedule::*`.
- If the test JSON triggers a serde rename collision (e.g. `last2minStopTime` vs. `last_2_min_stop_time`), trust the existing struct attributes and update the JSON literal — do not change the structs.

- [ ] **Step 2: Wire the module into `main.rs`**

In `schedule-processor/src/main.rs`, near the existing `mod csv_parser;` and `mod schedule_checks;` declarations (around lines 19–22), add:

```rust
mod json_loader;
use json_loader::parse_json;
```

(The `use` import is optional but mirrors the `use csv_parser::parse_csv;` line that's already there; including it keeps the call site in main.rs identical in shape across both parsers.)

- [ ] **Step 3: Run the tests and verify they fail with `todo!()` (or compile error if `Game`'s fields don't match)**

```
cargo test -p schedule-processor --bin schedule_processor -- json_loader
```

Expected: each of `parses_happy_path_json` and `injects_event_id` fails with a `not yet implemented` panic from `todo!()`. `missing_required_field_surfaces_serde_error` also fails with `todo!()` (because we hit `todo!()` *before* serde parsing). All three failures are expected at this stage. If you instead see a compile error about JSON field names, fix the JSON literal to match the actual `Game` struct, then re-run.

- [ ] **Step 4: Implement `parse_json`**

Replace the `todo!()` body with:

```rust
pub fn parse_json(
    json: &str,
    _offset: UtcOffset,
    event_id: EventId,
) -> Result<Schedule, Box<dyn std::error::Error>> {
    let sendable: SendableSchedule = serde_json::from_str(json)?;
    Ok((sendable, event_id).into())
}
```

- [ ] **Step 5: Run the tests and verify they pass**

```
cargo test -p schedule-processor --bin schedule_processor -- json_loader
```

Expected: `3 passed; 0 failed`.

- [ ] **Step 6: Commit**

```
git add schedule-processor/src/json_loader.rs schedule-processor/src/main.rs
git commit -m "$(cat <<'EOF'
feat(schedule-processor): add parse_json for portal-format JSON input

Adds schedule-processor/src/json_loader.rs with parse_json, symmetric
to the existing parse_csv. Delegates structural validation to serde
via SendableSchedule's existing Deserialize derive, and constructs
the internal Schedule via the new From<(SendableSchedule, EventId)>
in uwh-common. Three unit tests cover the happy path, event_id
injection, and a missing-required-field error case.

This task wires the module into main.rs but does not yet change the
file-dialog flow; that comes in the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Switch `main.rs` file dialog to accept `.csv` and `.json`, branch on extension

**Process:** Lean.

**Files:**
- Modify: `schedule-processor/src/main.rs` (the file-selection block at lines 77–88, and the parse block at lines 149–164).

- [ ] **Step 1: Change the file dialog filter and prompt text**

In `schedule-processor/src/main.rs`, replace:

```rust
info!("Please select a CSV schedule to process in the file dialog.");
let csv_path = FileDialog::new()
    .add_filter("CSV files", &["csv"])
    .set_title("Select Schedule CSV File")
    .pick_file();

let csv_path = if let Some(path) = csv_path {
    path
} else {
    error!("No file selected. Exiting.");
    return Err("No file selected".into());
};
```

with:

```rust
info!("Please select a schedule file (.csv or .json) to process in the file dialog.");
let schedule_path = FileDialog::new()
    .add_filter("Schedule files", &["csv", "json"])
    .set_title("Select Schedule File")
    .pick_file();

let schedule_path = if let Some(path) = schedule_path {
    path
} else {
    error!("No file selected. Exiting.");
    return Err("No file selected".into());
};
```

- [ ] **Step 2: Replace the CSV-only parse block with extension-based branching**

A few lines below (around line 149 in the pre-change file), replace:

```rust
info!("Reading csv file: {}", csv_path.display());
let csv = std::fs::read_to_string(&csv_path)?;
let schedule = match parse_csv(&csv, offset, event.id.clone()) {
    Ok(schedule) => schedule,
    Err(e) => {
        error!("Failed to parse CSV file: {e}");
        Text::new("Press any key close the app")
            .with_placeholder("Press Enter to proceed")
            .prompt()
            .unwrap_or_else(|_| {
                error!("Failed to proceed. Exiting.");
                std::process::exit(1);
            });
        return Err(e);
    }
};
```

with:

```rust
info!("Reading schedule file: {}", schedule_path.display());
let contents = std::fs::read_to_string(&schedule_path)?;
let ext = schedule_path
    .extension()
    .and_then(|e| e.to_str())
    .map(|e| e.to_ascii_lowercase());
let parse_result = match ext.as_deref() {
    Some("csv") => parse_csv(&contents, offset, event.id.clone()),
    Some("json") => parse_json(&contents, offset, event.id.clone()),
    _ => Err("Unsupported file type (must be .csv or .json)".into()),
};
let schedule = match parse_result {
    Ok(schedule) => schedule,
    Err(e) => {
        error!("Failed to parse schedule file: {e}");
        Text::new("Press any key close the app")
            .with_placeholder("Press Enter to proceed")
            .prompt()
            .unwrap_or_else(|_| {
                error!("Failed to proceed. Exiting.");
                std::process::exit(1);
            });
        return Err(e);
    }
};
```

- [ ] **Step 3: Verify the crate still compiles**

```
cargo build -p schedule-processor
```

Expected: clean build, no warnings.

- [ ] **Step 4: Verify clippy is clean**

```
cargo clippy -p schedule-processor --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 5: Commit**

```
git add schedule-processor/src/main.rs
git commit -m "$(cat <<'EOF'
feat(schedule-processor): accept .json schedule files in the file dialog

Updates the FileDialog filter from CSV-only to ["csv", "json"] and
branches on the picked file's extension to call either parse_csv or
parse_json. Unsupported extensions exit with a friendly error.
csv_path is renamed to schedule_path; the existing "Failed to parse /
Press Enter to proceed" prompt now covers both branches.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Integration test against the real-world Australian Nationals fixture

**Process:** Lean.

**Files:**
- Create: `schedule-processor/tests/json_real_world.rs` (this creates the `tests/` directory; currently absent in `schedule-processor`).

- [ ] **Step 1: Write the integration test**

Create `schedule-processor/tests/json_real_world.rs` with:

```rust
//! Integration test: load the supplied real-world Australian Nationals
//! schedule JSON and run the full validation pipeline against it.

use time::UtcOffset;
use uwh_common::uwhportal::schedule::EventId;

#[path = "../src/json_loader.rs"]
mod json_loader;

#[path = "../src/schedule_checks.rs"]
mod schedule_checks;

const FIXTURE: &str = include_str!(
    "../Mock Schedules for testing/2026 Australian Nationals - portal export.json"
);

#[test]
fn australian_nationals_json_parses_and_checks_pass() {
    let event_id = EventId::from_partial("test-australian-nationals");
    let schedule = json_loader::parse_json(FIXTURE, UtcOffset::UTC, event_id)
        .expect("Australian Nationals JSON should parse");
    schedule_checks::run_schedule_checks(&schedule)
        .expect("Australian Nationals JSON should pass all schedule checks");
}
```

Notes on the `#[path = ...]` includes:
- Integration tests under `tests/` are compiled as separate crates, so they can't directly import bin-private modules like `json_loader` and `schedule_checks` (the bin doesn't expose them as a library). The `#[path]` attribute pulls those source files into the test crate as if they lived next to it.
- This is a common Rust idiom for testing bin-only modules and matches how the schedule-processor is structured today (a single-bin crate with no `lib.rs`).
- If this turns out to cause compile issues (e.g. nested `mod` statements inside the included files conflicting), the alternative is to extract `json_loader` into a small `lib.rs` for the crate. Do NOT do that as part of this task — flag it and we'll decide whether to restructure separately.

- [ ] **Step 2: Run the integration test**

```
cargo test -p schedule-processor --test json_real_world
```

Expected: `1 passed; 0 failed`. The `run_schedule_checks` pass confirms 12 separate validators (unique game numbers, group/standings consistency, timing-rule integrity, court overlap, same-team-in-game, group naming uniqueness, standings teams match, final results, game cross-references) all green against the real schedule.

If the test fails on a check (e.g. `check_game_overlap` or `check_group_standings`), do NOT modify the validation code. Instead:
- If the failure is about something genuinely wrong in the supplied JSON (e.g. two games overlap on one court), report that back to the user and stop — they may need to correct the schedule.
- If the failure is something we mis-handle in `parse_json` (unlikely given the round-trip test in Task 1 covers field passing), debug `parse_json` first.

- [ ] **Step 3: Commit**

```
git add schedule-processor/tests/json_real_world.rs
git commit -m "$(cat <<'EOF'
test(schedule-processor): integration test for real-world JSON input

Loads the 2026 Australian Nationals portal-format JSON via parse_json
and runs the full run_schedule_checks pipeline on the result. This is
the strongest evidence that meaningful validation runs on JSON-loaded
schedules: every existing schedule check is exercised against a real
71-game tournament with pod-based finals, cross-pod seedings, and
populated standings/final-results order.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Final verification (no commit)

**Process:** Lean.

- [ ] **Step 1: Run `just check` from the worktree root**

```
just check
```

Expected: every gate green (fmt, clippy with `-D warnings`, tests across the workspace, audit). This is the same gate CI runs.

- [ ] **Step 2: Optional manual smoke test** (only if the user wants it)

```
cargo run -p schedule-processor
```

In the file dialog, pick `schedule-processor/Mock Schedules for testing/2026 Australian Nationals - portal export.json`. Select Production / Underwater Hockey, then pick any event (the schedule itself doesn't need to match the event for parsing — team mapping is the step that ties them together). Expect:

- The schedule summary prints showing 71 games, 12 groups (4 RR + 4 finals + 4 pods), 2 timing rules (RR, FINALS), 2 courts.
- `run_schedule_checks` runs and reports no errors.
- The interactive menu appears with "Map Teams" available.

This step is for the user to validate; the agentic worker should not run `cargo run` on its own.

- [ ] **Step 3: Hand off to the user for review and PR**

At this point the feature is complete. Per `.claude/rules/communication.md`, ask the user for approval before pushing the branch or opening a PR.

---

## Acceptance criteria cross-check (from spec)

| Spec criterion | Implemented by |
|---|---|
| File dialog offers both `.csv` and `.json` | Task 4, Step 1 |
| Australian-tournament JSON parses successfully | Task 3 + Task 5 |
| 71 games / 12 groups / 2 timing rules / 2 courts shown in summary | Task 5 integration test (and Task 6 manual smoke) |
| `run_schedule_checks` reports no errors | Task 5 integration test |
| Team mapping works | Unchanged behaviour downstream of `parse_json`; verifiable in Task 6 manual smoke |
| Save Schedule round-trips | Unchanged behaviour downstream of `parse_json`; the From-impl round-trip test in Task 1 covers structural preservation |
| Existing CSV path still works | Task 4 preserves it; `just check` in Task 6 exercises any existing CSV-side tests |
| Downstream crates still compile | Task 1, Step 5 |
| `just check` passes | Task 6, Step 1 |

---

## Notes for the executor

- **Worktree:** Always operate from inside `.worktrees/feat-schedule-processor-json-input/`. Per memory `feedback_cd_worktree_before_cargo`, the Bash tool doesn't preserve cwd between calls — use `cd .worktrees/feat-schedule-processor-json-input/ && <cmd>` or absolute paths.
- **No unrelated edits.** Per `.claude/rules/scope.md`, do not opportunistically tidy nearby code. If you spot something off, note it and propose a separate branch.
- **No new dependencies.** `serde_json` is already transitively in scope via `uwh-common`. If you find yourself reaching for a new crate, stop — that's a sign you've gone off-plan.
- **MSRV 1.85, Edition 2024.** Don't reach for newer-than-1.85 stdlib APIs.
- **No `unwrap()` or `expect()` in non-test production code without a justifying comment.** Tests are exempt.
- **Translations not affected.** No `fl!()` keys added or changed; no `.ftl` files touched.

If anything in this plan turns out to be wrong (e.g. a `Game` field name differs, the `#[path]` include trick conflicts with something), append a note under a **Deviations** heading at the bottom of this file as you implement, per the lean-process rule in `.claude/rules/plan-execution.md` — do NOT create a standalone deviations commit.
