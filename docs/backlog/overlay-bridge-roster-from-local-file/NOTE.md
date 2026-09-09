# Backlog: let the overlay bridge take rosters from a local file instead of the Portal

**Surfaced:** 2026-08-27, while walking the rebuilt bridge status page.
**Raised by:** the user — *"allow for an alternate source for the roster information than the UWH
Portal, likely a toggle between using UWH Portal or a file picker for a local CSV file that we can
pull the player names and info from"*.

**Status: not started.** Own branch, own Scope Card.

## The idea

The bridge currently gets player names from the UWH Portal and nowhere else. Add a second source:
the operator gives the bridge a path to a local CSV file, and names come from that instead. A
toggle on the status page chooses which source is in use.

## Why it is worth doing

Player names are the one thing the bridge cannot produce on its own. The refbox feed carries **cap
numbers only** — it has never carried names — so every name on the broadcast comes from a Portal
lookup over the internet. That makes the one cosmetic-but-visible part of the overlay depend on:

- the venue having working internet at all;
- the event being in the Portal, with rosters filled in;
- the Portal being up at that moment.

At a small or informal event, any of those can be false while the tournament runs perfectly well.
Today the result is a broadcast where penalties and fouls show a number and the word "Player"
instead of a name, with nothing the operator can do about it from the poolside.

A local file also covers the case where the roster is simply *wrong* in the Portal and there is no
time to fix it centrally before the next game.

## What the names are used for — and one thing that changes the shape of this

Today, names appear in **three** tables and never in the scorebug: `/penalties`, `/fouls` and
`/warnings` each carry a `player` column (`tables.rs`,
`EVENT_COLUMNS = ["team", "number", "player", "infraction"]`). Only cap numbers **that appear in
the current game's penalties, fouls or warnings** are ever looked up — see `server.rs`'s
`cap_numbers_for` / `roster_for`. The bridge never walks all 256 possible numbers.

**But the names and numbers must also appear in the pre-game roster reveal** (user, 2026-08-27).

**It already exists — in the `overlay` crate, not the bridge.** `overlay/src/pages/roster/` draws
it, and `overlay/src/main.rs` shows it between games, in the 30–181 second window of the countdown.
There are two forms and the crate picks between them on its own: `list.rs` renders `#number` and
name, while `picture.rs` renders player photos, and `picture.rs` falls back to the list whenever no
member of the team has a picture. Photos come from the Portal roster's `photos.uniform` and a
team-colour-keyed geared photo (`overlay/src/network.rs`).

The bridge serves none of this — there is no roster table — so vMix cannot reproduce the reveal
today whatever the roster source is. That matters here because the reveal changes the data
requirement completely:

- the three existing tables need only the handful of players who picked up a penalty, foul or
  warning;
- a roster reveal needs **every player on both teams**, before the game has started and before
  anyone has done anything.

So a file source must carry complete rosters, not just the names likely to be needed, and the
bridge will need a new table to serve them.

**Confirmed wanted** (user, 2026-08-27): *"we will definitely want to have this roster reveal and
picture with names as part of our overlay project"*. So this is not an open question — the reveal
is in. That makes two deliverables sharing one data model:

1. **A roster table the bridge serves**, so a vMix title can reproduce the reveal the `overlay`
   crate already draws.
2. **A second source for the roster behind it**, so the names need not come from the Portal.

They could be built separately, and the first is useful on its own — a Portal-sourced reveal in
vMix needs no CSV at all. Whoever picks this up should decide the order. What ties them together is
the roster representation itself, which both depend on, so that is worth settling once rather than
twice.

**Photos are the part a CSV cannot supply**, and that resolves itself neatly: `picture.rs` already
falls back to the list form when no member has a picture, so an event whose rosters come from a CSV
lands in the existing no-photos path rather than a new one. Worth confirming the same fallback
exists for whatever the bridge ends up serving, so a vMix reveal degrades the same way the overlay
already does rather than rendering empty frames.

