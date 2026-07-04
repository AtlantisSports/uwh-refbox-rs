# Timeout-Revive Release-to-Confirm Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the timeout-revive "RESTORED" (yellow) state persist until the button is released — release confirms the restore — and remove the 2-second decide window and the "hold-through immediately starts a timeout" behaviour.

**Architecture:** A state-machine simplification in the `refbox` app layer. Rename `RevivePhase::Deciding` → `RevivePhase::Restored`, drop the decide-window timer (message + handler + duration constant), and have the 5-second hold land directly in a timer-less `Restored` state that is confirmed on release.

**Tech Stack:** Rust 2024, iced 0.13 (`refbox` crate).

## Global Constraints

- **Crate scope:** `refbox` only. No `uwh-common`, no wire format, no `wireless-remote`.
- **MSRV:** Rust 1.85; **Edition:** Rust 2024. **Clippy:** `cargo clippy -p refbox -- -D warnings` clean.
- **No label/translation changes.** On-screen text stays TIMEOUT / RESTORED. The translation key `revive-deciding-line-2` keeps its current name (internal-only; renaming it would churn 15 locale files for no visible effect).
- **Keep** the 5-second RED hold (`TIMEOUT_REVIVE_HOLD_DURATION`) and its `TimeoutReviveHoldElapsed` timer.
- **Process:** lean. Compile + `cargo clippy -p refbox -- -D warnings` + `just check`. One commit. Behaviour verified by walkthrough (acceptance criteria below).

---

### Task 1: Simplify the revive state machine to release-to-confirm

All edits below must land together — removing the `TimeoutReviveDecideElapsed` variant requires removing its spawn and handler in the same change, or the crate will not compile.

**Files:**
- Modify: `refbox/src/app/mod.rs` — `RevivePhase` enum (~80-85), `TIMEOUT_REVIVE_DECIDE_DURATION` const (line 76), `TimeoutReviveReleased` comment (~3823), `TimeoutReviveHoldElapsed` handler tail (~3852-3865), and the entire `TimeoutReviveDecideElapsed` handler (~3867-3893).
- Modify: `refbox/src/app/message.rs` — variant decl (line 245 + its doc comment 244), classification group (line 374), `PartialEq` arm (lines 458-460), catch-all group (line 698).
- Modify: `refbox/src/app/view_builders/shared_elements.rs` — two `RevivePhase::Deciding` checks (lines 213, 294).

**Interfaces:**
- Produces: `RevivePhase::Restored` replaces `RevivePhase::Deciding` everywhere. No new public symbols. `TimeoutReviveDecideElapsed` and `TIMEOUT_REVIVE_DECIDE_DURATION` cease to exist.

- [ ] **Step 1: Rename the enum variant and update its doc comment**

In `refbox/src/app/mod.rs`, replace the `RevivePhase` body (currently):

```rust
pub(crate) enum RevivePhase {
    /// Finger down on a used-up button, counting down to the 5s revive.
    Reviving,
    /// Revived; finger still down, within the 2s "release to bank / hold to start" window.
    Deciding,
}
```

with:

```rust
pub(crate) enum RevivePhase {
    /// Finger down on a used-up button, counting down to the 5s revive.
    Reviving,
    /// Revived; finger still down. Stays here until release, which confirms the restore.
    Restored,
}
```

- [ ] **Step 2: Remove the decide-duration constant**

In `refbox/src/app/mod.rs`, delete line 76:

```rust
const TIMEOUT_REVIVE_DECIDE_DURATION: Duration = Duration::from_secs(2);
```

Leave `TIMEOUT_REVIVE_HOLD_DURATION` (line 73) untouched.

- [ ] **Step 3: Make the 5s hold land in `Restored` with no follow-up timer**

In `refbox/src/app/mod.rs`, in the `TimeoutReviveHoldElapsed` handler, replace the post-revive tail (currently):

```rust
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
```

with:

```rust
                let apply_task = self.apply_snapshot(snapshot);
                // Enter the "restored, hold to keep showing" state. It has no timer:
                // it persists until the finger is lifted, and release confirms the restore.
                self.timeout_revive_token += 1;
                let token = self.timeout_revive_token;
                self.timeout_revive = Some(ReviveHold {
                    color,
                    phase: RevivePhase::Restored,
                    token,
                });
                info!("Timeout revived for {color}; awaiting release to confirm, token={token}");
                apply_task
```

- [ ] **Step 4: Delete the `TimeoutReviveDecideElapsed` handler entirely**

