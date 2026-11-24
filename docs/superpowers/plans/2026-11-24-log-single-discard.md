# Log a hand-discarded queued result Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** DISCARD THIS SUBMISSION writes the same "discarded queued result" log line SWITCH ANYWAY already writes.

**Architecture:** Move the per-item `log::warn!` out of `PortalManager::discard_all_queued` into one private fn, and call it from `PortalManager::discard` for the removed row after the save succeeds.

**Tech Stack:** Rust, `log` crate (already used).

**Spec:** `docs/superpowers/specs/2026-11-24-log-single-discard-design.md`

## Global Constraints

- Line wording unchanged: `discarded queued result: event {event_id} game {game_number} ({kind})`, kind = `stats not sent` if `score_sent`, else `score and stats not sent`.
- Level `warn`. No new dependency, no translation, no UI change.
- MSRV 1.85; clippy `-D warnings`.

## Review Focus

- `discard` with an id that is not in the queue (already gone, e.g. sent meanwhile) → logs nothing.
- Save fails → row kept, nothing logged as discarded.
- Stats-pending row (`score_sent == true`) → kind reads `stats not sent`.
- SWITCH ANYWAY output byte-identical to before.
- (No log-capture harness in the crate — these are checked by reading the diff; existing discard tests must stay green. PO-approved: no new test.)

---

### Task 1: Shared discard log line

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` (`discard_all_queued` ~line 637, `discard` ~line 1137)

**Interfaces:**
- Produces: `fn log_discarded(item: &QueuedItem)` — private free fn (or associated fn) in `portal_manager/mod.rs`.

- [ ] **Step 1:** Add `log_discarded`, holding the existing `kind` match and `log::warn!` verbatim; make `discard_all_queued`'s loop call it.
- [ ] **Step 2:** In `discard`, find the removed row(s) from `before` (those with `it.id == *id`); after `persist()` returns `Ok`, call `log_discarded` on each. No row matched → no line.
- [ ] **Step 3:** With coordinator GO: `CARGO_TARGET_DIR=/home/estraily/projects/uwh-refbox-rs-assistant-chief/target CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox portal_manager` → all pass, then `just check` (same env) → green.
- [ ] **Step 4:** Commit `fix(refbox): log a queued result discarded by hand` (with the spec + this plan).
