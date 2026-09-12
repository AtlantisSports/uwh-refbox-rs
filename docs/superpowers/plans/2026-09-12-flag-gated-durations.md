# Flag-Gated Durations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `schedule-processor` refuses, before upload, any timing-rule duration left at zero while the switch that makes the rule use it is on — mirroring the Portal's own validator so the organiser gets our message naming the rule and the switch, instead of a server error.

**Architecture:** The branch's existing blanket `check_no_zero_durations` **shrinks** into a conditional check. One table, one row per duration, each row carrying the condition under which the rule actually uses that duration. Nothing new is added alongside it — there is exactly one zero-duration check in the target state, not two.

**Tech Stack:** Rust 2024, MSRV 1.85. `schedule-processor/src/schedule_checks.rs`. No new dependencies.

**Spec:** `docs/backlog/flag-gated-durations/NOTE.md` (untracked, main checkout), as corrected by `uwh-portal-5a`'s first-hand read of `api/Controllers/Models/EventGameTimingRuleModel.cs` at `4bf821f54` on 2026-09-12.

**Base:** `feat/uwh-common/single-period-flag`. That is the only base where `singlePeriod` exists, and without it the half-time check cannot be expressed at all. Eric ruled 2026-09-12 that both workspaces move together, one coordinated landing — not portal-then-refbox.

## Global Constraints

- **The eight gates, verbatim from the Portal's `AddDurationRules`.** Confirmed field-for-field by
  `uwh-portal-5a` against their code, not relayed from notes:

  | Field | Checked when |
  |---|---|
  | `halfPlayDuration` | always |
  | `minimumBreak` | always |
  | `halfTimeDuration` | `singlePeriod` is **not** true |
  | `teamTimeoutDuration` | `teamTimeoutCount` is **not** 0 |
  | `overtimeHalfPlayDuration` | `overtimeAllowed` is **not** false |
  | `overtimeHalfTimeDuration` | `overtimeAllowed` is **not** false |
  | `preOvertimeBreak` | `overtimeAllowed` is **not** false |
  | `preSuddenDeathDuration` | `suddenDeathAllowed` is **not** false |

- **The half-time gate is `singlePeriod != true`, NOT `!= false`.** Writing it the other way checks
  half-time only for single-period games — precisely inverted. This was caught in review before any
  code was written; do not "simplify" it back.
- **The Portal's predicates are `!= false` / `!= true` because its flags are nullable — an ABSENT
  flag makes the gate fire.** Our `TimingRule` flags are plain `bool`, and `single_period` carries
  `#[serde(default)]`, so an absent `singlePeriod` deserialises to `false` and the gate fires. The
  behaviours already agree; no `Option` is needed and none should be introduced.
- **`gameBlock` is OURS ONLY.** The Portal has no such check. Keep the branch's existing rule — a
  `gameBlock` that is present and zero is refused; an absent one is legal and means "derive it".
- **Keep sending `0` for a switched-off duration. Do not switch to omitting fields.** Proven live by
  `uwh-portal-5a` against a seeded API: both switches off with four zeroed durations returned 200 and
  the zeros persisted, while omitting a flag returned 400 at model binding. Our wire type has no
  `skip_serializing_if` on these fields, so omission is not available without a contract change.
- **Do not touch `calculate_occupied_times`.** It is a third copy of the game-sizing formula and is
  owned by the base branch's own work.
- **No `unwrap()`/`expect()` in non-test code. `just check` must exit 0.**

---

### Task 1: Shrink the blanket check into the conditional one

**Files:**
- Modify: `schedule-processor/src/schedule_checks.rs` (replace `check_no_zero_durations`, ~line 700+; rewire in `run_schedule_checks`, line 15)

**Interfaces:**
- Consumes: `TimingRule` from `uwh_common::uwhportal::schedule` (already glob-imported), including `single_period: bool` from the base branch.
- Produces: `check_flag_gated_durations(&Schedule) -> Result<(), Box<dyn std::error::Error>>`,
  `flag_gated_zero_durations(&TimingRule) -> Vec<ZeroDuration>`,
  `struct ZeroDuration { field: &'static str, gate: Option<&'static str> }`,
  `zero_duration_message(&TimingRule, &ZeroDuration) -> String`.

- [ ] **Step 1: Replace the check body**

