# No Next Game On This Court — Finishing Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the `no-next-game-on-court` feature so it can be reviewed and merged — rebase the
12 existing commits onto current master, implement the three late decisions that supersede the
approved spec, fix the stale-restore-note bug that silently undoes the feature, and complete the
manual walkthrough.

**Architecture:** The feature is already built and its gates were green at `992b77d8`. This plan
adds four behaviour changes on top of the same single fact the feature already records — *this
court's schedule is finished*, held as the engine's private `no_next_game` flag and reported as a
blank next-game number. The clock now stops the moment the last game ends rather than counting a
break down; the big clock block says so in words; the 30-second whistle joins the buzzer and the
countdown beeps in being suppressed; and the remembered-session note can no longer fire hours
later and re-adopt an old game.

**Tech Stack:** Rust 2024, iced 0.13 (GUI), tokio (async), `uwh-common` shared types, Fluent
(`fl!`) translations, `just` for the quality gates.

**Spec:** `docs/superpowers/specs/2026-08-05-no-next-game-on-court-design.md`

**Predecessor plan:** `docs/superpowers/plans/2026-08-05-no-next-game-on-court.md` — Tasks 1–9,
all complete. Its "Superseding decisions from the parallel session" section is the source of
Tasks 2–4 and 6 below. Its per-task review record is
`.superpowers/sdd/2026-08-05-no-next-game-on-court/progress.md` in the worktree.

**Superseded in part by:** `docs/superpowers/plans/2026-08-17-court-finished-behaviour.md` —
Decisions 9 and 10 there override walkthrough scenario 8 below: a fresh launch with no recorded
history now asks for an operator pick rather than offering the earliest game, and a late
addition on a finished court always needs an explicit REFRESH, in this session or any later one.

**Worktree:** `/home/estraily/projects/uwh-refbox-rs/.claude/worktrees/fix+uwh-common+no-next-game-on-court`
**Branch:** `fix/uwh-common/no-next-game-on-court` (currently `992b77d8`, 12 commits, unpushed, no PR)

## Global Constraints

- **MSRV Rust 1.85, edition 2024.** No APIs newer than 1.85.
- **Clippy is `-D warnings`** on Linux, Windows and macOS. Zero warnings; no new `#[allow(...)]`.
  `refbox` is a bin-only crate — use `cargo clippy -p refbox --all-features -- -D warnings`, **not**
  `--all-targets`.
- **No `unwrap()`/`expect()` in non-test production code** without a comment saying why it cannot panic.
- **No new dependencies.**
- **Exactly ONE new translation key** (`schedule-end`), with a value in **all 15 locales**. The
  English value is the literal `END` — all caps, no punctuation, no qualifier. Everything else in
  this feature still uses the bare ASCII hyphen `-` and needs no key.
- **`uwh-common` must still build without `std`:** `cargo build -p uwh-common --no-default-features`.
  Workspace-wide checks unify features and hide a break here.
- **Golden timing traces must not change.** If any golden trace file needs editing, stop and report —
  this work must not alter game timing.
- **Heavy process** (`.claude/rules/plan-execution.md`): this branch touches `uwh-common` and the
  game-clock state machine. Every task ends with its tests run and a review checkpoint before the
  next task starts.
- **Commits and pushes need the human's approval** (`.claude/rules/communication.md`). Each commit
  step means: stage the files, show the human the diff summary and the proposed message, then commit
  once they agree. Nothing is pushed and no PR is opened until Task 8.
- **Plan and spec docs stay untracked.** Never add `docs/superpowers/**` to a commit.
- **`refbox/tests/features/*.feature` files ARE tracked** — they are written specs, not runnable
  tests. There is no cucumber runner in this workspace.

## Known trap — do not reintroduce

Passing two `self.tm.lock()` calls as separate arguments to a view builder **deadlocks the UI** the
instant the page draws: the argument temporaries live to the end of the statement, so the first
guard is still held when the second `lock()` runs. Tests and clippy stay green; the window just
freezes. Read everything you need under **one** lock, in its **own** statement, then use the values.

## File Structure

| File | Responsibility | Task |
|------|----------------|------|
| (rebase only — no new responsibility) | Carry the 12 existing commits onto current master | 1 |
| `refbox/src/tournament_manager/mod.rs` | `end_game` stops the clock dead when the court is finished | 2 |
| `refbox/translations/*/refbox.ftl` (15 files) | The one new `schedule-end` key | 3 |
| `refbox/src/app/view_builders/shared_elements.rs` | Big clock block reads `END` over `--:--` | 3 |
| `refbox/src/app/mod.rs` | Whistle suppressed; stale restore note discarded; fresh-launch anchor fallback | 4, 5, 6 |
| `refbox/tests/features/court-finished.feature` | The written scenario spec for this feature | 7 |

---

### Task 1: Rebase onto current master and re-verify

The branch is 169 commits behind `origin/master`. `refbox/src/app/mod.rs` alone has moved ~2,000
lines on master in that window, so expect real conflicts there. Nothing else in this plan is
trustworthy until this task is green.

**Files:** none authored — conflict resolution only.

**Interfaces:**
- Produces: a rebased `fix/uwh-common/no-next-game-on-court` whose 12 commits sit on current
  `origin/master`, with `just check` green.

- [ ] **Step 1: Take a safety backup of the pre-rebase tip**

Run from the worktree:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.claude/worktrees/fix+uwh-common+no-next-game-on-court
git branch pre-rebase-backup-no-next-game-20260816 fix/uwh-common/no-next-game-on-court
git rev-parse pre-rebase-backup-no-next-game-20260816
```

Expected: prints `992b77d8...`. Do not delete this branch — it is the only copy of the pre-rebase
state, and this repo has a history of unpushed branches being the only copy of work.

- [ ] **Step 2: Fetch and rebase**

```bash
git fetch origin
git rebase origin/master
```

Expected: conflicts. Resolve them **preserving the branch's intent**, not by taking either side
wholesale. The three places to expect them:

1. `refbox/src/app/mod.rs`, the received-schedule handler. Master has added a `roster_tasks` vector
   and now returns `Task::batch(roster_tasks)` where the branch's version returns
   `self.apply_snapshot(snapshot)`. Keep master's `roster_tasks` plumbing **and** the branch's
   court-aware search with its `court_finished` flag; the restore path must push onto
   `roster_tasks` and return the batch, as master does.
2. `refbox/src/app/mod.rs`, `maybe_play_sound`. The branch adds a `starts_nothing` binding and gates
   the buzzer and countdown beeps on it. Master has not touched this function's logic — reapply the
   branch's two gates verbatim.
3. `refbox/src/tournament_manager/mod.rs`. Master's changes here are elsewhere in the file; the
   branch's `no_next_game` field, `set_no_next_game`, `next_game_number` blank arm, the
   `(GamePeriod::BetweenGames, _)` update arm, `start_play_now` guard and
   `TournamentManagerError::NoNextGameOnCourt` should all apply cleanly or near-cleanly.

If a conflict cannot be resolved without changing what the branch does, **stop and report it** —
do not invent a resolution.

- [ ] **Step 3: Confirm nothing was lost in the rebase**

```bash
git log --oneline origin/master..HEAD
git diff --stat origin/master...HEAD
```

Expected: 12 commits, same messages and order as before; the diffstat still lists all seven files
(`refbox/src/app/mod.rs`, `configuration.rs`, `game_info_table.rs`, `main_view.rs`,
`tournament_manager/mod.rs`, `uwh-common/src/game_snapshot.rs`,
`uwh-common/src/uwhportal/schedule.rs`). A file missing from that list means a resolution dropped
work — stop and report.

- [ ] **Step 4: Run the full gate**

```bash
just check
cargo build -p uwh-common --no-default-features
cargo test -p refbox golden
```

Expected: `just check` exit 0 (fmt, clippy, tests, audit); the no_std build succeeds; the golden
traces pass with **no scenario file edited**. Report the refbox and uwh-common test counts — the
pre-rebase figures were refbox 439 and uwh-common 69, and master has added its own since.

- [ ] **Step 5: Checkpoint with the human**

Report: conflicts hit, how each was resolved, and the four gate results. Do not start Task 2 until
the human confirms. No commit here — a rebase rewrites the existing commits and adds none.

---

### Task 2: The clock stops dead when the last game on a court ends

**Decision 1.** The approved spec said the break counts down and holds at 0:00; the human was
explicit that it must not count down at all. There is nothing to count down *to*.

A reference implementation exists as commit `026c6644` on the abandoned branch
`fix/refbox/court-finished-no-autostart` (worktree `.worktrees/court-finished`). It is **unverified** —
never built, never run — and it names the flag `court_finished` where this branch names it
`no_next_game`. Port the logic, adapt the name, and treat its two tests as unrun.

Keep the existing expiry guard as well. The two cover different orderings: this task covers "the
court was already finished when the game ended", and the existing `(GamePeriod::BetweenGames, _)`
guard covers "a break was already counting down when the court became finished".

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — `end_game`, immediately after
  `time_remaining_at_start` is computed and before the `info!("... Entering between games ...")` call
- Test: `refbox/src/tournament_manager/mod.rs` (`#[cfg(test)] mod test`)

