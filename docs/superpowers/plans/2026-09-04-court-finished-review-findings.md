# Court-Finished Review Findings — Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the twelve open findings from the 2026-09-04 review of the rebased
`fix/refbox/court-finished-behaviour` branch, so the branch can go to its walkthrough and PR.

**Architecture:** Order is chosen so the four findings that need no ruling from Eric land first,
and the shared predicate lands before the call sites that should use it. The five findings that
turn on what the app *should* do are held until Eric rules, one question at a time. Nothing here
re-walks any criteria: the walkthrough is run once, at the end, against a green tree.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13, `just` task runner.

**Spec:** `docs/superpowers/specs/2026-08-17-court-finished-behaviour-design.md`
**Decisions:** `docs/superpowers/specs/2026-08-17-court-finished-behaviour-decisions.md`
**Preceding plan:** `docs/superpowers/plans/2026-09-04-court-finished-rebase-and-recheck.md`

## Global Constraints

- MSRV Rust 1.85; edition 2024. No APIs newer than 1.85.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- `uwh-common` must still compile `no_std`. Task 1 touches it — check that first.
- No new `unwrap()`/`expect()` in production code without a comment proving it cannot panic.
- No new dependencies.
- **`--:--` stays untranslated.** Eric's ruling, 2026-09-04: it reads the same in every language.
- Heavy process applies (`.claude/rules/plan-execution.md`): this touches `uwh-common`, the game
  clock state machine, and the LED/overlay wire format. Per-task verification, strict deviation
  tracking.
- Branch, commit, push, PR, and branch **rename** all require Eric's explicit approval.
- `just check` is host-only and blind to a Windows break; `just lint` is not `--all-targets` and a
  binary-only dead-code error can hide behind an all-targets pass (that is how finding 5 of the
  rebase escaped). Run both forms.

## The twelve findings

| # | Finding | Ruling needed? | Task |
|---|---|---|---|
| 1 | Custom-site launch parks at `END --:--`, unrecoverable without a pick | **yes** | 7 |
| 2 | `next_game_number()` blank on a path `no_startable_next_game()` misses | no | 2 |
| 3 | App Options manual switch resumes numbering at "10", not "1" | **yes** | 8 |
| 4 | App Options mutates the engine mid-game with no confirmation | **yes** | 9 |
| 5 | `config_string_game_num` renders "Error: " instead of a dash | no | 1 |
| 6 | Court finished mid-break shows dashes over a still-running clock | **yes** | 5 |
| 7 | `LinkNoteGame::Unknown` unreachable; its test is circular | no | 4 |
| 8 | `NothingScheduled` collapsed into `CourtFinished` | **yes** | 6 |
| 9 | `reset_game_time` left at zero on a revived court | no | 3 |
| 10 | Finished-court predicate written four times, a fifth site missed | no | 1 |
| 11 | `--:--` untranslated | **answered, not fixed** | 10 |
| 12 | Branch named `fix/refbox/…` while changing `uwh-common` | approval | 11 |

---

## Task 1: One definition of "this court is finished" (findings 10 and 5)

**Files:**
- Modify: `uwh-common/src/game_snapshot.rs` (next to `next_game_number`, line ~140)
- Modify: `refbox/src/app/mod.rs:1047`, `refbox/src/app/view_builders/main_view.rs`,
  `refbox/src/app/view_builders/shared_elements.rs:641`,
  `refbox/src/app/view_builders/shared_elements.rs:1170`,
  `refbox/src/app/view_builders/game_info_table.rs`

**Interfaces:**
- Produces: `GameSnapshot::court_schedule_finished(&self) -> bool`, returning
  `self.current_period == GamePeriod::BetweenGames && self.next_game_number.is_empty()`.
  Tasks 5 and 6 call it.

The rule already half-lives in `uwh-common`: `GameSnapshot::next_game_number()` returns
`Option<&GameNumber>` and reports `None` for a blank. The *derived* question — "is this court
finished?" — is instead re-implemented at four call sites, and a fifth
(`config_string_game_num`) was missed, which is finding 5: `games.get("")` returns `None` and the
arm produces `fl!("error", number = "")`, so the operator reads **"Error: "** where the game-info
table reads "-".

