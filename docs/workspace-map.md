# Workspace Map

This document describes what each crate in the workspace does, what kinds of changes belong in
it, and what to be careful about when working there. Read this before making any change to
understand what you are touching and why.

---

## Main Workspace Crates

### `refbox`

**What it is:** The main application. This is the software the referee operator uses at a
tournament to manage a game — the clock, scores, fouls, timeouts, and all game events.

**Tech:** Built with the `iced` GUI framework. Communicates with the LED panel over serial,
the wireless remote over LoRa radio, and the overlay over the local network.

**Key files:**
- `refbox/src/app/mod.rs` — core application logic and state machine
- `refbox/src/app/view_builders/` — every screen/page in the UI
- `refbox/src/app/theme/` — visual styling (colours, button styles, etc.)
- `refbox/src/tournament_manager/` — game state and timing logic
- `refbox/src/config.rs` — user configuration (settings that persist between sessions)

**Changes belong here when:** fixing UI behaviour, changing how game events are recorded,
modifying what the referee operator sees or can do.

**Be careful:** Changes to `tournament_manager/` affect core game logic — the timing and state
machine that everything else depends on. Changes here need careful testing.

---

### `uwh-common`

**What it is:** The shared library that all other crates depend on. Contains the core data types
for game state, team/player information, and the wire format for communicating between components.

**Tech:** Designed to compile in both standard (with networking, file I/O) and embedded (no
standard library) environments. Uses `serde` for serialization.

**Key files:**
- `uwh-common/src/game_snapshot.rs` — the `GameSnapshot` type: the complete state of a game
- `uwh-common/src/uwhportal/` — data types for portal API responses (schedules, teams, players)
- `uwh-common/src/config.rs` — shared configuration types
- `uwh-common/src/color.rs` — team colour definitions

**Changes belong here when:** adding or modifying a data type that is shared between two or more
crates (e.g., a new field on `GameSnapshot`, a new portal API response type).

**Be careful:** This is the highest-impact crate in the workspace. Any change here can break
`refbox`, `schedule-processor`, `overlay`, `led-panel-sim`, and potentially `wireless-remote`.
Always check all dependent crates after changing this.

**Special rule:** Must remain `no_std` compatible for the embedded use case. Never add a
dependency that requires the standard library without gating it behind a feature flag.

---

### `schedule-processor`

**What it is:** A command-line tool run by the tournament organizer before each tournament. It
reads the tournament schedule (from a CSV export or the portal API), validates it for errors,
generates scoresheets, and outputs data the refbox can load.

**Tech:** CLI built with `clap`. Talks to the UWH Portal API via `reqwest`. Uses `uwh-common`
types for schedule and game data.

**Key files:**
- `schedule-processor/src/main.rs` — CLI entry point and main workflow
- `schedule-processor/src/csv_parser.rs` — parses CSV schedule exports
- `schedule-processor/src/schedule_checks.rs` — validates the schedule for errors
- `schedule-processor/src/scoresheets.rs` — generates scoresheet PDFs

**Changes belong here when:** fixing how schedules are parsed, adding new validation checks,
changing the scoresheet format, or adding new portal API interactions.

**Note:** This is a standalone tool — it does not run during a game. It is used in preparation
for a tournament.

---

### `overlay`

**What it is:** The broadcast stream overlay application. Displays a live graphic (scores, time,
team names) over the game video stream. Connects to a running refbox instance over the local
network to read game state.

**Tech:** Built with `macroquad` (a game-engine-style framework). Receives `GameSnapshot` data
from the refbox over the network.

**Key files:**
- `overlay/src/main.rs` — entry point, network connection, main loop
- `overlay/src/pages/` — individual overlay display pages (in-game, pre-game, final scores, etc.)
- `overlay/src/network.rs` — communication with the refbox

**Changes belong here when:** changing what the overlay displays, how it looks, or how it
connects to the refbox.

**Note:** Used at major tournaments; increasingly common. Changes here do not affect game
management.

---

### `beep-test`

**What it is:** A standalone tool for testing the buzzer/audio system independently of a game.
Useful for checking that sounds work correctly on a given hardware setup.

**Tech:** Built with `iced`, similar architecture to `refbox`.

**Changes belong here when:** fixing audio playback issues or adjusting how sounds are triggered
in test mode.

---

## Utility Crates

These crates are smaller and more self-contained. Changes here are usually narrow in scope.

| Crate | What it does |
|-------|-------------|
| `uwh-common` | (See above — not just a utility) |
| `matrix-drawing` | Drawing primitives for the LED panel display. Must be `no_std` compatible. |
| `fonts` | Embedded font data for the LED panel display. |
| `led-panel-sim` | Simulates the LED panel for testing without physical hardware. |
| `alphagen` | Converts image alpha channels to greyscale masks. Used for overlay assets. |
| `wireless-modes` | Defines the LoRa radio modes shared between `refbox` and `wireless-remote`. |

---

## `wireless-remote` (Excluded from main workspace)

**What it is:** Embedded firmware for the handheld referee button device (a Raspberry Pi Pico /
RP2040 microcontroller). When the referee presses the button, the firmware sends a signal over
LoRa radio to the refbox, which plays the appropriate buzzer sound.

**Special status:** This is NOT part of the main Cargo workspace. It lives in `wireless-remote/`
and has its own toolchain, target architecture, and build process.

**Do not touch without explicit discussion.** Changes to firmware require physical hardware to
test and validate. See `.claude/rules/embedded.md` for full rules.

---

## Dependency Map

Who depends on what:

```
refbox ──────────────────────┐
schedule-processor ──────────┤──► uwh-common
overlay ─────────────────────┤
led-panel-sim ───────────────┤
matrix-drawing ──────────────┘
wireless-remote ──────────────► wireless-modes, uwh-common (partially)
```

**Consequence:** Any breaking change to `uwh-common` must be verified against all crates above.
