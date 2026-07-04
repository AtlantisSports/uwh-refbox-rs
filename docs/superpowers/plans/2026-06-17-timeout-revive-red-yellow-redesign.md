# Timeout Revive — Red→Yellow Redesign (v2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework the timeout-revive hold so the button is RED while held (reviving), turns YELLOW for a ~2s window after the 5s revive, and — if held through that window — starts the team timeout; releasing in the window banks it.

**Architecture:** Replace v1's "brighten + inert 2s lockout" with a two-phase hold state machine (`Reviving` → `Deciding`) in the refbox app layer. The hold lives in one consolidated app field; the view keeps a single stable `mouse_area` and only swaps the inner button's colour (greyed → red → yellow). The existing `revive_team_timeout` and `start_team_timeout` actions are reused; no `tournament_manager` change.

**Tech Stack:** Rust 2024, iced 0.13, tokio async timers. All changes in the `refbox` crate (app + button theme).

**Design spec:** `docs/superpowers/specs/2026-06-17-timeout-revive-red-yellow-redesign.md`

**Branch / worktree:** `feat/refbox/timeout-revive-long-press` at
`/home/estraily/projects/uwh-refbox-rs/.claude/worktrees/feat+refbox+timeout-revive-long-press`
(already on the branch; v1 is committed there at `b926166e`, `7b6d9269`, `4d92f4fb`). v2 lands as **new commits on top** — do not rewrite history.

---

## Scope / process

- **`refbox` crate only** (app layer + `theme/button.rs`). No `tournament_manager`, `uwh-common`, wire-format, or other-crate changes.
- This is an **app-interaction state machine** (not the game state machine), so: careful compile/lint/`just check` + manual walkthrough. No new unit tests (the reused game actions are already tested; iced view/interaction is not unit-tested here).
- **One commit:** the message rename, field consolidation, handler rewrite, view rewrite, and theme swap are interdependent (intermediate states neither compile nor pass `-D warnings`). Do all steps, then compile + lint + test + commit at the end of Task 1.

## File structure (what changes)

| File | Change |
|------|--------|
| `refbox/src/app/theme/button.rs` | Remove `white_button_armed`/`black_button_armed`; add `red_button_armed`/`yellow_button_armed` |
| `refbox/src/app/theme/mod.rs` | Re-export: drop the two black/white-armed names, add the two red/yellow-armed names |
| `refbox/src/app/message.rs` | Rename the 4th revive variant `TimeoutReviveLockoutElapsed` → `TimeoutReviveDecideElapsed` (declaration + `is_repeatable` + both `PartialEq` arms) |
| `refbox/src/app/mod.rs` | Add `RevivePhase` enum + `ReviveHold` struct; replace the 4 v1 fields with `timeout_revive` + `timeout_revive_token`; rename the 2s constant; rewrite the 4 handlers; update the `build_timeout_ribbon` call site |
| `refbox/src/app/view_builders/shared_elements.rs` | Import `RevivePhase`; change signature to take `revive_hold`; rewrite the black & white `None`-arms |

---

## Task 1: Red→Yellow revive interaction (one commit)

**Files:** the five listed above. Line numbers are current-as-of-writing; re-read around each anchor before editing.

### Theme

- [ ] **Step 1: Remove the two v1 armed styles in `theme/button.rs`**

Delete the `white_button_armed` function and its doc comment (currently lines 109–115):

```rust
/// Like `white_button`, but always renders in the bright "active" colour even
/// when the button is non-interactive (no `on_press`). Used to brighten a
/// timeout button while it is held for the timeout-revive long-press, where the
/// button is wrapped in a `mouse_area` and has no `on_press` of its own.
pub fn white_button_armed(theme: &Theme, _status: Status) -> Style {
    white_button(theme, Status::Active)
}
```

Delete the `black_button_armed` function and its doc comment (currently lines 144–150):

```rust
/// Like `black_button`, but always renders in the bright "active" colour even
/// when the button is non-interactive (no `on_press`). Used to brighten a
/// timeout button while it is held for the timeout-revive long-press, where the
/// button is wrapped in a `mouse_area` and has no `on_press` of its own.
pub fn black_button_armed(theme: &Theme, _status: Status) -> Style {
    black_button(theme, Status::Active)
}
```

- [ ] **Step 2: Add the red + yellow armed styles in `theme/button.rs`**

