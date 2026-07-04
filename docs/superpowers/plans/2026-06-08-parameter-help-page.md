# Parameter Help Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the inline "HELP:" text off each length-parameter edit screen onto a dedicated help page reached by a small blue `?` button in the editor's upper-right corner, so long translated help strings can no longer push the Cancel/Done buttons off the bottom of the screen.

**Architecture:** This is a fresh re-implementation of the design recorded in ADR 007 (`docs/decisions/007-help-text-layout.md`) and the stale April branch `feat/refbox/help-expand-page`. The original branch is 346 commits behind master and conflicts in both code files, so we re-write it against current master — reusing only the design, not the patch. A new navigation state `AppState::ParameterEditorHelp` mirrors the existing `AppState::ParameterEditor` (same three fields). Two new messages (`ShowParameterHelp`, `CloseParameterHelp`) toggle between the editor and the help page. The help page reuses the editor's existing (already-translated) short title and the existing hint strings — **no new translation keys are added**.

**Tech Stack:** Rust 2024, iced 0.13, Fluent (`fl!`) localization. Crate: `refbox` only.

**Scope boundary:**
- Crate touched: `refbox` only. No `uwh-common`, no wire format, no other crate.
- Files touched: `refbox/src/app/message.rs`, `refbox/src/app/mod.rs`, `refbox/src/app/view_builders/configuration.rs`. No `.ftl` files.
- Explicitly NOT changing: which parameters have help text, the wording of any help text, or any other edit/config screen that does not currently show this inline HELP block. NOT reviving/merging the stale branch.

**Design decisions confirmed with the operator (2026-06-08):**
- `?` button placement: **upper-right corner** of the editor, small blue button, next to the time editor.
- Help-page heading: **reuse the editor's existing short title** (e.g. "HALF LEN"). No new full-word title keys → zero new translations.

**A note on testing:** `refbox` view-builders are iced `Element` trees with no existing unit-test harness; they are verified by compiling and by manually walking the UI (the same way the original April work and ADR 007 were verified). So tasks here verify via `cargo build` / `cargo clippy` (the compiler enforces exhaustive `match` arms and message wiring) plus a final manual walkthrough — not via failing unit tests. This matches the lean process in `.claude/rules/plan-execution.md` for refbox UI work.

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `refbox/src/app/message.rs` | The `Message` enum + its `is_repeatable`-style list and `PartialEq` | Add `ShowParameterHelp` / `CloseParameterHelp` variants in all four required places |
| `refbox/src/app/mod.rs` | `AppState` enum, `update()` message handlers, `view()` dispatch | Add `ParameterEditorHelp` state, two handlers, one view-dispatch arm |
| `refbox/src/app/view_builders/configuration.rs` | The editor view-builder + new help-page view-builder | Remove inline HELP text, add `?` button, add `build_parameter_help_page()` |

---

## Task 1: Add the two new messages

**Files:**
- Modify: `refbox/src/app/message.rs`

The `Message` enum here is matched exhaustively in several places in the same file (a "no-payload messages" grouping list, a same-variant `PartialEq` arm list, and a catch-all `PartialEq` arm list). All four insertion points use `ShowWarnings` as a stable neighbour — add the two new variants right after it in each spot. Missing any one of these is a compile error, which is the verification.

- [ ] **Step 1: Add the enum variants**

In the `pub enum Message { ... }` definition, find the line `ShowWarnings,` (around line 110) and add the two variants immediately after it:

```rust
    ShowWarnings,
    ShowParameterHelp,
    CloseParameterHelp,
```

- [ ] **Step 2: Add to the no-payload message grouping list**

Find the grouping match arm list that contains `| Self::ShowWarnings` (around line 289 — the block ending with payload-less variants). Add immediately after it:

```rust
            | Self::ShowWarnings
            | Self::ShowParameterHelp
            | Self::CloseParameterHelp
```

- [ ] **Step 3: Add the same-variant `PartialEq` arms**

Find the `PartialEq` impl's same-variant block containing `| (Self::ShowWarnings, Self::ShowWarnings)` (around line 363). Add immediately after it:

```rust
            | (Self::ShowWarnings, Self::ShowWarnings)
            | (Self::ShowParameterHelp, Self::ShowParameterHelp)
            | (Self::CloseParameterHelp, Self::CloseParameterHelp)
```

- [ ] **Step 4: Add the catch-all `PartialEq` arms**

Find the `PartialEq` catch-all block containing `| (Self::ShowWarnings, _)` (around line 565). Add immediately after it:

```rust
            | (Self::ShowWarnings, _)
            | (Self::ShowParameterHelp, _)
            | (Self::CloseParameterHelp, _)
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p refbox`
Expected: compiles (with a warning that the new messages are never constructed/handled yet — that is fixed in Tasks 2-3). No errors.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs
git commit -m "feat(refbox): add ShowParameterHelp/CloseParameterHelp messages"
```

---

## Task 2: Add the `ParameterEditorHelp` state and its handlers

**Files:**
- Modify: `refbox/src/app/mod.rs`

`AppState::ParameterEditor(LengthParameter, Duration, bool)` is the existing editor state (the third field is `single_half`). The new help state carries the identical three fields so we can round-trip back to the exact editor we came from.

- [ ] **Step 1: Add the state variant**

Find `ParameterEditor(LengthParameter, Duration, bool),` in the `enum AppState { ... }` (around line 195) and add the help state immediately after it:

```rust
    ParameterEditor(LengthParameter, Duration, bool),
    ParameterEditorHelp(LengthParameter, Duration, bool),
```

- [ ] **Step 2: Add the two message handlers**

Find the end of the `Message::EditParameter(param) => { ... }` handler block in `update()` (around line 2365-2393, the arm that sets `self.app_state = AppState::ParameterEditor(...)`). Immediately after that arm's closing `}`, add:

```rust
            Message::ShowParameterHelp => {
                if let AppState::ParameterEditor(param, dur, single_half) = self.app_state {
                    self.app_state = AppState::ParameterEditorHelp(param, dur, single_half);
                    trace!("AppState changed to {:?}", self.app_state);
                }
                Task::none()
            }
            Message::CloseParameterHelp => {
                if let AppState::ParameterEditorHelp(param, dur, single_half) = self.app_state {
                    self.app_state = AppState::ParameterEditor(param, dur, single_half);
                    trace!("AppState changed to {:?}", self.app_state);
                }
                Task::none()
            }
```

- [ ] **Step 3: Add the view-dispatch arm**

Find the `view()` dispatch arm for `AppState::ParameterEditor` (around line 3857). On current master it spans several lines because the editor takes a 5th `config` argument:

```rust
            AppState::ParameterEditor(param, dur, single_half) => build_game_parameter_editor(
                data,
                param,
                dur,
                single_half,
                self.edited_settings
                    .as_ref()
                    .map_or(&self.config.game, |s| &s.config),
            ),
```

Leave that arm exactly as-is and add the new help arm immediately after it. The help page needs no config (it shows text only, no validation):

```rust
            AppState::ParameterEditorHelp(param, dur, single_half) => {
                build_parameter_help_page(data, param, dur, single_half)
            }
```

(`build_parameter_help_page` does not exist yet — it is created in Task 3. This file will not compile until Task 3 is done; that is expected.)

- [ ] **Step 4: Confirm no other `AppState` match needs an arm**

Run: `cargo build -p refbox` and read the errors.
Expected at this point: errors about `build_parameter_help_page` not being found (resolved in Task 3). The compiler will ALSO flag any other non-exhaustive `match self.app_state` that lacks a `_` arm. The known editing-only matches (the EditTimeParameter duration-extraction match around line 1432, and the `ParameterEditComplete` matches around lines 2420 and 2498) all end in `_ => unreachable!()` and the help state never reaches them — do NOT add arms there. ONLY add a `ParameterEditorHelp` arm where the compiler reports a non-exhaustive match; in each such case mirror the existing `ParameterEditor` arm's behaviour. If the compiler reports none beyond the missing function, do nothing extra.

- [ ] **Step 5: Commit (after Task 3 makes it compile)**

This task is committed together with Task 3 since the file does not compile standalone. Proceed to Task 3.

---

## Task 3: Editor `?` button + the help page view-builder

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`

> **IMPORTANT — code re-grounded against current master (2026-06-08).** Since ADR 007 was written, the Game Block feature landed on master and reshaped this editor. The code below reflects the **actual current** `build_game_parameter_editor` (function starts at line 1406). Key realities to preserve:
> - The function now takes a **5th parameter** `config: &GameConfig`.
> - The length-parameter list uses **`LengthParameter::GameBlock`** (title `game-block`, hint `game-block-help`) — there is **no `NominalBetweenGame`** anymore. The full set is: `Half`, `HalfTime`, `GameBlock`, `MinimumBetweenGame`, `PreOvertime`, `OvertimeHalf`, `OvertimeHalfTime`, `PreSuddenDeath`.
> - The editor has **live Game Block validation** producing `value_color` (passed to `make_time_editor`) and an optional `validity_note` element, and the Done button uses `on_press_maybe` to disable when the block is too short. **All of this must be preserved** — it is unrelated to the help text we are relocating.
> - `make_game_time_button(...)` now takes **7 args** (trailing `None`); `make_time_editor(title, length, false, value_color)` now takes **4 args**.
>
> All helpers/constants used below are in scope via `use super::{ViewData, fl, message::*, shared_elements::*, theme::*};` and `use iced::{Alignment, Element, Length, alignment::{Horizontal, Vertical}, ...}`: `make_small_button(label, size)`, `make_button`, `make_time_editor`, `make_game_time_button`, `blue_button`, `red_button`, `green_button`, `MEDIUM_TEXT`, `SMALL_TEXT`, `MIN_BUTTON_SIZE`, `SPACING`, `container`, `text`, `row`, `column`, `vertical_space`, `horizontal_space`, `Vertical::Top`.