- [ ] **Step 1: Add the predicate to `uwh-common` with a doc comment stating the blank rule**,
      immediately below `next_game_number`. Keep it `no_std`-safe — no allocation, no `std`.

- [ ] **Step 2: Unit-test it in `uwh-common`** for all four combinations of period
      (BetweenGames / FirstHalf) against next-game-number (blank / "11"). Only
      BetweenGames+blank is true.

- [ ] **Step 3: Verify `no_std` still builds**

```bash
cargo build -p uwh-common --no-default-features
```

- [ ] **Step 4: Replace all five call sites with the predicate.** The four existing ones are
      behaviour-preserving. The fifth is the fix: give `config_string_game_num`'s `next_game`
      a blank arm that yields the same dash constant the game-info table uses
      (`NO_VALUE`, `game_info_table.rs:18`) rather than falling into the `fl!("error", …)` arm.

- [ ] **Step 5: Test the fifth site.** Assert `config_string_game_num` yields the dash, not an
      error string, for a BetweenGames snapshot with a blank next-game number and a schedule
      present. Then break the production line on purpose and confirm the test goes red — a check
      never seen failing is not a check.

- [ ] **Step 6: `just check`, then commit**

```bash
git commit -m "refactor(uwh-common): give the finished-court rule one definition"
```

---

## Task 2: Close the blank-number gap in the engine (finding 2)

**Files:** Modify: `refbox/src/tournament_manager/mod.rs` (`no_startable_next_game`, line ~357)

`next_game_number()` returns blank on three paths: an explicit `no_next_game`, being
`schedule_linked` with nothing named, **and** a `game_number` that will not parse as an integer.
`no_startable_next_game()` covers only the first two:

```rust
self.next_game.is_none() && (self.no_next_game || self.schedule_linked)
```

So with non-numeric game numbers — and the codebase's own fixtures use `"G27"`/`"G1"` — the UI
greys START NOW and shows `END --:--` (both keyed on the blank number) while the engine still
believes a game is startable, and `update()` will auto-start one with an empty game number.

The fix guards the class rather than the third case: the two questions are the same question.

```rust
fn no_startable_next_game(&self) -> bool {
    self.next_game_number().is_empty()
}
```

- [ ] **Step 1: Write the failing test.** Build a manager with `game_number = "G27"`,
      `next_game = None`, `no_next_game = false`, `schedule_linked = false`, BetweenGames, clock
      running. Assert `no_startable_next_game()` is true and that letting the break expire starts
      nothing. Run it and watch it fail before touching the implementation.

- [ ] **Step 2: Apply the one-line change.**

- [ ] **Step 3: Run the whole engine suite.** This predicate gates the between-games expiry and
      `start_play_now`; a regression here stops games starting at all. Expect no other test to
      move.

- [ ] **Step 4: `just check`, then commit**

```bash
git commit -m "fix(refbox): treat every blank next-game number as unstartable"
```

---

## Task 3: Recompute the mid-break reset on a revived court (finding 9)

**Files:** Modify: `refbox/src/tournament_manager/mod.rs` (`apply_next_game_start`, line ~1290)

`end_game`'s finished branch parks the clock and sets `reset_game_time = Duration::ZERO`
(line 1324). The ordinary path computes it as
`time_remaining_at_start.saturating_sub(self.config.post_game_duration)` (line 1342).
`apply_next_game_start` — the path a revived court takes when a game is added and adopted on
REFRESH — sets `clock_state` to `CountingDown` but never recomputes `reset_game_time`. The
mid-break reset at line 1474 (`game_clock_time(now) <= self.reset_game_time`) is then only
satisfied at 0:00, so the finished game's score and penalties stay on the display right up to the
next kickoff instead of clearing `post_game_duration` early as configured.

- [ ] **Step 1: Write the failing test.** End the last game on a court (finished state), add a
      next game, call `apply_next_game_start`, and assert `reset_game_time` equals
      `time_remaining_at_start - post_game_duration` rather than zero.

- [ ] **Step 2: Set `reset_game_time` in `apply_next_game_start`** using the same expression as
      line 1342, immediately after `time_remaining_at_start` is computed. Do not copy the formula
      by hand into a third place — extract it if this makes a third caller.

