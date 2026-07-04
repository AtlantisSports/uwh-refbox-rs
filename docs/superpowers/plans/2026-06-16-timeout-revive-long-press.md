# Timeout Revive via Long-Press — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the operator press-and-hold a used-up (greyed) team timeout button for 5 seconds to give that team back one timeout, with a 2-second safety lockout afterward.

**Architecture:** A small new method on `TournamentManager` lowers a team's used-timeout count (the only state-machine change). The app reuses the manual-alarm button's press-and-hold machinery (async timer + token cancellation) for the 5-second hold and a 2-second post-revive lockout. The greyed timeout button is wrapped in a `mouse_area` (like the alarm face) so it can capture the hold even though it is non-interactive; it brightens while held via an "armed" style that forces the active colour.

**Tech Stack:** Rust 2024, iced 0.13, tokio async timers. All changes are in the `refbox` crate.

**Design spec:** `docs/superpowers/specs/2026-06-16-timeout-revive-long-press-design.md`

**Branch:** `feat/refbox/timeout-revive-long-press` (create with the human's approval before Task 1).

---

## Scope / blast radius

- **`refbox` crate only.** No `uwh-common`, no wire format, no LED panel / overlay / wireless remote. The used-count is internal to the refbox.
- **Heavy-process care for Task 1** (state machine). Lean process for the app/view wiring in Task 2 (UI behaviour, verified by compile + manual walkthrough).
- No new config setting. No new translation keys (the button keeps its existing label, just brighter).

## File structure (what each touched file is responsible for)

| File | Responsibility / change |
|------|-------------------------|
| `refbox/src/tournament_manager/mod.rs` | New `can_revive_team_timeout` + `revive_team_timeout` methods, new `NoTimeoutToRevive` error variant, unit tests. |
| `refbox/src/app/theme/button.rs` | New `black_button_armed` / `white_button_armed` styles (force active colour). |
| `refbox/src/app/theme/mod.rs` | Re-export the two new styles. |
| `refbox/src/app/message.rs` | Four new `Message` variants + update the manual `is_repeatable` and `PartialEq` matches. |
| `refbox/src/app/mod.rs` | Two duration constants, four state fields + their init, four message handlers, and the `build_timeout_ribbon` call-site update. |
| `refbox/src/app/view_builders/shared_elements.rs` | `build_timeout_ribbon` gains two params; the black/white "no timeout running" arms get the hold-to-revive / lockout rendering; import `mouse_area`. |

> **Note on commit granularity:** Task 1 compiles and tests pass on its own. Task 2's pieces are interdependent — new `Message` variants make `update()` non-exhaustive, and new `pub` style functions are "never used" until the view calls them, both of which fail `-D warnings`. So Task 2 lands as **one commit** after all its steps; compile/lint only at the end of the task.

---

## Task 0: Create the branch (requires human approval)

- [ ] **Step 1: Confirm with the human, then create the branch**

The human must approve branch creation (project rule). Once approved:

```bash
git fetch origin master
git switch -c feat/refbox/timeout-revive-long-press origin/master
```

Expected: on a new branch based on the latest `origin/master`.

---

## Task 1: State-machine revive method (heavy care) — `tournament_manager/mod.rs`

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` (error enum ~line 2293; new methods after `switch_to_team_timeout` ~line 456; tests in the `mod test` block ~line 2367+)

- [ ] **Step 1: Write the failing tests**

Add these three tests inside the `#[cfg(test)] mod test { ... }` block (e.g. just after `test_switch_timeouts`). The test module already has `initialize()`, `GameConfig`, `TournamentManager::new`, `set_period_and_game_clock_time`, `set_timeout_state`, direct `tm.timeouts_used.black/.white` access, and the alias `TMErr` for `TournamentManagerError` (all used by existing timeout tests).

```rust
    #[test]
    fn test_revive_team_timeout() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 1,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));

        // Nothing used yet: button is enabled, so there is nothing to revive.
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(
            tm.can_revive_team_timeout(Color::Black),
            Err(TMErr::NoTimeoutToRevive(Color::Black))
        );
        assert_eq!(
            tm.revive_team_timeout(Color::Black),
            Err(TMErr::NoTimeoutToRevive(Color::Black))
        );
        assert_eq!(tm.timeouts_used.black, 0);

        // Use Black's timeout: button greys (TooManyTeamTimeouts) and revive applies.
        tm.timeouts_used.black = 1;
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TMErr::TooManyTeamTimeouts(Color::Black))
        );
        assert_eq!(tm.can_revive_team_timeout(Color::Black), Ok(()));

        // Revive gives one back; the button is enabled again.
        assert_eq!(tm.revive_team_timeout(Color::Black), Ok(()));
        assert_eq!(tm.timeouts_used.black, 0);
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));

        // White is untouched.
        assert_eq!(tm.timeouts_used.white, 0);
    }

    #[test]
    fn test_revive_team_timeout_guards() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 1,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);

        // Used up, but in a non-play period -> no revive (button greys for the
        // period, not the count, so giving one back would not re-enable it).
        tm.set_period_and_game_clock_time(GamePeriod::OvertimeFirstHalf, Duration::from_secs(10));
        tm.timeouts_used.black = 1;
        assert_eq!(
            tm.can_revive_team_timeout(Color::Black),
            Err(TMErr::NoTimeoutToRevive(Color::Black))
        );

        // Used up and in a half, but a timeout is currently running -> no revive.
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.set_timeout_state(Some(TimeoutState::Ref(ClockState::Stopped {
            clock_time: Duration::from_secs(0),
        })));
        assert_eq!(
            tm.can_revive_team_timeout(Color::Black),
            Err(TMErr::NoTimeoutToRevive(Color::Black))
        );

        // No timeout running, in a half, used up -> revive applies.
        tm.set_timeout_state(None);
        assert_eq!(tm.can_revive_team_timeout(Color::Black), Ok(()));
    }

    #[test]
    fn test_revive_team_timeout_respects_cap() {
        initialize();
        let config = GameConfig {
            num_team_timeouts_allowed: 2,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));

        // One of two used: button still enabled, so no revive is offered.
        tm.timeouts_used.black = 1;
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
        assert_eq!(
            tm.can_revive_team_timeout(Color::Black),
            Err(TMErr::NoTimeoutToRevive(Color::Black))
        );

        // Both used: button greys; revive gives back exactly one.
        tm.timeouts_used.black = 2;
        assert_eq!(
            tm.can_start_team_timeout(Color::Black),
            Err(TMErr::TooManyTeamTimeouts(Color::Black))
        );
        assert_eq!(tm.revive_team_timeout(Color::Black), Ok(()));
        assert_eq!(tm.timeouts_used.black, 1);
        assert_eq!(tm.can_start_team_timeout(Color::Black), Ok(()));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox revive_team_timeout`
Expected: FAIL — compile error, `no method named can_revive_team_timeout` / `no variant NoTimeoutToRevive`.

- [ ] **Step 3: Add the error variant**

In `enum TournamentManagerError` (around line 2298, right after the `TooManyTeamTimeouts` variant), add:

```rust
    #[error("The {0} team has no timeout to revive")]
    NoTimeoutToRevive(Color),
```

- [ ] **Step 4: Add the two methods**

Insert immediately after `switch_to_team_timeout` (after its closing brace around line 456):

```rust
    /// Returns `Ok` if a used team timeout can be given back to `color`.
    ///
    /// Only true when the team's timeout button is greyed *specifically because
    /// the team has used its allowed timeout(s)* during a half — i.e. giving one
    /// back would actually make a team timeout startable again. Returns `Err`
    /// when a timeout is running, when not in a half, or when nothing is used.
    pub fn can_revive_team_timeout(&self, color: Color) -> Result<()> {
        if self.timeout_state.is_some() {
            return Err(TournamentManagerError::NoTimeoutToRevive(color));
        }
        match self.current_period {
            GamePeriod::FirstHalf | GamePeriod::SecondHalf
                if self.timeouts_used[color] > 0
                    && self.timeouts_used[color] >= self.config.num_team_timeouts_allowed =>
            {
                Ok(())
            }
            _ => Err(TournamentManagerError::NoTimeoutToRevive(color)),
        }
    }

    /// Give one used team timeout back to `color` (lowers the used count by one,
    /// never below zero). Touches only the used-count — not the clock, period, or
    /// any active timeout. Errors if reviving does not apply (see
    /// `can_revive_team_timeout`).
    pub fn revive_team_timeout(&mut self, color: Color) -> Result<()> {
        self.can_revive_team_timeout(color)?;
        info!("Reviving a {color} team timeout");
        self.timeouts_used[color] = self.timeouts_used[color].saturating_sub(1);
        Ok(())
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p refbox revive_team_timeout`
Expected: PASS (3 tests).

- [ ] **Step 6: Run the full tournament_manager test suite (state-machine safety)**

Run: `cargo test -p refbox tournament_manager`
Expected: PASS — no existing timeout/reset tests regressed.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "feat(refbox): add revive_team_timeout to give back a used team timeout"
```

---

## Task 2: App, view, theme, and message wiring (one commit) — `refbox/src/app/`

This task lands as a single commit because its parts are interdependent (see the granularity note above). Do all steps, then compile + lint + commit at the end.

**Files:**
- Modify: `refbox/src/app/theme/button.rs` (after `white_button` ~line 107 and `black_button` ~line 134)
- Modify: `refbox/src/app/theme/mod.rs` (re-export list ~line 350)
- Modify: `refbox/src/app/message.rs` (variants ~line 233; `is_repeatable` ~line 341; `PartialEq` arms ~line 398 and ~line 625)
- Modify: `refbox/src/app/mod.rs` (consts ~line 70; fields ~line 131; init ~line 1330; handlers after ~line 3224; call site ~line 3928)
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (imports ~line 8; `build_timeout_ribbon` ~line 179)

- [ ] **Step 1: Add the two "armed" button styles**

In `refbox/src/app/theme/button.rs`, add after `white_button` (after its closing brace ~line 107):

```rust
/// Like `white_button`, but always renders in the bright "active" colour even
/// when the button is non-interactive (no `on_press`). Used to brighten a
/// timeout button while it is held for the timeout-revive long-press, where the
/// button is wrapped in a `mouse_area` and has no `on_press` of its own.
pub fn white_button_armed(theme: &Theme, _status: Status) -> Style {
    white_button(theme, Status::Active)
}
```

And add after `black_button` (after its closing brace ~line 134):

```rust
/// Like `black_button`, but always renders in the bright "active" colour even
/// when the button is non-interactive (no `on_press`). Used to brighten a
/// timeout button while it is held for the timeout-revive long-press, where the
/// button is wrapped in a `mouse_area` and has no `on_press` of its own.
pub fn black_button_armed(theme: &Theme, _status: Status) -> Style {
    black_button(theme, Status::Active)
}
```

- [ ] **Step 2: Re-export the two new styles**

In `refbox/src/app/theme/mod.rs`, edit the `pub use button::{ ... }` list (lines 350–356) to add `black_button_armed` and `white_button_armed`:

```rust
pub use button::{
    black_button, black_button_armed, black_selected_button, blue_button, blue_selected_button,
    blue_with_border_button, gray_button, green_button, green_selected_button, light_gray_button,
    light_gray_selected_button, orange_button, orange_selected_button, red_button,
    red_selected_button, white_button, white_button_armed, white_selected_button, yellow_button,
    yellow_selected_button,
};
```

- [ ] **Step 3: Add the four `Message` variants**

In `refbox/src/app/message.rs`, after `AlarmDelayElapsed(u64),` (line 233) add:

```rust
    /// Press-down on a used-up (greyed) team timeout button — begins the
    /// 5-second hold-to-revive.
    TimeoutRevivePressed(GameColor),
    /// Release of a hold-to-revive press before it completed.
    TimeoutReviveReleased(GameColor),
    /// The 5-second revive hold elapsed for the given team (token guards stale timers).
    TimeoutReviveHoldElapsed(u64, GameColor),
    /// The 2-second post-revive safety lockout elapsed for the given team.
    TimeoutReviveLockoutElapsed(u64, GameColor),
```

- [ ] **Step 4: Mark the new variants non-repeatable**

In `is_repeatable`, change the final `=> false` group (line 341 is `| Self::AlarmDelayElapsed(_) => false,`) to:

```rust
            | Self::AlarmDelayElapsed(_)
            | Self::TimeoutRevivePressed(_)
            | Self::TimeoutReviveReleased(_)
            | Self::TimeoutReviveHoldElapsed(_, _)
            | Self::TimeoutReviveLockoutElapsed(_, _) => false,
```

- [ ] **Step 5: Add the `PartialEq` comparison arms**

In the manual `PartialEq`, after the `AlarmDelayElapsed` arm (line 398: `(Self::AlarmDelayElapsed(a), Self::AlarmDelayElapsed(b)) => a == b,`) add:

```rust
            (Self::TimeoutRevivePressed(a), Self::TimeoutRevivePressed(b)) => a == b,
            (Self::TimeoutReviveReleased(a), Self::TimeoutReviveReleased(b)) => a == b,
            (Self::TimeoutReviveHoldElapsed(a, b), Self::TimeoutReviveHoldElapsed(c, d)) => {
                a == c && b == d
            }
            (Self::TimeoutReviveLockoutElapsed(a, b), Self::TimeoutReviveLockoutElapsed(c, d)) => {
                a == c && b == d
            }
```

- [ ] **Step 6: Add the `PartialEq` catch-all arms**

In the final catch-all group (line 625 is `| (Self::AlarmDelayElapsed(_), _)`), insert the four new variants into the `| (..., _)` chain that ends in `=> false,`:

```rust
            | (Self::AlarmDelayElapsed(_), _)
            | (Self::TimeoutRevivePressed(_), _)
            | (Self::TimeoutReviveReleased(_), _)
            | (Self::TimeoutReviveHoldElapsed(_, _), _)
            | (Self::TimeoutReviveLockoutElapsed(_, _), _)
            | (Self::TimeUpdaterStarted(_), _)
            | (Self::NoAction, _) => false,
```

- [ ] **Step 7: Add the two duration constants**

In `refbox/src/app/mod.rs`, after `const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);` (line 70) add:

```rust
/// How long the operator must hold a used-up team timeout button to revive
/// (give back) one team timeout. Deliberately long to guard against accidents.
const TIMEOUT_REVIVE_HOLD_DURATION: Duration = Duration::from_secs(5);
/// After a revive, the just-revived team's button ignores all input for this
/// long, so a lingering hold-press cannot immediately start a timeout.
const TIMEOUT_REVIVE_LOCKOUT_DURATION: Duration = Duration::from_secs(2);
```

- [ ] **Step 8: Add the four state fields**

In the app struct, after `alarm_delay_token: u64,` (line 131) add:

```rust
    /// Which team's used-up timeout button is currently held for the 5-second
    /// revive long-press (`None` = not holding).
    timeout_revive_held: Option<Color>,
    /// Bumped on each revive press to cancel stale hold timers.
    timeout_revive_token: u64,
    /// Which team is in the post-revive safety lockout (button inert for 2s).
    timeout_revive_lockout: Option<Color>,
    /// Bumped on each lockout start to cancel stale lockout timers.
    timeout_revive_lockout_token: u64,