Immediately after the `red_button` function's closing brace, add:

```rust
/// Like `red_button`, but always renders in the bright "active" colour even when
/// the button is non-interactive (no `on_press`). Colours a timeout button red
/// while it is held during the revive long-press, where the button is wrapped in
/// a `mouse_area` and has no `on_press` of its own.
pub fn red_button_armed(theme: &Theme, _status: Status) -> Style {
    red_button(theme, Status::Active)
}
```

Immediately after the `yellow_button` function's closing brace, add:

```rust
/// Like `yellow_button`, but always renders in the bright "active" colour even
/// when the button is non-interactive (no `on_press`). Colours a timeout button
/// yellow during the post-revive "decide" window, where the button is wrapped in
/// a `mouse_area` and has no `on_press` of its own.
pub fn yellow_button_armed(theme: &Theme, _status: Status) -> Style {
    yellow_button(theme, Status::Active)
}
```

- [ ] **Step 3: Update the re-export list in `theme/mod.rs`**

Replace the `pub use button::{ … };` block (currently lines 350–356) with:

```rust
pub use button::{
    black_button, black_selected_button, blue_button, blue_selected_button,
    blue_with_border_button, gray_button, green_button, green_selected_button, light_gray_button,
    light_gray_selected_button, orange_button, orange_selected_button, red_button,
    red_button_armed, red_selected_button, white_button, white_selected_button, yellow_button,
    yellow_button_armed, yellow_selected_button,
};
```

### Message rename

- [ ] **Step 4: Rename the 4th variant in `message.rs` (declaration)**

Change the variant declaration (currently line 244) from:

```rust
    TimeoutReviveLockoutElapsed(u64, GameColor),
```

to:

```rust
    TimeoutReviveDecideElapsed(u64, GameColor),
```

- [ ] **Step 5: Rename it in the `is_repeatable` match (currently line 372)**

Change `| Self::TimeoutReviveLockoutElapsed(_, _)` to `| Self::TimeoutReviveDecideElapsed(_, _)` (it stays in the same `=> false` group).

- [ ] **Step 6: Rename it in both `PartialEq` arms**

Positive arm (currently lines 455–457): change

```rust
            (Self::TimeoutReviveLockoutElapsed(a, b), Self::TimeoutReviveLockoutElapsed(c, d)) => {
                a == c && b == d
            }
```

to

```rust
            (Self::TimeoutReviveDecideElapsed(a, b), Self::TimeoutReviveDecideElapsed(c, d)) => {
                a == c && b == d
            }
```

Catch-all arm (currently line 694): change `| (Self::TimeoutReviveLockoutElapsed(_, _), _)` to `| (Self::TimeoutReviveDecideElapsed(_, _), _)`.

### App state, constant, types

- [ ] **Step 7: Rename the 2s constant in `mod.rs` (currently lines 75–76)**

Replace:

```rust
/// After a revive, the just-revived team's button ignores all input for this
/// long, so a lingering hold-press cannot immediately start a timeout.
const TIMEOUT_REVIVE_LOCKOUT_DURATION: Duration = Duration::from_secs(2);
```

with:

```rust
/// After the 5s revive, the team's button shows YELLOW for this long: releasing
/// in this window banks the timeout; holding through it starts a team timeout.
const TIMEOUT_REVIVE_DECIDE_DURATION: Duration = Duration::from_secs(2);
```

(Leave `TIMEOUT_REVIVE_HOLD_DURATION` = 5s on line 73 unchanged.)

- [ ] **Step 8: Add the `RevivePhase` enum and `ReviveHold` struct in `mod.rs`**

Add these as top-level items near the constants (e.g. just below the two `TIMEOUT_REVIVE_*` constants):

```rust
/// Which phase an in-progress timeout-revive long-press is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevivePhase {
    /// Finger down on a used-up button, counting down to the 5s revive.
    Reviving,
    /// Revived; finger still down, within the 2s "release to bank / hold to start" window.
    Deciding,
}

/// An in-progress timeout-revive long-press.
struct ReviveHold {
    color: Color,
    phase: RevivePhase,
    /// Token of the async timer this hold is currently waiting on; a timer whose
    /// token no longer matches the live hold is stale and ignored.
    token: u64,
}
```

- [ ] **Step 9: Replace the four v1 state fields with two (currently lines 172–178)**

