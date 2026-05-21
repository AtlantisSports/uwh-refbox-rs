# Shared Button Helpers Paragraph-Cache Fix — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 10 at `4d50e71d`)
**Chunk:** 11 of the beep-test redesign follow-on series (though the fix lives in a shared helper used across the whole app, not in BeepTest-specific code).
**Process gate:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change). But the change lives in a widely-shared helper, so walkthrough verification should spot-check multiple pages, not just the originally-reported screen.

---

## Goal

Remove the redundant text-widget alignment specifications (`text(...).align_x(Horizontal::Left).align_y(Vertical::Center)`) from the `make_button` and `make_smaller_button` helpers in `refbox/src/app/view_builders/shared_elements.rs`. The structural `container(t).center(Length::Fill)` wrapper already centers the text — the alignment-on-text adds nothing visually but triggers iced 0.13's paragraph-cache stale-anchor bug, producing the EDIT LEVELS letter-overlap artifact the operator observed after a page transition.

---

## Motivation

Operator surfaced 2026-05-21 during the Chunk 9 walkthrough and re-confirmed during Chunk 10 walkthrough: the EDIT LEVELS button on the BeepTest Settings landing page renders with letter overlap after navigating back from the Edit Levels sub-page. Screenshot shows the "V" in "LEVELS" with apparent overlap. Operator described the prior similar fix as "the iced feature [where] images/text from the new page is not being cleared" — pointing to a paragraph-cache stale-anchor symptom previously addressed in:

- `8a8d0186` — `fix(refbox): fix multi-label button text clipping on state transitions` (the original diagnosis: "iced 0.13 caches paragraph positions per text widget, so when game state changes the stale anchor caused text to render outside its clip bounds").
- `c972233c` — `fix(refbox): apply paragraph-cache pattern to game info button and portal rows` (extended the same fix to two more sites).

Both prior fixes removed `text(...).align_y(Vertical::Center)` and relied on a wrapping container's structural centering instead.

This chunk applies the same pattern to the shared `make_button` and `make_smaller_button` helpers. Those helpers are used by buttons throughout the app, so fixing the antipattern at the source benefits every button — including the EDIT LEVELS button that surfaced the report.

The redundant alignment specs are visually no-op in `make_button`'s current shape:

- `text(label).width(Length::Shrink)` — the text widget sizes itself to its content.
- `.align_x(Horizontal::Left)` — left-aligns text within its own bounds. With Shrink width, there is no extra space to align within; no-op.
- `.align_y(Vertical::Center)` — vertically centers text within its own bounds. With default Shrink height, no-op.
- `container(t).center(Length::Fill)` — the wrapping container centers the text widget within the button's Fixed-height bounds (both horizontally and vertically).

So removing the two alignment specs from the text widget produces the same visual layout while eliminating the paragraph-cache key that iced 0.13 caches against.

---

## Scope

### Files touched

- `refbox/src/app/view_builders/shared_elements.rs`:
  - `make_button` (lines 952-963): remove `.align_x(Horizontal::Left)` and `.align_y(Vertical::Center)` from the `text(label)` chain.
  - `make_smaller_button` (lines 965-976): same removal.
  - Top-of-file imports: if `Horizontal` and `Vertical` enums become unused after the removals, also remove them from the iced imports. (Check across the whole file — they may be used by other helpers.)

### Not touched

- Every call site of `make_button` and `make_smaller_button` — the fix is in the shared helper, no caller-side change needed.
- The container `.center(Length::Fill)` and the rest of each helper's structure (button padding, height, width).
- Other button helpers in the same file (e.g. `make_value_button`, `make_game_time_button`, `make_multi_label_button`) — only the two with the identified antipattern are touched. If those other helpers have the same antipattern, that's a future cleanup not covered here.
- Iced upgrade, theme files, or any non-text widget construction.
- Any BeepTest-specific code — Chunks 7-10 work is unaffected.
- `uwh-common`, `wireless-remote`, `overlay`, the LED panel, Chunk 6 work.

### Out of scope (intentionally deferred)

- **Per-call-site fix instead of helper-level fix** (Approach B/C from brainstorming). Rejected: the antipattern is in the helper; fixing once benefits every call site uniformly.
- **Audit of `make_value_button`, `make_game_time_button`, `make_multi_label_button` for the same antipattern.** If the operator observes the same artifact on buttons constructed via those other helpers in the future, a separate chunk can apply the same pattern.
- **iced framework-level fix.** Out of reach; the bug is in iced 0.13's paragraph cache.

---

## Design

### Approach A — remove both alignments from text in both helpers (approved through brainstorming)

**`make_button` (current, lines 952-963):**

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

**`make_button` (after):**

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

**`make_smaller_button` (current, lines 965-976):**

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

**`make_smaller_button` (after):**

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

### Imports cleanup

The two helpers were the file's primary users of `Horizontal::Left` and `Vertical::Center`. After the removal, those import items may be unused. Check the top-of-file imports for `iced::alignment::{Horizontal, Vertical}` (or a wildcard) and either:

- Remove the import if no other helper in the file uses them, or
- Leave the import in place if other helpers do still use them.

`cargo build` will warn on any unused import; clippy will fail on it under the `-D warnings` workspace setting. The implementation step runs `just check` so the right answer surfaces automatically.

### Pattern reference (per `.claude/rules/patterns.md`)

This is a direct application of the established pattern from `8a8d018` and `c972233c`: remove `text(...).align_y(Vertical::Center)` and rely on the surrounding container's structural centering. The case shape differs slightly — the prior fixes targeted `text.height(Fill) + text.align_y`, while these helpers use `text.height(Shrink) + text.align_y` — but the remedy is identical because both shapes register the same paragraph-cache anchor key in iced 0.13.

---

## Testing

No new unit test. Lean process. Verification via walkthrough.

**Walkthrough scenarios:**

1. **Reproduce the original report.** Launch refbox in BeepTest mode. Navigate **Main → Settings**. Note the EDIT LEVELS button text rendering. Tap EDIT LEVELS, make a change (e.g. ADD LEVEL), tap Cancel, return to the BeepTest Settings landing. Confirm:
   - The EDIT LEVELS button text renders cleanly with no letter overlap or partial-obscuring artifact.
   - The other three buttons on the landing (SOUND SETTINGS, APP MODE, LANGUAGE) also render cleanly.

2. **Spot-check across the app.** Open Hockey6V6 mode (Settings → Mode → Hockey6V6 → RESTART AND APPLY). Navigate:
   - Settings → Game Options → modify a field → Cancel back to Main.
   - Settings → App Options → modify a field → Cancel back to Main.
   - Settings → User Options → Display Options → modify a field → Cancel back to User Options.
   - Any confirmation prompts triggered along the way.

   At each transition, confirm no text-overlap artifacts on any button. (Pre-existing artifacts in other places, if any, are out of scope — the bar is "no new artifacts and the EDIT LEVELS one is fixed".)

3. **Reset to operator's preferred state.** Restart refbox into BeepTest mode for the operator's working environment.

`just check` passes (fmt, clippy, full test suite, audit). Any unused-import warning surfaces here and is fixed inline as part of Task 1.

---

## Acceptance criteria

- The EDIT LEVELS letter-overlap artifact no longer reproduces (Scenario 1).
- No new text-rendering artifacts appear elsewhere (Scenario 2).
- `just check` is green.
