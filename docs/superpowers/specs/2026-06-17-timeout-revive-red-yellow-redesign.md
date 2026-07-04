# Timeout Revive — Red→Yellow Redesign (v2) — Design

**Date:** 2026-06-17
**Status:** Approved (design); supersedes the post-revive behaviour in `2026-06-16-timeout-revive-long-press-design.md`
**Crate scope:** `refbox` only (app layer + button theme)
**Branch:** `feat/refbox/timeout-revive-long-press` (worktree `.claude/worktrees/feat+refbox+timeout-revive-long-press`)
**Process:** heavy care for the interaction state machine (timing + multiple states); the underlying game-state actions are reused unchanged.

---

## 1. Summary

This refines the just-built "hold a used-up team timeout button to revive it" feature. The v1 build (on the branch: commits `b926166e`, `7b6d9269`, `4d92f4fb`) brightened the button black/white while held, revived at 5 s, then made the button **inert** for a 2-second safety lockout. v2 changes both the colours and the post-revive behaviour:

- While held during the revive (0–5 s): the button is **RED**.
- At 5 s the timeout is given back and the button turns **YELLOW** while the finger stays down.
- **Releasing during the yellow window banks** the timeout (button → normal black/white available).
- **Holding through the yellow window (~2 s) starts a team timeout** for that team (reusing the normal start action), spending the just-revived timeout. Net effect of "revive then hold-through": the used-count returns to where it was, but a timeout is now legitimately running — i.e. "give it back **and** take it now," in one continuous press.

The state-machine method `revive_team_timeout` (v1 Task 1) and the existing `start_team_timeout` action are reused unchanged. No `uwh-common` / wire-format / hardware changes.

## 2. Behaviour specification (operator-facing contract)

State progression of a team's timeout button:

| Situation | Appearance | Notes |
|-----------|------------|-------|
| Team has used its timeout(s) | **greyed** (existing "used-up" look) | unchanged |
| Operator presses & holds the greyed button | **RED** | "reviving"; up to 5 s |
| Hold reaches 5 s | timeout given back; button turns **YELLOW** | shown only while the finger stays down |
| Release (or slide off) during the **RED** phase | back to **greyed** | cancelled; nothing given back |
| Release (or slide off) during the **YELLOW** phase | **black/white** (normal available) | banked; revive stands; no timeout started |
| Keep holding ~2 s through the **YELLOW** phase | team timeout **starts** → normal running **"End Timeout"** view | spends the revived timeout |

Key points:
- **Colours:** RED = reviving in progress; YELLOW = revived, "release to bank / hold to start"; black/white = banked/available (or normal).
- **Slide-off safety:** in the yellow window, moving the pointer off the button **banks** it (does not start). To start, the pointer stays on the button for the full ~2 s.
- **Durations:** 5 s reviving hold, ~2 s deciding window. (Same constants as v1; the 2 s is repurposed from "inert lockout" to "decide window".)
- **Amount:** revive gives back exactly one timeout (unchanged); the hold-through start consumes exactly one (the normal start action).
- **Availability** of the whole gesture is unchanged from v1: only when no timeout is currently running and the button is greyed *because the team is used-up* during First/Second half (governed by the existing `can_revive_team_timeout`).

## 3. Design / architecture (Approach A: a small two-phase state machine)

All changes are in the `refbox` app layer + button theme. The game-rule method (`revive_team_timeout`) and the start action (`start_team_timeout`) are reused.

### 3a. Interaction state (`refbox/src/app/mod.rs`)

Replace v1's four scattered fields (`timeout_revive_held`, `timeout_revive_token`, `timeout_revive_lockout`, `timeout_revive_lockout_token`) with a single consolidated value:

- `timeout_revive: Option<ReviveHold>` where `ReviveHold` carries `{ color: Color, phase: RevivePhase, token: u64 }`.
- `enum RevivePhase { Reviving, Deciding }` (Copy). Defined in `app/mod.rs` and made reachable to the view builder (it is read by `build_timeout_ribbon`).
- `token: u64` guards against stale async timers (mirrors the v1 alarm-token pattern); bumped on each press and on the Reviving→Deciding transition.

