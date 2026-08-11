# Backlog: colour the player-number grid cells to match the selected team

**Status: IMPLEMENTED 2026-08-10** — PR #2015 on `feat/refbox/team-coloured-grid-cells`, open for
review. Kept for the reasoning; the outstanding-work framing below is historical.

Outcomes that settled the open questions in this note:

- **"Judge by eye before accepting" — done, and solid white was confirmed.** Walked through locally
  with the light team's grid deliberately filled to all 12 cells, i.e. the brightest case 6v6 can
  produce. The human's ruling: solid white is fine, keep it. The grey fallback was NOT applied.
- **The Rugby worry in that section was wrong.** Rugby's rows are height-constrained, so its 15 cells
  come out ~70px tall against 89 wide — about 93,500 px² of fill against 6v6's 95,100. Rugby is
  marginally *less* coloured area, not more, so it needs no separate brightness decision.
- **High Contrast needs no check by eye after all.** The code answers it: `black_button` and
  `blue_button` produce an identical style in that mode, and `white_button` gives `HC_DARK_GREY`. High
  Contrast discards button colour by design, so the dark team's grid is unchanged there and the light
  team's is slightly lighter grey. Nothing was special-cased, so #1912's selection work is untouched.
- **Dark display mode was checked and approved** on 2026-08-10, after this note merged saying it
  had never been viewed. No visual gap remains. Lower risk than Light in any case, because Dark's
  `white()` is `rgb8(207,207,207)` rather than pure white.

The estimate held: the change is `cell_style` plus a threaded `PanelRole`, in one file.
**Surfaced:** 2026-08-10, during the TEAM SCORE walkthrough.
**Raised by:** the user — *"changing the color of the player number buttons to match the team
selection. that way there is a more clear difference between the number pad (black/white) and the
keypad (blue)."*

## The idea

The player-number grid's cells are currently `blue_button`, the same as the digit keypad. Colour
them to match the team whose roster is shown — black cells with white numbers for the dark team,
white cells with black numbers for the light team — leaving the digit keypad blue.

Two benefits: the operator can see *whose* roster it is without glancing at the team buttons in the
corner, and the grid becomes instantly distinguishable from the keypad when toggling between a team
that has a roster and one that does not.

## Why it is small (verified 2026-08-10)

- **Selection is a border, not a colour.** Every `*_selected_button` in
  `refbox/src/app/theme/button.rs` is its base style plus `border.width = BORDER_WIDTH` and
  `select_in_high_contrast(...)`. So a coloured cell does not collide with showing which cell is
  selected. `black_selected_button` (line 161) and `white_selected_button` (line 134) already exist.
- **High Contrast is already handled centrally** by `select_in_high_contrast` /
  `outline_in_high_contrast`, the same helpers every other button uses, with an existing
  `high_contrast_tests` module. This does not reopen the HC selection work from #1912.
- **Empty cells keep their look for free.** `black_button` at `Status::Disabled` falls back to
  `window_background()`, exactly as `blue_button` does, so the leftover `None` cells and the greyed
  panel render as they do today with no extra code.

Estimated ~20 lines: thread the team `GameColor` into `make_player_grid` → `make_grid_cell` (both in
`refbox/src/app/view_builders/keypad_pages/player_grid.rs`) and pick the style pair from it. The
colour is always available where a grid is built — `build_keypad_page` only builds one for
`PanelRole::Player(color)`.

## Judge by eye before accepting

**A 4×3 block of solid white is much brighter than a single WHITE button.** Twelve of them (or
fifteen in Rugby) is a different proposition from the one team-select button, especially outdoors on
the Pi. May need toning down — a lighter grey fill, or white with an outline instead of a solid
fill. This cannot be judged from the code.

Also check High Contrast mode by eye, since black-on-black and white-on-white are exactly the cases
that mode exists to fix.

## Scope when picked up

Own branch, own Scope Card. Deliberately **not** added to
`feat/refbox/player-number-grid`, even though it belongs to that feature conceptually: that branch
has already been walkthrough-verified, and this would change the very thing that was verified.
