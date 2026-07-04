# Referee/Official Names for Team-Assigned Portal Events — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the refbox game-info table show real official names when a Portal event assigns referees by team, and fix two role-string mismatches so individual officials (including the time/score helper) also resolve.

**Architecture:** One additive field (`team_id`) on the shared `RefereeAssignment` type lets refbox see team assignments. The render builder (`referee_rows`) gains the `teams` map (already in scope) and switches between two fixed layouts — 6 individual rows or 2 team rows (Water Referees / Deck Referees) — chosen by whether the game carries the team-only `Referees` role. Team names resolve from the `TeamList` exactly as the White/Black team names already do.

**Tech Stack:** Rust 2024, `serde`, `i18n-embed-fl` (`fl!` compile-checks FTL keys against en-US), `iced` 0.13.

**Spec:** `docs/superpowers/specs/2026-06-18-referee-team-names.md`

## Global Constraints

- Edition Rust 2024; MSRV 1.85 — no newer APIs.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must be clean; no new `#[allow]`.
- No `unwrap()`/`expect()` in non-test production code without a justifying comment.
- `uwh-common` is a shared wire-format crate → **heavy process**: per-task verification, full `just check`, and confirm `refbox`/`overlay`/`schedule-processor`/`led-panel-sim` still compile.
- All 15 locales get a value for every new key — **no English placeholders** (best-guess + flag for native review). Locales: de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN.
- Branch off **fresh `origin/master`** (PR #1255 merged 2026-06-18; `game_info_table.rs` is on master). Suggested branch (broadest crate = uwh-common): `feat/uwh-common/referee-team-names`. **Confirm with the human before creating it** (approval gate).
- Literal-value rule: keep the existing "Time/Score Helper" wording (F1); do not shorten.

## Data contract (confirmed live, event 2113-A, 19 games)

Each `refereeAssignments` entry: `{ identifier, role, userId, teamId, resultOf, seededBy, comments, isTeamRefereeAssignment }`. Team assignment ⇒ `userId: null`, `teamId: "teams/10753-A"` (full RavenDB id form — accepted by the existing `TeamId::from_full`). Team-mode events emit only `Referees` and `TimeOrScoreKeeper`.

Canonical roles (`base/Events/EventRefereeRole.cs`): `Chief, Water1, Water2, Water3, TimeOrScoreKeeper, TimeOrScoreHelper, Referees`.

## File Structure

- `uwh-common/src/uwhportal/schedule.rs` — add `team_id` to `RefereeAssignment`; update 2 serde tests, add 1.
- `refbox/src/app/view_builders/game_info_table.rs` — thread `teams` into `referee_rows`; extract pure `referee_layout_rows`; two-layout logic + role fixes + team resolution; add unit tests.
- `refbox/translations/<locale>/refbox.ftl` (×15) — add `gi-ref-water-referees`, `gi-ref-deck-referees`.

No change to `refbox/src/app/mod.rs` (the person name-map loop is correct as-is; teams resolve at render time).

---

### Task 1: Add `team_id` to `RefereeAssignment` (uwh-common — HIGH blast radius)

**Files:**
- Modify: `uwh-common/src/uwhportal/schedule.rs` (struct ~209-218; tests ~1424-1456)
- Test: same file (`#[cfg(test)]` module)

**Interfaces:**
- Produces: `RefereeAssignment.team_id: Option<TeamId>` (deserialized from JSON `"teamId"`, full-id form). Constructors must now set `team_id`.

- [ ] **Step 1: Write the failing test** — add to the test module:

```rust
#[test]
fn test_deserialize_referee_assignment_parses_team_id() {
    use serde_json::json;
    // Team assignment: userId null, teamId set (full RavenDB id form).
    let input = json!({
        "role": "Referees",
        "userId": null,
        "teamId": "teams/10753-A",
        "isTeamRefereeAssignment": true
    });
    let ra: RefereeAssignment = serde_json::from_value(input).unwrap();
    assert_eq!(ra.role, "Referees");
    assert_eq!(ra.user_id, None);
    assert_eq!(ra.team_id, Some(TeamId::from_full("teams/10753-A").unwrap()));
    assert_eq!(ra.display_name, None);

    // Absent teamId → None.
    let input2 = json!({ "role": "Chief", "userId": "u1" });
    let ra2: RefereeAssignment = serde_json::from_value(input2).unwrap();
    assert_eq!(ra2.team_id, None);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p uwh-common test_deserialize_referee_assignment_parses_team_id`
Expected: compile error — `RefereeAssignment` has no field `team_id`.

- [ ] **Step 3: Add the field** — in the struct (keep `display_name` last and `#[serde(skip)]`):

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeAssignment {
    pub role: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    /// Team id when the official is assigned by team (then `user_id` is `None`).
    /// Full RavenDB id form, e.g. `"teams/10753-A"`. Omitted from output when `None`.
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<TeamId>,
    /// Human-readable display name, resolved from the portal after fetching.
    /// Not present in the portal JSON; populated locally via name-map lookup.
    #[serde(skip)]
    pub display_name: Option<String>,
}
```

(`TeamId` is already in scope in this module; it derives `Clone, PartialEq, Eq` and has a manual `Debug` + `Deserialize`, so `Option<TeamId>` satisfies all of `RefereeAssignment`'s derives. `skip_serializing_if` keeps `team_id: None` out of serialized output so the serialize test stays exact.)

- [ ] **Step 4: Fix the two existing serde tests that the new field changes**

In `test_serialize_referee_assignment_skips_display_name`, add `team_id: None` to the constructor (assertion is unchanged — `None` is skipped):

```rust
let ra = RefereeAssignment {
    role: "Chief".to_string(),
    user_id: Some("u123".to_string()),
    team_id: None,
    display_name: Some("Alice".to_string()),
};
let serialized = serde_json::to_value(&ra).unwrap();
assert_eq!(serialized, json!({"role": "Chief", "userId": "u123"}));
```

In `test_deserialize_referee_assignment_ignores_unknown_fields`, `teamId` is no longer "unknown" and `"t789"` is not a valid full id, so change its input to keep only genuinely-unknown fields:

```rust
let input = json!({
    "role": "Water1",
    "userId": "u456",
    "identifier": "ABC123",
    "comments": "captain",
    "displayName": "will-be-ignored"
});
let ra: RefereeAssignment = serde_json::from_value(input).unwrap();
assert_eq!(ra.role, "Water1");
assert_eq!(ra.user_id, Some("u456".to_string()));
assert_eq!(ra.team_id, None);
assert_eq!(ra.display_name, None);
```

- [ ] **Step 5: Run uwh-common tests to verify they pass**

Run: `cargo test -p uwh-common`
Expected: PASS (new test + both edited tests + all others).

- [ ] **Step 6: Confirm downstream crates still compile (wire-format blast radius)**

Run: `cargo build -p refbox -p overlay -p schedule-processor -p led-panel-sim`
Run (no_std isolation): `cargo build -p uwh-common --no-default-features`
Expected: all succeed (additive `Option` field; only the in-test constructor needed updating).

- [ ] **Step 7: Commit**

```bash
git add uwh-common/src/uwhportal/schedule.rs
git commit -m "feat(uwh-common): parse teamId on referee assignments"
```

---

### Task 2: Add Water/Deck Referees locale keys (refbox — 15 locales)

**Files:**
- Modify: `refbox/translations/<locale>/refbox.ftl` for all 15 locales (insert after the existing `gi-ref-water-3` line).

**Interfaces:**
- Produces: FTL keys `gi-ref-water-referees`, `gi-ref-deck-referees` (referenced by Task 3's `fl!`).

- [ ] **Step 1: Add both keys to every locale** using these values (en-US is authoritative; others best-guess, flag for native review):

| Locale | `gi-ref-water-referees` | `gi-ref-deck-referees` |
|---|---|---|
| en-US | Water Referees | Deck Referees |
| de-DE | Wasserschiedsrichter | Deckschiedsrichter |
| es | Árbitros de agua | Árbitros de borde |
| fr | Arbitres Aquatiques | Arbitres de Bord |
| id-ID | Wasit Air | Wasit Tepi Kolam |
| it-IT | Arbitri di Vasca | Arbitri di Bordo |
| ja-JP | 水中審判 | デッキ審判 |
| ko-KR | 수중 심판 | 데크 심판 |
| ms-MY | Pengadil Air | Pengadil Tepi Kolam |
| nl-NL | Waterscheidsrechters | Kantscheidsrechters |
| pt-PT | Árbitros Aquáticos | Árbitros de Bordo |
| th-TH | ผู้ตัดสินในน้ำ | ผู้ตัดสินขอบสระ |
| tl-PH | Water Referees | Deck Referees |
| tr-TR | Su Hakemleri | Kenar Hakemleri |
| zh-CN | 水下裁判 | 岸上裁判 |

Each file gets two new lines, e.g. en-US after `gi-ref-water-3 = Water Referee 3`:

```
gi-ref-water-referees = Water Referees
gi-ref-deck-referees = Deck Referees
```

- [ ] **Step 2: Verify FTL parses and keys are present**

Run: `cargo build -p refbox`
Run: `for d in refbox/translations/*/; do grep -L "gi-ref-deck-referees" "$d/refbox.ftl"; done` (must print nothing — every locale has it)
Expected: build OK; no locale missing either key.

- [ ] **Step 3: Commit**

```bash
git add refbox/translations
git commit -m "feat(refbox): add Water/Deck Referees game-info labels"
```

---

### Task 3: Two-layout referee rows with team resolution + role fixes (refbox)

**Files:**
- Modify: `refbox/src/app/view_builders/game_info_table.rs` — `referee_rows` (≈267-325), call site (≈201-203), add `referee_layout_rows`, add tests in the `#[cfg(test)]` module.

**Interfaces:**
- Consumes: `RefereeAssignment.team_id` (Task 1); `gi-ref-water-referees` / `gi-ref-deck-referees` (Task 2); `TeamList = BTreeMap<TeamId, String>`.
- Produces: `referee_layout_rows(assignments: &[RefereeAssignment], teams: Option<&TeamList>) -> Vec<Row>` (private; tested directly).

- [ ] **Step 1: Write the failing tests** — add to the `#[cfg(test)] mod tests` block (top of file likely needs `use std::collections::BTreeMap;` and `use uwh_common::uwhportal::schedule::{RefereeAssignment, TeamId};` — add if absent):

```rust
fn person(role: &str, name: &str) -> RefereeAssignment {
    RefereeAssignment {
        role: role.to_string(),
        user_id: Some(format!("uid-{name}")),
        team_id: None,
        display_name: Some(name.to_string()),
    }
}
fn team(role: &str, full_id: &str) -> RefereeAssignment {
    RefereeAssignment {
        role: role.to_string(),
        user_id: None,
        team_id: Some(TeamId::from_full(full_id).unwrap()),
        display_name: None,
    }
}
fn teamlist() -> TeamList {
    BTreeMap::from([(TeamId::from_full("teams/10753-A").unwrap(), "Sharks".to_string())])
}
fn ref_pairs(rows: &[Row]) -> Vec<(String, String)> {
    rows.iter()
        .filter_map(|r| match r {
            Row::Referee { label, name } => Some((label.clone(), name.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn team_layout_shows_water_and_deck_referees() {
    let tl = teamlist();
    let rows = referee_layout_rows(
        &[team("Referees", "teams/10753-A"), team("TimeOrScoreKeeper", "teams/10753-A")],
        Some(&tl),
    );
    assert_eq!(
        ref_pairs(&rows),
        vec![
            (fl!("gi-ref-water-referees"), "Sharks".to_string()),
            (fl!("gi-ref-deck-referees"), "Sharks".to_string()),
        ]
    );
}

#[test]
fn individual_layout_resolves_person_and_corrected_helper_role() {
    let rows = referee_layout_rows(
        &[person("Chief", "Alice"), person("TimeOrScoreHelper", "Bob")],
        None,
    );
    let pairs = ref_pairs(&rows);
    assert_eq!(pairs.len(), 6); // all individual rows always shown
    assert!(pairs.contains(&(fl!("gi-ref-chief"), "Alice".to_string())));
    // Corrected role string ("TimeOrScoreHelper") now resolves into the helper row.
    assert!(pairs.contains(&(fl!("gi-ref-timekeeper-helper"), "Bob".to_string())));
}

#[test]
fn team_in_numbered_water_slot_is_per_slot() {
    let tl = teamlist();
    let rows = referee_layout_rows(&[team("Water2", "teams/10753-A")], Some(&tl));
    let pairs = ref_pairs(&rows);
    assert!(pairs.contains(&(fl!("gi-ref-water-2"), "Sharks".to_string())));
    assert_eq!(pairs.len(), 6); // individual layout (no Referees role)
}

#[test]
fn deck_referees_absorbs_team_helper_keeper_wins() {
    let mut tl = teamlist();
    tl.insert(TeamId::from_full("teams/999-B").unwrap(), "Rays".to_string());
    let rows = referee_layout_rows(
        &[
            team("Referees", "teams/10753-A"),
            team("TimeOrScoreKeeper", "teams/10753-A"),
            team("TimeOrScoreHelper", "teams/999-B"),
        ],
        Some(&tl),
    );
    // Keeper team wins the single Deck Referees row; helper absorbed.
    assert_eq!(
        ref_pairs(&rows),
        vec![
            (fl!("gi-ref-water-referees"), "Sharks".to_string()),
            (fl!("gi-ref-deck-referees"), "Sharks".to_string()),
        ]
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox team_layout_shows_water_and_deck_referees individual_layout_resolves_person_and_corrected_helper_role team_in_numbered_water_slot_is_per_slot deck_referees_absorbs_team_helper_keeper_wins`
Expected: compile error — `referee_layout_rows` does not exist.

- [ ] **Step 3: Implement** — replace the body of `referee_rows` and add `referee_layout_rows` directly below it:

```rust
fn referee_rows(
    game_number: &GameNumber,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> Vec<Row> {
    let assignments = schedule
        .and_then(|s| s.games.get(game_number))
        .and_then(|g| g.referee_assignments.as_deref())
        .unwrap_or(&[]);
    referee_layout_rows(assignments, teams)
}

// Two fixed layouts (all rows in the chosen layout always shown, "-" for empty):
//  * Team layout (a `Referees` role is present — only possible in Portal Team mode):
//    Water Referees (from `Referees`) + Deck Referees (from team `TimeOrScoreKeeper`,
//    with team `TimeOrScoreHelper` absorbed; keeper team wins if they differ).
//  * Individual layout (otherwise): Chief / Time-Score Keeper / Helper / Water 1-3,
//    each filled by a person name or a per-slot team name.
fn referee_layout_rows(assignments: &[RefereeAssignment], teams: Option<&TeamList>) -> Vec<Row> {
    // Resolve one assignment to a display string: person name, else team name, else "-".
    let resolve = |a: &RefereeAssignment| -> String {
        if a.user_id.is_some() {
            a.display_name.clone().unwrap_or_else(|| "-".to_string())
        } else if let Some(tid) = &a.team_id {
            teams
                .and_then(|t| t.get(tid).cloned())
                .unwrap_or_else(|| tid.full().to_string())
        } else {
            "-".to_string()
        }
    };

    let team_layout = assignments.iter().any(|a| a.role == "Referees");

    if team_layout {
        let mut water = "-".to_string();
        let mut deck = "-".to_string();
        let mut deck_from_keeper = false;
        for a in assignments {
            match a.role.as_str() {
                "Referees" => water = resolve(a),
                "TimeOrScoreKeeper" => {
                    deck = resolve(a);
                    deck_from_keeper = true;
                }
                "TimeOrScoreHelper" => {
                    if !deck_from_keeper {
                        deck = resolve(a);
                    }
                }
                _ => {}
            }
        }
        vec![
            Row::Referee { label: fl!("gi-ref-water-referees"), name: water },
            Row::Referee { label: fl!("gi-ref-deck-referees"), name: deck },
        ]
    } else {
        let mut chief = "-".to_string();
        let mut keeper = "-".to_string();
        let mut helper = "-".to_string();
        let mut water = ["-".to_string(), "-".to_string(), "-".to_string()];
        for a in assignments {
            match a.role.as_str() {
                "Chief" => chief = resolve(a),
                "TimeOrScoreKeeper" => keeper = resolve(a),
                "TimeOrScoreHelper" => helper = resolve(a),
                "Water1" => water[0] = resolve(a),
                "Water2" => water[1] = resolve(a),
                "Water3" => water[2] = resolve(a),
                _ => {}
            }
        }
        vec![
            Row::Referee { label: fl!("gi-ref-chief"), name: chief },
            Row::Referee { label: fl!("gi-ref-timekeeper"), name: keeper },
            Row::Referee { label: fl!("gi-ref-timekeeper-helper"), name: helper },
            Row::Referee { label: fl!("gi-ref-water-1"), name: water[0].clone() },
            Row::Referee { label: fl!("gi-ref-water-2"), name: water[1].clone() },
            Row::Referee { label: fl!("gi-ref-water-3"), name: water[2].clone() },
        ]
    }
}
```

- [ ] **Step 4: Update the call site** to pass `teams` (≈line 202):

```rust
    if using_uwhportal {
        rows.extend(referee_rows(current_game_num, schedule, teams));
    }
```

- [ ] **Step 5: Run the referee tests to verify they pass**

Run: `cargo test -p refbox`
Expected: PASS — the 4 new tests, plus `no_referees_without_portal` and `referee_rows_always_include_blank_helper_and_all_water` still pass (individual layout = today's default behavior; portal-off gate unchanged).

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/game_info_table.rs
git commit -m "feat(refbox): show team-assigned referees in game info"
```

---

### Task 4: Full verification + live walkthrough

- [ ] **Step 1: Full workspace gate**

Run: `just check`
Expected: fmt, clippy (`-D warnings`), all tests, audit — clean.

- [ ] **Step 2: Build the actual binary (clippy/tests don't build target/debug/refbox)**

Run: `cargo build -p refbox`

- [ ] **Step 3: Live walkthrough against event 2113-A** (operator-visible acceptance):

1. Launch refbox (background, `dangerouslyDisableSandbox`, `WAYLAND_DISPLAY=` for WSLg), connect to the Portal event used for 2113-A, open a game's Game Information page.
2. Confirm each game shows **two** referee rows — `Water Referees: <team name>` and `Deck Referees: <team name>` — instead of six dashes.
3. (If an individual-assigned event is available) confirm the six individual rows still show with correct names, and the time/score **helper** now resolves (previously broken by the `TimeOrScoreHelper` mismatch).
4. Record results in the PR "How to verify" section.

---

## Self-Review

- **Spec coverage:** team water → Water Referees (Task 3 team layout); team time/score → Deck Referees w/ helper absorbed (Task 3); per-slot water team (Task 3 individual); role-string fix `TimeOrScoreHelper` (Task 3 both layouts); `teamId` parse (Task 1); 2 new labels ×15 (Task 2); all-rows-shown / two-layout switch via `Referees` (Task 3). ✓
- **Placeholder scan:** every code/step is concrete; locale values are real strings (best-guess flagged). ✓
- **Type consistency:** `referee_layout_rows(&[RefereeAssignment], Option<&TeamList>)` and `referee_rows(.., teams)` match across Tasks 1/3; `team_id: Option<TeamId>` used identically in struct, tests, and resolution. ✓

## Notes / deferred

- F1 (keep "Time/Score Helper"), F3 (team-Chief → "Chief Referee: Team" in individual layout), F4 (keeper team wins Deck row) per spec.
- Best-guess "Deck Referees" translations (esp. tr-TR, zh-CN, nl-NL, th-TH chose poolside/edge senses over "ship deck") need native review — flag in PR.
- No `mod.rs` change: persons still resolve via the existing name-map loop; teams resolve at render from `TeamList`.