Constants: `TIMEOUT_REVIVE_HOLD_DURATION = 5 s` (reviving), `TIMEOUT_REVIVE_DECIDE_DURATION = 2 s` (deciding; renamed from the v1 lockout constant).

### 3b. Messages (`refbox/src/app/message.rs`)

Four variants (reuse the v1 names where possible; the 4th is renamed):
- `TimeoutRevivePressed(GameColor)` — finger down on a revive-eligible greyed button.
- `TimeoutReviveReleased(GameColor)` — finger up **or** pointer left the button (both `on_release` and `on_exit` map here).
- `TimeoutReviveHoldElapsed(u64, GameColor)` — the 5 s reviving timer fired.
- `TimeoutReviveDecideElapsed(u64, GameColor)` — the 2 s deciding timer fired (replaces v1 `TimeoutReviveLockoutElapsed`).

Manual `is_repeatable` and `PartialEq` arms updated to match (same shape as v1).

### 3c. Handlers (`update()` in `app/mod.rs`)

- **Pressed(color):** if a hold is already active for this color, ignore. Else set `timeout_revive = Some({color, Reviving, token})` (bump token) and schedule `sleep(HOLD) → TimeoutReviveHoldElapsed(token, color)`.
- **Released(color):** if a hold is active for this color, clear it (`timeout_revive = None`). In *Reviving* this cancels (nothing given back); in *Deciding* this banks (the revive already happened; the button becomes normal black/white).
- **HoldElapsed(token, color):** only if `timeout_revive` is still `Some` in *Reviving* for this color **and** token matches. Lock the TM and call `revive_team_timeout(color)`:
  - `Ok` → generate + apply snapshot; transition to `Deciding` (bump token); schedule `sleep(DECIDE) → TimeoutReviveDecideElapsed(newtoken, color)`; return `Task::batch([apply, decide_timer])`.
  - `Err` (state moved on, e.g. half ended) → clear `timeout_revive`; no-op.
- **DecideElapsed(token, color):** only if `timeout_revive` is still `Some` in *Deciding* for this color **and** token matches (i.e. still held). Lock the TM and call `start_team_timeout(color, now)`:
  - `Ok` → generate + apply snapshot; clear `timeout_revive` (button → running "End Timeout").
  - `Err` → clear `timeout_revive`; no-op.

Lock discipline mirrors the existing `Message::TeamTimeout` handler (lock → mutate → snapshot → drop → `apply_snapshot`).

### 3d. View (`build_timeout_ribbon` in `view_builders/shared_elements.rs`)

Signature: replace `revive_held` / `revive_lockout` params with `revive_hold: Option<(GameColor, RevivePhase)>` (the active hold, if any).

For each team colour C, in the `None` (no timeout running) arm — **keep the `mouse_area` stable** (same `on_press(TimeoutRevivePressed(C))` / `on_release` / `on_exit(→ TimeoutReviveReleased(C))` handlers and tree position across phases) so the continuous press keeps being tracked; only the **inner button's `.style()` changes**:
- If the active hold is `(C, Deciding)` → inner style **`yellow_button_armed`**.
- Else if `tm.can_revive_team_timeout(C).is_ok()` → inner style is **`red_button_armed`** when the active hold is `(C, Reviving)`, otherwise the team's normal greyed style (`black_button`/`white_button` with no inner `on_press`, which renders greyed).
- Else → the normal `on_press_maybe(can_start_team_timeout(C)…)` button (start, or greyed for another reason).

(During *Deciding*, `can_revive_team_timeout(C)` is false because the count was already decremented, so the explicit `Deciding` check must come first.)

The `referee`/`penalty` buttons and the final `row!` are unchanged. `mouse_area` is already imported (v1). The inner button in the hold branches has **no `on_press`** so the `mouse_area` reliably captures the press (the v1 pattern).

### 3e. Theme (`app/theme/button.rs` + `theme/mod.rs`)