- [ ] **Step 3: Confirm the ordinary between-games path is unchanged** — `reset_game_time` is
      already correct there, so the existing assertion at line 4431 must still pass untouched.

- [ ] **Step 4: `just check`, then commit**

```bash
git commit -m "fix(refbox): clear the old score early again on a revived court"
```

---

## Task 4: Remove the unreachable note state and its circular test (finding 7)

**Files:** Modify: `refbox/src/app/mod.rs` (`link_note_game` ~line 1086, `LinkNoteGame`,
`persist_link_session` ~line 2226, and `mod link_note_game_tests` ~line 8915)

`link_note_game` is called only from `persist_link_session`, inside `if self.uses_remote()`. Both
`commit_source` and the startup block keep `schedule_linked == uses_remote()`, so whenever it runs
`schedule_linked` is true — and with `next_game == None` that makes `next_game_number()`
short-circuit to blank before the arithmetic fallback. Line 1086 is therefore always true, the
function always returns `Write(None)`, `LinkNoteGame::Unknown` never occurs, and the early
`return` in `persist_link_session` is dead.

The test that guards it, `the_note_is_left_alone_until_the_schedule_is_known`, builds a
`TournamentManager` **without** `set_schedule_linked(true)` and asserts `next_game_number() == "1"`
— a state the call site cannot produce. It verifies a configuration the code never sees. This is
the circular-fixture trap: the fix is to change the code or the assertion, never to keep a test
passing against a fixture reality cannot reach.

- [ ] **Step 1: Decide which way to close it, and write the decision down.** Either the protection
      is wanted — in which case `link_note_game` must be able to observe the not-yet-known state,
      and the test should drive it through the real call site — or it is not, in which case
      `Unknown`, the dead `return`, and the test all go. Prefer deletion: the state is
      unreachable *by construction*, because linkage is kept in lockstep with the source.

- [ ] **Step 2: Make the change, and prove the remaining behaviour by mutation.** Whichever way
      it goes, break the surviving production line on purpose and confirm a test goes red.

- [ ] **Step 3: `just check`, then commit**

```bash
git commit -m "fix(refbox): drop the link-note state the call site cannot reach"
```

---

## Task 5: Court finished mid-break (finding 6) — **RULING NEEDED**

**Files:** Modify: `refbox/src/tournament_manager/mod.rs` (`set_no_next_game`, line ~341)
and/or `refbox/src/app/view_builders/shared_elements.rs:640`

`set_no_next_game()` sets `next_game = None; no_next_game = true` and **does not touch the
clock**. The call site's own comment claims otherwise: "Both are definite 'nothing is next here'
answers, and both park the clock." One of the two is wrong.

The consequence, when a REFRESH moves the upcoming game off this court mid-break: the banner
switches to `END --:--` immediately while the countdown keeps running for its remaining minutes,
all break sounds are suppressed, and the mid-break `reset()` still fires and wipes the finished
game's score — the opposite of the spec's "final score stays on screen".

**Ask Eric before writing code.** What should the operator see when a court becomes finished
while a break is already counting down? Recommendation: **park the clock at 0:00 immediately**, so
the display and the clock agree and the code matches the comment — it is the same state a court
reaching the end of its own schedule is left in, and the safer failure direction (nothing
auto-starts). The alternative — let the countdown finish and only then park — keeps a truthful
clock but means the banner promises an end that has not arrived.

- [ ] **Step 1: Put the question to Eric with that recommendation. Wait.**
- [ ] **Step 2: Write the failing test for whichever behaviour is ruled.**
- [ ] **Step 3: Implement, and correct whichever of comment or code was wrong.**
- [ ] **Step 4: `just check`, then commit.**

---

## Task 6: An unrecognised court is not a finished one (finding 8) — **RULING NEEDED**

**Files:** Modify: `refbox/src/app/mod.rs:6393` (and the sibling arm at ~6347)

`next_game_from_schedule` deliberately distinguishes `CourtFinished` from `NothingScheduled`, and
the enum's doc comment says they must be "kept apart so an empty court or an unreadable schedule
is never mistaken for a completed one". Both call sites then collapse them into the same
`tm.set_no_next_game()`.