Delete `check_no_zero_durations` entirely and put this in its place:

```rust
/// A duration a timing rule leaves at zero even though the rule will use it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ZeroDuration {
    /// The Portal's own field name, so the organiser can find the box to fill in.
    field: &'static str,
    /// The switch that makes this duration required, in words an organiser reads.
    /// `None` where every rule needs the duration whatever its switches say.
    gate: Option<&'static str>,
}

/// Every duration this rule will actually use but has left at zero.
///
/// A duration is only checked when its own switch says the rule uses it: a rule
/// with overtime turned off may carry zeros in all three overtime fields, and
/// that is correct, not a fault. This mirrors the Portal's `AddDurationRules`,
/// which gates each field with `.When(...)` and refuses only what the rule uses.
///
/// The half-time gate is `single_period` being FALSE. A single-period game has no
/// half-time, so a zero there is legal; a two-period game must have a real one.
/// Writing this gate the other way round checks half-time only for single-period
/// games, which is exactly backwards.
fn flag_gated_zero_durations(rule: &TimingRule) -> Vec<ZeroDuration> {
    // One row per duration: the value, whether this rule uses it, and the switch
    // that decides. A new duration is one row here, not a new branch elsewhere.
    let checks = [
        (rule.half_play_duration, true, "halfPlayDuration", None),
        (rule.minimum_break, true, "minimumBreak", None),
        (
            rule.half_time_duration,
            !rule.single_period,
            "halfTimeDuration",
            Some("this rule is not a single-period game"),
        ),
        (
            rule.team_timeout_duration,
            rule.team_timeout_count != 0,
            "teamTimeoutDuration",
            Some("it allows team timeouts"),
        ),
        (
            rule.ot_half_play_duration,
            rule.overtime_allowed,
            "overtimeHalfPlayDuration",
            Some("it allows overtime"),
        ),
        (
            rule.ot_half_time_duration,
            rule.overtime_allowed,
            "overtimeHalfTimeDuration",
            Some("it allows overtime"),
        ),
        (
            rule.pre_overtime_break,
            rule.overtime_allowed,
            "preOvertimeBreak",
            Some("it allows overtime"),
        ),
        (
            rule.pre_sudden_death_duration,
            rule.sudden_death_allowed,
            "preSuddenDeathDuration",
            Some("it allows sudden death"),
        ),
    ];

    let mut found: Vec<ZeroDuration> = checks
        .into_iter()
        .filter(|(value, is_used, _, _)| *is_used && value.is_zero())
        .map(|(_, _, field, gate)| ZeroDuration { field, gate })
        .collect();

    // Ours only - the Portal has no Game Block check. Absent means "derive it",
    // which is legal; only a Game Block that is present and zero is an error.
    if rule.game_block == Some(Duration::ZERO) {
        found.push(ZeroDuration {
            field: "gameBlock",
            gate: None,
        });
    }

    found
}

/// Read by a tournament organiser who has to go and fix it, so it names the rule,
/// the field as the Portal labels it, and the switch that makes it required.
fn zero_duration_message(rule: &TimingRule, zero: &ZeroDuration) -> String {
    match zero.gate {
        Some(gate) => format!(
            "Timing rule '{}' sets {} to zero, but {}. Give it a real length, or turn \
             that setting off.",
            rule.name, zero.field, gate
        ),
        None => format!(
            "Timing rule '{}' sets {} to zero. Every timing rule needs a real one.",
            rule.name, zero.field
        ),
    }
}

/// Refuse a timing rule that leaves a duration at zero while the switch that uses
/// it is on.
///
/// The Portal refuses these at upload. Catching them here means the organiser gets
/// a message naming the rule, the field and the switch, before anything is sent.
///
/// Every offending rule is reported, not just the first: an organiser fixing a
/// schedule wants the whole list in one pass.
fn check_flag_gated_durations(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid_rules = Vec::new();

    for rule in &schedule.timing_rules {
        let zeros = flag_gated_zero_durations(rule);
        if zeros.is_empty() {
            continue;
        }
        for zero in &zeros {
            error!("{}", zero_duration_message(rule, zero));
        }
        invalid_rules.push(rule.name.clone());
    }

    if invalid_rules.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Found timing rules with a zero duration the rule still uses: {}",
            invalid_rules.join(", ")
        )
        .into())
    }
}
```

