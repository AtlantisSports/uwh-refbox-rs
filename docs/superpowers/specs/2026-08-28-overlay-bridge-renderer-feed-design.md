# The bridge supplies game information to the NDI renderer — design

**Status:** Approved by Eric, 2026-08-28. Extends
`2026-08-26-vmix-overlay-bridge-design.md` (§4.7, §4.8). **Supersedes that document's §5.7** — see
"What this drops" below.

## 1. What this is

§4.7 decided that the NDI renderer stops connecting to a refbox and consumes this bridge over HTTP,
so the bridge becomes the only thing in the system that talks to a refbox. This document specifies
the bridge side of that, and only the bridge side.

**Scope boundary, set by Eric verbatim, 2026-08-28:** *"we don't need to touch on any of the
rendering stuff, that is what my colleague has been working on, all we need to do is supply the
information."*

So this design covers **what the bridge serves**. It does not decide, describe, or constrain what
the renderer draws, when it draws it, how it handles a version it does not recognise, its NDI
transport, its transparency, its packaging, or its layout. All of that belongs to the parallel
effort.

## 2. What this drops

**§5.7 ("Goals are served as a sequenced list, not a single slot") is superseded and will not be
built.** It proposed the bridge add goal identity — a list of the last N goals with monotonically
increasing ids — so a consumer could tell two goals by the same player apart inside the refbox's
15-second retention window, fixing a real defect where the current overlay draws one marker for two
goals.

That fix changes what a viewer sees, and Eric's instruction was *"at this point I do not want to
display anything differently than the current overlay."* It is also, on the boundary above, the
renderer's half rather than ours. So the bridge passes goal information through exactly as the
refbox sends it and adds nothing.

The defect §5.7 documented is real and is **not fixed by this work**. It remains available if the
renderer effort ever wants it, and §5.7's analysis — including its measured 15-second retention and
the one accepted false-positive case — stays valid as written.

## 3. A new endpoint, not new columns on the existing five

The bridge serves the renderer at **`GET /game`**. The five vMix tables (`/scorebug`, `/penalties`,
`/fouls`, `/warnings`, `/nextgame`) are not touched.

**This is a safety constraint, not a preference.** Each table row is a `BTreeMap`, chosen
deliberately so columns always serialise in a stable order (`tables.rs:6-7`). That order is
alphabetical, which was confirmed by running the stack (commit `2f728fca`): a vMix title left on
positional fallback received `blackFouls` where `blackTeam` was intended. Any column added to an
existing table inserts itself into that ordering and silently repoints every positionally-bound
title downstream of it. §4.7 already establishes that a vMix title has no logic and therefore no
error path, so the failure is wrong values on air with nothing reported anywhere.

A separate endpoint cannot do that to them. It also lets the renderer receive real numbers rather
than the display strings the vMix tables carry for vMix's benefit, so it never parses text back
into values, and it decouples the renderer from vMix's frozen text formats in both directions.

**Naming:** `/game` describes what it carries, not who reads it. Naming it after the renderer would
age badly the moment a second typed consumer appears.

## 4. What `/game` serves

A single JSON object (not an array — there is no table shape to preserve here, and no positional
consumer to protect). **The types below are the connected case.** When the refbox is not
connected every one of them is `null` instead, by §7's rule; `schemaVersion` and `connected` are
the two exceptions and are always real values.

| Key | Type | Source |
|-----|------|--------|
| `schemaVersion` | integer | This document, §6 |
| `connected` | boolean | `feed::Connection`, via `server::is_connected` |
| `period` | string | `snapshot.current_period` |
| `secsInPeriod` | integer | `snapshot.secs_in_period` |
| `blackScore` / `whiteScore` | integer | `snapshot.scores` |
| `blackTeam` / `whiteTeam` | string or null | resolved by the bridge, §5 |
| `timeout` | object or null | `snapshot.timeout` — `{ "kind": string, "secsRemaining": integer }` |
| `gameNumber` | string | `snapshot.game_number()` |
| `nextGameNumber` | string or null | `snapshot.next_game_number()`, which is itself an `Option` |
| `isOldGame` | boolean | `snapshot.is_old_game` |
| `recentGoal` | object or null | `snapshot.recent_goal` — `{ "team": "BLACK"\|"WHITE", "player": integer }` |
| `nextPeriodLenSecs` | integer or null | `snapshot.next_period_len_secs` |
| `penalties` | array | `snapshot.penalties` — §4a |
| `eventId` | string or null | `snapshot.event_id` — §4b |
| `portalBaseUrl` | string or null | `snapshot.portal_base_url` — §4b |