Replace:

```rust
    timeout_revive_held: Option<Color>,
    timeout_revive_token: u64,
    timeout_revive_lockout: Option<Color>,
    timeout_revive_lockout_token: u64,
```

(and their doc comments) with:

```rust
    /// The in-progress timeout-revive long-press, if any (`None` = no hold active).
    timeout_revive: Option<ReviveHold>,
    /// Monotonic source of revive-timer tokens (never reset; guards stale timers).
    timeout_revive_token: u64,
```

- [ ] **Step 10: Update the field initialisers (currently lines 1483–1486)**

Replace:

```rust
            timeout_revive_held: None,
            timeout_revive_token: 0,
            timeout_revive_lockout: None,
            timeout_revive_lockout_token: 0,
```

with:

```rust
            timeout_revive: None,
            timeout_revive_token: 0,
```

### Handlers

- [ ] **Step 11: Replace the four v1 handlers (currently lines 3775–3836) with the v2 handlers**

Replace the whole block of `Message::TimeoutRevivePressed` … through the end of `Message::TimeoutReviveLockoutElapsed` with:

```rust
            Message::TimeoutRevivePressed(color) => {
                // Press-down on a used-up (greyed) team timeout button: begin the
                // 5-second revive hold. The view only attaches this on an eligible button.
                if matches!(&self.timeout_revive, Some(h) if h.color == color) {
                    return Task::none();
                }
                self.timeout_revive_token += 1;
                let token = self.timeout_revive_token;
                self.timeout_revive = Some(ReviveHold {
                    color,
                    phase: RevivePhase::Reviving,
                    token,
                });
                info!("Timeout-revive hold started for {color}, token={token}");
                Task::future(async move {
                    sleep(TIMEOUT_REVIVE_HOLD_DURATION).await;
                    Message::TimeoutReviveHoldElapsed(token, color)
                })
            }
            Message::TimeoutReviveReleased(color) => {
                // Finger up, or pointer left the button. In Reviving this cancels
                // (nothing given back); in Deciding it banks the already-revived timeout.
                if matches!(&self.timeout_revive, Some(h) if h.color == color) {
                    self.timeout_revive = None;
                    info!("Timeout-revive hold released for {color}");
                }
                Task::none()
            }
            Message::TimeoutReviveHoldElapsed(token, color) => {
                // The 5-second revive hold elapsed. Only proceed if this is still the
                // current Reviving hold for this team.
                if !matches!(
                    &self.timeout_revive,
                    Some(h) if h.color == color
                        && h.token == token
                        && h.phase == RevivePhase::Reviving
                ) {
                    return Task::none();
                }
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                if tm.revive_team_timeout(color).is_err() {
                    // State moved on during the hold (e.g. half ended); nothing to do.
                    std::mem::drop(tm);
                    self.timeout_revive = None;
                    return Task::none();
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                let apply_task = self.apply_snapshot(snapshot);
                // Enter the 2-second "release to bank / hold to start" window.
                self.timeout_revive_token += 1;
                let token = self.timeout_revive_token;
                self.timeout_revive = Some(ReviveHold {
                    color,
                    phase: RevivePhase::Deciding,
                    token,
                });
                info!("Timeout revived for {color}; deciding window started, token={token}");
                let decide_task = Task::future(async move {
                    sleep(TIMEOUT_REVIVE_DECIDE_DURATION).await;
                    Message::TimeoutReviveDecideElapsed(token, color)
                });
                Task::batch(vec![apply_task, decide_task])
            }
            Message::TimeoutReviveDecideElapsed(token, color) => {
                // The 2-second window elapsed while still held: start the team timeout
                // (spending the just-revived timeout).
                if !matches!(
                    &self.timeout_revive,
                    Some(h) if h.color == color
                        && h.token == token
                        && h.phase == RevivePhase::Deciding
                ) {
                    return Task::none();
                }
                self.timeout_revive = None;
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                if tm.start_team_timeout(color, now).is_err() {
                    // State moved on during the window; nothing to do.
                    std::mem::drop(tm);
                    return Task::none();
                }
                if let AppState::TimeEdit(_, _, ref mut time) = self.app_state {
                    *time = Some(tm.timeout_clock_time(now).unwrap());
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                info!("Timeout-revive: held through, starting {color} team timeout");
                self.apply_snapshot(snapshot)
            }
```