- [ ] **Step 2: Rewire `run_schedule_checks`**

Replace the `check_no_zero_durations(schedule)?;` line (line 15) with:

```rust
    check_flag_gated_durations(schedule)?;
```

- [ ] **Step 3: Confirm it compiles before touching the tests**

Run: `cargo build -p schedule-processor`
Expected: compiles. The old tests still reference `check_no_zero_durations` and will fail to
compile under `cargo test`; that is Task 2's job and is expected at this point.

---

### Task 2: Replace the superseded tests with conditional ones

**Files:**
- Modify: `schedule-processor/src/schedule_checks.rs` (`mod tests`, ~line 766+)

**Interfaces:**
- Consumes: `a_valid_rule()` and `schedule_with_rules(Vec<TimingRule>)` — both already exist in the
  base branch's test module. `a_valid_rule()` is `RR`: two 12-minute halves, 3-minute half-time,
  4-minute minimum break, 1 timeout of 60s, overtime OFF, sudden death OFF, `single_period` false,
  overtime durations 300/60/180, `pre_sudden_death_duration` 60, `game_block` Some(1920).

- [ ] **Step 1: Delete the two superseded tests**

Delete `every_duration_field_is_rejected_at_zero` — it pins the blanket rule the PO overruled.
Delete `a_single_period_rule_still_needs_a_real_half_time` — it asserts the opposite of the
contract we now mirror.

- [ ] **Step 2: Rename the surviving tests' calls**

`a_rule_with_no_zero_durations_passes` and `an_absent_game_block_is_not_a_zero` both call
`check_no_zero_durations`. Change those two calls to `check_flag_gated_durations`. Leave the
`calculate_occupied_times` tests alone.

- [ ] **Step 3: Add the per-pair tests**

Append inside `mod tests`:

```rust
    /// The table is only worth anything if each row fires for its own field and
    /// stays quiet for every other, so each pair is asserted in both directions.
    fn fields_flagged(rule: &TimingRule) -> Vec<&'static str> {
        flag_gated_zero_durations(rule)
            .into_iter()
            .map(|z| z.field)
            .collect()
    }

    #[test]
    fn a_sound_rule_flags_nothing() {
        assert_eq!(fields_flagged(&a_valid_rule()), Vec::<&str>::new());
    }

    #[test]
    fn a_zero_half_play_duration_is_refused_whatever_the_switches() {
        let mut rule = a_valid_rule();
        rule.half_play_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["halfPlayDuration"]);
    }

    #[test]
    fn a_zero_minimum_break_is_refused_whatever_the_switches() {
        let mut rule = a_valid_rule();
        rule.minimum_break = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["minimumBreak"]);
    }

    #[test]
    fn a_two_period_rule_needs_a_real_half_time() {
        let mut rule = a_valid_rule();
        rule.single_period = false;
        rule.half_time_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["halfTimeDuration"]);
    }

    #[test]
    fn a_single_period_rule_may_leave_half_time_at_zero() {
        // The gate is `singlePeriod != true`: a single-period game has no
        // half-time, so a zero is correct rather than a fault. The Portal
        // accepts this shape, and refusing it here would block uploads they
        // allow. Replaces `a_single_period_rule_still_needs_a_real_half_time`,
        // which encoded the blanket rule the PO overruled on 2026-09-12.
        let mut rule = a_valid_rule();
        rule.single_period = true;
        rule.half_time_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn a_rule_allowing_timeouts_needs_a_real_timeout_length() {
        let mut rule = a_valid_rule();
        rule.team_timeout_count = 1;
        rule.team_timeout_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["teamTimeoutDuration"]);
    }

    #[test]
    fn a_rule_allowing_no_timeouts_may_leave_the_timeout_length_at_zero() {
        let mut rule = a_valid_rule();
        rule.team_timeout_count = 0;
        rule.team_timeout_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn overtime_on_needs_all_three_overtime_durations() {
        // All three are reported at once, not just the first: an organiser
        // fixing the rule wants the whole list in one pass.
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        assert_eq!(
            fields_flagged(&rule),
            vec![
                "overtimeHalfPlayDuration",
                "overtimeHalfTimeDuration",
                "preOvertimeBreak"
            ]
        );
    }

    #[test]
    fn overtime_off_may_leave_all_three_overtime_durations_at_zero() {
        // The commonest real shape by far: the round-robin rule in every one of
        // our exports carries zeros here with overtime switched off.
        let mut rule = a_valid_rule();
        rule.overtime_allowed = false;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn sudden_death_on_needs_a_real_pre_sudden_death_break() {
        let mut rule = a_valid_rule();
        rule.sudden_death_allowed = true;
        rule.pre_sudden_death_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["preSuddenDeathDuration"]);
    }

    #[test]
    fn sudden_death_off_may_leave_the_pre_sudden_death_break_at_zero() {
        let mut rule = a_valid_rule();
        rule.sudden_death_allowed = false;
        rule.pre_sudden_death_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn a_present_zero_game_block_is_still_refused() {
        // Ours only - the Portal has no Game Block check - so nothing upstream
        // would catch this if the row were dropped.
        let mut rule = a_valid_rule();
        rule.game_block = Some(Duration::ZERO);
        assert_eq!(fields_flagged(&rule), vec!["gameBlock"]);
    }

    #[test]
    fn the_message_names_the_rule_the_field_and_the_switch() {
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        let zeros = flag_gated_zero_durations(&rule);
        let msg = zero_duration_message(&rule, &zeros[0]);
        assert!(msg.contains("RR"), "must name the rule, got: {msg}");
        assert!(
            msg.contains("overtimeHalfPlayDuration"),
            "must name the field, got: {msg}"
        );
        assert!(
            msg.contains("it allows overtime"),
            "must name the switch that makes it required, got: {msg}"
        );
    }

    #[test]
    fn an_always_required_duration_does_not_invent_a_switch() {
        let mut rule = a_valid_rule();
        rule.half_play_duration = Duration::ZERO;
        let zeros = flag_gated_zero_durations(&rule);
        let msg = zero_duration_message(&rule, &zeros[0]);
        assert!(
            !msg.contains("but"),
            "there is no switch to name for an always-required duration, got: {msg}"
        );
        assert!(msg.contains("Every timing rule needs"), "got: {msg}");
    }

    #[test]
    fn check_flag_gated_durations_names_every_offending_rule_not_just_the_first() {
        // The blanket check this replaced returned on the first bad rule, so an
        // organiser fixed one, re-ran, and found another.
        let mut first = a_valid_rule();
        first.half_play_duration = Duration::ZERO;
        let mut second = a_valid_rule();
        second.name = "FINALS".to_string();
        second.minimum_break = Duration::ZERO;

        let err = check_flag_gated_durations(&schedule_with_rules(vec![first, second]))
            .expect_err("both rules are invalid");
        let msg = err.to_string();
        assert!(msg.contains("RR"), "should name the first rule, got: {msg}");
        assert!(
            msg.contains("FINALS"),
            "should name the second rule too, got: {msg}"
        );
    }

    #[test]
    fn the_production_finals_shape_is_refused() {
        // The exact shape every one of our JSON exports carried before this
        // branch: overtime allowed, all three overtime durations zero, only the
        // pre-sudden-death break filled in. The Portal refuses it (verified live
        // against their API), and `normalize_degenerate_overtime` used to be the
        // thing that stopped it crashing refbox at the end of such a game.
        let mut rule = a_valid_rule();
        rule.name = "FINALS".to_string();
        rule.overtime_allowed = true;
        rule.sudden_death_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        rule.pre_sudden_death_duration = Duration::from_secs(60);

        assert!(
            check_flag_gated_durations(&schedule_with_rules(vec![rule])).is_err(),
            "the shape that crashed the app must not reach an upload"
        );
    }

    #[test]
    fn run_schedule_checks_refuses_a_rule_whose_switched_on_duration_is_zero() {
        // Guards the WIRING, not the check. Every other test here calls
        // `check_flag_gated_durations` directly, so deleting its line in
        // `run_schedule_checks` would leave the whole suite green and the gate
        // silently gone. That exact gap got past the Game Block review.
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;

        let refused = run_schedule_checks(&schedule_with_rules(vec![rule]))
            .expect_err("a zero duration the rule uses must stop the schedule loading");
        assert!(
            refused.to_string().contains("zero duration"),
            "must be refused for the zero duration, not something else: {refused}"
        );

        run_schedule_checks(&schedule_with_rules(vec![a_valid_rule()]))
            .expect("the same schedule with sound durations passes");
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p schedule-processor schedule_checks`
Expected: all pass.

