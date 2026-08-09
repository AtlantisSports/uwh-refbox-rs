# Third-Party Integration: Running Your Own Site Instead of the UWH Portal

Accurate as of refbox v0.4.9. This is a best-effort description of what the software does
today. It carries no stability promise; a future release may change any of it without notice.

## You probably need eight calls, not eighteen

The refbox application, the pre-tournament schedule tool, and the stream overlay together
make eighteen different calls to the UWH Portal. If you only want to run a site that stands
in for the Portal on the day of the tournament — the thing the referee's box actually talks
to at poolside — you only need to support eight of them:

| # | Operation | Path | Auth |
|---|---|---|---|
| 1 | Link a refbox | `POST /api/events/{eventId}/access-keys/ref-box` | none |
| 2 | Verify token | `GET /api/events/{eventId}/access-keys/verify` | bearer |
| 3 | Event list | `GET /api/events` | none |
| 4 | Event teams | `GET /api/events/{eventId}/teams` | none |
| 5 | Schedule (privileged) | `GET /api/events/{eventId}/schedule/privileged` | bearer |
| 6 | Referees | `GET /api/events/{eventId}/referees` | none |
| 7 | Push scores | `POST /api/events/{eventId}/schedule/games/{gameNumber}/scores` | bearer |
| 8 | Push stats | `POST /api/admin/events/stats` | bearer |

The other ten calls serve two separate programs that are not the refbox itself: the
pre-tournament admin tool (`schedule-processor`), which uploads and manages a schedule before
the tournament starts, and the stream overlay, which pulls attachments for the video overlay.
If you're only standing up something for the refbox to talk to during a game, you can ignore
those ten.

## Full inventory

All eighteen calls the refbox ecosystem makes to the Portal today, across all three programs.
"Auth" is "bearer" when the call requires a bearer token in the `Authorization` header, and
"none" when it does not.

