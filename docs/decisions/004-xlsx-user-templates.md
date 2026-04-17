# ADR 004 — User-Provided XLSX Scoresheet Templates

**Status:** Accepted  
**Date:** 2026-04-12

---

## Context

The four built-in scoresheet styles (Detailed, Simple, SimpleTeamRefs, Col3x3) cover the
formats used in current tournaments. However, tournament organisers in other regions, or
referee bodies with their own official formats, need a way to create custom scoresheet
layouts without programming knowledge.

The longer-term goal is to integrate the scoresheet generator into the UWH Portal web
application. In that context, users would upload their own template file through a browser
and receive a PDF download — no software to install, no code to write. The built-in styles
developed here become the portal's default options.

The CLI schedule-processor serves as the proving ground: the template format, token contract,
and substitution logic are developed and tested here, then ported to the portal when that
integration begins.

---

## Decision

### 1. XLSX as the template format

Users design their scoresheet template in Excel or Google Sheets and save it as an XLSX file.
XLSX is chosen because:
- It is the most widely understood tool for table-based layout design among non-technical users
- It naturally supports merged cells, borders, row heights, and font styling
- Page setup (margins, orientation, paper size, print area) is configured inside the
  spreadsheet using standard tools the user already knows

### 2. Placeholder tokens

Cells in the template contain placeholder tokens of the form `{token_name}`. The tool finds
every cell containing a placeholder and replaces it with the corresponding game value.

Token names match the `GameRenderContext` field names exactly. This ensures the same template
works in both the CLI and the eventual portal implementation — the token contract is stable
across both targets. See `docs/scoresheet-styles.md` for the full token reference.

### 3. Numbered tokens for list fields

Fields that contain multiple values use numbered tokens:

- Water referees: `{water_ref_1}`, `{water_ref_2}`, `{water_ref_3}`
- Dark team players: `{dark_player_1_name}`, `{dark_player_1_cap}`, `{dark_player_2_name}`, etc.
- Light team players: `{light_player_1_name}`, `{light_player_1_cap}`, `{light_player_2_name}`, etc.

The template author decides how many rows to allocate. If the actual data has fewer entries
than the template provides, unused cells come out blank. If the data has more entries than
the template provides, the excess entries are not shown.

### 4. Optional fields become blank

Fields that have no value (e.g., referee names not yet assigned on the portal) are substituted
with an empty string. The cell is left blank on the printed form — acceptable for a paper
scoresheet where it can be filled in by hand.

### 5. Logos are static in XLSX templates

Built-in HTML styles receive logos via `GameRenderContext` and embed them as base64 data URIs.
XLSX templates do not support logo injection — the template author adds logos directly to
their Excel file as static images. The tool does not modify image content in the template.

### 6. Substitution approach

The tool treats the XLSX file as a ZIP archive of XML files. Cell value substitution is
performed directly on the XML — only the shared strings and cell value XML files are modified.
Formatting XML (styles, column widths, row heights, merge regions) is never touched, so the
visual layout is preserved exactly as the author designed it.

### 7. PDF generation

In the portal: PDF generation is handled server-side and is transparent to the user.

In the CLI: the PDF generation approach for XLSX templates is deferred — it will be decided
when implementation begins, once the portal target platform is better defined.

---

## Consequences

**Positive:**
- Non-technical users can create templates using familiar tools with no installation required
  (in the portal context)
- Merged cells, borders, and all visual formatting are naturally preserved by the substitution
  approach — formatting XML is never modified
- Template authors include only the tokens they need — unused fields are simply absent
- The token naming contract (GameRenderContext field names) is stable across CLI and portal

**Negative:**
- Numbered tokens impose an implicit maximum on displayed list entries — the template author
  must decide how many player rows to allocate
- No conditional blocks — a cell with an optional token is blank if the value is absent;
  there is no way to hide a row when a field is empty
- Logo injection is not supported in XLSX templates; logos must be embedded statically by
  the template author
- CLI PDF generation for XLSX templates is not yet decided

---

## Reference

See `docs/scoresheet-styles.md` for the full placeholder token reference.  
See ADR 003 for the `GameRenderContext` architecture this feature builds on.
