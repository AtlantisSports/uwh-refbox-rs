# Backlog: audit all 15 locales for text that does not fit its button or row

> **LARGELY SUPERSEDED for buttons** by `../auto-fit-button-text/NOTE.md`, approved 2026-08-10.
> Auto-fitting button text fixes this class of bug at the helper level (31 call sites) rather than
> auditing instances, and needs no new dependency. What remains for THIS audit afterwards is the
> non-button slots — chiefly the keypad panel's fixed 283px title row. Do this one second, and only
> for whatever auto-fit does not already cover.
>
> The German case below has since been **reproduced and characterised**: `MANNSCHAFT` is too wide,
> so iced word-wraps it and pushes `VERWARNUNG` out of the button entirely. Details in the auto-fit
> note.

**Status:** Idea. Not started. Not on any branch.
**Surfaced:** 2026-08-10, during the player-number-grid walkthrough.
**Raised by:** the user, from direct observation — **German "TEAM WARNING" does not currently fit**
on the warnings page. This is a pre-existing defect on `master`, not something the grid branch
introduced.

## The problem

Translations are longer than English in most European languages, and several UI slots are sized
around the English string. Nobody has systematically checked the other 14 locales against the
actual rendered widths, so overflow is discovered by accident — if at all, since the operator
running the event usually reads English.

**Known instance:** `team-warning-line-1` / `team-warning-line-2` on the warnings page.
- en-US: `TEAM` / `WARNING` — fits.
- de-DE: `MANNSCHAFT` / `VERWARNUNG` — **does not fit** (observed).
- fr: `AVERT.` / `D'ÉQUIPE`, es: `AVISO` / `DE EQUIPO` — unchecked.

## Why a systematic audit, not a one-off fix

Two things make this worth doing properly rather than patching the German string:

**1. Fixed-width slots are everywhere.** The keypad panel's title row is
`3 * MIN_BUTTON_SIZE + 2 * SPACING = 283px` and holds a label plus a value; buttons across the app
use `MIN_BUTTON_SIZE` multiples. Any of them can clip a long translation. `num-tos-per-half`
("NUM OF TEAM / T/Os PER HALF:") is already near the edge **in English**.

**2. Word order differs, so per-word reuse is unsafe.** Romance languages reverse the modifier:
"TEAM WARNING" becomes "AVERT. D'ÉQUIPE" (warning of-team). Any fix that composes a label from a
generic "TEAM" plus a noun will produce wrong grammar in fr/es/pt-PT. Two-line labels must stay
dedicated `*-line-1` / `*-line-2` key pairs, which is what the existing keys correctly do.

## Scope when picked up

- Enumerate every fixed-width text slot (buttons via `make_chrome_button` / `make_tile_button` / `make_multi_label_button` /
  `make_small_button`, and the keypad title row) and every key that fills one.
- For each of the 15 locales, determine whether the string fits at its slot's width and text size.
  Font metrics make this hard to compute reliably — **prefer rendering** over arithmetic. Setting
  the language in Settings and walking the pages is the honest method; a screenshot per locale per
  affected page is the deliverable.
- Decide a remedy per overflow: shorter translation (ask a speaker — do not invent abbreviations
  in a language the user cannot check), smaller text size for that slot, or a wider slot.
- Consider whether a rendered-width test is feasible in CI so this cannot regress silently.

## Explicitly NOT part of this

The `-` readout decision (see [../team-score-button-and-dash-readout/NOTE.md]) already removes the
worst offender — the title row's `"TEAM"` value — by replacing it with a single locale-neutral
glyph. This audit is about the remaining slots, which that change does not touch.

## Caution for whoever picks this up

Do not trust an English screenshot as evidence that a slot is safe. During the 2026-08-10 session
Claude claimed the warnings page's three-button row "proves the fit is safe" on the basis of an
English screenshot; the user immediately corrected it with the German counter-example. Render the
locale you are making a claim about.
