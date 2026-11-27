# Stale-Tap Class Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ignore and log any operator tap that arrives after the screen it was drawn on has gone, for every button, now and in future. Also fix the TIME EDIT crash when a team timeout ends while the page is open.

**Architecture:** `view()` wraps every message its widgets produce in `Message::Tap { screen, message }`, where `screen` is the `ScreenKind` it was drawn on. The very first thing `update()` does is pass the message through a pure function, `untag`, which drops a late tap (unless its message must never be dropped) and returns the bare message otherwise. Nothing after that point ever sees a `Tap`. The hand-written `tap_outlived_its_screen` list is removed.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13. Crate `refbox` only (a BIN crate: tests run with `cargo test -p refbox --bin refbox <filter>`).

**Spec:** `docs/superpowers/specs/2026-11-27-stale-tap-class-guard-design.md` (until the branch exists: `docs/backlog/stale-tap-class-guard/design.md` in the main checkout). Executors read both.

## Global Constraints

- Scope: `refbox/src/app/mod.rs` and `refbox/src/app/message.rs` only. No other crate, and no other file without asking.
- MSRV 1.85, edition 2024. No let-chains (`if let … && …`); nothing checks the MSRV locally.
- No new dependencies. No `unwrap()`/`expect()` in production code. No `#[allow(...)]`.
- Every new exhaustive classification has **no `_` arm** (`screen_kind`, `may_drop_when_late`).
- Every cargo command needs the coordinator's GO first, runs with `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4` (`RUST_TEST_THREADS=1` for a full test run, because of the ALSA flake), and is followed by "heavy done". A build in a fresh worktree (empty `target/`) is announced to the coordinator before it starts, and waits for its clear.
- Helpers leave changes uncommitted. The main session reviews and commits (`fix(refbox): …`).
- Heavy process: a per-task review (Opus) and a whole-branch review at the end.

## Review Focus

1. **A release lost on a screen change.** `AlarmReleased` or `TimeoutReviveReleased` drawn on one screen and arriving on another must still act, or the buzzer keeps sounding. Pinned in Task 2 (`releases_are_never_dropped`).
2. **Fast typing treated as a screen change.** Digits, +/- and colour taps change the values on a screen, never its kind. Pinned in Task 1 (`contents_do_not_change_the_kind`) and Task 2 (`same_kind_tap_passes`).
3. **TIME EDIT APPLY/CANCEL misread as a side exit** if the unwrap ran after `closed_by_apply_or_cancel` is computed. Pinned in Task 2 (`tagged_apply_comes_out_as_apply`), and checked in Task 3's review against the line order in `update()`.
4. **Updates page:** download progress advancing must not drop BACK. Pinned in Task 1 (the `Updates` case in `contents_do_not_change_the_kind`).
5. **The tagging never wired into `view()`.** No unit test can see it. Pinned by a debug log line per tagged tap (Task 3) and a walkthrough step that fails without it.

---

### Task 0 (main session): branch, worktree, docs

