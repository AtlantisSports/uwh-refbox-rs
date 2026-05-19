# Workspace Navigation

These rules map change types to the crates that own them. Use this before starting any task to
identify exactly where the change belongs and what the blast radius is.

## Crate Ownership

### `uwh-common` — Shared types and game logic

This is the highest-impact crate. **Changes here can break every other crate.**

Touch `uwh-common` only when:
- A data type needs to be added or modified that is used by more than one crate
- A wire format (serialization) change is required
- A core game logic rule needs to change

When changing `uwh-common`, always:
1. State which downstream crates may be affected before editing
2. Check `refbox`, `schedule-processor`, `overlay`, and `led-panel-sim` after the change
3. Run `just check` before committing

### `refbox` — Main application

Touch `refbox` for:
- UI behaviour changes (what the referee operator sees or can do)
- Game clock and state machine fixes
- Configuration changes
- Hardware communication (serial to LED panel, LoRa to wireless remote)
- Sound/buzzer handling

### `schedule-processor` — Tournament schedule CLI

Touch `schedule-processor` for:
- CSV parsing changes
- Schedule validation logic
- Scoresheet generation
- Portal API interactions (fetching schedules, coin flip endpoints)

### `overlay` — Stream broadcast display

Touch `overlay` for:
- What the stream overlay displays
- How the overlay connects to the refbox
- Overlay visual layout changes

### `wireless-remote` — Embedded firmware

**Do not touch without explicit discussion.** See `embedded.md` for full rules.

### Utility crates (`matrix-drawing`, `fonts`, `led-panel-sim`, `alphagen`, `wireless-modes`)

Changes here are narrow and self-contained. Always verify the owning crate still compiles after
changes to these.

## The Dependency Rule

If a task requires changing `uwh-common`, identify *every* crate that imports from it before
starting. The full list is: `refbox`, `schedule-processor`, `overlay`, `led-panel-sim`,
`matrix-drawing`, and partially `wireless-remote`.

A change to `uwh-common` that breaks any of these crates will fail CI. Check all of them.

## Multi-Crate Changes

Some features or fixes will legitimately span multiple crates (e.g., a new portal data type
added to `uwh-common` and consumed by `schedule-processor`). This is fine, but:

1. State upfront which crates will be touched and why
2. The branch scope should reflect the broadest crate involved
   (e.g., `feat/uwh-common/new-portal-type` even if `schedule-processor` is also touched)
3. Do not add a third crate to the change without discussion
