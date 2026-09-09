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
**Status: not started**, like everything above.

This section adds a *case*, not a second project. The local-file source described above is still
the deliverable. What changes is that the Portal is not there at all underneath it — which changes
what the file has to contain, and exposes one thing the bridge does not currently serve.

## How this case arose, and what was rejected

Two other routes were evaluated on 2026-09-09 and dropped.

- **refbox reading a spreadsheet directly.** It solves only the inbound half. It does nothing for
  the bridge, which asks a web address a question and cannot read a file on the scorer's laptop,
  and it adds a fourth game source to maintain beside Manual, Portal and Custom.
- **A stand-in "custom site" on the local network** — the route `docs/third-party-integration.md`
  documents and `docs/third-party-stub/stub_site.py` demonstrates. Technically sound and nearly
  free for us, since the contract document and a working example site already exist. Dropped on
  Eric's ruling: *"if we are not going to be calculating the team points and doing all the
  scheduling logic that UWH Portal currently does, then we will likely need to step back from this
  custom site approach."* Playoff and standings calculation turned out to be the real requirement,
  and a stand-in site would have had to reimplement the Portal's tournament engine to meet it.

**What was chosen instead:** refbox runs in **Manual mode**, and the bridge is shipped standalone.
The customer builds their own vMix graphics — our `overlay` crate is not involved, and neither is
the roster reveal it draws.

## Verified in code, 2026-09-09

Everything in this subsection was read in the source on the date given, not inferred.

- **With no Portal, the bridge builds no `Directory` at all.** `game_feed.rs` derives `event_id`
  and `portal_base_url` from the snapshot as *both or neither*; with neither, no directory is
  constructed and every name stays blank. This is deliberate — see the comment in `portal.rs`
  about the 2026-08-26 wrong-tournament incident.
- **Everything that is not a name still serves correctly.** Clock, period, both scores, timeouts,
  and the penalties / fouls / warnings tables. Each of those rows carries `team` (`"BLACK"` /
  `"WHITE"`), `number` — the cap number, documented in `game_feed.rs` as "the player's cap number,
  as the refbox reports it", present regardless of any roster — and `infraction`.
- **`player` being blank is the existing, supported path.** `Penalty.player` is an `Option<String>`
  and is `None` when the cap number is not on a roster. A file source that has no entry for a
  number must behave identically, exactly as the Portal section above already requires.
- **`blackTeam` / `whiteTeam` are blank in this case**, because they come from the same schedule
  lookup the directory provides.
- **The operator can set the game number by hand in Manual mode.** In
  `refbox/src/app/view_builders/configuration.rs`, when no remote source is in use the GAME button
  opens `KeypadPage::GameNumber` instead of a schedule picker, and the button's label is simply the
  typed number. So a game-number key genuinely exists at the refbox end with no Portal present.

## The gap: the game number reaches no vMix table

The game number **is** in the bridge's JSON feed (`GET /game` serves `gameNumber` and
`nextGameNumber`). It is in **none of the four vMix tables**. Their columns are pinned by
`the_vmix_tables_column_names_are_frozen` in `overlay-bridge/src/tables.rs`:

| Table | Columns |
|---|---|
| `/scorebug` | `blackFouls`, `blackScore`, `blackTeam`, `blackWarnings`, `clock`, `clockSeconds`, `connected`, `equalFouls`, `period`, `timeout`, `timeoutClock`, `timeoutClockSeconds`, `whiteFouls`, `whiteScore`, `whiteTeam`, `whiteWarnings` |
| `/nextgame` | `blackTeam`, `connected`, `court`, `startTime`, `whiteTeam` |
| `/penalties` | `connected`, `infraction`, `number`, `player`, `team`, `time`, `timeSeconds` |
| `/fouls`, `/warnings` | `connected`, `infraction`, `number`, `player`, `team` |

Without it there is **nothing for a vMix title to key on**: the operator would have to hand-pick
the two teams for every game, because nothing in the table feed says which game is being played.