**Interfaces:**
- Consumes: the private `no_next_game` flag (set by `set_no_next_game`, already on the branch).
- Produces: no new public API.

- [ ] **Step 1: Write the failing tests**

Add to `mod test` in `refbox/src/tournament_manager/mod.rs`:

```rust
    #[test]
    fn the_last_game_on_a_court_ends_with_the_clock_stopped() {
        // There is no next game, so there is nothing to count down TO. The clock stops
        // when the last game on the court ends, rather than running a break to zero.
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            minimum_break: Duration::from_secs(10),
            game_block: Duration::from_secs(60),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();

        tm.set_next_game(NextGameInfo {
            number: "9".to_string(),
            timing: None,
            start_time: None,
        });
        tm.start_play_now(start).unwrap();
        tm.stop_clock(start).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(0));
        tm.add_score(Color::Black, 5, start);
        // Game 9 is the last on this court — in the app this is recorded when it STARTS.
        tm.set_no_next_game();

        tm.end_game(start);

        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert!(
            !tm.clock_is_running(),
            "the clock must not run after the last game on a court"
        );
        assert_eq!(tm.game_clock_time(start), Some(Duration::ZERO));

        // And it stays put: with no countdown there is nothing to expire, so no game can
        // auto-start however long the refbox is left running.
        tm.update(start + Duration::from_secs(3600)).unwrap();
        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert!(!tm.clock_is_running());
        assert_eq!(
            tm.game_clock_time(start + Duration::from_secs(3600)),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn an_ordinary_game_end_still_starts_the_break_countdown() {
        // Guard against over-reach: mid-day, the break still counts down as always.
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            minimum_break: Duration::from_secs(10),
            game_block: Duration::from_secs(60),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();

        tm.set_next_game(NextGameInfo {
            number: "9".to_string(),
            timing: None,
            start_time: None,
        });
        tm.start_play_now(start).unwrap();
        tm.stop_clock(start).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(0));
        tm.add_score(Color::Black, 5, start);

        tm.end_game(start);

        assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
        assert!(tm.clock_is_running(), "the break counts down as usual");
        assert!(tm.game_clock_time(start).unwrap() > Duration::ZERO);
    }

    #[test]
    fn the_finished_games_score_survives_until_the_next_game_starts() {
        // The operator must still be able to read the final score off the screen after the
        // day ends. The mid-break reset lives inside the counting-down branch of `update`,
        // so a stopped clock never reaches it. When a game IS applied later, the reset
        // still happens at start_game, so the new game begins 0-0.
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            minimum_break: Duration::from_secs(10),
            game_block: Duration::from_secs(60),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();

        tm.set_next_game(NextGameInfo {
            number: "9".to_string(),
            timing: None,
            start_time: None,
        });
        tm.start_play_now(start).unwrap();
        tm.stop_clock(start).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(0));
        // NOTE: the second argument is the scoring player's CAP NUMBER, not an amount.
        // One call is one goal, so black's score below is 1.
        tm.add_score(Color::Black, 5, start);
        tm.set_no_next_game();
        tm.end_game(start);

        // An hour later the score is still on screen.
        let later = start + Duration::from_secs(3600);
        tm.update(later).unwrap();
        assert_eq!(tm.get_scores().black, 1);

        // A game added to the court afterwards clears it at kickoff, not before.
        tm.set_next_game(NextGameInfo {
            number: "11".to_string(),
            timing: None,
            start_time: None,
        });
        tm.apply_next_game_start(later).unwrap();
        assert_eq!(tm.get_scores().black, 1, "still showing during the break");
        tm.start_play_now(later).unwrap();
        assert_eq!(tm.get_scores().black, 0, "cleared when the next game starts");
    }
```

- [ ] **Step 2: Run the tests to verify the new behaviour fails**

Run: `cargo test -p refbox the_last_game_on_a_court_ends && cargo test -p refbox an_ordinary_game_end_still && cargo test -p refbox the_finished_games_score_survives`

Expected: `an_ordinary_game_end_still_starts_the_break_countdown` PASSES (unchanged path);
`the_last_game_on_a_court_ends_with_the_clock_stopped` FAILS on `clock_is_running`;
`the_finished_games_score_survives_until_the_next_game_starts` FAILS (the countdown reset clears
the score partway through the break).

- [ ] **Step 3: Stop the clock instead of starting a break**

In `refbox/src/tournament_manager/mod.rs`, in `end_game`, insert immediately **after** the
`let time_remaining_at_start = ...` binding and **before** the
`info!("... Entering between games ...")` call:

```rust
        if self.no_next_game {
            // No later game on this court, so there is nothing to count down TO. Stop the
            // clock outright instead of running a break down to zero: nothing can expire
            // and nothing can auto-start. The final score also stays on screen, because
            // the mid-break `reset()` only runs while a countdown is active, so it never
            // fires here — a later `start_game` still resets before the next game.
            info!(
                "{} Last game on this court is over; stopping the clock",
                self.status_string(now),
            );
            self.clock_state = ClockState::Stopped {
                clock_time: Duration::ZERO,
            };
            if was_running {
                self.send_clock_running(false);
            }
            self.reset_game_time = Duration::ZERO;
            return;
        }
```

`time_remaining_at_start` is computed above this and deliberately unused on this path — the
calculation is cheap and moving it below would reorder the `calc_time_to_next_game` call relative to
the logging. If clippy objects to the unused binding, restructure by moving the guard **above** the
`let time_remaining_at_start` binding rather than adding an `#[allow]`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox the_last_game_on_a_court_ends && cargo test -p refbox an_ordinary_game_end_still && cargo test -p refbox the_finished_games_score_survives`
Expected: PASS, 3 tests.

- [ ] **Step 5: Run the full engine suite including the golden traces**

Run: `cargo test -p refbox`
Expected: PASS. No golden trace file may change — none of them set `no_next_game`, so the guarded
path is untouched. If a golden trace fails, **stop and report**.

- [ ] **Step 6: Commit** (after the human approves the diff)

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): stop the clock when a court's last game ends"
```

---

### Task 3: The big clock block reads END over --:--

**Decision 2.** The banner today maps `GamePeriod::BetweenGames` to `fl!("next-game")` above the
break time. In the finished state that promises a game that is not coming.