### 3a: Modify `build_game_parameter_editor` — drop inline help, add `?` button

- [ ] **Step 1: Change the `match param` to produce only the title**

In `build_game_parameter_editor` (starts at line 1406), the current `let (title, hint) = match param { ... };` produces a `(title, hint)` tuple. After this task the editor no longer renders `hint`, so change it to produce only `title` (keeping the `single_half` conditional for `Half`). Replace the whole `let (title, hint) = match param { ... };` block with:

```rust
    let title = match param {
        LengthParameter::Half => {
            if single_half {
                fl!("game-len")
            } else {
                fl!("half-length")
            }
        }
        LengthParameter::HalfTime => fl!("half-time-lenght"),
        LengthParameter::GameBlock => fl!("game-block"),
        LengthParameter::MinimumBetweenGame => fl!("min-break"),
        LengthParameter::PreOvertime => fl!("pre-ot-break-abreviated"),
        LengthParameter::OvertimeHalf => fl!("ot-half-len"),
        LengthParameter::OvertimeHalfTime => fl!("ot-half-tm-len"),
        LengthParameter::PreSuddenDeath => fl!("pre-sd-break"),
    };
```

(Everything that follows the match — the `game_block_validity` / `value_color` / `validity_note` blocks and the Half-only `format_selector` block — is unchanged. Leave all of it exactly as-is.)

- [ ] **Step 2: Replace the time-editor push with a `?`-button row; remove the inline help text**

The current tail of the function (after the `format_selector` block) reads:

```rust
    col = col
        .push(vertical_space())
        .push(make_time_editor(title, length, false, value_color))
        .push(vertical_space())
        .push(
            text(fl!("help") + &hint)
                .size(SMALL_TEXT)
                .align_x(Horizontal::Center),
        );

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

Replace that entire tail with the following. This (a) wraps the time editor in a row with a small blue `?` button pinned top-right, (b) deletes the inline `text(fl!("help") + &hint)` push, and (c) keeps the `validity_note` push and the `on_press_maybe` Done button untouched:

```rust
    let help_button = make_small_button("?", MEDIUM_TEXT)
        .style(blue_button)
        .on_press(Message::ShowParameterHelp);

    // Time editor stays centred between two balancing spacers; the ? button sits
    // top-right (its width matched by the fixed-width spacer on the left), and
    // align_y(Top) pins it to the top of the row.
    let editor_row = row![
        horizontal_space().width(Length::Fixed(MIN_BUTTON_SIZE)),
        horizontal_space(),
        make_time_editor(title, length, false, value_color),
        horizontal_space(),
        help_button,
    ]
    .spacing(SPACING)
    .align_y(Vertical::Top);

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

Note: removing the inline `text(fl!("help") + &hint)` removes the only remaining use of `hint`, which is why Step 1 drops `hint` from the match. The `text` and `Horizontal` imports remain used elsewhere in the file, so no unused-import warnings result.

### 3b: Add `build_parameter_help_page`

- [ ] **Step 3: Add the new function**

Immediately after the closing `}` of `build_game_parameter_editor` (just before `fn font_family_id`), add. Note the match arms mirror the editor's current variants exactly (including `GameBlock`), and `make_game_time_button` takes the trailing `None`:

```rust
pub(in super::super) fn build_parameter_help_page<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    _length: Duration,
    single_half: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    // Title reuses the editor's short, already-translated label; body is the
    // existing hint string. No new translation keys are introduced.
    let (title, body) = match param {
        LengthParameter::Half => (
            if single_half {
                fl!("game-len")
            } else {
                fl!("half-length")
            },
            if single_half {
                fl!("length-of-game-during-regular-play")
            } else {
                fl!("length-of-half-during-regular-play")
            },
        ),
        LengthParameter::HalfTime => (fl!("half-time-lenght"), fl!("length-of-half-time-period")),
        LengthParameter::GameBlock => (fl!("game-block"), fl!("game-block-help")),
        LengthParameter::MinimumBetweenGame => (fl!("min-break"), fl!("min-time-btwn-games")),
        LengthParameter::PreOvertime => (fl!("pre-ot-break-abreviated"), fl!("pre-sd-brk")),
        LengthParameter::OvertimeHalf => (fl!("ot-half-len"), fl!("time-during-ot")),
        LengthParameter::OvertimeHalfTime => {
            (fl!("ot-half-tm-len"), fl!("len-of-overtime-halftime"))
        }
        LengthParameter::PreSuddenDeath => (fl!("pre-sd-break"), fl!("pre-sd-len")),
    };
    let body = body.replace('\n', " ");

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running, portal_indicator, None),
        container(text(title).size(MEDIUM_TEXT)).center_x(Length::Fill),
        text(body).size(SMALL_TEXT).width(Length::Fill),
        vertical_space(),
        row![
            make_button(fl!("back"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::CloseParameterHelp),
            horizontal_space(),
            horizontal_space(),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}
```