In `refbox/src/app/mod.rs`, remove the whole arm (from `Message::TimeoutReviveDecideElapsed(token, color) => {` through its closing `}`, immediately before `Message::BeepTestStart => {`):

```rust
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

- [ ] **Step 5: Update the `TimeoutReviveReleased` comment to drop the renamed phase**

In `refbox/src/app/mod.rs`, the `TimeoutReviveReleased` handler comment currently reads:

```rust
                // Finger up, or pointer left the button. In Reviving this cancels
                // (nothing given back); in Deciding it banks the already-revived timeout.
```

Change `in Deciding it banks` to `in Restored it confirms`:

```rust
                // Finger up, or pointer left the button. In Reviving this cancels
                // (nothing given back); in Restored it confirms the already-revived timeout.
```

(The handler body is unchanged — it already just clears `self.timeout_revive`.)

- [ ] **Step 6: Remove the message variant and its three other references in `message.rs`**

In `refbox/src/app/message.rs`:

(a) Delete the variant and its doc comment (lines 244-245):

```rust
    /// The 2-second post-revive decide window elapsed for the given team.
    TimeoutReviveDecideElapsed(u64, GameColor),
```

(b) In the classification group, delete the line:

```rust
            | Self::TimeoutReviveDecideElapsed(_, _)
```

(c) Delete the `PartialEq` arm:

```rust
            (Self::TimeoutReviveDecideElapsed(a, b), Self::TimeoutReviveDecideElapsed(c, d)) => {
                a == c && b == d
            }
```

(d) In the catch-all group, delete the line:

```rust
            | (Self::TimeoutReviveDecideElapsed(_, _), _)
```

- [ ] **Step 7: Point the view's yellow face at `RevivePhase::Restored`**

In `refbox/src/app/view_builders/shared_elements.rs`, change both occurrences of:

```rust
            if black_phase == Some(RevivePhase::Deciding) {
```
and
```rust
            if white_phase == Some(RevivePhase::Deciding) {
```

to use `RevivePhase::Restored` (i.e. `black_phase == Some(RevivePhase::Restored)` and `white_phase == Some(RevivePhase::Restored)`). The label lines inside (`fl!("timeout")`, `fl!("revive-deciding-line-2")`) are unchanged.

- [ ] **Step 8: Verify it compiles and lints**

Run from the worktree root (`.claude/worktrees/feat+refbox+timeout-revive-long-press`):

```bash
cargo clippy -p refbox -- -D warnings
```

Expected: finishes with no warnings/errors. (Mirrors CI / `just lint` — do NOT add `--all-targets` here; on local 1.85 it surfaces ~90 pre-existing test-code lints that are not real failures.)

- [ ] **Step 9: Run the full check suite**

```bash
just check
```

Expected: PASS (fmt, clippy, tests, audit). The existing 3 revive unit tests in `tournament_manager` are unaffected (the `revive_team_timeout` API did not change).

- [ ] **Step 10: Commit**

```bash
git add refbox/src/app/mod.rs refbox/src/app/message.rs refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): confirm timeout revive on release, drop decide window"
```

---

## Manual Walkthrough (after commit)

Build and launch:

```bash
cargo build -p refbox
WAYLAND_DISPLAY= ./target/debug/refbox   # background, sandbox disabled
```

Confirm the acceptance criteria:
1. Hold a greyed (used-up) team-timeout button → RED "HOLD TO / RESTORE".
2. Release during RED → nothing restored (button stays greyed).
3. Hold ~5s → YELLOW "TIMEOUT / RESTORED"; timeout is restored.
4. Keep holding YELLOW for 10+ seconds → it stays YELLOW and does **not** start a timeout.
5. Release from YELLOW → button returns to a normal available team-timeout button; pressing it starts a timeout normally.
6. Start a *running* team timeout separately → CANCEL (orange) for 15s then END (red): grace-window feature still works.

## Self-Review Notes

- **Spec coverage:** rename (Step 1) ✓; remove duration (Step 2) ✓; land in Restored, no timer (Step 3) ✓; delete decide handler (Step 4) ✓; release confirms (Step 5, plus existing release body) ✓; message.rs cleanup (Step 6) ✓; view key (Step 7) ✓.
- **Placeholder scan:** none.
- **Type consistency:** `RevivePhase::Restored` used consistently in mod.rs (Steps 1, 3) and shared_elements.rs (Step 7); removed symbols `TimeoutReviveDecideElapsed` / `TIMEOUT_REVIVE_DECIDE_DURATION` have no remaining references after Steps 2, 4, 6.
- **Out of scope (unchanged):** RED build-up + labels, all translations, grace-window feature, `revive_team_timeout` API and its tests.
