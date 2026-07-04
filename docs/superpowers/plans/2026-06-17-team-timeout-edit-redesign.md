# Team-Timeout Settings Page Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the team-timeout settings page's shared number-pad layout with a clean full-width panel: a 0/1 count toggle, a HALF/GAME toggle, preset length buttons, and a Cancel/Apply footer — with the toggles/presets disabled when the count is 0.

**Architecture:** The page stays a `KeypadPage::TeamTimeouts(duration, per_half)` AppState (the count lives in the existing `player_num` slot). `build_keypad_page` special-cases this page to render full-width (no number pad). Two new `Message` variants set the count and length directly (the number pad appended digits; the `+/−` editor was relative). The page view is rewritten; disabled controls come "for free" by omitting `on_press` (the `blue_button` style renders `Status::Disabled` greyed).

**Tech Stack:** Rust 2024, `iced` 0.13, Fluent (`fl!`) translations.

**Process note (lean):** This is `refbox` UI work — no `uwh-common`, no wire format, no timing state machine. Per `.claude/rules/plan-execution.md`, message-enum wire-up and view rebuilds are mechanical: compilation + `just check` + the manual walkthrough in Task 5 is sufficient verification. Do **not** add per-task TDD ceremony for the view rendering (there is no existing view-test harness for keypad pages to mirror).

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `refbox/translations/<locale>/refbox.ftl` (×15) | UI text | Add one key `team-timeout-count` |
| `refbox/src/app/message.rs` | `Message` enum | Add `SetTeamTimeoutCount(u32)` and `SetTeamTimeoutLength(Duration)` |
| `refbox/src/app/mod.rs` | `update()` handlers | Handle the two new messages (mutate `TeamTimeouts` AppState) |
| `refbox/src/app/view_builders/keypad_pages/team_timeout_edit.rs` | The page view | Full rewrite to Option A layout |
| `refbox/src/app/view_builders/keypad_pages/mod.rs` | Keypad page composition | Early-return full-width branch for `TeamTimeouts`; pass `player_num`; make old match arm `unreachable!()` |

---

## Task 1: Add the `team-timeout-count` translation key (all 15 locales)

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl` (and the other 14 locale files listed below)

The key follows the existing Fluent multiline style (value continues on an indented second line), placed in the existing `# Team Timeout Edit` section near `timeout-length`. Each translation reuses that locale's existing "team timeout" wording (from its `timeout-length` value) plus that language's word for "count/number". These are best-guess translations to be sanity-checked by the user's community — do **not** leave any locale in English (per project translation rule).

- [ ] **Step 1: Add the key to en-US**

In `refbox/translations/en-US/refbox.ftl`, in the `# Team Timeout Edit` section (just after the `timeout-length` entry), add:

```ftl
team-timeout-count = TEAM TIMEOUT
    COUNT:
```

- [ ] **Step 2: Add the key to the other 14 locales**

Add the corresponding entry to each file (`refbox/translations/<locale>/refbox.ftl`), in the same section near `timeout-length`:

```ftl
# de-DE
team-timeout-count = AUSZEIT
    ANZAHL:

# es
team-timeout-count = NÚMERO DE
    TIEMPOS DE ESPERA:

# fr
team-timeout-count = NOMBRE DE
    TEMPS MORTS:

# id-ID
team-timeout-count = JUMLAH
    TIME-OUT TIM:

# it-IT
team-timeout-count = NUMERO DI
    TIME-OUT:

# ja-JP
team-timeout-count = チームタイムアウト
    回数:

# ko-KR
team-timeout-count = 팀 타임아웃
    횟수:

# ms-MY
team-timeout-count = BILANGAN
    MASA REHAT:

# nl-NL
team-timeout-count = AANTAL
    TEAM TIME-OUTS:

# pt-PT
team-timeout-count = NÚMERO DE
    TEMPOS DE EQUIPA:

# th-TH
team-timeout-count = จำนวน
    พักทีม:

# tl-PH
team-timeout-count = BILANG NG
    TIMEOUT:

# tr-TR
team-timeout-count = TAKIM MOLASI
    SAYISI:

# zh-CN
team-timeout-count = 队伍暂停
    次数:
```

- [ ] **Step 3: Verify the key parses and is present in every locale**