### View call site

- [ ] **Step 12: Update the `build_timeout_ribbon` call (currently lines 4575–4581)**

Replace:

```rust
                main_view = main_view.push(build_timeout_ribbon(
                    &self.snapshot,
                    &self.tm,
                    self.config.mode,
                    self.timeout_revive_held,
                    self.timeout_revive_lockout,
                ));
```

with:

```rust
                main_view = main_view.push(build_timeout_ribbon(
                    &self.snapshot,
                    &self.tm,
                    self.config.mode,
                    self.timeout_revive.as_ref().map(|h| (h.color, h.phase)),
                ));
```

### View builder

- [ ] **Step 13: Import `RevivePhase` in `shared_elements.rs`**

Add near the top of the file (with the other `use` lines):

```rust
use crate::app::RevivePhase;
```

- [ ] **Step 14: Change the `build_timeout_ribbon` signature (currently lines 179–185)**

Replace the two params:

```rust
    revive_held: Option<GameColor>,
    revive_lockout: Option<GameColor>,
```

with one:

```rust
    revive_hold: Option<(GameColor, RevivePhase)>,
```

- [ ] **Step 15: Derive per-team phases right after the `tm` lock (currently line 186)**

Immediately after `let tm = tm.lock().unwrap();` add:

```rust
    let black_phase = match revive_hold {
        Some((GameColor::Black, p)) => Some(p),
        _ => None,
    };
    let white_phase = match revive_hold {
        Some((GameColor::White, p)) => Some(p),
        _ => None,
    };
```

- [ ] **Step 16: Rewrite the `black` `None`-arm (currently lines 189–230)**

Replace the entire `None => { … }` block of the `black` match with:

```rust
        None => {
            if black_phase == Some(RevivePhase::Deciding) {
                // Revived, still held: YELLOW "release to bank / hold to start" window.
                // The mouse_area stays in place (same handlers) so the continuous
                // press keeps being tracked across the colour change.
                mouse_area(
                    make_multi_label_button((
                        fl!("dark-timeout-line-1"),
                        fl!("dark-timeout-line-2"),
                    ))
                    .style(yellow_button_armed),
                )
                .on_press(Message::TimeoutRevivePressed(GameColor::Black))
                .on_release(Message::TimeoutReviveReleased(GameColor::Black))
                .on_exit(Message::TimeoutReviveReleased(GameColor::Black))
                .into()
            } else if tm.can_revive_team_timeout(GameColor::Black).is_ok() {
                // Used-up: greyed normally; RED while in the Reviving phase. The inner
                // button has no `on_press`, so the mouse_area captures the press/hold.
                let face = if black_phase == Some(RevivePhase::Reviving) {
                    make_multi_label_button((
                        fl!("dark-timeout-line-1"),
                        fl!("dark-timeout-line-2"),
                    ))
                    .style(red_button_armed)
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
                    .on_exit(Message::TimeoutReviveReleased(GameColor::Black))
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
```

(Leave the `Some(TimeoutSnapshot::Black(_))` and `Some(White|Ref|PenaltyShot)` arms of the `black` match unchanged.)

- [ ] **Step 17: Rewrite the `white` `None`-arm (currently lines 252–287)**

Replace the entire `None => { … }` block of the `white` match with:

```rust
        None => {
            if white_phase == Some(RevivePhase::Deciding) {
                mouse_area(
                    make_multi_label_button((
                        fl!("light-timeout-line-1"),
                        fl!("light-timeout-line-2"),
                    ))
                    .style(yellow_button_armed),
                )
                .on_press(Message::TimeoutRevivePressed(GameColor::White))
                .on_release(Message::TimeoutReviveReleased(GameColor::White))
                .on_exit(Message::TimeoutReviveReleased(GameColor::White))
                .into()
            } else if tm.can_revive_team_timeout(GameColor::White).is_ok() {
                let face = if white_phase == Some(RevivePhase::Reviving) {
                    make_multi_label_button((
                        fl!("light-timeout-line-1"),
                        fl!("light-timeout-line-2"),
                    ))
                    .style(red_button_armed)
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
                    .on_exit(Message::TimeoutReviveReleased(GameColor::White))
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
```

(Leave the `Some(TimeoutSnapshot::White(_))` and `Some(Black|Ref|PenaltyShot)` arms of the `white` match unchanged.)