So if a schedule is re-uploaded with courts renamed — "Court 1" becomes "Pool 1" — or the court
was carried in from another event, the refbox reports the day as over: blank number, clock parked,
banner `END`, START NOW greyed, and `persist_link_session` writes `current_game: None`. The court
in fact has a full afternoon of games under a new name.

**Ask Eric before writing code.** What should the operator see when the selected court appears
nowhere in the schedule? Recommendation: **treat it as "pick a game", not "day over"** — hold the
engine's state, leave the clock alone, and prompt for a pick, the same answer the spec already
gives for a court with no history. Reporting a finished day is the one answer that is
affirmatively wrong, and it is the one that stops the operator looking for the real cause.

- [ ] **Step 1: Put the question to Eric with that recommendation. Wait.**
- [ ] **Step 2: Split the arms and test both**, including a case that proves a genuinely finished
      court still reads finished — the distinction is worthless if it only moves the bug.
- [ ] **Step 3: `just check`, then commit.**

---

## Task 7: Give the link note a site, so a custom session resumes (finding 1)

**Ruled by Eric, 2026-09-04:** give the note a site now, rather than accepting that custom
sites need a game picked after every restart. I flagged this as new scope on a rebase-repair
branch; Eric chose it, so it is in.

**Files:**
- Modify: `refbox/src/portal_manager/link_session.rs` (the `LinkSessionFile` format)
- Modify: `refbox/src/app/mod.rs` (`decide_restore`, the startup restore branch,
  `persist_link_session`)

**Interfaces:**
- Produces: `link_session::NoteSite`, an enum — `Portal` (which of the two portals is still
  settled by the existing `mode` field) and `Custom { address: String }`. `Default` is `Portal`.
- `LinkSessionFile` gains `#[serde(default)] pub site: NoteSite`.
- `decide_restore` gains a `current_site: &NoteSite` parameter.

**Why it works at all:** the data is already being written. `persist_link_session` is gated only
on `uses_remote()`, which is true for Custom, so a custom session already writes the note —
startup just refuses to read it, because an event id cannot identify a site (event ids collide
between the Portal and a custom site by design). The address the note needs to carry is the one
piece missing.

**The address is the whole check.** `SiteTarget`'s own doc says a custom address "includes the
event", so comparing the configured address with the note's answers both "same site?" and "same
event?" in one comparison. No separate event check is needed, and editing just the event in the
URL correctly invalidates the note.

**No version bump, deliberately.** The field is purely additive and its default reproduces v2
behaviour exactly (v2 notes read as `Portal`, which is the only kind v2 startup would restore).
Holding at v2 means a rolled-back binary still reads the note instead of renaming it corrupt and
losing the link mid-tournament — which matters on a Pi with self-update. The v1 -> v2 bump was a
semantic change and needed one; this is not.

- [ ] **Step 1: Add `NoteSite` and the field in `link_session.rs`,** with tests that a v2 note
      (no `site` key) loads as `Portal`, and that a round-tripped `Custom` note keeps its address.

- [ ] **Step 2: Extend `decide_restore` and test the matrix.** A Portal note restores exactly as
      today, including into a Manual-configured refbox. A Custom note restores only when the
      configured source is Custom *and* the configured address matches. A Custom note against a
      different address does not restore. Run before implementing and watch the new cases fail.

- [ ] **Step 3: Teach the startup branch the custom case.** Replace the blanket
      "custom site restored; portal link note ignored" skip. A restored Custom note must set
      `source = Custom` and must NOT repoint the client — it is already built from the same
      configured address the note was matched against.

- [ ] **Step 4: Write the site in `persist_link_session`,** derived from the committed source and
      configured custom address, so the note it writes is the note startup will accept.

- [ ] **Step 5: Prove it by mutation** — break the address comparison and confirm a test goes red.

- [ ] **Step 6: `just check`, then commit.**

## Task 8: Manual numbering after App Options (finding 3) — **RULING NEEDED**

**Files:** Modify: `refbox/src/app/mod.rs:2351` (`apply_app_options` manual path)

