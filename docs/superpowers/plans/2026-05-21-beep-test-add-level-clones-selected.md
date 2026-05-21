# Beep-Test ADD LEVEL Clones Selected Column — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Chunk 10 of the BeepTest redesign — change ADD LEVEL so the inserted level is a clone of the currently selected column's count and duration, with the existing `count: 4, duration: 20s` defaults remaining only as the empty-state fallback.

**Architecture:** Single in-place change to the `Message::BeepTestEditAddLevel` handler body in `refbox/src/app/mod.rs`. The 15-cap guard, the Chunk 9 insert-after-selected position math, and the new-column-is-selected behavior are all unchanged. Inside the guard, the inline `Level { count: 4, duration: 20s }` literal is replaced by a clone-or-fallback expression: `levels.get(edited.selected_level).cloned().unwrap_or_else(|| Level { count: 4, duration: 20s })`. `Level` already derives `Clone` (verified at `refbox/src/config.rs:75`).

**Tech Stack:** Rust 2024, MSRV 1.85.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-add-level-clones-selected-design.md` (committed at `67576537`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change). One final code review at PR time; no per-task review. No new unit test; verification via walkthrough.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/mod.rs` | `Message::BeepTestEditAddLevel` arm of `update()` — replace the inline `Level { ... }` construction with a clone-or-fallback | Task 1 |

One coding task plus a walkthrough.

---

### Task 1: Replace inline-default with clone-or-fallback

**Files:**
- Modify: `refbox/src/app/mod.rs` (`Message::BeepTestEditAddLevel` arm, around line 3580)

- [ ] **Step 1: Apply the edit**

In `refbox/src/app/mod.rs`, find the `Message::BeepTestEditAddLevel` arm. After Chunk 9 the body reads:

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

Change to:

```rust
            Message::BeepTestEditAddLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if levels.len() < 15 {
                            let new_level = levels
                                .get(edited.selected_level)
                                .cloned()
                                .unwrap_or_else(|| crate::config::Level {
                                    count: 4,
                                    duration: std::time::Duration::from_secs(20),
                                });
                            let insert_at = (edited.selected_level + 1).min(levels.len());
                            levels.insert(insert_at, new_level);
                            edited.selected_level = insert_at;
                        }
                    }
                }
                Task::none()
            }
```

The clone-or-fallback expression must run **before** the `levels.insert(...)` call, because `insert` takes `&mut levels`, and `levels.get(...)` needs an immutable borrow. Resolving the `Option<Level>` into a fully-owned `Level` (via `.cloned().unwrap_or_else(...)`) drops the immutable borrow before the mutable insert.

That is the entire code change. The 15-cap guard, the `insert_at` clamp, the `selected_level = insert_at` post-insert update, and the `Task::none()` return all stay unchanged.

- [ ] **Step 2: Verify the workspace compiles**

```
cargo build -p refbox 2>&1 | tail -5
```

Expected: clean compile. No warnings introduced.

If `cargo build` reports a borrow-checker error around `levels.get(edited.selected_level)` and `levels.insert(...)` overlapping, that means the `let new_level = ...;` binding didn't drop the immutable borrow as expected. The fix is to wrap the clone-or-fallback in an explicit scope:

```rust
let new_level = { levels.get(edited.selected_level).cloned().unwrap_or_else(|| ...) };
```

(This is unlikely — Rust 2024's NLL/Polonius drop the borrow at `;`.)

- [ ] **Step 3: Run `just check`**

```
just check
```

Expected: PASS — fmt, clippy, full test suite, audit. The "Missing keys in translations/*" warnings about `portal-row-attempt-suffix` are pre-existing and unrelated; ignore them.

- [ ] **Step 4: Commit**

```
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): ADD LEVEL clones the selected column's count and duration"
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

- [ ] **Step 2: Scenario 1 — clone preserves selected column's values**

Navigate **Main → Settings → Edit Levels**. Select any non-last column. Modify its values: tap COUNT [+] a few times until the count reads e.g. `5`; tap TIME [+] a few times until duration reads e.g. `30`. With that column still selected, tap **ADD LEVEL**. Confirm:

- The new column appears immediately to the right of the selected one (Chunk 9 invariant).
- The new column is selected.
- The new column's count reads `5` (matching the source column).
- The new column's duration reads `30` (matching the source column).
- The per-level edit panel shows the cloned values, not the static `count: 4, duration: 20`.

- [ ] **Step 3: Scenario 2 — clone follows the freshly-modified column**

From the result of Scenario 1, the just-created new column is selected. Modify it further — e.g. tap COUNT [+] to make its count `7`. Tap ADD LEVEL again. Confirm the third new column has count `7` (matching the *latest* selected column's count, not the original column's `5`).

- [ ] **Step 4: Scenario 3 — 15-cap, insert-after-selected, and selection still work**

Continue adding levels until 15 exist. With an interior column selected at 15 levels, confirm:

- ADD LEVEL is grayed out (disabled style).
- Tapping it does nothing — no 16th column appears.

Then tap REMOVE LEVEL once (any column). Select a non-last column. Tap ADD LEVEL. Confirm:

- The new column appears immediately to the right of the selected one (insert position unchanged from Chunk 9).
- The new column's count and duration are clones of the selected column's (Chunk 10 invariant).
- The new column is selected (selection invariant unchanged from Chunks 7-9).

- [ ] **Step 5: Hand back to operator**

Report walkthrough results for Scenarios 1–3. If any scenario fails or behaves unexpectedly, stop and report — do not push/PR. Also note: the EDIT LEVELS-button text-rendering artifact observed during the Chunk 9 walkthrough is deferred to Chunk 11 — do not attempt to address it here.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:**
  - Spec §Goal (clone count + duration) → Task 1 Step 1 (full before/after handler body shows the `.cloned().unwrap_or_else(default)` substitution).
  - Spec §Why the fallback is needed → captured implicitly by retaining the `.unwrap_or_else(|| Level { count: 4, duration: 20s })` arm; no test step required for the empty-state path (out of scope).
  - Spec §Testing scenarios 1–3 → Task 2 Steps 2–4.
  - Spec §Acceptance criteria (`just check` green + walkthrough pass) → Task 1 Step 3 + Task 2.
- **No placeholders:** all steps show concrete code/commands. The Step 2 borrow-checker contingency is a real fallback (with a specific Rust 2024 scope-wrap fix), not a placeholder.
- **Type consistency:** No new types or identifiers introduced. `levels`, `edited`, `edited.selected_level`, `crate::config::Level`, `std::time::Duration` are all already in scope; `.cloned()` and `.unwrap_or_else(|| ...)` are standard `Option` combinators in the `core` prelude. `Level` derives `Clone` (verified at `refbox/src/config.rs:75`), so `.cloned()` on `Option<&Level>` yields `Option<Level>`.
- **Lean process:** one `just check` gate; no per-task code review; final review at PR time. No new unit test.
- **Single-task granularity:** the change is one connected handler-body rewrite; splitting it would create churn (an intermediate state with an unused `let new_level = ...` is silly).
- **Out-of-scope rendering issue:** the EDIT LEVELS-button text-rendering artifact is explicitly NOT addressed here, per the spec's Out of Scope section, and Task 2 Step 5 reminds the executing engineer of that.