Confirmed with the human on 2026-08-16: the label reads the literal `END` and the time reads
`--:--`. The banner's two-readout shape is kept — it sizes text differently when one readout has
the row to itself, so removing the time would re-proportion the whole block. **Already rejected:**
"NO NEXT GAME", the old "NEXT GAME" label with dashes, `0:00`, `99:00`.

**Files:**
- Modify: `refbox/translations/{de-DE,en-US,es,fr,id-ID,it-IT,ja-JP,ko-KR,ms-MY,nl-NL,pt-PT,th-TH,tl-PH,tr-TR,zh-CN}/refbox.ftl` (15 files)
- Modify: `refbox/src/app/view_builders/shared_elements.rs` — the `time_text` binding and the
  `GamePeriod::BetweenGames` arm of the period-label match

**Interfaces:**
- Consumes: `snapshot.next_game_number` being empty.
- Produces: the translation key `schedule-end`.

- [ ] **Step 1: Add the translation key to all 15 locales**

In each `refbox/translations/<locale>/refbox.ftl`, add a `schedule-end` line immediately after the
existing `next-game` line, keeping each file's existing spacing and comment style. Values —
English is the human's literal choice; the other 14 are best guesses for review, all meaning
"end / finished", following each file's existing casing convention (the CJK and Thai files do not
use caps because those scripts have no case):

| Locale | Value |
|--------|-------|
| `en-US` | `END` |
| `de-DE` | `ENDE` |
| `es` | `FIN` |
| `fr` | `FIN` |
| `id-ID` | `SELESAI` |
| `it-IT` | `FINE` |
| `ja-JP` | `終了` |
| `ko-KR` | `종료` |
| `ms-MY` | `TAMAT` |
| `nl-NL` | `EINDE` |
| `pt-PT` | `FIM` |
| `th-TH` | `จบ` |
| `tl-PH` | `TAPOS` |
| `tr-TR` | `SON` |
| `zh-CN` | `结束` |

For example, in `refbox/translations/en-US/refbox.ftl`:

```ftl
next-game = NEXT GAME
schedule-end = END
```

Every locale must have the key. The translation coverage check lives in `refbox/build.rs` (it is a
build script, not a test) and takes the union of all locales' keys, so a missing one shows up as a
build warning rather than a test failure — do not rely on `cargo test` to catch an omission.

- [ ] **Step 2: Show END over --:-- in the banner**

In `refbox/src/app/view_builders/shared_elements.rs`, the function that builds the clock banner
currently has:

```rust
    let time_text = secs_to_long_time_string(snapshot.secs_in_period);
```

with the label chosen a little above by `GamePeriod::BetweenGames => (fl!("next-game"), yellow_text)`.

Introduce the finished-state test once, next to the `time_text` binding, and use it in both places:

```rust
    // A blank next-game number between games means the selected court has no further
    // games (uwh-common's GameSnapshot::next_game_number reports None for it). The
    // banner says so rather than promising a game that is not coming: the label reads
    // END and the time reads dashes, keeping the banner's two-readout shape so nothing
    // around it changes size.
    let schedule_ended = snapshot.current_period == GamePeriod::BetweenGames
        && snapshot.next_game_number.is_empty();
```

Then replace the `time_text` binding with:

```rust
    let time_text = if schedule_ended {
        NO_TIME.to_string()
    } else {
        secs_to_long_time_string(snapshot.secs_in_period)
    };
```

and the `BetweenGames` arm of the period-label match with:

```rust
            GamePeriod::BetweenGames => {
                if schedule_ended {
                    (fl!("schedule-end"), yellow_text)
                } else {
                    (fl!("next-game"), yellow_text)
                }
            }
```

Add the constant at the top of the file, beside the other module constants:

```rust
/// Shown in place of the clock when the selected court's schedule is finished. Not a
/// translated string — it is punctuation, the same "nothing here" signal the game info
/// table draws with a hyphen.
const NO_TIME: &str = "--:--";
```

Leave the `.trim()` that follows the `time_text` binding where it is — `--:--` is unaffected by it,
and `secs_to_long_time_string` still needs it.

Note the `make_red` block a little above this: it flashes the banner at `secs_in_period == 30` during
alert periods. In the finished state after Task 2 the clock is stopped at 0, so it never passes 30
and never flashes. In the other ordering — a break already counting down when the court becomes
finished — it will flash at 30 while reading `END  --:--`. That is acceptable and out of scope; do
not change `make_red`.

- [ ] **Step 3: Verify it compiles clean**

Run: `cargo clippy -p refbox --all-features -- -D warnings`
Expected: no warnings, and no build-script warning about a locale missing `schedule-end`.

- [ ] **Step 4: Run the translation consistency test**

Run: `cargo test -p refbox translation`
Expected: PASS. This test checks key consistency across locales.

- [ ] **Step 5: Commit** (after the human approves the diff, including the 14 guessed values)

```bash
git add refbox/translations refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): show END on the clock when a court's schedule is finished"
```

---

### Task 4: The 30-second whistle is suppressed too

**Decision 3.** The branch already gates the start-of-play buzzer and the audible countdown beeps on
`break_starts_nothing`; the whistle leg (`secs_in_period == 30`) was missed.

After Task 2 the clock stops dead at the end of the last game, so the common path never reaches 30
seconds anyway. This gate matters for the **other ordering**: a break already counting down when a
schedule refresh reports the court finished. That break runs on to 0:00 and would otherwise blow a
30-second whistle for a game that is not coming.

**Files:**
- Modify: `refbox/src/app/mod.rs` — the whistle leg of the tuple returned by the `None` arm of
  `maybe_play_sound`
- Test: `refbox/src/app/mod.rs` (`#[cfg(test)] mod break_starts_nothing_tests`)

**Interfaces:**
- Consumes: `break_starts_nothing(period, next_game_number)`, already on the branch.
- Produces: no new API.

- [ ] **Step 1: Write the failing test**

`maybe_play_sound` takes `&self` on the whole app and is not directly testable in this codebase —
the existing `break_starts_nothing_tests` module tests the predicate instead. Add a test there that
pins the property this task depends on, so a later edit to the predicate cannot silently un-suppress
the whistle:

```rust
    #[test]
    fn a_finished_court_suppresses_every_break_sound() {
        // The whistle, the start-of-play buzzer and the countdown beeps are all gated on
        // this one predicate. Between games with a blank number nothing is coming, so
        // none of the three may sound.
        assert!(break_starts_nothing(GamePeriod::BetweenGames, ""));

        // Mid-game breaks of the LAST game on a court are not affected: a game is running
        // and its own whistle must still sound.
        for period in [
            GamePeriod::HalfTime,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::PreSuddenDeath,
        ] {
            assert!(!break_starts_nothing(period, ""), "{period:?}");
        }
    }
```

Run: `cargo test -p refbox break_starts_nothing`
Expected: PASS immediately — this is a characterisation test of existing behaviour. If it fails,
stop: the predicate is not what the whistle gate is about to rely on.

- [ ] **Step 2: Gate the whistle**

In `refbox/src/app/mod.rs`, in `maybe_play_sound`, the `None` arm currently ends with:

```rust
                (
                    prereqs && is_whistle_period && new_snapshot.secs_in_period == 30,
                    prereqs
                        && is_buzz_period
                        && !starts_nothing
                        && new_snapshot.secs_in_period == 0,
                )
```

Add the same gate to the whistle leg and extend the comment above the `starts_nothing` binding to
name all three sounds:

```rust
                (
                    prereqs
                        && is_whistle_period
                        && !starts_nothing
                        && new_snapshot.secs_in_period == 30,
                    prereqs
                        && is_buzz_period
                        && !starts_nothing
                        && new_snapshot.secs_in_period == 0,
                )
```

