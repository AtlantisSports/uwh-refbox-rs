# Single-period flag and no zero-length durations — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give single-period games an explicit `singlePeriod` flag instead of a zero half-time, and make any zero-length duration impossible to author.

**Architecture:** `uwh-common` gains the contract field and stops inferring the flag from a zero — in **both** places it currently does so. `schedule-processor` gains one new check, in the place both authoring paths already converge. `refbox` loses `normalize_degenerate_overtime`, which only ever ran on Portal-sourced configs and is redundant once zeros cannot be authored.

**Tech Stack:** Rust 2024, MSRV 1.85, serde/serde_json, `just` for validation.

**Spec:** `docs/superpowers/specs/2026-09-11-single-period-flag-and-no-zero-durations-design.md`

## Global Constraints

- **MSRV 1.85, Edition 2024.** No APIs newer than 1.85.
- **Clippy `-D warnings`**, zero warnings. Run `just check` before every commit.
- **No `unwrap()`/`expect()` in non-test code** without a comment proving it cannot panic.
- **`uwh-common` must stay `no_std`-compatible.** Add no dependencies to it.
- **Branch:** `feat/uwh-common/single-period-flag`. **Do not push and do not open a PR** — this is held unmerged until the Portal ships `singlePeriod`. Pushing needs Eric's approval.
- **Contract field name is `singlePeriod`** (camelCase on the wire, `single_period` in Rust). Not yet ratified by the Portal's core team; if it changes, it changes in one `#[serde(rename)]` and the fixtures.
- **Commit format:** `type(scope): description`, lowercase, imperative, no trailing period.
- Heavy process per `.claude/rules/plan-execution.md` — this is a wire-format change.

---

### Task 1: Add `singlePeriod` to the contract and stop inferring it

**Files:**
- Modify: `uwh-common/src/uwhportal/schedule.rs` (struct ~`:241-277`, conversion `:323` and `:341`)
- Test: same file, existing `#[cfg(test)]` module

**Interfaces:**
- Consumes: nothing.
- Produces: `TimingRule.single_period: bool`, serialised as `"singlePeriod"`, `#[serde(default)]` so a missing field means `false`. `GameConfig.single_half` is now set from it rather than derived.

- [ ] **Step 1: Write the failing tests**

Add to the existing test module in `uwh-common/src/uwhportal/schedule.rs`:

```rust
#[test]
fn single_period_flag_drives_single_half_not_the_zero() {
    // The flag is what decides, and a real positive half-time is simply unused.
    let json = r#"{"name":"RR","teamTimeoutCount":1,"teamTimeoutsCountedPerHalf":false,
"overtimeAllowed":false,"suddenDeathAllowed":false,"singlePeriod":true,
"halfPlayDuration":600,"halfTimeDuration":180,"teamTimeoutDuration":60,
"overtimeHalfPlayDuration":300,"overtimeHalfTimeDuration":60,"preOvertimeBreak":180,
"preSuddenDeathDuration":60,"minimumBreak":240}"#;
    let rule: TimingRule = serde_json::from_str(json).unwrap();
    assert!(rule.single_period);
    let config: GameConfig = rule.into();
    assert!(config.single_half, "the flag must decide, not the half-time value");
}

#[test]
fn a_missing_single_period_field_means_two_halves() {
    let json = r#"{"name":"RR","teamTimeoutCount":1,"teamTimeoutsCountedPerHalf":false,
"overtimeAllowed":false,"suddenDeathAllowed":false,
"halfPlayDuration":600,"halfTimeDuration":180,"teamTimeoutDuration":60,
"overtimeHalfPlayDuration":300,"overtimeHalfTimeDuration":60,"preOvertimeBreak":180,
"preSuddenDeathDuration":60,"minimumBreak":240}"#;
    let rule: TimingRule = serde_json::from_str(json).unwrap();
    assert!(!rule.single_period);
    let config: GameConfig = rule.into();
    assert!(!config.single_half);
}

#[test]
fn game_block_regulation_follows_the_flag_not_the_zero() {
    // The second magic-zero read. A single-period rule with a real unused
    // half-time must derive one half of play, not two halves plus the break.
    let json = r#"{"name":"RR","teamTimeoutCount":1,"teamTimeoutsCountedPerHalf":false,
"overtimeAllowed":false,"suddenDeathAllowed":false,"singlePeriod":true,
"halfPlayDuration":600,"halfTimeDuration":180,"teamTimeoutDuration":60,
"overtimeHalfPlayDuration":300,"overtimeHalfTimeDuration":60,"preOvertimeBreak":180,
"preSuddenDeathDuration":60,"minimumBreak":240}"#;
    let rule: TimingRule = serde_json::from_str(json).unwrap();
    let config: GameConfig = rule.into();
    // regulation = half_play (600) + minimum_break (240) = 840. NOT 600*2+180+240 = 1620.
    assert_eq!(config.game_block, Duration::from_secs(840));
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p uwh-common single_period -- --nocapture` and
`cargo test -p uwh-common game_block_regulation -- --nocapture`

