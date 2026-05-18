# Beep-Test Absorption Into Refbox — Design

**Date:** 2026-05-18
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch (when work begins):** `feat/refbox/beep-test-mode`

---

## Goal

Eliminate the standalone `beep-test/` crate by absorbing its functionality into `refbox` as a new
operating mode, alongside the existing Hockey 6v6, Hockey 3v3, and Rugby modes. After this work,
the workspace contains one operator-facing application binary that can be switched between
game-management and beep-test functions via the same Configuration page and restart pattern that
already governs Hockey↔Rugby switching.

---

## Motivation

### Why absorb at all

The standalone `beep-test/` crate is 2,557 lines and exists almost entirely as parallel copies of
code already in `refbox/`:

| Component | beep-test lines | refbox lines | Relationship |
|-----------|-----------------|--------------|--------------|
| `sim_app/mod.rs` | 288 | 288 | Byte-identical |
| `sim_app/display_simulator.rs` | 74 | 74 | Byte-identical |
| `sound_controller/sounds.rs` | 132 | 132 | Byte-identical |
| `sound_controller/mod.rs` | 553 | 903 | beep-test is a subset of refbox's |
| `app/update_sender.rs` | 695 | 871 | Same shape, different snapshot type |
| `tournament_manager/` (cadence engine) | 411 | 7,748 | Genuinely different purpose |

Pure duplication accounts for 494 lines. Subset duplication accounts for ~1,250 lines. Only the
411-line cadence engine in `beep-test/src/tournament_manager/` is logic refbox doesn't already
have.

### Why now

The standalone `beep-test` binary has no current independent use case — it is used only as a
sound-system test before or during refbox sessions, not as a standalone tool away from refbox.
That makes absorption strictly better than the alternative of keeping two binaries:

- One binary to install and version-manage on Raspberry Pi deployments.
- A single source of truth for shared infrastructure (`sim_app`, `sound_controller`,
  `update_sender`), eliminating the silent drift between duplicated files.
- Net workspace code reduction.

### Prior attempt and why it was the wrong shape

Three commits on the unmerged branches `Scoresheets` and `high-contrast-ui` from September 2025
(`91d7334`, `fe8bdad`, `1a17e21`) attempted a related integration. Those commits copied 197 lines
of new view_builder and 182 lines of config types into refbox while leaving the standalone
`beep-test/` crate alive — producing duplication rather than removing it. The work was tangled
with other in-flight experiments on those branches and never landed.

This spec supersedes that direction. The work described here **deletes** the standalone crate
rather than copying from it.

---

## Scope

### Crates touched

- **Primary:** `refbox` — all functional changes
- **Workspace-level:** root `Cargo.toml` (remove member), `justfile` (remove recipes if any),
  CI workflow files (remove steps if any)
- **Deleted in full:** `beep-test/` directory

### Not touched

- `uwh-common`
- `matrix-drawing`
- `overlay`
- `led-panel-sim`
- `wireless-modes`
- `wireless-remote`
- `schedule-processor`
- `fonts`
- `alphagen`

This is deliberate. The wire format between refbox and the LED panel stays byte-identical
(see "LED panel snapshot encoding" below), which is why the panel firmware (`matrix-drawing`),
the overlay, and the wireless-remote require no changes.

---

## Operator-facing behaviour

The Configuration page's mode selector — which today offers Hockey 6v6, Hockey 3v3, and Rugby —
gains a fourth option: **Beep Test**. Selecting it and pressing Apply triggers the same
restart-the-exe flow that already governs Hockey↔Rugby mode changes:

1. Operator opens Configuration page and selects Beep Test.
2. Operator presses Apply.
3. Refbox persists the new mode to disk, sets `RESTART_PENDING`, and calls `iced::exit()`.
4. `main()` detects the restart flag and spawns a fresh refbox process.
5. The new process reads `mode = BeepTest` from config and starts up showing the beep-test
   screen instead of the game-clock screen.

To leave Beep Test mode, the operator opens Configuration, selects Hockey or Rugby, and presses
Apply — the same restart happens, and refbox lands back in game-management mode.

Inside Beep Test mode, the operator sees a single screen with:

- The cadence timer
- Current level indicator
- Lap count
- A read-only levels table (matching today's standalone behaviour)
- Start / Stop controls
- Access to the Configuration page (so they can leave the mode)
- Access to sound settings (volume, whistle/buzzer enable) via the existing refbox sound
  settings — no separate beep-test sound settings UI

The LED panel mirrors the beep-test display while in this mode, the same way it mirrors the game
display in Hockey/Rugby modes.

---

## Architectural sketch

### Code that moves into refbox

| Source (in `beep-test/`) | Destination (in `refbox/`) | Notes |
|--------------------------|----------------------------|-------|
| `src/tournament_manager/mod.rs` (411 lines) | `src/beep_test/cadence.rs` | Relocated verbatim, then unit-tested |
| `src/snapshot.rs` (`BeepTestSnapshot`, `BeepTestPeriod`, `TimeSnapshot`) | `src/beep_test/snapshot.rs` | Relocated verbatim including the `From<BeepTestSnapshot> for GameSnapshotNoHeap` impl |
| `src/config.rs` (`BeepTest`, `Level`) | Added to `src/config.rs` `Config` struct as `beep_test: BeepTest` field | Default schedule preserved |
| `src/app/view_builders/` (beep-test screen UI) | `src/app/view_builders/beep_test.rs` | Re-written against today's refbox patterns (theme, `fl!` translations, Message/update/view shape) — not copy-pasted |

### Code that is deleted (covered by refbox's existing infrastructure)

| Deleted (in `beep-test/`) | Refbox's existing equivalent |
|---------------------------|------------------------------|
| `src/sim_app/mod.rs`, `display_simulator.rs` | `refbox/src/sim_app/` (byte-identical) |
| `src/sound_controller/sounds.rs` | `refbox/src/sound_controller/sounds.rs` (byte-identical) |
| `src/sound_controller/mod.rs` | `refbox/src/sound_controller/mod.rs` (strict superset; provides `trigger_whistle()`, `trigger_buzzer()`, and all the volume/enable controls beep-test uses) |
| `src/app/update_sender.rs` | `refbox/src/app/update_sender.rs` (strict superset of the LED panel send path) |
| `src/main.rs`, `src/app_icon.rs`, build scripts | Refbox's own startup |

### New code added to refbox

- A `Mode::BeepTest` variant in `refbox/src/config.rs`'s `Mode` enum, with a `Display` impl
  using a new `beep-test` translation key.
- The relocated `beep_test` module (cadence, snapshot, config types).
- A new view_builder for the beep-test screen.
- A handful of `Message` variants for start/stop/reset of beep-test plus update logic in
  `refbox/src/app/mod.rs`.
- Startup routing in `refbox/src/main.rs` that, when `config.mode == Mode::BeepTest`, brings up
  the beep-test view instead of the game view.
- Migration support for the new `beep_test` field in `Config::migrate`.
- Translation keys (English, French, Spanish) for the mode selector label and any new
  beep-test UI text.

### Net effect

- Approximately **2,557 lines deleted** (whole `beep-test/` crate).
- Approximately **700–900 lines added** to refbox (the relocated cadence engine, the new
  view_builder, mode-variant wiring, config field, tests).
- **Net code reduction in the workspace.**
- **One fewer binary** to build, ship, and version-manage.

---

## LED panel snapshot encoding

This is the highest-risk part of the work and also the part with the clearest pre-existing
convention.

### Existing convention (preserved)

`beep-test/src/snapshot.rs` already defines:

```rust
impl From<BeepTestSnapshot> for GameSnapshotNoHeap {
    fn from(snapshot: BeepTestSnapshot) -> Self {
        Self {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: <clamped beep-test seconds>,
            scores: BlackWhiteBundle { black: 0, white: snapshot.lap_count },
            ..Default::default()
        }
    }
}
```

When the standalone beep-test sends data to the LED panel today, it converts its own
`BeepTestSnapshot` into a standard `GameSnapshotNoHeap` with `current_period = BetweenGames` and
the lap count placed in the white-score field. The LED panel firmware (`matrix-drawing`) has no
beep-test awareness — it simply renders the `GameSnapshotNoHeap` it receives.

### What this design preserves

- The wire format (`GameSnapshotNoHeap` binary encoding) is **byte-identical** to today.
- The LED panel firmware needs **no changes**.
- The overlay (stream broadcast) needs **no changes**.
- The wireless-remote firmware needs **no changes** (and does not consume snapshot data anyway).
- `led-panel-sim` needs **no changes**.

When refbox is in BeepTest mode, the cadence engine produces `BeepTestSnapshot` values, and
refbox's existing `update_sender` ships them over the wire as `GameSnapshotNoHeap` via the
preserved `From` impl.

### Why this is the right call

The hardware LED display is not testable during this work — the user does not currently have
access to a panel for verification. Choosing the convention that doesn't alter the wire format
means:

- No risk of a packet-encoding mismatch that can only be caught on real hardware.
- The display behaviour in refbox's BeepTest mode is identical to the standalone beep-test's
  display behaviour today, byte-for-byte. If the standalone displays correctly on the panel
  today, refbox's BeepTest mode will display identically.

---

## Config and persistence

### Refbox `Config` struct additions

Refbox's `Config` (in `refbox/src/config.rs`) gains:

```rust
pub struct Config {
    pub mode: Mode,                  // gains BeepTest variant
    // ... existing fields unchanged ...
    pub beep_test: BeepTest,         // NEW: cadence schedule
    // ... existing fields unchanged ...
}
```

`BeepTest` and `Level` types are relocated from `beep-test/src/config.rs` verbatim. The default
`BeepTest` value carries the standard 10-level cadence schedule that the standalone binary
ships with today.

### Shared settings

- **Sound settings** are shared. Both refbox and standalone beep-test already use the same
  `SoundSettings` type from `sound_controller`. Refbox's existing `sound:` block in `Config`
  covers volumes, sound enables, and buzzer-sound choice for all three game modes plus
  BeepTest. No parallel `beep_test_sound` field is added.
- **Hardware settings** are shared. Beep-test's `Hardware` (`screen_x`, `screen_y`) is a
  strict subset of refbox's `Hardware` (`screen_x`, `screen_y`, `white_on_right`, `brightness`).
  The extra fields in refbox's version are harmless in BeepTest mode.

### Migration

Refbox's `Config::migrate` gains a clause that reads the optional `beep_test` table from old
config files and falls back to the default schedule when absent. Existing user config files
(which lack a `beep_test` section) migrate cleanly with the default schedule.

Old standalone beep-test config files — at a different on-disk path, owned by a different
confy APP_NAME — are **not** migrated into refbox. Users entering BeepTest mode in refbox for
the first time start from the default schedule. If they want custom levels, they edit refbox's
`config.toml` directly, the same way they would have edited beep-test's `config.toml`.

### No level-editing UI

Level customisation is TOML-only, matching today's standalone beep-test behaviour. The
beep-test view in refbox displays the levels table read-only. Adding a level-editing UI is
explicitly out of scope.

---

## Testing approach

### What can be verified without LED hardware

1. **Unit tests for the cadence engine.** Beep-test's `tournament_manager/mod.rs` currently has
   **zero unit tests**. As part of the relocation, the cadence engine moves into refbox in
   commit 1 and gains tests in commit 2 covering:
   - Clock starts and stops cleanly.
   - Level transitions fire at the configured elapsed time.
   - Lap counter increments correctly across level boundaries.
   - Whistle and buzzer triggers are emitted at the right moments.
   - The engine reaches the end of the schedule and stops.

2. **Config serialization and migration tests.** `test_ser_config` and `test_migrate_config`
   are extended to cover the new `beep_test` field — both round-tripping and migration from
   old config files that lack the section.

3. **`just check`** must pass after the change — fmt, clippy (zero warnings, all platforms),
   tests, and audit. This catches any dangling reference to `beep-test/` from the workspace
   `Cargo.toml`, `justfile`, or CI configuration.

4. **Visual verification via the simulator.** Refbox has `sim_app/`, an on-screen window that
   renders exactly what the LED panel would show. Because the simulator uses the same
   `matrix-drawing` rendering code that runs on the actual panel firmware, and because the wire
   format is unchanged, the simulator's coverage of the BeepTest visual experience is
   effectively complete:
   - Visual layout (where lap count appears, where the timer appears).
   - Frame-by-frame updates and cadence transitions.
   - Brightness setting changes.
   - The walkthrough is: launch refbox with the simulator enabled, switch to BeepTest mode in
     Configuration, watch the simulator window during cadence playback, confirm the display
     matches expectation, then switch back to Hockey or Rugby and confirm the restart lands
     cleanly in game mode.

### What is NOT verified by this approach

Documented as deferred-until-hardware-available, not as merge blockers:

- Real LED panel rendering under bright outdoor light and with the actual serial cable.
- Serial cable behaviour during the mode-switch restart (panel briefly disconnects when refbox
  quits, reconnects when the new process launches). This is the same behaviour the existing
  Hockey↔Rugby restart exercises, so if that works on hardware today, BeepTest will too.
- Real wireless-remote behaviour during BeepTest mode. BeepTest does not consume wireless-remote
  input in this design, and field-confirmation of "remote presses are silently ignored during
  BeepTest" requires hardware.

A specific walkthrough script will be drafted as part of the implementation plan, modelled on
the post-merge walkthroughs used for other refbox features.

---

## Acceptance criteria

The work is complete when all of the following are true:

1. `Mode::BeepTest` exists in refbox's `Mode` enum, with English / French / Spanish
   translations for the mode selector label.
2. Selecting BeepTest in the Configuration page and applying triggers a restart that lands in
   the beep-test view.
3. The beep-test view shows the cadence timer, level indicator, lap count, levels table, and
   start/stop controls, and is operable.
4. The LED panel simulator window mirrors the beep-test display while in BeepTest mode.
5. Selecting Hockey 6v6, Hockey 3v3, or Rugby from the Configuration page in BeepTest mode and
   applying triggers a restart that lands in the game view in the chosen mode.
6. The `beep-test/` directory no longer exists in the workspace.
7. The workspace `Cargo.toml`, `justfile`, and CI workflows contain no references to
   `beep-test`.
8. `just check` passes cleanly (fmt, clippy with `-D warnings` on Linux/Windows/macOS, tests,
   audit).
9. Unit tests for the relocated cadence engine exist and pass.
10. The walkthrough script (drafted in the implementation plan) has been executed by the
    operator with the simulator, with all simulator-verifiable scenarios passing.

---

## Risks and mitigations

### Cadence-engine semantics drift during relocation

The cadence engine is being moved from one crate to another. A careless move could change
behaviour in subtle ways.

**Mitigation:** Relocate verbatim in commit 1 with **zero** intentional changes. Add tests in
commit 2. This isolates "did the move work" from "did we want different behaviour" and makes a
git bisect trivial if a regression appears.

### Sound-controller divergence

Beep-test's `sound_controller/mod.rs` differs from refbox's by 464 lines. Most differences are
refbox having a superset, but a careful audit is required to confirm no beep-test-specific
sound path is silently dropped when we delete beep-test's copy.

**Mitigation:** Before the deletion commit, enumerate every public API surface that
beep-test's cadence engine touches in `sound_controller`. Confirm refbox's `sound_controller`
provides each one with matching semantics. Document any gaps. The deletion does not happen
until this audit is clean.

### Mode selector layout

Adding a fourth option to the Configuration page's mode selector may force a layout change.

**Mitigation:** Check this during implementation. If the existing selector doesn't accommodate
four options cleanly, the layout adjustment is a sub-task within this branch (not deferred,
not punted to a separate ticket).

### Overlay rendering of BeepTest snapshots

The overlay (stream broadcast) receives `GameSnapshotNoHeap` regardless of mode and renders
whatever it receives. In BeepTest mode, the overlay will display a `BetweenGames` period with
"score 0–N" where N is the lap count.

**Risk:** This may look odd on stream if anyone is broadcasting during a beep test.

**Mitigation:** Out of scope for this branch. If the appearance is unacceptable, an
overlay-side change to hide the score or show "Beep Test" text would be a separate ticket
against the `overlay` crate. Flagged for awareness, not addressed here.

---

## Out of scope (explicit)

- Wire format changes of any kind.
- LED panel firmware changes (`matrix-drawing` rendering).
- Wireless-remote firmware, or any wireless-remote interaction with BeepTest mode.
- Overlay rendering tweaks for BeepTest snapshots.
- Refactoring of `sound_controller/` or `update_sender.rs` beyond what is necessary to route
  cadence-engine triggers through refbox's existing sound paths.
- A level-editing UI for beep-test cadence levels.
- Migration of any user's existing standalone beep-test config file.
- "While we're in here" cleanup of surrounding refbox code. Observations get filed as
  suggestions for separate branches per `.claude/rules/scope.md`.
- Crate version bumps beyond what dependency removal requires.

---

## References

- `.claude/rules/scope.md` — scope enforcement rules
- `.claude/rules/workspace.md` — crate ownership and multi-crate change rules
- `.claude/rules/rust.md` — Rust patterns and CI gates
- `.claude/rules/plan-execution.md` — lean vs heavy process; this work is **lean**
  (refbox-only, no `uwh-common` changes, no wireless-remote changes, no state-machine
  changes to game logic)
- `refbox/CLAUDE.md` — refbox crate guide (iced patterns, message/update/view convention,
  translation system)
- Unmerged prior attempt on `Scoresheets` / `high-contrast-ui` branches (commits `91d7334`,
  `fe8bdad`, `1a17e21`) — **superseded** by this design
