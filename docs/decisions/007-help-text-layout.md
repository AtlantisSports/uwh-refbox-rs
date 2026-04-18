# 007 — Help text layout and overflow

**Date:** 2026-04-18
**Status:** proposed

## Context

Several time-parameter edit screens in the refbox show a block of
explanatory "HELP:" text below the time editor and above the
Cancel/Done buttons. When the help text is long enough to wrap onto
many lines, it pushes the Cancel/Done buttons off the bottom of the
screen — making it impossible to cancel or commit the edit without
restarting.

Observed on the "nominal break" screen in English. Because German,
Italian, and several other recently-added v0.4.0 languages are
typically 20–40% longer than English, this bug is likely to surface on
additional screens as soon as those translations are used.

The root cause is that the relevant page is built as a
`column![..., text, vertical_space(), buttons]` with iced's default
column height of `Length::Shrink`. Because the column grows to fit its
content rather than filling the window, the `vertical_space()` fillers
have no slack to collapse; long text simply pushes the buttons past
the bottom edge.

The immediate bug lives in a single function,
`make_length_config_page` (around `configuration.rs:960`), which
handles all 8 length-parameter screens. The same idiom — `column!`
without `.height(Length::Fill)` combined with `vertical_space()`
fillers — appears in ~10 other view_builders and is latent
elsewhere.

## Decision

Two coupled changes, in sequence:

### Part A — immediate fix (bug)

Make the length-config page's root column `.height(Length::Fill)` so
that `vertical_space()` becomes a real flex spacer. Long help text
then clips against its allotted region rather than pushing buttons
out of view.

Pair with a defensive sweep of the other view_builders that use the
same idiom (`time_edit.rs`, `score_add.rs`, `confirmation.rs`, and
several files under `keypad_pages/`) to apply the same
`.height(Length::Fill)` where it is missing.

### Part B — layout polish

Replace in-place help text with a collapsed preview + expand-page
pattern:

1. **In-place preview:** show approximately 2 lines of the help text
   in its current position, truncated with an ellipsis if longer.
2. **Expand affordance:** append a small "More..." button (or an
   expand icon) at the end of the preview.
3. **Full-screen help page:** tapping the affordance navigates to a
   dedicated page with the full help text, a title (e.g.
   "Help — Nominal Break"), and a single Back button that returns to
   the editor with state preserved.

This mirrors the existing pattern used for game details
(`ShowGameDetails`) and warnings (`ShowWarnings`) — full-screen
sub-pages reached from a tile — so it is consistent with the rest of
the app's navigation.

## Open design questions (to resolve during implementation)

- **Preview line count.** 2 lines feels right at current font sizes
  but should be verified across the widest-aspect window (laptop) and
  narrowest (small touchscreen). 3 lines may be better when space
  allows.
- **Expand affordance form.** A text "More..." link is
  self-documenting; a pictographic expand icon scales better across
  locales. Probably the icon, with an accessible label.
- **Back target.** Returning to the editor must preserve the
  in-progress value the operator had been entering. Implementation
  needs to confirm the state-machine support.

## Consequences

**Becomes easier:**

- Operators can always reach Cancel/Done regardless of help-text
  length or UI language.
- Help text can grow (or be translated freely) without triggering
  layout regressions.
- A dedicated help page leaves room for richer content (bullets,
  examples) in future.

**Becomes harder / constrained:**

- Each help entry needs a matching page title — one extra FTL key per
  parameter. Small one-off translation cost.
- A second navigation level for help slightly increases the click
  budget to read a full explanation.

**Scope:**

- Refbox-only; no changes to `uwh-common` or any other crate.
- Part A is a narrow bug fix (one function plus an optional
  defensive sweep). A few hours of work.
- Part B is a feature addition touching multiple screens and adding
  FTL keys across all 15 languages. Roughly 1–2 days.

Parts A and B should ship as separate branches/PRs to keep scope
clean and to give Part A the fast path.

## References

- Screenshot showing the overflow on the "nominal break" screen
  (English, 2026-04-18).
- `refbox/src/app/view_builders/configuration.rs::make_length_config_page`
  — location of the immediate bug.
- `vertical_space()` usages across `refbox/src/app/view_builders/` —
  candidates for the defensive sweep.
- `Message::ShowGameDetails`, `Message::ShowWarnings` — existing
  precedent for the "tile opens a sub-page" navigation pattern that
  Part B would mirror.
