# Reject Unknown Timing-Rule Field Names Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `schedule-processor` refuse a CSV whose "Timing Rule Field" cell is not a name
`TimingRule` understands, naming the rule and the offending column, instead of silently
discarding the value.

**Architecture:** `parse_timing_rule_row` pastes the spreadsheet cell into a JSON object as the
key (`format!("\"{field}\": {}", ...)`) and lets serde deserialize the assembled rule. `TimingRule`
has no `deny_unknown_fields`, so serde drops an unrecognised key without a word. The fix is a
const list of the names `TimingRule` understands, checked in the parser before the cell reaches
the JSON string. The function changes from `Option<...>` to `Result<Option<...>, String>`, matching
`parse_group`'s existing shape in the same file.

**Tech Stack:** Rust 2024, MSRV 1.85, `serde_json`, the `csv` crate. No new dependencies.

**Spec:** `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/memory/project_portal_should_send_game_block.md`
(section: "THE CSV BUILDER SILENTLY DISCARDS A MISSPELLED TIMING-RULE COLUMN")

## Global Constraints

- **Scope:** `schedule-processor/src/csv_parser.rs` only. Do not touch `refbox` or `uwh-common`.
- **Do NOT add `deny_unknown_fields` to `TimingRule`.** refbox deserializes the same type from the
  Portal and must keep tolerating fields the Portal adds later; making it strict would reject an
  entire schedule the first time that happens.
- **Preserve:** a row whose name, field and value cells are all empty still yields "no timing rule
  row here" — every schedule CSV has ~50 such rows and none may start erroring.
- **Preserve:** a row shorter than the timing-rule columns still yields "no timing rule row here".
- Clippy runs with `-D warnings`. Run `cargo fmt` before committing.
- Commit format: `type(scope): description`, lowercase, imperative, no trailing period.

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `schedule-processor/src/csv_parser.rs` | Turns the organiser spreadsheet into a `Schedule` | Add `TIMING_RULE_FIELDS`; change `parse_timing_rule_row` to return `Result`; update its one caller; add the file's first `#[cfg(test)]` module |

No other file changes. `parse_timing_rule_row` has exactly one caller
(`csv_parser.rs:84`) — verified by `grep -rn parse_timing_rule_row schedule-processor/src/`.

## Verified Facts This Plan Rests On

Checked against `origin/master` (`866b65e2`) on 2026-09-10:

- `TimingRule` (`uwh-common/src/uwhportal/schedule.rs:241-276`) has **15** JSON names:
  `name`, `teamTimeoutCount`, `teamTimeoutsCountedPerHalf`, `overtimeAllowed`,
  `suddenDeathAllowed`, `last2minStopTime`, `halfPlayDuration`, `halfTimeDuration`,
  `teamTimeoutDuration`, `overtimeHalfPlayDuration`, `overtimeHalfTimeDuration`,
  `preOvertimeBreak`, `preSuddenDeathDuration`, `minimumBreak`, `gameBlock`.
- `Indices::new(1)` puts the timing-rule columns at 19 / 20 / 21 (`8 * parallel + 11..13`).
- `parse_group` in the same file already returns `Result<Option<Group>, String>`, and `parse_csv`
  returns `Box<dyn std::error::Error>`, which `String` converts into. So `?` on a `String` error
  compiles unchanged at the call site.
- Across the 9 repo schedule CSVs that have timing-rule columns, **13 distinct field names appear,
  all of them in the list above, and not one row is partially filled** (never a name without a
  field, never a field without a value). So this check cannot make an existing repo CSV fail.
- `csv_parser.rs` has no `#[cfg(test)]` module yet; `main.rs`, `json_loader.rs`, `site.rs`,
  `scoresheets.rs` and `cmas_official.rs` do. `schedule-processor` is bin-only — there is no lib
  target, so a `tests/` integration test cannot reach these functions, but a `#[cfg(test)]` module
  inside `src/` compiles and runs under `cargo test` normally.

---

### Task 1: Reject an unrecognised timing-rule field name