**Leave room for photos later, but do not build them** (user, 2026-08-27): *"it is forseeable that
we may also provide pictures down the road in some sort of folder with a naming convention, but
that will not be part of this initial project"*. So a CSV-sourced player has no picture **for now**,
not by definition. The practical consequence is small but real: model a player's photo as *absent*
rather than *impossible*, so adding a folder-and-naming-convention source later is a new way to
populate an existing field rather than a change to the roster model and every table built on it.
Do not add the folder, a path setting, or a naming convention as part of this work.

## What the Portal path looks like today

The replacement has to slot in beside this, in `portal.rs`:

- `Directory::team_ids_for(game_number)` → the two `TeamId`s for a game;
- `Directory::refresh_roster(&TeamId)` → fetches and caches that team's roster;
- `Directory::player_name(&TeamId, cap_number)` → the lookup the tables use.

`player_name` returning `None` is already a supported, tested path — the row renders with the
number present and the name blank/placeholder. **A file source that has no entry for a number must
behave identically**, not invent anything.

## Things that will bite

- **The Portal's roster field names are not the obvious ones.** They are `capNumber`,
  `rosterName` and `roles` — not `number`/`name`/`role`. This was found by capturing a real
  response, after a plan had been written against the guessed names. Any importer written for a
  file format should be checked against a real exported file for the same reason, not against an
  assumed shape.
- **A file has no team ids, and there is already a way to deal with that.** The whole Portal
  path is keyed by `TeamId`, which comes from the schedule; a file will be keyed by a team *name*
  an organiser typed. `schedule-processor` solves the same problem already: `get_best_match` in
  `schedule-processor/src/main.rs` scores every typed name against every event team with
  `strsim::normalized_levenshtein` and pairs off the best match, so a name that is close but not
  identical still lands on the right team. `strsim` is already a workspace dependency, so reusing
  the approach costs nothing new.

  Two caveats before copying it wholesale. It is a *greedy* pairing — best pair first, then the
  next from what is left — which is fine for a whole event mapped once, and it never asks a human
  to confirm. In `schedule-processor` a wrong pairing is caught by someone reviewing the output
  before a tournament; here it would surface live, as the wrong team's names on air. Worth showing
  the operator what matched what on the status page, rather than matching silently.
- **Two teams per game, and the game changes.** Whatever the format, it has to cover a whole
  tournament's teams, or be re-picked every game. Re-picking every game is not acceptable at a
  poolside; assume one file for the event.
- **"Player" placeholder stays.** Settled 2026-08-26: when no name is known the tables show the
  placeholder, and that is not up for renegotiation as part of this.
- **Roles are deliberately ignored.** The Portal roster carries `roles`, and the bridge does not
  filter on them, because overlays are only used at events that enforce unique cap numbers. A file
  importer should not reintroduce role handling without a reason.
- **CSV, settled 2026-08-27, and for a good reason.** The user: *"we will be asking event
  organizers to generate the file, a csv is more reachable than a json for many"*. The file is
  authored by a volunteer organiser, not exported by a machine, so the format has to be one they
  can produce in a spreadsheet. That outweighs the neatness of reusing the Portal's JSON shape.

  **Use the field names already in use** (user, same message): `capNumber` and `rosterName`, as
  the Portal's own roster carries them, rather than inventing a second vocabulary for the same
  two things. One name per concept across the bridge, the Portal and the file.

## Scope when picked up

Own branch. Touches `portal.rs` (or a new module beside it), the status page for the toggle and
the path field, and `config` to remember the chosen path between runs — it should be remembered,
on the same "remembered automatically for next time" rule the refbox address follows.

**This does not change what the overlay displays** — user's ruling, 2026-08-27:
*"just where it gets the information to display"*. The overlay shows a player's name either way;
the bridge is relaying an operator-supplied roster from a different place, not deriving,
interpreting or inventing a value. The standing rule is not in play here.

The roster-reveal table above is the part that genuinely adds something new to what is served, and
that is worth its own decision.

