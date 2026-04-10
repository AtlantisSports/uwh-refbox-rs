# refbox — Crate Guide

The `refbox` crate is the main application. This is the software the referee operator uses at a
tournament to manage a game.

---

## What This Crate Does

The refbox provides a graphical interface for:
- Starting, stopping, and pausing the game clock
- Recording goals, fouls, and penalties
- Managing team timeouts
- Communicating with the LED panel (serial), wireless remote (LoRa), and overlay (network)
- Loading game schedules from the tournament portal

---

## Key Files and What They Do

| File/Directory | Purpose |
|----------------|---------|
| `src/app/mod.rs` | Core application logic: handles all user actions and game state transitions |
| `src/app/view_builders/` | One file per screen/page in the UI |
| `src/app/theme/` | Visual styling: colours, button styles, text styles |
| `src/app/message.rs` | All possible user actions and events (the `Message` enum) |
| `src/tournament_manager/` | Game state and timing logic — the most critical code |
| `src/tournament_manager/mod.rs` | Game clock, state machine, score and penalty tracking |
| `src/config.rs` | User settings that persist between sessions |
| `src/sound_controller/` | Buzzer/sound management |
| `src/sim_app/` | Simulator mode for testing without hardware |
| `translations/` | UI text in English, Spanish, and French |

---

## Architecture: How the UI Works

This application uses the `iced` framework (version 0.13), which follows an Elm-like pattern:

1. **State** — the app holds the complete game state in memory
2. **View** — the UI is rendered from that state (files in `view_builders/`)
3. **Message** — every user action produces a `Message`
4. **Update** — the `update()` function in `mod.rs` handles each message and changes the state

This means: to add a new button, you add a `Message` variant, add the button to the relevant
`view_builder` file, and handle the message in `update()`.

---

## The Tournament Manager

`src/tournament_manager/` contains the game clock and state machine. This is the most critical
code in the crate. Changes here can cause:
- The clock to tick at the wrong time
- The game to get stuck in an unexpected state
- Scores or penalties to be recorded incorrectly

**Always run `just test` after any change to tournament_manager/**.

---

## Iced Patterns to Follow

- All theme/styling goes in `src/app/theme/` — never inline styles
- All UI text that users see must go through the translation system (`translations/`)
- Follow the existing message naming convention: `Message::VariantName`
- Do not introduce new widget types from `iced` without first checking if an existing one
  in `shared_elements.rs` can be reused

---

## Dependencies to Be Aware Of

- `uwh-common` — provides all core game types; changes there ripple into this crate
- `tokio` — the async runtime; all network and hardware communication is async
- `log` / `log4rs` — logging; all significant events should be logged
- Feature flags: `iced/default` uses `tiny-skia` on Linux and `wgpu` on macOS/Windows