**Files:**
- Modify: `schedule-processor/src/csv_parser.rs:84` (the caller)
- Modify: `schedule-processor/src/csv_parser.rs:590-602` (`parse_timing_rule_row`)
- Test: `schedule-processor/src/csv_parser.rs` (new `#[cfg(test)] mod tests` at the end of the file)

**Interfaces:**
- Produces: `parse_timing_rule_row(row: &csv::StringRecord, indices: &Indices) -> Result<Option<(String, String)>, String>`
- Produces: `const TIMING_RULE_FIELDS: [&str; 15]`

- [ ] **Step 1: Write the failing tests**

Append to the end of `schedule-processor/src/csv_parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a row with just the three timing-rule cells filled in, using the
    /// single-game-per-row column layout.
    fn timing_rule_row(name: &str, field: &str, value: &str) -> (csv::StringRecord, Indices) {
        let indices = Indices::new(1);
        let mut cells = vec![String::new(); indices.timing_rule_value + 1];
        cells[indices.timing_rule_name] = name.to_string();
        cells[indices.timing_rule_field] = field.to_string();
        cells[indices.timing_rule_value] = value.to_string();
        (csv::StringRecord::from(cells), indices)
    }

    #[test]
    fn known_field_name_is_accepted() {
        let (row, indices) = timing_rule_row("RR", "gameBlock", "1800");
        assert_eq!(
            parse_timing_rule_row(&row, &indices),
            Ok(Some(("RR".to_string(), "\"gameBlock\": 1800".to_string())))
        );
    }

    #[test]
    fn misspelled_field_name_is_rejected_naming_rule_and_column() {
        // Every one of these is silently discarded before the fix.
        for misspelling in ["Game Block", "gameblock", "GameBlock", "gameBlock ", ""] {
            let (row, indices) = timing_rule_row("RR", misspelling, "1800");
            let err = match parse_timing_rule_row(&row, &indices) {
                Err(e) => e,
                other => panic!("'{misspelling}' should be rejected, got {other:?}"),
            };
            assert!(err.contains("RR"), "error should name the rule: {err}");
            assert!(
                err.contains(&format!("'{misspelling}'")),
                "error should quote the offending column: {err}"
            );
        }
    }

    #[test]
    fn every_field_name_the_type_understands_is_accepted() {
        // Guards the list against drifting away from `TimingRule` — a name
        // dropped from the const would start being rejected as unknown.
        for field in TIMING_RULE_FIELDS {
            let (row, indices) = timing_rule_row("RR", field, "1");
            assert!(
                parse_timing_rule_row(&row, &indices).is_ok(),
                "'{field}' should be accepted"
            );
        }
    }

    #[test]
    fn wholly_empty_row_is_not_a_timing_rule_row() {
        let (row, indices) = timing_rule_row("", "", "");
        assert_eq!(parse_timing_rule_row(&row, &indices), Ok(None));
    }

    #[test]
    fn row_too_short_for_the_timing_rule_columns_is_skipped() {
        let indices = Indices::new(1);
        let row = csv::StringRecord::from(vec!["2026-06-26", "09:00"]);
        assert_eq!(parse_timing_rule_row(&row, &indices), Ok(None));
    }
}
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p schedule-processor csv_parser
```

Expected: compile error — `parse_timing_rule_row` returns `Option`, not `Result`, so
`.expect_err` and `Ok(Some(...))` do not typecheck. That is the red state.

- [ ] **Step 3: Add the const and change the function**

Replace `parse_timing_rule_row` (`csv_parser.rs:590-602`) with:

```rust
/// The JSON field names that `TimingRule` understands.
///
/// The spreadsheet's "Timing Rule Field" cell is pasted into the assembled JSON
/// object as the key, and `TimingRule` deliberately does not use
/// `deny_unknown_fields` — refbox reads the same type from the Portal and must
/// keep tolerating fields the Portal adds later. That leniency means serde would
/// drop a misspelled column without a word, so the spelling is checked here
/// instead, where a spreadsheet is the thing being read.
///
/// `name` is in the list because it is a field of the type, but it arrives from
/// its own "Timing Rule Name" column; putting it in the field column produces a
/// duplicate key and fails loudly at deserialization.
const TIMING_RULE_FIELDS: [&str; 15] = [
    "name",
    "teamTimeoutCount",
    "teamTimeoutsCountedPerHalf",
    "overtimeAllowed",
    "suddenDeathAllowed",
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

pub(crate) fn parse_timing_rule_row(
    row: &csv::StringRecord,
    indices: &Indices,
) -> Result<Option<(String, String)>, String> {
    let (Some(name), Some(field), Some(value)) = (
        row.get(indices.timing_rule_name),
        row.get(indices.timing_rule_field),
        row.get(indices.timing_rule_value),
    ) else {
        return Ok(None);
    };

    if name.is_empty() && field.is_empty() && value.is_empty() {
        return Ok(None);
    }

    if !TIMING_RULE_FIELDS.contains(&field) {
        return Err(format!(
            "Timing rule '{name}' has an unrecognised field name '{field}'. \
             Check the spelling of that cell in the 'Timing Rule Field' column. \
             Expected one of: {}",
            TIMING_RULE_FIELDS.join(", ")
        ));
    }

    Ok(Some((
        name.to_string(),
        format!("\"{field}\": {}", value.to_lowercase()),
    )))
}
```

- [ ] **Step 4: Update the one caller**

`csv_parser.rs:84` becomes:

```rust
        if let Some((name, value)) = parse_timing_rule_row(&row, &indices)? {
```

- [ ] **Step 5: Run the tests and watch them pass**

```bash
cargo test -p schedule-processor csv_parser
```

Expected: 5 tests pass.

- [ ] **Step 6: Prove the check can actually fail on a real file**

A passing test is not evidence on its own. Break a real CSV on purpose and watch the tool refuse it:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/reject-unknown-timing-rule-fields
mkdir -p /tmp/claude-1000/tr-check
sed 's/halfPlayDuration/halfplayduration/' \
  "schedule-processor/Mock Schedules for testing/2026 CMAS AUHC - UWHPortal.csv" \
  > /tmp/claude-1000/tr-check/misspelled.csv
```

Then parse both the original and the misspelled copy through `parse_csv` (a throwaway
`#[test]` that reads the two files, or the CLI's own parse path). Expected: the original
parses; the misspelled copy returns an error naming the rule and `halfplayduration`.
Record the exact output in the Deviations section. Before the fix, the misspelled copy
parsed **successfully** with a wrong half length — confirm that too by stashing the change.

- [ ] **Step 7: Format, lint and test the workspace**

