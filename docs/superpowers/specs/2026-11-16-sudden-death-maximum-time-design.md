# Sudden death with a maximum time (portal register #183) — refbox design

**Status:** ✅ PO-APPROVED 2026-11-16, as written ("1"), including P1–P6 and the design-session calls C1–C5
(Game Info page; confirm window at a level 0:00; zero maximum refused; rugby waits for a penalty shot; merge gated on
the field name, now satisfied). Portal field name AGREED (`suddenDeathMaxDuration`). Nothing is built. No plan yet.
**Source:** portal register `#183` (uwh-portal `docs/superpowers/specs/2026-09-26-feature-requests-register.md`,
branch `docs/feature-requests-register`), handed to the refbox design lane under the PO's "split by workspace"
ruling (2026-11-02). The portal keeps its server piece (the timing-rule field and the existing `TiedGameCoinFlip`);
this document is the refbox's half, and the website refbox copies it.
**Grounded at:** uwh-refbox-rs `origin/master` `976ca95e`.
**Builder check** run 2026-11-16 (fresh reviewer): three line references corrected (sudden-death entry, `start_game_clock`,
rugby penalty-shot end), four missed sites added (snapshot timing helpers, the buzzer, the panel hide-time, the
confirm-window "goal disallowed" branch), three design-session calls added (zero maximum, rugby, merge gated on the
field name), one PO question (schedule-processor scope).

**Working name:** "Golden Goal" is the design session's internal name for this feature, to keep it apart from
ordinary unlimited sudden death. **It never appears on any screen, in any translation, or in any wire field.**
The operator sees "SUDDEN DEATH", "MAXIMUM TIME" and "MAX".

## Intent

Some tournaments have only so much pool time. Each game gets a fixed amount of time to reach a result through play;
if sudden death has not produced a goal by then, the game stops and the result is decided off the field — the
tie-breakers, then a coin toss if they do not separate the teams — so the tournament can move on (PO, 2026-11-16).

Today the refbox cannot do this: sudden death's clock counts **up** with no end (`ClockState::CountingUp`,
`GamePeriod::duration` returns `None` for `SuddenDeath`), so a long sudden death delays every later game.

## Goals

- An event can give sudden death a **maximum time**. With one set, the sudden-death clock counts **down** from it.
- The first goal still ends the game, exactly as today.
- At 0:00 with no goal, the game ends **level**; the refbox records the level score and the portal's existing
  tie-breakers / `TiedGameCoinFlip` decide it.
- With no maximum set, sudden death behaves **exactly as today**.

## Non-goals

