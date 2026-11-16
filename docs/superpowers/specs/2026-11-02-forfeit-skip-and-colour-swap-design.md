# Skip a forfeited game; swap a game's colours (portal register #152) — refbox design

**Status:** ✅ PO-APPROVED 2026-11-02, as written, including the design session's calls (option 1); R9 added the
same day. Cross-repo shape AGREED 2026-11-02 (see "Agreement" below), including R9's not-played mark. Nothing is built; the portal pieces build after #118.
Builder check run 2026-11-02 (fresh reviewer): two false claims corrected (overlay name source; end-of-game
CANCEL), one gap closed (held next game must yield to a forfeit), one PO question (R8).
**Source:** portal register `#152` (uwh-portal `docs/superpowers/specs/2026-09-26-feature-requests-register.md`,
branch `docs/feature-requests-register`), handed to the refbox design lane under the PO's "split by workspace"
ruling (2026-11-02). The portal keeps its own server pieces; this document is the refbox's half.
**Grounded at:** uwh-refbox-rs `origin/master` `638edf30`.

## Intent

Two things go wrong at the pool today and the refbox cannot help with either:

1. **A forfeited game.** An organiser records a forfeit on the portal (#118's Game Management tab, designed, not
   built). The refbox does not know, so it counts down to the forfeited game and starts its clock. The portal
   already throws away a refbox score for a forfeited game (#118 Q303), so this is about the referee's day, not
   about protecting results.
2. **Teams in the wrong caps.** Teams jump in wearing the opposite colours to the schedule. Every goal the refbox
   records is credited to a colour, so the result, the stats and the stream overlay all name the wrong team.

## Goals

- The refbox steps past a game the portal has marked forfeited, with no clock run and no score sent.
- The operator can correct a colour mix-up at the pool, and the portal, its schedule and the overlays follow.
- A colour swap made on the portal reaches the refbox too: one record, either side can change it.

## Non-goals

- The portal's server pieces: marking forfeits in the schedule the refboxes download, receiving and storing a
  refbox swap (#118 Q307's swap), its history line. Designed by the portal from the agreement this design needs.
- The website refbox in uwh-portal. It copies the desktop refbox's behaviour in its own repo.
- Recording a forfeit FROM the refbox. Forfeits are recorded on the portal only.
- Any change to the LED panel or the wireless remote.

## Anti-goals (a failure even if it technically works)

- Team names flipping sides on the operator's screen mid-game when the operator did nothing.
- A swap that moves goals between colours. Goals, fouls, warnings and penalties stay with the colour that earned
  them; only the team names move (PO, 2026-11-02, correcting the synopsis).
- A swap message that undoes an organiser's own swap because both happened.
- A site that does not understand the new swap message (a custom site, an older portal) jamming the upload queue
  so that scores stop going out.

## Rulings (PO, 2026-11-02, this lane's chat unless marked)

| # | Ruling |
|---|--------|
| R0 | Both halves, both refboxes (portal design session, synopsis option 1). |
| R1 | After a forfeited game the refbox counts down to the NEXT game's SCHEDULED start, no early start (portal Q468 option 1). |
| R2 | **When the refbox learns of changes: keep today's timing** (option 3). It re-reads the schedule at the end of every game, on REFRESH, and at start-up / event pick. No automatic re-read on a timer. A forfeit entered during a break needs a REFRESH. Declined: re-read about once a minute between games (1); also during play (2). |
| R3 | The forfeit behaviour below, as played back, **including every [my call] in it** (option 1). |
| R4 | Swap colours works until the game's final score is sent (option 1). After that, fixes are on the portal's Game Management tab. Declined: before kickoff only (2); until the game ends (3). |
| R5 | **The SWAP COLOURS button is on the score edit page, at the bottom between BACK and APPLY** (PO's own answer; declined my INFORMATION-page, main-screen and Settings options). |
| R6 | **A mix-up known before the game is fixed on the portal, then the refbox is REFRESHed. One known just before or during the game is swapped on the refbox, which updates the portal** (PO's own answer to the pre-kickoff gap: the score edit page cannot be opened between games). |
| R8 | **The refbox's own stream overlay re-reads team names from the portal during a game, about every 15 s like the overlay bridge** (option 1, after the builder check found the overlays take names from the portal, not the refbox). Built as its own follow-up PR. Declined: accept that the classic overlay shows a swap only from the next game (2); the refbox sends team names to the overlays itself (3). |
| R9 | **A game the portal marks "Not played: both teams withdrew" (#118 Q311) is skipped exactly like a forfeited one**; its info-table line reads "Game N not played, skipped" (option 1; the portal adds a mark for these games). Declined: run it as an ordinary game (2); decide later (3). |
| R7 | The swap behaviour below, as played back, with one PO change: **the swap is saved by APPLY and cancelled by BACK (which reads CANCEL once anything on the page has changed)**, like every other change on that page. This replaces my separate "are you sure?" box. |

## Part 1 — skipping a forfeited game

**How the refbox learns.** The portal marks a forfeited game in the schedule it sends (`GET
/api/events/{eventId}/schedule/privileged`). The refbox sees the mark at its next schedule download (R2): the end
of every portal-linked game, REFRESH on the INFORMATION page, start-up and event pick.

**Between games** (R1, R3):
- If the next game on this court is forfeited, the refbox skips it. The NEXT GAME countdown runs to the scheduled
  start of the next game on this court that is NOT forfeited. No clock runs for the forfeited game and nothing is
  sent to the portal for it.
- Several forfeited games in a row are all skipped. [my call, R3]
- If no playable game remains on this court, the refbox shows the existing end-of-schedule state (banner END,
  START NOW inactive). [my call, R3]
- The game-info table names the real next game and adds a line saying what was skipped. [my call, R3] Sketch:

```
┌─────────────────────────────────┐
│  NEXT GAME              12:34   │
├─────────────────────────────────┤
│  GAME 13 · 10:40                │
│  Sharks  v  Rays                │
│  Game 12 forfeited, skipped     │
└─────────────────────────────────┘
```

- The game picker in Settings labels a forfeited game FORFEITED. [my call, R3]
- The LED panel and the stream overlay simply show the real next game. [my call, R3]

**Edges:**
- **Picked by hand.** The operator may still pick a forfeited game in Settings; the refbox then runs it as an
  ordinary game and sends its score as usual; the portal ignores that score (#118 Q303). This leaves room for teams
  who turn up and play anyway; the real fix is unticking the forfeit on the portal. [my call, R3]
- **Forfeit arrives after kickoff** (the game was already running when the mark arrived): the game carries on
  unchanged; its score goes to the portal, which ignores it (Q303). [my call, R3] Under R2 this can only happen
  through the restart path, since a running game offers RESTART, not REFRESH.
- **Forfeit undone on the portal:** the game comes back as the next game at the next download, if its scheduled
  start has not already passed. [my call, R3]

## Part 2 — swapping a game's colours

**Before the game** (R6): the organiser swaps on the portal (#118 Q307); the operator presses REFRESH; the
schedule arrives with the teams already on their new sides. Nothing new on the refbox for this direction beyond
reading the schedule as it comes.

**At kickoff or during play** (R4, R5, R6, R7): a **SWAP COLOURS** button on the score edit page.

```
┌──────────────────────────────────────┐
│ FIRST HALF                    8:41   │
│        BLACK            WHITE        │
│       Sharks             Rays        │
│      [-] 2 [+]        [-] 0 [+]      │
│ [BACK]   [SWAP COLOURS]    [APPLY]   │
└──────────────────────────────────────┘
```

- **Where:** the score edit page the operator opens by tapping a score during play, between BACK and APPLY (R5).
  Also on the same page in its end-of-game form (opened when the operator answers that the final score is not
  correct), between CANCEL and DONE, so a swap is possible right up to the score being sent (R4). [my call, R3-style
  play-back, approved in R7]
- **Team names on the page:** the page shows each colour's team name under BLACK / WHITE, so the operator can see
  what SWAP COLOURS did before saving. Today the page shows only BLACK / WHITE. [design session's call]
- **Pending until saved** (R7): pressing SWAP COLOURS switches the two names on the page. Pressing it again
  switches them back. APPLY (or DONE) saves the swap together with any score change on the page; BACK, which
  reads CANCEL once anything on the page has changed, throws it away. The swap counts as a change for APPLY's
  enabled state and for the BACK → CANCEL label.
- **What changes once saved:**
  - the team names switch sides everywhere the refbox shows them: the game-info table, the INFORMATION page and
    the score edit page. **The overlays are NOT fed names by the refbox** (corrected after the builder check): the
    refbox's game data carries no team names (`uwh-common/src/game_snapshot.rs`); the overlay bridge reads names
    from the portal's schedule every 15 s (`overlay-bridge/src/server.rs` `PORTAL_REFRESH_INTERVAL`), and the
    classic stream overlay reads them once per game (`overlay/src/network.rs` `fetch_game_data`). So the overlays
    follow a refbox swap only once the portal has applied it — see "Overlays" below;
  - the cap-number lists switch with the teams, so tapping a White goal offers the roster of the team actually
    wearing white. Without this the operator would pick the other team's players. [my call, approved in R7]
  - goals, fouls, warnings, penalties and the score stay with their colour (anti-goal).
- **Telling the portal:** on save the refbox sends ONE message saying which team wore which colour in that game
  ("game 12: Sharks dark, Rays light"), not "swap". Sending the same message twice, or after an organiser's own
  swap, cannot flip the result back. [my call, approved in R7] It goes out at once (so the portal's live view and
  the overlays follow during the game), through the same upload queue as scores: if the refbox is offline it waits
  and goes when the connection returns. [my call, approved in R7]
- **The swap goes before the final score** (corrected 2026-11-02 by the agreement): the portal refuses a swap for a
  game that already has a final score (`409 AlreadyScored`). Only a final score counts; live updates and stats never
  do. So a game's final-score upload waits in the queue until that game's swap is delivered or permanently refused.
  Today each queued item retries on its own, so this ordering is new refbox work. Goals stay with their colour
  either way (#118 Q307).
- **End-of-game form** (corrected after the builder check): on that page CANCEL is permanently disabled today
  (`score_edit.rs`) and stays so; a pending swap is undone by pressing SWAP COLOURS again before DONE. If the
  confirmation countdown runs out first, the app returns to the main screen without reading the page
  (`Message::AutoConfirmScores`), so an UNSAVED swap is lost, exactly as an unsaved score change is today.
  [design session's call]
- **When the swap cannot be delivered.** Today the refbox's portal code reports every failed upload the same way
  (no status number kept). The swap needs two cases told apart: "this site does not accept swaps" (dropped, logged,
  never blocks the queue — see custom sites below) and any temporary failure (expired login, outage, server error),
  which is retried like a score and, while unsent, shows on the portal indicator and its attention page the way a
  stuck score does. How the two are told apart is settled with the portal in the agreement. [design session's call]
- **A portal swap during play** cannot reach the refbox mid-game: under R2 a running game has no REFRESH. It
  arrives with the end-of-game download. No mid-game name flip (anti-goal).

## Overlays (R8)

- **Overlay bridge (vMix feed):** no change. It already re-reads the portal schedule every 15 s, so it shows a
  refbox swap about 15 s after the portal has applied it.
- **Classic stream overlay (`overlay` crate):** today it reads a game's team names once, when the game loads. It
  gains a re-read of the portal schedule during a game, about every 15 s, so a mid-game swap appears within about
  15 s of the portal applying it. A failed re-read keeps what is on screen (never blanks a name it already has).
  This is a third crate, so it ships as its OWN follow-up PR (`.claude/rules/workspace.md`: no third crate on one
  branch without discussion; discussed and ruled here).
- On a custom site that does not accept swaps, neither overlay can follow a refbox swap; accepted.

## What crosses to the portal (cross-repo; goes coordinator → portal coordinator, never direct)

1. **The schedule download gains an optional forfeit mark per game.** The refbox reads it if present. Older
   refboxes ignore fields they do not know (the schedule `Game` type does not reject unknown fields), so they keep
   today's behaviour and the portal still discards their score (Q303).
2. **A new refbox → portal call: "game N: team X dark, team Y light"** (absolute, repeat-safe), with the refbox
   access key. The portal applies it as #118 Q307's swap; its history shows "Refbox (<court>)".
3. **Not touched: the live game updates** (`PUT /api/events/{eventId}/live-games/{gameNumber}`, the agreement in
   portal PR #1146). Their body names colours only (Black/White), never teams, so a swap needs no new field there.
   The portal design session assumed a new live-updates agreement version; on this design none is needed. The
   coordinators confirm.
4. **Custom sites** (`docs/third-party-integration.md`): item 1 is an optional field they may leave out; item 2 is
   a new optional call. A site that refuses it (for example "not found") must NOT block the queue: the refbox
   records the refusal in its log, drops that one message, keeps its own swap on screen, and keeps sending scores
   and stats. [design session's call] The third-party document gains both items.

**Agreement (2026-11-02, AGREED between the refbox and portal design lanes via the coordinators; portal register
516187b57f; not PO-ruled on the portal side). This replaces items 1 and 2 where they differ:**
- **Forfeit:** there is no new field. The refbox reads the existing per-game `score.dark.{value,forfeit}` /
  `score.light.{value,forfeit}` in `/schedule/privileged`. A game is forfeited when either side's `forfeit` is
  true; only one side can forfeit (#118 Q310). An absent or null `score` means not forfeited. The portal pins this
  JSON with an API test.
- **Not played (R9):** each game in `/schedule/privileged` gains an optional `"notPlayed": "BothTeamsWithdrew"`.
  Absent or null = played as normal. It is a reason string, so the refbox skips on ANY present non-null value,
  including a reason it does not know yet. It is never set on a POD game where only one partner withdrew (#118
  Q317: that game plays on). The portal pins it with the same API test.
- **Swap call:** `POST /api/events/{partialEventId}/schedule/games/{gameNumber}/colours`, body
  `{"darkTeamId": "teams/…", "lightTeamId": "teams/…"}` (the colours actually worn; absolute). Auth is the existing
  PushStats scope, the same as the score upload. Answers:
  - `200`: applied, or already that way. Repeat-safe.
  - Permanent refusals, each with a JSON reason. The refbox shows the reason and does not retry, and the held final
    score is then released: `404 EventNotFound` / `GameNotFound`; `409 TeamsDoNotMatch`; `409 TeamsNotSet` (a race
    guard only, because the refbox never sends a swap when a side has no team id in its schedule); `409
    AlreadyScored` (a final score exists, from the score upload or typed on Game Management; never from live
    updates or stats).
  - **Does not accept swaps:** a `404` or `405` whose body is NOT one of those named reasons. The refbox keeps its
    local swap, logs it, and stops sending.
  - **Temporary:** no connection, timeout, `5xx`, `429`, `401`, `403`. These are retried the way a score is retried
    today (`401`/`403` show as a login problem).
  - Swapping a forfeited game is accepted.

## Constraints

- Heavy process (`.claude/rules/plan-execution.md`): the schedule type and the portal client live in `uwh-common`
  (shared by every crate), and the change touches the next-game decision in the game engine's surroundings.
- `uwh-common` must still compile without the standard library where it does today.
- MSRV 1.85; every change to the refbox's English text needs every translation file updated (`just check` fails
  on an English-only key).
- The website refbox copies this behaviour; nothing here may depend on desktop-only facts the portal cannot carry.

## Left to the builder (with the traps that apply)

- **Where the skip lives.** The next game is decided in more than one place: at kickoff (looking ahead to the
  next game), on every schedule receipt between games, on start-up restore of a remembered game, and in the
  catch-up schedule the engine uses to judge how far behind the court is. Every one must skip a forfeited game.
  Guard the class (for example, one "next playable game on this court" rule in `uwh-common`), not each caller.
- **The held next game must yield to a forfeit** (found by the builder check). Today, when a schedule arrives
  between games, the game the engine already holds always wins (`refbox/src/app/mod.rs` `next_game_from_schedule`,
  rule 2), and the engine sets it automatically at the previous kickoff. Left alone, a forfeit learned at game end
  or on REFRESH would never be skipped. Required: an AUTOMATICALLY chosen next game that is now forfeited is
  replaced by the next playable game; a game the OPERATOR picked by hand is kept (R3's "picked by hand" edge). The
  engine does not record which it was today; the builder adds that record.
- **The swap's local record.** The refbox must keep showing its own swap even if the end-of-game schedule download
  comes back before the portal has applied it (offline queue). How it remembers that is the builder's choice.
- **Swap with a side not yet a real team** (a "Winner of game 9" placeholder still unresolved at kickoff): the
  message shape is the agreement's choice; the refbox must never invent a team.
- **Tests:** a test that injects a message into the app never reaches the network, and a test cannot see the
  background job an update returns. Pin the rules (next playable game; the swap message's content; queue
  behaviour on refusal) in plain functions and test those.

## Not doing

- An automatic schedule re-read on a timer (R2 declined it).
- A Swap button anywhere other than the score edit page (R5); no swap between games on the refbox (R6).
- Recording or undoing a forfeit from the refbox.
- Any LED-panel or wireless-remote change; the website refbox (portal repo); the portal's server pieces.
- Sending team names from the refbox to the overlays (R8 declined it); any change to the overlay bridge.
