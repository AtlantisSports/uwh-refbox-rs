# ADR 003 — Scoresheet Style Architecture

**Status:** Accepted  
**Date:** 2026-04-12  
**Branch:** `feat/refbox/referee-display-names` (authored in this session)

---

## Context

The schedule-processor generates PDF scoresheets for UWH tournaments. As of early 2026,
there are four styles (Detailed, Simple, SimpleTeamRefs, Col3x3), each implemented as a
large self-contained function in `scoresheets.rs`. The file is over 2,500 lines.

Problems with the current state:
- Styles share no common foundation — page size, logo embedding, and referee fields are
  duplicated across each style's HTML template
- Adding a new style means copying a large block of code and modifying it
- There is no documented process for "how to add a style"
- Page sizes are inconsistent (US Letter vs A4 mixed across styles)
- Some styles have access to roster data and logos; others do not — there is no single
  "everything available" contract

The tournament also runs in multiple regions and languages. Referee role labels like
"Chief Ref" need to be customisable per style without changing the data layer.

---

## Decision

### 1. Introduce `GameRenderContext`

All data available for rendering a single game scoresheet is collected into one struct
before any style function is called. This struct includes:

- Game metadata (number, pool, time)
- Team names
- Player rosters (both teams)
- Referee assignments (chief, water refs, timer) from the portal
- Tournament logos (left and right)
- Event name and dates

This is the single source of truth passed to every render function.

### 2. Standardise on A4

All styles use A4 as the page size. Orientation (portrait or landscape) is declared in
the style's own CSS — it is not a code-level setting. This replaces the current mix of
US Letter sizes across styles.

A4 rationale: most tournaments outside North America use A4. It is the international
standard and produces better results on non-US hardware.

### 3. Pure render functions

Each style is a function `fn render_style_name(ctx: &GameRenderContext) -> String`.
It receives context, returns an HTML string. No side effects, no I/O, no network.

### 4. Style owns all display text

The `GameRenderContext` provides normalised English field names (e.g., `chief_ref`).
The style is responsible for all label text. A Spanish-language style writes "Juez
Central". A French style writes "Arbitre Principal". There is no automatic translation.
This is by design: translation is a layout concern, not a data concern.

### 5. Logos and rosters available to all styles

Every render function receives logos and rosters via `GameRenderContext`. Styles that
do not need them simply ignore the fields. This ensures any new style can use these
resources without requiring a data pipeline change.

---

## Consequences

**Positive:**
- Adding a new style is straightforward: add an enum variant, write a render function,
  add a match arm. See `docs/scoresheet-styles.md` for the full process.
- Styles are independently testable
- Consistent page size across all PDF output
- Clear separation between data (what the portal provides) and display (what the style shows)

**Negative:**
- Existing styles need to be migrated to the new `GameRenderContext` struct — this is
  refactoring work, not a quick change
- The migration will be done incrementally (one style at a time) rather than all at once,
  so there will be a transitional period where some styles use the new context and some
  do not

---

## Reference

See `docs/scoresheet-styles.md` for the full style authoring guide.
