# Shared Button Helpers Paragraph-Cache Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Chunk 11 — remove the redundant `text(...).align_x(Horizontal::Left).align_y(Vertical::Center)` alignment specifications inside `make_button` and `make_smaller_button` in `refbox/src/app/view_builders/shared_elements.rs`, eliminating the iced 0.13 paragraph-cache stale-anchor source that produced the EDIT LEVELS letter-overlap artifact.

**Architecture:** Two parallel surgical edits in one file. The wrapping `container(t).center(Length::Fill)` already provides structural centering — the alignment-on-text specs are visually no-op (text is `Length::Shrink`, so there's no extra space to align within) but iced still caches a per-widget paragraph anchor against them. Removing them invalidates the stale-anchor source. If the removals leave `iced::alignment::{Horizontal, Vertical}` unused in this file, clippy (`-D warnings`) will surface that and they must also be pruned.

**Tech Stack:** Rust 2024, MSRV 1.85; iced 0.13.

**Spec:** `docs/superpowers/specs/2026-05-21-shared-button-helpers-paragraph-cache-fix-design.md` (committed at `4ff8122c`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work). With extra walkthrough care: the helpers are widely shared, so the walkthrough must spot-check buttons across multiple pages (not just BeepTest Settings). One final code review at PR time; no per-task review. No new unit test.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/view_builders/shared_elements.rs` | `make_button` and `make_smaller_button` helpers — remove redundant text alignment specs; clean up resulting unused imports if any | Task 1 |

One coding task plus a walkthrough.

---

### Task 1: Remove redundant text alignment in both helpers

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`make_button`, around lines 952-963, and `make_smaller_button`, around lines 965-976; possibly the top-of-file iced imports if alignment enums become unused)

- [ ] **Step 1: Apply the edit to `make_button`**

In `refbox/src/app/view_builders/shared_elements.rs`, find `make_button`. Current code:

```rust
pub(super) fn make_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    let t = text(label)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Center)
        .width(Length::Shrink);
    button(container(t).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}
```

Change to:

```rust
pub(super) fn make_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    let t = text(label).width(Length::Shrink);
    button(container(t).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}