| # | Method | Path | Caller(s) | Auth | Source |
|---|---|---|---|---|---|
| 1 | POST | `/api/events/{eventId}/access-keys/ref-box` | refbox | none | `uwh-common/src/uwhportal/mod.rs:206` |
| 2 | POST | `/api/authentication` | schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:264` |
| 3 | GET | `/api/events/{eventId}/access-keys/verify` | refbox | bearer | `uwh-common/src/uwhportal/mod.rs:300` |
| 4 | POST | `/api/admin/events/stats` | refbox | bearer | `uwh-common/src/uwhportal/mod.rs:325` |
| 5 | POST | `/api/events/{eventId}/schedule/games/{gameNumber}/scores` | refbox | bearer | `uwh-common/src/uwhportal/mod.rs:353` |
| 6 | GET | `/api/events/{eventId}/schedule/privileged` | refbox + schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:399` |
| 7 | GET | `/api/events/{eventId}/referees` | refbox + schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:449` |
| 8 | GET | `/api/events/{eventId}/teams` | refbox + schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:501` |
| 9 | GET | `/api/events` | refbox + schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:537` |
| 10 | POST | `/api/events/{eventSlug}/schedule` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:580` |
| 11 | POST | `/api/events/{eventSlug}/schedule/map-teams` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:614` |
| 12 | GET | `/api/events/{eventId}/schedule` | schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:647` |
| 13 | GET | `/api/admin/get-event-team` | schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:671` |
| 14 | GET | `/api/events/{eventSlug}/schedule/coin-flips` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:694` |
| 15 | GET | `/api/events/{eventId}/participants` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:726` |
| 16 | GET | `/api/admin/events/game-referees` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:772` |
| 17 | POST | `/api/events/{eventSlug}/schedule/coin-flips` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:816` |
| 18 | GET | `/api/admin/events/{eventId}/overlay-attachments` | overlay | none | `overlay/src/network.rs:174` |

These eighteen operations sit on sixteen distinct paths: the coin-flips endpoint serves both
a read and a write, and the schedule endpoint serves both a public read and an upload, each
under the same path with a different HTTP method.

The `/admin/` segment in a path is not a reliable signal for whether a call needs a token:
two `/admin/` paths need none (`get-event-team`, `overlay-attachments`) and two do
(`events/stats`, `events/game-referees`). Go by the "Auth" column, not the path.

## The refbox eight

Full detail on the eight calls a stand-in site must answer, in the same order as the table
above. Every entry uses the same headings so you can skim them side by side. Two general rules
that apply across all eight, spelled out here because they matter more than any single field:

- An event ID in a URL **path** is always the short form (`1234-A`). An event or team ID in a
  **query parameter** is always the long form (`events/1234-A` or `teams/5678-B`). Only one of
  the eight — push stats — uses the long form, and only because it's a query parameter. The
  [Data formats](#data-formats) section documents this in full; it's mentioned here because it
  affects call 8 below.
- "Fields refbox actually reads" lists only what the deserialising code in
  `uwh-common/src/uwhportal/mod.rs` actually pulls out of the response. A stand-in site can
  return an object containing only those fields (plus anything else the shape requires just to
  parse) and refbox will work correctly. Where a response shape is genuinely large (the
  schedule, the stats body), the field-by-field breakdown lives in
  [Data formats](#data-formats) instead of being repeated here.

#### 1. Link a refbox

`POST /api/events/{eventId}/access-keys/ref-box`  ·  source: `uwh-common/src/uwhportal/mod.rs:206`

**When refbox calls it:** When the operator opens the portal login screen (from Game Options →
UWH Portal, or from the portal status page's GO TO LOGIN button) and types in the numeric code
given to them by the tournament site's admin. See the worked example below — this is the one
call in the eight that's a back-and-forth conversation rather than a single request.

**Authentication:** none

**Query parameters:** none

**Request body:** `{"refBoxId": "<string>", "code": "<string>"}` — note both values are sent
as JSON strings, not numbers, even though both are made of digits. Example:
```json
{ "refBoxId": "482913", "code": "7419" }
```

**Successful response:** `200` with `{"accessKey": "<token>"}`. Example:
```json
{ "accessKey": "a1b2c3d4e5f6" }
```

**Fields refbox actually reads:** `accessKey` on success; `reason` on a `400`. Nothing else in
the response is read.

**On failure:** See the worked example below for the two specific `400` reasons refbox
recognises. Any other status code, or a `400` whose body doesn't contain a recognised `reason`,
is treated as an unexpected error: refbox logs it and leaves the operator on the code-entry
screen with no visible message — there is no third error state shown in the UI.

##### The login flow, step by step

This is the only one of the eight that's a conversation instead of a single call:

1. refbox generates a random six-digit number once, the first time it's needed, and reuses it
   for the rest of that run (`mod.rs:199`). This is the `refBoxId` — it identifies *this
   physical refbox*, not the operator or the event.
2. An admin on the tournament site enters that six-digit number into the site, which issues a
   short code.
3. The admin reads (or otherwise gives) that code to the operator, who types it into refbox.
4. refbox posts `{"refBoxId": "<six digits>", "code": "<code>"}` to call 1.
5. Success is `200` with `{"accessKey": "<token>"}`. refbox stores this token and uses it as
   the bearer token for every call marked "bearer" in the table above.
6. Failure is `400` with `{"reason": "NoPendingLink"}` (the site has no record of that
   `refBoxId` waiting to be linked — e.g. the admin never entered it, or entered a different
   number) or `{"reason": "InvalidCode"}` (the code typed into refbox doesn't match). These two
   strings must be spelled **exactly** this way — refbox matches on the literal string and
   shows a different on-screen message for each (`mod.rs:236-248`). Any other value of
   `reason`, or a `400` with no `reason` field at all, is reported as an unknown error rather
   than shown as either of the two known messages.

A custom site is free to skip this whole exchange. Since refbox only ever checks the token by
sending it as a bearer header (call 2) and never re-derives it from the login response, an
operator can type any string directly into refbox as if it were the `accessKey`, and a custom
site only has to accept that string as a valid bearer token afterwards. Nothing about calls 2–8
depends on the token having been produced by call 1.

#### 2. Verify token

`GET /api/events/{eventId}/access-keys/verify`  ·  source: `uwh-common/src/uwhportal/mod.rs:300`

**When refbox calls it:** Twice, for different reasons. First, whenever the operator opens Game
Options with a token already saved, or picks/changes the event there — refbox checks the token
before showing the portal settings as usable. Second, automatically in the background as a
standing health check, for as long as an event is selected: every 5 minutes while everything is
healthy, dropping to every 15 seconds once a problem is detected (so it notices a recovery
quickly).

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** none

**Request body:** none

**Successful response:** `200`. The body is never parsed — refbox only checks the status code,
so the body can be empty.

**Fields refbox actually reads:** none.

**On failure:** Any non-`200` response, or a request that never completes at all (no network,
DNS failure, timeout), counts as failure — but refbox tells the two apart for this call
specifically. A dropped connection ("the site is unreachable") turns the portal status
indicator red *without* asking the operator to log in again, because the saved token might
still be perfectly valid. An HTTP response that isn't `200` (a `401`, or anything else) is
treated as "the token itself is bad": the indicator goes red *and* the operator is prompted to
log in again. This distinction is unique to this call; calls 7 and 8 below treat both kinds of
failure the same way.

#### 3. Event list

`GET /api/events`  ·  source: `uwh-common/src/uwhportal/mod.rs:537`

**When refbox calls it:** At startup, if UWH Portal mode is already turned on, and again
whenever the operator turns "Use UWH Portal" on from off in Game Options.

**Authentication:** none

**Query parameters:** `limit` (always the literal string `"100"`), `filter` (`"Past"` or
`"InProgressOrUpcoming"`, from an operator setting for whether to include past events), and
`isSchedulePublished` (always `"true"` — refbox only ever asks for events whose schedule has
been published).

**Request body:** none

**Successful response:** `200` with `{"totalCount": <number>, "items": [ <event>, ... ]}`.
Example, with everything an entry needs to parse:
```json
{
  "totalCount": 1,
  "items": [
    {
      "id": "events/1234-A",
      "name": "Example Open 2026",
      "slug": "example-open-2026",
      "dateRange": { "startsOn": "2026-08-08T09:00:00Z", "endsOn": "2026-08-09T18:00:00Z" }
    }
  ]
}
```

**Fields refbox actually reads:** `totalCount` must be present as a number, but its value is
never used — `0` is fine even if `items` isn't empty. Per entry: `id` and `name` (shown in the
event picker), `slug` (must be present to parse successfully, but refbox never displays or acts
on it), and `dateRange.startsOn` / `dateRange.endsOn` (used only to sort the picker, earliest
tournament first — never displayed). `teams`, `schedule`, and `courts` may be omitted entirely:
refbox fills those in itself via calls 4 and 5, per event, right after this call returns.

**On failure:** Any non-`200` response or transport failure: refbox logs the error and the
event list stays whatever it was before (empty, on a first run). There's no retry — the
operator has to turn "Use UWH Portal" off and on again, or restart refbox, to trigger another
attempt.

#### 4. Event teams

`GET /api/events/{eventId}/teams`  ·  source: `uwh-common/src/uwhportal/mod.rs:501`

**When refbox calls it:** Automatically, once per event, immediately after call 3 returns —
refbox fetches every listed event's teams right away, not just the event the operator ends up
picking.

**Authentication:** none

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with a `teams` array. Example:
```json
{
  "teams": [
    { "team": { "id": "teams/1234-A", "name": "Black Sheep" } },
    { "team": { "id": "teams/5678-B", "name": "White Knights" } }
  ]
}
```

**Fields refbox actually reads:** The top-level `teams` array must be present (an empty array
is fine). Per entry: `team.id` (must start with `"teams/"` and have at least 3 characters after
it, or the whole call is treated as a parse failure) and `team.name`. Nothing else in an entry
is read.

**On failure:** Any non-`200` response or transport failure: refbox logs the error and that
event's team list is simply never populated (stays empty) — no retry, no visible message to the
operator unless they try to use that event's teams.

#### 5. Schedule (privileged)

`GET /api/events/{eventId}/schedule/privileged`  ·  source: `uwh-common/src/uwhportal/mod.rs:399`

**When refbox calls it:** When the operator picks an event in Game Options, right after a
successful login (call 1), when refbox restarts with a previously-linked event remembered from
last time, and whenever the operator taps REFRESH on the game-info screen.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with the full event schedule: every game (its two teams, start
time, court, and timing rule), any non-game calendar entries, and the tournament's pool/group
structure. This is one of the two large, shared response shapes — see
[Data formats](#data-formats) for the exact JSON, including which parts may be omitted.

**Fields refbox actually reads:** The whole shape is deserialised, so every field the shape
marks as required must be present (even as an empty array) — see Data formats for exactly
which. What the operator screens actually display and act on, per game: `number`, the two
teams, `startsOn`, `court`, and the timing-rule name (matched against the schedule's own list of
timing rules). Referee assignments' user IDs are matched against call 6's response to show
names, purely for display — a missing or failed call 6 doesn't block the schedule from loading.

**On failure:** Any non-`200` response or transport failure: refbox logs the error and leaves
whatever schedule it already had (if any) unchanged — nothing crashes, but the operator sees no
update. If this was triggered by the REFRESH button, the button's "Refreshing…" spinner clears
on failure just as it does on success, rather than sticking.

#### 6. Referees

`GET /api/events/{eventId}/referees`  ·  source: `uwh-common/src/uwhportal/mod.rs:449`

**When refbox calls it:** Every time call 5 (schedule) is requested — the two are fetched
together, to attach display names to the schedule's referee assignments.

**Authentication:** none

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with an object holding referee-like entries under
`tournamentReferee` (a single object or absent) and `referees.dedicated` /
`referees.hybrid` / `referees.timeOrScoreKeeper` (each an array, or absent). Example:
```json
{
  "tournamentReferee": null,
  "referees": {
    "dedicated": [
      { "user": { "id": "user-abc123", "username": "reef_ref" }, "rosterName": "Casey" }
    ],
    "hybrid": [],
    "timeOrScoreKeeper": []
  }
}
```

**Fields refbox actually reads:** refbox flattens every entry it finds (regardless of which
category it came from) into a single lookup from user ID to display name. Per entry, the ID is
`user.id`, falling back to `userId`, falling back to `id`. The display name is the entry's
`rosterName` if it's non-empty, otherwise `user.username`. An entry missing both an ID and a
name is silently skipped rather than causing an error — and a missing category (or a missing
`referees` object entirely) just means fewer names, not a failure.

**On failure:** Any non-`200` response or transport failure: refbox logs a warning (not an
error) and proceeds without any referee names — the schedule still loads normally, and referee
rows show a placeholder ("-") instead of a name.

#### 7. Push scores

`POST /api/events/{eventId}/schedule/games/{gameNumber}/scores`  ·  source: `uwh-common/src/uwhportal/mod.rs:353`

**When refbox calls it:** Automatically the moment a game ends (clock reaches the end of the
final period, including overtime or sudden death). This call and call 8 are queued together as
one item and a background task submits them in sequence — score first, then stats — retrying on
failure roughly every 15 seconds. See "On failure" below for the full retry/give-up behaviour,
which is shared with call 8.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** `force` (boolean, `true` or `false`). Ordinarily `false`. It's set to
`true` only when the operator taps "FORCE THIS GAME RESULT" on the portal attention screen after
a submission was rejected — telling the site to overwrite whatever score it currently has for
that game instead of rejecting the mismatch. A plain RETRY does not set `force`.

**Request body:** `{"dark": {"value": <0-255>}, "light": {"value": <0-255>}}`. Note the naming:
**`dark` is the black team's score, `light` is the white team's score** — not "home/away" or
"team 1/team 2". Example:
```json
{ "dark": { "value": 7 }, "light": { "value": 3 } }
```

**Successful response:** `200`. The body is never parsed — only the status code matters.

**Fields refbox actually reads:** none.

**On failure:** Any non-`200` response and a transport failure (can't reach the site at all)
are treated identically here — unlike call 2, refbox cannot tell a rejected/conflicting score
(a `409`), an expired token (a `401`), and a server error (a `500`) apart, and doesn't try to.
Any of them leaves the item in a local on-disk queue, retried automatically about every 15
seconds. If a queued game goes unresolved for 30 minutes, it stops auto-retrying and is flagged
to the operator to either FORCE or discard by hand. If it's still unresolved after 120 hours (5
days), it's dropped from the active queue and archived to a local file rather than retried
forever.

#### 8. Push stats

`POST /api/admin/events/stats`  ·  source: `uwh-common/src/uwhportal/mod.rs:325`

**When refbox calls it:** Immediately after call 7 succeeds for the same game — the two are
always attempted as a pair, never independently, as part of the same end-of-game queue item
described under call 7.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** `eventId` (the event ID, **long form** — `events/1234-A` — the one
exception among these eight to the short-form-in-path rule, since this is a query parameter,
not a path segment) and `gameNumber` (the game's number as a plain string, e.g. `"3"`).

**Request body:** A JSON object of per-team, per-player statistics for the game that just
ended. This is the other large, shared response shape — see [Data formats](#data-formats) for
the exact fields.

**Successful response:** `200`. The body is never parsed — only the status code matters.

**Fields refbox actually reads:** none — see Data formats for what the *request* body must
contain; nothing comes back that refbox reads.

**On failure:** Same non-`200`-vs-transport-failure handling as call 7 (both count as failure,
neither is distinguished from the other). The difference is what happens next: if the score
(call 7) already succeeded and only stats failed, the item is marked "stats-pending" rather than
retried automatically — the score is safely recorded either way, so refbox stops nagging about
the stats and never escalates the indicator to red over it. Stats-pending items are only
retried when the operator explicitly taps that item (or uses RETRY ALL) on the portal detail
page; there is no automatic retry loop for stats alone. A stats-pending item is still subject to
the same 120-hour archive-and-drop as any other queued item.

## Data formats

_(To be filled in.)_

## The other ten

_(To be filled in.)_

## Keeping this document honest

This document can drift from the code. The check below catches one specific kind of drift —
the set of paths — by comparing every `/api/...` path found in the source files against every
`/api/...` path found in this document, after normalising placeholder names (like
`{eventId}`) to a common `{}` on both sides so that naming differences don't cause false
alarms:

```bash
diff \
  <(rg -o -N '/api/[A-Za-z0-9/{}_-]+' uwh-common/src/uwhportal/mod.rs overlay/src/network.rs \
     | sed 's/^[^:]*://; s/{[^}]*}/{}/g' | sort -u) \
  <(rg -o '/api/[A-Za-z0-9/{}_-]+' docs/third-party-integration.md \
     | sed 's/{[^}]*}/{}/g' | sort -u) \
  && echo "IN SYNC"
```

This only proves the *paths* still match — it says nothing about whether the request or
response bodies documented here still match what the code sends and expects. The real test
of that is rebuilding a working stub server from this document alone (Task 5) and confirming
it actually stands in for the Portal.