---

### Task 3: Undo the invented durations the blanket rule forced

**Files:**
- Modify: `schedule-processor/tests/fixtures/portal-schedule-with-finals.json`
- Modify: `schedule-processor/src/json_loader.rs` (`HAPPY_PATH_JSON`, ~line 54)

The base branch rewrote **eleven** real zeros into fabricated numbers purely to satisfy the blanket
check. Under the conditional rule, eight of them were never needed and revert cleanly; the remaining
three revert only once the FINALS rule carries the right flag. Leaving invented values in place
would violate `feedback_never_invent_fallbacks` for no remaining reason.

- [ ] **Step 1: Revert the `RR` rule in the JSON fixture**

In `portal-schedule-with-finals.json`, the `RR` rule has `overtimeAllowed: false` and
`suddenDeathAllowed: false`, so all four are unguarded. Restore the export's real values:

```json
      "overtimeHalfPlayDuration": 0,
      "overtimeHalfTimeDuration": 0,
      "preOvertimeBreak": 0,
      "preSuddenDeathDuration": 0,
```

- [ ] **Step 2: Fix the `FINALS` rule in the JSON fixture — one flag, then revert**

Set `"overtimeAllowed": false` and restore its three overtime zeros:

```json
      "overtimeAllowed": false,
...
      "overtimeHalfPlayDuration": 0,
      "overtimeHalfTimeDuration": 0,
      "preOvertimeBreak": 0,
      "preSuddenDeathDuration": 60,
```

This is not a cosmetic choice. The rule has every overtime field at zero and only the
pre-sudden-death break filled in, which is what sudden-death-only looks like; it matches Eric's
ruling of 2026-09-12 — *"Playoff and final games will have either Overtime and Sudden Death
allowed, or just Sudden Death"*; and the base branch's own `finals_timing_rule()` in
`refbox/src/tournament_manager/mod.rs` is **already** `overtime_allowed: false`, so this makes the
branch agree with itself instead of contradicting itself.

- [ ] **Step 3: Add the `singlePeriod` field to both fixture rules**

The Portal marks `singlePeriod` `[Required]` and rejects its absence at model binding — verified
live, not read. A real export under the target contract carries it, so the fixture should too. Add
to each of the two rules:

```json
      "singlePeriod": false,
```

- [ ] **Step 4: Revert the `RR` rule in `HAPPY_PATH_JSON`**

Same four values, same reason — both switches are off:

```json
                "overtimeHalfPlayDuration": 0,
                "overtimeHalfTimeDuration": 0,
                "preOvertimeBreak": 0,
                "preSuddenDeathDuration": 0,
```

- [ ] **Step 5: Run the fixture-driven tests**

Run: `cargo test -p schedule-processor`
Expected: all pass, including `real_shape_schedule_parses_and_passes_all_checks`, which runs the
whole check suite over the 71-game fixture.

---

### Task 4: Full gate and commit

- [ ] **Step 1: Run the workspace gate**

Run: `just check`
Expected: exit 0 — fmt, lint, tests, audit all clean.

Note: `just check` is host-only and does not cover Windows or aarch64. This change is pure schedule
validation logic with no platform-specific code, so that gap does not apply here.

- [ ] **Step 2: Commit**

```bash
git add schedule-processor/src/schedule_checks.rs \
        schedule-processor/src/json_loader.rs \
        schedule-processor/tests/fixtures/portal-schedule-with-finals.json \
        docs/superpowers/plans/2026-09-12-flag-gated-durations.md
git commit -m "feat(schedule-processor): refuse a zero duration the rule still uses"
```

---

## Deviations

**Messages carry a full remedy per row, not a shared closing sentence.** The plan had
`ZeroDuration { field, gate }` with one shared ending, "Give it a real length, or turn that setting
off." That inverts on `halfTimeDuration` — it is the only gate phrased as a negative, and the fix
there is to turn single-period ON, not off. The field is now `reason` and carries the whole clause
including the remedy. Two tests assert the complete sentence for a negative gate and a positive one,
because asserting the fragments separately still passes if they are assembled into nonsense.

