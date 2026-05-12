# ADR 020 — ListOfPlacements + SeededBy.group

**Status:** Accepted (retroactive)
**Date:** 2026-05-13
**Audit unit:** 2 — ListOfPlacements + SeededBy
**Audit PR:** *(to be filled at Final Integration)*

## Context

The UWH Portal API gained a `ListOfPlacements` tournament format for seeded
group playoffs, in which individual final placings can reference either a
specific game result or a seeded position — and seeds in this format can span
groups, meaning a seed reference may carry no group name at all. To represent
this on the refbox side, the schedule type needed a new record shape
(`FinalPlacingSource`), a new variant in the existing `FinalResults` enum
(`ListOfPlacements`), and a wire-format change to `SeededBy.group` (now
`Option<String>` instead of `String`) so that nameless cross-group seeds are
representable.

The fix shipped in commit `6907ef8` (2026-04-11), with a downstream refbox
display fix in `803d985` (2026-04-18). This ADR records what survived the AI
Code Audit on 2026-05-13.

## Decision

The following behaviours are kept in the codebase, audited 2026-05-13.

### 1. New `FinalPlacingSource` type (B2.1)

A new data shape in
[uwh-common/src/uwhportal/schedule.rs](../../uwh-common/src/uwhportal/schedule.rs)
describes how a single tournament placing is determined: either a game result
(`result_of`) or a seeded position (`seeded_by`), both fields optional and
both omitted from JSON when absent. The type permits four states (both set,
only one set in either combination, or neither set), but only the two
"exactly one Some, the other None" states are semantically valid. This is a
**known invariant the consumers must respect**: the looseness matches the
portal's wire contract (two parallel optional fields rather than a tagged
enum), not a defect of the type, but consumers are responsible for treating
"both Some" or "both None" as invalid input.

Verification: round-trip serde tests for the type were added in Task 6.2 (audit
commit `f97d946`), confirming that JSON shape is preserved across serialize
then deserialize for the valid forms.

### 2. `FinalResults::ListOfPlacements` variant (B2.2)

A third way the tournament rules can describe how final standings are
determined, alongside the existing `ListOfGames` and `Standings` variants. The
new variant carries a `Vec<FinalPlacingSource>` — a sequence of individual
placing records — so the rules can mix game-result placings and seeded-group
placings in one block.