Expected: compile error — `TimingRule` has no field `single_period`. That is a legitimate failure for the first run.

- [ ] **Step 3: Add the field to `TimingRule`**

In the struct, immediately after `sudden_death_allowed` (keeping the booleans together):

```rust
    /// One playing period and no half-time break. Replaces the former
    /// convention where a zero `halfTimeDuration` carried this meaning:
    /// a mode belongs in a flag, not in a magic value. `half_time_duration`
    /// still carries a real, unused number when this is set — exactly as
    /// `ot_half_play_duration` does when `overtime_allowed` is false.
    ///
    /// `default` so a rule from a Portal that predates the field reads as
    /// two halves, which is correct: no current event is single-period.
    #[serde(default, rename = "singlePeriod")]
    pub single_period: bool,
```

- [ ] **Step 4: Destructure it and use it in both places**

Add `single_period,` to the `let TimingRule { .. } = self;` destructuring, then:

At `:323`, replace `single_half: half_time_duration == Duration::ZERO,` with:

```rust
            single_half: single_period,
```

At `:341`, inside the `game_block` fallback, replace
`let regulation = if half_time_duration == Duration::ZERO {` with:

```rust
                let regulation = if single_period {
```

- [ ] **Step 5: Run the tests and confirm they pass**

Run: `cargo test -p uwh-common`
Expected: PASS, including the three new tests and every pre-existing one.

**One pre-existing test will fail, and it is named:** `test_timing_rule_game_block_single_half_derived` (same file, ~`:1077`). Its comment literally says *"halfTimeDuration == 0 signals single-half"* — it encodes the convention being replaced. Update it to set `"singlePeriod":true` with `"halfTimeDuration":180`, and keep its assertion (`game_block == 720`) unchanged: regulation is still `half_play` alone, so the expected value does not move. **Do not weaken the assertion to make it pass** — if 720 no longer holds, the conversion is wrong, not the test.

Check `test_timing_rule_game_block_uses_schedule_minimum_break` (~`:1088`) still passes untouched: it is a two-half rule and must be unaffected.

- [ ] **Step 6: Verify `no_std` still holds**

Run: `cargo check -p uwh-common --no-default-features`
Expected: success. (No dependency was added, so this should be unaffected — confirm rather than assume.)

- [ ] **Step 7: Commit**

```bash
git add uwh-common/src/uwhportal/schedule.rs
git commit -m "feat(uwh-common): add an explicit singlePeriod timing-rule flag"
```

---

### Task 2: Teach the schedule builder the new field

**Files:**
- Modify: `schedule-processor/src/csv_parser.rs` (`TIMING_RULE_FIELDS` at `:609`)
- Modify: `schedule-processor/tests/fixtures/timing-rule-fields.csv`
- Test: `schedule-processor/src/csv_parser.rs`, existing `#[cfg(test)]` module

**Interfaces:**
- Consumes: `TimingRule.single_period` from Task 1.
- Produces: `singlePeriod` accepted in the spreadsheet's "Timing Rule Field" column. `TIMING_RULE_FIELDS` becomes `[&str; 15]`.

- [ ] **Step 1: Run the existing contract test and watch it fail**

Run: `cargo test -p schedule-processor timing_rule_field_names_match_the_type`

Expected: FAIL. This test asks `TimingRule` what it serialises and compares against `TIMING_RULE_FIELDS`, so Task 1 has already broken it. **This is the point of the test** — confirm it fails for the right reason (`singlePeriod` present in the type, absent from the list) before fixing it.

- [ ] **Step 2: Add the field to the accepted list**

In `csv_parser.rs`, change the array length and add the entry after `"suddenDeathAllowed"`:

```rust
const TIMING_RULE_FIELDS: [&str; 15] = [
    "teamTimeoutCount",
    "teamTimeoutsCountedPerHalf",
    "overtimeAllowed",
    "suddenDeathAllowed",
    "singlePeriod",
    "last2minStopTime",
    "halfPlayDuration",
    "halfTimeDuration",
    "teamTimeoutDuration",
    "overtimeHalfPlayDuration",
    "overtimeHalfTimeDuration",
    "preOvertimeBreak",
    "preSuddenDeathDuration",
    "minimumBreak",
    "gameBlock",
];
```

- [ ] **Step 3: Run the contract test and confirm it passes**

Run: `cargo test -p schedule-processor timing_rule_field_names_match_the_type`
Expected: PASS.

- [ ] **Step 4: Add the field to the fixture spreadsheet**

In `schedule-processor/tests/fixtures/timing-rule-fields.csv`, add one row alongside the other timing-rule rows (the leading commas place it in the timing-rule columns; copy the exact comma count from the row above it):

```
,,,,,,,,,,,,,,,,,,,RR,singlePeriod,FALSE
```

- [ ] **Step 5: Run the crate's tests**

Run: `cargo test -p schedule-processor`
Expected: PASS.

- [ ] **Step 6: Note — the JSON authoring path needs no code**

`json_loader.rs` deserialises a whole portal-shaped document straight into `Schedule`, so `TimingRule` picks the field up from Task 1 with no loader change. Only its fixture needs updating, which is Task 4. Nothing to do here; this step exists so an executor does not go looking for a JSON parser change that does not exist.

- [ ] **Step 7: Commit**

```bash
git add schedule-processor/src/csv_parser.rs schedule-processor/tests/fixtures/timing-rule-fields.csv
git commit -m "feat(schedule-processor): accept singlePeriod in the timing-rule column"
```

---

### Task 3: Refuse every zero-length duration

**Files:**
- Modify: `schedule-processor/src/schedule_checks.rs` (add a check, register it in `run_schedule_checks` at `:9`)
- Test: `schedule-processor/src/schedule_checks.rs`, `#[cfg(test)]` module

**Interfaces:**
- Consumes: `Schedule.timing_rules: Vec<TimingRule>`.
- Produces: `fn check_no_zero_durations(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>>`, called from `run_schedule_checks` — so it covers the CSV and JSON paths at once (`main.rs:267-268` parses either, `main.rs:319` runs the checks).

- [ ] **Step 1: Write the failing tests**

