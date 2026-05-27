# schedule-processor JSON Input — Design

**Date:** 2026-05-27
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/schedule-processor/json-input` (off `origin/master` @ `ca3254f3`)

---

## Goal

Let the tournament organizer feed a portal-format schedule **JSON** file
into the `schedule-processor` CLI, alongside the existing CSV path. After
selecting the file, the rest of the tool — validation, team mapping,
upload to the portal — runs exactly as it does today.

The immediate motivation is an Australian-tournament schedule that arrived
as a JSON file in the portal's native wire format
([`SendableSchedule`](../../../uwh-common/src/uwhportal/schedule.rs)).
Today the only way to get this JSON into the portal via `schedule-processor`
is to first convert it to CSV; this design removes that detour.

---

## Motivation (plain English)

Tournament organizers sometimes hand us a schedule in the JSON shape the
UWH Portal accepts directly. That shape is *already* the destination
format — converting it back to CSV just so `schedule-processor` will read
it is busywork that can introduce errors.

The fix is small: teach `schedule-processor` to read either format. The
validation, team-name → portal-team-ID mapping, and upload-to-portal
steps all run on the same in-memory `Schedule` value, so they don't care
which format the file came from.

---

## Scope

### Files touched

- `uwh-common/src/uwhportal/schedule.rs` — **purely additive**. Add
  `impl From<(SendableSchedule, EventId)> for Schedule`, sitting
  immediately below the existing inverse
  `impl From<Schedule> for SendableSchedule`. No new types, no wire
  format change, no breaking changes to any existing API. Approximately
  11 lines of code + one unit test for the new impl.
- `schedule-processor/src/main.rs` — change the file dialog filter to
  accept `.csv` or `.json`; after a file is picked, branch on extension
  to call either `parse_csv` (existing) or `parse_json` (new). No other
  change to `main.rs`.
- `schedule-processor/src/json_loader.rs` — **new file**. Contains
  `parse_json(&str, UtcOffset, EventId) -> Result<Schedule, _>`,
  symmetric to `parse_csv`'s public signature.
- `schedule-processor/Mock Schedules for testing/2026 Australian Nationals
  - portal export.json` — **new fixture**. The Australian-tournament
  JSON the organizer supplied, used by an integration test and for any
  future regression coverage.
- `schedule-processor/tests/json_real_world.rs` — **new integration
  test** (creates the `tests/` directory; currently absent). Loads the
  mock JSON fixture, parses it via `parse_json`, runs
  `run_schedule_checks` on the result, asserts no errors.

### Files NOT touched

- `refbox/` — refbox continues to fetch schedules from the portal; no
  offline-schedule loading is added.
- `schedule-processor/src/csv_parser.rs` — untouched. The CSV path is
  preserved as-is.
- `schedule-processor/src/schedule_checks.rs` — untouched. All
  `run_schedule_checks` validation runs unchanged on the JSON-loaded
  `Schedule`.
- `wireless-remote/` — out of scope (no relation).

### Crates in play

`schedule-processor` (main change) plus a purely additive 11-line touch
in `uwh-common`. Per `.claude/rules/plan-execution.md`, the
`uwh-common` touch triggers **heavy process for that one task**:
after the `uwh-common` change lands, we run the downstream-crate
checklist (`cargo check -p refbox -p schedule-processor -p overlay
-p led-panel-sim` and `cargo build -p uwh-common --no-default-features`)
to confirm no breakage. Because the change is purely additive and
introduces no new types, no wire-format change, and no breaking
changes to any existing API, the risk is bounded and the check is
expected to pass on the first try.

---

## Design

### Input selection

In `main.rs`, change the rfd file dialog filter from CSV-only to a
union filter:

```rust
// Today:
let csv_path = FileDialog::new()
    .add_filter("CSV files", &["csv"])
    .set_title("Select Schedule CSV File")
    .pick_file();

// Becomes:
let schedule_path = FileDialog::new()
    .add_filter("Schedule files", &["csv", "json"])
    .set_title("Select Schedule File")
    .pick_file();
