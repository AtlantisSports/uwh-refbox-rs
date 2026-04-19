# ADR 007 — Help Expand Page — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the overflow bug where long help text pushes Cancel/Done off-screen in length-parameter edit pages by removing the inline help text from the editor entirely and moving it behind a `?` button in the upper-right that opens a dedicated help page.

**Architecture:** Add a new `AppState::ParameterEditorHelp(LengthParameter, Duration)` variant that carries the same tuple as `ParameterEditor`, so round-tripping from editor → help → editor preserves the in-progress duration the operator was typing. A new message pair (`ShowParameterHelp`, `CloseParameterHelp`) toggles between the two states. The editor's help-text block is **removed** and replaced by a blue `?` button floating in the upper-right of the edit page, outside the editor panel. The full-screen help page has the same page chrome (timer bar at top, timeout ribbon at bottom) but swaps the editor panel for a container holding the parameter's title and full help text, with a single BACK button in place of Cancel/Done. Because the editor no longer contains any variable-height content, the overflow bug cannot recur regardless of translation length.

**Tech Stack:** Rust 1.85 / edition 2024, iced 0.13 GUI, fluent translation system (15 languages).

**Testing approach:** refbox has no UI-test harness. Verification is manual: launch the app, navigate to a help-bearing length editor in a long-translation language (German), confirm the bug before the fix and the fix after. `just check` is the automated gate (formatting, clippy, unit tests on non-UI code).

---

## Prerequisites

- Branch `master` is checked out, working tree clean.
- Approved to cut a feature branch: `feat/refbox/help-expand-page`.
- Approved design: remove inline help text from editor; `?` button (blue, informational) in upper-right of the edit page, outside the editor panel; help page has same chrome as edit page with editor content replaced by the help text and a single BACK button; new `ParameterEditorHelp` AppState variant.
- `just install-hooks` has been run at some point (pre-commit hook active).

---

## Task 0: Cut the feature branch

**Files:** none

- [ ] **Step 1: Confirm working tree is clean on the current branch**

Run: `git status`
Expected: either clean, or only untracked `.claude/scheduled_tasks.lock`.

- [ ] **Step 2: Switch to master and pull**

Run: `git checkout master && git pull --ff-only`
Expected: `Already up to date` or fast-forward.

- [ ] **Step 3: Cut the feature branch**

Run: `git checkout -b feat/refbox/help-expand-page`
Expected: `Switched to a new branch 'feat/refbox/help-expand-page'`.

- [ ] **Step 4: Confirm the bug is reproducible before any change**

Run: `just check` (should pass — baseline green)
Then launch the app (requires `dangerouslyDisableSandbox:true` per session memory):
Run: `cargo run -p refbox`
In the app: Settings → Game Options → Nominal Break button. Change language to German via Settings → Language if not already set. Observe whether Cancel/Done are visible below the help text.

Expected: **bug reproduces** (Cancel/Done off-screen or heavily clipped). Screenshot to `/mnt/c/Users/Eric/Desktop/Screenshot.png` for before/after comparison.

If the bug does not reproduce, stop and escalate — the fix may already be in place from another branch.

- [ ] **Step 5: No commit yet.** The baseline reproduction is noted mentally (and in the PR body later). Proceed to Task 1.

---

## Task 1: Add new AppState variant

**Files:**
- Modify: `refbox/src/app/mod.rs` — `enum AppState` at line 102-120.

- [ ] **Step 1: Add the variant to `AppState`**

Edit `refbox/src/app/mod.rs` to insert `ParameterEditorHelp` immediately after `ParameterEditor`:

```rust
    ParameterEditor(LengthParameter, Duration),
    ParameterEditorHelp(LengthParameter, Duration),
    ParameterList(ListableParameter, usize),
```

- [ ] **Step 2: Run type-check**

Run: `cargo check -p refbox`
Expected: build succeeds; possibly a non-exhaustive-match warning at the view-render match in `view()` (around line 2371).

- [ ] **Step 3: Handle the variant in the view-render match**

In `refbox/src/app/mod.rs` around line 2371 where `AppState::ParameterEditor(param, dur)` is rendered, add a sibling arm:

```rust
            AppState::ParameterEditor(param, dur) => build_game_parameter_editor(data, param, dur),
            AppState::ParameterEditorHelp(param, dur) => build_parameter_help_page(data, param, dur),
```

`build_parameter_help_page` does not yet exist — the build will fail. That is intentional; Task 2 creates it.

- [ ] **Step 4: Check the config-back-navigation match handles the new variant**