The comment on the `starts_nothing` binding reads "no buzzer at 0:00 and no countdown beeps on the
way there" — update it to also mention the 30-second whistle.

- [ ] **Step 3: Verify it compiles clean and the tests pass**

Run: `cargo clippy -p refbox --all-features -- -D warnings && cargo test -p refbox`
Expected: no warnings; all tests pass, golden traces untouched.

- [ ] **Step 4: Commit** (after the human approves the diff)

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): silence the break whistle when a court's schedule is finished"
```

---

### Task 5: A stale restore note can no longer undo the finished state

**Found live during the walkthrough (scenario B failed on it).** This is pre-existing master
behaviour, not something this branch introduced — but this branch is what makes it harmful.

On launch, `portal_link.json` is restored into `self.pending_restore_game`. It is consumed by
`.take()` inside the received-schedule handler — but that whole block sits behind
`if self.edited_settings.is_none()` **and** `current_period == BetweenGames`. If the first schedule
arrives while the operator is in Settings — which is exactly when they are picking their event,
court and game — the note is neither used nor cleared. It sits there until the next schedule arrives
outside Settings, typically the refresh at the **end** of the game just played, and then fires.

Observed: the operator picked game 71, the last on court 1; at game start the engine correctly
recorded "no further games on this court"; at game end the stale note said "game 1", so the refresh
adopted tournament game 1 with START NOW live — undoing the finished state entirely.

The agreed fix direction: the remembered game exists only to put the operator back where they were
**at startup**. Discard it as soon as they pick a game themselves or a game starts, so it can never
fire hours later.

**Files:**
- Modify: `refbox/src/app/mod.rs` — `handle_game_start`, and the settings-apply path that commits a
  portal game selection

**Interfaces:**
- Consumes: `self.pending_restore_game`.
- Produces: no new API — a narrower lifetime for an existing field.

- [ ] **Step 1: Write the failing test**

`handle_game_start` and the apply handlers take `&mut self` on the whole app and are not unit-tested
in this codebase — consistent with the Task 7 ruling in the predecessor plan. There is no honest
unit test to write here, so this task is verified by the walkthrough (Task 8, scenario 1) instead.
**Say so explicitly in the commit message and the PR body** rather than implying test coverage.

Do not fabricate a test that exercises a helper this fix does not go through.

- [ ] **Step 2: Discard the note when a game starts**

In `handle_game_start`, at the top of the function — before the schedule lookup — add:

```rust
        // A remembered game exists only to put the operator back where they were at
        // startup. Once a game has actually started, it is stale: leaving it in place
        // lets it fire on the end-of-game schedule refresh hours later and re-adopt an
        // old game, which would silently undo the finished-court state.
        self.pending_restore_game = None;
```

- [ ] **Step 3: Discard the note when the operator picks a game themselves**

Find the settings-apply path that commits a portal game selection to the engine (the same place the
predecessor plan's Task 7 deviation added `clear_portal_next_game()` for the portal ON→OFF toggle —
`apply_app_options` and its sibling `apply_game_options`). Wherever the operator's own selection is
committed, add:

```rust
        // The operator has made their own choice; the remembered game is spent.
        self.pending_restore_game = None;
```

Read the surrounding code before editing: if both apply paths funnel through one place, put it in
the one place. **Do not** hold two `self.tm.lock()` guards across this edit — see the "Known trap"
section at the top of this plan.

- [ ] **Step 4: Verify it compiles clean and nothing regressed**

Run: `cargo clippy -p refbox --all-features -- -D warnings && cargo test -p refbox`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit** (after the human approves the diff)

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): drop the remembered game once the operator moves on"
```

---

### Task 6: A fresh launch pre-selects the earliest game on the court

Lifted from the abandoned branch. Since the next-game search became court-aware, a fresh launch has
no anchor: the engine's game number is still `"0"` and `"0"` is not in the schedule, so the search
cannot judge what follows it and nothing is pre-selected. The old arithmetic picked game `"1"` here.
Losing that is a regression in convenience — though the old behaviour was itself wrong on a
multi-court event, where game `"1"` often belongs to another court.

The fix: when the anchor is absent **and** the game number is `"0"`, select the earliest game on the
**selected court**.

**Files:**
- Modify: `refbox/src/app/mod.rs` — the received-schedule handler's court-aware branch (the `None`
  arm of the `schedule.games.get(&tm.game_number())` match, which currently falls through to
  "cannot judge")

**Interfaces:**
- Consumes: `Schedule::next_game_on_court` (already on the branch).
- Produces: no new API.

- [ ] **Step 1: Understand the arm you are changing**

The branch's refresh branch matches on the anchor game:

```rust
                                            match schedule.games.get(&tm.game_number()) {
                                                Some(anchor) => { /* court-aware search */ }
                                                // The game we are on is not in this
                                                // schedule (it changed under us, or no
                                                // game has started yet): we cannot judge
                                                // what follows it, so leave the engine's
                                                // state alone.
                                                None => (None, None),
                                            }
```

That `None` arm deliberately covers two different situations, and only one of them is safe to act
on. **Do not** collapse them — an earlier review round caught exactly that mistake, where treating
"cannot judge" as "court finished" wiped a freshly picked game on a fresh launch.

- [ ] **Step 2: Add the fresh-launch fallback**

Replace that `None` arm with:

```rust
                                                None if tm.game_number() == "0" => {
                                                    // Fresh launch: no game has started
                                                    // yet, so there is no anchor to search
                                                    // from. Offer the earliest game on the
                                                    // selected court — never game "1" by
                                                    // arithmetic, which on a multi-court
                                                    // event belongs to another court.
                                                    match schedule.next_game_on_court(
                                                        court,
                                                        time::OffsetDateTime::UNIX_EPOCH,
                                                    ) {
                                                        Some(game) => (
                                                            Some(game),
                                                            schedule.get_game_timing(&game.number),
                                                        ),
                                                        None => (None, None),
                                                    }
                                                }
                                                // The game we are on is not in this
                                                // schedule — it changed under us. We
                                                // cannot judge what follows it, so leave
                                                // the engine's state alone.
                                                None => (None, None),
```

Note what this arm deliberately does **not** do: it never sets `court_finished`. A fresh launch onto
a court with no games at all is "nothing to offer", not "the day is done" — the operator has not
played anything yet.

`UNIX_EPOCH` is the anchor that makes "everything on this court" the search window. `app/mod.rs` has
no `use time::...` import — it fully-qualifies as `time::OffsetDateTime::...` everywhere. Match that;
do not add an import.

- [ ] **Step 3: Write the failing test for the query's edge**

The refresh handler itself is not unit-testable, but the query it leans on is. Add to `mod tests` in
`uwh-common/src/uwhportal/schedule.rs`, beside the existing `next_game_on_court` tests:

```rust
    #[test]
    fn next_game_on_court_from_the_epoch_finds_the_earliest_game() {
        // A fresh launch has no anchor game, so it searches from the beginning of time to
        // offer the first game on the selected court.
        let schedule = two_court_schedule();
        let first = schedule
            .next_game_on_court("Court 2", OffsetDateTime::UNIX_EPOCH)
            .expect("court 2 has games");
        assert_eq!(first.number, "2");
    }
```

Run: `cargo test -p uwh-common next_game_on_court`
Expected: PASS, 5 tests. This exercises the existing query at a new input; no production change is
needed in `uwh-common`.

- [ ] **Step 4: Verify it compiles clean and nothing regressed**

Run: `cargo clippy -p refbox --all-features -- -D warnings && cargo test -p refbox && cargo test -p uwh-common && cargo build -p uwh-common --no-default-features`
Expected: no warnings; all tests pass; the no_std build succeeds.

