# Team-timeout 15-second grace window (Cancel → End) + ref/penalty cancel relabel

**Date:** 2026-06-16
**Crate:** `refbox` (UI + `tournament_manager` state machine)
**Process:** **Heavy** — touches the game timing-and-state engine (timeout charging + clock resume), which is golden-trace-guarded.
**Status:** Design approved in chat 2026-06-16. **Supersedes** `2026-06-16-cancel-timeout-button-rename-orange-design.md` (that was a rename-only design based on a misunderstanding; the real feature is below).

---

## Goal

Give **team** timeouts a 15-second "grace" window with a two-phase button, and relabel the
ref-timeout / penalty-shot cancel buttons.

- **Team timeout, first 15 s:** button reads **"Cancel Timeout" (orange)**. Pressing it — or
  switching to the other team — **undoes** the timeout: the game clock resumes and the team is
  **not charged** its timeout.
- **Team timeout, after 15 s:** button reads **"End Timeout" (red)**; the "Switch to <other team>"
  button reverts to a disabled **"<other> Timeout"** (no more switching).
- **Ref timeout / penalty shot:** button relabelled to **"Cancel Ref Timeout" / "Cancel Pen Shot"
  (orange)** the whole time — **no** time flip, **no** new undo behaviour (ending those already
  costs a team nothing).

## Scope boundary

In scope: `refbox/src/tournament_manager/mod.rs` (one new method), `refbox/src/app/message.rs`
(one new message), `refbox/src/app/mod.rs` (one handler), the two view builders
(`shared_elements.rs`, `main_view.rs`), and the 15 locale files.

Out of scope:
- The grace window / undo for **ref timeouts and penalty shots** (label + colour only).
- Changing *which* timeout-type switches the rules allow (the type-level `can_switch_*` rules stay).
- Restoring elapsed **game-clock** time on cancel — we only **resume** the clock (per the user).
- LED panel and stream overlay.

---

## Current behaviour (verified in code)

