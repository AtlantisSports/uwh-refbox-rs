# Beep-Test ADD LEVEL Clones Selected Column — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 9 at `586c98a5`)
**Chunk:** 10 of the beep-test redesign follow-on series
**Process gate:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change).

---

## Goal

Change `Message::BeepTestEditAddLevel` so the inserted level is a **clone of the currently selected column's `count` and `duration`**, instead of always using the static defaults `count: 4, duration: 20s`. The insert position (Chunk 9) and the 15-cap (Chunks 7/8) remain unchanged.

---

## Motivation

Surfaced 2026-05-21 during the Chunk 9 walkthrough. The operator confirmed the insert-after-selected behavior works correctly, but noted that the newly-inserted level always uses the static defaults — a worse starting point than the column the operator was just editing. When building a graduated cadence, the operator typically wants the next level to *resemble* the previous one (small adjustments in count or duration), not snap back to a fixed pair. Cloning the selected column gives the operator a useful starting point and avoids unnecessary re-tapping of `[+]` / `[-]` buttons after every ADD LEVEL.

The change is a small in-place rewrite of the handler's level-construction step. No view, `Message`, or layout change.

---

## Scope

### Files touched

- `refbox/src/app/mod.rs` — the `Message::BeepTestEditAddLevel` arm of `update()` (around line 3580). Replace the inline `Level { count: 4, duration: 20s }` construction with a clone-or-fallback expression.

### Not touched

- `refbox/src/app/view_builders/beep_test_settings.rs` — the empty-state fallback view that renders an enabled ADD LEVEL button when `levels` is empty stays as-is.
- The 15-cap guard (`if levels.len() < 15`).
- The Chunk 9 `insert_at` clamp pattern.
- The `Level` struct or any derived trait.
- `Message::BeepTestEditRemoveLevel`.
- Translations, other view-builders, the LED panel, Chunk 6 work.

### Out of scope (intentionally deferred)

- **The EDIT LEVELS button text-rendering artifact** observed during the Chunk 9 walkthrough (looks like the same iced text-reflow issue a prior chunk addressed elsewhere). Tracked as Chunk 11.
- **Changing the empty-state fallback defaults** away from `count: 4, duration: 20s`. The fallback only fires when `levels.get(selected_level)` returns `None` (empty-staged-levels), which is reachable only via a degenerate manually-edited config. Keeping `count: 4, duration: 20s` matches pre-Chunk-10 behavior on that path so the empty-state UX is unchanged.
- **Disabling ADD LEVEL when `levels.is_empty()`.** This would be a different fix; the empty-state fallback view exists precisely to let operators recover from an empty state, so disabling the button is the wrong direction.
- **Cloning more than count + duration** (e.g., a future per-level "label" field). Not requested.
- **Changing the per-level lap-count cap** (Chunk 1's `level.count >= 5`).

---

## Design

### Approach A — `.cloned().unwrap_or_else(default)` (approved through brainstorming)

After Chunk 9 the handler body is:

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

Replace the inline `Level { ... }` construction with a clone-or-fallback. After:

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

`Level` derives `Clone` (verified at `refbox/src/config.rs:75`), so `.cloned()` on an `Option<&Level>` yields `Option<Level>`. The `.unwrap_or_else(|| ...)` fallback only fires when `levels.get(edited.selected_level)` is `None`, which in practice means `levels` is empty (or `selected_level` is stale past the end — also handled).

### Why the fallback is needed

The empty-state fallback view in `build_edit_panel` (`refbox/src/app/view_builders/beep_test_settings.rs` around line 478, post-Chunk-8) renders an **enabled** ADD LEVEL button whenever `levels.get(selected)` returns `None`. That path is reachable when:

- A manually-edited `config.toml` ships `beep_test.levels = []`.
- A future deserialization or migration produces an empty levels list.
- Any defensive state that the codebase's other guards (`saturating_sub`, `.min()`, etc.) suggest the authors treat as possible.

Without a fallback, `levels[edited.selected_level].clone()` (or any unwrap-less variant) would panic. Panicking on operator input is unacceptable for a tournament tool. `.unwrap_or_else(default)` matches the rest of the codebase's defensive style.

### Why the fallback keeps `count: 4, duration: 20s`

These are the pre-Chunk-10 inline defaults. Keeping them in the fallback path means the empty-state UX is unchanged from Chunk 9 — only the "selected-column-exists" path's behavior changes.

### Pattern reference (per `.claude/rules/patterns.md`)

The `levels.get(idx).cloned().unwrap_or_else(default)` pattern is a standard Rust `Option` combinator chain, used widely in Rust codebases. It mirrors the codebase's existing defensive style: `saturating_sub`, `.min()`, `if let Some(ref mut levels)` guards, the Chunk 9 `(selected + 1).min(len)` clamp.

---

## Testing

No new unit test. Lean process (refbox UI, no state-machine change). Verification via walkthrough.

**Walkthrough scenarios:**

1. **Clone preserves selected column's values.** Open Edit Levels. Pick any non-last column, tap COUNT [+] and TIME [+] a few times to give it distinctive values (e.g. count = 5, duration = 30 s). With that column still selected, tap ADD LEVEL. Confirm:
   - The new column appears immediately to the right of the selected one (Chunk 9 behavior unchanged).
   - The new column's count and duration both equal the selected column's values (5 and 30 s in this example), not the static defaults.
   - The per-level edit panel reflects the cloned values.
2. **Clone follows the freshly-modified column.** From the result of Scenario 1, modify the selected column further (the new one) — e.g. count = 7. Tap ADD LEVEL again. Confirm the third column clones the second column's most-recent values, not the original first column's.
3. **The 15-cap, insert-after-selected, and selection-after-insert behaviors all still work.** Quickly verify by selecting an interior column at 15 levels (ADD LEVEL grayed), selecting a non-last column at 14 levels (ADD LEVEL inserts the clone immediately right of selected), and confirming the cloned new column is selected after insert.

`just check` passes (fmt, clippy, full test suite, audit).

---

## Acceptance criteria

The three walkthrough scenarios pass; `just check` is green.
