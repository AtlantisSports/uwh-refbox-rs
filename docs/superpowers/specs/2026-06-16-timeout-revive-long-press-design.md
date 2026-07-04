# Timeout Revive via Long-Press — Design

**Date:** 2026-06-16
**Status:** Approved (design); implementation not started
**Crate scope:** `refbox` only
**Process:** Heavy care for the state-machine method; lean for the UI wiring

---

## 1. Summary

When team timeouts are allowed and a team has already used its allowed timeout(s), that
team's timeout button greys out (as it does today). This feature lets the operator
**press and hold that greyed button for 5 seconds** to give the team back **one** timeout,
re-enabling the button. It is a deliberate, rarely-used **correction** control — for undoing
an accidental timeout charge or honouring a referee's ruling that a charged timeout should not
count.

## 2. Motivation & purpose

The "used" count is currently a one-way door: once a team's timeout is consumed, there is no
in-app way to give it back until the next half/game reset. Operators occasionally need to undo
a mis-tap or apply a ref's correction. The chosen design is intentionally **guarded and
low-discoverability** (a long hold, no prominent control) because the need is rare and an
accidental revive should be hard to trigger.

## 3. Behaviour specification (the operator-facing contract)

- **Trigger:** Press and hold a team's timeout button while it is greyed *because that team has
  used its allowed timeout(s)*.
- **While held:** The button **brightens** (toward its active appearance) so the operator can
  see the press has registered. There is **no countdown and no progress fill**.
- **On a continuous 5-second hold:** The refbox gives that team back **one** timeout. The button
  immediately returns to its normal **active "TIMEOUT"** appearance. There is **no confirmation
  prompt and no completion flash**.
- **Safety period:** For **2 seconds** after the button returns to active, the button **ignores
  all input** — a press, a tap, or another hold does nothing. This guarantees the finger that was
  holding cannot immediately start (or re-trigger) a timeout when it lifts. After the 2 seconds,
  the button behaves normally and starting a timeout requires a fresh, deliberate press.
- **Release before 5 seconds:** Nothing changes; the button simply returns to greyed. No harm.
- **Per successful hold:** Exactly **one** timeout is returned (the used count drops by one,
  never below zero). With the usual setting of one timeout per team, one hold fully restores it.

### When the hold-to-revive is available

The hold affordance is active **only** when **all** of the following hold:

1. **No timeout is currently running** (`snapshot.timeout == None`). It does **not** apply while
   the button reads "Switch to…" during an opponent/ref/penalty-shot timeout. (Rationale:
   overloading a hold onto the "switch" state is confusing, and a rare correction can wait until
   the active timeout ends.)
2. The team has **at least one used timeout to give back** (`timeouts_used[color] > 0`).
3. The team is **at or above its allowed cap** (`timeouts_used[color] >= num_team_timeouts_allowed`),
   i.e. the button is greyed *specifically because the team is timed-out*, not greyed for some
   other reason.

If the button is greyed for any reason other than "used up" (e.g. a period where timeouts cannot
be started), the hold does nothing.

## 4. Scope, non-goals, blast radius