```

- [ ] **Step 9: Initialise the four fields**

In the struct construction, after `alarm_delay_token: 0,` (line 1330) add:

```rust
            timeout_revive_held: None,
            timeout_revive_token: 0,
            timeout_revive_lockout: None,
            timeout_revive_lockout_token: 0,
```

- [ ] **Step 10: Add the four message handlers**

In `update()`, after the `Message::AlarmDelayElapsed(token) => { ... }` arm (closes ~line 3224) add:

```rust
            Message::TimeoutRevivePressed(color) => {
                // Press-down on a used-up (greyed) team timeout button: begin the
                // hold-to-revive timer. The view only attaches this on an eligible button.
                if self.timeout_revive_held == Some(color) {
                    return Task::none();
                }
                self.timeout_revive_held = Some(color);
                self.timeout_revive_token += 1;
                let token = self.timeout_revive_token;
                info!("Timeout-revive hold started for {color}, token={token}");
                Task::future(async move {
                    sleep(TIMEOUT_REVIVE_HOLD_DURATION).await;
                    Message::TimeoutReviveHoldElapsed(token, color)
                })
            }
            Message::TimeoutReviveReleased(color) => {
                // Released before the hold completed: cancel. The bumped token makes
                // any still-pending hold timer a no-op when it fires.
                if self.timeout_revive_held == Some(color) {
                    self.timeout_revive_held = None;
                    info!("Timeout-revive hold released for {color}");
                }
                Task::none()
            }
            Message::TimeoutReviveHoldElapsed(token, color) => {
                // The 5-second hold elapsed. Only revive if this is the current hold
                // and the button is still held; re-validate against current state.
                if token != self.timeout_revive_token
                    || self.timeout_revive_held != Some(color)
                {
                    return Task::none();
                }
                self.timeout_revive_held = None;
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                if tm.revive_team_timeout(color).is_err() {
                    // State moved on during the hold (e.g. half ended); nothing to do.
                    std::mem::drop(tm);
                    return Task::none();
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                let apply_task = self.apply_snapshot(snapshot);
                // Start the 2-second safety lockout on this team's button.
                self.timeout_revive_lockout = Some(color);
                self.timeout_revive_lockout_token += 1;
                let lockout_token = self.timeout_revive_lockout_token;
                info!("Timeout revived for {color}; safety lockout started, token={lockout_token}");
                let lockout_task = Task::future(async move {
                    sleep(TIMEOUT_REVIVE_LOCKOUT_DURATION).await;
                    Message::TimeoutReviveLockoutElapsed(lockout_token, color)
                });
                Task::batch(vec![apply_task, lockout_task])
            }
            Message::TimeoutReviveLockoutElapsed(token, color) => {
                // Safety period over: let the button become pressable again.
                if token == self.timeout_revive_lockout_token
                    && self.timeout_revive_lockout == Some(color)
                {
                    self.timeout_revive_lockout = None;
                    info!("Timeout-revive safety lockout ended for {color}");
                }
                Task::none()
            }
```

- [ ] **Step 11: Pass the new state into the ribbon builder**

At the `build_timeout_ribbon(...)` call (line 3928) add the two new arguments:

```rust
                main_view = main_view.push(build_timeout_ribbon(
                    &self.snapshot,
                    &self.tm,
                    self.config.mode,
                    self.timeout_revive_held,
                    self.timeout_revive_lockout,
                ));
```

- [ ] **Step 12: Import `mouse_area` in shared_elements**

In `refbox/src/app/view_builders/shared_elements.rs`, add `mouse_area` to the `widget::{ ... }` import (the line listing `horizontal_space, image, svg, ...` ~line 10):

```rust
        Button, Container, Image, Row, Space, Text, button, container,
        container::Style as ContainerStyle, horizontal_space, image, mouse_area, svg, svg::Svg,
        text, text::Style as TextStyle, vertical_space,
```

- [ ] **Step 13: Extend the `build_timeout_ribbon` signature**

Change the signature (lines 179–183) to:

```rust
pub(in super::super) fn build_timeout_ribbon<'a>(
    snapshot: &GameSnapshot,
    tm: &Arc<Mutex<TournamentManager>>,
    mode: Mode,
    revive_held: Option<GameColor>,
    revive_lockout: Option<GameColor>,
) -> Row<'a, Message> {
```

- [ ] **Step 14: Rewrite the `black` button match to support hold-to-revive**

Replace the whole `let black = match snapshot.timeout { ... };` block (lines 186–210) with the following. Note every arm now ends in `.into()` and the binding is typed `Element<'a, Message>`:

```rust
    let black: Element<'a, Message> = match snapshot.timeout {
        None => {
            if revive_lockout == Some(GameColor::Black) {
                // Safety period right after a revive: active-looking but inert, so a
                // lingering hold-press cannot immediately start a timeout.
                make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                    .on_press(Message::NoAction)
                    .style(black_button)
                    .into()
            } else if tm.can_revive_team_timeout(GameColor::Black).is_ok() {
                // Used-up: the greyed button is hold-to-revive. A non-interactive
                // button wrapped in a mouse_area captures the press/hold; while held
                // it brightens via the "armed" style (forces the active colour).
                let held = revive_held == Some(GameColor::Black);
                let face = if held {
                    make_multi_label_button((
                        fl!("dark-timeout-line-1"),
                        fl!("dark-timeout-line-2"),
                    ))
                    .style(black_button_armed)
                } else {
                    make_multi_label_button((
                        fl!("dark-timeout-line-1"),
                        fl!("dark-timeout-line-2"),
                    ))
                    .style(black_button)
                };
                mouse_area(face)
                    .on_press(Message::TimeoutRevivePressed(GameColor::Black))
                    .on_release(Message::TimeoutReviveReleased(GameColor::Black))
                    .into()
            } else {
                make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                    .on_press_maybe(
                        tm.can_start_team_timeout(GameColor::Black)
                            .ok()
                            .map(|_| Message::TeamTimeout(GameColor::Black, false)),
                    )
                    .style(black_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::Black(_)) => {
            make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(yellow_button)
                .into()
        }
        Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("switch-to"), fl!("dark-team-name-caps")))
                .on_press_maybe(
                    tm.can_switch_to_team_timeout(GameColor::Black)
                        .ok()
                        .map(|_| Message::TeamTimeout(GameColor::Black, true)),
                )
                .style(black_button)
                .into()
        }
    };
```