- A team timeout **charges the team immediately on start**: `timeouts_used[color] += 1`
  ([mod.rs:398](refbox/src/tournament_manager/mod.rs#L398)). It counts **down** from
  `config.team_timeout_duration`, so the snapshot value is **remaining** seconds.
- **Ending** a team timeout already **resumes the game clock** (`start_game_clock`)
  ([mod.rs:541](refbox/src/tournament_manager/mod.rs#L541)).
- **Switching** to the other team already **refunds** the first team
  ([mod.rs:458-460](refbox/src/tournament_manager/mod.rs#L458-L460)).
- The golden-trace guard tracks period/clock/score/**timeout snapshot**/conf-pause/penalties —
  **not** `timeouts_used`. Refunding a timeout therefore cannot change a golden trace.

## Chosen approach

**A — charge then refund.** Keep charging on start (unchanged). Add a separate
`cancel_team_timeout` that resumes the clock and refunds the charge. **Leave `end_timeout`
untouched** so the existing/normal path and its golden traces are undisturbed. (Rejected B —
"charge only at 15 s" — because it adds time-triggered mutation inside the clock engine, the
highest-risk area, for no operator-visible difference.)

The 15-second cutoff is a hard-coded `const GRACE_WINDOW: Duration = Duration::from_secs(15)`
(not config — YAGNI).

---

## Behaviour model

While a **team** timeout is running, with `elapsed = team_timeout_duration − remaining`:

```
elapsed < 15 s (grace):     active slot   = "CANCEL"/"TIMEOUT"  orange, press → CancelTimeout
                            other team    = "SWITCH TO <other>" enabled (if team has a TO left)
elapsed ≥ 15 s (locked):    active slot   = "END"/"TIMEOUT"     red,    press → EndTimeout
                            other team    = "<other> TIMEOUT"   greyed/disabled
(ref & penalty slots during a team timeout stay greyed with their own names — the rules don't
 allow switching team→ref or team→penalty; this is the existing honest-label behaviour.)
```

Ref-timeout slot (whole duration): `"CANCEL REF"/"TIMEOUT"` orange, press → `EndTimeout`.
Penalty-shot slot (whole duration): `"CANCEL PEN"/"SHOT"` orange, press → `EndTimeout`.

Center single-line button (shown when "track fouls & warnings" is off, during any timeout)
mirrors the same text/colour, single-line: `CANCEL TIMEOUT` / `END TIMEOUT` (team, by phase),
`CANCEL REF TIMEOUT`, `CANCEL PEN SHOT`.

### Decisions (approved)
1. **Switching does not restart the 15-second clock** — the window runs from the original start
   (switch keeps the same timeout clock).
2. **Center button mirrors** the team-timeout flip and shows the ref/penalty cancel labels.
3. **"End Timeout" returns** as the after-15 s team label; "Cancel …" labels are new keys.

### Edge cases
- **Boundary/rounding:** the snapshot is whole seconds (truncated), so the flip lands within ~1 s
  of the true 15 s. Acceptable; the cancel/end decision and the label are both derived from the
  same elapsed value so they always agree.
- **Team timeout started while the game clock was stopped** (`ClockState::Stopped`, rare): the
  timeout clock doesn't advance, so elapsed stays 0 and the button stays "Cancel". Acceptable —
  cancel still just clears the timeout and refunds; there is no charged-and-stuck state.

---

## Architecture / changes

### `tournament_manager/mod.rs` — new method (end_timeout untouched)
```rust
/// Cancel a team timeout within the grace window: resume the game clock and
/// refund the team its timeout. Mirrors end_timeout's team branch + a refund.
pub fn cancel_team_timeout(&mut self, now: Instant) -> Result<()> {
    match &self.timeout_state {
        Some(TimeoutState::Team(color, cs)) => {
            let color = *color;
            match cs {
                ClockState::Stopped { .. } => self.timeout_state = None,
                ClockState::CountingDown { .. } => {
                    self.start_game_clock(now);
                    self.timeout_state = None;
                }
                ClockState::CountingUp { .. } => return Err(TournamentManagerError::InvalidState),
            }
            self.timeouts_used[color] = self.timeouts_used[color].saturating_sub(1);
            Ok(())
        }
        _ => Err(TournamentManagerError::NotInTimeout),
    }
}
```

### `app/message.rs` — new variant
`Message::CancelTimeout` (team-only cancel within grace). Wire it into the same `PartialEq`/
match plumbing the existing `EndTimeout` uses.

### `app/mod.rs` — handler
`Message::CancelTimeout` → `tm.cancel_team_timeout(now)`, then regenerate/apply snapshot like the
`EndTimeout` handler. (Cancelling mid-play never ends the game, so the `EndTimeout` handler's
"would end the game" check is not needed here.)

### `app/view_builders/shared_elements.rs` — `build_timeout_ribbon`
- Add a grace helper: `within_grace = team_timeout_duration.saturating_sub(remaining) < 15 s`,
  using `tm.config.team_timeout_duration` (tm is already locked here) and the slot's remaining secs.
- **Active team slot:** `within_grace` → `("cancel-timeout-line-1","…-2")` + `orange_button` +
  `Message::CancelTimeout`; else → `("end-timeout-line-1","…-2")` + `red_button` +
  `Message::EndTimeout`.
- **Other team slot** (during a team timeout): `can_switch_to_team_timeout(other).is_ok() &&
  within_grace` → "Switch to <other>" enabled; else → that team's own start label, disabled
  (the honest-label pattern).
- **Ref slot active:** `("cancel-ref-timeout-line-1","…-2")` + `orange_button` + `EndTimeout`.
- **Penalty slot active:** `("cancel-pen-shot-line-1","…-2")` + `orange_button` + `EndTimeout`.
- The other slots' honest-label behaviour (show the start label disabled when a switch isn't
  allowed) stays as designed.

### `app/view_builders/main_view.rs` — center button
Pick label/colour/message from `snapshot.timeout`: team + grace → `cancel-timeout`/orange/Cancel;
team + locked → `end-timeout`/red/End; ref → `cancel-ref-timeout`/orange/End; penalty →
`cancel-pen-shot`/orange/End.

### Translations (15 locales, no English placeholders)
- **Restore** `end-timeout = END TIMEOUT` and `end-timeout-line-1 = END` (the earlier rename is
  reverted; these are now the after-15 s team labels).
- **Add** (reusing each locale's existing `cancel`/`timeout`/`ref`/`pen-shot` words where natural):
  - `cancel-timeout` (center) + `cancel-timeout-line-1`/`-2`  → "CANCEL" / `{ timeout }`
  - `cancel-ref-timeout` (center) + `cancel-ref-timeout-line-1`/`-2` → "CANCEL REF" / `{ timeout }`
  - `cancel-pen-shot` (center) + `cancel-pen-shot-line-1`/`-2` → "CANCEL PEN" / "SHOT"
  Exact per-locale wording is gathered from the existing keys during implementation (as for the
  earlier rename), confirmed by diffing each locale against en-US so none is left in English.

---

## Golden-trace / safety-guard impact

- `cancel_team_timeout` resumes the clock and clears the timeout — **identical** to `end_timeout`
  for the fields the guard tracks (clock, timeout snapshot, score). The refund touches
  `timeouts_used`, which the guard does **not** track. **No existing golden trace changes.**
- `end_timeout` is **not modified**, so all existing end-timeout traces are byte-for-byte stable.
- **Add** golden scenario(s) for the new path (start team timeout → cancel within 15 s → clock
  resumes) to lock the behaviour in, blessed with `UPDATE_GOLDEN=1`. Adding new scenarios does not
  alter existing baselines.

## Acceptance criteria
- Team timeout: button is "Cancel Timeout" (orange) for the first 15 s and "End Timeout" (red)
  after; pressing Cancel within 15 s resumes the clock and the team's timeout count is **not**
  reduced (refunded); after 15 s the team's timeout stays used.
- "Switch to <other team>" is enabled only within the 15 s window; after, it shows "<other>
  Timeout" disabled.
- Ref / penalty buttons read "Cancel Ref Timeout" / "Cancel Pen Shot" (orange) throughout, and
  ending them behaves exactly as today.
- All 15 locales updated, no English placeholders.
- `just check` clean; new golden scenario(s) pass; existing golden traces unchanged.

## How to verify (operator-visible)
1. Start a team timeout; within 15 s the button is orange "Cancel Timeout" and the other team
   shows "Switch to …" enabled. Press Cancel → game resumes and that team still has its timeout
   available (start another to confirm the count wasn't used).
2. Start a team timeout, wait past 15 s; button turns red "End Timeout" and the other team's slot
   shows "<other> Timeout" greyed. Press End → game resumes and the timeout counts as used.
3. Start a ref timeout → "Cancel Ref Timeout" orange throughout; penalty shot → "Cancel Pen Shot"
   orange throughout.
4. With "track fouls & warnings" off, the center button shows the same text/colour per phase/type.