**In scope (all in the `refbox` crate):**
- A new state-machine method to return one used timeout.
- App-layer press-and-hold wiring (reusing the manual-alarm button's proven pattern).
- A view change to make the greyed timeout button hold-sensitive and to apply the brighten +
  the 2-second post-revive lockout.

**Non-goals / explicitly not doing:**
- No change to the timeout-counting rules themselves (per-half vs per-game reset logic is
  untouched; revive only lowers the count).
- No wire-format / `uwh-common` change. The used-count is internal to the refbox and is **not**
  transmitted to the LED scoreboard, the stream overlay, or the wireless remote, so none of them
  need to know about a revive.
- No new configuration setting. The feature is **always available**; the 5-second hold is the
  only guard.
- No new on-screen text (the button keeps its existing label, just brighter), so **no new
  translation keys** are required.
- No general "edit timeout count" UI and no reverse direction (charging a timeout that was not
  auto-counted). This is one-directional: give a used timeout back.
- No `wireless-remote`, `overlay`, or `uwh-common` edits.

**Blast radius:** Single crate (`refbox`). The only piece inside the high-care state-machine zone
is the small count-adjustment method.

## 5. Design / architecture

### 5a. State-machine change (`refbox/src/tournament_manager/mod.rs`)

Add a small, well-isolated public method that mirrors the existing `saturating_sub` pattern
already used in `switch_to_team_timeout`:

```rust
/// Give one used team timeout back to `color`. Returns an error if the team has
/// no used timeout to revive. Touches only the used-count; does not affect the
/// clock, period, or any active timeout.
pub fn revive_team_timeout(&mut self, color: Color) -> Result<()>;
```

Behaviour:
- If `timeouts_used[color] > 0`: decrement by one (saturating at zero).
- Else: return an error (a new `TournamentManagerError` variant, e.g. `NoTimeoutToRevive(color)`,
  or the nearest suitable existing variant).
- Does **not** touch the clock, current period, active timeout, scores, or penalties.

This is the single piece that gets **heavy-process care**: its own unit test(s) and review.

### 5b. App-layer wiring (`refbox/src/app/mod.rs`, `message.rs`, `view_builders/shared_elements.rs`)

Reuse the manual-alarm button's press-and-hold mechanism (the model for state fields, async
hold timer, and token-based stale-timer cancellation already lives in `mod.rs`).

**New app state fields** (parallel to `mouse_alarm_held` / `alarm_delay_token`):
- `timeout_revive_held: Option<GameColor>` — which team's button is currently being held
  (`None` = not holding).
- `timeout_revive_token: u64` — incremented on each new press to cancel stale hold timers.

**New `Message` variants** (`message.rs`):
- `TimeoutRevivePressed(GameColor)` — emitted on press-down of an eligible greyed button.
- `TimeoutReviveReleased(GameColor)` — emitted on release.
- `TimeoutReviveHoldElapsed(u64, GameColor)` — fired by the 5-second async timer.
- `TimeoutReviveLockoutElapsed(u64, GameColor)` — fired by the 2-second safety timer (see 5c).

**View change** (`build_timeout_ribbon` in `shared_elements.rs`): when the
"hold-to-revive available" precondition (Section 3) holds, wrap the greyed timeout button in a
`mouse_area` (the same widget the alarm face uses) with `on_press(TimeoutRevivePressed(color))`
and `on_release(TimeoutReviveReleased(color))`. While `timeout_revive_held == Some(color)`,
render the button with a brightened style (toward its active colour). The exact armed shade is a
minor visual detail to settle during implementation; reuse an existing style if a suitable one
exists.

**Update handlers** (`mod.rs`), mirroring the alarm handlers:
- `TimeoutRevivePressed(color)`: set `timeout_revive_held = Some(color)`, bump
  `timeout_revive_token`, and spawn a `Task::future(sleep(HOLD_DURATION) ->
  TimeoutReviveHoldElapsed(token, color))`.
- `TimeoutReviveReleased(color)`: if held, clear `timeout_revive_held = None`. (The bumped token
  ensures a late hold timer no-ops.)
- `TimeoutReviveHoldElapsed(token, color)`: only proceed if `token == timeout_revive_token`
  **and** `timeout_revive_held == Some(color)` **and** the revive precondition (Section 3) still
  holds (re-validated against current state). If so, call `tm.revive_team_timeout(color)`, log it
  (`info!`, consistent with the alarm logging), regenerate the snapshot, apply it, clear the held
  state, and **start the 2-second safety lockout** (5c). Otherwise do nothing.

**Constants:** `TIMEOUT_REVIVE_HOLD_DURATION = 5s`, `TIMEOUT_REVIVE_LOCKOUT_DURATION = 2s`.

### 5c. The 2-second safety lockout

Goal: immediately after a revive, the just-revived team's button must not register any input for
2 seconds, so the lingering hold-press (or its release) cannot start a timeout.

- On a successful revive, record the lockout — e.g. `timeout_revive_lockout: Option<GameColor>`
  plus a dedicated token — and spawn `Task::future(sleep(2s) -> TimeoutReviveLockoutElapsed(token,
  color))`.
