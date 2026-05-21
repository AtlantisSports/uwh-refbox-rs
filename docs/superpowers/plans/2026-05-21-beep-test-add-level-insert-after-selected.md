# Beep-Test ADD LEVEL Inserts After Selected — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Chunk 9 of the BeepTest redesign — change ADD LEVEL to insert immediately to the right of the currently selected column (and select the new level), rather than always appending at the end.

**Architecture:** Single handler-body rewrite in `refbox/src/app/mod.rs`. The existing 15-cap guard, new-level defaults (count = 4, duration = 20 s), and `Task::none()` return are unchanged. Inside the guard, `levels.push(...)` + `selected_level = levels.len() - 1` becomes `levels.insert(insert_at, ...)` + `selected_level = insert_at`, where `insert_at = (edited.selected_level + 1).min(levels.len())` clamps the position to be in-range for any state (mid-list, last-column, empty list, stale selection).

**Tech Stack:** Rust 2024, MSRV 1.85.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-add-level-insert-after-selected-design.md` (committed at `f1ef83b0`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change). One final code review at PR time; no per-task review. No new unit test; verification via walkthrough.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/mod.rs` | `Message::BeepTestEditAddLevel` arm of `update()` — handler body rewrite | Task 1 |

One coding task plus a walkthrough.

---

### Task 1: Rewrite the `Message::BeepTestEditAddLevel` handler body

**Files:**
- Modify: `refbox/src/app/mod.rs` (`Message::BeepTestEditAddLevel` arm, around line 3580)

- [ ] **Step 1: Apply the edit**

In `refbox/src/app/mod.rs`, find the `Message::BeepTestEditAddLevel` arm. After Chunk 8 it reads:

```rust
            Message::BeepTestEditAddLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if levels.len() < 15 {
                            levels.push(crate::config::Level {
                                count: 4,
                                duration: std::time::Duration::from_secs(20),
                            });
                            edited.selected_level = levels.len() - 1;
                        }
                    }
                }
                Task::none()
            }
```

Change to:

```rust
            Message::BeepTestEditAddLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if levels.len() < 15 {
                            let insert_at = (edited.selected_level + 1).min(levels.len());
                            levels.insert(
                                insert_at,
                                crate::config::Level {
                                    count: 4,
                                    duration: std::time::Duration::from_secs(20),
                                },
                            );
                            edited.selected_level = insert_at;
                        }
                    }
                }
                Task::none()
            }
```

That is the entire code change. The 15-cap guard, new-level defaults, and `Task::none()` are unchanged. The `Message::BeepTestEditRemoveLevel` arm immediately below stays untouched.

- [ ] **Step 2: Verify the workspace compiles**

```
cargo build -p refbox 2>&1 | tail -5
```

Expected: clean compile. No warnings introduced.

If `cargo build` fails with a borrow-checker error around the `edited.selected_level` read inside the `if let Some(ref mut levels) = edited.beep_test_levels` block, that means Rust's field-level-disjoint-borrow rule didn't apply here. The fix is to capture `let sel = edited.selected_level;` *before* the inner `if let Some(ref mut levels)`, then use `sel` in the `insert_at` calculation. (This is unlikely — the existing post-Chunk-8 code already accesses `edited.selected_level = levels.len() - 1` in the same scope and compiles cleanly.)

- [ ] **Step 3: Run `just check`**

```
just check
```

Expected: PASS — fmt, clippy, full test suite, audit.

- [ ] **Step 4: Commit**

```
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): ADD LEVEL inserts to the right of selected column"
```

With Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

Per `.claude/rules/pr-review.md`: "Smoke-tested locally — refbox (or the affected artifact) was launched and the change exercised in a real session before any push/PR/merge/tag-push."

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(Operator config has `mode = "BeepTest"`, so refbox comes up in BeepTest mode directly.)

- [ ] **Step 2: Scenario 1 — insert in the middle**

Navigate **Main → Settings → Edit Levels**. Ensure at least 3 levels are staged (if not, tap ADD LEVEL once or twice to reach ≥ 3; that's still the only available behaviour pre-this-fix). Tap a *non-last* column header to select it — for example, the column at position 3 of 5. Tap **ADD LEVEL**. Confirm:

- A new column appears at position 4 (immediately to the right of the selected column).
- The new column is highlighted (selected).
- The previous columns 4 and 5 have shifted right to positions 5 and 6.
- The per-level edit panel shows the new level's default values (count, duration).

- [ ] **Step 3: Scenario 2 — insert at end (last-column selection)**

Tap the last (right-most) column to select it. Tap ADD LEVEL. Confirm:

- The new column appears at the very end (right-most position).
- The new column is selected.
- This matches pre-Chunk-9 behaviour for this case (last-column-selected → append).

- [ ] **Step 4: Scenario 3 — 15-cap still works**

Continue selecting columns and tapping ADD LEVEL until 15 columns exist. Try the "insert in the middle" scenario again at the 15th level — i.e. select an interior column with 15 levels staged, observe the ADD LEVEL button. Confirm:

- ADD LEVEL is grayed out (disabled style) regardless of which column is selected.
- Tapping disabled ADD LEVEL does nothing — no 16th column appears.

- [ ] **Step 5: Scenario 4 — REMOVE LEVEL still shifts correctly**

From the middle-insert state (Scenario 1's result, or any state with ≥ 2 levels and a non-last column selected), tap REMOVE LEVEL. Confirm:

- The selected column disappears.
- Selection moves to a remaining column (typically the one to the left of the removed position; existing behaviour).
- The remaining columns stay at their locked 1/15 width (Chunk 8's column-lock invariant unbroken).

- [ ] **Step 6: Hand back to operator**

Report walkthrough results for Scenarios 1–4. If any scenario fails or behaves unexpectedly, stop and report — do not push/PR.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:**
  - Spec §Design `insert_at` clamp + `insert` + select-new → Task 1 Step 1 (full before/after handler body).
  - Spec §Testing scenarios 1–4 → Task 2 Steps 2–5.
  - Spec §Acceptance criteria (`just check` green + walkthrough pass) → Task 1 Step 3 + Task 2.
- **No placeholders:** all steps show concrete code/commands. The Step 2 borrow-checker contingency is a real fallback (not a placeholder) — it tells the executing engineer what to do *if* the unlikely borrow-checker error appears, with a specific fix (capture `sel` before the inner `if let`).
- **Type consistency:** No new types or identifiers introduced. `edited`, `levels`, `crate::config::Level`, `std::time::Duration`, `selected_level` are all already in scope and unchanged in type from Chunk 8's code.
- **Lean process:** one `just check` gate; no per-task code review; final review at PR time. No new unit test.
- **Task granularity:** one coding task is the right granularity here — the change is a single connected handler-body rewrite where splitting would just create churn (e.g., adding `insert_at` separately from changing `push` to `insert` would leave an unused variable mid-edit).
