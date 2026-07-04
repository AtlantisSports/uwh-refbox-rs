# "Cancel" → "Back" When Nothing Has Changed — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On every refbox config-style page that has a change-gated Apply button, the footer's red button reads **BACK** while there are no pending changes (Apply disabled) and **CANCEL** the moment a change is made.

**Architecture:** Two mechanisms, no behavioural change. (1) For the eight pages whose footer label comes from the translation system, a tiny shared helper `cancel_or_back_label(has_changes)` returns `fl!("cancel")` or `fl!("back")`, driven off the page's existing *has-changes* predicate (NOT `apply_enabled` — a page can have pending changes that Apply still refuses, e.g. an incomplete portal selection, and that must still read "Cancel"). (2) For the two language pickers, whose buttons render in the *previewed* language rather than the active locale, a new `Language::back_text()` mirrors the existing `Language::cancel_text()`. The button's `on_press` message is unchanged everywhere — pressing "Back" with no changes is the identical no-op revert/navigate that "Cancel" does today.

**Tech Stack:** Rust 2024, iced 0.13, `i18n-embed-fl` (`fl!` macro), existing `page_has_changes` / `param_edit_has_changes` predicates.

## Global Constraints

- MSRV Rust 1.85, edition 2024. No new dependencies.
- Clippy `-D warnings` must stay clean on all platforms.
- No new `.ftl` translation key is needed: `back = …` already exists in all 15 locales. The new `Language::back_text()` strings MUST match each locale's existing `back =` value verbatim (listed in Task 5).
- Literal labels are exactly `CANCEL` / `BACK` (via the existing `cancel` / `back` keys) — do not introduce new wording.
- Behaviour is label-only. Do NOT change any button's `on_press` message or any commit/save model (the buzzer and language Apply/confirm models stay as-is).
- Pages already showing "Back" (Main menu, User-options menu, Parameter-help page, BeepTest landing) and the Updates page are OUT OF SCOPE — do not touch them.

---

### Task 1: Shared `cancel_or_back_label` helper

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (add helper near `make_button`, ~line 1019; add a test in the file's `#[cfg(test)]` module)

**Interfaces:**
- Produces: `pub(super) fn cancel_or_back_label(has_changes: bool) -> String` — returns `fl!("cancel")` when `has_changes`, else `fl!("back")`. Re-exported to `configuration.rs` and `beep_test_settings.rs` via the existing `pub(super) use shared_elements::*` in `view_builders/mod.rs`.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `refbox/src/app/view_builders/shared_elements.rs` (create the block at the end of the file if none exists, with `use super::*;` as its first line):

```rust
#[test]
fn cancel_or_back_label_swaps_on_changes() {
    // Pending changes → the "cancel" label; nothing to discard → the "back" label.
    // Compared against the same fl! keys so the assertion holds regardless of which
    // locale the loader resolves to.
    assert_eq!(cancel_or_back_label(true), fl!("cancel"));
    assert_eq!(cancel_or_back_label(false), fl!("back"));
    assert_ne!(cancel_or_back_label(true), cancel_or_back_label(false));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox cancel_or_back_label_swaps_on_changes`
Expected: FAIL — `cannot find function cancel_or_back_label in this scope`.

- [ ] **Step 3: Write minimal implementation**

Add this function to `refbox/src/app/view_builders/shared_elements.rs`, immediately above the `pub(super) fn make_button<'a, ...` definition (~line 1019):

```rust
/// Label for a config-page footer's red button: `CANCEL` when the page has
/// pending edits to discard, `BACK` when it does not. "Cancel" implies
/// discarding changes; with nothing to discard the button is plain navigation,
/// so it reads "Back". Driven off the page's *has-changes* predicate, NOT its
/// Apply-enabled flag — a page can hold pending changes that Apply still
/// refuses (e.g. an incomplete portal selection), and those must still read
/// "Cancel". Mirrors the cancel/back swap already used in `make_updates_page`.
/// The two language pickers do not use this — they label their buttons in the
/// *previewed* language via `Language::back_text()`.
pub(super) fn cancel_or_back_label(has_changes: bool) -> String {
    if has_changes {
        fl!("cancel")
    } else {
        fl!("back")
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox cancel_or_back_label_swaps_on_changes`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): add cancel_or_back_label footer helper"
```

---

### Task 2: Wire the helper into both shared footers

Covers pages 1–4 (App / Display / Sound / Remotes via `make_cancel_apply_footer`) and 7–8 (BeepTest Sound / Edit Levels via `make_beep_test_cancel_apply_footer`). Mechanical label swap — verified by compilation + walkthrough, no unit test.

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs:420-466` (`make_cancel_apply_footer`)
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs:929-951` (`make_beep_test_cancel_apply_footer`)

**Interfaces:**
- Consumes: `cancel_or_back_label` (Task 1); existing `page_has_changes(page, edited, snapshot)`.

- [ ] **Step 1: Update `make_cancel_apply_footer`**

In `refbox/src/app/view_builders/configuration.rs`, replace:

```rust
    let apply_blocked = matches!(page, ConfigPage::Game) && edited.uwhportal_incomplete();
    let apply_enabled = page_has_changes(page, edited, snapshot) && !apply_blocked;

    let cancel = make_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(page));