**`schedule_checks.rs` has no test module at all** — create one at the end of the file. The file already has `use uwh_common::uwhportal::schedule::*;` at the top, so `use super::*;` brings `Schedule`, `TimingRule` and `EventId` into scope.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn schedule_with_rules(timing_rules: Vec<TimingRule>) -> Schedule {
        Schedule {
            event_id: EventId::from_partial("test-event"),
            games: Default::default(), // GameList = IndexMap<GameNumber, Game>
            non_game_entries: vec![],
            groups: vec![],
            timing_rules,
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }
```

Then, inside that module, build the rule with a helper so each test changes exactly one field:

```rust
fn a_valid_rule() -> TimingRule {
    TimingRule {
        name: "RR".to_string(),
        team_timeout_count: 1,
        team_timeouts_counted_per_half: false,
        overtime_allowed: false,
        sudden_death_allowed: false,
        single_period: false,
        last_2_min_stop_time: false,
        half_play_duration: Duration::from_secs(720),
        half_time_duration: Duration::from_secs(180),
        team_timeout_duration: Duration::from_secs(60),
        ot_half_play_duration: Duration::from_secs(300),
        ot_half_time_duration: Duration::from_secs(60),
        pre_overtime_break: Duration::from_secs(180),
        pre_sudden_death_duration: Duration::from_secs(60),
        minimum_break: Duration::from_secs(240),
        game_block: Some(Duration::from_secs(1920)),
    }
}

#[test]
fn a_rule_with_no_zero_durations_passes() {
    let schedule = schedule_with_rules(vec![a_valid_rule()]);
    assert!(check_no_zero_durations(&schedule).is_ok());
}

#[test]
fn every_duration_field_is_rejected_at_zero() {
    // Exhaustive on purpose: a new duration on TimingRule that nobody adds
    // here would otherwise be silently unguarded.
    let setters: Vec<(&str, fn(&mut TimingRule))> = vec![
        ("halfPlayDuration", |r| r.half_play_duration = Duration::ZERO),
        ("halfTimeDuration", |r| r.half_time_duration = Duration::ZERO),
        ("teamTimeoutDuration", |r| r.team_timeout_duration = Duration::ZERO),
        ("overtimeHalfPlayDuration", |r| r.ot_half_play_duration = Duration::ZERO),
        ("overtimeHalfTimeDuration", |r| r.ot_half_time_duration = Duration::ZERO),
        ("preOvertimeBreak", |r| r.pre_overtime_break = Duration::ZERO),
        ("preSuddenDeathDuration", |r| r.pre_sudden_death_duration = Duration::ZERO),
        ("minimumBreak", |r| r.minimum_break = Duration::ZERO),
        ("gameBlock", |r| r.game_block = Some(Duration::ZERO)),
    ];
    for (field, set) in setters {
        let mut rule = a_valid_rule();
        set(&mut rule);
        let schedule = schedule_with_rules(vec![rule]);
        let err = check_no_zero_durations(&schedule)
            .expect_err(&format!("a zero {field} must be refused"));
        let msg = err.to_string();
        assert!(msg.contains(field), "the message must name the setting, got: {msg}");
        assert!(msg.contains("RR"), "the message must name the rule, got: {msg}");
    }
}

#[test]
fn an_absent_game_block_is_not_a_zero() {
    // `gameBlock` is optional. Absent means "derive it", which is legal;
    // only a present zero is an error.
    let mut rule = a_valid_rule();
    rule.game_block = None;
    let schedule = schedule_with_rules(vec![rule]);
    assert!(check_no_zero_durations(&schedule).is_ok());
}

#[test]
fn a_single_period_rule_still_needs_a_real_half_time() {
    // The whole point of the flag: single-period does NOT license a zero.
    let mut rule = a_valid_rule();
    rule.single_period = true;
    rule.half_time_duration = Duration::ZERO;
    let schedule = schedule_with_rules(vec![rule]);
    assert!(check_no_zero_durations(&schedule).is_err());
}
```

Close the `mod tests` block after the last test.

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p schedule-processor check_no_zero -- --nocapture` and `cargo test -p schedule-processor zero_durations -- --nocapture`
Expected: compile error — `check_no_zero_durations` not found.

- [ ] **Step 3: Write the check**

```rust
/// Refuse any zero-length duration. Every duration carries a real number and
/// the on/off flag alone decides whether it is used — a zero must never stand
/// in for "this feature is off", because a mode hidden in a magic value is
/// both unreadable and, for the playing durations, a source of crashes.
///
/// `gameBlock` is checked only when present: absent means "derive it", which
/// is legal. `teamTimeoutCount` is a count, not a duration, and is not checked.
fn check_no_zero_durations(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    for rule in &schedule.timing_rules {
        let mut zeroed: Vec<&str> = Vec::new();
        let fields: [(&str, Duration); 8] = [
            ("halfPlayDuration", rule.half_play_duration),
            ("halfTimeDuration", rule.half_time_duration),
            ("teamTimeoutDuration", rule.team_timeout_duration),
            ("overtimeHalfPlayDuration", rule.ot_half_play_duration),
            ("overtimeHalfTimeDuration", rule.ot_half_time_duration),
            ("preOvertimeBreak", rule.pre_overtime_break),
            ("preSuddenDeathDuration", rule.pre_sudden_death_duration),
            ("minimumBreak", rule.minimum_break),
        ];
        for (name, value) in fields {
            if value.is_zero() {
                zeroed.push(name);
            }
        }
        if rule.game_block == Some(Duration::ZERO) {
            zeroed.push("gameBlock");
        }
        if !zeroed.is_empty() {
            return Err(format!(
                "Timing rule '{}' sets {} to zero. No duration may be zero — give it a real \
                 length, and use the matching on/off setting to say whether it is used. For a \
                 single-period game set 'singlePeriod' to TRUE and leave 'halfTimeDuration' at a \
                 normal length.",
                rule.name,
                zeroed.join(", ")
            )
            .into());
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Register it**

In `run_schedule_checks`, after `check_unique_timing_rule_names(schedule)?;`:

```rust
    check_no_zero_durations(schedule)?;
```

- [ ] **Step 5: Run the tests and confirm they pass**

Run: `cargo test -p schedule-processor`
Expected: PASS. Fixtures that still carry zeros will fail here — that is Task 4.

- [ ] **Step 6: Commit**

```bash
git add schedule-processor/src/schedule_checks.rs
git commit -m "feat(schedule-processor): refuse a zero-length duration"
```

---

### Task 4: Move the fixtures off the old convention

**Files:**
- Modify: `schedule-processor/src/json_loader.rs` (`HAPPY_PATH_JSON`, the timing rule around `:44-60`)
- Modify: `schedule-processor/tests/fixtures/portal-schedule-with-finals.json` (`:1508-1535`)
- Already handled in Task 1: the round-trip JSON in `uwh-common/src/uwhportal/schedule.rs` (`:1080`, `:1092`). Confirm it here rather than changing it twice.

**Interfaces:**
- Consumes: Tasks 1 and 3.
- Produces: nothing new — fixtures that express the same games under the new convention.

- [ ] **Step 1: Run the suite and list what fails**

Run: `cargo test -p schedule-processor 2>&1 | tail -40`
Expected: failures in the fixtures that still encode zeros. Note each one before changing anything.

- [ ] **Step 2: Fix `HAPPY_PATH_JSON`**

It is a two-half game with overtime off, so it needs real numbers rather than the zero idiom. Replace the four zeroed lines with:

```json
                "overtimeHalfPlayDuration": 300,
                "overtimeHalfTimeDuration": 60,
                "preOvertimeBreak": 180,
                "preSuddenDeathDuration": 60,
```

Leave `"overtimeAllowed": false` and `"suddenDeathAllowed": false` exactly as they are — the flags are what say those are unused, which is the whole point.

- [ ] **Step 3: Fix `portal-schedule-with-finals.json`**

Two rules. For **RR** (`overtimeAllowed: false`, `suddenDeathAllowed: false`), replace its four zeroed durations with the same positive values as Step 2.

For **FINALS** (`overtimeAllowed: true`, `suddenDeathAllowed: true`), the zeros are the shape `normalize_degenerate_overtime` exists to catch. Under rule 2 the flags stand on their own, so give it real overtime lengths:

```json
      "overtimeHalfPlayDuration": 300,
      "overtimeHalfTimeDuration": 60,
      "preOvertimeBreak": 180,
```

Leave `"preSuddenDeathDuration": 60` as it is — already non-zero.

- [ ] **Step 4: Run the suite and confirm it passes**

Run: `cargo test -p schedule-processor`
Expected: PASS.

- [ ] **Step 5: Confirm no zeroed duration remains anywhere**

Run:
```bash
grep -rn '"\(half\|overtime\|preOvertime\|preSuddenDeath\|minimum\|team\|game\)[A-Za-z]*\(Duration\|Break\|Block\)": *0' \
  schedule-processor uwh-common --include=*.json --include=*.rs
```
Expected: no output. If anything is listed, fix it the same way.

- [ ] **Step 6: Commit**

```bash
git add schedule-processor/src/json_loader.rs schedule-processor/tests/fixtures/portal-schedule-with-finals.json
git commit -m "fix(schedule-processor): move the fixtures off the zero-duration idiom"
```

---

### Task 5: Delete the overtime workaround

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` (remove the fn at `:1323`, its call sites at `:1379` and `:1539`, and its tests)

**Interfaces:**
- Consumes: Tasks 1, 3 and 4 — zeros can no longer be authored.
- Produces: nothing. A Portal timing rule is now adopted exactly as sent.

**Why this is last:** both call sites operate only on Portal-sourced timing rules, never on a manually built config. Refbox's own one-second settings floor is *not* the guard that replaces this — the Portal's rejection of zeros is. Removing it before zeros are impossible would reintroduce a crash at the end of a finals game.

- [ ] **Step 1: Read the tests before deleting them**

Run: `sed -n '8505,8600p' refbox/src/tournament_manager/mod.rs`

Four tests reference the function: `test_normalize_degenerate_overtime`, the two finals tests around `:8570-8590`, and any other hit from `grep -n normalize_degenerate_overtime`. Note what each one asserts. The two finals tests assert end-to-end behaviour (`!tm.config.overtime_allowed`, `tm.config.sudden_death_allowed`) — those assertions encode the *old* normalisation and must go with it, not be rewritten to expect the same outcome by another route.

- [ ] **Step 2: Delete the function, its call sites, and its tests**

- Remove `fn normalize_degenerate_overtime` (`:1323-1327`) and its doc comment.
- At `:1377-1380`, collapse to:

```rust
        if let Some(ref timing) = next_game_info.timing {
            self.config = timing.clone().into();
```

- At `:1537-1540`, collapse to:

```rust
        if let Some(timing) = self.next_game.take().and_then(|info| info.timing) {
            self.config = timing.into();
        }
```

- Delete `test_normalize_degenerate_overtime` and the finals tests that assert the normalisation. If `finals_timing_rule()` becomes unused, delete it too — clippy will say so.

- [ ] **Step 3: Build and run the crate's tests**

Run: `cargo test -p refbox`
Expected: PASS.

- [ ] **Step 4: Re-run the degenerate-config probe specifically**

Run: `cargo test -p refbox zero_probe -- --nocapture`

Expected: PASS. This is the check that matters most in this task. `zero_probe` drives every combination of seven zeroed durations and allows only `half_play` and `ot_half_play` to fail a tick. Removing the normalisation changes what reaches those paths, so read the printed report rather than trusting the exit code alone.

If it now fails for a config outside that allowlist, **stop and report** — do not widen `MAY_FAIL` to make it pass. That would be editing the test to fit the code.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "refactor(refbox): drop the degenerate-overtime workaround"
```

---

### Task 6: Whole-workspace verification

**Files:** none modified unless something breaks.

- [ ] **Step 1: Check every crate that depends on `uwh-common`**

Run: `cargo check --workspace --all-targets`
Expected: success. The dependants are `refbox`, `schedule-processor`, `overlay`, `overlay-bridge`, `led-panel-sim`, `matrix-drawing`. A new struct field with `#[serde(default)]` should not break any of them; confirm rather than assume.

- [ ] **Step 2: Run the full gate**

Run: `just check`
Expected: fmt, lint, tests and audit all clean.

Note: `just check` is host-only and does not cover Windows or aarch64. That is a known limit, not something to fix here.

- [ ] **Step 3: Confirm the branch is still unpushed**

Run: `git status -sb && git log --oneline origin/master..HEAD`
Expected: six commits ahead, no upstream push. **Do not push. Do not open a PR.** Both need Eric's approval, and this branch waits on the Portal.

- [ ] **Step 4: Commit nothing**

If Steps 1-3 are clean there is nothing to commit. If a dependant crate needed a fix, commit it as `fix(<crate>): <what>` and re-run `just check`.

---

## Deviations

All six tasks completed 2026-09-11. `just check` exits 0. Eight commits, branch unpushed.

1. **Tasks 3 and 4 were committed together.** The plan had Task 3 commit a check that Task 4's
   fixtures would then fail. Combining them keeps every commit green and bisectable.

2. **A seventh piece of work was added, with Eric's approval:** `calculate_occupied_times` in
   `schedule_checks.rs` sized every game as two halves plus a break. It was already wrong for
   single-period games and this branch made it worse, because the half-time it adds is now a real
   number rather than zero — a 12-minute single period read as 31 minutes instead of 16, which
   would report overlaps between games that do not overlap. Fixed here rather than deferred,
   because the regression is one this branch introduces. It was a **third** site reasoning about
   game length from the old assumption; the spec had found only two.

3. **Task 5 rewrote the FINALS fixture and two tests instead of deleting them.** The plan said to
   delete tests that encoded the old normalisation. Two of them also covered behaviour worth
   keeping — that the score-confirm pause is not zero-length, and that a tie goes to sudden death.
   The fixture now expresses a finals rule as one must now be authored, so that coverage survives.
   Only `test_normalize_degenerate_overtime`, which tested nothing but the removed function, was
   deleted.

4. **An obsolete uwh-common test was inverted rather than deleted.**
   `test_timing_rule_single_half_when_no_halftime_break` asserted the zero *meant* single-period.
   It is now `test_zero_halftime_alone_no_longer_signals_single_half`, so the removal has a guard
   instead of an absence.

5. **One extra fixture cleaned.** `test_timing_rule_game_block_uses_schedule_minimum_break` still
   carried zeroed overtime durations. Verified its assertion does not move (1560s) before changing
   it. There is now no zero-duration idiom anywhere in either crate.

## What the verification does and does not prove

`zero_probe` passes, and its report shows the dangerous shape — overtime allowed with a zero
overtime half — reporting a recoverable tick failure rather than crashing or hanging. **But the
probe builds `GameConfig` values directly and never goes through a portal timing rule, which was
the only path `normalize_degenerate_overtime` ever ran on.** So the probe did not exercise the
deleted code and is not, by itself, evidence the deletion is safe. What makes it safe is that the
input is refused at authoring time (Task 3).

`just check` is host-only: it does not cover Windows or aarch64.
