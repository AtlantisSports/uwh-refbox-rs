# Single-period flag, and no zero-length durations

**Date:** 2026-09-11
**Status:** Design approved by Eric 2026-09-11. Building now on `feat/uwh-common/single-period-flag`,
to be held unmerged until the Portal's `singlePeriod` lands.
**Crates:** `uwh-common` (contract + conversion), `schedule-processor` (validation), `refbox`
(remove the workaround). Branch scope is `uwh-common` — the broadest crate involved.

---

## The rules this implements

Eric ruled, first-hand, on 2026-09-11:

1. **No time duration may be zero.** Every duration always carries a real number, and **the on/off
   setting alone decides whether it is used** — never the value.
2. **`suddenDeathAllowed` is a separate parameter from overtime.** It has no bearing on whether
   overtime is allowed, nor on the overtime half or overtime half-time durations. The only
   relationship is ordering: overtime, then the pre-sudden-death break, then sudden death.
3. **Rule 1 covers the half-time break.** A zero half-time is the same magic-value pattern — a
   duration doing a flag's job — so **single period gets its own explicit flag**.

Rule 3 is the one that makes this a contract change rather than a local fix.

## Goal

Make a zero-length duration impossible to author, and give single-period games an honest way to
say what they are.

## Scope boundary

**In scope:** the Portal timing-rule contract gains `singlePeriod`; `uwh-common` reads it instead
of inferring it; `schedule-processor` gains duration validation it does not currently have;
`normalize_degenerate_overtime` is deleted from `refbox`.

**Explicitly out of scope:** refbox's game engine and clock behaviour beyond deleting that one
function; refbox's settings screen (already compliant, see below); anything on the Portal side —
that is the other team's branch; the separate open defect where a zero *playing* duration panics
past the config guard (`docs/backlog/zero-length-period-panics-past-the-config-guard/`).

---

## Current state — what is already right, and what is not

Verified against `origin/master` on 2026-09-11. (Read via `git show origin/master:<path>` — the
main checkout is routinely tens of commits stale.)

| Entry point | Complies with rule 1? | Evidence |
|---|---|---|
| refbox settings screen | **Yes, already** | `MIN_PERIOD_LENGTH = 1s` and `param_length_too_short` (`refbox/src/app/view_builders/configuration.rs:1892,1916`) constrain every length parameter; `GameBlock` is excluded only because `game_block_validity` is stricter and already refuses zero |
| refbox single-period selection | **Yes, already** | An explicit operator toggle — `AppState::ParameterEditor(param, dur, single_half)`, committed on Apply (`refbox/src/app/mod.rs:5647`), labelled `two-halves` / `one-period` ("2 HALVES" / "1 PERIOD") |
| Portal → refbox | **No** | `uwh-common/src/uwhportal/schedule.rs:323` infers the flag: `single_half: half_time_duration == Duration::ZERO` |
| `schedule-processor` | **No** | No duration validation of any kind — no `is_zero` or `Duration::ZERO` anywhere in its source |
| `normalize_degenerate_overtime` | **No** | `refbox/src/tournament_manager/mod.rs:1323` lets a duration override a flag |

The important consequence: **refbox already has the concept.** Only the wire contract conflates the
flag with the value. This is a contract fix, not a new feature in the app.

## The contract change

`TimingRule` gains one boolean:

```
"singlePeriod": true
```

Named from Eric's own operator vocabulary ("1 PERIOD"), and consistent with the existing booleans
`overtimeAllowed`, `suddenDeathAllowed`, `teamTimeoutsCountedPerHalf`. The portal session has no
objection but cannot ratify it; **core sign-off governs, and that gate is unstarted.**

**Semantics.** `singlePeriod: true` means one playing period and no half-time break.
`halfTimeDuration` still carries a real positive number that simply goes unused — exactly parallel
to a positive `overtimeHalfPlayDuration` sitting unused when `overtimeAllowed` is false.

**Deserialisation.** `#[serde(default, rename = "singlePeriod")]`, defaulting to `false`. A missing
field therefore means "two halves", which is correct: Eric confirms no current event is a
single-period game, so nothing is misread during the changeover.

---

## Changes by crate

### `uwh-common`

- Add `single_period: bool` to `TimingRule` (`src/uwhportal/schedule.rs`).
- In the `TimingRule -> GameConfig` conversion, replace
  `single_half: half_time_duration == Duration::ZERO` with `single_half: single_period`.

This is the highest-blast-radius crate in the workspace. Per `.claude/rules/workspace.md`, every
dependant is checked after the change: `refbox`, `schedule-processor`, `overlay`, `overlay-bridge`,
`led-panel-sim`, `matrix-drawing`.

### `schedule-processor`

