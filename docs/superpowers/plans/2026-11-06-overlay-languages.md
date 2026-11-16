# Overlay: English, Spanish and French, following the refbox's language

**Status:** built, and verified by the organiser with the refbox (2026-11-06)
**Crates:** `uwh-common` (one added field), `refbox` (fills it in), `overlay-bridge` (passes it on),
`overlay` (translations and the setting). No change to the LED panel, the wireless remote or
Stream Manager.
**Process:** heavy for the `uwh-common` field and the two feeds (they are wire formats); lean for
the overlay's wording.
**Builds on:** the foul-cards work (`2026-11-05-overlay-foul-descriptions.md`), so the new GOAL /
FOUL cards and foul names are translated too. Worked on in a separate local copy
(`Downloads\overlay-languages`), no branch or commit until the organiser has seen it working.

## Goal

The stream overlay shows its fixed words in the refbox's language: English, Spanish or French
(the three CMAS languages). When the refbox's language is changed, the overlay follows within a
second, with no restart. An overlay setting can instead fix the language.

## Behaviour

- **Overlay setting `language`** in the overlay's settings file:
  - `"follow-refbox"` (default): use the language the refbox sends.
  - `"en"`, `"es"` or `"fr"`: always that language, whatever the refbox says.
- **Fallbacks:** English when following a refbox that sends no language (an older refbox) or a
  language the overlay doesn't have (e.g. German).
- **What is translated:** the overlay's fixed words only, about 25: BLACK / WHITE, 1ST HALF /
  2ND HALF / HALF TIME and the overtime and sudden-death periods, TIMEOUT, REFEREE, PENALTY SHOT,
  NEXT GAME, REFEREES, GOAL, FOUL, "Both teams", and the 11 foul names on the description bars.
- **Not translated:** team names, player names, roles and anything else that comes from the
  schedule or the portal.

## Acceptance criteria (what the organiser can check)

1. Refbox in Spanish, overlay on `follow-refbox`: the in-game bar, timeouts, penalty shot, half
   time, next game, roster pages and goal / penalty / foul cards read in Spanish. Same for French.
2. Change the refbox's language while the overlay runs: the overlay switches within a second.
3. Refbox in German: the overlay shows English.
4. Overlay set to `"fr"` with the refbox in English: the overlay stays French.
5. In bridge mode (overlay-bridge in between) 1–4 behave the same.
6. An older refbox (no language sent) still works with the new overlay, in English; the new
   refbox still works with an older overlay and an older bridge (they ignore the extra item).
7. No word is cut off or overflows on any overlay screen in Spanish or French (checked with
   screenshots of each screen).
8. `just check` (or the same commands run directly) passes.

## How it works (architectural sketch)

### `uwh-common` — one added field

`GameSnapshot` gains `language: Option<String>`, a language code such as `"es"` or `"zh-CN"`.
`None` from an older refbox. Serde reads a missing optional field as `None`, and older readers
ignore unknown fields, so both directions keep working (criterion 6). Only the JSON snapshot
(`GameSnapshot`) changes; the LED panel's compact binary snapshot (`GameSnapshotNoHeap`) does not.
A test checks that a snapshot without the field still reads, and one with it round-trips.

Downstream crates that build a `GameSnapshot` in full (refbox, overlay, overlay-bridge,
stream-manager, their tests) get `language: None` where they don't care.

### `refbox` — fill it in

Where the refbox sends a snapshot to the overlays (the two `send_snapshot` calls in
`refbox/src/app/mod.rs`), it sets `language` to the language its screens are actually in
(`LANGUAGE_LOADER.current_languages()[0]`, the same value the settings page already uses). That
covers both an explicitly chosen language and "use the computer's language".

### `overlay-bridge` — pass it on

The `/game` feed gains `language` (the snapshot's value; `null` when disconnected). The feed's
contract allows an added field without a version bump; older overlays ignore it.

### `overlay` — wording and the setting

- New module `overlay/src/words.rs`: a `Lang` (En / Es / Fr) and one function per word or a
  `Word` enum with a `text(lang)` match. Plain Rust tables, **no new dependency** (the refbox's
  translation library would be a new dependency for the overlay, which the project rules ask to
  avoid without discussion; 25 words don't need it). The compiler then guarantees every word
  has all three languages.
- `Lang::pick(setting, snapshot_language)` decides the language each frame (pure, tested).
- Every page that draws a fixed word asks `words` for it, using the current `Lang`.
- The overlay's settings file gains `language`, defaulting to `follow-refbox`, and an old settings
  file without it still loads.
- Plain mode reads `language` from the refbox snapshot; bridge mode reads it from the feed.

### Starting wording (to be checked by native speakers before release)

Taken from the refbox's existing Spanish and French translations where they exist, shortened
where the overlay has little room. The organiser (or someone they choose) reviews the full list
in the PR. Known issue: the refbox's Spanish for Illegal Advance ("Uso de la mano libre") means
"use of the free arm"; the overlay will use a corrected wording, and the refbox's own text is
left to the separate foul-names change.

## Tasks (rough)

1. `uwh-common`: add `language`, with compatibility tests. Check every crate still builds.
2. `refbox`: set `language` on sent snapshots, with a test.
3. `overlay-bridge`: `language` in the `/game` feed, with tests (connected / disconnected).
4. `overlay`: `words.rs` with all words in three languages; `Lang::pick` with tests; settings
   field with a test that an old settings file loads.
5. `overlay`: replace each fixed word on every page and on the cards.
6. Try-out: screenshots of every screen in each language (plain and bridge mode); fix anything
   cut off. Organiser checks criteria 1–7.
7. `just check`, then (with the organiser's OK) branches and PRs for Eric.

## Out of scope

- Languages beyond English, Spanish and French (the tables make adding one later easy).
- A settings screen for the overlay's language (it's in the settings file, like the overlay's
  other settings).
- Changing the refbox's own wording, including the foul-names fix ("Illegal Advancement", the
  Spanish error): a separate change.
- Translating names, roles or anything from the schedule or portal.
- Stream Manager's YouTube descriptions.

## Deviations

- **Status:** built 2026-11-06 and verified by the organiser. Split into two PRs as agreed:
  (A) `uwh-common` + `refbox` (the `language` field and the refbox filling it in), and
  (B) `overlay` + `overlay-bridge` (wording, setting, passing the language on). Adding the field
  forces a one-line `language: None` in test fixtures in `overlay-bridge/src/feed.rs` and in the
  overlay's bridge reader, so PR A touches those two lines too.
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
- **Total dismissal** stays "TD" in all languages (to be confirmed by native speakers).
- The refbox's own clippy findings (`player_grid.rs`, `app/mod.rs` field assignment) and the
  overlay's bridge-only dead-code warnings already exist on master and are not touched.
