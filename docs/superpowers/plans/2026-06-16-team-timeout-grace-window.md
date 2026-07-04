# Team-timeout 15-second grace window — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline, this session) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Give team timeouts a 15-second grace window — "Cancel Timeout" (orange) that undoes the timeout (resume clock + refund) for the first 15 s, then "End Timeout" (red) with team-switching disabled — and relabel the ref-timeout / penalty-shot buttons to "Cancel Ref Timeout" / "Cancel Pen Shot" (orange).

**Architecture:** Add one `tournament_manager` method (`cancel_team_timeout` = resume clock + refund) and one `Message::CancelTimeout`; leave `end_timeout` untouched (keeps golden traces stable). The UI computes "within 15 s" from the timeout's remaining seconds vs `team_timeout_duration` and chooses the label/colour/message; team-switch is gated on the same window. New cancel labels reuse existing words via Fluent references.

**Tech Stack:** Rust 2024, iced 0.13, Fluent (`fl!`). Reference design: `docs/superpowers/specs/2026-06-16-team-timeout-grace-window-design.md`.

**Worktree:** `/home/estraily/projects/uwh-refbox-rs/.claude/worktrees/feat+refbox+cancel-timeout-button/` (branch `feat/refbox/cancel-timeout-button`, off origin/master). All paths below are relative to it. `cd` into it before every `cargo`/`just`.

**Heavy-process notes:** Task 2 (state machine) is real TDD. UI is verified by compile + `just check` + walkthrough (no iced widget-label test harness). `cargo clippy -p refbox -- -D warnings` mirrors CI for this crate; `cargo test -p refbox`. A button built with no `.on_press(...)` renders greyed/disabled in iced 0.13.

---

## Task 1: Reset the worktree to pristine

The worktree currently holds the **superseded** rename edits (translations + the two view builders). Discard them and start the real feature from clean origin/master.

- [ ] **Step 1: Discard all uncommitted edits**

Run: `cd <worktree> && git restore refbox/`
Then: `git status --short`
Expected: empty (clean tree at 188b0c9f).

- [ ] **Step 2: Sanity-check the baseline labels are back**

Run: `grep -n "^end-timeout = \|^end-timeout-line-1 = " refbox/translations/en-US/refbox.ftl`
Expected: `end-timeout = END TIMEOUT` and `end-timeout-line-1 = END` (originals restored).

(No commit — nothing changed vs origin/master.)

---

## Task 2: `cancel_team_timeout` in the tournament manager (TDD)

**Files:**
- Test + impl: `refbox/src/tournament_manager/mod.rs`

- [ ] **Step 1: Write the failing test** (add inside the `#[cfg(test)] mod test`, e.g. right after `test_end_timeouts`)

```rust
    #[test]
    fn test_cancel_team_timeout_refunds_and_resumes() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 1,
            team_timeout_duration: Duration::from_secs(60),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();

        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(300));
        tm.start_clock(start);
        assert_eq!(tm.start_team_timeout(Color::Black, start), Ok(()));
        assert_eq!(tm.timeouts_used.black, 1);

        // Cancel within the grace window: clock resumes, the team is refunded.
        let cancel_at = start + Duration::from_secs(5);
        assert_eq!(tm.cancel_team_timeout(cancel_at), Ok(()));
        assert_eq!(tm.timeout_state, None);
        assert_eq!(tm.timeouts_used.black, 0);
        assert!(tm.clock_is_running());

        // Cancel with no timeout, or a non-team timeout, is an error.
        assert_eq!(tm.cancel_team_timeout(cancel_at), Err(TMErr::NotInTimeout));
    }
```

- [ ] **Step 2: Run it — verify it fails to compile (method missing)**

Run: `cd <worktree> && cargo test -p refbox cancel_team_timeout 2>&1 | tail -15`
Expected: compile error — no method `cancel_team_timeout`.

- [ ] **Step 3: Implement the method** (add immediately after `end_timeout`, after line ~600)