- [ ] **Step 5: Commit** (after the human approves the diff)

```bash
git add refbox/src/app/mod.rs uwh-common/src/uwhportal/schedule.rs
git commit -m "fix(refbox): offer the court's earliest game on a fresh launch"
```

---

### Task 7: Port the written scenario spec

This project keeps `.feature` files as written specs — there is no cucumber runner. The kept branch
has none; the abandoned branch wrote seven scenarios that are the clearest statement of what this
feature promises. Port them, updated for the three late decisions and for the fact that this branch
also dashes the game info table.

**Files:**
- Create: `refbox/tests/features/court-finished.feature`

Source: `.worktrees/court-finished/refbox/tests/features/court-finished.feature`. Its header refers
to a two-plan split that does not apply here, and its clock-stop scenario predates decisions 2 and 3
— both need updating.

**Interfaces:** none — documentation.

- [ ] **Step 1: Write the file**

```gherkin
# Written walkthrough scenarios for branch fix/uwh-common/no-next-game-on-court.
# Oracle: docs/superpowers/specs/2026-08-05-no-next-game-on-court-design.md, plus the
# three late decisions recorded in
# docs/superpowers/plans/2026-08-05-no-next-game-on-court.md.
# These files are documentation — there is no cucumber runner in this workspace.

Feature: A court whose schedule is finished
  On a multi-court event the refbox finds the upcoming game by searching its own court.
  When there is no later game on that court, the day on that court is finished: nothing
  starts by itself, nothing is adopted from another court, the table and the clock say so
  plainly, and START NOW is unavailable until the operator picks a game.

  Scenario: The last game on a court does not roll into another court's game
    Given a two-court event where court 1 holds games 1 and 3 and court 2 holds games 2 and 4
    And court 1 is selected
    When game 3 is started
    Then the refbox records that court 1's schedule is finished
    And game 4 is not adopted as the upcoming game
    And the Next Game block reads dashes, not game 4 or its team names

  Scenario: The clock stops when the last game on a court ends
    Given court 1 is selected and game 3 — the last game on court 1 — is in progress
    When game 3 ends
    Then the clock stops immediately, with no countdown toward a next game
    And the big clock block reads END over --:--
    And no buzzer, whistle or countdown beep sounds for a game that is not coming
    And game 3's own end-of-game buzzer still sounds as usual
    And game 3's final score stays on screen
    And the middle block, the settings values and the referee names all read dashes
    And no game ever starts, however long the refbox is left running
    And game 3's own result is submitted to the portal as usual

  # Covers the other ordering: a court can be marked finished while a between-games break
  # is ALREADY counting down. That countdown must expire into a stop, not into a game,
  # and must stay silent on the way there.
  Scenario: A break already running when the court becomes finished expires into a stop
    Given a between-games break is counting down
    When the court's schedule becomes finished before the break expires
    Then the break reaches 0:00 and the clock holds there
    And no 30-second whistle and no start-of-play buzzer sound
    And no game starts

  Scenario: START NOW is unavailable until a game is picked
    Given court 1's schedule is finished
    Then the START NOW button is greyed and does nothing when pressed
    When the operator picks a game on court 1 in Settings
    Then START NOW is available again

  # Regression guard: the court is already "finished" from the moment the last game on it
  # STARTS, so the mid-game breaks of that very game must keep START NOW available and
  # must keep their own whistle.
  Scenario: START NOW still works at half time of the last game on a court
    Given court 1 is selected and game 3 — the last game on court 1 — is in progress
    When the first half ends and the clock reaches half time
    Then START NOW is available and starts the second half
    And the half-time 30-second whistle sounds as usual
    And the same holds for pre-overtime and pre-sudden-death breaks of that game

  Scenario: A game added later is picked up by a refresh
    Given court 1's schedule is finished
    When a new game on court 1 is added in the portal and the schedule is refreshed
    Then that game becomes the upcoming game
    And the clock counts down toward it again
    And START NOW is available again

  # Regression guard for the stale remembered-session note: it exists only to restore the
  # operator's place at startup, and must never fire hours later.
  Scenario: A remembered game does not resurrect itself at the end of the day
    Given the refbox was launched with a remembered game from an earlier session
    And the operator picked a different game while the first schedule arrived
    When that game is played and ends
    Then the end-of-game refresh does not re-adopt the remembered game
    And court 1's schedule stays finished

  Scenario: A fresh launch offers the earliest game on the selected court
    Given the refbox is launched with court 1 selected and no game started yet
    When the schedule arrives
    Then the earliest game on court 1 is offered as the upcoming game
    And no game from another court is offered

  Scenario: Ordinary mid-day operation is unchanged
    Given court 1 is selected and game 1 is in progress
    Then game 3 is the upcoming game
    When game 1 ends and the break counts down to zero
    Then game 3 starts as usual

  Scenario: Manual mode is unchanged
    Given the portal is switched off
    Then games number on and auto-start exactly as before

  Scenario: Switching the portal off mid-day clears the finished state
    Given court 1's schedule is finished
    When the operator switches the portal off and applies
    Then the refbox no longer treats any court as finished
    And START NOW is available again
```

- [ ] **Step 2: Confirm it does not break the build**

Run: `cargo test -p refbox`
Expected: PASS, unchanged count — `.feature` files are inert documentation and no runner picks them
up. If the test count changes, something *is* reading them; stop and report.

- [ ] **Step 3: Commit** (after the human approves)

```bash
git add refbox/tests/features/court-finished.feature
git commit -m "docs(refbox): add the court-finished scenario spec"
```

---

### Task 8: Full verification, the operator walkthrough, review and PR

**Files:** none — gates, walkthrough and the PR.

- [ ] **Step 1: Run every gate**

```bash
just check
cargo build -p uwh-common --no-default-features
cargo test -p refbox golden
```

Expected: `just check` exit 0; the no_std build succeeds; the golden traces pass with **no scenario
file edited**. Record the test counts.

- [ ] **Step 2: Build the real binary before the walkthrough**

```bash
cargo build -p refbox
```

`just check` builds a **test** binary, not `target/debug/refbox`. Walking through with a stale
binary is a known way to "verify" the wrong build.

- [ ] **Step 3: Ask the human which configuration reaches the screen**

Before launching, confirm with the human: which event, which court, which game is last on it, and
whether the run is against the dev portal or a custom site. The custom-vs-Portal distinction has
caught wording defects that no test could.

Launch with the portal override set — launching **without** `UWH_PORTAL_URL_OVERRIDE` hits
production and wipes `portal_link.json`:

```bash
UWH_PORTAL_URL_OVERRIDE=<dev portal URL> WAYLAND_DISPLAY= ./target/debug/refbox
```

Only one refbox at a time — the config directory is shared.

- [ ] **Step 4: Walk the eight scenarios, ONE at a time**

These are the eight from the predecessor plan's Verification section, updated for the late
decisions. Run **one** scenario, report the result, and **wait** for the human before the next.

1. **Restart in the finished state.** Play out the last game on court 1. Confirm: the clock stops
   dead (it does not count down), the banner reads `END  --:--`, the middle block, settings and
   referee names read dashes, the finished game's score is still on screen, and no whistle, beep or
   buzzer sounds. Then close and reopen the refbox. Expect the same finished state within one
   schedule fetch, and `portal_link.json` still holding the court with **no** game number after
   five-plus minutes idling — it must never become `"1"`. *(Also covers Task 5.)*