**The presence check `uwh-portal-5a` recommended for `singlePeriod` was NOT added.** Their finding
was that an absent `singlePeriod` deserialises to `false`, passes our check, and is then refused by
the Portal at model binding, stranding the organiser. A probe confirmed the first half and refuted
the second: `single_period` has `#[serde(default)]` but no `skip_serializing_if`, and the upload
payload is built from the same `TimingRule` (`SendableSchedule.timing_rules`), so re-serialising
always emits `"singlePeriod": <bool>`. The Portal never receives it absent from this tool. Adding
the check would have refused input files that upload correctly today.

**`docs/third-party-integration.md` updated, widening the branch past `schedule-processor`.**
Directed by Eric 2026-09-12 after a review finding. The published contract still told integrators
that `halfTimeDuration: 0` signals a single-half game — the convention the base branch replaced with
`singlePeriod`, and which this check now re-legalises as an ordinary unused value. Left alone, an
integrator would classify single-period and two-period games by the wrong field and never learn
`singlePeriod` exists. The table gains `singlePeriod` (16 fields, renumbered), corrects the
`halfTimeDuration` row, adds a "which durations may be `0`" table stating each gate, and fixes two
stale line references. No crate code outside `schedule-processor` is touched.

**Ruled by Eric 2026-09-12: there is no Portal timing-rule editor, and will not be one until that
feature is built.** This retires the review's concern that a rule authored in the Portal UI bypasses
this check — no such client path exists.

Refined the same day by `uwh-portal-5a` and verified first-hand in their repo rather than relayed:
"every rule reaches the Portal through this tool" is still an overstatement. `ModifyTimingRule`
(`api/Controllers/EventScheduleController.cs:890`, `PUT .../schedule/timing-rules/{identifier}`)
has no caller in their repo and no `.ts`/`.tsx` client constructs that path — but it is reachable
directly by anyone with edit rights on the event, and validates nothing until their branch lands.
So this gate is sufficient for everything a person can author, and the one machine-reachable gap is
closed by the Portal change this work is already waiting on. The doc comment states that split
rather than claiming sole coverage.

**Two full code reviews were run, 22 findings, all fixed or answered.** The second was needed
because adding `docs/third-party-integration.md` to the diff staled the first. Three findings were
real defects in this work rather than nits, and are recorded because each is a pattern worth not
repeating: two tests named "…whatever_the_switches" that never varied a switch; an exhaustiveness
guard that read SERIALISED keys and so could not see an `Option` duration under
`skip_serializing_if` (`gameBlock`'s own shape), now also guarded by a no-`..` destructure of
`TimingRule` and proved with a fake field and an `E0027`; and a plainly false sentence published in
the third-party contract — "it is what refbox itself sends" — when refbox never sends a `TimingRule`
at all, only receives them.

**Sequencing confirmed by Eric 2026-09-12, after these deviations were written:** this branch and
`feat/schedule-processor/game-block-check` land together — the portal's #965 sets
`RequireGameBlock = true` with no feature flag, so without the game-block check every upload from
this tool fails the moment #965 merges. Making `singlePeriod` required (dropping its
`serde(default)`) is a SEPARATE branch and starts only AFTER these merge; it is deliberately not in
this diff.

**The approved design spec now states the opposite requirement, and is knowingly left stale.**
`docs/superpowers/specs/2026-09-11-single-period-flag-and-no-zero-durations-design.md:109` reads
"**Blocks the build; not a warning** — Eric's requirement is that the builder *cannot* pass a zero",
with an exhaustive nine-field list at line 100 and the old function name `check_no_zero_durations`.
The PO overruled that blanket requirement on 2026-09-12 in favour of the conditional rule this
branch implements. Per `.claude/rules/plan-execution.md` the spec is left accurate-as-of-approval
and the change is recorded here instead of amended mid-execution — but a reader who lands on the
spec first will read this branch as a regression against a stated PO requirement unless they reach
this section. Worth an amendment in one pass when the single-period work lands.

**Backwards compatibility is explicitly out of scope.** Ruled by Eric 2026-09-12: he controls the
schedule builder and is its only user, so there is no legacy authoring path to accommodate. This
retires the "an older spreadsheet expresses single-period as `halfTimeDuration: 0`" reasoning that
originally motivated looking at the half-time message. The wording fix above stands on its own
merits regardless — it is simply the correct remedy to name.