**`period` and `timeout.kind` carry the same strings the vMix tables already serve** — `period` is
`snapshot.current_period.to_string()`, exactly as the `/scorebug` `period` column, and `timeout.kind`
is `tables::timeout_label`'s output. Deliberately not a second, machine-flavoured vocabulary: two
names for the same concept, differing only by consumer, is worse than one imperfect name. The cost
is that both are `Display` implementations written for people, so **renaming a period or timeout
label for UI reasons is a meaning change and requires a §6 version bump** even though no field moved.
`timeout.secsRemaining` is the timeout's own countdown, not `secsInPeriod`.

**Why this list.** It is every `GameSnapshot` field the current overlay reads, derived from the
overlay's own source rather than guessed at the renderer's design: `event_id`, `game_number`,
`secs_in_period`, `scores`, `current_period`, `timeout`, `next_game_number`, `is_old_game`,
`recent_goal`, `penalties`, `next_period_len_secs`.

Nothing on that list is withheld. Every field is served, including `penalties` (§4a) and the
`event_id` / `portal_base_url` pair (§4b).

### 4a. Penalties

An array, one object per penalty, mirroring the columns `tables::penalty_row` already produces so
the two consumers describe a penalty the same way:

| Key | Type | Source |
|-----|------|--------|
| `team` | `"BLACK"` or `"WHITE"` | which bundle the penalty came from |
| `number` | integer | `penalty.player_number` |
| `player` | string or null | resolved from the roster; `null` when that number is not on it |
| `secsRemaining` | integer or null | `PenaltyTime::Seconds`; `null` for a total dismissal |
| `totalDismissal` | boolean | `true` for `PenaltyTime::TotalDismissal` |
| `infraction` | string | `penalty.infraction.short_name()`, as the vMix table serves it |

`secsRemaining` and `totalDismissal` replace the vMix table's `time` / `timeSeconds` pair, which
encodes a dismissal as the literal string `"TD"` with an empty seconds column. A typed consumer
should not have to recognise `"TD"`, and the two fields together carry the same information without
loss.

**Ordered exactly as `tables::penalties` orders it**, so both consumers agree on which penalty is
first.

**Two deliberate differences from the vMix table, both giving the renderer more than vMix gets:**

- **Not padded.** The vMix table pads up to `PENALTY_ROWS` (10) with blank rows because a vMix title
  needs a fixed number of rows to bind to. An array has no such need, so an empty list means no
  penalties.
- **Not truncated.** The vMix table takes only the first 10. The array carries every penalty the
  snapshot holds — the same reasoning `/scorebug` already applies to its foul counts, which are
  documented as the true untruncated totals rather than whatever the matching table shows.

### 4b. The event id and the portal address travel together

Both are served, and **a consumer resolving anything itself must use `portalBaseUrl` and never an
address of its own.**

This is the resolved form of the wrong-tournament bug, not an open exposure to it. Event ids are not
unique across portal environments — `1889-B` is one tournament on the development portal and a
different one on production — so an id resolved against the wrong portal returns real names for the
wrong event with no error anywhere (`uwh-common/src/game_snapshot.rs:62-65`). Carrying the refbox's
own portal address on the snapshot is what fixed it. Serving the id *without* the address would be
the genuinely unsafe combination, because it hands over an identifier with no way to know which
portal it belongs to.

**Both or neither.** The bridge's own code already treats them as a unit — `server.rs:198-201`
returns them only as a pair — and `/game` follows that: if either is absent, both are served as
`null`.

**`portalBaseUrl` is served with any embedded credentials removed.** `base_url` is a plain `String`
normalised only by trimming a trailing slash (`uwh-common/src/uwhportal/mod.rs:179`); nothing
anywhere strips a `user:password@` prefix. That has not mattered so far because the value was only
ever used to build requests, but `/game` is readable by anything on the network, so serving the raw
string would newly expose a credential an operator had typed into a custom site address. Serve
scheme, host and path only. This is a bridge-side redaction; it does not change what the refbox
sends.

## 5. Names are resolved by the bridge

`blackTeam` / `whiteTeam` carry names the bridge resolved, by the same path the vMix `/scorebug`
already uses (`portal::Directory::names_for`, via `server::names_for_game`). `null` when nothing has
been resolved yet — never a placeholder, and never an id for the consumer to resolve itself.

This is §4.7's decision restated in serving terms rather than a new one. It is what removes the
wrong-tournament bug: event ids are not unique across portal environments, so the same id looked up
against the wrong portal returns real names for the wrong event with no error anywhere
(`uwh-common/src/game_snapshot.rs:62-65`).

## 6. The version stamp

`schemaVersion` is a single integer, starting at **1**.

