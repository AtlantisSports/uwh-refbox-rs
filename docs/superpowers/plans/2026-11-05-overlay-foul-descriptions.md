# Overlay: foul descriptions on penalties, and foul cards

**Status:** built, and verified by the organiser with the refbox and vMix (2026-11-06)
**Crates:** `overlay` and `overlay-bridge`. No change to the refbox, `uwh-common` or Stream Manager.
**Process:** lean (overlay layout work), with extra care on the bridge feed, which is a published
contract (adding a field is allowed without a version bump; nothing is removed or renamed).

## Goal

1. **Penalties show their foul.** When a penalty is entered, a description bar with the foul
   name ("OBSTRUCTION") slides out next to the penalty card, holds for 10 seconds, and slides back.
   The penalty card itself behaves as today.
2. **Fouls get their own card.** When a foul (no penalty) is entered, a new **foul card** appears in
   the same place as the penalty cards: a yellow "FOUL" tab, the player's cap number and name, and
   the description bar with the foul name. After 10 seconds the description bar slides back and
   the whole foul card fades out.

## Design (approved in the mock-up, 2026-11-05)

- **Foul card:** exactly the penalty card's artwork and shape (same curve between tab and name
  bar, same rounded ends, same shadow), with the tab **yellow** (#F2B705, shaded like the red one)
  instead of red, and dark text "FOUL". The name bar keeps the team colour (white for White, dark
  for Black), like the penalty card.
- **Description bar:** a separate bar after the name bar, with a small gap. It is the same height
  as the name bar and lined up with it, and **both ends are rounded** like the name bar's right
  end. Text is in **CAPITALS**.
  - Penalty: the same semi-transparent black as the name bar, with a **thin red edge** on its left.
  - Foul: yellow, with dark text.
- **Animation:** the bar reveals from left to right (about 0.45 s), holds for **10 s**, then hides
  right to left (about 0.45 s). The foul card then fades out like other cards do.
- **Font:** Bahnschrift, as on the other cards.
- **Position and stacking:** the same top-left stack as goal and penalty cards. A foul card takes
  a slot in the stack while it is shown, and the cards below move as they do today.

## Acceptance criteria (what the organiser can check)

With the refbox and the overlay (bridge mode, NDI into vMix, and the plain window mode):
1. Enter a penalty, e.g. Black #12, 1 min, Obstruction. The penalty card appears as today, and the
   "OBSTRUCTION" bar slides out next to it. After 10 s it slides back; the penalty card stays
   until the penalty ends.
2. Enter a foul, e.g. White #4, Stick Foul. A yellow-tab foul card with "#4 <name>" appears, the
   "STICK FOUL" bar slides out, and after 10 s the bar slides back and the card fades away.
3. In vMix, the new pieces key cleanly over the video, with no black boxes or halos (the overlay's
   transparency output includes them).
4. A penalty or foul deleted on the refbox within the 10 s disappears (no stale card).
5. Goals, penalties without a known foul ("Unknown"), timeouts and all other screens look and
   behave exactly as before. A penalty with an unknown foul shows no description bar.
6. `just check` (or the same commands run directly) passes.

## How it works (architectural sketch)

### Data

- **Plain mode** (overlay reads the refbox directly): the refbox snapshot already carries
  `penalties[*].infraction` and `fouls` (black / white / "equal" lists, each with an optional cap
  number and the infraction). Nothing new is needed.
- **Bridge mode** (`overlay-bridge` feeds the overlay over `/game`):
  - Penalties: the feed already carries `infraction` (the refbox's short name). The overlay
    currently throws it away (`bridge_network::build_snapshot` sets `Infraction::Unknown`). It
    will now map the short name back to the `Infraction` value.
  - Fouls: **new** `fouls` list in the feed, one entry per foul: team ("Black"/"White"/none), cap
    number (optional), player name (resolved from the rosters the same way as for penalties), and
    infraction short name. This is an added field, which the feed's contract allows without a
    version bump. Older overlays ignore it.

### Overlay (`overlay/src/flag.rs` and its renderer)

- `Type::Penalty` gains the infraction and the moment the card appeared. When the infraction is
  known, the description bar is drawn after the card, with its reveal, hold and hide timing worked
  out from that moment.
- New `Type::Foul(colour, infraction, since)` card:
  - Created when a foul appears in the snapshot that wasn't there before. Fouls are matched by
    team, cap number and infraction, counting duplicates, as for the Stream Manager highlights.
  - Moved to the fading-out list 10 s plus the hide time after it appeared, or at once if the foul
    is deleted on the refbox.
  - The first snapshot after the overlay starts or reconnects sets the starting point, so fouls
    already on the refbox don't all pop up at once.
- **Artwork:** new `Foul White.png` and `Foul Black.png` in `overlay/assets/color/1080`, made from
  the penalty cards with the red tab turned yellow, plus matching transparency masks in
  `overlay/assets/alpha`. These are copies of the penalty masks, since the shape is the same.
- **Description bar** is drawn by code (a rounded bar plus text) into **both** the colour picture
  and the transparency picture, the same way the existing cards' text is drawn, so it keys cleanly
  in vMix.
- **10 s** is a constant (`DESCRIPTION_HOLD`), easy to change, or to move into the settings file
  later if wanted.
- Fouls with no team ("equal" fouls): the card uses the white name bar and reads "Both teams"
  instead of a player.

### Bridge (`overlay-bridge/src/game_feed.rs`)

- `GameFeed` gets `fouls: Option<Vec<Foul>>` (`None` when disconnected, like `penalties`).
- `fouls_of(snapshot, rosters)` builds the list in the refbox's order, with names resolved like
  `penalties_of`.
- Tests: a connected feed carries fouls; a disconnected feed has `fouls: null`; names resolve; the
  existing feed tests still pass.

## Tasks (rough)

1. Bridge: `fouls` in the feed, with tests.
2. Overlay, bridge mode: read `infraction` and `fouls` back into the snapshot, with tests.
3. Overlay: the foul card type, appearance and expiry rules, with tests for the matching and
   timing (pure functions, no drawing).
4. Overlay: yellow-tab artwork and masks; draw the description bar into colour and transparency.
5. Local try-out with the refbox simulator or a real refbox: plain window, then bridge and NDI into
   vMix (acceptance criteria 1–5). Screenshots for the organiser.
6. `just check`, then (with the organiser's OK) a branch `feat/overlay/foul-descriptions` and a PR
   for Eric.

## Out of scope

- Warnings (the refbox also records them); not requested.
- A setting for the 10 s (it's a constant for now).
- Any refbox, `uwh-common`, Stream Manager or vMix-table change.
- Changing the look of goal or penalty cards beyond adding the description bar.

## Deviations

- **No new artwork files.** The foul card is made from the penalty card artwork when the overlay
  starts (red tab pixels blended to yellow, the same recolouring as the approved mock-up), and it
  uses the penalty card's transparency mask as is. This keeps the two cards' shapes identical
  forever and adds no files to keep in step.
- **Fouls already on the refbox when the overlay starts** aren't shown (the first snapshot sets the
  starting point). Reconnecting doesn't reset it: fouls entered while the overlay was cut off
  appear when it reconnects.
- **Ordering in the stack:** penalties on top (as before), then fouls, then goals.
- **A foul with a team but no cap number** shows the team name instead of a player.
- **After the first try-out (organiser's feedback):** a White team penalty's description bar is
  see-through white with black text (matching its name bar), not black. The bar now waits 1 s
  after the card appears before sliding out, so a foul card is shown about 11.9 s in total.
- **Eric's feedback (2026-11-06), items 1 and 2:** goal, penalty and foul cards now fit the name:
  a standard width (480 px) that fits most names, growing for longer ones up to 1000 px (beyond
  that the name is cut short with ".."). Only the plain middle of the artwork stretches; the tab
  and rounded end are unchanged, and the description bar follows the card's end. Penalty times
  on the red tab are white for both teams (black was hard to read on the darker red). His items 3
  (foul names, e.g. "Illegal Advancement") and 4 (English / Spanish / French) are separate work.
