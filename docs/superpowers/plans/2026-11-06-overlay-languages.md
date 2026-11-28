# Overlay: English, Spanish and French, set in the overlay's own settings

**Status:** built and walked by the organiser (2026-11-25). The two rulings from that walk (boxes
widen to fit each word, abbreviations written out) were built the same day and still need the
organiser's look in Spanish and French (see Deviations).
**Crates:** `overlay` only (translations and the setting). No change to `uwh-common`, the refbox,
`overlay-bridge`, the LED panel, the wireless remote or Stream Manager, and no wire format change.
**Process:** lean (the overlay's wording and one setting).
**Builds on:** the foul-cards work (`2026-11-05-overlay-foul-descriptions.md`), so the new GOAL /
FOUL cards and foul names are translated too.

## Goal

The stream overlay shows its fixed words in English, Spanish or French (the three CMAS
languages), chosen by the overlay's own `language` setting. English by default. The refbox's
language never changes the broadcast.

## Behaviour

- **Overlay setting `language`** in the overlay's settings file: `"en"` (default), `"es"` or
  `"fr"`. The refbox's language plays no part.
- **Fallback:** English when the settings file has no `language` (one written before languages
  existed).
- **What is translated:** the overlay's fixed words only, about 25: BLACK / WHITE, 1ST HALF /
  2ND HALF / HALF TIME and the overtime and sudden-death periods, TIMEOUT, REFEREE, PENALTY SHOT,
  NEXT GAME, REFEREES, GOAL, FOUL, "Both teams", and the 11 foul names on the description bars.
- **Not translated:** team names, player names, roles and anything else that comes from the
  schedule or the portal.

## Acceptance criteria (what the organiser can check)

1. Overlay set to `"es"`: the in-game bar, timeouts, penalty shot, half time, next game, roster
   pages and goal / penalty / foul cards read in Spanish. Same for French with `"fr"`.
2. Overlay set to `"fr"` with the refbox in English (or Spanish): the overlay stays French.
3. An overlay settings file without `language` loads, and the overlay shows English.
4. In bridge mode (overlay-bridge in between) 1–3 behave the same.
5. No wire change: any refbox and any bridge work with the new overlay, and the reverse.
6. No word is cut off or overflows on any overlay screen in Spanish or French (checked with
   screenshots of each screen).
7. `just check` (or the same commands run directly) passes.

## How it works (architectural sketch)

No wire change: the refbox's snapshot and the overlay-bridge's `/game` feed are untouched.

### `overlay` — wording and the setting

- New module `overlay/src/words.rs`: a `Lang` (En / Es / Fr) and one function per word or a
  `Word` enum with a `text(lang)` match. Plain Rust tables, **no new dependency** (the refbox's
  translation library would be a new dependency for the overlay, which the project rules ask to
  avoid without discussion; 25 words don't need it). The compiler then guarantees every word
  has all three languages.
- `Lang` is itself the setting: it reads and writes as `"en"`, `"es"` or `"fr"` (tested).
- Every page that draws a fixed word asks `words` for it, using the current `Lang`.
- The overlay's settings file gains `language`, defaulting to `"en"`, and an old settings file
  without it still loads.
- Plain mode and bridge mode both use the setting; neither reads a language from the refbox.

### Starting wording (to be checked by native speakers before release)

Taken from the refbox's existing Spanish and French translations where they exist, shortened
where the overlay has little room. The organiser (or someone they choose) reviews the full list
in the PR. Known issue: the refbox's Spanish for Illegal Advance ("Uso de la mano libre") means
"use of the free arm"; the overlay will use a corrected wording, and the refbox's own text is
left to the separate foul-names change.

## Tasks (rough)

1. `overlay`: `words.rs` with all words in three languages; the `Lang` setting with a
   short-code test; settings field with a test that an old settings file loads.
2. `overlay`: replace each fixed word on every page and on the cards.
3. Try-out: screenshots of every screen in each language (plain and bridge mode); fix anything
   cut off. Organiser checks criteria 1–6.
4. `just check`, then (with the organiser's OK) branches and PRs for Eric.

## Out of scope

- Languages beyond English, Spanish and French (the tables make adding one later easy).
- A settings screen for the overlay's language (it's in the settings file, like the overlay's
  other settings).
- Changing the refbox's own wording, including the foul-names fix ("Illegal Advancement", the
  Spanish error): a separate change.
- Translating names, roles or anything from the schedule or portal.
- Stream Manager's YouTube descriptions.

## Deviations

- **Status:** built 2026-11-06 and verified by the organiser; after the 2026-11-25 ruling below it
  was walked again the same day in English, Spanish and French.
- **Words not in the original list:** "GAME" (in "GAME #12") and the fallback team names
  BLACK / WHITE shown when the schedule gives no names (translated only while they are the
  fallback; real team names never are).
- **Layout:** the timeout and penalty-shot flag lines, "NEXT GAME" and the pre-game period are
  centred on where the English word sits, so English is unchanged; a flag line too long for its
  box shrinks (20 down to 12). Measured with the overlay's font: everything fits. Three Spanish
  words were shortened to fit: "DESC. PRÓRROGA", "ANTES M. SÚBITA", and "T. MUERTO" on team
  timeout flags. The GOAL tab text is now centred like FOUL (about 2 px from before in English).
- **Foul names in Spanish/French** use corrected wording where the refbox's own is wrong or
  awkward (Illegal Advance in Spanish, Obstruction, Sub Foul spelling, Grabbing the Wall,
  Delay of Game in French). English foul names still come from `uwh-common` (unchanged here).
- **Total dismissal** is "TD" in English and Spanish, and "EXP. DÉF." (expulsion définitive) in
  French, after the native speakers' check.
- **Spanish pre-sudden-death** is "PRE M. SÚBITA" (the native speakers' wording), not the
  "ANTES M. SÚBITA" above.
- The refbox's own clippy findings (`player_grid.rs`, `app/mod.rs` field assignment) and the
  overlay's bridge-only dead-code warnings already exist on master and are not touched.
- PO ruling 2026-11-25: the overlay does not follow the refbox's language (a referee's terminal
  must not change the broadcast); `follow-refbox` and the snapshot's `language` field were
  dropped. A language picker on the overlay-bridge status page may follow as its own PR.
- PO rulings 2026-11-25, from the walk (open, for Cagatay, on this PR before it merges): a box
  whose word is longer than the English widens from its centre to fit, keeping its curved ends
  (never stretched, never shrinking the word); and abbreviations are avoided wherever possible
  ("TIEMPO MUERTO", not "T. MUERTO"). Seen on the walk: "PRÓXIMO PARTIDO" runs onto the edges of
  the next-game box. Abbreviations today: "T. MUERTO", "DESC. PRÓRROGA", "PRE M. SÚBITA",
  "PROLONG. 1/2", "EXP. DÉF.".
- Both rulings built 2026-11-25, replacing the "Layout" and "Total dismissal" notes above where
  they differ:
  - **Widening.** A box is widened by repeating one plain column of its artwork, so both ends
    are drawn exactly as they are and nothing is stretched; no word is drawn smaller or cut short.
    With no extra width the artwork is drawn as before, so English is unchanged (checked picture
    by picture against the previous build: 19 scenes, identical).
  - **Which boxes.** The next-game box (next-game, pre-game and roster-list pages) and the
    time-and-period box widen from their centre. A timeout or penalty-shot flag is fixed behind
    the time box, so it grows rightwards, and its slide is lengthened to match; when the time box
    itself is wider the flag sits further right by as much. A card's tab grows rightwards and the
    name bar and description bar move with it.
  - **How much room.** The English decides: the next-game box and each flag have the room their
    English words take; the period name has 180 px during play and 192 px in overtime (the
    longest English name); a card's tab has 76 px (English "GOAL"). In Spanish and French every
    tab is as wide as that language's "FOUL" needs, so names on stacked cards start in line.
  - **Words now written out.** Spanish: "TIEMPO MUERTO" on team flags, "DESCANSO PRÓRROGA",
    "PRE MUERTE SÚBITA". French: "PROLONGATION 1" / "PROLONGATION 2", "MI-TEMPS PROLONGATION",
    "AVANT PROLONGATION" (the last two were also cut short, though not on the walk's list).
  - **Left as they are:** the penalty tab's short forms, "TD" in English and Spanish and "ED"
    (expulsion définitive) in French: at the 2026-11-25 sync-up Eric ruled these stay short.
    Also the ordinals "1ª", "1re", "2e".
- **Stream language chosen on the bridge (added 2026-11-27).** At the 2026-11-25 sync-up Eric
  asked for a dropdown in the bridge, three languages for now, so the stream's language is set
  apart from the refbox's. The bridge's status page has a "Stream language" section (English,
  Español, Français); the choice is saved in the bridge's settings and sent in `/game` as
  `streamLanguage` (never null, sent while the refbox is away too; adding a field does not bump
  the schema version). An overlay reading from a bridge follows it within a poll and it takes
  over from the overlay's own `language` setting; with no bridge, or an older bridge that does
  not send it, the overlay's own setting stays in charge. Built in this branch at the organiser's
  choice rather than in a separate PR. Checked live: a running overlay switched Spanish → French
  when the dropdown was changed.
