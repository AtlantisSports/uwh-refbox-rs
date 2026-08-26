//! Library crate for `overlay-bridge`.
//!
//! Modules live here, not in `main.rs`, so that a module no task has wired into the running
//! binary yet is still compiled and linted normally: unused `pub` items in a library are
//! legitimately part of its API surface and are exempt from the `dead_code` lint, whereas the
//! same items declared only inside a binary's `main.rs` would be flagged as unreachable. The
//! binary (`main.rs`) is kept thin -- CLI parsing, config load, wiring, the tokio runtime -- and
//! pulls everything else from here.

pub mod feed;
pub mod portal;
pub mod state;