**Eric agreed on 2026-09-09 to add `gameNumber` to the `/scorebug` table.** It is additive, so
existing vMix setups keep working. Note that the frozen-columns test exists precisely to catch a
column added silently — its own comment records that an earlier version could not fail — so
updating it is a deliberate step, not an inconvenience to route around.

## What the file has to carry in this case

Two things, not one:

1. **Rosters**, exactly as specified above — `capNumber` and `rosterName`, one file for the event.
2. **A game number to two team names mapping.** This is new. With no Portal there is no schedule,
   so nothing else can turn the operator's typed game number into the two names the scorebug needs.

Both halves key off the same game number the operator types into refbox, which is what makes them
join up.

## A consequence for the fuzzy team-name matching above

The "Things that will bite" section proposes reusing `schedule-processor`'s `get_best_match` /
`strsim` pairing to reconcile organiser-typed team names against the event's real teams.

**That applies only to the Portal-present case.** With no Portal there are no event teams to match
*against*, and no `TeamId` exists anywhere — the file is the authority rather than something
reconciled to a Portal record. Whoever builds this must not assume a `TeamId` is available; the
roster has to be keyable by whatever identity the file itself carries.

This is the main structural difference between the two cases, and it is worth settling in the data
model before either is built, since both feed the same tables.

## Shipping the bridge standalone

- **`overlay-bridge` is in no release workflow at all.** `.github/workflows/release.yml` builds
  `refbox` (Windows, macOS arm, macOS x86, Raspberry Pi) and `overlay` (Raspberry Pi only). The
  bridge has never been built or shipped by CI.
- **vMix is Windows-only**, so this needs a Windows build. **Eric ruled 2026-09-09 that Windows
  only is fine for now.** That also keeps it clear of the unsigned-macOS problem currently blocked
  on the Apple Developer ID.
- **Ship it as extra assets on refbox's existing `v*.*.*` release, the same shape `overlay` uses.
  Do NOT give it its own tag pattern.** Verified 2026-09-09 in `refbox/src/updater/`:
  `Version::parse` accepts `X.Y.Z` with an optional leading `v` and "returns None on anything
  else", and the updater reads GitHub's `/releases/latest` and its `tag_name`. GitHub hands
  "latest" to the newest non-draft, non-prerelease release *regardless of tag shape* — so a
  `bridge-v1.0.0` release published after a refbox one claims that endpoint, fails to parse, and
  **every deployed Pi silently stops finding refbox updates**. The only mitigation is
  `make_latest: false`, a single flag in a workflow file that does not look load-bearing. Extra
  assets on the existing release remove the hazard instead of guarding it.
- A **zip is what preserves the executable bit** — GitHub serves loose assets with no Unix mode.
  Irrelevant for a Windows `.exe`, but worth knowing if a Linux build is ever added.
- See `docs/superpowers/specs/2026-08-28-overlay-ships-with-refbox-design.md` for the pattern to
  copy, and `docs/release-checklist.md` for what the release is then expected to contain.

## Stated but NOT verified — check before relying on these

- **That Manual mode posts no scores anywhere.** This is Eric's stated intent for the setup ("no
  schedule from the Portal/custom and no score pushing"), and it is what makes the Portal-less case
  coherent. It was *not* confirmed in the source. What was confirmed is that `enqueue_game_end` in
  `refbox/src/portal_manager/mod.rs` queues every finished game, and that refbox refuses to change
  site or address while anything is queued. Anyone building this should establish what Manual mode
  actually does at game end before assuming the queue stays empty.
- Whether the customer's streaming machine and the refbox are the same computer. If they are not,
  the bridge's address must be reachable from both, and `localhost` will not do.

## Still out of scope here

The roster reveal table described earlier in this note is still wanted, but it is not part of this
case — the customer is building their own graphics and not using our `overlay`. Photos remain out,
on the same terms as above. Score push-back was deferred by Eric on 2026-09-09 as "a separate
issue, maybe version 2".