- **Add** `red_button_armed(theme, _status) -> Style { red_button(theme, Status::Active) }` and `yellow_button_armed(theme, _status) -> Style { yellow_button(theme, Status::Active) }` — force the bright/active colour on the non-interactive held button (same trick as v1's `*_button_armed`). Both inherit `red_button`/`yellow_button`'s High-Contrast handling (HC renders an outline with the colour as accent — already tested for red).
- **Remove** the now-unused `black_button_armed` / `white_button_armed` (and their `theme/mod.rs` re-exports); **add** the two new re-exports. Leaving the old ones would fail `-D warnings` (never used).

## 4. Edge cases

- **Release before 5 s** (Reviving) → cancel; greyed; nothing given back.
- **Release / slide-off during the yellow window** (Deciding) → bank; button black/white; revive stands; no timeout started.
- **Hold through the yellow window** → `start_team_timeout`; button → End Timeout. (Revive −1 then start +1 ⇒ used-count returns to its pre-revive value with a timeout now running.)
- **Game state changes mid-hold** (e.g. half ends): `HoldElapsed`/`DecideElapsed` re-validate by calling `revive_team_timeout` / `start_team_timeout`, which return `Err` if no longer valid → clear the hold, no-op.
- **Stale timers / rapid re-press**: guarded by the token + phase check (a timer fires only if `timeout_revive` is still the matching phase/token).
- **`mouse_area` press tracking across the colour swap**: because the `mouse_area` stays in the same tree position with the same handlers and only the inner `.style()` changes, iced preserves its press/hover state — confirm with a quick check early in implementation (low risk).

## 5. Testing & verification

- **Automated:** `revive_team_timeout` (v1 Task 1) and `start_team_timeout` already have unit tests; both are reused unchanged. The interaction (phases/timers/colours) is iced app-layer and is not unit-tested. Gate: `just check` (fmt + workspace clippy `-D warnings` + tests + audit) clean.
- **Manual walkthrough** (operator drives the running refbox):
  - Use a team's timeout so its button greys.
  - Press & hold → **red**; at ~5 s → **yellow**.
  - Release during yellow → button goes **black/white** (banked, available); confirm no timeout started.
  - Repeat; this time **hold through** the yellow window → a team timeout **starts** (End Timeout view).
  - Release during the **red** phase (before 5 s) → back to greyed, nothing given back.
  - During yellow, **slide the pointer off** the button → banks (no start).

## 6. Files touched

| File | Change |
|------|--------|
| `refbox/src/app/theme/button.rs` | +`red_button_armed`, +`yellow_button_armed`; −`black_button_armed`, −`white_button_armed` |
| `refbox/src/app/theme/mod.rs` | re-export update (drop black/white-armed, add red/yellow-armed) |
| `refbox/src/app/message.rs` | rename 4th variant to `TimeoutReviveDecideElapsed`; keep the other three; update `is_repeatable` + `PartialEq` |
| `refbox/src/app/mod.rs` | `RevivePhase` enum + consolidated `timeout_revive` field (replacing v1's four fields); rename the 2 s constant; rewrite the four handlers per §3c; update the `build_timeout_ribbon` call site |
| `refbox/src/app/view_builders/shared_elements.rs` | signature param → `revive_hold: Option<(GameColor, RevivePhase)>`; rework the black/white `None`-arm per §3d |

No change to `tournament_manager/` (the revive + start methods already exist), `uwh-common`, or any other crate.

## 7. Decisions & assumptions log

- Colours: **red** while reviving (0–5 s), **yellow** while deciding (held, 5–7 s), black/white when banked. *(confirmed)*
- Yellow shows **only while the finger is down**; releasing resolves immediately (bank). *(confirmed — "just during the time the user still has it pressed")*
- Hold-through the yellow window **starts** the team timeout (revive-and-use in one continuous press); release within it banks. *(confirmed)*
- Slide-off during yellow **banks** (does not start). *(design choice, surfaced and accepted)*
- 5 s hold + 2 s decide durations retained. *(unchanged from v1)*
- Implemented as a two-phase state machine (Approach A), reusing `revive_team_timeout` and `start_team_timeout`; v1's brighten styles and inert lockout are removed. *(confirmed)*
- This supersedes the v1 post-revive behaviour; the change is additional commits on the same branch (v1 commits are not yet in a PR).
- Yellow is also the Ref-Timeout/"End Timeout" colour; during the yellow window the button still reads "[TEAM] TIMEOUT" and is only yellow while held, so it reads as distinct. *(accepted)*
