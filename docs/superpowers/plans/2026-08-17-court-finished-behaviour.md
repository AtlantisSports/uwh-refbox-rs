# Court-finished behaviour — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the refbox derive "is this court finished?" from a persisted fact (which game was
last played) instead of storing the conclusion or guessing it, so a restart, a refresh or an
offline launch can never replay a finished court.

**Architecture:** The link note (`portal_link.json`) gains `last_played` — the anchor — alongside
the game the operator is currently on. Every path that needs "what is next" asks one function,
which searches this court's schedule for the first game after the anchor. Four guess-paths are
deleted outright; the engine is taught to report a blank next-game number rather than invent one
while it is linked to a schedule.

**Tech Stack:** Rust 2024, iced 0.13, serde/serde_json, `time`, tokio. Crates touched: `refbox`
only (see "Blast radius" below).

**Spec:** `docs/superpowers/specs/2026-08-17-court-finished-behaviour-design.md`
**Decisions record:** `docs/superpowers/specs/2026-08-17-court-finished-behaviour-decisions.md`
**Branch:** `fix/refbox/court-finished-behaviour`, based on `df772fec`
(`fix/uwh-common/no-next-game-on-court`, 21 commits, never pushed — this branch supersedes the
code it touches and the two ship as one PR).

---

## Global Constraints

- **MSRV Rust 1.85, edition 2024.** No APIs newer than 1.85.
- **Clippy `-D warnings`**, all targets, all features. `just lint` before every commit.
- **No `unwrap()`/`expect()` in production code** without a comment saying why it cannot panic.
- **No new dependencies.** Everything needed is already in `refbox`'s `Cargo.toml`.
- **Heavy process** (`.claude/rules/plan-execution.md`): per-task verification, per-task code
  review, strict deviation tracking. Trigger is blast radius — this changes the game-clock state
  machine.
- **Run cargo from inside the worktree**, not the shared checkout:
  `cd /home/estraily/projects/uwh-refbox-rs/.claude/worktrees/fix+refbox+court-finished-behaviour`
- **The five rules govern every decision.** If a task looks like it is adding a guard rather than
  removing a guess, it has gone wrong — stop and re-read the spec.
- **Never widen scope to `uwh-common`, `overlay` or the LED panel.** See "Blast radius".

### Blast radius — what this actually touches

The spec named `uwh-common` because the *dependency branch* changed it (`game_snapshot.rs`,
`uwhportal/schedule.rs`). Those changes are already in the base commit and are correct as they
stand. **This plan adds no `uwh-common` change.** If a task appears to need one, that is a signal
to stop and check with the human first. Heavy process still applies, because the game-clock state
machine (`refbox/src/tournament_manager/mod.rs`) is squarely in scope.

---

## File Structure

| File | Responsibility after this change |
|---|---|
| `refbox/src/portal_manager/link_session.rs` | The on-disk note. Now v2: event, court, **`current_game`**, **`last_played`** (+ its start time), mode, timestamp. Reads v1 notes too. |
| `refbox/src/tournament_manager/mod.rs` | The engine. Knows whether it is schedule-linked; never invents a next-game number when it is. One predicate answers "can a game legitimately start next?" |
| `refbox/src/app/mod.rs` | Holds the live anchor, advances it only on a recorded result, persists it, and asks one decision function what is next. The three deleted guess-paths live here. |
| `refbox/tests/features/court-finished.feature` | Written walkthrough scenarios. Two are wrong by decision and get rewritten. |
| `docs/superpowers/plans/2026-08-16-no-next-game-on-court-finish.md` | Walkthrough scenario 8 corrected for the same reason. |

---

## Two planning decisions that go beyond the literal spec

**Flag these to the human before Task 1 starts.** Both are judgement calls made while reading the
code; neither is in the spec text.

1. **The note stores the anchor's start time as well as its number.** Rule 2's search is "the next
   game on this court whose start time is after `last_played`'s". If the anchor game is itself
   removed from the schedule — the "game removed / moved away" row — its start time is no longer
   discoverable, and the search would have to answer "Unknown" instead of "show whatever is
   genuinely next". Persisting `last_played_start` alongside `last_played` closes that hole for
   one extra field. **Recommendation: include it.**

2. **"Ask the operator" adds no new UI — SETTLED 2026-08-17.** The spec says the three "nothing is
   next" states are displayed identically — no upcoming game, clock stopped, START NOW greyed and
   inert — and criterion 10 says the refbox "offers nothing, asks for a pick". "Ask" means *offer
   nothing and let the operator pick in Settings*, which is the existing affordance (feature-file
   scenario "START NOW is unavailable until a game is picked" already describes it). No dialog, no
   new screen, no new translated wording. A visible prompt would be additive and is not in scope.

### A court with no *games* versus a court with no *history*

Human correction, 2026-08-17: **the court picker only ever offers courts that appear in the
schedule** (`event.courts` is built from `schedule.games.values()`), so a court with no games at
all cannot be selected. `NothingScheduled` is therefore unreachable from the picker, and reachable
only one way: the operator is already on a court and a schedule change moves every game off it —
decision 20, "court closed mid-day, games moved elsewhere". Keep the state and its test; do not
expect to reach it by selecting an empty court.

"No **history**" is a different and genuinely common situation: the court exists and has games, but
*this refbox* has no record of playing one on it. It arises on the first launch of a morning, on a
replacement box brought out mid-day, and whenever the operator changes court (decision 21). That is
what `NeedsPick` is for.

---

## The model, in one place

Four inputs decide what is next. In priority order:

| # | Condition | Answer |
|---|---|---|
| 1 | A startup restore of `current_game` is outstanding | that game |
| 2 | The engine already holds a next game (the operator's pick, or the one set at the last kickoff) | that game |
| 3 | Schedule available, `last_played` known | first game on **this court** after the anchor's start time → `Game`; none → `CourtFinished` |
| 4 | Schedule available, this court has no games at all | `NothingScheduled` |
| 5 | Schedule available, `last_played` absent | `NeedsPick` |
| 6 | No usable schedule | `Unknown` — show `current_game` if the engine was seeded with it, else nothing |

`last_played` advances **only** when a game ends with a recorded result. Abandoned and interrupted
games leave it alone, which is what re-offers the same game.

Rows 3–6 all display identically. They are kept apart internally so an empty court or an unknown
one is never mistaken for a completed one — that conflation is what caused the original defects.

---

## Task 1: The note gains the anchor (`LinkSessionFile` v2)

**Files:**
- Modify: `refbox/src/portal_manager/link_session.rs:31-45` (struct + version), `:58-80` (loader)
- Test: same file, `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `LinkSessionFile { version: u32, event_id: EventId, court: Option<String>,
  current_game: Option<GameNumber>, last_played: Option<GameNumber>,
  last_played_start: Option<OffsetDateTime>, mode: Mode, last_active: OffsetDateTime }`,
  `LinkSessionFile::CURRENT_VERSION == 2`. Tasks 3, 4 and 5 read and write these fields.

**Why v1 notes must still load:** a version bump that quarantines the old note would force every
operator to re-select their game on upgrade day, mid-tournament. A v1 note carries `game_number`,
which is exactly `current_game`, so it migrates for free with `last_played = None`. A v1 note that
recorded the *old* finished encoding (court present, no game number) migrates to
"no current game, no anchor" → `NeedsPick`. That is the safe direction: the old encoding degrades
to "ask", never to "replay".

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests`:

```rust
#[test]
fn v1_note_migrates_game_number_to_current_game() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("portal_link.json");
    // Exactly the v1 shape: `game_number`, no `last_played`.
    let v1 = r#"{"version":1,"event_id":"events/2113-A","court":"1",
                 "game_number":"G27","mode":"Hockey6V6",
                 "last_active":"2026-08-17T09:00:00Z"}"#;
    std::fs::write(&path, v1).unwrap();
    let note = load_or_none(tmp.path()).unwrap().expect("v1 note must still load");
    assert_eq!(note.current_game, Some("G27".to_string()));
    assert_eq!(note.last_played, None);
    assert_eq!(note.last_played_start, None);
    assert!(path.exists(), "a v1 note must not be quarantined");
}

#[test]
fn v1_finished_encoding_migrates_to_no_history() {
    // v1 recorded a finished court as "court, but no game number". Under v2 that
    // is not a conclusion any more: no current game and no anchor, which the
    // decision function answers with NeedsPick — never a replay.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("portal_link.json");
    let v1 = r#"{"version":1,"event_id":"events/2113-A","court":"1",
                 "game_number":null,"mode":"Hockey6V6",
                 "last_active":"2026-08-17T09:00:00Z"}"#;
    std::fs::write(&path, v1).unwrap();
    let note = load_or_none(tmp.path()).unwrap().unwrap();
    assert_eq!(note.current_game, None);
    assert_eq!(note.last_played, None);
}

#[test]
fn v2_round_trips_the_anchor() {
    let tmp = TempDir::new().unwrap();
    let note = LinkSessionFile {
        version: LinkSessionFile::CURRENT_VERSION,
        event_id: EventId::from_full("events/2113-A").unwrap(),
        court: Some("1".to_string()),
        current_game: None,
        last_played: Some("G27".to_string()),
        last_played_start: Some(datetime!(2026-08-17 14:00:00 UTC)),
        mode: Mode::Hockey6V6,
        last_active: datetime!(2026-08-17 14:22:03 UTC),
    };
    save(tmp.path(), &note).unwrap();
    assert_eq!(load_or_none(tmp.path()).unwrap(), Some(note));
}

#[test]
fn a_future_version_is_still_quarantined() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("portal_link.json");
    let mut note = sample(OffsetDateTime::now_utc());
    note.version = 999;
    std::fs::write(&path, serde_json::to_string(&note).unwrap()).unwrap();
    assert!(load_or_none(tmp.path()).unwrap().is_none());
    assert!(!path.exists());
}
```

Update the existing `sample()` helper to the new field names (`current_game: Some("G27"…)`,
`last_played: None`, `last_played_start: None`). The existing
`unknown_version_is_renamed_and_none_returned` test is superseded by
`a_future_version_is_still_quarantined` — delete it rather than leave two tests asserting
different things about version handling.

- [ ] **Step 2: Run the tests to verify they fail**

```
cargo test -p refbox --lib portal_manager::link_session
```
Expected: compile error — `current_game`, `last_played`, `last_played_start` do not exist.

- [ ] **Step 3: Implement**

```rust
/// The remembered live portal link, persisted next to `portal_queue.json`.
///
/// v2 records a **fact** (which game was last played) rather than a conclusion
/// ("this court is finished"). A fact stays true across restarts and refreshes;
/// a conclusion goes stale, which is what replayed a finished court.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSessionFile {
    pub version: u32,
    pub event_id: EventId,
    pub court: Option<String>,
    /// The game the operator is on right now — in progress, or the confirmed
    /// upcoming game. **Absent whenever nothing is next**, which is what makes an
    /// offline restart show the finished state instead of a phantom game.
    /// Read from v1 notes under its old name.
    #[serde(alias = "game_number")]
    pub current_game: Option<GameNumber>,
    /// The game most recently played to a recorded result on this court: the
    /// anchor the schedule search starts from. Absent when the refbox holds no
    /// history for this court, which always requires an operator pick.
    #[serde(default)]
    pub last_played: Option<GameNumber>,
    /// The anchor's scheduled start. Persisted rather than looked up so the
    /// search still works when the anchor game itself has been removed from the
    /// schedule.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_played_start: Option<OffsetDateTime>,
    pub mode: Mode,
    #[serde(with = "time::serde::rfc3339")]
    pub last_active: OffsetDateTime,
}

impl LinkSessionFile {
    pub const CURRENT_VERSION: u32 = 2;
}
```

In `load_or_none`, accept any version up to the current one instead of only an exact match:

```rust
    match serde_json::from_slice::<LinkSessionFile>(&bytes) {
        // v1 notes migrate in place: `game_number` reads as `current_game` via
        // the serde alias, and the anchor fields default to absent. Forcing a
        // re-pick on upgrade day, mid-tournament, would be worse than the
        // one-day gap in history that this leaves.
        Ok(note) if note.version <= LinkSessionFile::CURRENT_VERSION => Ok(Some(note)),
        Ok(note) => {
            log::error!(
                "portal_link.json has unknown version {}; renaming and ignoring",
                note.version
            );
            rename_corrupt(&path)?;
            Ok(None)
        }
        Err(e) => { /* unchanged */ }
    }
```

Note: `save` always stamps `CURRENT_VERSION` via its caller, so a migrated v1 note is rewritten as
v2 the first time anything persists. Verify the caller in Task 4 sets `version:
LinkSessionFile::CURRENT_VERSION` (it already does).

- [ ] **Step 4: Run the tests to verify they pass**

```
cargo test -p refbox --lib portal_manager::link_session
```
Expected: PASS, including the pre-existing round-trip, corrupt-file and freshness tests.

- [ ] **Step 5: Fix the compile fallout at the two existing call sites**

`refbox/src/app/mod.rs:1605` (write) and `:2729` (read) name `game_number`. Rename to
`current_game` and add `last_played: None, last_played_start: None` at the write site as a
placeholder — Task 4 fills them in properly. The read site's
`new.pending_restore_game = note.game_number.clone()` becomes `note.current_game.clone()`.

- [ ] **Step 6: Verify and commit**

```
cargo clippy -p refbox --all-targets -- -D warnings && cargo test -p refbox --lib
git add refbox/src/portal_manager/link_session.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): record the last game played in the link note"
```

---

## Task 2: The engine never invents a next game while schedule-linked

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — struct `:40-67`, `new()` `:70-95`,
  `next_game_number()` `:208-229`, and the five gates at `:1169` (`end_game`), `:1371`
  (`pause_has_ended`), `:1664` (`start_game_clock`), `:1913` (`start_play_now`), `:2191`
  (`end_confirm_pause`)
- Test: same file, `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `TournamentManager::set_schedule_linked(&mut self, linked: bool)` and the private
  `fn no_startable_next_game(&self) -> bool`. Task 5 calls the setter.

**What this deletes and why:** `next_game_number()` currently answers `game_number + 1` whenever it
has no next-game info. On a multi-court event that number is another court's game, and after an
offline restart it is the phantom that played a game unattended and queued a 0–0. Sequential
numbering is the *specification* in manual mode and a *guess* in portal mode, so the engine has to
know which it is in.

**The gates matter as much as the number.** Today the "hold the clock" gates test `no_next_game`,
which is only set when the app has positively established the court is finished. A schedule-linked
engine with no next game — offline restart, schedule not yet arrived — passes those gates and
would start a game with a blank number. One predicate covers both cases.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_schedule_linked_engine_reports_no_next_number_rather_than_guessing() {
    let mut tm = TournamentManager::new(Default::default());
    tm.set_game_number("6");
    tm.set_schedule_linked(true);
    // No schedule has named a next game. The old arithmetic answered "7" —
    // on a two-court event that is the other court's game.
    assert_eq!(tm.next_game_number(), "");
}

#[test]
fn manual_mode_still_numbers_sequentially() {
    let mut tm = TournamentManager::new(Default::default());
    tm.set_game_number("6");
    tm.set_schedule_linked(false);
    assert_eq!(tm.next_game_number(), "7");
}

#[test]
fn a_schedule_supplied_game_still_wins_when_linked() {
    let mut tm = TournamentManager::new(Default::default());
    tm.set_game_number("6");
    tm.set_schedule_linked(true);
    tm.set_next_game(NextGameInfo { number: "11".to_string(), timing: None, start_time: None });
    assert_eq!(tm.next_game_number(), "11");
}

#[test]
fn an_unparseable_manual_number_is_an_error_not_a_default() {
    // Manual game numbers come from a numeric keypad, so this cannot happen in
    // normal operation. If it ever does it is a bug, and the safe failure is to
    // start nothing — never to silently restart the day at game 1.
    let mut tm = TournamentManager::new(Default::default());
    tm.set_game_number("not-a-number");
    tm.set_schedule_linked(false);
    assert_eq!(tm.next_game_number(), "");
}

#[test]
fn a_schedule_linked_engine_with_no_next_game_refuses_start_play_now() {
    let mut tm = TournamentManager::new(Default::default());
    tm.set_schedule_linked(true);
    assert!(matches!(
        tm.start_play_now(Instant::now()),
        Err(TournamentManagerError::NoNextGameOnCourt)
    ));
}

#[test]
fn half_time_of_the_last_game_still_starts_the_second_half() {
    // Criterion 8, and the easiest thing here to break. The court is flagged
    // finished from the moment the last game STARTS, so every gate must test the
    // period as well: "finished" describes the schedule AFTER this game, and the
    // last game is often a final. Drive the engine into HalfTime using whatever
    // setup the neighbouring period tests already use — do not add a public
    // setter to the engine just to write this test.
    let mut tm = TournamentManager::new(Default::default());
    tm.set_schedule_linked(true);
    tm.set_no_next_game();
    // ... reach GamePeriod::HalfTime via the suite's existing helper ...
    assert!(tm.start_play_now(Instant::now()).is_ok());
}
```

**Regression risk to watch in this task.** All five gates are already period-aware —
`start_game_clock` and `end_game` test `BetweenGames`, and `start_play_now` matches the
`BetweenGames` arm only. Widening the *flag* half of each condition must not widen the *period*
half, or the half-time, pre-overtime and pre-sudden-death breaks of the last game on a court lose
their whistle and their START NOW. That is criterion 8, and it is a rule the spec wrote down
explicitly because it is the easiest thing for a future change to break.

- [ ] **Step 2: Run the tests to verify they fail**

```
cargo test -p refbox --lib tournament_manager
```
Expected: compile error — `set_schedule_linked` does not exist.

- [ ] **Step 3: Implement**

Add the field next to `no_next_game`, defaulting to `false` in `new()`:

```rust
    /// True while the refbox is linked to a schedule source (portal or custom
    /// site). Sequential numbering is the specification in manual mode, where
    /// there is no schedule to contradict it; while linked, a number no schedule
    /// supplied is a guess, and a guessed number is another court's game.
    schedule_linked: bool,
```

```rust
    pub fn set_schedule_linked(&mut self, linked: bool) {
        if self.schedule_linked != linked {
            info!("Schedule-linked set to {linked}");
        }
        self.schedule_linked = linked;
    }

    /// No game can legitimately be started next: either the court is known to be
    /// finished, or we are linked to a schedule that has not named one. Both mean
    /// the clock must hold rather than count down toward a game that is not
    /// coming — and neither may be answered with arithmetic.
    fn no_startable_next_game(&self) -> bool {
        self.next_game.is_none() && (self.no_next_game || self.schedule_linked)
    }
```

Rewrite `next_game_number()`:

```rust
    pub fn next_game_number(&self) -> GameNumber {
        if let Some(ref info) = self.next_game {
            return info.number.clone();
        }

        if self.no_next_game || self.schedule_linked {
            // Blank means "no next game on this court" — `GameSnapshot::next_game_number`
            // reports `None` for it. Guessing here would name another court's game.
            return GameNumber::new();
        }

        match self.game_number.parse::<u32>() {
            Ok(num) => (num + 1).to_string(),
            Err(_) => {
                // Manual game numbers come from a numeric keypad, so reaching this
                // is a bug, not a runtime condition. Report it and start nothing;
                // the old "default to 1" silently restarted the day.
                error!(
                    "Manual game number '{}' is not an integer; refusing to name a next game",
                    self.game_number
                );
                GameNumber::new()
            }
        }
    }
```

Replace `self.no_next_game` with `self.no_startable_next_game()` at exactly these five gates —
`end_game:1169`, `pause_has_ended:1371`, `start_game_clock:1664`, `start_play_now:1913`,
`end_confirm_pause:2191`. **Do not touch** the assignments at `:183`, `:239`, `:248` and `:1235`;
those are setters and the start-of-game reset, and they must keep operating on the raw flag.

Update each gate's existing comment to say "no next game this engine can identify" rather than
"the selected court has no further games" — the condition is now broader than a finished court.

- [ ] **Step 4: Run the tests to verify they pass**

```
cargo test -p refbox --lib tournament_manager
```
Expected: PASS. **The whole existing `tournament_manager` suite must stay green** — it is the
regression net for the clock. If a pre-existing test now fails, stop: either the gate change is
wrong, or that test encodes the arithmetic this task deletes. Record which, in Deviations, before
changing any existing assertion.

- [ ] **Step 5: Verify and commit**

```
cargo clippy -p refbox --all-targets -- -D warnings && cargo test -p refbox --lib
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): stop the engine inventing a game number while schedule-linked"
```

---

## Task 3: The app holds, advances and persists the anchor

**Files:**
- Modify: `refbox/src/app/mod.rs` — struct fields near `:192`, constructor near `:2673`,
  `persist_link_session` `:1590-1622`, `handle_game_end` `:1496-1549`, the restore block
  `:2715-2750`, `clear_portal_selections_to_manual` (around `:1770`), and the court/event switch
  paths
- Test: `refbox/src/app/mod.rs`, new `#[cfg(test)] mod anchor_tests`

**Interfaces:**
- Consumes: `LinkSessionFile { current_game, last_played, last_played_start }` from Task 1.
- Produces: `RefBoxApp.last_played: Option<GameNumber>`,
  `RefBoxApp.last_played_start: Option<OffsetDateTime>`, and the free function
  `fn anchor_after_game_end(recorded: Option<&GameNumber>, ended: &GameNumber, scheduled_start:
  Option<OffsetDateTime>, current: (Option<GameNumber>, Option<OffsetDateTime>)) ->
  (Option<GameNumber>, Option<OffsetDateTime>)`. Task 4 reads the two fields.

**Where the anchor advances:** `handle_game_end` already distinguishes the three cases — a result
recorded *for this game*, a result recorded for a different game, and no result at all. Only the
first advances the anchor. That is decision 18 (an abandoned game has not happened as far as the
tournament is concerned, so the same game is offered again) landing for free on a seam that already
exists.

**Where the anchor is written to disk:** `persist_link_session` runs on Apply, on portal-off and on
the ~5-minute health heartbeat. Five minutes is far too slow for acceptance criterion 2, which
closes and reopens the app immediately after the last game. `handle_game_end` must persist
directly.

- [ ] **Step 1: Write the failing test**

The anchor-advance rule is extracted into a free function so it is testable without the app —
`update()`/`apply_*` are not unit-testable in this codebase.

```rust
#[cfg(test)]
mod anchor_tests {
    use super::anchor_after_game_end;
    use time::macros::datetime;

    #[test]
    fn a_recorded_result_advances_the_anchor() {
        let start = datetime!(2026-08-17 14:00:00 UTC);
        let got = anchor_after_game_end(
            Some(&"6".to_string()), &"6".to_string(), Some(start), (None, None));
        assert_eq!(got, (Some("6".to_string()), Some(start)));
    }

    #[test]
    fn an_abandoned_game_leaves_the_anchor_alone() {
        // No result was recorded, so as far as the tournament is concerned the
        // game has not happened. Re-offering it costs seconds; skipping it loses
        // the result and only surfaces at reconciliation.
        let prev_start = datetime!(2026-08-17 13:00:00 UTC);
        let current = (Some("5".to_string()), Some(prev_start));
        let got = anchor_after_game_end(
            None, &"6".to_string(), Some(datetime!(2026-08-17 14:00:00 UTC)), current.clone());
        assert_eq!(got, current);
    }

    #[test]
    fn a_result_recorded_for_a_different_game_leaves_the_anchor_alone() {
        let prev_start = datetime!(2026-08-17 13:00:00 UTC);
        let current = (Some("5".to_string()), Some(prev_start));
        let got = anchor_after_game_end(
            Some(&"5".to_string()), &"6".to_string(),
            Some(datetime!(2026-08-17 14:00:00 UTC)), current.clone());
        assert_eq!(got, current);
    }

    #[test]
    fn an_unscheduled_game_still_advances_the_number() {
        // A game the schedule does not know still counts as played. The search
        // then falls back to looking the anchor's time up, and answers Unknown if
        // it cannot — never a guess.
        let got = anchor_after_game_end(
            Some(&"6".to_string()), &"6".to_string(), None, (None, None));
        assert_eq!(got, (Some("6".to_string()), None));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```
cargo test -p refbox --lib anchor_tests
```
Expected: compile error — `anchor_after_game_end` not found.

- [ ] **Step 3: Implement**

```rust
/// The anchor after a game leaves the clock: advanced only when the result that
/// was recorded belongs to the game that just ended.
///
/// `recorded` is the game number the newest recorded result belongs to, or `None`
/// when no result was recorded at all. Both non-matching cases mean the game was
/// abandoned or interrupted, and the anchor must not move — the same game is
/// offered again, which is recoverable, where skipping it is not.
fn anchor_after_game_end(
    recorded: Option<&GameNumber>,
    ended: &GameNumber,
    scheduled_start: Option<time::OffsetDateTime>,
    current: (Option<GameNumber>, Option<time::OffsetDateTime>),
) -> (Option<GameNumber>, Option<time::OffsetDateTime>) {
    if recorded_result_matches_ended_game(recorded, ended) {
        (Some(ended.clone()), scheduled_start)
    } else {
        current
    }
}
```

Add the two fields to `RefBoxApp` beside `pending_restore_game`, documented as the anchor, and
initialise both to `None` in the constructor.

In `handle_game_end`, inside the branch that already matches a recorded result to the ended game,
advance and persist:

```rust
                    let scheduled_start = self
                        .schedule
                        .as_ref()
                        .and_then(|s| s.games.get(game_number))
                        .map(|g| g.start_time);
                    let (anchor, anchor_start) = anchor_after_game_end(
                        Some(&recorded_game),
                        game_number,
                        scheduled_start,
                        (self.last_played.clone(), self.last_played_start),
                    );
                    self.last_played = anchor;
                    self.last_played_start = anchor_start;
```

Then, at the end of `handle_game_end` and outside the `uses_remote()` block, persist unconditionally
— the note write is the whole point of the task, and a five-minute heartbeat is not good enough:

```rust
        // Write the anchor down now. Acceptance criterion 2 closes and reopens the
        // app seconds after the last game ends; the health-tick heartbeat that
        // normally refreshes the note is ~5 minutes away.
        self.persist_link_session();
```

In `persist_link_session`, fill the two new fields from `self`:

```rust
                    last_played: self.last_played.clone(),
                    last_played_start: self.last_played_start,
```

In the startup restore block, seed them from the note:

```rust
                    new.last_played = note.last_played.clone();
                    new.last_played_start = note.last_played_start;
```

Clear both wherever the event or court changes, and on portal-off — the anchor is per-event and
per-court, and a carried-over anchor points at a real but wrong game and looks entirely plausible
(decision 25):
- `clear_portal_selections_to_manual` (around `:1770`) — portal switched off
- the Apply paths that commit a new `current_event_id` or `current_court`
  (`apply_app_options` and `apply_game_options`): clear if either differs from the live value,
  **before** the new value is committed.

- [ ] **Step 4: Run the test to verify it passes**

```
cargo test -p refbox --lib anchor_tests
```
Expected: PASS.

- [ ] **Step 5: Verify and commit**

```
cargo clippy -p refbox --all-targets -- -D warnings && cargo test -p refbox --lib
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): advance the schedule anchor only on a recorded result"
```

---

## Task 4: Rewrite the decision function, delete the single-use flag, wire the new states

**Files:**
- Modify: `refbox/src/app/mod.rs` — `enum NextGameFromSchedule` `:7050-7061`,
  `next_game_from_schedule` `:7063-7132`, its test module `:7134+`, the `RecvSchedule` call site
  `:5240-5310`, and every `pending_restore_court_finished` site (`:198`, `:1421`, `:1631`, `:1804`,
  `:2674`, `:2735`, `:5245`)
- Test: `refbox/src/app/mod.rs`, `mod refresh_next_game_tests`

**Interfaces:**
- Consumes: `RefBoxApp.last_played` / `last_played_start` (Task 3).
- Produces: `enum NextGameFromSchedule { Game(GameNumber), CourtFinished, NothingScheduled,
  NeedsPick, Unknown }` and `next_game_from_schedule(schedule, restore_num, engine_next,
  last_played, last_played_start, court)`.

**These three deletions are the point of the whole branch. Delete; do not guard.**

1. `pending_restore_court_finished` and its `std::mem::take` — a one-shot flag holding a
   *conclusion*. Once consumed it fell through to "offer the earliest game", which is scenario 4's
   Critical. The anchor makes it superfluous: a restart derives the finished state fresh, every
   time, from a fact.
2. The `anchor_num == "0"` → "offer the earliest game on this court" branch — the direct cause of
   that Critical. Rule 3 leaves it no legitimate caller.
3. The `anchor_num: &GameNumber` parameter itself — replaced by `last_played: Option<&GameNumber>`.
   "No history" and "anchor is game 6" are now distinct values rather than both collapsing to
   `"0"`, which is what made the branch look reasonable in the first place.

**`pending_restore_game` stays, and stays one-shot.** This is the distinction that matters: it
restores a *fact* (the game the operator was on), not a conclusion, and what it now falls through
to — deriving from the anchor — is correct and idempotent. That is why consuming it is safe here
and consuming the finished flag was not. It must stay one-shot, or the feature file's
"a remembered game does not resurrect itself at the end of the day" scenario breaks.

- [ ] **Step 1: Write the failing tests**

Rewrite `mod refresh_next_game_tests` against the new signature. Keep the existing
`two_court_schedule()` helper (court 1 holds games 9 and 11; court 2 holds 10 and 12) and the
existing tests for restore-wins and pick-survives-a-refresh, adjusted to the new arguments. Add:

```rust
    #[test]
    fn the_anchor_finds_the_next_game_on_this_court() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule, None, None, Some(&"9".to_string()), None, Some("Court 1")),
            NextGameFromSchedule::Game("11".to_string())
        );
    }

    #[test]
    fn nothing_after_the_anchor_is_a_finished_court() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule, None, None, Some(&"11".to_string()), None, Some("Court 1")),
            NextGameFromSchedule::CourtFinished
        );
    }

    #[test]
    fn the_same_answer_however_many_times_it_is_asked() {
        // Scenario 4's Critical: the old one-shot flag was consumed on the first
        // refresh and the second re-adopted game 1. Nothing is consumed here.
        let schedule = two_court_schedule();
        for _ in 0..5 {
            assert_eq!(
                next_game_from_schedule(
                    &schedule, None, None, Some(&"11".to_string()), None, Some("Court 1")),
                NextGameFromSchedule::CourtFinished
            );
        }
    }

    #[test]
    fn no_anchor_asks_the_operator_rather_than_offering_the_earliest_game() {
        // Supersedes decision 9. A replacement box brought out mid-day has no
        // anchor and would have been confidently offered game 9, played hours ago.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(&schedule, None, None, None, None, Some("Court 1")),
            NextGameFromSchedule::NeedsPick
        );
    }

    #[test]
    fn a_court_with_no_games_is_not_a_finished_court() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(&schedule, None, None, None, None, Some("Court 7")),
            NextGameFromSchedule::NothingScheduled
        );
    }

    #[test]
    fn a_game_added_to_a_finished_court_is_found_by_the_next_search() {
        let mut schedule = two_court_schedule();
        schedule.games.insert(
            "13".to_string(),
            game_at("13", "Court 1", time::macros::datetime!(2026-08-05 11:00 UTC)),
        );
        assert_eq!(
            next_game_from_schedule(
                &schedule, None, None, Some(&"11".to_string()), None, Some("Court 1")),
            NextGameFromSchedule::Game("13".to_string())
        );
    }

    #[test]
    fn a_removed_anchor_still_searches_from_its_remembered_start_time() {
        // The anchor game was moved to another court. Its number is gone from the
        // schedule, but its start time was written down, so the search still
        // answers what is genuinely next here.
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule,
                None,
                None,
                Some(&"absent".to_string()),
                Some(time::macros::datetime!(2026-08-05 09:00 UTC)),
                Some("Court 1"),
            ),
            NextGameFromSchedule::Game("11".to_string())
        );
    }

    #[test]
    fn an_unjudgeable_anchor_is_unknown_never_a_guess() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(
                &schedule, None, None, Some(&"absent".to_string()), None, Some("Court 1")),
            NextGameFromSchedule::Unknown
        );
    }

    #[test]
    fn no_court_selected_is_unknown() {
        let schedule = two_court_schedule();
        assert_eq!(
            next_game_from_schedule(&schedule, None, None, Some(&"9".to_string()), None, None),
            NextGameFromSchedule::Unknown
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```
cargo test -p refbox --lib refresh_next_game_tests
```
Expected: compile errors — wrong arity, and `NeedsPick`/`NothingScheduled` do not exist.

- [ ] **Step 3: Implement the decision function**

```rust
/// What a schedule says is next on this court.
///
/// The last three are all "nothing is next" and are **displayed identically**.
/// They are kept apart so an empty court or an unreadable schedule is never
/// mistaken for a completed one — that conflation is what caused the original
/// defects.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NextGameFromSchedule {
    Game(GameNumber),
    /// The schedule was read and holds nothing after the anchor.
    CourtFinished,
    /// The schedule was read and this court has no games at all.
    NothingScheduled,
    /// The schedule was read but the refbox holds no history for this court, so
    /// it cannot know which games are already played. Requires an operator pick.
    NeedsPick,
    /// Nothing can be judged: no court selected, or no usable anchor.
    Unknown,
}

/// Decide what a freshly-received schedule means for the upcoming game.
///
/// The refbox never invents a game: every answer here comes from the schedule or
/// from the operator. In priority order:
///
/// 1. A startup restore re-selects the game the operator was on. A fact about
///    where they were, applied once, to bootstrap the display.
/// 2. The game the engine already holds wins over any search: the operator may
///    have picked a game out of order, and a refresh must not silently replace
///    that choice with the next game in schedule order.
/// 3. Otherwise search this court for the first game after the anchor — the game
///    last played to a recorded result. Nothing after it means the court is
///    finished, and that answer is the same however many times it is asked,
///    because nothing is consumed to produce it.
/// 4. With no anchor there is no safe automatic answer: a court the refbox holds
///    no record for is a fresh morning or a replacement box mid-day, and it
///    cannot tell them apart. Ask.
fn next_game_from_schedule(
    schedule: &Schedule,
    restore_num: Option<&GameNumber>,
    engine_next: Option<&GameNumber>,
    last_played: Option<&GameNumber>,
    last_played_start: Option<time::OffsetDateTime>,
    court: Option<&str>,
) -> NextGameFromSchedule {
    if let Some(num) = restore_num {
        return NextGameFromSchedule::Game(num.clone());
    }

    if let Some(num) = engine_next {
        return NextGameFromSchedule::Game(num.clone());
    }

    let Some(court) = court else {
        return NextGameFromSchedule::Unknown;
    };

    if !schedule.games.values().any(|game| game.court == court) {
        return NextGameFromSchedule::NothingScheduled;
    }

    let Some(anchor) = last_played else {
        return NextGameFromSchedule::NeedsPick;
    };

    // Prefer the schedule's own copy of the anchor's start time; fall back to the
    // one written down when it was played, so a game moved off this court since
    // does not blind the search.
    let anchor_start = schedule
        .games
        .get(anchor)
        .map(|game| game.start_time)
        .or(last_played_start);

    match anchor_start {
        Some(start) => match schedule.next_game_on_court(court, start) {
            Some(game) => NextGameFromSchedule::Game(game.number.clone()),
            None => NextGameFromSchedule::CourtFinished,
        },
        // The anchor is not in this schedule and no start time was remembered for
        // it, so nothing can be judged. Not a guess, and not "finished".
        None => NextGameFromSchedule::Unknown,
    }
}
```

- [ ] **Step 4: Delete the single-use flag**

Remove the `pending_restore_court_finished` field (`:198`) and all six other references. At `:2735`
the whole derivation goes — a note with a court and no game no longer means "finished", it means
"nothing was next when we last looked", and the anchor decides. At `:1421`, `:1631` and `:1804`
delete only the flag line and trim the comments that mention "the remembered-finished flag"; keep
`self.pending_restore_game = None` and the rest of each comment intact.

- [ ] **Step 5: Wire the new states at the `RecvSchedule` call site**

At `:5244-5309`, drop the `std::mem::take` of the flag and pass the anchor instead:

```rust
                                        let restore_num = self.pending_restore_game.take();
                                        // Safety: `self.schedule` was assigned from `schedule` two lines above.
                                        let schedule = self.schedule.as_ref().unwrap();
                                        let was_court_finished = tm.next_game_number().is_empty();
                                        let decision = next_game_from_schedule(
                                            schedule,
                                            restore_num.as_ref(),
                                            tm.next_game_info().as_ref().map(|info| &info.number),
                                            self.last_played.as_ref(),
                                            self.last_played_start,
                                            self.current_court.as_deref(),
                                        );
```

Match on the new states:

```rust
                                        let found = match decision {
                                            NextGameFromSchedule::Game(ref number) => {
                                                schedule.get_game_and_timing(number)
                                            }
                                            _ => (None, None),
                                        };
```

and, in place of the old `else if decision == NextGameFromSchedule::CourtFinished`:

```rust
                                        } else {
                                            match decision {
                                                // Both are definite "nothing is next
                                                // here" answers, and both park the
                                                // clock. Kept apart above so an empty
                                                // court is never recorded as a
                                                // completed one.
                                                NextGameFromSchedule::CourtFinished
                                                | NextGameFromSchedule::NothingScheduled => {
                                                    tm.set_no_next_game();
                                                }
                                                // Nothing is known well enough to act
                                                // on. Leave the engine as it is and let
                                                // the operator pick; with the portal
                                                // linked, `next_game_number` already
                                                // refuses to invent one.
                                                NextGameFromSchedule::NeedsPick
                                                | NextGameFromSchedule::Unknown => {}
                                                NextGameFromSchedule::Game(_) => {}
                                            }
                                        }
```

- [ ] **Step 6: Run the tests to verify they pass**

```
cargo test -p refbox --lib
```
Expected: PASS. Confirm with `grep -c pending_restore_court_finished refbox/src/app/mod.rs` that the
count is **0**.

- [ ] **Step 7: Verify and commit**

```
cargo clippy -p refbox --all-targets -- -D warnings && cargo test -p refbox --lib
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): derive the finished court from the anchor, not a one-shot flag"
```

---

## Task 5: Startup seeds the engine from the note, so an offline restart is right

**Files:**
- Modify: `refbox/src/app/mod.rs` — the startup restore block `:2715-2750` and wherever
  `commit_source` (`:1145`) settles the live source
- Test: `refbox/src/app/mod.rs`, extend `mod link_note_game_tests`

**Interfaces:**
- Consumes: `set_schedule_linked` (Task 2), `note.current_game` (Task 1).
- Produces: no new public surface.

**The defect this closes:** acceptance criterion 4. Today `pending_restore_game` is only consumed
when a schedule arrives. With the network off no schedule ever arrives, so the engine sits with no
next game, falls to `game_number + 1`, invents game 1, plays it unattended and queues a 0–0 that is
delivered on reconnect. Two changes remove it: the engine refuses to invent a number while linked
(Task 2), and the note's `current_game` is pushed into the engine at startup rather than held back
for a schedule that may never come.

**A finished court seeds nothing.** Its note has an anchor and no `current_game`, so there is
nothing to seed, and the engine — linked, with no next game — parks the clock and greys START NOW.
That is the finished state, reached with no network and no flag.

- [ ] **Step 1: Write the failing test**

`update()` is not unit-testable, so cover the engine contract that makes this work:

```rust
    #[test]
    fn a_linked_engine_seeded_with_a_remembered_game_reports_it() {
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_schedule_linked(true);
        // What startup does with a note that holds a current game, before any
        // schedule has arrived: timing and start time are unknown and stay unknown.
        tm.set_next_game(NextGameInfo {
            number: "11".to_string(), timing: None, start_time: None,
        });
        assert_eq!(tm.next_game_number(), "11");
        assert_eq!(link_note_game(&tm), LinkNoteGame::Write(Some("11".to_string())));
    }

    #[test]
    fn a_linked_engine_with_nothing_seeded_reports_no_game() {
        // The finished-court note: an anchor, no current game, nothing seeded.
        let mut tm = TournamentManager::new(GameConfig::default());
        tm.set_schedule_linked(true);
        assert_eq!(tm.next_game_number(), "");
        assert_eq!(link_note_game(&tm), LinkNoteGame::Write(None));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```
cargo test -p refbox --lib link_note_game_tests
```
Expected: FAIL on the second test — a fresh engine's `next_game_number()` is `"1"` until Task 2's
change is in place, and `link_note_game` therefore answers `Unknown`. (If Task 2 is already
committed, the first test fails only on the missing seed helper; confirm both assertions before
implementing.)

- [ ] **Step 3: Implement**

In `commit_source`, keep the engine's view of linkage in step with the app's — one choke point, so
it cannot drift:

```rust
        self.tm.lock().unwrap().set_schedule_linked(self.uses_remote());
```

In the startup restore block, after `new.current_court = note.court.clone()`, seed the engine when
the note holds a current game:

```rust
                    new.pending_restore_game = note.current_game.clone();
                    // Push the remembered game into the engine now rather than
                    // waiting for a schedule. With the network off no schedule ever
                    // arrives, and an engine with nothing next used to answer
                    // `game_number + 1` — the phantom that played a game unattended
                    // and queued a 0-0. Timing and start time stay unknown until a
                    // schedule confirms them.
                    if let Some(ref number) = note.current_game {
                        new.tm.lock().unwrap().set_next_game(NextGameInfo {
                            number: number.clone(),
                            timing: None,
                            start_time: None,
                        });
                    }
```

Also call `set_schedule_linked(true)` on this path, since a restored link does not route through
`commit_source`. Do it once, immediately after `new.source = GameSource::Portal;`.

**Check for a startup-ordering trap before committing:** `TournamentManager::new` runs before the
note is read, so the engine starts unlinked. Confirm by inspection that nothing between
construction and the restore block calls `next_game_number()` — if something does, it will see the
manual arithmetic answer once. Note the finding either way in Deviations.

- [ ] **Step 4: Run the tests to verify they pass**

```
cargo test -p refbox --lib
```
Expected: PASS.

- [ ] **Step 5: Verify and commit**

```
cargo clippy -p refbox --all-targets -- -D warnings && cargo test -p refbox --lib
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): restore the remembered game without waiting for a schedule"
```

---

## Task 6: Correct the feature file and the walkthrough

**Files:**
- Modify: `refbox/tests/features/court-finished.feature:61-69` and `:88-94`
- Modify: `docs/superpowers/plans/2026-08-16-no-next-game-on-court-finish.md` — walkthrough
  scenario 8

These two scenarios are wrong **by decision, not by defect** — they describe behaviour the design
session deliberately ruled against, and leaving them would make the next reader think the
implementation is broken.

- [ ] **Step 1: Replace the "picked up by a refresh" scenario**

Decision 10 ruled that a finished court stays finished until the operator asks — a human ruling
against the recommendation to re-read automatically, because an unattended box changing state on
its own is its own hazard. Delete the "Same session only…" comment block above it (the restart
residual it describes no longer exists) and replace lines 61-69 with:

```gherkin
  # Decision 10: a finished court stays finished until the operator asks. Restarting no
  # longer loses the ability to find a late addition — the anchor survives, so the search
  # runs again on every REFRESH, in this session or any later one.
  Scenario: A game added to a finished court is adopted on REFRESH
    Given court 1's schedule is finished
    When a new game on court 1 is added in the portal
    Then nothing changes until the operator presses REFRESH
    When the operator presses REFRESH
    Then that game becomes the upcoming game
    And the clock counts down toward it again
    And START NOW is available again
    And the same holds after a restart, because the anchor is remembered
```

- [ ] **Step 2: Replace the "fresh launch offers the earliest game" scenario**

Decision 9 was superseded: it and decision 21 are the same situation internally — no anchor for
this court — with opposite answers the refbox cannot tell apart. Replace lines 88-94 with:

```gherkin
  # Decision 9 SUPERSEDED. A court the refbox holds no record for is either a fresh
  # morning or a replacement box brought out mid-day, and it cannot tell them apart.
  # Offering the earliest game would confidently offer a game played hours ago.
  Scenario: A court with no recorded history requires an operator pick
    Given the refbox is launched with court 1 selected and no game played on it yet
    When the schedule arrives
    Then no game is offered as the upcoming game
    And the clock is stopped and START NOW is greyed
    And no game from another court is offered
    When the operator picks a game on court 1 in Settings
    Then that game becomes the upcoming game
```

- [ ] **Step 3: Add a scenario for the offline restart**

Criterion 4 is one of the three Criticals and has no written scenario. Add after the restart
scenario at `:71`:

```gherkin
  # Scenario 2's Critical: with no network the old code fell back to arithmetic,
  # invented game 1, played it unattended and queued a 0-0 that was delivered on
  # reconnect.
  Scenario: A restart with no network comes back finished, not inventing a game
    Given court 1's schedule is finished
    When the refbox is closed, the network is switched off, and it is reopened
    Then it shows the finished state with no upcoming game
    And no game starts, however long it is left running
    And nothing is queued for the portal
    And nothing is posted when the network returns
```

- [ ] **Step 4: Correct walkthrough scenario 8**

In `docs/superpowers/plans/2026-08-16-no-next-game-on-court-finish.md`, update walkthrough scenario
8 to match: a refresh is required for a late addition, and a court with no history asks for a pick.
Do not restructure the document — change the scenario text and add a one-line note pointing at this
plan.

- [ ] **Step 5: Commit**

Documentation only; no code check needed.

```
git add refbox/tests/features/court-finished.feature docs/superpowers/plans/2026-08-16-no-next-game-on-court-finish.md
git commit -m "docs(refbox): correct the court-finished scenarios ruled against in design"
```

---

## Task 7: Full check and the ten-criterion manual walkthrough

**Files:** none — this task produces evidence, not code.

**Why this task is not optional:** the manual walkthrough is what caught all three Criticals. 604
unit tests caught none. Criteria 2, 3 and 4 are those three Criticals and **must be demonstrated,
not reasoned about**.

- [ ] **Step 1: Confirm the three spec rows this plan deliberately does not change**

Each of these is a spec requirement that current code is believed to satisfy already. Confirm by
reading the code — not by assuming — and record the finding. If any is *not* satisfied, stop and
report it before continuing; it is a new task, not a quiet fix.

1. **"Picks a game listed on another court → not offered"** (decision 28). Check that the game
   picker's list is filtered to `current_court`, and that changing court clears the pending game
   selection (`select_court` in the edited-settings type).
2. **"Portal returns → send queued results automatically; leave the game state alone until
   REFRESH"** (decision 15). Check that reconnection does not itself fire a schedule fetch —
   `RecvSchedule` should only be reachable from startup restore, explicit REFRESH, event selection
   and end-of-game.
3. **"Any schedule change during play never disturbs a game in progress"** (decision 12, a hard
   guarantee). Check that the `RecvSchedule` handler's `tm.current_period() ==
   GamePeriod::BetweenGames` guard still wraps every state-changing branch after Task 4's rewrite.

- [ ] **Step 2: Run the full gate**

```
cd /home/estraily/projects/uwh-refbox-rs/.claude/worktrees/fix+refbox+court-finished-behaviour
just check
```
Expected: fmt, lint, tests and audit all clean. `just lint` is not `--all-targets`; also run
`cargo clippy --workspace --all-targets --all-features -- -D warnings` and expect the known
pre-existing `player_grid.rs` failure only — anything else is ours.

- [ ] **Step 4: Build a real binary before demonstrating anything**

`just check` builds a *test* binary. Build the app itself, or the walkthrough runs stale code:

```
cargo build -p refbox
```

- [ ] **Step 5: Set up the two-court event**

Use the local mock portal recipe (`reference_local_mock_portal_recipe`): needs `--allow-http`, and
`portal_link.json` may only be edited while refbox is **stopped**. Launch with
`WAYLAND_DISPLAY=` unset and `UWH_PORTAL_URL_OVERRIDE` set — launching without the override hits
production and wipes the link note. Court 1's games must be exhaustible within the session.

- [ ] **Step 6: Demonstrate the ten criteria, one at a time**

Run **one** criterion, report what was observed, and wait before running the next. Criteria 2, 3
and 4 first — they are the Criticals.

| # | What to do | What must happen |
|---|---|---|
| 2 | Play out the last game, then close and reopen **twice in quick succession** | Finished both times. No countdown, no game started, nothing posted. |
| 3 | Reopen and press **REFRESH** repeatedly | Finished every time. |
| 4 | Reopen **with the network off** | Finished; nothing started, nothing queued; nothing posted when the network returns. |
| 1 | Play out the last game | Clock stops dead, `END --:--`, dashes, score retained, result posted for that game only. |
| 5 | Add a game to the court, press REFRESH | Adopted; countdown runs; START NOW live; previous score stays until the new game starts. |
| 6 | Restart mid-event between games | Resumes at the same game; the note is never overwritten while the schedule is unknown. |
| 7 | Pick an out-of-order game, press REFRESH | The pick survives. |
| 8 | During the last game on a court, reach half time and pre-overtime | Whistle, beeps and a working START NOW, exactly as normal. |
| 9 | Switch the portal off from the finished state | Break counting, START NOW live, numbering from 1. |
| 10 | Point the refbox at a court it has no history for | Offers nothing; the operator picks. |

Criterion 8 is the load-bearing one for a real tournament: the court is flagged finished from the
moment the last game *starts*, and the last game is often a final.

- [ ] **Step 7: Record the results and request review**

Write the observed result for each criterion into the Deviations section below. Then run
`superpowers:requesting-code-review` for the whole branch, and prepare the PR body per
`.claude/rules/pr-review.md` — plain-language summary, scope, and how to verify.

**Known open item, deliberately not fixed here:** the LED panel shows a frozen `NEXT GAME IN 0:30`
in the finished state. The engine clock is provably 0 and the panel draws only `secs_in_period`.
Two code-reading explanations were already wrong — instrument the wire, do not theorise. Out of
scope for this branch (`docs/backlog/court-finished-panel-state/NOTE.md`).

---

## Out of scope — do not drift into these

- Restoring a live game (clock, score, penalties) across a restart, and the unconfirmed score lost
  at shutdown. Same capability, separate design.
- Two refboxes on one court. Needs portal-side arbitration; the "warn if a result exists"
  half-measure was rejected because it fires on legitimate corrections and would be trained away.
- LED panel and stream overlay display of the finished state.
- End-of-game buzzer precision — pre-existing, own branch.
- Undelivered queued results at shutdown.

---

## Deviations

Record anything that diverged from this plan, and the observed results of Task 7, here. Per
`.claude/rules/plan-execution.md` and `feedback_no_standalone_deviation_commits`, fold these notes
into the relevant code commit — do not create standalone deviation commits.

| Task | What differed | Why |
|---|---|---|
| 1 | Accepted v1 notes instead of quarantining them | A version bump that discarded the old note would force a re-pick on upgrade day, mid-tournament. A v1 note migrates for free via a serde alias. |
| 2 | Left `set_schedule_linked` dead until Task 5 (Ruling 4) | The plan deliberately split the engine change from its wiring. An `allow(dead_code)` added and removed one task later is churn in the state machine's own file. Clippy went clean at Task 5 as predicted. |
| 3 | Persist call went INSIDE `uses_remote()`, not after it (Ruling 2) | Outside, manual mode would reach the note's DELETE branch from the game-clock path, for no benefit. |
| 3 | +1 fix round: the custom-site Apply path never cleared the anchor | Review Critical. A stale anchor from the old site was written under the new event id — decision 25's exact failure. |
| 4 | +1 fix round: two tests could not fail | One was a defect in THIS PLAN's text: the "same answer however many times it is asked" loop calls a pure function five times, which the type system already guarantees. Replaced with the post-consumption state. The other pinned the anchor-start precedence, which inverting the code had not broken. |
| 5 | +1 fix round: custom-site startup never set the flag (Critical), and a finished court showed a 15-minute countdown before parking (violating criterion 2's "No countdown") | The first left this plan's headline defect fully live for custom sites. The second is Ruling 7: park at 0:00, matching the state a played-to-finish court is left in. |
| 5 | Third `set_schedule_linked` write site accepted against the plan's own constraint (Ruling 14) | It only ever writes `false`, from `reset_to_manual_break`, whose name asserts manual; the single-writer alternative needs a non-reentrant lock dropped and borrows re-choreographed for zero behaviour change. |
| 6 | Corrected items 2, 3 and 4 of the old walkthrough doc, not just scenario 8 (Rulings 9, 10) | Leaving them produced a document contradicting ITSELF. Stopped at the task bodies: those are a completed plan for a superseded branch, and rewriting them would erase why this branch exists. |
| rebase | 3 conflicts, one SEMANTIC: master renamed `make_button` -> `make_chrome_button` and deleted the old name | Keeping both sides, or taking ours verbatim, would not have compiled. |
| rebase | Controller removed 5 `.unwrap()`s on `tm.lock()` and their poison comments | master replaced the game-state mutex with a poison-recovering wrapper returning the guard directly. |
| final review | +1 fix round, 2 high: clock work ran BEFORE the source was committed, so switching Portal->Manual refused to start the break (failing criterion 9) and hot-looped the updater | Both were ordering bugs from Task 5's wiring. `just check` passed straight over them. |

## Status at checkpoint, 2026-08-31

Code complete at `717823ed` + docs at `dc15d310`. `just check` green (708 tests). Walkthrough
9/10 (see `docs/backlog/court-finished-panel-state/WALKTHROUGH-RESULTS-2026-08-31.md`).

**BLOCKER: master gained 105 commits after our rebase**, 18 touching `refbox/src/app/mod.rs`
(1347 lines) — including source-selection ported onto SharedGame, per-source event data, and
"drop a schedule/teams/token from the site the refbox has left", which is the same territory as
this branch's anchor clearing. A second rebase is required and will need judgement, not just
textual conflict resolution. Per `.claude/rules/pr-review.md` that rebase STALES both mandatory
checks: the code review and the human walkthrough must both be re-run against the rebased diff.