```bash
cargo fmt --all
just check
```

Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add schedule-processor/src/csv_parser.rs
git commit -m "fix(schedule-processor): reject an unrecognised timing-rule field name"
```

---

### Task 2: Run a live organiser template through the updated builder

This is the point of the change, not an extra. It discharges by *running* the question that has
been asked three times and answered sideways: do the live organiser templates carry a `gameBlock`
row, and under what spelling? Mandatory `gameBlock` on the Portal side puts this on the critical
path — a template without the row means no organiser can upload at all.

~~**Blocked on Eric supplying a live template.**~~ **UNBLOCKED and DONE — see below.** No CSV in
this repo has a `gameBlock` row in any spelling; the nine that have timing-rule columns are older
exports, which is why they disagreed with Eric. Both were true.

**DONE 2026-09-10. Eric supplied the 2026 USA UWH Nationals template and it discharges the
precondition outright.** Result: **parses clean, and every one of its four timing rules carries a
`gameBlock` row, spelled exactly `gameBlock`.**

| rule | half | halfTime | minBreak | declared `gameBlock` | floor | tightest real gap between its games |
|---|---|---|---|---|---|---|
| RR | 720s | 180s | 240s | **1920s** (32:00) | 1860s | 1920s |
| XO | 720s | 180s | 240s | **1920s** (32:00) | 1860s | single game |
| PO | 720s | 180s | 240s | **2100s** (35:00) | 1860s | 2100s |
| MD | 720s | 180s | 240s | **2400s** (40:00) | 1860s | 2400s |

Every declared block is **above** `game_block_minimum()` and **below** the 60-minute ceiling, so
the queued Task-2 checks would pass this schedule silently — the right outcome. And on every rule
with more than one consecutive game the declared value equals the tightest observed gap **exactly**,
on both courts. This is authored, correct data, not a guess.

It also re-confirms why derivation was abandoned: RR's observed gaps are
`[1920, 2520, 5520, 6420]` — the 10-minute, 60-minute and 75-minute breaks. The minimum happens to
be right here; a session that never ran back-to-back would have derived a break as the block.

**One thing the template exposes, live rather than theoretically:** it sets
`last2minStopTime` **TRUE for the MD (medal) rules** and FALSE elsewhere. The Portal has no such
field and drops it, so a Portal-sourced medal game shows "No" on the game-info screen against an
organiser who wrote Yes. That is the recorded `last2minStopTime` bug with a real event attached.

- [ ] ~~**Step 1: Ask Eric for one current organiser template CSV**~~ (any event; the timing-rule
      columns are what matter).
- [ ] **Step 2: Run it through the built binary**

```bash
cargo run -p schedule-processor -- <the template>
```

- [ ] **Step 3: Record the outcome in Deviations** — one of:
  - parses clean and a `gameBlock` row is present → the precondition is discharged, say so;
  - parses clean and there is no `gameBlock` row → templates need the row added, and that is
    Eric's to do before the Portal enables enforcement;
  - **refused, naming a column** → the template has a spelling this repo's exports do not, and
    the check just earned itself. Report the exact name.

---

## Deviations

**Task 1 executed 2026-09-10. All 5 tests pass; `just check` exit 0.**

1. **The evidence file had to use `gameBlock`, not `halfPlayDuration`.** Step 6 originally
   proposed misspelling `halfPlayDuration`. That turned out to be the wrong demonstration:
   `halfPlayDuration` is a *required* serde field, so dropping it already failed loudly with
   "missing field". **The silent discard only bites the two fields that carry
   `#[serde(default)]` — `gameBlock` and `last2minStopTime`** — which sharpens the recorded fact:
   a misspelt `minimumBreak` is NOT silently discarded, it fails as a missing field. `gameBlock`,
   the field this whole workstream turns on, is one of the two that vanish quietly.

   Before/after, same real CSV (2026 CMAS AUHC) with one appended `EMRR` timing-rule row:

   | field cell | before the fix | after the fix |
   |---|---|---|
   | `gameBlock` | parses, `EMRR -> Some(1800s)` | parses, `EMRR -> Some(1800s)` |
   | `Game Block` | **parses clean, `EMRR -> None`** | **refused, naming `EMRR` and `'Game Block'`** |

2. **The `Mock Schedules for testing/` CSVs are untracked**, so they do not exist in this
   worktree. They were copied from the main checkout for the evidence run. Nothing in the test
   suite depends on them.

3. **Lint coverage.** `cargo clippy --workspace --all-targets --all-features` cannot run on this
   machine — `grafton-ndi` needs the NDI SDK. `cargo clippy -p schedule-processor --all-targets`
   is **clean (exit 0)**, which is the gate that matters here since all new code sits in a
   `#[cfg(test)]` module that `just check` does not lint. `--all-targets` across the whole
   workspace reports 3 **pre-existing** errors, all in files this branch does not touch
   (`overlay-bridge/src/status.rs:918`, `refbox/src/app/view_builders/keypad_pages/player_grid.rs:126`,
   `refbox/src/app/mod.rs:8372`).

4. The rejection test uses a `match` rather than `.expect_err(&format!(...))`, to avoid
   `clippy::expect_fun_call`. Patched into the plan before execution.

## Explicitly Out Of Scope

- `deny_unknown_fields` on `TimingRule` (would break refbox against a future Portal field).
- The Game Block floor / 60-minute-ceiling checks — queued next, separate branch.
- B-1 and B-2 (the refbox `game_block` fixes) — queued next, separate branch.
- Trimming whitespace from the field cell. `"halfPlayDuration "` is silently discarded today and
  will now be a loud error, which is the improvement asked for; quietly accepting it would be a
  fallback, and inventing fallbacks is exactly what this change removes.
- A "did you mean…" suggestion using the `strsim` dependency the crate already carries. Worth
  proposing later; not asked for here.
- Naming the CSV row number in the error. Would need a new parameter; the rule name plus the
  column name is what was specified.


