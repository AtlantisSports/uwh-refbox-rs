# Fouls without a player, and Equal fouls, reach the Portal — design

**Status:** APPROVED by the PO 2026-11-16 (short design, small change). Refbox only.
**Portal side:** uwhportal#564 / portal register #338, rulings Q860 + Q861 (PO, 2026-11-16). Not built yet.

## Why

Today the refbox sends the Portal only fouls that name both a team and a player. Fouls the refs recorded without a
player, and "both teams at fault" (Equal) fouls, never reach the Portal, so its foul counts are incomplete. The
Portal currently rejects such fouls (and with them the whole game's stats), which is why they were held back
(an untracked backlog note, `docs/backlog/portal-accept-team-and-equal-fouls/NOTE.md`, kept in the
main checkout only; the Portal side is uwhportal#564). The PO has ruled the Portal will accept them.

## What the PO ruled

- A team stays mandatory on a BLACK/WHITE foul; the **player number becomes optional**. A foul with no number is
  not a separate "team foul" kind — it means the refs did not record which player.
- The **"=" (Equal) button stays.** On the Portal an Equal foul counts toward BOTH teams' totals (Q860) and is
  labelled "Equal Puck" there (Q861). The refbox's own "=" / "EQUAL" wording is NOT changed by this work.
- **Landing gate:** no refbox release may carry these fouls until the Portal change is LIVE IN PRODUCTION. The PR is
  built and checked now but held unmerged until the coordinator confirms that; rebase and re-check before merging.

## Goals

1. Foul screen: with BLACK or WHITE picked, DONE is enabled with the number left blank (an infraction is still
   required). A blank number is stored as "no player" — never as player 0.
2. End of game: every foul — Black, White and Equal — is added to the stats sent to the Portal. Wire shape:
   no player → `playerCapNumber: null`; Equal → `side: null` (and `playerCapNumber: null`).

3. Summary page (PO ruling 2026-11-16, option 1): a numberless BLACK/WHITE foul shows NO number, not the "T"
   that team warnings show (the shared box would otherwise print "T"). Team warnings keep "T".

## Non-goals

- Any Portal change. Sending warnings to the Portal (separate backlog item). Fouls in the in-play running stats list.
- Changing the "=" button or the "EQUAL" heading to match the Portal's "Equal Puck".

## Approaches considered

- **Chosen — relax the number + send every foul.** Smallest change; matches the PO's framing.
- A separate "team foul" button: rejected — a missing number is not a separate kind of foul (PO).
- Send no-player fouls but keep holding Equal fouls back: rejected — the Portal will accept them (Q860).

## Known code facts (origin/master, 2026-11-16)

- `end_game` (refbox/src/tournament_manager/mod.rs ~2226) loops only over the Black and White foul lists and skips
  fouls with no player. Equal fouls live in a separate list (`fouls[None]`) the loop never visits — so the fix is
  both: drop the player check AND include the Equal list. Test to rewrite: `end_game_sends_only_fully_attributed_fouls`.
- `foul_add_can_commit` (refbox/src/app/view_builders/keypad_pages/foul_add.rs) requires a number once BLACK/WHITE is
  picked; its test `foul_individual_needs_number` pins that and must change.
- The commit site (refbox/src/app/mod.rs ~5285) turns the keypad number into `Some(n)` for BLACK/WHITE; a blank
  keypad is 0, so 0 must map to `None`. Check the edit path (changing an existing foul) the same way.
- The foul list already shows a foul with no player (Equal fouls use it today).
- `GameStats::add_foul` already serialises null player/side (unit test
  `foul_events_serialize_with_type_side_player_and_called`).

## Verification

- Automated: the rewritten end_game test pins all three kinds on the wire; the screen-gate test pins "BLACK/WHITE with
  blank number may be saved"; a test pins blank → no player (never 0).
- PO walkthrough: record a player foul, a BLACK foul with no number, and an Equal foul; each shows correctly in the
  foul list; editing a numbered foul to blank works.
- Wire check against a Portal is not possible until the Portal change exists; the test above stands in for it.