Search `refbox/src/app/mod.rs` for existing matches on `ParameterEditor` to find places that need a parallel arm for `ParameterEditorHelp`. Key location: the back-navigation block around line 1576 where `AppState::ParameterEditor(_, _) => ConfigPage::Game` appears. Add:

```rust
                    AppState::ParameterEditor(_, _) => ConfigPage::Game,
                    AppState::ParameterEditorHelp(_, _) => ConfigPage::Game,
```

Also check the mutable-borrow match around line 629 — help state is read-only, so it does not need an arm there; the `_` fallback is fine.

- [ ] **Step 5: Build (will still fail — Task 2 needed)**

Run: `cargo check -p refbox`
Expected: single error, "cannot find function `build_parameter_help_page`".

- [ ] **Step 6: No commit yet.** The state is incomplete; commit after Task 2 compiles.

---

## Task 2: Create the help page view builder

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` — add `build_parameter_help_page` alongside `build_game_parameter_editor` (around line 1007).

The help page uses the same chrome as the editor (timer bar at top). In place of the editor panel, it shows a container with the parameter title at the top and the full help text below. A single BACK button replaces the Cancel/Done row.

- [ ] **Step 1: Add the new function**

Below `build_game_parameter_editor` in `refbox/src/app/view_builders/configuration.rs`, insert:

```rust
pub(in super::super) fn build_parameter_help_page<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    _length: Duration,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let (title, body) = match param {
        LengthParameter::Half => (
            fl!("half-length"),
            fl!("length-of-half-during-regular-play"),
        ),
        LengthParameter::HalfTime => (fl!("half-time-lenght"), fl!("length-of-half-time-period")),
        LengthParameter::NominalBetweenGame => {
            (fl!("nom-break"), fl!("system-will-keep-game-times-spaced"))
        }
        LengthParameter::MinimumBetweenGame => (fl!("min-break"), fl!("min-time-btwn-games")),
        LengthParameter::PreOvertime => (fl!("pre-ot-break-abreviated"), fl!("pre-sd-brk")),
        LengthParameter::OvertimeHalf => (fl!("ot-half-len"), fl!("time-during-ot")),
        LengthParameter::OvertimeHalfTime => {
            (fl!("ot-half-tm-len"), fl!("len-of-overtime-halftime"))
        }
        LengthParameter::PreSuddenDeath => (fl!("pre-sd-break"), fl!("pre-sd-len")),
    };

    let help_panel = container(
        column![
            text(title)
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Center),
            text(body)
                .size(SMALL_TEXT)
                .align_x(Horizontal::Center),
        ]
        .spacing(SPACING)
        .align_x(Alignment::Center)
        .padding(PADDING),
    )
    .style(light_gray_container) // match the editor panel's visual container style — confirm actual style name during implementation
    .width(Length::Fill);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        vertical_space(),
        help_panel,
        vertical_space(),
        make_button(fl!("back"))
            .style(blue_button)
            .width(Length::Fill)
            .on_press(Message::CloseParameterHelp),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
```

**Notes:**
- `light_gray_container` is a placeholder for whatever container style visually matches the editor panel (`make_time_editor` returns a styled Container — find its style and reuse it here for visual continuity). If no existing name fits, use `container::transparent` and accept a slightly different look — the user has approved "basically the same page, just removing the editor content."
- `_length` is intentionally unused on the help page (no time is being edited). The parameter still carries it through so round-trip back to the editor preserves state.
- `blue_button`, `MEDIUM_TEXT`, `SMALL_TEXT`, `PADDING`, `SPACING`, `make_button`, `make_game_time_button`, `container` may or may not already be imported in this file. Check the top of `configuration.rs` and add missing imports from `shared_elements.rs` and `theme/` as needed.
- `fl!("back")` is a new FTL key — added in Task 4. Check first whether it already exists in `en-US/refbox.ftl` (likely — other sub-pages use a back button).
- `Message::CloseParameterHelp` is new — added in Task 3.

- [ ] **Step 2: Build (still fails — Task 3 needed)**

Run: `cargo check -p refbox`
Expected: errors about missing `Message::CloseParameterHelp` and possibly missing imports. Fix imports as they come up.

- [ ] **Step 3: No commit yet.** Proceed to Task 3.

---

## Task 3: Add the message pair and wire update()

**Files:**
- Modify: `refbox/src/app/message.rs` — `enum Message`, plus the three categorisation match arms (around lines 163, 207, 365).
- Modify: `refbox/src/app/mod.rs` — `update()` handlers.

- [ ] **Step 1: Add variants to the Message enum**

In `refbox/src/app/message.rs` around line 77–79 (near `ShowGameDetails` / `ShowWarnings`), insert:

```rust
    ShowGameDetails,
    RequestPortalRefresh,
    ShowWarnings,
    ShowParameterHelp,
    CloseParameterHelp,
    EditGameConfig,