- [ ] **Step 4: Verify the whole workspace compiles and lints clean**

Run: `cargo build -p refbox`
Expected: PASS (the `build_parameter_help_page` reference from Task 2 now resolves).

Run: `cargo clippy -p refbox -- -D warnings`
Expected: PASS, zero warnings. (Per `reference_refbox_bin_crate_clippy_scope`, use `-p refbox` without `--all-targets` to mirror CI/`just lint`.)

- [ ] **Step 5: Commit Tasks 2 + 3 together**

```bash
git add refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): move length-parameter help text to a dedicated page"
```

---

## Task 4: Manual walkthrough verification

**Files:** none (verification only)

There is no unit-test harness for these iced views, so confirm behaviour by running the app. Per `feedback_user_drives_refbox_ui` and `feedback_refbox_wsl_wayland_unset`, the executor launches the app and the operator drives the UI.

- [ ] **Step 1: Launch the app**

Run (background, sandbox disabled, X11 forced for WSL):
`WAYLAND_DISPLAY= cargo run -p refbox`

- [ ] **Step 2: Walkthrough checklist (operator confirms each)**

1. Game Options → tap a length parameter (e.g. Half Length). The editor shows the time keypad with a small blue `?` in the upper-right and Cancel/Done visible at the bottom. The old inline "HELP:" line is gone.
2. For Half Length, the 2 HALVES / 1 PERIOD selector still appears above the editor and still works.
3. Tap `?` → the help page appears: short title heading, the full help text below, and a red BACK button. No text overflows the buttons.
4. Tap BACK → returns to the same editor with the same staged value and the same 2 HALVES / 1 PERIOD choice intact.
5. Repeat steps 3-4 for a long-text parameter (e.g. Game Block, whose help text is long) in a long language (switch to German/Italian) and confirm Cancel/Done stay on-screen the whole time.
6. On the Game Block editor specifically, confirm the live validation is intact: a too-short value still colours the value red and disables Done; the "too short"/"tight" note still appears below the editor.
7. On the editor, change the value and tap Done → the value is saved as before (the `?`/help round-trip did not disturb editing).

- [ ] **Step 3: Run the full local gate before any PR**

Run: `just check`
Expected: fmt, lint, tests, audit all clean.

---

## Task 5: Clean up the stale branch (only after this work is committed)

**Files:** none (git housekeeping)

The old `feat/refbox/help-expand-page` branch (local only, no remote, no PR) is superseded by this work.

- [ ] **Step 1: Delete the stale local branch**

```bash
git branch -D feat/refbox/help-expand-page
```

Expected: branch deleted. Nothing is lost — it had no remote tracking branch and no open PR; its design lives on in ADR 007 and this plan.

---

## Deviations

(Record any execution deviations from this plan here, per the lean process in `.claude/rules/plan-execution.md`.)

---

## Self-Review

- **Spec coverage:** ADR 007's decision (collapse inline help → expand page; preview bounded so Cancel/Done stay on-screen) is realized by Task 3a (remove inline text, add `?`) + Task 3b (dedicated page). Two-message round-trip = Tasks 1 + 2. Confirmed UI decisions (upper-right `?`, reuse short title) are baked into Task 3.
- **No new translation keys:** confirmed — every `fl!` key the help page uses (`game-len`, `half-length`, `half-time-lenght`, `game-block`, `min-break`, `pre-ot-break-abreviated`, `ot-half-len`, `ot-half-tm-len`, `pre-sd-break`, the body hints incl. `game-block-help`, and `back`) is already referenced by the existing editor on master, so all exist in all 15 locales. `feedback_translate_all_locales_no_placeholders` imposes no work here.
- **Type consistency:** `ParameterEditorHelp(LengthParameter, Duration, bool)` matches `ParameterEditor`'s three fields everywhere it is constructed/destructured (Task 2 handlers + Task 2 view dispatch + Task 3 function signature `(data, param, dur, single_half)`). `build_parameter_help_page` signature in Task 3 matches the call site in Task 2.
- **Placeholder scan:** every code step shows complete code; no TODO/TBD.