```rust
    /// Cancel a team timeout within its grace window: resume the game clock (if it
    /// was running) and refund the team the timeout that `start_team_timeout`
    /// charged. Mirrors `end_timeout`'s team branch plus the refund; only valid
    /// while a team timeout is active.
    pub fn cancel_team_timeout(&mut self, now: Instant) -> Result<()> {
        match &self.timeout_state {
            Some(TimeoutState::Team(color, cs)) => {
                let color = *color;
                info!("{} Cancelling {color} team timeout", self.status_string(now));
                match cs {
                    ClockState::Stopped { .. } => self.timeout_state = None,
                    ClockState::CountingDown { .. } => {
                        self.start_game_clock(now);
                        self.timeout_state = None;
                    }
                    ClockState::CountingUp { .. } => {
                        error!("Invalid timeout state");
                        return Err(TournamentManagerError::InvalidState);
                    }
                }
                self.timeouts_used[color] = self.timeouts_used[color].saturating_sub(1);
                Ok(())
            }
            _ => Err(TournamentManagerError::NotInTimeout),
        }
    }
```

- [ ] **Step 4: Run the test — verify it passes**

Run: `cd <worktree> && cargo test -p refbox cancel_team_timeout 2>&1 | tail -15`
Expected: `test ... test_cancel_team_timeout_refunds_and_resumes ... ok`.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "feat(refbox): add cancel_team_timeout (resume clock + refund)"
```

---

## Task 3: `Message::CancelTimeout` + update handler

**Files:**
- `refbox/src/app/message.rs` (variant + `is_repeatable` + `PartialEq`)
- `refbox/src/app/mod.rs` (handler)

- [ ] **Step 1: Add the variant** — in `message.rs`, immediately after `EndTimeout,` (line ~140):

```rust
    EndTimeout,
    CancelTimeout,
```

- [ ] **Step 2: `is_repeatable`** — in the big match (line ~327), add after `| Self::EndTimeout`:

```rust
            | Self::EndTimeout
            | Self::CancelTimeout
```

- [ ] **Step 3: `PartialEq` — equal arm** (line ~398), add after `| (Self::EndTimeout, Self::EndTimeout)`:

```rust
            | (Self::EndTimeout, Self::EndTimeout)
            | (Self::CancelTimeout, Self::CancelTimeout)
```

- [ ] **Step 4: `PartialEq` — fall-through arm** (line ~630), add after `| (Self::EndTimeout, _)`:

```rust
            | (Self::EndTimeout, _)
            | (Self::CancelTimeout, _)
```

- [ ] **Step 5: Handler** — in `mod.rs`, immediately after the `Message::EndTimeout => { ... }` arm (ends ~line 3470), add:

```rust
            Message::CancelTimeout => {
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                // The Cancel button is only rendered for an active team timeout
                // inside its grace window, so this call is valid by construction.
                tm.cancel_team_timeout(now).unwrap();
                tm.update(now).unwrap();
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot)
            }
```

- [ ] **Step 6: Build**

Run: `cd <worktree> && cargo build -p refbox 2>&1 | tail -15`
Expected: builds clean (no non-exhaustive-match errors — if any other match over `Message` is non-exhaustive, add a `CancelTimeout` arm mirroring `EndTimeout`).

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add Message::CancelTimeout and its update handler"
```

---

## Task 4: Translations — add the cancel labels (15 locales, reference-based)

The new labels reuse existing localized words via Fluent message references, so the **same 9 lines** go into every locale file — no per-language translation, no English placeholders.

**Files:** all 15 `refbox/translations/<locale>/refbox.ftl`.

- [ ] **Step 1: Confirm the referenced keys exist in every locale**

Run: `cd <worktree> && for k in '^cancel = ' '^timeout = ' '^ref = ' '^pen-shot = '; do echo "== $k =="; grep -rlE "$k" refbox/translations/*/refbox.ftl | wc -l; done`
Expected: each prints `15`. (If any is <15, that locale is missing a referenced word — stop and report.)

