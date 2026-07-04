# Editor Top-Spacing Standardization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin the time-editor box to the top of the refbox parameter-editor and game-time edit screens by removing the flexible spacer above it, and move the "time is paused" note below the editor on the game-time screen.

**Architecture:** Layout-only edits to two `iced` view-builder functions. `make_time_editor` is a shrink-height container whose vertical position is set entirely by the parent `column`'s spacers, so the change is purely reordering/removing `vertical_space()` calls. No state, message, wire-format, or shared-type changes.

**Tech Stack:** Rust 2024, `iced` 0.13, the `refbox` crate.

## Global Constraints

- MSRV Rust 1.85; Edition 2024. No newer language/stdlib features.
- Clippy `-D warnings` across all platforms; zero warnings.
- All theme/styling stays in `src/app/theme/`; no inline styles.
- Lean process (refbox UI, layout-only): no per-task code review, no
  fabricated unit tests for layout. Compile + `just check` + walkthrough.
- Do NOT touch `keypad_pages/team_timeout_edit.rs` — already top-pinned by
  PR #1188, out of scope.
- Branch: `refactor/refbox/editor-top-spacing` (worktree
  `.worktrees/editor-top-spacing`, cut from `origin/master` @ 8f92bc8c).
- Commit format: `refactor(refbox): <description>`.

---

### Task 1: Game-time edit screen — top-pin editor, move note below

**Files:**
- Modify: `refbox/src/app/view_builders/time_edit.rs` (the final `column![...]`
  in `build_time_edit_view`).

**Interfaces:**
- Consumes: existing `edit_row` (a `row` built earlier in the function),
  `make_game_time_button(...)`, `fl!("Note-Game-time-is-paused")`.
- Produces: nothing consumed by other tasks.

- [ ] **Step 1: Reorder the column**

Replace this block:

```rust
    column![
        make_game_time_button(
            snapshot,
            false,
            true,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        vertical_space(),
        text(fl!("Note-Game-time-is-paused"))
            .size(SMALL_TEXT)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        vertical_space(),
        edit_row,
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
```

with this block (editor directly under the status bar; note moved below the
editor; single flexible spacer before the footer):

```rust
    column![
        make_game_time_button(
            snapshot,
            false,
            true,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        edit_row,
        text(fl!("Note-Game-time-is-paused"))
            .size(SMALL_TEXT)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::TimeEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
```

(`vertical_space` is still used once before the footer, so its import stays
live — no unused-import warning.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds with no errors and no new warnings.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/time_edit.rs
git commit -m "refactor(refbox): top-pin game-time editor, note below"
```

---

### Task 2: Game-parameter editors — top-pin editor box

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (the column-assembly
  tail of `build_game_parameter_editor`, after `editor_row` is built).

**Interfaces:**
- Consumes: existing `col` (a `column` already holding the status bar and the
  optional Half-Length selector), `editor_row`, `validity_note`.
- Produces: nothing consumed by other tasks.

- [ ] **Step 1: Remove the two spacers around the editor**

Replace this block:

```rust
    col = col
        .push(vertical_space())
        .push(editor_row)
        .push(vertical_space());

    if let Some(note) = validity_note {
        col = col.push(note);
    }

    col.push(vertical_space())
        .push(
            row![
                make_button(fl!("cancel"))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
                horizontal_space(),
                make_button(fl!("done"))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press_maybe(
                        (!matches!(game_block_validity, Some(GameBlockValidity::TooShort)))
                            .then_some(Message::ParameterEditComplete { canceled: false }),
                    ),
            ]
            .spacing(SPACING),
        )
        .into()
```

with this block (editor pushed directly onto `col` with no leading or trailing
spacer; validity note directly under it; single flexible spacer before footer):

```rust
    col = col.push(editor_row);

    if let Some(note) = validity_note {
        col = col.push(note);
    }

    col.push(vertical_space())
        .push(
            row![
                make_button(fl!("cancel"))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
                horizontal_space(),
                make_button(fl!("done"))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press_maybe(
                        (!matches!(game_block_validity, Some(GameBlockValidity::TooShort)))
                            .then_some(Message::ParameterEditComplete { canceled: false }),
                    ),
            ]
            .spacing(SPACING),
        )
        .into()
```

(`vertical_space` remains used before the footer and on other pages in this
file, so its import stays live.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds with no errors and no new warnings.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "refactor(refbox): top-pin game-parameter editor box"
```

---

### Task 3: Full validation + visual walkthrough

**Files:** none (verification only).

- [ ] **Step 1: Run the full check suite**

Run: `just check`
Expected: fmt, lint, test, audit all clean.

- [ ] **Step 2: Rebuild the run binary explicitly**

Run: `cargo build -p refbox`
Expected: success. (`just check` builds a separate test binary and does NOT
refresh `target/debug/refbox`, so this rebuild is required before launching.)

- [ ] **Step 3: Visual walkthrough**

Launch the built binary and confirm against the spec's acceptance criteria:
1. A normal parameter editor (e.g. Half Time): editor box sits just under the
   status bar, not floating in the middle.
2. Half Length editor: editor box sits just under the 2-HALVES/1-PERIOD
   selector.
3. Game Block editor: as the red "too short" / yellow "tight" note appears and
   disappears, the editor box does NOT move.
4. Game-time edit screen: editor box just under the status bar, and the
   "time is paused" note appears BELOW the editor.
5. Footer (Cancel / Done) stays pinned at the bottom on every screen.

---

## Self-Review

- **Spec coverage:** configuration.rs param editor (Task 2), time_edit.rs +
  note relocation (Task 1), team_timeout_edit untouched (Global Constraints),
  all 5 acceptance criteria (Task 3 walkthrough). No gaps.
- **Placeholder scan:** none — every code step shows full before/after.
- **Type consistency:** no new symbols introduced; edits only remove/reorder
  `vertical_space()` and move an existing `text(...)` note.
