# Design: Game Block "reverts on reopen" (Bug B)

- **Date:** 2026-06-16 (outcome 2026-06-17)
- **Branch:** `fix/refbox/param-editor-reopen-value` (worktree off `origin/master`)
- **Crate scope:** `refbox` only
- **Status:** RESOLVED — see Outcome below. The "Keep Game and Apply saves Game Block"
  design originally approved here was **abandoned during manual verification** and replaced
  with a narrower fix.

## OUTCOME (2026-06-17 — supersedes the design below)

Manual verification revealed the design's central assumption was wrong: the mid-game
config-change dialog (`GameConfigChangedFromApply`,
[confirmation.rs:52-68](../../../refbox/src/app/view_builders/confirmation.rs#L52-L68))
offers only **Go Back / Discard / End Current Game and Apply** — there is **no "Keep Game and
Apply" button**. That button exists only on the *game-number*-change dialog
(`GameNumberChangedFromApply`). So the originally-approved fix (make `KeepGameAndApply` commit
Game Block, plus a `set_game_block` engine method) wired up a path this dialog never reaches.

**Decisions made with the user:**
1. **Apply behavior: accept current behavior.** Any mid-game game-parameter change (Game Block
   included) requires "End Current Game and Apply." It is infrequent and acceptable. No
   keep-game path, no special-casing Game Block, no skipping the dialog. The engine changes
   (`set_game_block` + its test, and the `KeepGameAndApply` commit) were **reverted**.
2. **Keypad reopen glitch: fixed.** A separate, genuine bug — likely the real "reverts on
   reopen" symptom — is that the parameter editor seeded its keypad from the last-saved config
   (`self.config.game`) instead of the in-progress edits (`edited_settings.config`). Fixed in
   `Message::EditParameter` ([app/mod.rs ~2835](../../../refbox/src/app/mod.rs#L2835)) to read
   from `edited_settings` when present, mirroring the adjacent `single_half` read. Applies to
   all length parameters.

**Final change:** one file, `refbox/src/app/mod.rs` (the keypad seed). No engine/test changes.
No automated test added — the app/UI `update()` layer has no test harness in this codebase
(user agreed manual verification is sufficient). Golden-trace guard re-run: `golden_traces_match
_baseline ... ok` (zero drift — the time engine is unchanged).

---

_The design below is retained for history; it is NOT what shipped._

## Problem (plain English)

When you edit **Game Block** during a live game and press **Apply**, the refbox shows the
"this changes the game — *End Game and Apply* / *Keep Game and Apply*?" safety popup. If you
pick **Keep Game and Apply**, your Game Block change is silently thrown away. When you reopen
the editor, it shows the old number.

Two things combine to cause this:

1. **The "Keep Game and Apply" button discards your edit.** It only updates the game *number*
   and never saves the edited settings (`app/mod.rs` ~1094). This affects *every* setting, not
   just Game Block — Game Block is simply where it was noticed.
2. **The settings editor reads its starting values from the game engine's copy of the config**
   (`app/mod.rs:2388`, built from `tm.config()`), which "Keep Game and Apply" never updates — so
   on reopen you see the old value. A related drift: the parameter keypad seeds its initial value
   from `self.config.game` (`app/mod.rs:2844-2853`) while edits are saved to
   `edited_settings.config` (`app/mod.rs:2907-2918`), so even reopening the keypad mid-session
   shows the stale stored value.

## Why Game Block is safe to save mid-game

Game Block is a **scheduling value** — it only determines when the *next* game starts. In the
tournament manager it is consumed solely at game start (`tournament_manager/mod.rs:1081`,
`next_scheduled_start = sched_start + game_block`). It never touches the running clock. So it can
be saved during a game without disturbing the current game.

By contrast, **clock settings** (half length, timeout duration, overtime, etc.) *cannot* be
applied to a game already running — doing so would corrupt the live clock. That is the entire
reason the End/Keep popup exists, and why `tm.set_config()` (`tournament_manager/mod.rs:154`) is
gated to `BetweenGames` only.

## Design (confirmed)

1. **Keep the popup for every mid-game settings change — no special-casing of Game Block.**
   Consistent, predictable flow (matches the user's preference for predictable UI over
   conditional popups).
2. **Fix "Keep Game and Apply" so it saves the Game Block change** (the value that is safe to
   apply forward) instead of discarding it. The current game keeps playing; Game Block takes
   effect for the next game's slot.
3. **Fix the keypad drift** so reopening shows your in-progress edit, not the old stored value.
   (This mirrors how `single_half` is already read, and fixes the in-session revert for all
   length parameters.)

### Boundary — explicitly out of scope

- **Clock-affecting settings keep today's behavior under "Keep Game and Apply":** the game you
  chose to keep runs on its current settings. To change those you choose "End Game and Apply."
  We are *not* redefining what "Keep Game" does to a live clock, and *not* saving clock-setting
  edits for the next game under Keep-Game.
- **We are not removing the popup for pure Game Block edits** (consistency over conditional UI).
- **We are not extending this to other scheduling fields** (Minimum Break, Nominal Break,
  Post-Game). Game Block only, per the reported bug. (Possible future follow-up.)
- **We are not recomputing the current game's already-scheduled next-start.** Game Block edited
  mid-game takes effect for future games — matching the existing "Game Block is authoritative,
  locked at game start" behavior the user previously confirmed.

## Implementation sketch (refbox crate only)

### A. New tournament-manager method — `tournament_manager/mod.rs`

```rust
/// Update the Game Block (the start-to-start slot for the next game).
/// Unlike `set_config`, this is NOT gated to BetweenGames: `game_block` only
/// feeds the next-game-start computation (consumed at game start), so changing
/// it mid-game is safe and does not touch the running clock or current period.
/// Does not recompute the current `next_scheduled_start`, which was locked when
/// this game started (Game Block is authoritative / locked at game start).
pub fn set_game_block(&mut self, game_block: Duration) {
    self.config.game_block = game_block;
}
```

### B. Fix the `KeepGameAndApply` arm — `app/mod.rs` ~1094

After `tm.set_game_number(&edited.game_number)`, when `new_config` is present:

- `tm.set_game_block(config.game_block)` — so the engine's config (the editor's source of truth)
  reflects the new value → no revert.
- `self.config.game.game_block = config.game_block` — so the persisted config matches.

Only `game_block` is copied; clock fields are intentionally left as the running game's. (`new_config`
is borrowed here, e.g. `if let Some(ref config) = new_config`, since `game_block` is `Copy`.)

### C. Fix the `EditParameter` keypad seed — `app/mod.rs` ~2835

Read the initial duration (and `single_half`) from `edited_settings.config` when present, falling
back to `self.config.game` — mirroring the existing `single_half` read at lines 2836-2840. Applies
to all `LengthParameter` variants.

## Testing (TDD — failing test first)

- **tournament_manager unit test:** `set_game_block` updates `config().game_block` while a game is
  in progress (`current_period != BetweenGames`) without changing `current_period` or clock state,
  and without altering `next_scheduled_start`.
- **App-level behavior:** if `app/mod.rs` exposes a unit-testable path for the confirmation flow,
  assert that the `KeepGameAndApply` path commits `game_block` to both `tm.config()` and
  `self.config.game`. If `update()` is not unit-testable in isolation, document the manual steps
  below as the verification of record.
- **Manual verification:** mid-game, change Game Block, Apply, pick "Keep Game and Apply," reopen
  editor → new value shown. Also reopen the keypad immediately after editing (without applying) →
  in-progress value shown. Confirm the running clock is undisturbed.

## Acceptance criteria (operator-observable)

1. During a live game, change Game Block, press Apply, pick **Keep Game and Apply**. Reopen
   Settings → Game Options → Game Block shows the **new** value (not the old one).
2. During a live game, open the Game Block keypad, type a new value, confirm, then reopen the
   keypad → it shows the value you just entered.
3. The current game's clock is unaffected by the Game Block change.
4. `just check` passes (fmt, lint, tests, audit).
