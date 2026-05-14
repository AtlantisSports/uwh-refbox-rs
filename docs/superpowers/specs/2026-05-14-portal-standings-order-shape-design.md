# Portal Standings-Order Shape — Design Spec

> **STATUS: SUPERSEDED (2026-05-14).** This design was written before discovering that an equivalent fix already exists on the local branch `feat/schedule-processor/csv-display-order` (commits `19aa3b4`, `dd01718`, `0c0b87c`, `41cca0a`, `079f3c1`, `165a803`). The existing implementation uses a narrower `GroupReference { name: String }` (name-only, with the comment that the portal server-side resolves identifier from name via `IReferenceWithNameAsPrimaryKey.TryFillIdentifier`) instead of the `{identifier, name}` shape proposed below. The existing branch also covers schedule-processor CSV parsing, which this design does not. **Refer to that branch for the actual implementation.** This document is preserved as a record of the parallel brainstorm.

---

**Date:** 2026-05-14
**Scope:** `uwh-common/src/uwhportal/schedule.rs` (wire-format types)
**Origin:** Surfaced during Unit 5 audit (referee names display) walkthrough on 2026-05-13/14, when refbox crashed deserializing the production schedule.

## Context

The portal's `Schedule` JSON object includes `standingsOrder` and `finalResultsOrder` fields. Refbox's `uwh-common` crate models both as `Option<Vec<usize>>`, but production now emits them as arrays of `{identifier, name}` objects:

```json
"standingsOrder": [
  {"identifier": "17f8b454-e4bd-49bb-99b1-4c35fe25afaa", "name": "Pod A Round Robin"},
  {"identifier": "f231bb1b-...", "name": "..."}
]
```

This mismatch makes refbox unusable against production: every schedule fetch fails with `Failed to get schedule: invalid type: map, expected usize at line 1 column 40773`.

**Authoritative source:** the C# portal backend at `/home/estraily/projects/uwh-portal/base/Events/Scheduling/EventSchedule.cs:20` defines these fields as `GroupReference[]?`, where `GroupReference` carries `Identifier` and `Name`. The portal evolved its schema; refbox's Rust types did not catch up. The uwh-portal web client's TypeScript types are also out of date (`number[]?` in `js/@underwater-web/lib/refbox/uwhPortalClient.ts:63-64`), but TypeScript coerces silently while Rust's serde fails strictly.

**Confirmed scope of mismatch:** the affected fields are PARSED but NOT READ anywhere in the workspace. Verified via `grep -rn "standings_order|final_results_order|standingsOrder|finalResultsOrder"` across `refbox/`, `uwh-common/`, `schedule-processor/`, `overlay/`. Only struct definitions, copy operations in the `From<Schedule> for SendableSchedule` impl, and `None` defaults in test fixtures and `csv_parser.rs`. The fix can be deserialization-only.

## Decision

Define a new public struct `GroupReference { identifier: String, name: String }` in `uwh-common/src/uwhportal/schedule.rs` and change the four affected field types from `Option<Vec<usize>>` to `Option<Vec<GroupReference>>` on both `Schedule` and `SendableSchedule`. No consumer logic changes — the fields remain parsed-but-unused.

**Approach chosen:** A — new named struct. Alternatives considered and rejected:
- **B — Custom serde helper that exposes only one field (identifier or name):** loses information at the type boundary; future consumers can't recover the other field without re-parsing; mirrors the C# backend less faithfully.
- **C — Tuple `(String, String)` or type alias:** tuple positions lose self-documenting names; less idiomatic for a well-defined wire-format shape.

The new struct's serde-default forward-tolerance handles future portal-side additions (e.g., a third field like `slug`) without code changes. `#[serde(deny_unknown_fields)]` was considered and rejected — it would create exactly the brittleness this fix exists to address.

## Design

### Section 1 — Architecture

**Location:** `uwh-common/src/uwhportal/schedule.rs`, declared near the existing `Group` struct (~line 462) for cohesion with the other named portal types.

**Definition:**

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupReference {
    pub identifier: String,
    pub name: String,
}
```

**Derives:** `Clone, Debug, PartialEq, Eq` matches the surrounding pattern (every other named-data struct in this file derives the same set). `Serialize, Deserialize` because both directions are needed — `SendableSchedule` is uploaded to the portal via `push_event_schedule`.

**no_std:** `String` is available via `alloc::string::String`. uwh-common already uses `String` throughout in no_std mode. No new dependencies; no `std`-only features.

**API surface impact:** adds one new public type to `uwh_common::uwhportal::schedule`. Standard crate convention.

### Section 2 — Field changes

Four type signature edits in `uwh-common/src/uwhportal/schedule.rs`:

**In `Schedule` struct (~line 484):**
```rust
// Before:
pub standings_order: Option<Vec<usize>>,
pub final_results_order: Option<Vec<usize>>,