Verification: serde round-trip tests added in Task 6.2 (commit `f97d946`)
confirm the variant serializes and deserializes correctly. The schedule-processor
downstream consumer arm (kept behaviour #7 below) confirms reachability by
static analysis: the variant is consumed in `check_final_results`.

**Real-portal-data reachability is unverified** — see "What was not verified"
below.

### 3. `SeededBy.group` is now `Option<String>` (B2.3)

A wire-format change. In plain English: a seed reference can now belong to "no
group", which represents the cross-group placement entries used in
`ListOfPlacements`. The serde shape behaves as follows:

- When the group is `Some("...")`, the JSON shape is unchanged from before
  (`"group":{"name":"..."}`).
- When the group is `None`, the field is omitted entirely from the JSON
  (`skip_serializing_if = "Option::is_none"`).
- When deserializing JSON that has no `group` field, `#[serde(default)]`
  produces `None`; this means newer clients reading older payloads (which
  always included `group`) will see `Some(...)`, and older clients reading
  newer payloads (which may omit `group`) will see `None`. No breakage in
  either direction.

Verification: the existing `test_serialize_seeded_by` and
`test_deserialize_seeded_by` tests in
[uwh-common/src/uwhportal/schedule.rs](../../uwh-common/src/uwhportal/schedule.rs)
still pass, confirming round-trip preservation for the populated case. The
new `ListOfPlacements` round-trip tests added in Task 6.2 (`f97d946`)
exercise the omitted-group case end-to-end.

### 4. `option_item_name` serde helper (B2.4)

A 28-line private serde module in
[uwh-common/src/uwhportal/schedule.rs](../../uwh-common/src/uwhportal/schedule.rs)
that parallels the existing `item_name` helper but handles `Option<String>`
instead of `String`. Kept as-is because the parallel pattern matches the
surrounding file's serde style. Consolidating both helpers into a single
generic implementation is a clean refactor opportunity but a separate concern
from this audit; logged as a Findings Backlog candidate for a future refactor
branch.

### 5. `single_half` detection fix in `TimingRule::into()` (B2.5)

A real bug fix in
[uwh-common/src/uwhportal/schedule.rs](../../uwh-common/src/uwhportal/schedule.rs):
the previous check `half_play_duration == Duration::ZERO` could never be true
for any real game (a game must have non-zero play time), so single-half games
(games configured with no halftime break) were never recognized. The fix
checks `half_time_duration == Duration::ZERO` — the actual signal that no
halftime exists.

**Scope note:** this fix was bundled inside a feature commit (`6907ef8` for
`ListOfPlacements`), violating the project's "one branch = one concern" rule
in `.claude/rules/scope.md`. The bundling is flagged for the Process
refinements log.

Verification: paired regression tests added in Task 6.7 (audit commit
`6a6b49a`) — `test_timing_rule_single_half_when_no_halftime_break` and
`test_timing_rule_two_halves_when_halftime_break_present`. The first fails if
the fix is reverted to `half_play_duration == Duration::ZERO`, panicking on
the "single_half should be true" assertion — the exact signature of the
original bug.

### 6. `schedule-processor` `csv_parser` handles optional group (B2.6)

A mechanical downstream consumer change in
[schedule-processor/src/csv_parser.rs](../../schedule-processor/src/csv_parser.rs)
required to absorb the `SeededBy.group: Option<String>` change. Both call
sites that walk seeded-by entries now pattern-match `group: Some(group)` so
that nameless seeds are pass-through (the CSV group-name remapping step
leaves them alone — there is nothing to remap).

Verification: `cargo test -p schedule-processor` clean.

### 7. `schedule-processor` `schedule_checks` handles optional group + new `ListOfPlacements` arm (B2.7)

Two related changes in
[schedule-processor/src/schedule_checks.rs](../../schedule-processor/src/schedule_checks.rs):

- Optional-group handling: the seed-reference walks in the validator now
  pattern-match `group: Some(group)`, silently skipping nameless seeds. This
  is mechanical absorption of the wire-format change.
- A new `ListOfPlacements` arm in `check_final_results`, currently a no-op
  with a comment explaining that cross-group placement validation is not
  performed here.

The validation gap is deliberate: the existing validator is single-group-aware,
and closing the gap properly would require a different validator pass that
understands cross-group references. The arm's explanatory comment makes the
trade-off visible in the code rather than silently ignoring the variant.
Logged as a Findings Backlog candidate — close when the first real
placements-using tournament surfaces a validation case the current code
misses.

### 8. `refbox` `shared_elements::get_team_name` "Unknown" fallback (B2.8)

A mechanical downstream display fix in
[refbox/src/app/view_builders/shared_elements.rs](../../refbox/src/app/view_builders/shared_elements.rs)
to absorb the `SeededBy.group: Option<String>` change. Nameless seeds render
as `Seed N of Unknown`, mirroring the existing "Unknown" fallback in
`uwh-common`'s `ScheduledTeam::Display` impl. Keeping the literal consistent
across both display sites is the right call; translating it would have to
touch both sites together, which is a cross-crate concern logged as a
Findings Backlog candidate, not pursued during this audit.

Verification: `cargo test -p refbox --bin refbox` clean (83 tests pass).

## Consequences

- Tournament organisers can configure placements-style playoffs (seeded group
  brackets where placings reference game results AND group seeds in the same
  standings block).
- The wire format for `SeededBy.group` is now a permanent "field optional
  with elision when absent" contract; future consumers can rely on it. Older
  clients reading new payloads with `group` absent will see `None` — graceful,
  not a crash.
- The `single_half` game configuration is now actually detectable; tournament
  configurations using a single play period (no halftime break) work as
  designed.
- Future refactors must not regress the defensive `Option::is_some` matches
  in `schedule-processor` (#6, #7) or the `as_deref().unwrap_or("Unknown")`
  display fallback (#8). The Task 6.7 regression tests catch the
  `single_half` part, and the Task 6.2 round-trip tests catch the wire-format
  part — but the consumer-side mechanical absorbs (#6, #7, #8) are not
  test-covered beyond compilation.
- The "both Some / both None" invariant on `FinalPlacingSource` is an
  unconstrained state in the type — consumers must check, the type does not
  prevent it.

## What was removed during audit

Nothing was removed during this audit. Every behaviour in the original commits
(`6907ef8` and `803d985`) was reviewed in Task 4 and decided `@user_verified`.
The audit added four new tests (two for `single_half` regression coverage,
two for `ListOfPlacements` serde round-trip coverage) but did not delete any
code or documentation from the original commits.

## What was not verified

Real-portal-data reachability of the `ListOfPlacements` variant was not
verified during this audit (Task 6.3 skipped). The audit confirmed:

- (a) the type compiles in every consumer of `uwh-common`;
- (b) serde round-trip preserves shape for all three documented
  placement-source forms;
- (c) the schedule-processor downstream arm exists and compiles.

The audit did NOT confirm whether any real portal export currently emits a
`ListOfPlacements` block. If a future tournament uses this format and
surfaces an unexpected schedule shape, revisit. This is tracked as a Findings
Backlog item rather than a known defect because the audit did not show
evidence of a problem — only of a missing exercise.

## Audit reference

- Audit branch: `audit/uwh-common/list-of-placements` (cut 2026-05-12 from
  `origin/master` in worktree `.worktrees/audit-unit-2-list-of-placements/`)
- Audit PR: *(to be filled at Final Integration)*
- Original commits:
  - `6907ef8` 2026-04-11 — `feat(uwh-common): add FinalResults::ListOfPlacements and fix SeededBy.group type`
  - `803d985` 2026-04-18 — `feat(refbox): handle Option<String> group in SeededBy display`
- Audit-branch commits added during Unit 2:
  - `f97d946` — `test(uwh-common): add serde round-trip tests for ListOfPlacements variant`
  - `6a6b49a` — `test(uwh-common): add regression test for single_half detection fix`