```

After the file is picked, branch on extension (case-insensitive):

```rust
let schedule = match schedule_path.extension().and_then(|e| e.to_str()) {
    Some(ext) if ext.eq_ignore_ascii_case("csv")  => parse_csv(&contents, offset, event_id.clone())?,
    Some(ext) if ext.eq_ignore_ascii_case("json") => parse_json(&contents, offset, event_id.clone())?,
    _ => return Err("Unsupported file type (must be .csv or .json)".into()),
};
```

The "Failed to parse" error path and its "Press Enter to proceed"
prompt is shared by both branches — same UX whether the parse failure
came from CSV or JSON.

### The new `From` impl in `uwh-common`

Add this immediately below the existing
`impl From<Schedule> for SendableSchedule` at
[schedule.rs:505](../../../uwh-common/src/uwhportal/schedule.rs#L505):

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

Why this shape:
- `(SendableSchedule, EventId)` as the input type bundles the two
  pieces of information needed to construct a `Schedule`. `From<T>`
  is the idiomatic Rust conversion trait — no need to invent a
  custom constructor.
- The compiler will require that every field on `Schedule` is named
  explicitly here. If `Schedule` (or `SendableSchedule`) grows a
  new field in the future, this impl will fail to compile until
  it's updated — exactly the safety net we want.
- It's symmetric with the existing `From<Schedule> for SendableSchedule`
  conversion; both live in the same file, next to each other.

### The `parse_json` function

Lives in a new file `schedule-processor/src/json_loader.rs`. Public
signature symmetric to `parse_csv`:

```rust
pub fn parse_json(
    json: &str,
    _offset: UtcOffset,
    event_id: EventId,
) -> Result<Schedule, Box<dyn std::error::Error>>;
```

Internally, it's now just two lines:

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

1. `serde_json::from_str::<SendableSchedule>(json)?` — deserialize.
   The existing `Deserialize` derive on `SendableSchedule` does all the
   field-name and shape validation; we get rich error messages
   ("missing field `standingsOrder` at line 1247 column 3") for free.
2. `(sendable, event_id).into()` — invoke the new `From` impl in
   `uwh-common`, which handles the `Vec<Game> → IndexMap<GameNumber, Game>`
   shape change and `event_id` injection.

The `offset` parameter is accepted for signature symmetry with
`parse_csv` but ignored: the JSON timestamps are already
`OffsetDateTime` values in UTC (`Z`-suffixed), and the internal
`Schedule` is happy to hold UTC times. All comparisons in
`schedule_checks` work the same way regardless of offset, and the
upload step re-serializes the same UTC times the portal already
expects.

### What stays the same downstream

Once `parse_json` returns a `Schedule`, the rest of `main.rs` runs
unchanged:

- Schedule summary print (game count, group names, timing-rule
  names, courts, unassigned teams)
- `run_schedule_checks(&schedule)`
- The `--allow-failures` flag still gates whether check failures
  abort
- Interactive team-name → portal-team-ID mapping
- Save schedule / print schedule / upload schedule / save team map /
  print team map menu loop
- Upload via `portal_client.push_event_schedule(...)`

---

## Acceptance criteria

A non-programmer reviewer should be able to confirm all of:

1. **Run the schedule-processor.** When prompted for a file, the
   dialog now offers both `.csv` and `.json` extensions in the filter.
2. **Pick the Australian-tournament JSON** (committed as a mock
   fixture). The tool parses it successfully and prints the schedule
   summary (71 games, 12 groups, 2 timing rules, 2 courts).
3. **Schedule checks pass.** `run_schedule_checks` runs and reports
   no errors against this JSON.
4. **Team mapping works.** The interactive team-mapper offers to
   map the 40-ish team names from the JSON to the portal's event
   teams.
5. **Save the schedule.** The "Save Schedule to File" option
   produces a JSON file that is structurally equivalent to the
   input (round-trip preserved for `standingsOrder`,
   `finalResultsOrder`, groups, games, timing rules).
6. **Pick a CSV instead.** The existing CSV path still works
   identically — no regression.
7. **Downstream crates still compile.** After the `uwh-common`
   `From` impl is added, `cargo check -p refbox -p schedule-processor
   -p overlay -p led-panel-sim` and `cargo build -p uwh-common
   --no-default-features` all succeed unchanged.
8. **`just check` passes.**

---

## Error handling

| Scenario | Behaviour |
|---|---|
| File extension is neither `.csv` nor `.json` | Friendly error: "Unsupported file type (must be .csv or .json)"; show "Press Enter to proceed" prompt; exit. |
| JSON is structurally invalid (e.g. missing required field, wrong type) | `serde_json` error surfaced via the same `error!("Failed to parse JSON file: {e}")` + "Press Enter to proceed" path used by CSV failures. Error message includes the JSON path that failed. |
| JSON has a `.json` extension but content is actually CSV (or vice versa) | We branch on extension, not content-sniffing — so this falls through to the "Failed to parse" path with a JSON-shaped error message. Telling the user "your `.json` file looks like CSV" is more useful than silently guessing. |
| `nonGameEntries` is empty (as in the Australian JSON) | Allowed; logged at info level as "Schedule has 0 non-game entries" so it's visible but not blocking. The operator simply won't see warm-up / break rows on the day. |
| `run_schedule_checks` fails | Same as today: error printed; aborts unless `--allow-failures` is set. |
| Portal event not selected before file load | Not possible — file-selection step runs *after* event selection in `main.rs`. The `event_id` is always available when `parse_json` is called. |

---

## Testing strategy

### Unit test — `uwh-common/src/uwhportal/schedule.rs`

Inline test for the new `From<(SendableSchedule, EventId)> for Schedule`
impl, added next to the existing schedule-test module:

- **Round-trip.** Construct a `Schedule`, convert to
  `SendableSchedule`, convert back via the new `From` impl with the
  same `EventId`. Assert equality with the original `Schedule`.
  This is the safety net that catches drift if either type grows a
  new field.

### Unit tests — `schedule-processor/src/json_loader.rs`

Inline `#[cfg(test)]` module at the bottom of the file. Three tests:

1. **Happy path.** A small synthetic JSON (2 games, 1 group, 1 timing
   rule, minimal `standingsOrder` and `finalResultsOrder`) parses
   into a `Schedule` with the expected fields populated.
2. **`event_id` injection.** The `EventId` passed to `parse_json`
   lands on the resulting `Schedule.event_id`.
3. **Missing required field.** A JSON with `games` omitted produces
   a `serde_json::Error`; we assert the error message mentions the
   field name.

### Integration test — `schedule-processor/tests/json_real_world.rs`

A new integration test (or inline in `tests/` if there's no existing
folder — currently there isn't, so this creates `tests/`). It:

1. Loads the committed `2026 Australian Nationals - portal export.json`
   mock fixture.
2. Calls `parse_json` with a placeholder `EventId` constructed via
   `EventId::from_partial("test-event")` (a value the schedule checks
   never inspect — they only look at fields *inside* the parsed
   `Schedule`).
3. Calls `run_schedule_checks` on the result.
4. Asserts no errors.

This is the strongest validation that meaningful checks run on
JSON-loaded schedules: the *entire* validation pipeline against a
real-world tournament's worth of data.

### Existing coverage

`csv_parser.rs`, `schedule_checks.rs`, and the `uwh-common` schedule
types are untouched, so their existing tests cover the unchanged
behaviour without any modification.

### `just check`

Must remain green: `cargo fmt --check`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo test`, `cargo
audit`. No new dependencies introduced (already using `serde_json`
transitively via `uwh-common`).

---

## Out of scope (explicit)

- **No offline-schedule loading for refbox.** Refbox still fetches
  from the portal at runtime. This change is purely about getting
  the JSON *into* the portal.
- **No new schedule checks.** The 12 existing checks in
  `schedule_checks.rs` are sufficient; both CSV and JSON loaders
  feed them the same `Schedule` value.
- **No CSV-format changes.** Existing CSV files continue to work
  unchanged.
- **No scoresheet PDF generation.** Currently triggered separately;
  not affected by this change.
- **No new types or wire-format changes in `uwh-common`.** The only
  `uwh-common` touch is a purely additive `From` impl symmetric to
  an inverse impl that already exists in the same file. No new
  structs, no field renames, no `Serialize`/`Deserialize` changes.
  The JSON already matches `SendableSchedule` exactly; recent commits
  (`f3b05ba9 fix(uwh-common): use GroupReference for schedule order
  fields ...`) already align the wire format with what the portal
  sends, and the Australian JSON uses that shape.
- **No `nonGameEntries` synthesis.** The Australian JSON has none.
  We pass that through as-is; if the organizer wants warm-up rows
  shown to the operator, they would add them to the JSON
  themselves. Not our problem to invent them.

---

## Open questions

None at this point. All design questions raised during the
brainstorming phase have a concrete answer above. If new questions
surface during planning, they'll be added to the implementation
plan's Deviations section per `.claude/rules/plan-execution.md`.

---

## References

- `schedule-processor/src/main.rs` — CLI entry point (file dialog,
  event selection, parse-then-validate flow).
- `schedule-processor/src/csv_parser.rs` — existing CSV parser whose
  public signature we mirror.
- `schedule-processor/src/schedule_checks.rs` — the 12 validation
  checks that run on every `Schedule`, regardless of input source.
- `uwh-common/src/uwhportal/schedule.rs:492` — `SendableSchedule`
  definition (the wire format the JSON conforms to).
- `uwh-common/src/uwhportal/schedule.rs:204` — `GroupReference`
  definition (the `{name: "..."}` shape used in `standingsOrder` and
  `finalResultsOrder`).
- `f3b05ba9 fix(uwh-common): use GroupReference for schedule order
  fields to match portal wire format` — recent commit aligning the
  wire format with what the portal accepts.
- `0c0b87c4 feat(schedule-processor): parse and populate standings_order
  and final_results_order from CSV` — companion commit on the CSV
  side; the JSON path inherits the same fields via deserialization.