Two routes to one operator intent behave differently. The Game page calls
`reset_to_manual_break`, which sets `game_number = "0"` so manual numbering restarts at 1. App
Options runs only `clear_portal_next_game()` + `commit_source(Manual)`, leaving `game_number` at
the portal's value — so after portal game 9 the break auto-starts "10", which on a two-court event
is the *other* court's number: exactly the invented number this branch exists to remove. With a
non-numeric portal number it degrades into Task 2's blank case.

**Ask Eric.** Recommendation: **make App Options match the Game page** and reset to "0". One
operator intent should have one behaviour, and the Game page's is the correct one.

- [ ] **Step 1: Put the question to Eric. Wait.**
- [ ] **Step 2: Test that both routes leave the same engine state**, so they cannot drift again.
- [ ] **Step 3: `just check`, then commit.**

---

## Task 9: Mid-game confirmation for App Options (finding 4) — **RULING NEEDED**

**Files:** Modify: `refbox/src/app/mod.rs:2361` (`apply_app_options`)

`apply_game_options` raises `ConfirmationKind::SwitchToManualFromApply` when the operator switches
to MANUAL mid-game — "End game and apply" vs "Keep game and apply". `apply_app_options` has no
mid-game gate at all: it takes the engine lock and discards the resolved `NextGameInfo` (with its
timing rule) and `next_scheduled_start`, silently, with a game in progress.

**Ask Eric.** Recommendation: **raise the same confirmation**, reusing the existing
`ConfirmationKind` rather than adding a second one. Discarding a running game's follow-on timing
without asking is the kind of thing that is only noticed at the next kickoff.

- [ ] **Step 1: Put the question to Eric. Wait.**
- [ ] **Step 2: Implement, reusing the existing confirmation variant.**
- [ ] **Step 3: `just check`, then commit.**

---

## Task 10: Record the `--:--` ruling (finding 11) — answered, not fixed

**Files:** Modify: `refbox/src/app/view_builders/shared_elements.rs:593`

Eric ruled on 2026-09-04 that `--:--` needs no translation: it reads the same in every language.
The constant already carries a comment saying it is punctuation rather than translated text, which
is the same position — so nothing about the string changes.

- [ ] **Step 1: Add the ruling and its date to the existing comment**, so the next review sees the
      question was asked and answered rather than missed. No behaviour change, no new key, no
      touching the 15 locale files.

- [ ] **Step 2: Commit with the doc reconciliation of Task 12** — this is one line and does not
      warrant its own commit.

---

## Task 11: Branch name scope (finding 12) — **APPROVAL NEEDED**

`.claude/rules/workspace.md` says the branch scope must reflect the broadest crate involved. This
diff adds `Schedule::next_game_on_court`, changes `GameSnapshot::next_game_number()` semantics, and
Task 1 adds `court_schedule_finished()` — all public `uwh-common` API consumed by `refbox`,
`overlay` and the overlay bridge. The branch is `fix/refbox/court-finished-behaviour`; the rule
points at `fix/uwh-common/court-finished-behaviour`.

- [ ] **Step 1: Ask Eric whether to rename.** Recommendation: **rename**. It is the cue reviewers
      read to know a shared-type change is in play, and this branch is unpushed with no PR, so a
      rename costs nothing now and cannot be done cheaply later. Renaming is a git operation and
      needs his say-so.

```bash
git branch -m fix/refbox/court-finished-behaviour fix/uwh-common/court-finished-behaviour
```

Note: the worktree directory name and `.superpowers/sdd/` path keep the old name. Harmless, but
say so rather than leaving it to surprise someone.

---

## Task 12: Re-verify, then hand over the walkthrough

- [ ] **Step 1: `just check` plus the strict lint form.** A binary-only dead-code error hides
      behind an `--all-targets` pass — that is how the rebase's dead `font_family_id` escaped.

```bash
just check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p uwh-common --no-default-features
```

- [ ] **Step 2: Re-run the `code-review` skill against the whole rebased diff.** Every task above
      changes behaviour, so the 2026-09-04 review no longer covers the diff. Pass an explicit
      `origin/master...HEAD` target — the skill otherwise picks a stale local base.