- [ ] PO approval to create the branch (an approval gate in this project).
- [ ] `git fetch origin master`, then make the worktree `~/projects/uwh-refbox-rs-stale-tap` on the new branch `fix/refbox/stale-tap-class-guard` from `origin/master`. Never branch in the shared checkout.
- [ ] Move `design.md` to `docs/superpowers/specs/2026-11-27-stale-tap-class-guard-design.md` and this plan to `docs/superpowers/plans/2026-11-27-stale-tap-class-guard.md` in the worktree (absolute paths). Commit: `docs(refbox): add stale-tap class guard design and plan`.
- [ ] Before the first cargo run: `mv ~/projects/uwh-refbox-rs-start-now-buzzer/target ~/projects/uwh-refbox-rs-stale-tap/target` (PR #4798 merged 2026-11-27, that worktree is done). Registry dependencies stay compiled, only the workspace crates rebuild, so this is not a fresh build. Then tell the coordinator the buzzer worktree is free for cleanup, and ask for a GO.

### Task 1: `ScreenKind` and `screen_kind`

**Files:**
- Modify: `refbox/src/app/message.rs` (new `ScreenKind` and its sub-enums, next to `ConfigPage`)
- Modify: `refbox/src/app/mod.rs` (new `fn screen_kind`, near `AppState`; new test module `screen_kind_tests`)

**Interfaces:**
- Produces: `pub enum ScreenKind` (`Debug, Clone, Copy, PartialEq, Eq`), and `fn screen_kind(state: &AppState) -> ScreenKind`.
- Sub-kinds: `KeypadKind { AddScore, Penalty, GameNumber, TeamTimeouts, FoulAdd, WarningAdd, PortalLogin }`; `ConfirmationKindTag`, one payload-free variant per `ConfirmationKind` variant. Reuse `ConfigPage`, `LengthParameter`, `ListableParameter` and `BeepTestConfigPage` directly (all already `Copy + Eq`).
- `ScreenKind` has one variant per `AppState` variant. Fields follow the spec's table: `Keypad(KeypadKind)`, `EditGameConfig(ConfigPage)`, `Confirmation(ConfirmationKindTag)`, `ParameterEditor(LengthParameter)`, `ParameterEditorHelp(LengthParameter)`, `ParameterList(ListableParameter)`, `BeepTestSettings(BeepTestConfigPage)`, `ScoreEdit { is_confirmation: bool }`. Every other variant has no fields (including `Updates` and `PortalAttentionAction`).

- [ ] **Step 1: Write the failing tests** in `screen_kind_tests`:
  - `contents_do_not_change_the_kind`: pairs of states that differ only in excluded fields map to the same kind. One pair per excluded field in the spec's table: keypad number (`KeypadPage(Penalty…, 7)` vs `…, 42`); `TimeEdit` durations; `ScoreEdit` scores; overview scroll indices; `ParameterEditor` duration and flag; `ParameterList` index; `PortalDetailPage.scroll_index`; `PortalAttentionAction` `discard_armed` and item; `Updates` with two different `state`s; `ConfirmScores` scores.
  - `type_changes_the_kind`: a Penalty keypad ≠ a WarningAdd keypad; `EditGameConfig(Main)` ≠ `EditGameConfig(Game)`; `ScoreEdit { is_confirmation: true }` ≠ `{ false }`; `ParameterEditor(Half, …)` ≠ `ParameterEditor(HalfTime, …)`; two different `ConfirmationKind` variants ≠; `MainPage` ≠ `ConfirmScores`.
- [ ] **Step 2: Run** `cargo test -p refbox --bin refbox screen_kind_tests`. Expected: fails to compile (no `screen_kind`).
- [ ] **Step 3: Implement** `ScreenKind`, its sub-enums and `screen_kind`. Exhaustive matches on `AppState`, `KeypadPage` and `ConfirmationKind`, with no `_` arm.
- [ ] **Step 4: Run** the same command. Expected: PASS. (No clippy yet: `refbox` is a bin crate, so items only tests use are dead code to clippy until Task 3 wires them in. Clippy runs in Task 3.)
- [ ] **Step 5: Commit** `fix(refbox): name each screen's kind for the late-tap guard`.

### Task 2: the `Tap` wrapper, `may_drop_when_late`, and `untag`

**Files:**
- Modify: `refbox/src/app/message.rs`: the `Tap` variant; arms in the hand-written `impl PartialEq for Message` (around line 552) and in `is_repeatable` (around line 401); new `may_drop_when_late`.
- Modify: `refbox/src/app/mod.rs`: new `fn untag`, placed where `tap_outlived_its_screen` is now; a `Tap` arm in `handle_message`; new test module `untag_tests`.

**Interfaces:**
- Consumes: `ScreenKind`, `screen_kind` (Task 1).
- Produces:
  - `Message::Tap { screen: ScreenKind, message: Box<Message> }`
  - `impl Message { pub fn may_drop_when_late(&self) -> bool }`. `false` exactly for `AlarmReleased`, `TimeoutReviveReleased(_)` and every message that does not come from the screen (portal replies, `Recv*`, ticks, `TimeUpdaterStarted`, `AlarmDelayElapsed`, `TimeoutReviveHoldElapsed`, `SpacebarPressed`/`SpacebarReleased`, `Updates*Done`/`Downloaded`/`Verified`/`Installed`, `UpdaterHealthyCheck`, `AutoConfirmScores`, `PortalRefreshFinished`, `NoAction`, …). `Tap` delegates to its inner message. `true` for every other message.
  - `fn untag(message: Message, now_on: ScreenKind) -> Option<Message>`. An untagged message → `Some(message)`. `Tap` with `screen == now_on` → `Some(*inner)`. `Tap` with `screen != now_on` → `None` if `inner.may_drop_when_late()`, else `Some(*inner)`. Logging is the caller's (Task 3), so `untag` stays pure.
  - `is_repeatable`: `Tap` delegates to its inner message. `PartialEq`: two `Tap`s are equal when both screen and inner are equal.
  - `handle_message` `Tap` arm: `error!("A tagged tap reached handle_message: {message:?}")`, then handle the inner message. No panic.

- [ ] **Step 1: Write the failing tests** in `untag_tests`. Helper: `fn tap(on: &AppState, m: Message) -> Message`, which tags `m` with `screen_kind(on)`.
  - `late_keypad_taps_are_dropped`: for each of the Penalty, WarningAdd and FoulAdd keypads, and each of `PenaltyEditComplete { canceled: false, deleted: false }`, `ChangeKind(_)`, `ChangeInfraction(_)` and the matching Warning/Foul `…EditComplete`: tagged on the keypad, arriving on `ConfirmScores(scores)` → `None`.
  - `late_keypad_cancel_is_dropped`: the keypad's CANCEL (`…EditComplete { canceled: true, .. }`), arriving on `ConfirmScores` → `None`.
  - `late_time_change_is_dropped`: `ChangeTime { increase: true, secs: 1, timeout: false }` tagged on `TimeEdit(..)`, arriving on `MainPage` → `None`. The same message tagged on `TimeEdit`, arriving on `ParameterEditor(Half, …)` → `None`.
  - `late_digit_onto_another_keypad_is_dropped`: `KeypadButtonPress(_)` tagged on the Penalty keypad, arriving on the TeamTimeouts keypad → `None`.
  - `late_colour_tap_is_dropped`: a `ChangeColor(..)` value tagged on the Penalty keypad, arriving on `MainPage` → `None`.
  - `releases_are_never_dropped`: `AlarmReleased` and `TimeoutReviveReleased(GameColor::White)`, tagged on `MainPage`, arriving on `ConfirmScores` → `Some(` the bare message `)`.
  - `same_kind_tap_passes`: `KeypadButtonPress(_)` tagged on `KeypadPage(Penalty…, 7)`, arriving on `KeypadPage(Penalty…, 72)` → `Some(` bare `)`.
  - `untagged_messages_pass`: `NewSnapshot(_)`, `PortalUiTick` and `RecvSchedule(..)` with any `now_on` → `Some(` unchanged `)`.
  - `tagged_apply_comes_out_as_apply`: `TimeEditComplete { canceled: false }` tagged on `TimeEdit(..)`, arriving on `TimeEdit(..)` with different durations → `Some(TimeEditComplete { canceled: false })`, and `matches!(…, Message::TimeEditComplete { .. })` holds on the result.
  - `old_list_cases_still_dropped`: one assertion per case in today's `stale_tap_gate_tests` (mod.rs:11849-12000), rewritten as `untag(tap(old_screen, msg), screen_kind(new_screen))`. Each case keeps its outcome: dropped stays `None`, passed stays `Some`.
  - `tap_repeatability_follows_inner` and `tap_equality`: `Tap` delegates `is_repeatable`; two `Tap`s with the same screen and inner are `==`, and with different screens are `!=`.
- [ ] **Step 2: Run** `cargo test -p refbox --bin refbox untag_tests`. Expected: fails to compile.
- [ ] **Step 3: Implement** the variant, the three `Message` arms, `may_drop_when_late` (exhaustive, no `_`), `untag`, and the `handle_message` arm.
- [ ] **Step 4: Run** the `untag_tests`, `screen_kind_tests` and `stale_tap_gate_tests` filters. Expected: all PASS (the old list is still in place until Task 3). No clippy yet (dead code until Task 3).
- [ ] **Step 5: Commit** `fix(refbox): classify late taps by the screen they were drawn on`.

### Task 3: wire it in, and remove the old list

**Files:**
- Modify: `refbox/src/app/mod.rs`: `update()` (line 4964), the end of `view()` (the `container(main_view)` around line 9627), `handle_message` (remove the `tap_outlived_its_screen` check at line 5075), and delete `tap_outlived_its_screen` and `stale_tap_gate_tests`.

**Interfaces:**
- Consumes: `untag`, `screen_kind`, `Message::Tap` (Tasks 1-2).

- [ ] **Step 1: `view()`:** compute `let screen = screen_kind(&self.app_state);` and map the finished element with `.map(move |message| Message::Tap { screen, message: Box::new(message) })`. One map, on the outermost element, covering every screen.
- [ ] **Step 2: `update()`:** the first statement, before `time_edit_was_running`, does the following:
  - If the message is a `Tap`, log `debug!("Tap from {screen:?}: {inner:?}")`. Every tagged tap logs this line, dropped or not; it is the walkthrough's proof that the tagging is wired in.
  - Run `untag(message, screen_kind(&self.app_state))`. On `None`: `warn!("Ignoring {inner:?}: drawn on {screen:?}, now on {now:?}")` and `return Task::none()`, with `last_message` untouched. On `Some(m)`: carry on with `m` as `message`.
- [ ] **Step 3:** Delete the `tap_outlived_its_screen` call in `handle_message`, the function, its doc comment and `stale_tap_gate_tests`. Every case of theirs now lives in `old_list_cases_still_dropped` (Task 2).
- [ ] **Step 4: Run** `cargo test -p refbox --bin refbox untag_tests screen_kind_tests zero_gap_gate_tests` (one filter per run if cargo takes only one). Expected: PASS. Then `cargo clippy -p refbox -- -D warnings`. Expected: clean. Then `cargo build -p refbox`.
- [ ] **Step 5: Reviewer check:** in the diff, the unwrap comes before `closed_by_apply_or_cancel`; no `matches!(message, …)` or `message ==` check runs on the wrapped value; `rg 'Message::Tap'` shows only `view()`, `untag`, the three `Message` impls, the `handle_message` arm and tests.
- [ ] **Step 6: Commit** `fix(refbox): ignore taps that arrive after their screen has gone`.

### Task 4 (main session, tiny): TIME EDIT timeout +/- after the timeout ended

**Files:**
- Modify: `refbox/src/app/mod.rs`: the `ChangeTime` arm (around line 5108); new test in the `untag_tests` module, or its own small module.

**Interfaces:**
- Produces: `fn time_edit_target<'a>(game_dur: &'a mut Duration, timeout_dur: &'a mut Option<Duration>, timeout: bool) -> Option<(&'a mut Duration, bool)>`. Returns `Some((game_dur, true))` when `!timeout`, `Some((t, false))` when `timeout` and the value is `Some(t)`, and `None` when `timeout` and the value is `None`.

- [ ] **Step 1: Write the failing test** `timeout_change_after_timeout_ended_has_no_target`: `time_edit_target(&mut g, &mut None, true)` is `None`; `(&mut g, &mut Some(t), true)` gives `t` with `false`; `(&mut g, &mut None, false)` gives `g` with `true`.
- [ ] **Step 2: Run** it. Expected: fails to compile.
- [ ] **Step 3: Implement** the function, and use it in the `TimeEdit` arm of `ChangeTime`. On `None`: `warn!("Ignoring the timeout +/- on TIME EDIT: the timeout has already ended")` and `return Task::none()`.
- [ ] **Step 4: Run** the test, then clippy. Expected: PASS, clean.
- [ ] **Step 5: Commit** `fix(refbox): ignore a TIME EDIT timeout change after the timeout ended`.

### Task 5 (main session): whole-branch checks and handover to the PO

- [ ] Rebase on the latest `origin/master` (it moves fast); `cargo audit` (no GO needed).
- [ ] With a GO: `RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=4 just check`. Expected: green.
- [ ] Whole-branch review (Opus), then the built-in `code-review` skill on the finished diff, run from the worktree. Fix or answer every finding.
- [ ] PO walkthrough. Isolated launch: `XDG_CONFIG_HOME=<scratch>`, `UWH_PORTAL_URL_OVERRIDE=http://localhost:9 --allow-http`, `--log-location <scratch dir>`, `-v` (refbox sets its log level only from `-v`; it ignores `RUST_LOG`). Steps include ones that can fail:
  - Ordinary taps on several screens (keypad digits, TIME EDIT +/-, ALARM hold and release, the timeout revive hold) all still work.
  - After those taps, the log has a "Tap from …" line for each. With no such lines, the tagging is not wired in.
  - No "Ignoring … drawn on" line appears during ordinary use.
- [ ] Pre-PR disclosure to the PO (the three checks, plus the dangling-work sweep), then ask to push.

## Deviations

- **`ConfigPageKind` (Task 1).** The plan reused `ConfigPage` inside `ScreenKind::EditGameConfig`. It carries
  values (`Remotes(usize, bool)`, `CustomSite(bool)`) and the remotes flag can change by itself when a remote's
  id arrives, which would have made valid taps look late. A payload-free `ConfigPageKind` is used instead.
- **Walkthrough log level (Task 5).** The plan said `RUST_LOG=debug`; refbox ignores it and takes its level only
  from `-v`. Corrected above.