- [ ] **Step 15: Rewrite the `white` button match the same way**

Replace the whole `let white = match snapshot.timeout { ... };` block (lines 212–236) with:

```rust
    let white: Element<'a, Message> = match snapshot.timeout {
        None => {
            if revive_lockout == Some(GameColor::White) {
                make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                    .on_press(Message::NoAction)
                    .style(white_button)
                    .into()
            } else if tm.can_revive_team_timeout(GameColor::White).is_ok() {
                let held = revive_held == Some(GameColor::White);
                let face = if held {
                    make_multi_label_button((
                        fl!("light-timeout-line-1"),
                        fl!("light-timeout-line-2"),
                    ))
                    .style(white_button_armed)
                } else {
                    make_multi_label_button((
                        fl!("light-timeout-line-1"),
                        fl!("light-timeout-line-2"),
                    ))
                    .style(white_button)
                };
                mouse_area(face)
                    .on_press(Message::TimeoutRevivePressed(GameColor::White))
                    .on_release(Message::TimeoutReviveReleased(GameColor::White))
                    .into()
            } else {
                make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                    .on_press_maybe(
                        tm.can_start_team_timeout(GameColor::White)
                            .ok()
                            .map(|_| Message::TeamTimeout(GameColor::White, false)),
                    )
                    .style(white_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::White(_)) => {
            make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(yellow_button)
                .into()
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("switch-to"), fl!("light-team-name-caps")))
                .on_press_maybe(
                    tm.can_switch_to_team_timeout(GameColor::White)
                        .ok()
                        .map(|_| Message::TeamTimeout(GameColor::White, true)),
                )
                .style(white_button)
                .into()
        }
    };
```