```

- [ ] **Step 2: Add to the "discrete event" categorisation at ~line 163**

Add `ShowParameterHelp` and `CloseParameterHelp` to the pipe-separated list alongside `ShowGameDetails` and `ShowWarnings`.

- [ ] **Step 3: Add to the PartialEq self-match at ~line 207**

Add `(Self::ShowParameterHelp, Self::ShowParameterHelp)` and `(Self::CloseParameterHelp, Self::CloseParameterHelp)` to the same match block.

- [ ] **Step 4: Add to the sanitize/categorise match at ~line 365**

Add `(Self::ShowParameterHelp, _) | (Self::CloseParameterHelp, _)` to the same block.

- [ ] **Step 5: Wire handlers in `update()`**

In `refbox/src/app/mod.rs` around line 1246 (the `Message::ShowGameDetails` arm), add sibling arms:

```rust
            Message::ShowParameterHelp => {
                if let AppState::ParameterEditor(param, dur) = self.app_state {
                    self.app_state = AppState::ParameterEditorHelp(param, dur);
                    trace!("AppState changed to {:?}", self.app_state);
                }
                Task::none()
            }
            Message::CloseParameterHelp => {
                if let AppState::ParameterEditorHelp(param, dur) = self.app_state {
                    self.app_state = AppState::ParameterEditor(param, dur);
                    trace!("AppState changed to {:?}", self.app_state);
                }
                Task::none()
            }
```

**Note:** the `if let` guards are defensive — if the message arrives from an unexpected state, ignore rather than panic. Matches the codebase's broader pattern.

- [ ] **Step 6: Build (still will fail — FTL keys from Task 4 needed)**

Run: `cargo check -p refbox`
Expected: the build may succeed if FTL keys are looked up at runtime, or fail on fluent-macro checks. If it succeeds, run `cargo build -p refbox` to produce the binary for Task 5's manual test.

- [ ] **Step 7: No commit yet.** Proceed to Task 4.

---

## Task 4: Add the new FTL key (if needed)

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl` (add first).
- Possibly modify: all other 14 language files under `refbox/translations/*/refbox.ftl`.

Only one new string is needed: `back` (English: `"BACK"`). The `?` button label is a literal Unicode character with no FTL key. The parameter titles and help body strings already exist — they are reused verbatim from the editor.

- [ ] **Step 1: Check whether `back` already exists**

Run: `grep -rn "^back " refbox/translations/en-US/refbox.ftl`
If present, skip Steps 2–4; Task 4 is complete.
If absent, proceed.

- [ ] **Step 2: Add the new key to `en-US/refbox.ftl`**

Match the file's existing ordering (alphabetical in most sections, or by topical grouping). Insert:

```
back = BACK
```

(Confirm casing against existing button labels — `cancel = CANCEL`, `done = DONE` suggests all-caps.)

- [ ] **Step 3: Build and run the English version manually**

Run: `cargo build -p refbox`
Expected: success, no fluent runtime errors.

- [ ] **Step 4: Propagate the key to all 14 non-English languages**

For each of `de-DE`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN`:

Use the existing v0.4.0 translation convention. If each language's refbox.ftl already marks newly-added strings with an UNVERIFIED comment pending native-speaker review, follow that pattern. Otherwise add the translated value (the word "BACK" is one of the most commonly-needed UI strings and may already have reliable translations in another file in the repo).

Specific per-language values for "BACK" (common defaults — double-check against the glossary files in `docs/superpowers/specs/`):
- de-DE: `ZURÜCK`
- es: `ATRÁS`
- fr: `RETOUR`
- it-IT: `INDIETRO`
- nl-NL: `TERUG`
- pt-PT: `VOLTAR`
- id-ID: `KEMBALI`
- ms-MY: `KEMBALI`
- tl-PH: `BUMALIK`
- tr-TR: `GERİ`
- th-TH: `ย้อนกลับ`
- ko-KR: `뒤로`
- ja-JP: `戻る`
- zh-CN: `返回`

If these don't match the glossary spec for a given language, use the glossary spec's term.

- [ ] **Step 5: Rebuild**

Run: `cargo build -p refbox` — no build errors.

- [ ] **Step 6: No commit yet.** Task 5 still pending.

---

## Task 5: Remove inline help from editor; add `?` button in upper-right

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` — `build_game_parameter_editor` at lines 950-1006.

The current editor layout is a vertical column of [time button, editor panel, help text, Cancel/Done row]. After this task, the editor becomes [time button, (row containing the centered editor panel with a `?` button floating to its upper right), Cancel/Done row]. No inline help text remains.