- [ ] **Step 2: Add the 9 keys to each locale**, in the `## Timeout ribbon` section (right after `end-timeout-line-2`). Identical text in all 15 files:

```
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
```

Use a small verified script (mirror `/tmp/rename_locales.py`'s pattern: for each locale, assert the anchor line `end-timeout-line-2 = { timeout }\n` occurs once, insert the block after it, write back). Do **not** hand-edit 15 files.

- [ ] **Step 3: Verify all 15 locales got all 9 keys**

Run: `cd <worktree> && for k in cancel-timeout cancel-timeout-line-1 cancel-timeout-line-2 cancel-ref-timeout cancel-ref-timeout-line-1 cancel-ref-timeout-line-2 cancel-pen-shot cancel-pen-shot-line-1 cancel-pen-shot-line-2; do n=$(grep -rhE "^$k = " refbox/translations/*/refbox.ftl | wc -l); echo "$k: $n"; done`
Expected: every key prints `15`.

- [ ] **Step 4: Build (Fluent keys are validated at compile)**

Run: `cd <worktree> && cargo build -p refbox 2>&1 | tail -8`
Expected: builds clean.

- [ ] **Step 5: Commit**

```bash
git add refbox/translations
git commit -m "feat(refbox): add Cancel Timeout / Ref Timeout / Pen Shot labels (all locales)"
```

---

## Task 5: Timeout ribbon — grace flip, switch-gate, ref/penalty relabel

**Files:** `refbox/src/app/view_builders/shared_elements.rs` (`build_timeout_ribbon`, ~lines 179-293).

- [ ] **Step 1: Add a grace const + helper** near the top of the file (after the imports/macros):

```rust
/// Team timeouts can be cancelled (undone) for this long after they start.
pub(in super::super) const TIMEOUT_GRACE_SECS: u16 = 15;

/// True while a team timeout is still inside its cancel/grace window.
/// `remaining` is the timeout's remaining seconds (from the snapshot);
/// `team_timeout_duration` is the configured full length.
pub(in super::super) fn team_timeout_in_grace(
    team_timeout_duration: Duration,
    remaining: u16,
) -> bool {
    (team_timeout_duration.as_secs() as u16).saturating_sub(remaining) < TIMEOUT_GRACE_SECS
}
```

- [ ] **Step 2: Replace the `black` slot match** (currently ~186-210) with grace-aware logic:

```rust
    let team_to_dur = tm.config().team_timeout_duration;

    let black = match snapshot.timeout {
        None => make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_team_timeout(GameColor::Black)
                    .ok()
                    .map(|_| Message::TeamTimeout(GameColor::Black, false)),
            )
            .style(black_button),
        Some(TimeoutSnapshot::Black(remaining)) => {
            if team_timeout_in_grace(team_to_dur, remaining) {
                make_multi_label_button((fl!("cancel-timeout-line-1"), fl!("cancel-timeout-line-2")))
                    .on_press(Message::CancelTimeout)
                    .style(orange_button)
            } else {
                make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                    .on_press(Message::EndTimeout)
                    .style(red_button)
            }
        }
        Some(TimeoutSnapshot::White(other_remaining)) => {
            if team_timeout_in_grace(team_to_dur, other_remaining)
                && tm.can_switch_to_team_timeout(GameColor::Black).is_ok()
            {
                make_multi_label_button((fl!("switch-to"), fl!("dark-team-name-caps")))
                    .on_press(Message::TeamTimeout(GameColor::Black, true))
                    .style(black_button)
            } else {
                make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                    .style(black_button)
            }
        }
        Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                .style(black_button)
        }
    };
```

- [ ] **Step 3: Replace the `white` slot match** (mirror of black):

```rust
    let white = match snapshot.timeout {
        None => make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_team_timeout(GameColor::White)
                    .ok()
                    .map(|_| Message::TeamTimeout(GameColor::White, false)),
            )
            .style(white_button),
        Some(TimeoutSnapshot::White(remaining)) => {
            if team_timeout_in_grace(team_to_dur, remaining) {
                make_multi_label_button((fl!("cancel-timeout-line-1"), fl!("cancel-timeout-line-2")))
                    .on_press(Message::CancelTimeout)
                    .style(orange_button)
            } else {
                make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                    .on_press(Message::EndTimeout)
                    .style(red_button)
            }
        }
        Some(TimeoutSnapshot::Black(other_remaining)) => {
            if team_timeout_in_grace(team_to_dur, other_remaining)
                && tm.can_switch_to_team_timeout(GameColor::White).is_ok()
            {
                make_multi_label_button((fl!("switch-to"), fl!("light-team-name-caps")))
                    .on_press(Message::TeamTimeout(GameColor::White, true))
                    .style(white_button)
            } else {
                make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                    .style(white_button)
            }
        }
        Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                .style(white_button)
        }
    };
```

- [ ] **Step 4: Replace the `referee` slot match** — active ref timeout = orange "Cancel Ref Timeout" + EndTimeout; the non-active arm keeps the honest-label (switch-or-disabled) behaviour:

```rust
    let referee = match snapshot.timeout {
        None => make_multi_label_button((fl!("ref-timeout-line-1"), fl!("ref-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_ref_timeout()
                    .ok()
                    .map(|_| Message::RefTimeout(false)),
            )
            .style(yellow_button),
        Some(TimeoutSnapshot::Ref(_)) => {
            make_multi_label_button((fl!("cancel-ref-timeout-line-1"), fl!("cancel-ref-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(orange_button)
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => match tm.can_switch_to_ref_timeout() {
            Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("ref")))
                .on_press(Message::RefTimeout(true))
                .style(yellow_button),
            Err(_) => {
                make_multi_label_button((fl!("ref-timeout-line-1"), fl!("ref-timeout-line-2")))
                    .style(yellow_button)
            }
        },
    };
```

- [ ] **Step 5: Replace the `penalty` slot match** — active penalty shot = orange "Cancel Pen Shot" + EndTimeout; non-active keeps honest-label:

```rust
    let penalty = match snapshot.timeout {
        None => make_multi_label_button((fl!("penalty-shot-line-1"), fl!("penalty-shot-line-2")))
            .on_press_maybe(
                tm.can_start_penalty_shot()
                    .ok()
                    .map(|_| Message::PenaltyShot(false)),
            )
            .style(red_button),
        Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("cancel-pen-shot-line-1"), fl!("cancel-pen-shot-line-2")))
                .on_press(Message::EndTimeout)
                .style(orange_button)
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_)) => {
            let can_switch = if mode == Mode::Rugby {
                tm.can_switch_to_rugby_penalty_shot()
            } else {
                tm.can_switch_to_penalty_shot()
            };
            match can_switch {
                Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("pen-shot")))
                    .on_press(Message::PenaltyShot(true))
                    .style(red_button),
                Err(_) => make_multi_label_button((
                    fl!("penalty-shot-line-1"),
                    fl!("penalty-shot-line-2"),
                ))
                .style(red_button),
            }
        }
    };
```

- [ ] **Step 6: Build + lint**

Run: `cd <worktree> && cargo build -p refbox && cargo clippy -p refbox -- -D warnings 2>&1 | tail -15`
Expected: clean, zero warnings.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): team-timeout grace flip + switch-gate, ref/pen cancel relabel"
```

---

## Task 6: Center single-line button (main_view)

**Files:** `refbox/src/app/view_builders/main_view.rs` (the `if snapshot.timeout.is_some() { ... else { ... } }` block, ~lines 82-91).

The center cancel button (shown only when "track fouls & warnings" is off) must pick label/colour/message by timeout type, mirroring the ribbon.

- [ ] **Step 1: Replace the `else` branch** that currently pushes a single `end-timeout` button:

```rust
        } else {
            let cancel_btn = match snapshot.timeout {
                Some(TimeoutSnapshot::Black(remaining)) | Some(TimeoutSnapshot::White(remaining)) => {
                    if team_timeout_in_grace(game_config.team_timeout_duration, remaining) {
                        make_button(fl!("cancel-timeout"))
                            .style(orange_button)
                            .on_press(Message::CancelTimeout)
                    } else {
                        make_button(fl!("end-timeout"))
                            .style(red_button)
                            .on_press(Message::EndTimeout)
                    }
                }
                Some(TimeoutSnapshot::Ref(_)) => make_button(fl!("cancel-ref-timeout"))
                    .style(orange_button)
                    .on_press(Message::EndTimeout),
                Some(TimeoutSnapshot::PenaltyShot(_)) => make_button(fl!("cancel-pen-shot"))
                    .style(orange_button)
                    .on_press(Message::EndTimeout),
                // Unreachable: this block is guarded by `snapshot.timeout.is_some()`.
                None => make_button(fl!("end-timeout"))
                    .style(red_button)
                    .on_press(Message::EndTimeout),
            };
            center_col = center_col.push(cancel_btn);
        }
