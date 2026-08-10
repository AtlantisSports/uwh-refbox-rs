# Backlog: TEAM SCORE button + `-` readout on the keypad panel

**Status: SHIPPED 2026-08-10** — PR #1998, merged to master as `732d04e2`. Nothing left to do here;
this note is kept for the reasoning behind the decisions, not as outstanding work.

The one open question below (§4, symbol vs. word for the team button) was **decided: words.** TEAM
SCORE and TEAM WARNING are two-line text labels with dedicated keys in all 15 locales. That leaves
the German fit problem live — `MANNSCHAFT` is wider than the button — which is deliberately left to
the auto-fit branch rather than patched here. See `../auto-fit-button-text/NOTE.md`.
**Surfaced:** 2026-08-10, during the player-number-grid walkthrough.
**Supersedes:** the earlier `panel-team-value-translation` note (deleted). That note proposed
translating the hardcoded English `"TEAM"` value and redesigning the title row to stop it
overflowing. The user's design below removes the need for both.

## What was wrong

Two things, both made newly visible by the player-number grid (the grid had no title before, so
these only ever showed on the number pad):

1. The title row's value is a hardcoded English `"TEAM"` (three arms of `make_panel_label` in
   `refbox/src/app/view_builders/keypad_pages/mod.rs`) — untranslated for every non-English
   operator, and long translations overflow the fixed 283px row.
2. On the penalty, foul and warning pages the value reads `0` when nothing is chosen. `0` is not a
   value there — `penalty_edit_can_commit` / `foul_add_can_commit` / `warning_add_can_commit` all
   require `player_num > 0` — so it is an empty field wearing a digit.

## The approved design

### 1. Readout becomes `-`

Replace the value with a literal `-` wherever no individual player is named. One glyph, no locale,
cannot overflow any row. **This removes the need for a translated "TEAM" value entirely.**

Applies to: team goal (`AddScore` with `player_num == 0`), team warning (`WarningAdd` with
`team_warning`), equal foul (`FoulAdd` with `color == None`), and the not-yet-entered state on the
penalty / foul / warning pages.

**Does NOT apply to** the `PanelRole::NotPlayer` pages — timeouts-per-half (`0` is a legitimate
value) and portal login (a code may legitimately begin with `0`). Those keep showing real digits.

### 2. TEAM SCORE button on the score page

Mirror the warnings page: a three-button row of `TEAM SCORE` / `BLACK` / `WHITE`. Selecting TEAM
SCORE disables the grid/pad, exactly as `TEAM WARNING` does via `PanelRole::TeamEntry`.

Covers the real cases the user named: the scorer is unknown, or a penalty goal where no individual
is credited.

**User decisions — implement these literally:**
- **TEAM SCORE is NOT the default.** The page opens with it off and the grid live.
- **DONE is disabled** unless either a player number is provided or TEAM SCORE is selected. This is
  a change from today, where `score_add.rs` has an unconditional
  `.on_press(Message::AddScoreComplete { canceled: false })`.
- Claude raised that this costs a mandatory extra tap per goal at no-roster events, where every
  goal is a team goal. The user's ruling: **"an additional tap for accuracy is ok."** Do not
  re-litigate this.
- **The three-button row moves to the top** of the score page. Today `score_add.rs` centres
  BLACK/WHITE with `vertical_space()` above and below; the warnings page has its row at the top.
- **The team option goes BETWEEN black and white**, i.e. `BLACK / TEAM SCORE / WHITE`. The user
  confirmed this by eye from the foul page, whose `=` button already sits in the middle, and wants
  it consistent across pages. **This also means moving the existing `TEAM WARNING` button** on the
  warnings page from the left to the middle — a layout change to an established page, so it is in
  this branch's blast radius even though its behaviour does not change.

### 4. OPEN QUESTION — should the team button be a symbol instead of a word?

Not decided. Raised because the foul page's middle button is a bare `=` glyph at `LARGE_TEXT` with
**no translation at all** (`refbox/src/app/view_builders/keypad_pages/foul_add.rs`, the
`ChangeColor(None)` button — note `equal = EQUAL` exists in the locale files but this button does
not use it).

If TEAM SCORE / TEAM WARNING became glyphs, the two new keys and the German fit risk both vanish,
the same trick that `-` pulls for the readout. Against it: `=` reads naturally as "both teams
equally", but "one team, no individual" has no obvious symbol, and `TEAM WARNING` is a control
operators have already learned to read. Ask the user before assuming either way.

### 3. Translation cost

The button needs **two** new keys, `team-score-line-1` / `team-score-line-2`, following the
existing `team-warning-line-1` / `team-warning-line-2` pattern, in all 15 locales.

**It cannot be composed from a generic "TEAM" plus "SCORE".** Romance languages reverse the
modifier — "TEAM WARNING" is fr "AVERT. D'ÉQUIPE", es "AVISO DE EQUIPO" — so a composed label
would be grammatically wrong in fr/es/pt-PT. `dark-score-line-1` does already carry a standalone
word for score in every locale (SCORE / PUNTUACIÓN / TORE / 得点) but the line-1 slot needs the
*team* word, which no existing key provides reusably (`team-warning-line-1` is fr "AVERT.", es
"AVISO" — those mean *warning*, not *team*).

**Check fit before shipping** — German "TEAM WARNING" already does not fit on the warnings page
today, so a German "TEAM SCORE" is at similar risk. See
[../translation-fit-audit/NOTE.md]. Do not assume an English screenshot proves the fit.

## Scope when picked up

Own branch, own Scope Card and brainstorming pass — it was deliberately kept off
`feat/refbox/player-number-grid`, which was code-complete and green when this was designed.

- `make_panel_label` (the `-` change) touches every keypad page, so the number pad on the game
  number, timeouts and portal login pages is in the blast radius even though their behaviour must
  not change.
- Needs a DONE-gate test mirroring `penalty_gate_depends_only_on_the_player_number` in
  `penalty_edit.rs`, since the score page's gate becomes conditional.
- A new `BoolGameParameter`-style toggle for TEAM SCORE, mirroring
  `BoolGameParameter::TeamWarning`'s handling in `mod.rs`.