Run:
```bash
for l in de-DE en-US es fr id-ID it-IT ja-JP ko-KR ms-MY nl-NL pt-PT th-TH tl-PH tr-TR zh-CN; do
  grep -q "team-timeout-count" "refbox/translations/$l/refbox.ftl" && echo "$l OK" || echo "$l MISSING";
done
```
Expected: all 15 print `OK`.

- [ ] **Step 4: Commit**

```bash
git add refbox/translations/
git commit -m "feat(refbox): add team-timeout-count translation key"
```

---

## Task 2: Add the two new messages and their handlers

**Files:**
- Modify: `refbox/src/app/message.rs` (the `Message` enum)
- Modify: `refbox/src/app/mod.rs` (the `update()` match)

The count is stored in the `player_num` slot of `AppState::KeypadPage(KeypadPage::TeamTimeouts(dur, per_half), player_num)` (a `u32`). The length is the `dur` field. The number pad appended digits (so it can't set 0 when already 1), and `ChangeTime` is relative — so we add two direct-set messages.

- [ ] **Step 1: Add the message variants**

In `refbox/src/app/message.rs`, add to the `Message` enum (place them near `ChangeTime` / `ToggleBoolParameter`). `Duration` is already imported in this file (used by `KeypadPage::TeamTimeouts(Duration, bool)`):

```rust
    /// Set the team-timeout count directly (team-timeout edit page 0/1 toggle).
    SetTeamTimeoutCount(u32),
    /// Set the team-timeout length to a preset value (team-timeout edit page).
    SetTeamTimeoutLength(Duration),
```

- [ ] **Step 2: Add the handlers in `update()`**

In `refbox/src/app/mod.rs`, in the `update()` match (place right after the `Message::KeypadButtonPress(key) => { ... }` arm, around line 1990), add:

```rust
            Message::SetTeamTimeoutCount(count) => {
                if let AppState::KeypadPage(KeypadPage::TeamTimeouts(_, _), ref mut val) =
                    self.app_state
                {
                    *val = count;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
            Message::SetTeamTimeoutLength(new_len) => {
                if let AppState::KeypadPage(KeypadPage::TeamTimeouts(ref mut dur, _), _) =
                    self.app_state
                {
                    *dur = new_len;
                } else {
                    unreachable!()
                }
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
```

- [ ] **Step 3: Build to verify the enum + handlers compile**

Run: `cargo build -p refbox`
Expected: compiles. (The new messages aren't emitted by any view yet — that's Task 3. No "unused variant" warning: enum variants don't warn when unused.)

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add direct set messages for team-timeout count and length"
```

---

## Task 3: Rewrite the team-timeout edit page (Option A layout)

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/team_timeout_edit.rs` (full rewrite)

The page builder now also receives the current count (`player_num`). When the count is 0, the HALF/GAME buttons and length presets are rendered without `on_press`, so the `blue_button`/`blue_selected_button` styles draw them in their greyed `Status::Disabled` state. The count buttons are always interactive. The footer confirm button changes from `fl!("done")` to `fl!("apply")`.

- [ ] **Step 1: Replace the file contents**

Replace the entire contents of `refbox/src/app/view_builders/keypad_pages/team_timeout_edit.rs` with:

```rust
use super::*;
use iced::{
    Length, Theme,
    widget::{
        button::{Status, Style},
        column, row, text, vertical_space,
    },
};
use std::time::Duration;

type StyleFn = fn(&Theme, Status) -> Style;

/// Length presets shown on the team-timeout edit page: (label, seconds).
const LENGTH_PRESETS: [(&str, u64); 5] = [
    ("0:30", 30),
    ("0:45", 45),
    ("1:00", 60),
    ("1:15", 75),
    ("1:30", 90),
];

pub(super) fn make_team_timeout_edit_page<'a>(
    duration: Duration,
    timeouts_counted_per_half: bool,
    count: u32,
) -> Element<'a, Message> {
    // Count 0 means "no team timeouts": the period and length controls are
    // meaningless, so they render disabled (greyed, non-pressable). The count
    // buttons themselves stay active. Any non-zero count shows "1" selected.
    let zero_selected = count == 0;
    let count_enabled = !zero_selected;

    let (zero_style, zero_msg): (StyleFn, _) = if zero_selected {
        (blue_selected_button, Message::NoAction)
    } else {
        (blue_button, Message::SetTeamTimeoutCount(0))
    };
    let (one_style, one_msg): (StyleFn, _) = if zero_selected {
        (blue_button, Message::SetTeamTimeoutCount(1))
    } else {
        (blue_selected_button, Message::NoAction)
    };

    let count_row = row![
        text(fl!("team-timeout-count"))
            .size(SMALL_PLUS_TEXT)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .align_y(Vertical::Center),
        make_button("0")
            .style(zero_style)
            .width(Length::Fill)
            .on_press(zero_msg),
        make_button("1")
            .style(one_style)
            .width(Length::Fill)
            .on_press(one_msg),
    ]
    .spacing(SPACING);

    // HALF/GAME toggle. Styles always reflect the current selection so the
    // operator can still see the chosen period while disabled; on_press is
    // only attached when the count is non-zero.
    let half_style: StyleFn = if timeouts_counted_per_half {
        blue_selected_button
    } else {
        blue_button
    };
    let game_style: StyleFn = if timeouts_counted_per_half {
        blue_button
    } else {
        blue_selected_button
    };
    let (half_msg, game_msg) = if timeouts_counted_per_half {
        (
            Message::NoAction,
            Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
        )
    } else {
        (
            Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
            Message::NoAction,
        )
    };
    let mut half_button = make_button(fl!("half"))
        .style(half_style)
        .width(Length::Fill);
    let mut game_button = make_button(fl!("game"))
        .style(game_style)
        .width(Length::Fill);
    if count_enabled {
        half_button = half_button.on_press(half_msg);
        game_button = game_button.on_press(game_msg);
    }

    let counted_per_row = row![
        text(fl!("timeouts-counted-per"))
            .size(SMALL_PLUS_TEXT)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .align_y(Vertical::Center),
        half_button,
        game_button,
    ]
    .spacing(SPACING);

    // Length presets. Selected = the preset matching the current duration.
    let make_preset = |label: &'a str, secs: u64| -> Element<'a, Message> {
        let preset_dur = Duration::from_secs(secs);
        let selected = duration == preset_dur;
        let style: StyleFn = if selected {
            blue_selected_button
        } else {
            blue_button
        };
        let mut b = make_button(label).style(style).width(Length::Fill);
        if count_enabled {
            b = b.on_press(if selected {
                Message::NoAction
            } else {
                Message::SetTeamTimeoutLength(preset_dur)
            });
        }
        b.into()
    };

    let presets_row = row![
        make_preset(LENGTH_PRESETS[0].0, LENGTH_PRESETS[0].1),
        make_preset(LENGTH_PRESETS[1].0, LENGTH_PRESETS[1].1),
        make_preset(LENGTH_PRESETS[2].0, LENGTH_PRESETS[2].1),
        make_preset(LENGTH_PRESETS[3].0, LENGTH_PRESETS[3].1),
        make_preset(LENGTH_PRESETS[4].0, LENGTH_PRESETS[4].1),
    ]
    .spacing(SPACING);

    let length_block = column![
        text(fl!("timeout-length"))
            .size(SMALL_PLUS_TEXT)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .align_y(Vertical::Center),
        presets_row,
    ]
    .spacing(SPACING);

    column![
        count_row,
        counted_per_row,
        vertical_space(),
        length_block,
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
```

> Note on imports: `make_button`, `blue_button`, `blue_selected_button`, `red_button`,
> `green_button`, `SMALL_PLUS_TEXT`, `MIN_BUTTON_SIZE`, `SPACING`, `Vertical`, `Element`,
> `Message`, `BoolGameParameter`, and the `fl!` macro all come in via `use super::*;`
> (they are used by the current version of this file). Only `text`, `column`, `row`,
> `vertical_space`, `Length`, `Theme`, `Status`, `Style`, and `Duration` need the explicit
> `use` lines shown above. `horizontal_space` and `make_time_editor` are no longer used.

- [ ] **Step 2: Build (expect ONE error at the call site)**

Run: `cargo build -p refbox`
Expected: FAILS — `make_team_timeout_edit_page` now takes 3 args but `keypad_pages/mod.rs` still calls it with 2. This is fixed in Task 4. (If any *other* error appears — e.g. an unused import — fix it here before moving on.)

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/keypad_pages/team_timeout_edit.rs
git commit -m "feat(refbox): rebuild team-timeout edit page as full-width preset panel"
```

---

## Task 4: Render the page full-width (skip the shared number pad)

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/mod.rs`

`build_keypad_page` hardcodes the 0–9 number pad on the left for every page. For `TeamTimeouts` we return early with just the game-time bar + the full-width panel, so the number pad isn't drawn. The existing `match page` arm for `TeamTimeouts` then becomes unreachable.

- [ ] **Step 1: Add the early-return branch**

In `refbox/src/app/view_builders/keypad_pages/mod.rs`, inside `build_keypad_page`, immediately **after** the `let enabled = match page { ... };` block (and before `let setup_keypad_button = ...`), add:

```rust
    // The team-timeout settings page does not use the shared number pad; it
    // renders as a full-width panel below the game-time bar.
    if let KeypadPage::TeamTimeouts(dur, per_half) = &page {
        let (dur, per_half) = (*dur, *per_half);
        return column![
            make_game_time_button(snapshot, false, false, mode, clock_running, portal_indicator),
            make_team_timeout_edit_page(dur, per_half, player_num),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into();
    }
```

(`snapshot`, `mode`, `clock_running`, `portal_indicator` are already destructured from `data` at the top of the function; `column`, `Length`, `make_game_time_button`, and `make_team_timeout_edit_page` are already in scope.)

- [ ] **Step 2: Make the old match arm unreachable**

Still in `build_keypad_page`, find the `match page { ... }` that builds the right-hand content and change the `TeamTimeouts` arm from:

```rust
                KeypadPage::TeamTimeouts(dur, per_half) =>
                    make_team_timeout_edit_page(dur, per_half),
```

to:

```rust
                KeypadPage::TeamTimeouts(_, _) => {
                    unreachable!("TeamTimeouts is handled by the early return above")
                }
```

- [ ] **Step 3: Build**

Run: `cargo build -p refbox`
Expected: compiles cleanly.

- [ ] **Step 4: Lint**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: zero warnings. (Watch for an unused-import warning in `team_timeout_edit.rs` — e.g. if `horizontal_space` was left in. Remove any flagged import.)

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/keypad_pages/mod.rs
git commit -m "feat(refbox): render team-timeout edit page full-width without number pad"
```

---

## Task 5: Full check + manual walkthrough

**Files:** none (verification only)

- [ ] **Step 1: Run the full check suite**

Run: `just check`
Expected: fmt, lint, tests, audit all clean. (If `just fmt-check` flags formatting, run `just fmt` and amend the relevant commit.)

- [ ] **Step 2: Launch the app**

First build the real binary (clippy/just build a test binary, not `target/debug/refbox`):
```bash
cargo build -p refbox
```
Then launch (WSL X11 launch per project notes):
```bash
WAYLAND_DISPLAY= ./target/debug/refbox
```
(Run in background; the user drives the UI.)

- [ ] **Step 3: Walk through the acceptance criteria**

Navigate to the team-timeout settings page and confirm:
1. Full-width panel, **no number pad**.
2. Count row shows **TEAM TIMEOUT COUNT:** with `[0]` `[1]`; current value highlighted; tapping the other switches the highlight.
3. With count **1**: HALF/GAME and all five length presets are active; current length preset is highlighted; default is **1:00**.
4. Set count to **0**: HALF/GAME and the presets grey out and can't be pressed; CANCEL and APPLY still work.
5. Switch **0 → 1**: previously highlighted HALF/GAME and length choices reappear.
6. Confirm button reads **APPLY**; pressing it saves count, counted-per, and length; CANCEL discards.
7. Open another keypad page (e.g. add score or a penalty) and confirm the number pad is unchanged there.

- [ ] **Step 4: Record the walkthrough result**

Note pass/fail for each criterion in the PR description (and in a "Deviations" note at the bottom of this plan if anything diverged).

---

## Self-Review (completed by plan author)

- **Spec coverage:** layout (Tasks 3–4), count 0/1 (Tasks 2–3), counted-per (Task 3), length presets incl. 1:00 default selected (Task 3), disable-when-0 (Task 3), preserved selections (styles still reflect state when disabled — Task 3), Apply label (Task 3), new `team-timeout-count` key in all locales (Task 1), no number pad for this page only (Task 4), other pages unchanged (Task 4 early-return is scoped to `TeamTimeouts`; verified in Task 5 step 3.7). All covered.
- **Placeholder scan:** none — every code/step is concrete.
- **Type consistency:** `make_team_timeout_edit_page(Duration, bool, u32)` defined in Task 3 and called with `(dur, per_half, player_num)` in Task 4. `SetTeamTimeoutCount(u32)` / `SetTeamTimeoutLength(Duration)` defined in Task 2, emitted in Task 3, handled in Task 2. `player_num` is `u32` (matches `build_keypad_page` signature and the AppState slot).
- **Note:** translations in Task 1 are best-guess and flagged for the user's community to verify.

---

## Deviations (recorded during execution)

1. **Plan under-specified two exhaustive `Message` matches.** Task 2 in the plan only added the
   enum variants + `update()` handlers, but the codebase has two more exhaustive matches over
   `Message` that required arms for the new variants to compile:
   - `Message::is_repeatable()` — new variants added to the `=> false` group (idempotent
     absolute sets; correct, since the dedup guard at `mod.rs:1523` should suppress exact
     consecutive duplicates).
   - manual `impl PartialEq for Message` — added both same-variant equality arms
     (`(SetTeamTimeoutCount(a), SetTeamTimeoutCount(b)) => a == b`, same for Length) and the
     different-variant catch-all (`(SetTeamTimeoutCount(_), _) | ... => false`).
2. **Transient dead-code at the Task 2 commit.** With the messages added but not yet emitted by
   any view, `clippy -D warnings` failed with "variants … never constructed". The plan assumed
   unused variants don't warn here — they do. Resolved by the Tasks 3+4 commit (the view emits
   them). The Task 2 commit (`b22902af`) carries a transient warning that the next commit clears;
   final branch is clippy-clean.
3. **`make_game_time_button` has 7 params, not 6.** The plan's early-return snippet omitted the
   trailing `overrun_label: Option<String>`; the real call passes `None` as the 7th arg.
4. **Tasks 3 and 4 committed together** (`880e2daa`) since Task 3 alone doesn't compile (call-site
   mismatch). Every commit on the branch compiles except the transient dead-code in (2).
5. **Controller path-bug mishap (corrected, no lasting effect).** While completing the
   interrupted Task 2, the controller briefly edited the *main checkout's* `message.rs` instead of
   the worktree copy (wrong absolute path); reverted with `git checkout -- refbox/src/app/message.rs`
   in the main checkout. Main checkout confirmed clean afterward (only the pre-existing
   `CLAUDE.md` modification remains). No effect on the branch.
6. **Open minor (not fixed — out of scope):** the `text_displayed` match in `keypad_pages/mod.rs`
   still lists `TeamTimeouts` in an or-pattern that is now unreachable at runtime (early return
   handles it). Harmless; left untouched per scope discipline.

## Status
- Tasks 1–4 implemented + committed (3 commits on `feat/refbox/team-timeout-edit-redesign`).
- `just check` green (fmt, lint, all tests pass, audit = pre-existing allowed advisories only).
- Final holistic code review: **approved, no required fixes.**
- Walkthrough DONE (user-verified). Post-walkthrough refinements applied (3 more commits, 6 total):
  unselected buttons → grey (`light_gray_button`); Cancel/Apply footer → config-page style with
  center gap; length presets reduced to 3 (0:30/1:00/1:30), label+presets on one row (label
  FillPortion(3), presets FillPortion(2) each); removed dead space between rows 2–3; colon added
  to `timeout-length` (15 locales); Game-Parameters tile → compact `TEAM TIMEOUTS: 0/1HALF/1GAME`
  via new `team-timeouts-label` key (15 locales) reusing Part A's compact value logic.
- Two bug fixes added: (1) count persists on reopen mid-config-edit — init from
  `edited_settings.config` not `self.config.game` (mod.rs ~2098); (2) value-button render artifact
  (iced cached text-anchor: removed `align_y(Center)`+`height(Fill)` from make_value_button text).
- `just check` green on final branch. **PR #1188 MERGED to master 2026-06-17** (merge-queue).
- REMAINING follow-up: best-guess translations (3 keys × 15 locales) need native/community review.
