//! Beep-test mode: cadence engine, snapshot types, and configuration.
//!
//! Absorbed from the standalone `beep-test/` crate per the design at
//! `docs/superpowers/specs/2026-05-18-beep-test-absorption-design.md`.

// The cadence engine is a verbatim relocation from `beep-test/src/tournament_manager/mod.rs`.
// All types and methods are intentionally unused until Task 7 wires them into `RefBoxApp`.
#[allow(dead_code)]
pub mod cadence;
pub mod snapshot;