```

The `.align_x(Horizontal::Left)` and `.align_y(Vertical::Center)` calls are removed; the rest of the function body is identical. The wrapping `container(t).center(Length::Fill)` keeps the text visually centered.

- [ ] **Step 2: Apply the same edit to `make_smaller_button`**

Immediately below `make_button` in the same file is `make_smaller_button` with the identical antipattern shape (only difference: `XS_BUTTON_SIZE` instead of `MIN_BUTTON_SIZE` for the height). Current code:

```rust
pub(super) fn make_smaller_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    let t = text(label)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Center)
        .width(Length::Shrink);
    button(container(t).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(XS_BUTTON_SIZE))
        .width(Length::Fill)
}
```

Change to:

```rust
pub(super) fn make_smaller_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    let t = text(label).width(Length::Shrink);
    button(container(t).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(XS_BUTTON_SIZE))
        .width(Length::Fill)
}
```

- [ ] **Step 3: Verify the workspace compiles**

```
cargo build -p refbox 2>&1 | tail -10
```

Expected: clean compile. The build may surface **unused import** warnings for `Horizontal` and/or `Vertical` at the top of `shared_elements.rs`. That's expected if Steps 1+2 were the only sites using those enums. Note the exact warnings for Step 4.

- [ ] **Step 4: Resolve any unused-import warnings**

If `cargo build` surfaced warnings of the form `unused import: ...Horizontal` or `unused import: ...Vertical`, scan the file for other uses (`grep -n "Horizontal::\|Vertical::" refbox/src/app/view_builders/shared_elements.rs`) and remove the enum names from the iced import block at the top of the file.

Examples of how the existing import block might look (the exact form depends on what's already there; do not assume):

```rust
use iced::alignment::{Horizontal, Vertical};
```

If only `Horizontal` is unused but `Vertical` is still used elsewhere in the file, leave `Vertical`:

```rust
use iced::alignment::Vertical;
```

If both are unused, drop the whole `use iced::alignment::{...}` line. If they're part of a wildcard or nested import, prune only the unused parts.

If `cargo build` is clean (no unused-import warnings), this step is a no-op — both enum names are still used by other helpers in the file. Skip to Step 5.

- [ ] **Step 5: Run `just check`**

```
just check
```

Expected: PASS — fmt, clippy, full test suite, audit. Because the workspace builds clippy with `-D warnings`, any leftover unused-import would fail here; Step 4 must be complete before Step 5 passes. Pre-existing "Missing keys in translations/* portal-row-attempt-suffix" warnings are unrelated and stay.

- [ ] **Step 6: Commit**

```
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "fix(refbox): drop redundant text alignment in shared button helpers"
```

With Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

Per `.claude/rules/pr-review.md`: "Smoke-tested locally — refbox (or the affected artifact) was launched and the change exercised in a real session before any push/PR/merge/tag-push." The helper is widely shared; spot-check across multiple pages, not just the originally-reported screen.

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(Operator config has `mode = "BeepTest"`, so refbox comes up in BeepTest mode directly.)

- [ ] **Step 2: Scenario 1 — reproduce the original EDIT LEVELS report**

Navigate **Main → Settings**. Note the rendering of the four buttons on the BeepTest Settings landing (SOUND SETTINGS, EDIT LEVELS, APP MODE, LANGUAGE) and the BACK button at the bottom. Tap **EDIT LEVELS**. On the Edit Levels page, tap ADD LEVEL once. Tap the per-page **CANCEL** button (red, bottom of Edit Levels page) to discard the change and return to the BeepTest Settings landing.

Confirm at the BeepTest Settings landing:

- The **EDIT LEVELS** button text renders cleanly — no letter overlap, no partial obscuring, no doubled glyphs.
- The other three buttons on the landing (SOUND SETTINGS, APP MODE BEEP TEST, LANGUAGE) also render cleanly.
- The BACK button at the bottom renders cleanly.

If any artifact appears, screenshot and stop — Approach A's diagnosis may have been wrong and we need to iterate.

- [ ] **Step 3: Scenario 2 — spot-check across Hockey6V6 mode flows**

Switch to Hockey6V6: from the BeepTest Settings landing, tap APP MODE BEEP TEST to cycle to **Hockey6V6**. A blue **RESTART TO APPLY** button appears at the bottom-right. Tap it. Refbox restarts in Hockey6V6.

Once in Hockey6V6, walk through these flows, observing button text at every transition:

1. **Settings → Game Options.** Modify the half length field (or any other field). Tap **CANCEL**. Returns to Settings landing.
2. **Settings → App Options.** Modify any field. Tap **CANCEL**. Returns to Settings landing.
3. **Settings → User Options → Display Options.** Modify any field. Tap **CANCEL**. Returns to User Options.
4. **Settings → User Options → Sound Options → any Remotes sub-page** (if available). Navigate in and back out.

At each return-from-sub-page transition, confirm no text-overlap artifacts on any button (the Settings landing's buttons, the parent page's buttons, etc.). Pre-existing artifacts elsewhere are out of scope, but no NEW artifacts should appear that weren't there before.

- [ ] **Step 4: Scenario 3 — reset to operator's preferred state**

From Hockey6V6, tap APP MODE → cycle to **BeepTest** → RESTART AND APPLY. Refbox restarts in BeepTest mode (operator's working environment).

Verify Scenario 1's clean-rendering result still holds after the second restart.

- [ ] **Step 5: Hand back to operator**

Report walkthrough results for Scenarios 1–3. If Scenario 1 still shows the EDIT LEVELS artifact, stop and report — the diagnosis was wrong, do not push/PR.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:**
  - Spec §Design `make_button` change → Task 1 Step 1 (full before/after).
  - Spec §Design `make_smaller_button` change → Task 1 Step 2 (full before/after).
  - Spec §Imports cleanup → Task 1 Steps 3 + 4 (driven by the build output).
  - Spec §Testing scenarios 1–3 → Task 2 Steps 2–4.
  - Spec §Acceptance criteria (`just check` green + artifact gone + no new artifacts) → Task 1 Step 5 + Task 2.
- **No placeholders:** all steps show concrete code/commands. Step 4's import handling is conditional on the build output, but the conditional is explicit ("if warnings, do this; if clean, skip") with concrete grep + edit instructions for each branch.
- **Type consistency:** No new types or identifiers. The change is purely deletion of `.align_x(Horizontal::Left)` and `.align_y(Vertical::Center)` from existing chains.
- **Lean process:** one `just check` gate; no per-task code review; final review at PR time. No new unit test.
- **Walkthrough rigor:** scenarios 2 and 3 explicitly cover Hockey6V6 (not just BeepTest) so the spot-check actually exercises buttons in flows the operator regularly uses outside BeepTest. The helpers are shared across the whole app — visiting only BeepTest Settings would under-test.
- **Diagnosis caveat:** if Scenario 1 still shows the artifact, the operator's "edit out the redundant alignments" diagnosis was wrong and a different fix is needed. Step 5 explicitly handles this by halting before push/PR.