- **No tie-breaker or coin toss in the refbox.** The refbox records a level score; deciding it stays the portal's.
- **No new game stage.** There is no "Golden Goal" period; the game is in `GamePeriod::SuddenDeath` throughout.
- **No change to overtime.** (The PO's first idea, a single-period overtime, was set aside: see Approaches.)
- **No change to the Game Block check or schedule catch-up maths** (P5).
- **The clipped TEAM TIMEOUTS value** on the settings page is a separate item
  (`docs/backlog/team-timeouts-value-clipped/`), not fixed here — but this design must not repeat it.
- **Back → Home button relabel** (PO ruling 2026-11-16) is a separate design-lane item.

## Anti-goals (a failure even if it technically works)

- A display that shows "GOLDEN GOAL", "OVERTIME", or anything other than "SUDDEN DEATH" during this period.
- Zero (0:00) used to mean "no maximum". A mode belongs in an explicit choice, not a magic value — the same lesson
  as the zero-length half-time replaced by the `singlePeriod` flag.
- A sudden death that reaches 0:00 and keeps going, restarts, or ends with a winner the refbox invented.
- Any existing event (no maximum set, or a portal that never sends the field) behaving differently from today.

## Approaches considered

1. **An optional maximum on sudden death — CHOSEN.** The game stays in `SuddenDeath`; when a maximum is set, its
   clock counts down from it and 0:00 ends the game level. Every display (LED panel, overlay, overlay-bridge,
   portal) already labels `SuddenDeath` "SUDDEN DEATH" and shows whatever `secs_in_period` it is sent, so none of
   them needs a new state. Either repo can ship first (see Agreement).
2. **A single-period overtime with "first goal wins"** (the PO's first idea). Overtime already counts down, but
   every display shows overtime as "OVERTIME", so showing "SUDDEN DEATH" would mean special-casing every display.
   It also needs a new "a goal ends overtime" rule. Rejected once the PO chose "SUDDEN DEATH" on screen.
3. **A new `GamePeriod` (e.g. GoldenGoal).** Every display, the wire format and every translation would have to
   learn a new stage. Rejected by the PO's display choice (screens show "SUDDEN DEATH").

## What the operator sees

### Settings page (event settings grid, portal mode OFF) — PO decisions

The **SUDDEN DEATH ALLOWED** button no longer toggles on a single tap. It opens a new page.

The **button's value** shows:
- `NO` / `YES` as today when no maximum is set;
- when a maximum is set: **`MAX`** on top, at the **label** text size, with the time (e.g. `5:00`) below it at the
  **normal value** size used by every other time on the grid (HALF LENGTH 15:00, etc.). The two lines together must
  fit the button without clipping, in every language. (The TEAM TIMEOUTS button's two-line value "1/HALF" is clipped
  today; that is the failure to avoid.)

```
┌───────────────────────────┐
│ SUDDEN DEATH         MAX  │   MAX  = label text size
│ ALLOWED:            5:00  │   5:00 = normal value size
└───────────────────────────┘
```

The **new page** copies the TEAM TIMEOUTS page (`keypad_pages/team_timeout_edit.rs`) for its labelled rows, blue
selected / light-grey unselected choice buttons, greyed-out controls that do not apply, and CANCEL / APPLY footer;
and copies the standard time setter (`shared_elements::make_time_editor`, as used by the HALF LENGTH editor) for the
length:

```
┌──────────────────────────────────────┐
│ SUDDEN DEATH     [  NO  ]  [ YES ]   │
│ MAXIMUM TIME     [ NONE ]  [ SET ]   │
│                                      │
│            MAXIMUM TIME              │
│     [+]                      [+]     │
│     [-]        5:00          [-]     │
│                                      │
│ [CANCEL]                  [APPLY]    │
└──────────────────────────────────────┘
```

- `NO` selected → the MAXIMUM TIME row and the time setter are greyed out (not pressable), but still show the
  stored choice, as TEAM TIMEOUTS does when the count is 0.
- `NONE` selected → the time setter is greyed out.
- `SET` selected → the time setter is live.
- New words: **`MAX`** and **`MAXIMUM TIME`**, in capitals like the rest of the page (PO: "1" to all-caps),
  translated into every refbox locale. (NO / YES / NONE / SET: reuse existing strings where they exist; a new one is
  translated like the others.) Translations ship unchecked by native speakers unless the PO arranges otherwise — say
  so in the PR.

### During the game — PO decisions

- Every screen says **"SUDDEN DEATH"**, as today. With a maximum set, the clock counts **down**.
- The first goal ends the game, as today.

### Design-session proposals, confirmed by the PO (P1–P6)

- **P1 – Last 10 seconds.** With a maximum set, the clock flashes in its last 10 seconds like every other period.
  Today `shared_elements.rs` (~l.636) excludes `SuddenDeath` from the flash only because it counts up; the
  exclusion applies only when there is no maximum.
- **P2 – At 0:00.** The buzzer sounds and the game ends **level**, entering the normal between-games break, exactly
  like a level game today when sudden death is off. Whatever is in progress at that instant (a penalty, a timeout, a
  score-confirmation window) is handled the way the refbox already handles the end of a game.
- **P3 – No maximum.** Sudden death is unchanged: counts up, ends only on a goal.
- **P4 – Overtime + a maximum.** Allowed, not blocked: a level overtime leads into a time-limited sudden death.
- **P5 – Game Block and catch-up maths.** Unchanged. `Game::game_block_minimum` already leaves overtime out; the
  maximum is left out the same way.
- **P6 – Source of the setting.** Portal events: a new optional field on the event's timing rules. No portal: the
  settings page above.

### Design-session calls not individually walked with the PO (builder may revisit, flag in the PR)

- **Game Info page.** `game_info_table.rs` lists "sudden death allowed"; it should show the maximum the same way the
  settings button does (`MAX` + time), so the two pages agree.
- **Score-confirmation window at 0:00.** P2 says "the way the end of a game is handled"; for a normal game end that
  includes the score-confirmation window, so a level 0:00 sudden death gets it too. **Trap:** `end_confirm_pause`
  (~l.3351) reads a level score in `SuddenDeath` as "the deciding goal was disallowed, carry on" and stays in sudden
  death. The plan must tell "goal disallowed" apart from "time expired" there, or a level 0:00 resumes sudden death.
- **A zero maximum is refused, never read as "no maximum".** Settings page: SET with 0:00 cannot be applied (APPLY
  greyed, as other invalid lengths are). Portal: a `0` maximum is refused the same way the refbox refuses other
  zero-length periods (precedent: a zero half-time is refused without the `singlePeriod` flag). Absent/null is the only
  "no maximum".
- **Rugby mode: a rugby penalty shot running at 0:00 is waited for**, exactly as the refbox already extends a half
  for an unfinished rugby penalty shot ("Extending Second Half for unfinished penalty shot", ~l.2522). The maximum
  applies in rugby mode as in hockey.
- **Merging is gated on the agreed portal field name.** "Either side can ship first" holds only once both use the
  same name; a mismatch is silent (unknown fields are ignored), so no refbox PR carrying the field merges until the
  coordinators have confirmed the name. **Satisfied 2026-11-16:** `suddenDeathMaxDuration` agreed (see Agreement).

## Data and wire

- **`uwh_common::config::Game`** gains an optional sudden-death maximum (suggested name `sudden_death_max`,
  `Option<Duration>`; absent = no maximum). Serde-defaulted so every existing saved settings file loads as
  "no maximum"; the TOML migration path in `config.rs` (`process_duration` et al.) must accept files without it.
- **`uwh_common::uwhportal::schedule::TimingRule`** gains the matching optional portal field (name to be agreed —
  see Agreement), `#[serde(default)]`, converted into `Game` alongside `sudden_death_allowed`.
- **`GameSnapshot` / the wire to LED panel, overlay, overlay-bridge: unchanged.** `current_period` stays
  `SuddenDeath`; `secs_in_period` simply counts down. Verified on `976ca95e`: `matrix-drawing` (labels "SUDDEN\nDEATH"),
  `led-panel-sim`, `overlay` (`overtime_and_sudden_death` page) and `overlay-bridge` (relay only) key on the period,
  not on the clock direction.
- **`schedule-processor` is mothballed (PO, 2026-11-16):** it is being phased out for the Portal's own schedule
  tooling, and nobody will use the refbox copy. It gets only what keeps it compiling and green: the new field added to
  its `TimingRule` test literals (`csv_parser.rs` ~l.812) and deliberately **excluded** from `TIMING_RULE_FIELDS`
  (~l.610) so its drift test (~l.832) still passes and no spreadsheet column, upload, check or score-sheet row is
  added. The Portal's schedule tooling (upload, checks, score sheet) gets the maximum instead — sent through the
  coordinators.
- Blast radius (workspace rule): `uwh-common` change → check `refbox`, `schedule-processor` (builds `TimingRule`
  literals), `overlay`, `overlay-bridge`, `led-panel-sim`, `matrix-drawing`. This is **heavy process** under
  `.claude/rules/plan-execution.md` (uwh-common + the game-clock state machine).

## Engine: what has to change (orientation, not a script)

Sudden death's "counts up, never expires" assumption is spread through `refbox/src/tournament_manager/mod.rs`.
With a maximum set, **every** one of these must treat `SuddenDeath` as a counting-down period. Guard the class, not
the cases: find them all by searching for `SuddenDeath` and `CountingUp`, do not trust this list to be complete.

- `GamePeriod::duration` (`uwh-common/src/game_snapshot.rs` ~l.211) returns `None` for `SuddenDeath`; it has no
  `Game` field to return. The new field is the obvious source; check every caller, because `None` is currently read
  as "untimed" in several places (e.g. the confirm-pause resume maths, ~l.3380).
- Entering sudden death builds `CountingUp` in three places: `update()` when the pre-sudden-death break runs out
  (~l.2559–2585, the normal path), `start_play_now` (~l.3139, manual start) and `start_game_clock` (~l.2842, SD arm
  ~l.2860). `update()` also has an "Impossible state: in sudden death with clock counting down" arm (~l.2564) — that
  arm is where the new 0:00 expiry belongs.
- `handle_rugby_pen_shot_end` (~l.2807, rugby only) treats a sudden-death shot end with the clock stopped as an
  error; see the rugby call above. Hockey penalty shots stop the game clock, so they cannot run past 0:00.
- **Outside the tournament manager** (a search of that file alone misses these):
  - `uwh-common/src/game_snapshot.rs`: `time_elapsed_at` (~l.228) and `time_between` (~l.243–254) treat sudden death
    as counting up (penalty timing depends on them); `next_period_dur` returns `None` after `PreSuddenDeath`.
  - `refbox/src/app/mod.rs` ~l.1958 and ~l.1984: the automatic buzzer logic excludes `SuddenDeath` entirely
    (`end_stops_play` is false for it). **P2's buzzer at 0:00 does not happen without changing this.**
  - `refbox/src/app/update_sender.rs` ~l.514–523: the panel's hide-time display treats `PreSuddenDeath` and
    `SuddenDeath` as having no timed next period.
- Score-confirm pause on a sudden-death goal (~l.3298: `clock_at_pause`, `time_into_sd`) stores time **into** sudden
  death; a rejected/edited score that resumes sudden death must resume the **remaining** time when counting down.
- `set_scores` (~l.452) ends the game on an unequal sudden-death score — unchanged.
- **The 0:00 expiry is the new seam.** Read `reference: end_game is NOT the end-of-game seam` before touching it: in
  the app the normal game end goes `could_end_game → pause_for_confirm → end_confirm_pause → end_game`, and
  `update()`'s own `end_game` arm is unreachable for a normal game. A change only in `end_game` ships green and does
  nothing. Prove the expiry through the app's updater path, not by injecting messages.
- Ref timeouts during sudden death stop the clock as today; the remaining time resumes.

## Agreement with the portal (to be confirmed through the coordinators)

- **Field: AGREED 2026-11-16.** `suddenDeathMaxDuration`, integer seconds, optional (absent = no maximum, `0`
  refused). Confirmed by the portal design session (via both coordinators, checked against portal `origin/main`
  `9cf0e7a2fe`): the same name end to end on the portal — API model, stored rule, website types, Schedule Builder CSV
  column, website refbox game state. The refbox may merge with it. Portal-side work is registered by the portal's
  design lane.
- **Ordering:** none required. The refbox ignores unknown portal fields (no `deny_unknown_fields` on `TimingRule`),
  and a portal that does not send the field means "no maximum" = today's behaviour. Either side can ship first.
- **A level result:** the refbox sends a level final score, exactly as it does today for a level game with sudden
  death off. The portal's tie-breakers / `TiedGameCoinFlip` take it from there; no new result flag.
- **Portal displays and the website refbox:** anything on the portal that shows the refbox's sudden-death clock must
  accept a countdown. The website refbox copies this behaviour (its own lane).

## Acceptance (what the PO can observe)

1. Settings page, no portal: SUDDEN DEATH opens the new page; choosing YES + SET 5:00 + APPLY shows `MAX` / `5:00`
   on the button, unclipped. NONE shows `YES`; NO shows `NO`.
2. A level game reaching sudden death with a 5:00 maximum: the screen says SUDDEN DEATH and the clock counts down
   from 5:00; it flashes in the last 10 seconds.
3. A goal before 0:00 ends the game as today.
4. No goal by 0:00: the buzzer sounds, the game ends level, the break to the next game begins, and the portal
   receives a level score.
5. With no maximum: sudden death counts up and ends only on a goal, as today.
6. LED panel and overlay show SUDDEN DEATH with the counting-down time.

## Left to the builder

- Field and message names inside the refbox; how the new page's state is staged (follow
  `reference: new settings value needs snapshot and commit site` — APPLY enable, revert, and every escape path).
- The exact sizes for the two-line `MAX` value, provided nothing clips in any locale.
- Test design (pin the rules in pure functions where a test cannot see a returned `Task`).