```

with:

```rust
    let apply_blocked = matches!(page, ConfigPage::Game) && edited.uwhportal_incomplete();
    let has_changes = page_has_changes(page, edited, snapshot);
    let apply_enabled = has_changes && !apply_blocked;

    let cancel = make_button(cancel_or_back_label(has_changes))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(page));
```

- [ ] **Step 2: Update `make_beep_test_cancel_apply_footer`**

In `refbox/src/app/view_builders/beep_test_settings.rs`, replace:

```rust
    let cancel = make_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(cancel_message);
```

with:

```rust
    let cancel = make_button(cancel_or_back_label(has_changes))
        .style(red_button)
        .width(Length::Fill)
        .on_press(cancel_message);
```

(The `has_changes: bool` parameter is already in scope — it currently only gates Apply.)

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds with no errors or warnings.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "feat(refbox): swap shared footer Cancel->Back when unchanged"
```

---

### Task 3: Game Options inline footer

Page 5. The Game Options page builds its own footer. Label off `page_has_changes(ConfigPage::Game, …)` — NOT the fuller `apply_enabled`, so a pending-but-uncommittable edit (incomplete portal / too-short Game Block) still reads "Cancel".

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs:892-901` (inside `make_event_config_page`)

**Interfaces:**
- Consumes: `cancel_or_back_label` (Task 1); existing `page_has_changes`, `apply_blocked`, `game_block_too_short`.

- [ ] **Step 1: Update the Game footer**

Replace:

```rust
    let game_block_too_short =
        !using_uwhportal && matches!(game_block_validity(config), GameBlockValidity::TooShort);
    let apply_enabled = page_has_changes(ConfigPage::Game, settings, page_entry_snapshot)
        && !apply_blocked
        && !game_block_too_short;

    let cancel_btn = make_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::Game));
```

with:

```rust
    let game_block_too_short =
        !using_uwhportal && matches!(game_block_validity(config), GameBlockValidity::TooShort);
    let has_changes = page_has_changes(ConfigPage::Game, settings, page_entry_snapshot);
    let apply_enabled = has_changes && !apply_blocked && !game_block_too_short;

    let cancel_btn = make_button(cancel_or_back_label(has_changes))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::Game));
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): swap Game Options footer Cancel->Back when unchanged"
```

---

### Task 4: Parameter editor inline footer

Page 6 (Half Length / Game Block / Half Time / etc. editor, reached from Game Options). Label off `param_edit_has_changes(…)` — a changed-but-too-short Game Block still reads "Cancel".

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs:1664-1684` (end of `build_game_parameter_editor`)

**Interfaces:**
- Consumes: `cancel_or_back_label` (Task 1); existing `param_edit_has_changes`, `game_block_validity`, `old_length`.

- [ ] **Step 1: Update the parameter-editor footer**

Replace:

```rust
    let apply_enabled = !matches!(game_block_validity, Some(GameBlockValidity::TooShort))
        && param_edit_has_changes(length, old_length, param, single_half, config.single_half);

    col.push(vertical_space())
        .push(
            row![
                make_button(fl!("cancel"))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
```

with:

```rust
    let has_changes =
        param_edit_has_changes(length, old_length, param, single_half, config.single_half);
    let apply_enabled =
        !matches!(game_block_validity, Some(GameBlockValidity::TooShort)) && has_changes;

    col.push(vertical_space())
        .push(
            row![
                make_button(cancel_or_back_label(has_changes))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): swap parameter-editor footer Cancel->Back when unchanged"
```

---

### Task 5: `Language::back_text()`

The two language pickers render their buttons in the *previewed* language (not the active locale), so they cannot use `fl!`. Add a `back_text()` method mirroring the existing `cancel_text()`. The strings below are copied verbatim from each locale's `back =` value so the pickers stay consistent with the eight `fl!("back")` pages.

**Files:**
- Modify: `refbox/src/app/languages.rs` (add `back_text` method after `cancel_text`, ~line 99; add a `#[cfg(test)]` module at end of file)

**Interfaces:**
- Produces: `pub fn back_text(self) -> &'static str` on `Language`.

- [ ] **Step 1: Write the failing test**

Add to the end of `refbox/src/app/languages.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_text_matches_known_values() {
        assert_eq!(Language::English.back_text(), "BACK");
        assert_eq!(Language::French.back_text(), "RETOUR");
        assert_eq!(Language::German.back_text(), "ZURÜCK");
        assert_eq!(Language::Japanese.back_text(), "戻る");
        assert_eq!(Language::Mandarin.back_text(), "返回");
        // Back must read differently from Cancel in every language.
        for lang in [
            Language::English,
            Language::French,
            Language::Spanish,
            Language::Mandarin,
            Language::Korean,
            Language::Italian,
            Language::German,
            Language::Tagalog,
            Language::Indonesian,
            Language::Dutch,
            Language::Japanese,
            Language::Malay,
            Language::Portuguese,
            Language::Thai,
            Language::Turkish,
        ] {
            assert!(!lang.back_text().is_empty());
            assert_ne!(lang.back_text(), lang.cancel_text());
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox back_text_matches_known_values`
Expected: FAIL — `no method named back_text found`.

- [ ] **Step 3: Write minimal implementation**

Add this method immediately after the closing `}` of `cancel_text` (before `apply_text`) in `refbox/src/app/languages.rs`:

```rust
    pub fn back_text(self) -> &'static str {
        match self {
            Self::English => "BACK",
            Self::French => "RETOUR",
            Self::Spanish => "ATRÁS",
            Self::Mandarin => "返回",
            Self::Korean => "뒤로",
            Self::Italian => "INDIETRO",
            Self::German => "ZURÜCK",
            Self::Tagalog => "BUMALIK",
            Self::Indonesian => "KEMBALI",
            Self::Dutch => "TERUG",
            Self::Japanese => "戻る",
            Self::Malay => "KEMBALI",
            Self::Portuguese => "VOLTAR",
            Self::Thai => "กลับ",
            Self::Turkish => "GERİ",
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox back_text_matches_known_values`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/languages.rs
git commit -m "feat(refbox): add Language::back_text for previewed-language footers"
```

---

### Task 6: Wire `back_text()` into both language pickers

Pages 9–10. Each shows the picker's red button in the previewed language; swap it to `back_text()` when there are no changes. Mechanical swap — compile + walkthrough, no unit test.

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs:1978-1983` (main Language picker, in `make_language_select_page`)
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs:756-761` (BeepTest Language picker)

**Interfaces:**
- Consumes: `Language::back_text` (Task 5); existing `apply_enabled` (Language page, equals `page_has_changes`) and `has_changes` (BeepTest picker).

- [ ] **Step 1: Update the main Language picker**

In `refbox/src/app/view_builders/configuration.rs`, replace:

```rust
            let cancel_btn = button(make_label(selected.cancel_text(), selected_font))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::LanguageSelectComplete { canceled: true });