- While the lockout is active for `color`, the view renders that team's button in its **normal
  active style but inert**: it does nothing on press. Use the established no-op press pattern
  (`.on_press(Message::NoAction)`) so the button keeps its active appearance (rather than
  flickering back to a greyed/disabled look) while ignoring taps and holds.
- `TimeoutReviveLockoutElapsed(token, color)`: if the token matches, clear the lockout and let
  the view re-render the button as fully active and pressable.

This keeps the button looking active throughout (active → 2s inert-but-active → active &
pressable), with no greyed flicker.

## 6. Edge cases

- **Release before 5s:** held state cleared on release; the stale token makes any late hold timer
  a no-op. No change.
- **Game state changes mid-hold** (e.g. half ends and per-half reset zeroes the counts, or a
  timeout starts): at the 5-second mark the precondition is re-validated and `revive_team_timeout`
  re-checks `timeouts_used > 0`. If reviving no longer applies, nothing happens (safe).
- **Switching which button is held:** a new `TimeoutRevivePressed` bumps the token and replaces
  `timeout_revive_held`, so only the most recent hold can complete (mirrors the alarm behaviour).
- **`num_team_timeouts_allowed == 0`:** condition (2) (`timeouts_used > 0`) prevents offering a
  revive when nothing was ever used.
- **Multiple allowed timeouts (cap > 1):** one successful hold returns exactly one; the button
  becomes active again at one-below-cap, after which a normal tap starts a timeout. Further
  revives are only offered if it greys out again.

## 7. Testing & verification

**Automated (unit, on the state-machine method):**
- Consume a team timeout → assert `can_start_team_timeout` now errors (button would grey) →
  `revive_team_timeout` → assert `can_start_team_timeout` is `Ok` again and the count dropped by
  one.
- `revive_team_timeout` with nothing used → returns the error, count stays at zero (saturating).
- Sanity: per-half / per-game reset behaviour is unchanged by the new method.

**Manual walkthrough (operator drives the running refbox):**
- Start a team timeout → confirm the button greys.
- Hold the greyed button ~5s → confirm it returns to active.
- Immediately try to tap it → confirm nothing happens for ~2s (safety period), then a fresh tap
  starts a timeout normally.
- Hold ~2s then release → confirm nothing changes.
- Confirm the hold does nothing while an opponent/ref timeout is running ("Switch to…" state).

**Gate:** `just check` (format, lint, tests, security scan) clean before any PR.

## 8. Files touched

| File | Change |
|------|--------|
| `refbox/src/tournament_manager/mod.rs` | New `revive_team_timeout` method (+ error variant) + unit tests. **(state-machine — heavy care)** |
| `refbox/src/app/message.rs` | New `Message` variants (Pressed / Released / HoldElapsed / LockoutElapsed). |
| `refbox/src/app/mod.rs` | New state fields, four message handlers, two duration constants. |
| `refbox/src/app/view_builders/shared_elements.rs` | Make the eligible greyed button hold-sensitive (`mouse_area`), brighten while held, render inert-but-active during the lockout. |
| `refbox/src/app/theme/button.rs` *(maybe)* | An "armed/held" brighter style, if no existing style is reusable. |

## 9. Decisions & assumptions log

- **Use case:** rare correction → guarded, low-discoverability design. *(confirmed)*
- **Hold duration:** 5 seconds. *(confirmed)*
- **Confirmation:** none — the long hold is the only guard. *(confirmed)*
- **Hold feedback:** brighten only; no countdown, no fill, no completion flash. *(confirmed)*
- **Safety period after revive:** 2 seconds of inert-but-active button. *(confirmed)*
- **Revive amount:** exactly one timeout per successful hold. *(assumed; confirmed by silence)*
- **Availability scope:** only when no timeout is running. *(confirmed by silence)*
- **Always-on, no setting.** *(confirmed by silence)*
- **Approach:** reuse the manual-alarm press-and-hold machinery. *(confirmed)*
- **Doc handling:** this spec is a local working document (consistent with the repo's other
  `docs/superpowers/specs/` files) and is **not** committed to a feature branch or PR.
