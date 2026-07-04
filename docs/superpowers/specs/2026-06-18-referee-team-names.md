# Spec: Referee/Official names for team-assigned Portal events

**Date:** 2026-06-18
**Status:** Draft — awaiting human sign-off before writing the implementation plan
**Crates:** `uwh-common` (one new field — high blast radius), `refbox` (render logic + 15 locales)
**Scope boundary:** the *content* of the referee rows in the game-info **table** only. NOT the
table layout (PR #1255), NOT the Portal side, NOT `overlay` / LED panel / `schedule-processor` /
`wireless-remote`.

---

## Plain-English summary

When a referee is assigned to a game in the UWH Portal, the game-info screen in refbox is supposed
to show that official's name next to their role (Chief Referee, Water Referee 1, etc.). Today it
shows a dash ("-") for **every** official whenever the event assigns referees **by team** instead
of by individual person — which is how real tournaments are increasingly run.

The cause: refbox only understands officials identified by *person*. When the Portal says "this
whole team is providing the referees," refbox has no person to look up, so it gives up and shows a
dash. There are also two smaller mismatches in how refbox reads the Portal's role labels.

This change makes refbox understand team assignments too: it resolves a team to that team's name
and shows it against the right role. It also fixes the two label mismatches so every official —
person or team — appears correctly.

---

## Root cause (confirmed against live event 2113-A)

`refbox/src/app/view_builders/game_info_table.rs::referee_rows` does three wrong things:

1. **Skips every team assignment.** It does `if r.user_id.is_none() { continue; }` — but team
   assignments always have `userId = null` and a `teamId` instead. So they are all dropped.
2. **Matches the wrong role string for the time/score helper.** It matches
   `"TimeOrScoreKeeperHelper"`, but the Portal's actual enum value is `"TimeOrScoreHelper"`.
3. **Has no handler for the `"Referees"` catch-all role**, which is the role the Portal uses for
   team-assigned water officials (it falls through the `_ => {}` arm and is ignored).

Live confirmation (event `2113-A`, `/api/events/2113-A/schedule/privileged`, 19 games):

```json
{
  "role": "Referees",            "userId": null, "teamId": "teams/10753-A", "isTeamRefereeAssignment": true
}
{
  "role": "TimeOrScoreKeeper",   "userId": null, "teamId": "teams/10753-A", "isTeamRefereeAssignment": true
}
```

Only those two roles appear for a team-assigned event. `teamId` is the **full** RavenDB id form
(`"teams/..."`), which the existing `uwh_common::uwhportal::schedule::TeamId` deserializer
(`from_full`) already accepts.

---

## Canonical Portal roles (authoritative — `base/Events/EventRefereeRole.cs`)

Exactly seven: `Chief, Water1, Water2, Water3, TimeOrScoreKeeper, TimeOrScoreHelper, Referees`.

- `TimeOrScoreHelper` — only when the event enables time/score helpers.
- `Referees` — **only** when the event's assignment mode is `Team` (the catch-all water role).
- Any role with `teamId != null` is a *team* assignment (`IsTeamRefereeAssignment`).

---

## Display model (agreed with human)

A slot is a **team** assignment when `user_id` is `None` and `team_id` is `Some`; otherwise it is an
**individual** assignment (person).

| Portal role | Individual (person name) | Team (team name) |
|---|---|---|
| `Chief` | `Chief Referee: <name>` | `Chief Referee: <team>` (fallback; only in "All" mode) |
| `Water1` | `Water Referee 1: <name>` | `Water Referee 1: <team>` (per-slot) |
| `Water2` | `Water Referee 2: <name>` | `Water Referee 2: <team>` (per-slot) |
| `Water3` | `Water Referee 3: <name>` | `Water Referee 3: <team>` (per-slot) |
| `TimeOrScoreKeeper` | `Time/Score Keeper: <name>` | **`Deck Referees: <team>`** |
| `TimeOrScoreHelper` | `Time/Score Helper: <name>` (individual row kept) | *absorbed into Deck Referees — no separate row* |
| `Referees` (catch-all) | — (never individual) | `Water Referees: <team>` |

Key decisions captured:
- **Team in a numbered water slot → per-slot label** (`Water Referee 2: TeamA`), same label an
  individual in that slot would get. (Human decision, 2026-06-18.)
- **Team in `TimeOrScoreKeeper` → "Deck Referees"**, *not* "Time/Score Keeper". (Human decision.)
- **Team helper (`TimeOrScoreHelper`) is absorbed into the single Deck Referees row** — no separate
  deck-helper row. If both keeper-team and helper-team are present, the Deck Referees row shows the
  `TimeOrScoreKeeper` team; if only a helper-team is present, it shows that team.
- **`Referees` catch-all → "Water Referees"** — the only inherently-team, group-style row.
- **"Deck Referees" exists only in the Team layout.** The `TimeOrScoreKeeper`→Deck-Referees mapping
  above applies when the layout is Team (triggered by a `Referees` role). In the rare "All"-mode
  case where a team sits in `TimeOrScoreKeeper` with no `Referees` role, the Individual layout
  renders it per-slot as `Time/Score Keeper: <team>`.

### Name resolution
- **Person:** already works — `RefereeAssignment.display_name` is populated post-fetch from the
  `/referees` name-map (prefers `rosterName`, else `user.username`). No change needed.
- **Team:** resolve `team_id` against the `TeamList` (`BTreeMap<TeamId, String>`) already in scope
  in the table builder, mirroring `get_team_name`: `teams.get(id).cloned().unwrap_or_else(|| id.full())`.

### Rendering: which rows appear (human decision, 2026-06-18)
**All rows in the chosen layout are always shown** — empty slots render "-" (today's behavior is
kept, NOT changed to filled-only). The table switches between **two fixed layouts**:

**Trigger:** the game has a `Referees`-role assignment → **Team layout**. Otherwise → **Individual
layout**. (`Referees` exists only in the Portal's Team assignment mode, so its presence is the
unambiguous team-catch-all signal. In Team mode there are no individual water roles, so the two
layouts never mix.)

**Individual layout — 6 rows, always shown (existing order preserved):**

1. Chief Referee
2. Time/Score Keeper
3. Time/Score Helper
4. Water Referee 1
5. Water Referee 2
6. Water Referee 3

Each row's value = the person name, **or** a team name when a team sits in that exact slot
(per-slot, e.g. `Water Referee 2: TeamA`), **or** "-" if unassigned. This is the default and
matches today's always-6-rows behavior, so the existing always-render test largely survives.

**Team layout — 2 rows, always shown:**

1. Water Referees  (team from the `Referees` role, else "-")
2. Deck Referees   (team from `TimeOrScoreKeeper`; `TimeOrScoreHelper` team absorbed; else "-")

A *team-assigned* event therefore shows exactly these two rows.

---

## Data contract change (`uwh-common` — HIGH blast radius)

Add one field to `RefereeAssignment` in `uwh-common/src/uwhportal/schedule.rs`:

```rust
#[serde(rename = "teamId")]
pub team_id: Option<TeamId>,
```

- `TeamId` already exists in the same module; its `Deserialize` calls `from_full`, which matches the
  `"teams/..."` values seen live.
- `display_name` stays `#[serde(skip)]` (person-only, populated post-fetch).
- The existing constructor in the unit test (`schedule.rs` ~line 1322) must add `team_id`.
- Add a test asserting `teamId` deserializes into `team_id` (and that an absent `teamId` → `None`).
- Downstream crates that build `RefereeAssignment` literals must add the field. (Audit: only
  `refbox` and the test currently construct it — verify during planning with a workspace grep.)

Because this is a shared wire-format type, the heavy process applies: per-task verification,
`just check` across the workspace, and confirmation that `refbox`, `overlay`, `schedule-processor`,
and `led-panel-sim` still compile.

---

## Render-logic change (`refbox/src/app/view_builders/game_info_table.rs`)

- Thread `teams: Option<&TeamList>` into `referee_rows` (already in scope at the call site, line ~202).
- Decide the layout first: scan the game's assignments for a `"Referees"` role → Team layout,
  else Individual layout.
- Per assignment, determine person-vs-team (`user_id.is_some()` → person; else `team_id` → team)
  and resolve the display string (person `display_name`; team via the `TeamList` lookup mirroring
  `get_team_name`). Empty stays "-".
- **Individual layout:** match `"Chief"`, `"Water1/2/3"`, `"TimeOrScoreKeeper"`,
  `"TimeOrScoreHelper"` (corrected string) into the six always-shown rows; a per-slot team name is
  allowed in any of these rows.
- **Team layout:** `"Referees"` → Water Referees row; `"TimeOrScoreKeeper"` (team) → Deck Referees
  row, with `"TimeOrScoreHelper"` (team) absorbed (keeper team wins if both differ). Two rows, both
  always shown.
- Tests:
  - The existing `referee_rows_always_include_blank_helper_and_all_water` and
    `no_referees_without_portal` should still pass essentially unchanged (Individual layout is the
    default and still emits all rows / no schedule still yields the blank Individual layout) —
    verify and adjust only if assertions reference exact counts.
  - Add: team-assigned game → exactly the two Team-layout rows with the resolved team name;
    corrected `TimeOrScoreHelper` resolves a person into the Time/Score Helper row; a per-slot team
    shows `Water Referee N: <team>`; deck-helper absorption (keeper + helper teams → single Deck
    Referees row).

---

## Locale keys (`refbox/translations/<locale>/refbox.ftl`, all 15 locales)

Existing: `gi-ref-chief`, `gi-ref-timekeeper`, `gi-ref-timekeeper-helper`, `gi-ref-water-1/2/3`.

Add two new keys, with a best-guess translation in **every** locale (no English placeholders):

- `gi-ref-water-referees` = `Water Referees` (en-US)
- `gi-ref-deck-referees` = `Deck Referees` (en-US)

Locales: de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH,
tr-TR, zh-CN. (Translations are best-guess; flag for native review, per project convention.)

---

## Acceptance criteria

1. With event 2113-A loaded, each game's info table shows **two** referee rows —
   `Water Referees: <team 10753-A name>` and `Deck Referees: <team 10753-A name>` — instead of six
   dashes.
2. An individual-assigned event still shows the correct person names against the correct roles
   (no regression), and the time/score **helper** now resolves (previously broken by the
   `TimeOrScoreHelper` string mismatch).
3. A team in a numbered water slot shows `Water Referee N: <team>`.
4. `just check` is green across the workspace; `uwh-common`, `overlay`, `schedule-processor`,
   `led-panel-sim` all still compile.

---

## Open spec-review flags (low-stakes; sensible defaults chosen — object if wrong)

- **F1.** Individual time/score-helper label kept as the existing **"Time/Score Helper"** (not
  shortened to "T/S Helper"). Literal-value rule — not changing copy without instruction.
- **F2.** RESOLVED (human, 2026-06-18): keep **all rows shown** with "-" for empty. Two fixed
  layouts — 6 individual rows vs 2 team rows — switched by the presence of a `Referees`-role
  assignment. The `Referees`-role presence as the switch signal is my implementation of that; flag
  if a different signal was intended.
- **F3.** Team-assigned `Chief` (only possible in "All" mode) → "Chief Referee: <team>" fallback;
  not absorbed into Deck Referees.
- **F4.** Deck Referees row name preference when keeper-team and helper-team differ → keeper-team.

---

## Risks / notes

- **Highest risk** is the `uwh-common` field addition (shared wire format). Mitigated: additive
  `Option` field, existing deserializer already accepts the live id format, full `just check`.
- This branches off **fresh `origin/master`** (PR #1255 merged 2026-06-18, so `game_info_table.rs`
  exists there). The current local spike branch is far behind and must not be used as a base.