> The `referee`, `penalty`, and final `row![black, referee, penalty, white]` lines are unchanged. `black`/`white` are now `Element`; `referee`/`penalty` stay `Button`. The `row!` macro converts each independently (both implement `Into<Element>`), so this compiles.

- [ ] **Step 16: Compile**

Run: `cargo build -p refbox`
Expected: builds with no errors.

- [ ] **Step 17: Lint (mirrors CI / `just lint`)**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: zero warnings. (Watch for: unused import, never-constructed variant, never-used function — all should be resolved because the view constructs the variants and uses the armed styles.)

- [ ] **Step 18: Commit**

```bash
git add refbox/src/app/theme/button.rs refbox/src/app/theme/mod.rs \
        refbox/src/app/message.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): hold a used team timeout button to revive it"
```

---

## Task 3: Full check + manual walkthrough

**Files:** none (verification only)

- [ ] **Step 1: Run the full gate**

Run: `just check`
Expected: format, lint, tests, and audit all clean.

- [ ] **Step 2: Launch the refbox for manual verification**

Build and launch the binary in the background (WSLg: force X11; needs sandbox disabled for the Wayland/Pulse sockets):

```bash
cargo build -p refbox
WAYLAND_DISPLAY= ./target/debug/refbox
```

(The human drives the UI and reports observations.)