```

(If `TimeoutSnapshot` or `team_timeout_in_grace` is not already in scope in `main_view.rs`, add `use` lines: `team_timeout_in_grace` comes from the shared module — confirm via the existing `use super::*;`; `TimeoutSnapshot` is from `uwh_common::game_snapshot`.)

- [ ] **Step 2: Build + lint**

Run: `cd <worktree> && cargo build -p refbox && cargo clippy -p refbox -- -D warnings 2>&1 | tail -15`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/main_view.rs
git commit -m "feat(refbox): center cancel button mirrors per-type label/colour"
```

---

## Task 7: Full validation + golden-trace check + walkthrough + PR

- [ ] **Step 1: Full gate**

Run: `cd <worktree> && just check 2>&1 | tail -40`
Expected: fmt, clippy, tests, audit all clean. **The golden-trace tests must pass with no baseline changes** — `end_timeout` is untouched and the snapshot/clock behaviour of cancel matches end, so no existing `.trace` file should change.

- [ ] **Step 2: Confirm no golden trace files were modified**

Run: `cd <worktree> && git status --short refbox/src/tournament_manager/golden_traces/`
Expected: empty. (If any `.trace` changed, stop — the state-machine behaviour drifted unexpectedly; investigate before continuing.)

- [ ] **Step 3: Manual walkthrough** — launch the built binary in the background (`WAYLAND_DISPLAY=` to force X11, `dangerouslyDisableSandbox`). Human drives; confirm the design's "How to verify":
  1. Team timeout: orange "Cancel Timeout" for ~15 s; Cancel resumes the clock and the team still has its timeout (start another to confirm). Other team shows "Switch to …" enabled within the window.
  2. Team timeout past 15 s: red "End Timeout"; other team shows "<other> Timeout" greyed; End uses up the timeout.
  3. Ref timeout: orange "Cancel Ref Timeout" throughout. Penalty shot: orange "Cancel Pen Shot" throughout.
  4. "Track fouls & warnings" off: the center button shows the same text/colour per phase/type.

- [ ] **Step 4: Code review + PR** — run `superpowers:requesting-code-review`, then prepare the PR per `.claude/rules/pr-review.md` (plain-language What/Why/Scope/How-to-verify). Get the human's approval before pushing/opening.

---

## Deviations / notes

- Penalty two-line label is **"CANCEL"/"PEN SHOT"** (reuses the existing `pen-shot` word) rather than the "CANCEL PEN"/"SHOT" split discussed — confirm this is acceptable.
- `Message::CancelTimeout` uses `.unwrap()` in its handler, matching the existing `EndTimeout` handler; justified by a comment (the button only renders when the call is valid).