2. **Restart in the finished state with no network.** Same, with Wi-Fi off. **Once a known, parked
   residual — the phantom game could appear after the nominal break — this is now CLOSED**: the
   refbox no longer invents a game number while it is attached to a schedule, so a finished court
   restarted offline parks the clock at 0:00 and starts nothing, however long it is left running.
   Confirm: no game starts, nothing is queued for the portal, and nothing is posted when the network
   returns. *(See the superseding note near the top of this plan, and the offline-restart scenario
   in the `.feature` file.)*
3. **Leaving the finished state by refresh, same session.** Add a game to court 1 in the portal.
   **This item originally said to "wait for the refresh (or tap REFRESH)... with no operator
   action" — that is WRONG under Decision 10.** No automatic or periodic schedule re-fetch runs
   while a court sits idle-finished (only game-end, REFRESH, event/court selection and re-login
   trigger a schedule arrival), so nothing happens until the operator presses REFRESH. Press
   REFRESH and expect the table to fill in, the banner to go back to `NEXT GAME` with a real time,
   the clock to start counting down, and START NOW to go live. The previous game's score stays
   visible until the new game starts. *(See the superseding note near the top of this plan, item 4
   above, and the corrected REFRESH scenario in the `.feature` file.)*
4. **Leaving the finished state by refresh after a restart.** As above but restart first. **Once a
   known, parked residual — the anchor was `"0"` after a restart, so refresh could not find a new
   addition and only picking the game in Settings recovered — this is now CLOSED**: the anchor is
   persisted, so REFRESH re-runs the search after a restart exactly as it does within the same
   session, and adopts the new game without needing an operator pick. *(See the superseding note
   near the top of this plan, and the corrected REFRESH scenario in the `.feature` file.)*
5. **App-page portal off from the finished state.** Settings → App Options → portal off → APPLY.
   Expect the break counting down, START NOW live immediately, and numbering continuing from the
   last game.
6. **Sounds in the finished state.** Audible countdown on. Reach the finished state via a break that
   was already counting down (scenario 3 in the `.feature` file — set the court finished mid-break)
   and run it from 0:40 through 0:00. Expect **no 30-second whistle**, no countdown beeps and no
   start-of-play buzzer. Control run on a court that still has games: whistle, beeps and buzzer
   exactly as before, and the end-of-game buzzer unaffected in both runs.
7. **Out-of-order pick survives REFRESH.** After game 9 on court 1, pick game 15 in Settings, tap
   REFRESH. Expect game 15 with its own teams, block and scheduled start — not game 11.
8. **Manual mode control, and a fresh launch.** Portal off from launch: numbering, auto-start, beeps
   and buzzer all unchanged. Then portal on, fresh launch with court 1 selected and no game started:
   expect no game offered, the clock stopped and START NOW greyed, and no game from another court
   offered either — not the earliest game on court 1. Pick a game on court 1 in Settings and confirm
   it becomes the upcoming game. Then add a further game to court 1 in the portal: confirm nothing
   changes until REFRESH is pressed, and that pressing it adopts the new game — the same refresh
   requirement holds whether or not the refbox has been restarted in between. *(Covers Task 6; see
   the superseding note near the top of this plan.)*

- [ ] **Step 5: Record the outcome**

Append a "Verification" section to the bottom of this plan file: which scenarios were confirmed, on
what hardware, and anything that deviated. Do not commit this file — `docs/superpowers/**` stays
untracked.

If a scenario fails, **stop**. Report it, fix it as its own task with its own tests, and restart the
walkthrough from that scenario.

- [ ] **Step 6: Request code review**

Use `superpowers:requesting-code-review` over the whole branch (`origin/master...HEAD`). Because
this branch touches `uwh-common` and the game-clock state machine, review the full diff, not just
this plan's commits. Apply `superpowers:receiving-code-review` to the findings — verify each before
implementing it.

- [ ] **Step 7: Push and open the PR** (ask the human first — this is the last approval gate)

Confirm the branch name is unchanged and the remote has no existing PR, then push and open a PR
whose body follows `.claude/rules/pr-review.md`:

```
## What changed
[Plain English: what the operator now sees and what the refbox now does]

## Why
[The multi-court problem: a guessed game number named another court's game, auto-started it, and
posted a result against it]

## Scope
[Which crates and files, and why uwh-common is involved]

## How to verify
[The walkthrough scenarios, written so a non-programmer can follow them]
```

State plainly in the body which parts are covered by automated tests and which were verified only by
the manual walkthrough — the app's `update()`/`apply_*` handlers and the schedule-refresh branch are
not unit-testable in this codebase, and Task 5 in particular has **no** automated coverage.

- [ ] **Step 8: Verify the push landed**

```bash
git cherry -v origin/master
```

Expected: no `+` lines. A `+` means a commit is orphaned and did not land.

---

## Deviations

**Task 2 sent the implementer to the wrong seam (plan defect).** It specified the patch site
inside `end_game` and wrote the tests against `end_game` directly. A normal game end never
reaches `end_game` that way — the app goes `could_end_game → pause_for_confirm →
end_confirm_pause`, and that function overwrites `clock_state` right after calling `end_game`.
The change shipped green with three passing tests and did nothing at all in the running app.
Caught by the Task 8 code review, fixed in `e448a3ed` + `cb899eb4`. See
`reference_end_game_is_not_the_end_of_game_seam` in memory.

**Task 3's plan snippet did not compile.** `secs_to_long_time_string` returns an `ArrayString<8>`,
not a `String`, so the two branches had incompatible types. `.to_string()` added on the else arm.
`schedule_ended` also had to be hoisted ~200 lines above the `time_text` binding, since the
period-label match uses it too.

**Task 4's test was a duplicate.** The specified test repeated two tests already in that module
assertion-for-assertion. Dropped; the function's stale doc comment ("*both* the buzzer and the
countdown") was corrected to name all three sounds instead.

**Task 5's borrow-checker rationale was wrong.** First-statement placement is required for
CORRECTNESS (both apply handlers have early returns that raise confirmation pages), not for
compilation — disjoint field borrows compile fine either way. Verified by experiment.

**Task 6's plan snippet was stale.** Commit `cde25178`, already on this branch, had extracted the
inline anchor match into a pure function `next_game_from_schedule`. Change made there instead,
which also made it unit-testable (11 tests). One existing test repointed from `"0"` to `"77"`,
because `"0"` now means "fresh launch" rather than "anchor absent".

**Human ruling 2026-08-16 (supersedes the Task 2 approved consequence).** The finished game's
score STAYS in the score tiles indefinitely. Everything else about the end of a game is unchanged.

## Verification

**Automated, at `cb899eb4`:** `just check` exit 0; `cargo build -p uwh-common --no-default-features`
exit 0; refbox 600 tests, uwh-common 78; golden timing traces green with no scenario file edited.

**Walkthrough, 2026-08-16, against a LOCAL MOCK portal** (not the dev portal — see
`reference_local_mock_portal_recipe`). Two courts; court 1 holds games 1 and 3, court 2 holds 2, 4, 5.

Scenario 1 — **first run PARTIAL PASS; re-run after `cb899eb4` PASSED in full.**

First run: dashes during game 3, `END` over `--:--`, middle block/settings/referee rows dashed,
Prior Game keeps 3 with its score, clock stops dead — all correct. But `Resetting game` fired at
game end and wiped the live score. Cause and fix in `cb899eb4`.

Re-run on the `cb899eb4` binary — **PASSED**:
- Score tiles hold the finished score (BLACK 1 / WHITE 0). Human ruling honoured.
- Log shows `Not starting the game clock: no further games on this court`, and **no**
  `Resetting game`, **no** `Starting the game clock` after the stop.
- POST log: game **3 only**. Nothing written against games 2, 4 or 5 on the other court. This is
  the original bug, demonstrably gone.
- Three buzzers fired at the three period ends, and were NOT suppressed by the finished-court
  gating — the end-of-game buzzer still sounds, as intended.

