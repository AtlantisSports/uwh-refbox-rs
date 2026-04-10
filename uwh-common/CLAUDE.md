# uwh-common — Crate Guide

`uwh-common` is the shared library that all other crates in the workspace depend on. It defines
the core data types for game state, team and player information, and the wire format for
communication between components.

**This is the highest-impact crate in the workspace. Changes here affect everything.**

---

## What This Crate Does

- Defines the `GameSnapshot` type — the complete state of a game at a point in time
- Defines portal API request/response types used by `schedule-processor` and `refbox`
- Defines the wire format for network communication between `refbox` and `overlay`
- Provides colour definitions, configuration types, and shared utilities

---

## Key Files and What They Do

| File | Purpose |
|------|---------|
| `src/game_snapshot.rs` | `GameSnapshot` — the central game state type |
| `src/uwhportal/schedule.rs` | Types for tournament schedule data from the portal |
| `src/uwhportal/mod.rs` | `UwhPortalClient` — HTTP client for the portal API |
| `src/config.rs` | Shared configuration types |
| `src/color.rs` | Team colour definitions |
| `src/bundles.rs` | Grouped data bundles for network transmission |
| `src/wire_format.md` | Documentation of the network wire format |

---

## The `no_std` Requirement

This crate must compile in two environments:

1. **Standard** (with `std` feature) — used by `refbox`, `schedule-processor`, `overlay`, etc.
2. **Embedded** (without `std`) — used by `wireless-remote` and potentially `matrix-drawing`

**Rules:**
- Never add a `use std::...` import at the top level without a `#[cfg(feature = "std")]` guard
- Never add a dependency that unconditionally requires `std`
- If a dependency is std-only, add it under `[dependencies]` with
  `optional = true` and gate its use behind `#[cfg(feature = "std")]`
- Always test `--no-default-features` compiles after any change: `cargo build -p uwh-common --no-default-features`

---

## Changing `GameSnapshot`

`GameSnapshot` is the most important type in the codebase. Every other component reads from it.

Before adding or modifying a field:
1. State what the field represents in plain English
2. Identify which crates will need to be updated to use it
3. Consider backward compatibility — older overlays or refbox versions may be receiving
   serialized snapshots; breaking changes to the wire format need a version bump

---

## Changing Portal Types (`uwhportal/`)

The portal API types represent external data from the UWH Portal. Before changing these:
1. Confirm the portal API actually sends/accepts the new format
2. Use `serde` rename attributes if the API field name differs from Rust conventions
3. Use `Option<T>` for fields that may be absent from older API responses

---

## Downstream Impact Checklist

After any change to this crate, verify these all compile and pass tests:

- [ ] `cargo check -p refbox`
- [ ] `cargo check -p schedule-processor`
- [ ] `cargo check -p overlay`
- [ ] `cargo check -p led-panel-sim`
- [ ] `cargo build -p uwh-common --no-default-features` (no_std check)
- [ ] `just test`