- [ ] **Step 1: Remove help-text variables**

In `build_game_parameter_editor`, the current match on `param` produces a tuple `(title, hint)`. After this task the `hint` value is unused here (it's used only in the help page). Change the match to produce just `title`:

```rust
    let title = match param {
        LengthParameter::Half => fl!("half-length"),
        LengthParameter::HalfTime => fl!("half-time-lenght"),
        LengthParameter::NominalBetweenGame => fl!("nom-break"),
        LengthParameter::MinimumBetweenGame => fl!("min-break"),
        LengthParameter::PreOvertime => fl!("pre-ot-break-abreviated"),
        LengthParameter::OvertimeHalf => fl!("ot-half-len"),
        LengthParameter::OvertimeHalfTime => fl!("ot-half-tm-len"),
        LengthParameter::PreSuddenDeath => fl!("pre-sd-break"),
    };
```

- [ ] **Step 2: Replace the column body with the new layout**

Replace the entire `column![...]` block (lines 980-1006 of the current file) with:

```rust
    let help_button = make_button("?")
        .style(blue_button)
        .width(Length::Fixed(60.0))
        .on_press(Message::ShowParameterHelp);

    let editor_row = row![
        horizontal_space(),
        make_time_editor(title, length, false),
        column![
            help_button,
            vertical_space(),
        ]
        .width(Length::Fixed(60.0)),
    ]
    .spacing(SPACING)
    .align_y(Vertical::Top)
    .height(Length::Fill);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        editor_row,
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
```

**Notes on the layout:**
- The `editor_row` places the editor panel in the middle with a `horizontal_space()` on the left and a fixed-width column (60 px wide) on the right holding the `?` button at the top. This puts the `?` at the upper-right of the edit area, outside the editor panel.
- The left `horizontal_space()` is a flex spacer; combined with the right fixed-width column, the editor is not perfectly centered but sits slightly left of center — matches the mock-up in `/mnt/c/Users/Eric/Desktop/Screenshot 1.png`.
- If precise symmetry is desired (editor perfectly centered despite the right column), add a matching `horizontal_space()` after `make_time_editor(...)` or widen the left one with `.width(Length::FillPortion(1))`. Visual tuning during manual verification.
- Button size (60.0 px) matches the mock-up's button dimensions approximately. Adjust during visual QA if needed.

- [ ] **Step 3: Build**

Run: `cargo check -p refbox`
Expected: success.

- [ ] **Step 4: Run the app manually and verify**

Run: `cargo run -p refbox`

Verify in **English** first:
- [ ] Settings → Game Options → Nominal Break opens the editor.
- [ ] Editor panel is centered (or slightly left-of-center to match the mock-up).
- [ ] A blue `?` button sits in the upper-right, outside the editor panel.
- [ ] Cancel and Done buttons are visible below.
- [ ] No help text is shown inline in the editor.

Verify navigation:
- [ ] Tap the `?` button. The help page opens.
- [ ] Help page shows parameter title at top, full help body below, BACK button at bottom.
- [ ] Timer bar and timeout ribbon present on the help page (same chrome as editor).
- [ ] Tap BACK. Returns to the editor.
- [ ] Before tapping `?`, type a partial duration (e.g. `+` a few times). After returning from help, the typed value is still shown.

Verify in **German** (the original bug case):
- [ ] Settings → Language → Deutsch.
- [ ] Repeat the Nominal Break check. Cancel/Done must be visible. Layout must not overflow. `?` button still in the upper-right.
- [ ] Tap `?`. The German help text (which is very long) shows in full on the help page. If the German help body itself exceeds the help-page's vertical budget, make a note for a follow-up; the primary bug (Cancel/Done hidden on the editor) is gone.

Verify at least one other long-translation language (Italian or Indonesian):
- [ ] Same expectations as German.

Take before/after screenshots (`Screenshot.png` or `Screenshot 1.png` etc.) for the PR body.

- [ ] **Step 5: No commit yet — run the quality gate.**

---

## Task 6: Run `just check` and resolve any failures

**Files:** whatever `just check` flags.

- [ ] **Step 1: Run the full quality gate**

Run: `just check`
Expected: green. If red, read each error's plain-English cause; fix; re-run.

Common failures to expect on a new refbox UI change:
- Clippy wants `.as_str()` or clone adjustments on `fl!()` concatenations — apply.
- `cargo fmt` alignment in the new blocks — run `just fmt` to auto-fix.

- [ ] **Step 2: Confirm green `just check` output**

Expected: the `just check` exits 0.

---

## Task 7: Commit

**Files:** all changes staged.

- [ ] **Step 1: Review the diff**

Run: `git diff`
Confirm the changes match the plan: one new AppState variant, one new view builder, one new Message pair, modified `build_game_parameter_editor` block, new FTL keys across 15 languages.

Nothing else should be modified. If anything else is touched, investigate (may be an opportunistic formatting change that should be reverted per the scope rule).

- [ ] **Step 2: Stage files**

Run: `git add refbox/src/app/mod.rs refbox/src/app/message.rs refbox/src/app/view_builders/configuration.rs refbox/translations/`
(Do not use `git add -A`.)

- [ ] **Step 3: Commit with the standard format**

Run:

```bash
git commit -m "$(cat <<'EOF'
fix(refbox): move length-parameter help text off the editor page

The inline help text has been removed from length-parameter edit pages
and moved to a dedicated help page reached via a blue ? button in the
upper-right of each editor. Before this fix, long help text (notably
the German and Italian nominal-break strings) pushed the Cancel and
Done buttons off the bottom of the screen, making it impossible to
commit or cancel an edit. The new layout keeps the editor free of any
variable-height content, so the bug cannot recur for any translation.
Tapping the ? button opens the help page; tapping BACK returns to the
editor with the in-progress duration value preserved.
EOF
)"
```

Expected: pre-commit hook runs formatting check, clippy, and branch-name check; commits on success.

---

## Task 8: Open the PR

**Files:** none (GitHub only).

- [ ] **Step 1: Push the branch**

**Approval gate:** confirm with the human before pushing. Push is visible to the remote.

Run: `git push -u origin feat/refbox/help-expand-page`

- [ ] **Step 2: Create the PR**

Run:

```bash
gh pr create --title "fix(refbox): move length-parameter help text off the editor page" --body "$(cat <<'EOF'
## What changed

The inline help text has been removed from length-parameter edit pages.
A blue \`?\` button now sits in the upper-right of each editor; tapping
it opens a dedicated help page that shows the parameter title, the full
help text, and a BACK button. BACK returns to the editor and the
duration the operator was typing is preserved.

## Why

Before this change, long help text — notably the German and Italian
translations of the Nominal Break help string — pushed the Cancel and
Done buttons off the bottom of the screen. The operator could not
commit or cancel an edit without restarting the app. The issue was
reported 2026-04-18 and is described in ADR 007 (revised 2026-04-19).

Moving help text off the editor page removes the root cause: the
editor no longer contains any variable-height content, so the overflow
bug cannot recur for any translation, now or in future.

## Scope

Changes are limited to:
- \`refbox/src/app/mod.rs\`, \`refbox/src/app/message.rs\`,
  \`refbox/src/app/view_builders/configuration.rs\` — UI state, message
  routing, view builders.
- \`refbox/translations/*/refbox.ftl\` — at most one new FTL key
  (\`back\`) across 15 languages, if not already present.

No changes to \`uwh-common\`, \`schedule-processor\`, \`overlay\`, or any
other crate.

## How to verify

1. Pull this branch and run \`just check\` — expected: green.
2. Run the refbox app: \`cargo run -p refbox\`.
3. Change the language to German (Settings → Language → Deutsch).
4. Settings → Game Options → Nominal Break.
5. Before this PR, the Cancel and Done buttons were off-screen.
   With this PR: Cancel and Done are visible. The editor shows only
   the time controls (no help text). A blue \`?\` button is in the
   upper-right.
6. Tap the \`?\` button — the full help page opens, showing the
   parameter title and the complete help text with a BACK button.
7. Tap BACK — you return to the editor.
8. Before tapping \`?\`, type a partial time (e.g. change minutes by
   a few). After returning from help, confirm the typed value is
   preserved.
9. Repeat in English and Italian to confirm no regression on other
   languages.

See before/after screenshots attached.
EOF
)"
```

- [ ] **Step 3: Wait for CI**

Watch the PR's checks. If red, read the failure in plain English and fix on a new commit to the branch.

- [ ] **Step 4: Human reviews via `docs/review-checklist.md`**

Hand off. No further action until review feedback arrives.

---

## After-merge housekeeping

Not part of this plan's execution, but recorded for the session that merges:

- Update `docs/decisions/007-help-text-layout.md` on the local `docs/workspace/backlog-adrs` branch: status `proposed` → `accepted`, add the merged PR number to the References section.
- Update `docs/superpowers/plans/2026-04-19-backlog-adrs-roadmap.md`: mark Phase 1 complete; Phase 2 (ADR 006) becomes next.
- Update memory: the post-v0.4.0 handover gets a line about ADR 007 shipped.