Scenario 1 — **restart half FAILS (Critical), 2026-08-17.** The play-out half stands as recorded
above. The restart half, never actually run before today, fails.

Chain, each step observed against the local mock, refbox `cb899eb4`:

1. Launch in the finished state (note = court "1", game `null`). Screen is **correct**: `END` over
   `--:--`, START NOW greyed, table dashed, log says `No further games scheduled on this court`,
   nothing POSTed.
2. **Within ~18s the note becomes `game_number: "1"`** — the first game of the day, played hours
   earlier. Uncontaminated run: launch, zero interaction, quit at 18s; the note held `"1"` on exit.
   Nothing on screen reflects this.
3. Relaunch from that note: `Restoring portal link ... game Some("1"))` → `Setting upcoming game
   info ... Game { number: "1", Sharks, Barracudas }` → `Setting between games time ... 45s`. The
   line `No further games scheduled on this court` is **absent**. The finished state is gone.
4. Left alone: `[00:00.000 BTWNGMS] Entering first half of game 1`, queues game 3 behind it, plays
   game 1 out, and **POSTs `{"dark":{"value":0},"light":{"value":0}}` against game 1 — twice** —
   then rolls into replaying game 3. Evidence: `mock-portal/posts.log.replay-defect-evidence`.

So the note is a route **around** every guard this branch added: it re-enters the finished court
through the restore path, where the finished conclusion is never reached. Same bug class the branch
exists to kill (a result posted against a game nobody played), different route — see
`feedback_guard_the_class_not_the_case`. The fix must guard the class, not add a sixth gate.

**Root cause — ESTABLISHED 2026-08-17 by three independent routes, which agree.**

*Where the `"1"` comes from:* `next_game_number()` (`tournament_manager/mod.rs:208-229`) falls
through to `game_number + 1` when neither `next_game` nor `no_next_game` is set. At launch
`game_number` is `"0"`, so it answers `0 + 1 = "1"`. Not a scheduled game — an arithmetic
fallback. It would answer `"1"` on any court, for any schedule.

*Why it reaches disk:* `persist_link_session` (`mod.rs:1547`) reads the **cached**
`self.snapshot.next_game_number`. `self.snapshot` is assigned in exactly one place (`mod.rs:775`),
during snapshot generation — and the finished state deliberately holds the break clock stopped,
which is what stops snapshots being generated (the hazard is already noted in the comment at
`mod.rs:1626-1629`). Wire probe on the JSON port, uncontaminated run:

```
00:42:16.273  SNAPSHOT  game_number='0'  next_game_number='1'
00:42:18.115  LOG       No further games scheduled on this court
00:42:18.479  NOTE      game_number='1'      <- written 363ms AFTER the engine knew
00:42:18.752  SNAPSHOT  next_game_number=''  <- cached copy catches up, 273ms later
00:47:52.188  NOTE      game_number=null     <- next token check-in, ZERO interaction
```

*Falsifiable prediction, confirmed:* if this were a permanent wrong belief the note would stay
`"1"`; if it is a stale-cache write the next portal check-in must fix it unaided. It did, at
+5m33s. **So the exposure window is ~5.5 minutes after every launch in the finished state.** This
also retracts the earlier suspicion that the Settings visit caused run 1's revert — it did not;
the heartbeat did.

*Why it is a class defect:* every other site asking "is this court finished?" reads the **live**
engine — `tm.next_game_number()` at `mod.rs:1631` and `mod.rs:5210`. The note-writer is the only
one reading the cached copy, and it is the only answer that survives a restart.

*Fix shape:* have `persist_link_session` ask the live engine like its peers, rather than adding a
sixth gate or an extra re-save trigger. Lock safety checked: none of the three call sites
(`mod.rs:1739`, `3629`, `3940`) holds the `tm` lock at the point of call.

**FIXED 2026-08-17, verified live.** `refbox/src/app/mod.rs` only.

**First attempt was wrong, and the human caught it.** It read the live engine but still wrote on
every save, mapping "I don't know yet" to `None`. Asked to prove a mid-event restart still resumes
correctly, the live test showed the *mirror* of the original bug:

```
01:10:12.152  NOTE  game_number='4'          <- the operator's resume point
01:10:13.734  LOG   Restoring portal link ... game Some("4")
01:10:20.650  NOTE  game_number=None         <- ERASED
01:10:20.712  LOG   Setting upcoming game info ... Game 4   <- 62ms too late
```

Quit inside that window and the next launch offers the *earliest* game on the court, sending the
operator back to the start of the day. Same race, opposite damage. (The human also saw the main
view oscillate `1` → `4` → `1` before settling — the same ignorance window, visible on screen.)

The real defect is broader than "reads the cached snapshot": **the note was written from ignorance**,
filled first with a guess, then with an erasure. `None` was doing double duty for "court finished"
(knowledge) and "no schedule yet" (ignorance).

Final shape — three states, not two:

```rust
enum LinkNoteGame {
    Write(Option<GameNumber>),  // knowledge: the game, or None = court finished
    Unknown,                    // ignorance: leave any existing note untouched
}
```

`link_note_game(&TournamentManager)` distinguishes them through existing public API: a game in
progress → `Write(Some(game_number))`; `next_game_info()` present → `Write(Some(number))`; else a
blank `next_game_number()` means the engine was *told* the court is finished → `Write(None)`;
anything else is the arithmetic guess → `Unknown`. `persist_link_session` returns early on
`Unknown`, touching nothing.

4 tests (`link_note_game_tests`) against a real `TournamentManager`. `just check` exit 0; refbox
604 (600 + 4), uwh-common 78, golden traces green, `--no-default-features` ok.

Three live cases, fresh binary, same rig:

| Case | Setup | Result |
|---|---|---|
| Mid-event resume | note = court 2, game 4 | note **untouched**, `last_active` unchanged; restored `game Some("4")` ✓ |
| Finished court | note = court 1, no game | note stayed blank, never `"1"` ✓ |
| Double restart | quit + relaunch | `No further games scheduled on this court`; no upcoming game, no countdown, no game start, POST log empty ✓ |

Known, accepted consequence: on a genuinely fresh start the note is now created at the first
check-in *after* the schedule is known (~5 min) rather than within seconds with a guess in it. Kill
refbox before that and no note exists — correct, since nothing was known.

Verified against the same rig, same reproduction, fresh binary:

```
BEFORE                                  AFTER
SNAPSHOT next_game_number='1'           SNAPSHOT next_game_number='1'   <- still stale
NOTE     game_number='1'   <- poisoned  NOTE     game_number=None       <- immune
SNAPSHOT next_game_number=''            SNAPSHOT next_game_number=''
```

Quit inside the old poison window, then relaunch: `Restoring portal link ... game None` →
`No further games scheduled on this court`, with **no** `Setting upcoming game info`, **no**
`between games time`, **no** `Entering first half`, and no POSTs. Evidence:
`docs/backlog/court-finished-panel-state/probe-evidence-after-fix-2026-08-17.txt`.

Note the cached-snapshot staleness itself is *unchanged* — deliberately. It is inherent to the
finished state holding the clock stopped. The fix removes the note's dependence on it rather than
trying to keep the cache fresh.

Scenario 2 — **the un-networked residual is NOT cosmetic. Own branch, 2026-08-17.**

Portal unreachable (mock stopped → connection refused), note in the finished state. Every step
observed with zero interaction:

1. Launch: `Restoring portal link ... game None`, `Failed to get event list`, and **no**
   `No further games scheduled on this court` — the finished fact cannot be applied without a
   schedule. Wire probe: `next_game_number='1'` **immediately**, so the phantom game is on screen
   from launch, not just after the break. START NOW is live, since `break_starts_nothing` keys off
   the number being *empty*.
