# Require player number + infraction before saving an add/edit entry — Design

**Date:** 2026-06-17
**Crate:** `refbox` (UI only)
**Relationship:** separate concern from the Apply-button rollout
(`feat/refbox/apply-button-rollout`). Touches a shared file (`keypad_pages/mod.rs`), so it
will be built **on top of** that branch to avoid a collision (see "Branch & sequencing").

---

## Goal

Stop an incomplete penalty / warning / foul from being recorded, by disabling the green **"Done"**
button on the add/edit screens until the required fields are filled in. Catch it at the point of
entry, before it reaches the overview list. The "Done" label itself does **not** change — only its
enabled/disabled state.

## Required fields (the criteria)

Gated fields: **player number** (≥ 1; `0` means "not entered") **and infraction**
(anything other than the unset `Infraction::Unknown`, shown as "?"). Penalty kind and team colour
always have a default and never gate. **Cancel and Delete stay enabled at all times.**

### Foul screen (`make_foul_add_page`) — team selector: Black / "=" (Equal) / White
| Option | Infraction required | Player # required | Done enabled when |
|--------|--------------------|-------------------|-------------------|
| "=" (Equal, `color == None`) | Yes | No | infraction selected |
| Black / White (individual) | Yes | Yes | infraction selected **and** number ≥ 1 |

### Warning screen (`make_warning_add_page`) — TEAM toggle + colour
| Mode | Infraction required | Player # required | Done enabled when |
|------|--------------------|-------------------|-------------------|
| TEAM (`team_warning == true`) | Yes | No | infraction selected |
| Individual (`team_warning == false`) | Yes | Yes | infraction selected **and** number ≥ 1 |

### Penalty screen (`make_penalty_edit_page`) — Black / White only (always individual)
| "Track fouls & warnings" setting | Infraction required | Player # required | Done enabled when |
|----------------------------------|--------------------|-------------------|-------------------|
| ON (infraction picker shown) | Yes | Yes | infraction selected **and** number ≥ 1 |
| OFF (no infraction picker) | No | Yes | number ≥ 1 |

Penalties have no team concept, so they always need a player number. The infraction is only
required when the "track fouls & warnings" setting is on — which is exactly when the penalty
screen shows the infraction picker (so the operator can actually set it).

## Approach (chosen: gate inside each screen)

Each of the three screen builders receives the current keypad player number and disables its own
"Done" with a small, local, unit-testable helper, via `on_press_maybe` (the same mechanism used
everywhere else for greying a button). This mirrors the game-number-editor pattern.

The keypad player number lives in the shared dispatcher `build_keypad_page` (its `player_num`
argument); the three call sites pass it through to the screen builders. The infraction and the
team/colour/setting flags are already arguments to each builder.

### New helper per screen (use `!matches!(...)` so no `PartialEq` is required on `Infraction`)

```rust
// foul_add.rs
fn foul_add_can_commit(infraction: Infraction, color: Option<GameColor>, player_num: u32) -> bool {
    !matches!(infraction, Infraction::Unknown) && (color.is_none() || player_num > 0)
}

// warning_add.rs
fn warning_add_can_commit(infraction: Infraction, team_warning: bool, player_num: u32) -> bool {
    !matches!(infraction, Infraction::Unknown) && (team_warning || player_num > 0)
}

// penalty_edit.rs
fn penalty_edit_can_commit(
    infraction: Infraction,
    track_fouls_and_warnings: bool,
    player_num: u32,
) -> bool {
    player_num > 0 && (!track_fouls_and_warnings || !matches!(infraction, Infraction::Unknown))
}
```

The green "Done" button changes from `.on_press(Message::…EditComplete { canceled: false, … })`
to `.on_press_maybe(<helper>(…).then_some(Message::…EditComplete { canceled: false, … }))`.

### Signature / plumbing changes
- `make_foul_add_page` gains `player_num: u32`.
- `make_warning_add_page` gains `player_num: u32`.
- `make_penalty_edit_page` gains `player_num: u32`.
- In `build_keypad_page` (`keypad_pages/mod.rs`), the `FoulAdd`, `WarningAdd`, and `Penalty` arms
  pass the existing `player_num` to those builders. No other call sites exist (these builders are
  only invoked from `build_keypad_page`).

No `RefBoxApp` field, `AppState`, message, translation, or dependency changes.

## Behavior notes / edge cases
- **New foul** opens on "=" with no infraction → Done greys until an infraction is picked
  (Equal needs no number).
- **New warning** opens Individual with no infraction → Done greys until both an infraction and a
  number are entered (or TEAM is switched on, which drops the number requirement).
- **New penalty** → Done greys until a number is entered (and an infraction, if tracking is on).
- **Editing an existing entry** seeds the keypad with the stored number and the stored infraction,
  so a complete entry shows Done enabled; a legacy entry missing a field greys Done until fixed —
  consistent and desirable.
- **Toggling** team/equal or the colour re-renders the screen, so Done enables/disables live.

## Out of scope (unchanged)
- The overview/list screens (Penalties / Warnings / Fouls) — they keep the Apply behavior from the
  other branch.
- The "Done" *label* on these add/edit screens (stays "Done").
- Penalty **kind** and team **colour** are never gated (always defaulted).
- Score-add, portal-login, game-number, team-timeout, and any other keypad page.

## Testing
- Unit-test each `*_can_commit` helper against its truth table (e.g. foul: Unknown+Equal+0 → false;
  Set+Equal+0 → true; Set+Black+0 → false; Set+Black+5 → true; Unknown+Black+5 → false; and the
  analogous cases for warning and penalty, including penalty with tracking off requiring only a
  number).
- `cargo test -p refbox` (no `--lib`); `cargo clippy -p refbox -- -D warnings`; `just check`.
- Walkthrough each screen: button greys until the criteria are met, enables once they are; Cancel
  and Delete always work.

## Branch & sequencing
- New branch `feat/refbox/add-entry-required-fields`, built **on top of** `feat/refbox/apply-button-rollout`
  (both edit `keypad_pages/mod.rs`; stacking avoids a merge collision). Its own commit(s), its own PR.
- Recommended PR order: merge the Apply-button PR first, then rebase this onto master and open its PR.
- Lean process (refbox UI). Spec/plan stay local, uncommitted. Branch creation needs the user's OK.
