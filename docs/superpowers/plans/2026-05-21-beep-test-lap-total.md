# Beep-Test LAP Tile Shows Total Laps — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Source the LAP tile's display value from `snapshot.lap_count` instead of `active_within_lap`, so the counter accumulates across the whole run instead of resetting per level.

**Architecture:** One-line code change in `beep_test.rs`. The cadence engine already maintains `lap_count` in the shape we want; the within-level helper stays unchanged so the levels-table active-cell highlight keeps working.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-lap-total-design.md` (committed at `5304059`).

**Process:** Lean (refbox UI work, per `.claude/rules/plan-execution.md`).

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/view_builders/beep_test.rs` | `lap_value` computation in `build_beep_test_page` | Task 1 |

One file, one task for the code, one task for the walkthrough.

---

### Task 1: Source LAP from `snapshot.lap_count`

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test.rs` (the `lap_value` `let` binding inside `build_beep_test_page`, currently lines 76–79)

- [ ] **Step 1: Replace the `lap_value` computation**

Find:

```rust
let lap_value: String = match snapshot.current_period {
    BeepTestPeriod::Level(0) => 1.to_string(),
    BeepTestPeriod::Level(_) => active_within_lap.unwrap_or(1).to_string(),
};
```

Replace with:

```rust
let lap_value: String = snapshot.lap_count.to_string();
```

Leave the `active_within_lap` computation above untouched — it still drives the active-cell highlight in the levels table further down.

- [ ] **Step 2: Verify build**

```
just check
```

Must pass.

- [ ] **Step 3: Commit**

```
git add refbox/src/app/view_builders/beep_test.rs
git commit -m "feat(refbox): LAP tile shows total laps across the run"
```

Include the Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

- [ ] **Step 2: Walk the spec's acceptance criteria**

In BeepTest mode:

1. Fresh state → LAP shows `0`.
2. Press START → warmup counts down for 10s; LAP stays at `0`.
3. Warmup ends, Level 1 Lap 1 begins → LAP reads `1`.
4. At each lap boundary, LAP increments by exactly 1.
5. Across a level boundary (Level 1's last lap → Level 2 Lap 1), LAP does
   NOT reset; it just continues incrementing. LEVEL tile updates separately.
6. Press PAUSE → LAP holds at its current value.
7. Press RESET → LAP returns to `0`.

Report any failure.

- [ ] **Step 3: Hand back to operator**

Report walkthrough results. Do not push.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:** Spec §Design (one-line change) maps to Task 1 Step 1. Spec §Acceptance Criteria maps to Task 2 Step 2.
- **No placeholders.** Concrete code/commands in every step.
- **Type consistency.** No new types. `snapshot.lap_count` is a `u8` and `Display`s as its integer value.
- **Lean process.** Single code commit + walkthrough. Smallest plan in this chunk series.