- [ ] **Step 3: Walk through the behaviour**

Confirm each, with `num_team_timeouts_allowed = 1` (the default), during a half:
- Start a team timeout, then end it → that team's timeout button greys out.
- Press and **hold** the greyed button → it brightens while held; at ~5 seconds it returns to its normal active "TIMEOUT" look. No confirmation, no flash.
- **Immediately** try to tap/hold it → nothing happens for ~2 seconds (safety lockout); after that, a fresh tap starts a timeout normally.
- Hold the greyed button ~2 seconds then release → nothing changes (button stays greyed).
- While an opponent/ref timeout is running (button reads "Switch to…") → holding does nothing.

- [ ] **Step 4: Record the walkthrough result**

Note the outcome in the plan's Deviations section (below) or the PR description. If anything failed, debug before opening a PR.

---

## Deviations

(Record here if execution diverged from the plan — lean process, no standalone deviation commits.)

---

## Self-review (completed during planning)

**1. Spec coverage:**
- 5-second hold → Task 2 Step 7 (`TIMEOUT_REVIVE_HOLD_DURATION`) + Step 10 handler.
- Brighten while held, no flash → Task 2 Steps 1–2 (armed styles) + Steps 14–15 (held branch). No flash anywhere.
- Give back exactly one → Task 1 (`saturating_sub` by one).
- 2-second safety lockout, inert-but-active → Task 2 Step 7 (`TIMEOUT_REVIVE_LOCKOUT_DURATION`) + Step 10 (lockout task) + Steps 14–15 (`revive_lockout` branch with `NoAction`).
- Only when no timeout running + greyed for used-up + in a half → Task 1 `can_revive_team_timeout` (period + `timeout_state` + count checks) + the view gating on `is_ok()`.
- Always-on, no setting; refbox-only; no new translation keys → confirmed (no config/locale files touched).
- Per-half/per-game untouched → only `timeouts_used` is decremented; reset logic unchanged; `test_revive_team_timeout_guards` exercises a non-play period.

**2. Placeholder scan:** No TBD/TODO/"handle edge cases"; every code step shows complete code.

**3. Type consistency:** `can_revive_team_timeout`/`revive_team_timeout` (Task 1) match their uses in Task 2 Steps 10, 14, 15. Message variant names and field types (`Option<Color>` in mod.rs == `Option<GameColor>` in shared_elements, same `uwh_common` `color::Color`) are consistent across declaration, handlers, call site, and view. `black_button_armed`/`white_button_armed` are defined (Step 1), re-exported (Step 2), and used (Steps 14–15).