- [ ] **Step 3: Update the two docs.** Record each ruling in
      `docs/superpowers/specs/2026-08-17-court-finished-behaviour-decisions.md`, and note in
      `docs/backlog/court-finished-panel-state/WALKTHROUGH-RESULTS-2026-08-31.md` which findings
      changed behaviour the earlier walkthrough had observed.

- [ ] **Step 4: Rebuild the binary before any walkthrough** — the rig runs
      `./target/debug/refbox` and a stale binary silently walks the old code.

- [ ] **Step 5: Hand Eric criteria 2, 4, 5 and 9, one at a time.** Criterion 9 still needs his
      decision on `wsl --shutdown`, which kills every WSL session including peer Claude sessions.
      Walk 9 from the GAME page, not the App page.

## Deviations

**1. Findings 3 and 4 were not reachable, and Eric caught it — twice.** The review described an
operator changing the game source on App Options. There is no such control there (its only cycle is
App **Mode**), the MANUAL GAMES button is on the Game page and only *stages* the change, and the
Game page cannot be left carrying a staged change: its exits are its own APPLY and a Cancel whose
`revert_from_snapshot` restores `source` from `PageEntrySnapshot::Game`. So `edited.source` always
equals the committed source when `apply_app_options` runs, and the branch's guard there asked for
Manual and not-Manual at once. Not a rebase artifact — the App page had the same controls at this
branch's own original base. Consequence: ~30 lines the branch added across `ff493841`, `406824d4`
and `717823ed` were removed as dead, on Eric's ruling.

**2. Finding 8 was traced to a defensive branch, not a live one.** `NothingScheduled` needs a court
to vanish from an event it was chosen from — a restored note's court is never re-checked against a
later schedule. Eric confirmed courts do not disappear mid-event, so it was routed with the other
answers the refbox declines to act on (one line) rather than given a state of its own. I had framed
it as routine and had to withdraw that.

**3. Finding 1 grew, on Eric's ruling.** I recommended accepting that custom sites need a game
picked after each restart and logging the improvement. Eric chose to give the link note a site
instead, which I flagged as new scope on a rebase-repair branch. `NoteSite` records `Portal` or
`Custom { address }`; a custom address already includes the event, so one comparison answers both
"same site?" and "same event?". No version bump — additive, default reproduces v2.

**4. Finding 11 is answered, not fixed.** Eric ruled `--:--` needs no translation: it reads the same
in every language. The constant already said as much; the ruling is now recorded beside it.

**5. Finding 7's guard was kept, not deleted.** The plan preferred deletion. Writing a guess into the
link note has twice re-posted a finished court's day, so on an asymmetric risk the three-line net
stays — with the test rewritten to stop claiming it covers live behaviour, and a new test pinning
the invariant that makes it unreachable.

**6. The re-review found a blocking regression this branch causes, plus three defects in the fixes
above.** `switch_to_source` — a path master added — resets the clock through
`reset_to_manual_break`, which commit `717823ed` taught to drop the schedule link. On a
remote-to-remote switch that left the engine unlinked while the app stayed remote, so it resumed
arithmetic numbering, auto-started a phantom game 1 on the site just switched to, and posted a 0-0
against it. Fixed with `reset_for_site_switch`. The three self-inflicted ones: the parking made an
existing expiry-guard test pass for the wrong reason; arming the mid-break reset on every
`apply_next_game_start` reintroduced the held-on score it was added to fix; and routing
`no_startable_next_game` through `next_game_number` multiplied an `error!` onto the per-tick path.

**7. One re-review finding is recorded, not closed.** `NoteSite::Portal` does not separate portal
*environments*, so a note written under `UWH_PORTAL_URL_OVERRIDE` restores into production. It
changes the note every ordinary session writes, so it wants its own branch.

**8. Verification.** `just check` exit 0 at 48 commits: **1196 tests passing, 0 failing**. Every fix
carries a test that was broken on purpose first. `cargo clippy --workspace --all-targets` fails on
three pre-existing issues outside this diff (`overlay-bridge/src/status.rs`,
`keypad_pages/player_grid.rs`, and a `mod.rs` line that is on master); the `--all-features` form
cannot run here at all — it fails building `grafton-ndi`, which needs the NDI SDK.