```

with:

```rust
            // `apply_enabled` here is exactly page_has_changes(ConfigPage::Language, …)
            // (the Language page has no extra Apply gate), so it doubles as the
            // has-changes signal for the Cancel/Back swap.
            let footer_label = if apply_enabled {
                selected.cancel_text()
            } else {
                selected.back_text()
            };
            let cancel_btn = button(make_label(footer_label, selected_font))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::LanguageSelectComplete { canceled: true });
```

- [ ] **Step 2: Update the BeepTest Language picker**

In `refbox/src/app/view_builders/beep_test_settings.rs`, replace:

```rust
    let cancel_btn = button(make_label(selected.cancel_text(), selected_font))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::BeepTestLanguageCancel);
```

with:

```rust
    let footer_label = if has_changes {
        selected.cancel_text()
    } else {
        selected.back_text()
    };
    let cancel_btn = button(make_label(footer_label, selected_font))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::BeepTestLanguageCancel);
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "feat(refbox): swap language-picker footers Cancel->Back when unchanged"
```

---

### Task 7: Full verification + walkthrough

**Files:** none (verification only).

- [ ] **Step 1: Run the full check suite**

Run: `just check`
Expected: fmt, lint (`-D warnings`), tests, and audit all pass.

- [ ] **Step 2: Build the real binary for the walkthrough**

Run: `cargo build -p refbox`
(Per project note: `just check` builds a *test* binary, not `target/debug/refbox`. Build explicitly before launching so the walkthrough exercises the new code, not stale code.)

- [ ] **Step 3: Launch and walk through every page**

Launch (background, sandbox disabled, X11 forced):
`WAYLAND_DISPLAY= cargo run -p refbox`

Confirm on each page that the red footer button reads **BACK** on entry (nothing changed) and flips to **CANCEL** after one edit, then back to **BACK** if the edit is reverted:

1. App Options — also confirm the blue "Check Version" button still sits between Back/Cancel and Apply.
2. Display Options
3. Sound Options — the case that surfaced this (greyed Apply after committing a buzzer should now read "Back").
4. Remotes
5. Game Options — with portal OFF: a too-short Game Block keeps the label as **CANCEL** (pending change Apply refuses). With portal ON but incomplete selection: edit something → label is **CANCEL** even though Apply is disabled.
6. Parameter editor (e.g. Half Length, Game Block): opens reading **BACK**; change the value → **CANCEL**; a too-short Game Block still reads **CANCEL**.
7. BeepTest → Sound settings
8. BeepTest → Edit Levels
9. Language picker: opens **BACK** in the current language; select a different language → its **CANCEL** word appears; reselect the current language → **BACK** again.
10. BeepTest → Language picker: same as #9.

Also confirm the genuinely-back pages are unchanged (Main menu, User-options menu, Parameter-help, BeepTest landing still say "Back"; Updates page unchanged).

- [ ] **Step 4: Watch for the known render artifact**

While clicking between pages, watch for the iced 0.13 stale-box / garbled-text artifact (the label width changes between "CANCEL" and "BACK"). It is not expected here (the footer button is fixed-size and centered), but confirm no ghost text appears after a page transition.

---

## Self-Review

**Spec coverage:** All 10 pages from the agreed scope are covered — pages 1–4 + 7–8 (Task 2), page 5 (Task 3), page 6 (Task 4), pages 9–10 (Tasks 5+6). The helper (Task 1) and `back_text` (Task 5) are the two reusable units. No new `.ftl` key (back already in 15 locales). Behaviour-only-label constraint honoured: no `on_press` message changed.

**Placeholder scan:** No TBD/TODO; every code step shows full before/after.

**Type consistency:** `cancel_or_back_label(bool) -> String` consumed identically in Tasks 2–4. `Language::back_text(self) -> &'static str` mirrors `cancel_text` exactly and is consumed in Task 6 alongside `cancel_text`/`selected_font`. Label predicate is the *has-changes* signal (`page_has_changes` / `param_edit_has_changes` / `has_changes`), never the narrower `apply_enabled`, except on the Language page where they are provably equal.

---

# Phase 2 — game-action pages (added 2026-06-25)

