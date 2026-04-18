# 007 — Help text layout and overflow

**Date:** 2026-04-18
**Revised:** 2026-04-19 (see Context)
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

The page is built in `build_game_parameter_editor`
(`refbox/src/app/view_builders/configuration.rs:950`). Its root column
holds, top-to-bottom: a time-display button, a time editor (the
increment/decrement keypad returned by `make_time_editor`), the help
text, and a Cancel/Done row — separated by `vertical_space()` flex
spacers. The root column already carries `.height(Length::Fill)`
(line 1005), so those spacers do absorb whatever vertical slack the
viewport offers.

When the help text stays short, the spacers carry the surplus and
everything fits. When the help text wraps onto many lines, the rigid
content (time button + keypad + help text) grows until it exceeds the
viewport height; the spacers have already shrunk to zero and can
shrink no further, so the Cancel/Done row is pushed past the bottom
edge. Adding `.height(Length::Fill)` cannot fix this — the attribute
is already present. Only **reducing the space the help text is
allowed to occupy** can keep the buttons on-screen.

**Revision note (2026-04-19).** The original draft blamed a missing
`.height(Length::Fill)` on the root column and proposed a two-part
fix (Part A: add the attribute; Part B: move help to its own page).
A code read on 2026-04-19 showed the attribute is already in place
and the bug still reproduces — so Part A as described would have
been a no-op. This revision drops Part A entirely and promotes the
Part B design to the sole fix.

## Decision

Replace the in-place help text with a collapsed preview + expand-page
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

Because the preview is bounded to ~2 lines, its contribution to the
editor's rigid content stays small and the Cancel/Done row remains
on-screen regardless of the help string's translated length.

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
- One PR touching multiple screens and adding FTL keys across all
  15 languages. Roughly 1–2 days.

## References

- Screenshot showing the overflow on the "nominal break" screen
  (English, 2026-04-18).
- `refbox/src/app/view_builders/configuration.rs::build_game_parameter_editor`
  (line 950) — the function that builds every length-parameter edit
  screen and exhibits the overflow.
- `Message::ShowGameDetails`, `Message::ShowWarnings` — existing
  precedent for the "tile opens a sub-page" navigation pattern that
  this decision mirrors.