**Settled 2026-08-27:** a typed or pasted **file path** is fine — no upload. That avoids
multipart form handling in the bridge entirely, and means the file can be edited in place between
games without re-picking it. It also means the path is a setting worth remembering between runs,
like the refbox address.

---

# 2026-09-09: a second case — no Portal at all

**Surfaced:** 2026-09-09, evaluating a prospective client in China who cannot reach
`uwhportal.com` without a VPN.
**Status: not started.**

**This is a bigger piece of work than the sections above, not a variant of them.** It needs a
second half to the file format, a new column on a published vMix table, and a release path for a
crate that has never been released. Scope it on its own terms rather than inheriting the scope
statement above.

## How this case arose, and what was rejected

Two other routes were evaluated on 2026-09-09.

- **refbox reading a spreadsheet directly.** Rejected: it solves only the inbound half, does
  nothing for the bridge (which asks a web address a question and cannot read a file on the
  scorer's laptop), and adds a fourth game source to maintain beside Manual, Portal and Custom.
- **A stand-in "custom site" on the local network** — the route `docs/third-party-integration.md`
  documents and `docs/third-party-stub/stub_site.py` demonstrates.

Eric's first statement was conditional — *"**if** we are not going to be calculating the team
points and doing all the scheduling logic that UWH Portal currently does, then we will **likely**
need to step back from this custom site approach"* — and he then settled it in the same exchange:
*"we will just send the time/state/penalties/etc. to the overlay and then the customer can do their
own vMix work"*, and *"we will just be shipping the vMix bridge as a standalone (doesn't even use
our own overlay graphics)"*.

**Be precise about why, because the obvious reading is wrong.** A stand-in site is *not* forced to
compute standings in order to run games — `docs/third-party-integration.md` says `groups` is "not
needed to run a game — a stub can always send `[]`", and the same is true of `standingsOrder` and
`finalResultsOrder`. Nothing in refbox reads them either (every reference in `refbox/` and
`overlay*/` is an empty test fixture). The custom site was dropped because the *client* wanted
playoff and finals calculation displayed, which the Portal does and a stub would have had to
reimplement — not because a stub cannot serve games. Note that the chosen route does not supply
that calculation either; it was traded away, not solved.

**What was chosen instead:** refbox runs in **Manual mode**, and the bridge is shipped standalone.
The customer builds their own vMix graphics — our `overlay` crate is not involved, and neither is
the roster reveal it draws.

## Verified in code, 2026-09-09

Read in the source on that date, not inferred.

- **With no Portal, the bridge builds no `Directory`.** `game_feed.rs` derives `event_id` and
  `portal_base_url` from the snapshot as *both or neither*; with neither, no directory is built.
  Deliberate — see the `portal.rs` comment on the 2026-08-26 wrong-tournament incident.
- **What still serves:** clock, period, both scores, timeouts, and the penalty / foul / warning
  rows. Each of those rows carries `team` (`"BLACK"`/`"WHITE"`), `number` — the cap number,
  documented as "the player's cap number, as the refbox reports it", present regardless of any
  roster — and `infraction`.
- **What goes blank:** `player` on every row, `blackTeam`/`whiteTeam` on `/scorebug`, and **the
  whole of `/nextgame` except `connected`** — `tables::next_game` reads `court` and `startTime`
  off the same directory-supplied value as the two team names, so budget that endpoint as empty,
  not partial.
- **A blank name is the existing, supported path, and it is blank — not a placeholder.**
  `tables.rs` states "cap numbers with no name never become a placeholder" and `game_feed.rs`
  pins "an unknown cap number serves null, never a placeholder". **This corrects the "'Player'
  placeholder stays" line earlier in this note**: that placeholder lives in
  `overlay/src/network.rs`, a different crate, which this case does not use. A file source with no
  entry for a number must serve blank, matching the bridge.
- **`enqueue_game_end` does not run in Manual mode.** Its only call site is inside
  `if self.uses_remote() {` in `refbox/src/app/mod.rs`, and `uses_remote()` is
  `!matches!(self.source, GameSource::Manual)`. So Manual mode queues nothing and posts nothing.
  (An earlier draft of this note asserted the opposite as a confirmed fact. It was wrong.)

## The game number: it does come from refbox, with one gap to know about

The operator's typed number reaches the bridge. Both manual apply paths in
`refbox/src/app/mod.rs` call `tm.set_next_game(NextGameInfo { number: edited.game_number, .. })`,
and `TournamentManager::next_game_number()` returns that stored number in preference to anything
else. `GameSnapshot::game_number()` reports `next_game_number` throughout the between-games window,
which is where the bridge reads it. So a number the operator applied is what goes out.

**The gap:** starting a game consumes it — `self.next_game.take()` in
`refbox/src/tournament_manager/mod.rs`. From then until the operator applies a new number,
`next_game_number()` falls back to arithmetic: `game_number.parse::<u32>() + 1`.

For sequential numbering that fallback is correct and needs no operator action. It is wrong only
when the schedule skips — finish game 7, next game is 12, operator has not applied it — and then
the bridge reports `8` for the whole pre-game window and the file lookup returns **game 8's teams,
on air, with nothing failing to say so**.

So the operator step is: **apply the next game's number between games whenever the sequence is not
+1.** That belongs in the customer's operating instructions and in any walkthrough of this feature.
Worth considering whether the bridge's status page should show the game number it is currently
serving, so a wrong one is visible to the operator before it is visible to the audience.

**Integer game numbers only — Eric's ruling, 2026-09-09: "we only need to support integer game
numbers for this approach".** This matches the Manual keypad, which parses with
`.parse().unwrap_or(0)` on entry and writes the integer back on exit, so `SF1` or `G27` cannot be
typed there anyway. Note the side effect: opening that page with a non-numeric number left over
from a previously linked session silently resets it to `0`.

## What the file has to carry

Two things, not one:

1. **Rosters**, exactly as specified above — `capNumber` and `rosterName`, one file for the event.
2. **A game number to two team names mapping.** New. With no Portal there is no schedule, so
   nothing else can turn the game number into the two names the scorebug needs.

Both halves key off the same integer game number the bridge reports.

## The `gameNumber` column — decided, but not for the reason first given

**Eric agreed on 2026-09-09 to add `gameNumber` to the `/scorebug` table.** He also ruled out the
alternative explicitly: *"I definitely do not want to have to have the operator pick the game
number, this should just come from the refbox info."*

**The original justification was wrong and should not be repeated.** It claimed the column was
needed or vMix could not tell which game was on. It is not: once the bridge holds the file's
game-to-teams mapping it can fill the `blackTeam`/`whiteTeam` columns that already exist —
`tables::scorebug` takes the names as a caller-supplied argument and does not consult a directory
itself. The column's real value is letting a title display the game number, and giving the operator
a way to see what the bridge is keying on.

Today the number is in the JSON feed (`GET /game` serves `gameNumber` and `nextGameNumber`) and in
none of the five table endpoints. Their column sets — four distinct sets across five endpoints,
since `/fouls` and `/warnings` share one — are pinned by
`the_vmix_tables_column_names_are_frozen` in `overlay-bridge/src/tables.rs`:

| Endpoint | Columns |
|---|---|
| `/scorebug` | `blackFouls`, `blackScore`, `blackTeam`, `blackWarnings`, `clock`, `clockSeconds`, `connected`, `equalFouls`, `period`, `timeout`, `timeoutClock`, `timeoutClockSeconds`, `whiteFouls`, `whiteScore`, `whiteTeam`, `whiteWarnings` |
| `/nextgame` | `blackTeam`, `connected`, `court`, `startTime`, `whiteTeam` |
| `/penalties` | `connected`, `infraction`, `number`, `player`, `team`, `time`, `timeSeconds` |
| `/fouls`, `/warnings` | `connected`, `infraction`, `number`, `player`, `team` |

**Adding a column is not free.** `docs/superpowers/specs/2026-08-26-vmix-integration-steps.md`
records, marked verified on a live run, that rows are `BTreeMap`s so **columns serve in
alphabetical order**, and that vMix's `Column: Auto` matches by name and then **falls back to
matching by position**. Inserting `gameNumber` lands it between `equalFouls` and `period` and
shifts eight columns down one, so any existing title bound positionally starts reading the wrong
field. That spec's standing advice — *"Always set Column explicitly"* — is the mitigation, and it
should be repeated to the customer rather than assumed.

The frozen-columns test exists precisely to catch a column added silently; its own comment records
that an earlier version could not fail. Updating it is a deliberate step.

## A consequence for the fuzzy team-name matching above

"Things that will bite" proposes reusing `schedule-processor`'s `get_best_match` / `strsim` pairing
to reconcile organiser-typed names against the event's real teams.

**That applies only to the Portal-present case.** With no Portal there are no event teams to match
*against*, and no `TeamId` exists anywhere — the file is the authority rather than something
reconciled to a Portal record. Whoever builds this must not assume a `TeamId` is available; the
roster has to be keyable by whatever identity the file itself carries.

This is the main structural difference between the two cases, and it is worth settling in the data
model before either is built, since both feed the same tables.

## Shipping the bridge standalone

- **`overlay-bridge` is in no *release* workflow.** `.github/workflows/release.yml` builds `refbox`
  (Windows, macOS arm, macOS x86, Raspberry Pi) and `overlay` (Raspberry Pi only). It is otherwise
  fully covered by CI — it is a workspace member, and `rust.yml` builds, clippy-lints and tests it
  on every push. Only the release path is missing.
- **vMix is Windows-only**, so this needs a Windows build. **Eric ruled 2026-09-09 that Windows
  only is fine for now** — his words: *"windows only build is fine for now"*. That also keeps it
  clear of the unsigned-macOS problem currently blocked on the Apple Developer ID.
- **Ship it as extra assets on refbox's existing `v*.*.*` release, the shape `overlay` already
  uses. Do not give it its own tag pattern.** Verified 2026-09-09: `Version::parse` in
  `refbox/src/updater/version.rs` accepts `X.Y.Z` with an optional leading `v` and "returns None on
  anything else", and `release.rs` reads the forge's `/releases/latest` and its `tag_name`. That
  endpoint returns the newest non-draft, non-prerelease release regardless of tag shape, so a
  `bridge-v1.0.0` release published after a refbox one claims it, and every deployed Pi stops
  finding refbox updates.

  **Two corrections to how this hazard is usually stated.** It is not *silent*: an unparseable tag
  becomes `UpdateError::BadVersion`, surfacing as "The downloaded update wasn't valid and was not
  installed" — a wrong-but-visible error that misleadingly blames the download. And
  `make_latest: false` is not the only mitigation; `prerelease: true` also keeps a release off that
  endpoint. The recommendation stands, because extra assets remove the hazard instead of guarding
  it with a flag that does not look load-bearing — but do not argue it from "silent" or "only".
- The pattern to copy is in `.github/workflows/release.yml` itself: the `build-overlay-rpi` job,
  the overlay staging steps in `upload-release`, and the comment at the top of that job explaining
  why the overlay is kept out of `refbox.zip`. `docs/release-checklist.md` records what the release
  is then expected to contain, and `docs/workspace-map.md` describes the crate.
- A **zip is what preserves the executable bit** — loose release assets are served with no Unix
  mode. Irrelevant for a Windows `.exe`, but it matters if a Linux build is ever added.

## Still out of scope here

The roster reveal table described earlier in this note is still wanted, but not in this case — the
customer is building their own graphics and not using our `overlay`. Photos remain out, on the same
terms as above. Score push-back was deferred by Eric on 2026-09-09: *"Reporting the scores is a
separate issue and maybe will be included in version 2"*.