---

## Code review (mandatory pre-PR check 1)

Run 2026-09-10 with the built-in `code-review` skill at high effort, base passed explicitly as
`origin/master` (the local `master` ref is ~100 commits stale and peer sessions own it —
[[reference_code_review_skill_picks_stale_base]]). **11 findings. Every factual claim was
independently verified before acting; all held.** Fixed in `8a248ccd`.

**Accepted and fixed (9):**
1. **No trimming.** The csv reader is `Trim::None` and every sibling parser in this file trims
   (`parse_group` csv_parser.rs:411, `parse_games` csv_parser.rs:314) — mine did not. A trailing
   space would refuse the sheet with the rejected and expected names looking identical. Now trims
   all three cells. My plan had this as a deliberate "out of scope" decision; the review was right
   and the plan was wrong.
2. **Circular test.** `for x in L { assert L.contains(x) }` never touched `TimingRule` — exactly
   the anti-pattern in [[reference_test_asserting_on_its_own_fixture]], which I had in memory and
   wrote anyway. Replaced with a comparison against the keys `TimingRule` actually serialises.
3. **Rule name untrimmed** → a stray space made a separate `IndexMap` key no game could match.
4. **`name` in the accepted list.** With a *textual* rule name it produces a JSON **syntax** error,
   not the duplicate-key error my doc comment claimed — and listing it first under "Expected one
   of" actively invited the mistake. Removed from the list; given its own message.
5. **Empty value cell** with a valid field name reached the assembler → raw serde syntax error.
6. **Blank field with a stray value** misdiagnosed as a spelling mistake in an empty column.
   5 and 6 fixed together by handling every partial-fill combination.
7. **No row number.** Added (`spreadsheet_row`, header = row 1). Beyond the literal Scope Card
   wording ("naming the rule and column") but squarely its intent: the operator must find the cell.
8. **Doc comment overstated the bug** ("serde would drop a misspelled column") — true for 2 of 15.
9. **No mock CSV**, violating `schedule-processor/CLAUDE.md`, **a crate-level rules file I never
   read** — same trap as [[reference_crate_level_claude_md_not_autoloaded]], different crate.

**Partly accepted (1):** finding 11 asked for a mock CSV in `Mock Schedules for testing/`. That
directory is **gitignored** (`.gitignore:16`), so a mock added there is never committed and CI
never runs it. Added a tracked fixture at `schedule-processor/tests/fixtures/timing-rule-fields.csv`
instead, driven end-to-end through `parse_csv`, which meets the rule's intent with coverage that
survives.

**Pushed back (1):** finding 7 also asked to accumulate every bad cell rather than bail on the
first. Declined: `parse_games`, `parse_group` and the row reader all bail via `?`, so accumulating
in this one place would make the file inconsistent for a modest gain. Row numbers already let the
operator fix each in one pass. Worth revisiting only if operators report it.

**Not actioned (1):** finding 9 notes `docs/third-party-integration.md:1370-1385` is a third copy
of the field list. It documents the Portal wire contract, not the CSV column set, and this change
does not make it stale. The new drift test anchors const↔type, which was the actionable half.

### Mutation testing — every new guard was seen RED

A passing test proves nothing on its own. Each guard was broken on purpose:

| mutation | result |
|---|---|
| drop `"gameBlock"` from the const | **compile error** — array length 14 vs 13 |
| add `"gameBlockSeconds"` to the const | drift test **FAILED** |
| misspell a const entry (`gameBlok`) | drift test **FAILED**, with a readable diff |
| remove the `.trim()` calls | 2 tests **FAILED** |
| remove the unrecognised-field check | 2 tests **FAILED** |
| add a 16th field to `TimingRule` upstream | **compile error** (uwh-common's own exhaustive destructure fails first) |
| change only `rename = "gameBlock"` upstream | drift test **FAILED** — this is the one genuinely silent drift, and it is caught |

**Process note for next time:** while running the upstream mutations I first edited
`../../uwh-common/...`, which from a worktree is the **shared main checkout**, not the worktree's
copy. Restored within seconds and `git status` verified clean, but the correct relative path from
a worktree root is `uwh-common/...` with no `../..`.