Scope extension approved by the user: apply the same swap to the five game-action pages that
also have an always-enabled (or edit-mode) Cancel plus a change-gated Apply. All reuse the
existing `cancel_or_back_label` helper. Out of scope: keypad add/entry sub-screens, game-number
edit, team-timeout edit, and the score-confirmation screen (see Task 12 decision).

### Task 8: Penalties / Warnings / Fouls overview footers

Each page already computes `let has_changes = any_pending_change(...)` and gates Apply on it.
Swap only the red Cancel button's label.

**Files:**
- Modify: `refbox/src/app/view_builders/penalties.rs:66` (`make_button(fl!("cancel"))` → `make_button(cancel_or_back_label(has_changes))`)
- Modify: `refbox/src/app/view_builders/warnings.rs:57` (same)
- Modify: `refbox/src/app/view_builders/fouls.rs:63` (same)

- [ ] **Step 1:** In each of the three files, change `make_button(fl!("cancel"))` (the red button whose `.on_press` is the `*OverviewComplete { canceled: true }` message) to `make_button(cancel_or_back_label(has_changes))`. Leave everything else (style, width, on_press, the blue "new" button, the Apply button) untouched.
- [ ] **Step 2:** `cargo build -p refbox` — expect clean.
- [ ] **Step 3:** Commit: `feat(refbox): swap penalty/warning/foul overview footers Cancel->Back when unchanged`

### Task 9: Time-edit footer

`build_time_edit_view` gates Apply on `time_edit_has_changes(time, timeout_time, old_time, old_timeout_time)` computed inline. Hoist it into a `has_changes` binding and reuse for both the label and Apply.

**Files:**
- Modify: `refbox/src/app/view_builders/time_edit.rs:60-72`

- [ ] **Step 1:** Just before the footer `row![`, add `let has_changes = time_edit_has_changes(time, timeout_time, old_time, old_timeout_time);`. Change the Cancel button to `make_button(cancel_or_back_label(has_changes))` and change the Apply button's gate to `has_changes.then_some(Message::TimeEditComplete { canceled: false })`.
- [ ] **Step 2:** `cargo build -p refbox` — expect clean.
- [ ] **Step 3:** Commit: `feat(refbox): swap time-edit footer Cancel->Back when unchanged`

### Task 10: Score-edit footer (edit mode only)

`build_score_edit_view` has two modes. In **confirmation** mode (`is_confirmation == true`) the red button is disabled (`cancel_btn_msg = None`) and the user pressed "Done" to commit the final score — per user decision, keep its label as `CANCEL` (unchanged). In **edit** mode, swap based on `score_edit_has_changes(scores, old_scores)`.

**Files:**
- Modify: `refbox/src/app/view_builders/score_edit.rs:156`

- [ ] **Step 1:** Just before the final `row![`, add:
```rust
    // Confirmation mode keeps the (disabled) "Cancel" label — the operator is
    // committing a final score, not navigating back. Edit mode swaps to "Back"
    // when the scores are unchanged.
    let cancel_label = if is_confirmation {
        fl!("cancel")
    } else {
        cancel_or_back_label(score_edit_has_changes(scores, old_scores))
    };
```
Then change `make_button(fl!("cancel"))` to `make_button(cancel_label)` (keeping `.on_press_maybe(cancel_btn_msg).style(red_button)` unchanged).
- [ ] **Step 2:** `cargo build -p refbox` — expect clean.
- [ ] **Step 3:** Commit: `feat(refbox): swap score-edit footer Cancel->Back when unchanged`

### Task 11: Phase 2 verification

- [ ] `just check` — expect fmt/lint/test/audit clean.
- [ ] `cargo build -p refbox`, then walkthrough the five Phase-2 pages in the running app.

## Notes / Deviations

(record any execution deviations here per the lean-process rule — fold into the relevant code commit, no standalone deviation commits)

- 2026-06-25: Phase 2 added after the Phase-1 review surfaced these five sibling footers. User
  approved including them and chose to keep the score-CONFIRMATION screen's disabled button as
  "CANCEL" (swap applies only in score-EDIT mode).