2. Default `nominal_break = 900` counts down; at +15min: `[00:00.000 BTWNGMS] Entering first half of
   game 1`, 2 sounds fired, game plays out unattended.
3. Result **queued to disk**: `{"game_number":"1","black_score":0,"white_score":0,"score_sent":false}`
   — survives refbox exiting.
4. Next launch with the portal reachable: `POST /schedule/games/1/scores {"dark":0,"light":0}`,
   queue drained. **The real result is overwritten.**

So the outage delays the corruption, it does not contain it. Evidence:
`docs/backlog/court-finished-panel-state/offline-phantom-log-2026-08-17.txt` and
`posts-offline-phantom-evidence-2026-08-17.log`. The committed fix behaved correctly throughout —
the note was untouched while nothing was known, and recorded game 1 only once a game was genuinely
in progress. This is a *separate* defect, not a regression.

**Root cause (traced, not guessed):** the finished fact IS carried across a restart —
`pending_restore_court_finished = note.court.is_some() && note.game_number.is_none()`
(`mod.rs:2735`). But it is only ever *consumed* inside `next_game_from_schedule`, within a match on
`schedule.games` (`mod.rs:7111`). No schedule → never applied → the arithmetic guess wins.

**Human design proposal 2026-08-17** — encode "last game" as a sentinel number (`9999`, or
`9991..9995` per court) so it survives offline. Principle is right (make "finished" positive, not an
absence — the same lesson as `LinkNoteGame::Unknown`); the sentinel is the wrong carrier:
`GameNumber` is a `String` that travels to the LED panel, overlay and portal, so `9999` would be a
real displayed/POSTed value; per-court variants duplicate the note's own `court` field (two sources,
one fact — today's bug class); and every game-number site would need "…unless 999x", which is
enumeration, not a guard. Counter-proposal: an explicit `court_finished: bool` in the note (local,
versioned file — no hardware/portal format involved), **and apply it on the offline path**, not only
inside the schedule lookup. Restoring a remembered "finished" while offline is strictly safer than
inventing a game: worst case the operator picks one, or the next refresh corrects it.
Lifetime is already solved — `set_next_game` clears `no_next_game`, and
`pending_restore_court_finished = false` is the first statement of both APPLY handlers and of game
start, so picking a game resets it with no new logic.

Scenario 3 — **PASSES on REFRESH; the "no operator action" claim is FALSE.** 2026-08-17.

Reached the finished state by *playing* game 3 (score BLACK 1 / WHITE 0), then added game 6
(Squid v Manta, court 1, starts 10:30) to the mock schedule at 02:12:02 and touched nothing.

*Play-out re-confirmed on the fixed binary:* `Ending game 3. Score is Black: 1, White: 0` →
`Last game on this court is over; stopping the clock` →
`Not starting the game clock: no further games on this court`. POST: **game 3 only**, with its goal
in the stats payload. Screen: `END --:--`, START NOW greyed, BLACK 1 / WHITE 0 held, Prior Game 3
(Eels 1 / Turtles 0), all settings and referee rows dashed.

*The automatic path does not exist.* After 8+ minutes: **2 schedule fetches all session**, both
*before* game 6 existed (startup 02:05:57, end-of-game 02:11:21). The portal check-in ran at
02:16:57 — so the app was alive and working — but it only validates the token; it does not re-read
the schedule. Zero mentions of `"6"` in the log until the operator pressed REFRESH.

> Mechanism: refbox re-reads the schedule at startup and at the end of a game. On a finished court
> no more games end, so it never re-reads it again. A late-added game is never noticed, however long
> the refbox is left running.

*REFRESH recovers fully* (02:24:17): `Setting upcoming game info ... Game { number: "6", Squid,
Manta }` → `Next Game Info set to ... "6"` → `Setting between games time ... 90s`. Screen:
`NEXT GAME 1:26` counting down, Current Game 6, Game Block 4:50, Squid/Manta, START NOW **green**,
and BLACK 1 / WHITE 0 still on the tiles — every sub-claim of the scenario met, including "the
previous game's score stays visible until the new game starts". POSTs still game 3 only.

*Residual:* the note is only rewritten on the ~5-min check-in, so between the REFRESH and that
write it still says "finished". Quit in that window → next launch comes back finished and needs
another REFRESH. Mild, recoverable, no bad data.

**Severity:** operational gap, not corruption — there is a manual recovery and nothing wrong is
sent. But the operator must *know* to press REFRESH, and nothing on screen hints that the schedule
changed. Own branch; a candidate fix is to keep re-reading the schedule while a court is finished,
which is exactly the state where the app currently stops asking.

**Open:**
- Scenarios 4–8, plus a clean re-run of scenario 1 (never yet run idle, untouched, for 5+ min).
  **NOT PR-ready.**
- Offline phantom → own branch, design first. Do not fold into this one.
- Finished court never re-reads the schedule (scenario 3) → own branch.
- Rig note: mock timings raised to 90s halves / 20s half-time / 90s break at 02:04 so WSL's UI lag
  stops making taps impossible (backup `schedule.json.before-scenario3`). Earlier scenarios ran at
  30/10/45. This also fixes scenario 6's otherwise 15-second window.
- Human ruling 2026-08-17: the `sound_controller` `trigger_flash().unwrap()` crash at the end-of-game
  buzzer (`mod.rs:466`, cascading to `647`) is a **WSL artifact, not a code defect** — the flash
  channel only backs up because WSL starves the consumer; the code is untouched by this branch and
  has years of field use. Recorded, not filed.
- ~~Watch during scenarios 4 and 7: does a manual Settings pick survive?~~ **RULED OUT by trace,
  2026-08-17.** All three Apply paths call `tm.set_next_game(edited.game_number)` (`mod.rs:1839`,
  `1894`, `1988`), so an operator's pick is a game the engine *knows* and is recorded normally.
  Scenarios 4 and 7 still exercise it, but it is not an open regression.
  Human ruling 2026-08-17: manual mode (portal off) need not remember anything across a restart —
  and should not, so moving between events clears. That is already the shipped behaviour: with the
  portal off `persist_link_session` takes its **delete** branch (`mod.rs:1574-1576`), so no note
  survives at all. The fix therefore forgets exactly two things, both of which should be forgotten:
  a number the engine only guessed, and a court whose day is over.
- Uncommitted at this point; branch still 20 commits at `cb899eb4`.
- **LED panel shows `NEXT GAME IN 0:30`, frozen and unexplained.** Established: the engine's clock
  is definitively 0, the panel draws only `secs_in_period`, and only one refbox + one simulator
  were running. Those three facts are mutually inconsistent, so something between engine and panel
  substitutes a value. **Two attempts to explain it from code reading were both wrong — instrument
  the wire, do not theorise.** Captured in `docs/backlog/court-finished-panel-state/NOTE.md`.
- Buzzer precision could not be established from logs: two runs disagreed (1.0s vs 3.7s gap between
  buzzer and expiry), and the clock stop/edit events are not logged at INFO. The earlier "fires 1s
  early" claim was NOT established by evidence — the truncation mechanism is real in the code, but
  the actual error needs deliberate measurement on its own branch.
- Sound audibility could not be verified at all: WSL has no audio device. Needs the Pi.

**Out of scope, needs its own branch:**
- The end-of-game buzzer fires ~1s EARLY. `secs_in_period` is truncated with `as_secs()`, so it
  reads 0 for the whole final second and the buzzer keys off `secs_in_period == 0`. Pre-existing
  master behaviour, affects every game, sits under the golden traces.
- The LED panel cannot display `--:--`: its snapshot struct has no next-game field. A wire-format
  change reaching the physical firmware.