### Build, lint, test, commit

- [ ] **Step 18: Compile**

Run: `cargo build -p refbox`
Expected: builds, no errors. If a borrow/type error appears in the view, re-read the surrounding code — every match arm must end `.into()` and be `Element<'a, Message>`; the inner button in the red/greyed branch must have **no** `on_press`.

- [ ] **Step 19: Lint (mirrors `just lint`)**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: zero warnings. In particular, the old `black_button_armed`/`white_button_armed` are gone (would be "never used") and the new `red_button_armed`/`yellow_button_armed` are used by the view.

- [ ] **Step 20: Tests**

Run: `cargo test -p refbox`
Expected: all pass (266 as of v1; the existing `revive_team_timeout` and timeout tests are unaffected).

- [ ] **Step 21: Format + commit**

```bash
cargo fmt -p refbox
git add refbox/src/app/theme/button.rs refbox/src/app/theme/mod.rs \
        refbox/src/app/message.rs refbox/src/app/mod.rs \
        refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): red-while-held, yellow decide window, hold-through starts timeout

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

(If the pre-commit hook reports formatting, the `cargo fmt -p refbox` above should already have handled it; re-`git add` the five files and re-commit if needed.)

---

## Task 2: Full check + manual walkthrough

**Files:** none (verification only).

- [ ] **Step 1: Full gate**

Run: `just check`
Expected: fmt, lint, tests, audit all clean (the `lru`/`macroquad` audit advisories are pre-existing, allow-listed).

- [ ] **Step 2: Build + launch for the walkthrough**

```bash
cargo build -p refbox
pkill -x refbox 2>/dev/null || true
WAYLAND_DISPLAY= ./target/debug/refbox
```

(Launch in the background; the human drives the UI.)

- [ ] **Step 3: Walk through the behaviour** (default config: 1 timeout/team; during a half)

- Use a team's timeout, then End Timeout → that team's button greys.
- Press & hold the greyed button → it turns **RED** while held; at ~5s it turns **YELLOW**.
- **Release during the yellow window** → button turns **black/white** (banked, available); confirm no timeout started.
- Repeat; this time **keep holding** through the yellow window → a team timeout **starts** (running "End Timeout" view).
- **Release during the red phase** (before 5s) → back to greyed, nothing given back.
- During the yellow window, **slide the pointer off** the button → banks (no timeout starts).

- [ ] **Step 4: Record the result** in the Deviations section / PR description.

---

## Deviations

(Record here if execution diverged — lean process, no standalone deviation commits.)

---

## Self-review (completed during planning)

**1. Spec coverage:**
- Red while held → Steps 2, 16, 17 (`red_button_armed` in the Reviving branch).
- Yellow decide window → Steps 2, 16, 17 (`yellow_button_armed` in the Deciding branch) + Step 7 constant.
- 5s revive then 2s decide → Step 11 handlers (`HoldElapsed` → revive → `Deciding` + decide timer).
- Release in window banks / hold-through starts → Step 11 (`Released` clears; `DecideElapsed` calls `start_team_timeout`).
- Slide-off banks → `on_exit` → `TimeoutReviveReleased` in Steps 16/17.
- Stable mouse_area across colour change → both Deciding and Reviving branches render `mouse_area(...)` with identical handlers at the same position (Steps 16/17); flagged in the spec as the one thing to confirm early (Step 18 build + Step 3 walkthrough).
- Retire v1 brighten + lockout → Steps 1, 9, 11 (no `Deciding`-inert/NoAction path remains).
- Reuse game actions, no `tournament_manager` change → Step 11 calls existing `revive_team_timeout`/`start_team_timeout`.

**2. Placeholder scan:** none — every code step has complete code and exact anchors.

**3. Type consistency:** `RevivePhase` (Step 8) is used in the field/handlers (9, 11), the call site map (12), the signature (14), the phase derivation (15), and the view (16, 17). `ReviveHold { color, phase, token }` fields match every `matches!`/constructor use. `TimeoutReviveDecideElapsed(u64, GameColor)` is consistent across declaration (4), `is_repeatable` (5), `PartialEq` (6), the handler (11), and the scheduling site (11). `red_button_armed`/`yellow_button_armed` are defined (2), re-exported (3), and used (16, 17).