// After:
pub standings_order: Option<Vec<GroupReference>>,
pub final_results_order: Option<Vec<GroupReference>>,
```

**In `SendableSchedule` struct (~line 521):** identical change to the same two field types.

**`From<Schedule> for SendableSchedule` impl (~line 541-542):** no change — `schedule.standings_order` and `schedule.final_results_order` already pass through as opaque values; the type propagates.

**Other consumers** (`csv_parser.rs:152-153`, test fixtures at `schedule.rs:1216-1217`): no change — they all use `None`, which is type-agnostic and continues to type-check.

**Net diff:** ~5 lines added (the new struct definition) + 4 field type lines modified; no field deletions. Approximately 9 lines of total change in this file.

### Section 3 — Tests

One new serde roundtrip test in the `mod tests` block of `uwh-common/src/uwhportal/schedule.rs`:

```rust
#[test]
fn test_group_reference_serde_roundtrip() {
    // Production-shape: standingsOrder is an array of {identifier, name} objects.
    // Confirmed against /api/events/2081-A/schedule/privileged on 2026-05-14.
    let json = r#"[{"identifier":"abc-123","name":"Pod A"},{"identifier":"def-456","name":"Pod B"}]"#;
    let refs: Vec<GroupReference> = serde_json::from_str(json).unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].identifier, "abc-123");
    assert_eq!(refs[0].name, "Pod A");
    assert_eq!(refs[1].identifier, "def-456");
    assert_eq!(refs[1].name, "Pod B");

    // Roundtrip preserves the shape exactly.
    let back = serde_json::to_string(&refs).unwrap();
    assert_eq!(back, json);
}
```

**Test rationale:**
- Locks the wire format: a future change that breaks `[{identifier, name}, ...]` parsing surfaces as a test failure.
- Covers serialization too — `SendableSchedule` is uploaded back to the portal via `push_event_schedule`.
- Does NOT test serde's default tolerance of unknown extra fields (e.g., `{"identifier":"x","name":"y","slug":"z"}`); that would be testing serde, not our code.

Existing test fixtures (`schedule.rs:1216-1217`) use `None` and need no update.

### Section 4 — Verification + risk

**Verification steps (in order):**

1. **`just check`** workspace-wide from the new worktree — expected clean, modulo the two pre-existing dep advisories (RUSTSEC-2026-0002 in `lru` via `iced_wgpu`, RUSTSEC-2025-0035 in `macroquad` via `overlay`).
2. **`cargo build -p uwh-common --no-default-features`** — confirms the new struct compiles in no_std mode (per `uwh-common/CLAUDE.md`'s Downstream Impact Checklist).
3. **Production-JSON parse check:** run `cargo test` against a one-off test that reads the captured `/tmp/prod-schedule.json` from disk and calls `serde_json::from_str::<Schedule>(...)`. Test asserts deserialization succeeds. Test gated behind `#[ignore]` so it doesn't run in CI (it depends on a local file) but can be invoked explicitly via `cargo test --ignored test_production_schedule_parses`. If the captured JSON file isn't present, the test is skipped at run time.
4. **Launch refbox against production** (config at `~/.config/refbox/default-config.toml` already pointed at `https://api.uwhportal.com` with a valid token) — confirm the log shows `Got schedule` and no `Failed to get schedule: invalid type...` error. Then load an event and verify the Court field populates.

**Risk / blast radius:**

- **Refbox ↔ overlay wire format:** `SendableSchedule` is uploaded back to the portal via `push_event_schedule` (uwh-common/src/uwhportal/mod.rs:445). The shape change is symmetric — what the portal sends, we send back unchanged. Overlay doesn't read these fields (grep-confirmed). Safe.
- **schedule-processor CSV import:** sets `standings_order: None` (csv_parser.rs:152). Type change preserves None compatibility. Safe.
- **wireless-remote:** doesn't use uwh-common's `Schedule` type at all. Out of scope; spot-check via grep before final declaration.
- **uwh-common no_std build:** `GroupReference` only uses `String` (already in scope via `alloc::string::String`). No new dependencies.

**The only realistic failure mode:** if production's `standingsOrder` shape evolves AGAIN after this lands (e.g., introduces a deeper restructure). Serde's default tolerance handles flat additions but not nested restructure. Future portal change → future fix branch.

## Out of scope

- Reading any value from `standings_order` / `final_results_order` anywhere. The fields remain parsed-but-unused, same as today.
- Adding `identifier` to the existing Rust `Group` struct. The portal's C# `Group` likely has an `Identifier` field that the Rust `Group` does not model — a pre-existing wire-format gap. Flag as Findings-Backlog candidate; not addressed here.
- Touching Unit 5's audit branch (`audit/refbox/referee-names`). That work stays paused until this lands; the standings-order fix is independent.
- Updating the uwh-portal web client's TypeScript types. Their bug; their fix.

## References

- **Production JSON evidence:** captured by curl on 2026-05-14 to `/tmp/prod-schedule.json` (41 KB). The new shape appears at column 40773.
- **C# backend (authoritative):** `/home/estraily/projects/uwh-portal/base/Events/Scheduling/EventSchedule.cs:20` defines `GroupReference[]? StandingsOrder` and `GroupReference[]? FinalResultsOrder`. The `GroupReference` type usage at `EventScheduleCalculationContext.cs:405` confirms the `{Identifier, Name}` shape.
- **TypeScript web client (out of date):** `/home/estraily/projects/uwh-portal/js/@underwater-web/lib/refbox/uwhPortalClient.ts:63-64` still claims `number[]?`. TypeScript coerces silently so the web client doesn't crash.
- **uwh-common conventions:** `uwh-common/CLAUDE.md` mandates the no_std build check and the downstream consumer verification list.
