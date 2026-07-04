# Backlog: "Cancel" → "Back" on config pages when nothing has changed

**Status:** Idea. Not started. Not on any branch.
**Surfaced:** 2026-06-25, during the buzzer-sounds-picker walkthrough (feat/refbox/buzzer-sounds-picker).
**Raised by:** the user (operator), as an explicitly separate task — NOT part of the buzzer work.

## The idea

On configuration pages that have an Apply button, the footer's red **"CANCEL"** button should
instead read **"BACK"** whenever the page has no pending changes (i.e. Apply is disabled). As
soon as a property on the page changes (Apply becomes enabled), the button reverts to **"CANCEL"**.

Rationale: "Cancel" implies discarding changes. When there is nothing to discard, the operator is
really just navigating back, so "Back" is the more honest, less alarming label. This removes the
"why does it say Cancel when I didn't change anything?" friction.

## Why it came up

In the buzzer picker, picking a sound + Apply commits immediately (mirrors the Language picker),
so on returning to the Sound page its Apply is correctly greyed ("nothing left to save"). That
greyed-Apply state felt confusing. The deeper observation: a "Cancel" button on a page with no
pending changes reads oddly. The fix the user prefers is the label swap above — applied
**uniformly across all config pages**, not just the buzzer/sound pages.

## Scope when picked up

- This is a **cross-cutting** change to the shared config-page footer pattern, not a one-page tweak.
  The enabled/disabled signal already exists (`page_has_changes(...)` in
  `refbox/src/app/view_builders/configuration.rs`), so the same predicate that gates Apply can
  drive the Cancel/Back label.
- Decide the propagation surface: all `ConfigPage::*` footers, and likely the BeepTest config
  pages too (`beep_test_settings.rs`) for consistency.
- New translation key needed (`back`) in all 15 locales, alongside the existing `cancel`.
- Behaviour to confirm: when the button reads "Back" (no changes), pressing it should be a plain
  navigate-to-parent — semantically identical to today's Cancel-with-no-changes (which reverts an
  unchanged snapshot = a no-op revert), so likely no logic change beyond the label.

## Explicitly NOT decided here

Whether the buzzer should instead defer its save to the Sound page's Apply (the "make buzzer match
the page" option) — the user chose to leave the buzzer Apply model as-is. This backlog item is only
about the Cancel/Back label.