**It is bumped only when a field is removed, renamed, or changes meaning. Adding a field never
bumps it.** Eric's decision, 2026-08-28.

The reason is operational: bumping it is expected to stop a consumer, so bumping it for a change
that could not have broken anything would take a graphic off a live stream for no reason. This
matches how the refbox's own wire format already behaves on purpose — `game_snapshot.rs` carries
two deliberate tests proving an unknown extra field and an absent field are both tolerated
(`:1122`, `:1143`).

What the renderer does on a version it does not recognise is the renderer's decision, per §1. The
bridge's only obligation is to state the version truthfully.

## 7. The disconnect rule

When the refbox is not connected, **every game value is `null` and `connected` is `false`**. Keys
are always present either way. `schemaVersion` is not a game value and is **never** nulled: a
consumer must be able to check the contract it is reading against while the refbox is away,
which is exactly when it would otherwise have nothing to check.

- **Keys stay present** for the reason `tables.rs:486` already gives for the vMix tables: to a JSON
  consumer an absent key and an empty value are different things, and only one of them is safe to
  read blind.
- **`null`, never a zero or an empty string.** A disconnected score served as `0` is a plausible
  value rather than a blank, and plausible values invented during an outage are precisely what
  produced the phantom 0-0 result bug. `null` cannot be mistaken for a real reading.
- **`connected` is the only field that answers whether the refbox is alive**, per §4.8 R2. This
  matters because `recentGoal`, `timeout` and `nextGameNumber` are legitimately `null` during normal
  play; `connected` is what distinguishes "nothing to report" from "nobody is reporting".
- **Never inferred from timing**, per §4.8 R3. The value comes from `feed::Connection` and nothing
  else. The refbox goes silent for ~25 seconds whenever the clock is stopped, so any timing-derived
  liveness rule would blank the graphic at every stoppage.

**This is a deliberate, Eric-approved exception to "nothing displays differently."** Eric chose it
knowingly over holding the last received values (2026-08-28). Today the overlay has no disconnected
case at all (`overlay/src/main.rs:281`) and freezes on its last frame indefinitely, so a stream can
sit showing a stale score from a finished game. Serving `null` makes that impossible to reproduce
through this path. **Do not "restore" the frozen behaviour as a regression fix — it is not one.**

## 8. How this is proved

- **Every field in §4's table is present**, with the right type, for a connected snapshot.
- **Every game value is `null` and `connected` is `false`** when disconnected, with no key missing.
  This must be asserted per key, not by counting keys.
- **`recentGoal` reproduces `snapshot.recent_goal` exactly**, including that a repeat goal by the
  same player inside the retention window is indistinguishable — the §5.7 defect is *preserved*
  here, deliberately, and a test that expected it fixed would be wrong.
- **The five vMix tables' column names are unchanged.** A test pinning their exact column sets,
  which is worth having independently of this work: it is the only thing that would catch the
  alphabetical-shift hazard of §3 being reintroduced by a later change.
- **Penalties are neither padded nor truncated** (§4a): a snapshot with no penalties serves an empty
  array, and one with more than ten serves all of them. Both need asserting, because the vMix
  behaviour this deliberately differs from is what a reader would otherwise assume.
- **A total dismissal serves `totalDismissal: true` and `secsRemaining: null`** — never the string
  `"TD"`, and never `0`, which would read as a penalty about to expire.
- **The event id and portal address are both-or-neither** (§4b), and the served address carries no
  `user:password@` prefix even when the refbox reports one.

## 9. Out of scope

- Everything the renderer does with this: drawing, timing, transparency, NDI, layout, packaging,
  and its response to an unrecognised version.
- Goal identity and sequencing (§2).
- Serving `fouls` or `warnings` in typed form. `/scorebug` already carries all three counts and the
  vMix tables carry the detail; adding typed lists is additive rather than breaking under §6, so it
  can follow if the renderer effort wants it. `penalties` **is** in scope — see §4a.
- Any change to the five vMix tables, to `refbox`, or to `uwh-common`. This work is confined to
  `overlay-bridge/`.
- A vMix-facing goal callout. vMix cannot consume a sequenced list, and a latched one is a different
  feature needing its own design.

## 10. Open, and not blocking

The field list in §4 is derived from what the current overlay reads, not from the renderer effort's
own requirements, so it is worth confirming with Eric's colleague. It is not a gate: §6's
additive-safe rule means anything missing is a one-line addition rather than a renegotiation.

Nothing is deliberately withheld from that list any more. Eric's instruction, 2026-08-28, was that
penalties be included and that guessing was unnecessary because the information already exists —
which was right: `tables::penalty_row` had already settled every field and the roster lookup, so
§4a mirrors it rather than inventing a shape.