- Reject any zero duration when building a schedule, naming the timing rule and the setting.
  **Blocks the build; not a warning** — Eric's requirement is that the builder *cannot* pass a zero.
  The fields, exhaustively: `halfPlayDuration`, `halfTimeDuration`, `teamTimeoutDuration`,
  `overtimeHalfPlayDuration`, `overtimeHalfTimeDuration`, `preOvertimeBreak`,
  `preSuddenDeathDuration`, `minimumBreak`, and `gameBlock` **when present** (it is optional, and
  already has a stricter rule of its own that refuses zero as a special case). `teamTimeoutCount`
  is a count, not a duration, and is out of scope — rule 1 is about durations.
- Accept `singlePeriod` from both authoring paths: the CSV `Timing Rule Field` column and the JSON
  loader.
- Add it to `TIMING_RULE_FIELDS`. The existing test `timing_rule_field_names_match_the_type`
  (`src/csv_parser.rs`) asks `TimingRule` what it serialises, so it fails loudly if the field is
  added to the type but not to the accepted list.
- Update the fixtures that currently encode the old convention:
  `src/json_loader.rs` (`HAPPY_PATH_JSON`), `tests/fixtures/portal-schedule-with-finals.json`,
  and the round-trip JSON in `uwh-common/src/uwhportal/schedule.rs`.

### `refbox`

- Delete `normalize_degenerate_overtime`, its two call sites
  (`src/tournament_manager/mod.rs:1379` and `:1539`), and its tests.

**Both call sites operate only on Portal-sourced timing rules.** It never sees a manually built
config, so refbox's settings-screen floor is not the guard that replaces it — the Portal's
rejection of zeros is. It therefore comes out **last**, and only once zeros are genuinely
impossible to author.

---

## Sequencing

Eric's call: **both sides build in full, in parallel, each assuming the other finishes, and land
together.** Our branch is built against `singlePeriod` as though it exists, held unmerged, and
landed when the Portal's change is ready.

The ordering constraint that drives this: until `singlePeriod` exists, a zero half-time is the only
way to express a single-period game. Rejecting it first would make single-period schedules
unbuildable — and Eric requires single-period games to work from an uploaded Portal schedule, not
only from refbox's local settings.

**Accepted risk, stated explicitly:** core sign-off for the contract field is unstarted and
undated. We are building against an unratified contract on Eric's instruction, reaffirmed after the
risk was put to him twice. If core changes the name or shape, it is cheap to follow now and
expensive once both sides are built.

**Portal-side state at the time of writing:** the portal session has **not** started and will not
act on an instruction relayed through this session — building a contract field ahead of core
sign-off requires core's approval at their tier, and the product owner's approval does not
substitute for it. Eric is directing them himself. Our build does not depend on theirs existing,
only on the field name surviving core.

---

## Acceptance criteria

Things Eric can observe, without reading code:

1. Building a schedule with any duration set to zero **fails**, and the message names the timing
   rule and the setting.
2. Building a schedule marked single-period **succeeds** with an ordinary positive half-time
   length sitting unused.
3. A single-period game loaded from a Portal schedule shows as a single period at the pool — one
   playing period, no half-time break.
4. A game with overtime switched off still loads and plays correctly with a positive overtime
   length present.
5. `just check` passes; every crate depending on `uwh-common` still builds.

---

## Testing

- One test per duration field proving zero is refused by the builder.
- A test proving `singlePeriod: true` reaches refbox as a single-period game, and that a missing
  field yields two halves.
- `timing_rule_field_names_match_the_type` already guards the contract field list.
- `refbox/src/tournament_manager/zero_probe.rs` drives all combinations of seven zeroed durations
  and asserts no tick crashes or hangs; its `MAY_FAIL` allowlist is `[half_play, ot_half_play]`.
  **Re-check it after `normalize_degenerate_overtime` is deleted** — that function is currently
  upstream of some of those paths, and its removal may change what the probe exercises.

---

## Open items

1. **Provenance of the zero-overtime shape — unresolved, does not block.** Eric's account is that
   it only ever came from a hand-built set of game parameters. The code points at Portal data: both
   call sites are Portal-only, the test is labelled "the FINALS rule", there is a
   `finals_timing_rule()` helper, and `portal-schedule-with-finals.json:1530` carries exactly that
   shape. The two reconcile if the hand-building happened in the Portal's own timing-rule editor.
   Raised with Eric in writing; unresolved. It does not block, because the workaround is removed
   only after zeros are impossible. The portal session has been asked to check production for
   FINALS-type rules with `overtimeAllowed: true` and a zero `overtimeHalfPlayDuration`.
2. **Field name not ratified.** `singlePeriod` is the working name pending core.
3. **No migration planned**, resting entirely on Eric's confirmation that he controls all schedules
   and none carry the zero shape. If that turns out to be wrong, the failure is silent: existing
   schedules begin failing validation on their next save.
