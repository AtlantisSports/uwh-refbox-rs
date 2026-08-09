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

This section gives the exact field-by-field shape of everything the calls above only summarised:
the two ways an ID can be written, the schedule the refbox downloads, the two timestamp formats,
and the stats records refbox uploads after a game.

### The two ID forms

Every event ID and team ID in this API can be written two ways: a **short form** (just the ID
itself, e.g. `1234-A`) and a **long form** (the ID with its type prefixed, e.g. `events/1234-A` or
`teams/5678-B`). Which one appears follows a single rule, true everywhere in this API:

- An ID that appears in a URL **path** is always the **short form**.
- An ID that appears in a **query parameter** is always the **long form**.

That's the whole rule — the "long form" is just the short form with `events/` or `teams/` stuck on
the front, or removed. In the code, `EventId::partial()` strips the `events/` prefix and
`EventId::full()` keeps it (`uwh-common/src/uwhportal/schedule.rs:714` and `:718`); `TeamId` has the
identical pair of methods (`schedule.rs:762` and `:766`).

Worked example, using the event and teams from the schedule example below:
- Short form, in a path: `GET /api/events/{eventId}/schedule/privileged` with `eventId` = `1234-A`
- Long form, in a query parameter: `POST /api/admin/events/stats?eventId=events/1234-A&gameNumber=1`

Across the full eighteen-call inventory, exactly three calls put an ID in a query parameter, and
so are the only three that use the long form:
- Push stats — `eventId` (`uwh-common/src/uwhportal/mod.rs:334`) — one of the refbox eight, call 8
  above.
- Team roster fetch — `teamId` (`mod.rs:676`) — part of the other ten, used by schedule-processor.
- Game referees fetch — `eventId` (`mod.rs:779`) — also part of the other ten.

Every other ID in this API — including every `{eventId}` in the path tables above — is the short
form.

### The schedule payload

This is the body returned by call 5 (`GET /api/events/{eventId}/schedule/privileged`). The whole
shape is deserialised as one Rust structure, so every field described below as "required" must be
present in the response — as an empty array or object where that's all there is — or the whole
schedule fails to load. Fields described as "optional" may be left out of the JSON entirely.

#### Top level: `Schedule` (`uwh-common/src/uwhportal/schedule.rs:513`)

| Field | Required? | Contents |
|---|---|---|
| `eventId` | required | The event ID, long form (`events/1234-A`) |
| `games` | required (may be `{}`) | **An object**, not an array — keys are game numbers as strings, values are `Game` objects (see below). The key string and the `Game`'s own `number` field should match. |
| `nonGameEntries` | required (may be `[]`) | Calendar entries (breaks, ceremonies) that aren't games. Not needed to run a game — a stub can always send `[]`. |
| `groups` | required (may be `[]`) | Pool/division structure and standings rules. Not needed to run a game — a stub can always send `[]`. |
| `timingRules` | required | Array of `TimingRule` objects (see below). Every game's `timingRule.name` must match one of these by name, or refbox can't find that game's timing. |
| `standingsOrder` | optional — may be omitted | Not needed to run a game |
| `finalResultsOrder` | optional — may be omitted | Not needed to run a game |
| `refereesByGameNumber` | optional — may be omitted | Team-supplied referee assignments, separate from the per-game `refereeAssignments` field below |

#### `Game` (`schedule.rs:226`) — what refbox needs to run a game

| JSON field | Rust type | Required? | Meaning |
|---|---|---|---|
| `number` | string | required | The game's number, as text (e.g. `"1"`) |
| `dark` | `ScheduledTeam` | required | The black-capped team |
| `light` | `ScheduledTeam` | required | The white-capped team |
| `startsOn` | timestamp | required | Scheduled start time — see [timestamp formats](#the-two-timestamp-formats) below |
| `court` | string | required | Court name, e.g. `"A"` |
| `timingRule` | object | required | **Not a bare string** — it's `{"name": "<string>"}`, matched by name against the schedule's top-level `timingRules` array |
| `refereeAssignments` | array of `RefereeAssignment` | optional — may be omitted | See below |
| `description` | string | optional — may be omitted | Free-text note shown on the game-info screen |

#### `ScheduledTeam` (`schedule.rs:36`)

All four fields are optional; in practice exactly one is populated per team. For a stub server,
only `teamId` matters — the other three describe teams not yet decided (winner-of, loser-of,
seeded-by-group, or a placeholder name), which a stand-in site can ignore:

| JSON field | Contents |
|---|---|
| `teamId` | The team ID, long form (`teams/5678-B`) |
| `pendingAssignmentName` | A placeholder name, when no team is assigned yet |
| `resultOf` | `{"type": "Winner"\|"Loser", "gameNumber": "<string>"}` |
| `seededBy` | `{"number": <int>, "group": {"name": "<string>"}}` (`group` itself is optional) |

#### `RefereeAssignment` (`schedule.rs:210`)

| JSON field | Required? | Contents |
|---|---|---|
| `role` | required | Free-text role name, e.g. `"Head Referee"` — refbox doesn't validate this against a fixed list |
| `userId` | optional | Portal user ID, matched against call 6's response to show a name |
| `teamId` | optional | Set instead of `userId` when a team (not an individual) supplies the referee, long form |

#### `TimingRule` (`schedule.rs:241`) — all fifteen fields

**Every duration here is a whole number of seconds — not milliseconds.** The code enforces this
with a custom `secs_only_duration` serializer (`schedule.rs:576-615`); a fractional or
millisecond value will not parse.

| # | JSON field | Type | Required? | Meaning |
|---|---|---|---|---|
| 1 | `name` | string | required | Matched by name from a `Game.timingRule.name` |
| 2 | `teamTimeoutCount` | integer | required | Team timeouts allowed per team |
| 3 | `teamTimeoutsCountedPerHalf` | bool | required | Whether the count in #2 resets each half |
| 4 | `overtimeAllowed` | bool | required | |
| 5 | `suddenDeathAllowed` | bool | required | |
| 6 | `last2minStopTime` | bool | optional, defaults to `false` | |
| 7 | `halfPlayDuration` | integer seconds | required | |
| 8 | `halfTimeDuration` | integer seconds | required | `0` signals a single-half game |
| 9 | `teamTimeoutDuration` | integer seconds | required | |
| 10 | `overtimeHalfPlayDuration` | integer seconds | required | |
| 11 | `overtimeHalfTimeDuration` | integer seconds | required | |
| 12 | `preOvertimeBreak` | integer seconds | required | |
| 13 | `preSuddenDeathDuration` | integer seconds | required | |
| 14 | `minimumBreak` | integer seconds | required | Minimum gap the schedule packs between games |
| 15 | `gameBlock` | integer seconds | optional | Total scheduled slot length for the game. If omitted, refbox works one out itself from the other durations (`schedule.rs:322-335`) — a stub server can simply leave it out. |

#### Worked example: a complete two-game schedule

Event `events/1234-A` ("Example Open 2026"), teams `teams/1234-A` ("Black Sheep"),
`teams/5678-B` ("White Knights"), and `teams/9012-C` ("Reef Sharks") — the same event and the
first two teams used in the stats example below, so the two examples describe one consistent
tournament:

```json
{
  "eventId": "events/1234-A",
  "games": {
    "1": {
      "number": "1",
      "dark": { "teamId": "teams/1234-A" },
      "light": { "teamId": "teams/5678-B" },
      "startsOn": "2026-08-08T09:00:00Z",
      "court": "A",
      "timingRule": { "name": "RR" },
      "refereeAssignments": [
        { "role": "Head Referee", "userId": "user-abc123" }
      ],
      "description": "Pool A opener"
    },
    "2": {
      "number": "2",
      "dark": { "teamId": "teams/9012-C" },
      "light": { "teamId": "teams/5678-B" },
      "startsOn": "2026-08-08T10:00:00Z",
      "court": "B",
      "timingRule": { "name": "RR" }
    }
  },
  "nonGameEntries": [],
  "groups": [],
  "timingRules": [
    {
      "name": "RR",
      "teamTimeoutCount": 1,
      "teamTimeoutsCountedPerHalf": true,
      "overtimeAllowed": true,
      "suddenDeathAllowed": true,
      "last2minStopTime": false,
      "halfPlayDuration": 900,
      "halfTimeDuration": 180,
      "teamTimeoutDuration": 60,
      "overtimeHalfPlayDuration": 300,
      "overtimeHalfTimeDuration": 180,
      "preOvertimeBreak": 180,
      "preSuddenDeathDuration": 60,
      "minimumBreak": 240
    }
  ]
}
```

Game 2 shows what's genuinely optional in practice: no `refereeAssignments`, no `description`, and
it reuses the same `"RR"` timing rule as game 1 rather than needing its own entry in `timingRules`.

### The two timestamp formats

There are two different timestamp formats in this API, and **they are not interchangeable** —
using one where the other is expected is a silent failure, not a rejected request, because both
happen to parse successfully even when the digits after the seconds don't match what the field
normally contains.

- **Schedule times** (`startsOn` / `endsOn`, anywhere they appear) always use whole seconds, never
  a fractional part: `iso8601_4dig_year_no_subsecs`, defined at `schedule.rs:13-20`.
- **Stats event times** (`occurredOn`, inside the stats records below) always include exactly nine
  fractional digits (nanoseconds), even when the value happens to fall on a whole second:
  `iso8601_short_year`, defined at `refbox/src/tournament_manager/game_stats.rs:10-14`.

(The name `iso8601_short_year` in the code is misleading — both formats use a full four-digit year
and both write UTC as `Z`. The only real difference between them is whether fractional seconds are
present.)

Side by side, for the same instant:

| Field | Format name in code | Example |
|---|---|---|
| `startsOn` (schedule) | `iso8601_4dig_year_no_subsecs` | `2026-08-08T09:00:00Z` |
| `occurredOn` (stats) | `iso8601_short_year` | `2026-08-08T09:00:00.000000000Z` |

A stub server building a schedule response should always write `startsOn` without a fractional
part. A stub server reading or storing the stats push (call 8) should expect `occurredOn` to
always carry nine fractional digits, even for round-second timestamps.

### The stats records

This is the request body for call 8 (`POST /api/admin/events/stats`) — see that call's entry
above for its query parameters (`eventId`, long form, and `gameNumber`). The body is **a bare JSON
array** of event objects, with no wrapping object — refbox builds it by serialising the array
directly (`game_stats.rs:96-104`) and sends those exact bytes as the request body
(`uwh-common/src/uwhportal/mod.rs:325-337`). refbox sorts the events by `occurredOn` before
sending, so a stub server can rely on chronological order.

Every element has a `"$type"` field naming which of three kinds it is: `"goal"`, `"penalty"`, or
`"foul"` (`game_stats.rs:107-153`). This is the most Portal-shaped part of the whole surface — it's
detailed data meant for the Portal's own statistics pages, not information refbox itself needs
back. **A site that only wants final scores can accept this call and discard the body entirely —
refbox only requires a `200` response and never reads anything back from it.**

All three kinds share five fields, then each adds its own:

| Field | Type | Meaning |
|---|---|---|
| `playerCapNumber` | integer (`foul`: integer or `null`) | The player's cap number. `null` on a `foul` only, for a team-level infraction ("both at fault") with no specific player. |
| `side` | string (`foul`: string or `null`) | `"dark"` or `"light"` — same black/white convention as push-scores. `null` on a `foul` only, alongside a `null` `playerCapNumber`. |
| `gamePeriod` | string | refbox's internal period name — one of `BetweenGames`, `FirstHalf`, `HalfTime`, `SecondHalf`, `PreOvertime`, `OvertimeFirstHalf`, `OvertimeHalfTime`, `OvertimeSecondHalf`, `PreSuddenDeath`, `SuddenDeath` (`uwh-common/src/game_snapshot.rs:129-141`) |
| `periodTime` | number (seconds, may have a fractional part) | The game clock's value the instant the event was recorded. During a timed half (regulation or overtime) this counts **down** — time remaining in the period. During Sudden Death, which has no fixed length, the clock instead counts **up** from zero — so `periodTime` there is time *elapsed*, not remaining (`refbox/src/tournament_manager/mod.rs:1873-1886`, `:2200-2206`). |
| `occurredOn` | timestamp | See [the two timestamp formats](#the-two-timestamp-formats) — this is always the `occurredOn` (fractional) form, never the `startsOn` form |

Fields specific to each kind:

**`goal`** — no extra fields beyond the five above.

**`penalty`** adds:
| Field | Type | Meaning |
|---|---|---|
| `duration` | integer seconds, or `null` | Whole seconds: `30`, `60`, `120`, `240`, or `300`, depending on the penalty's length. `null` only when `isTotalDismissal` is `true`. |
| `isTotalDismissal` | bool | `true` for a Total Dismissal (no fixed duration — the player is out for the rest of the game) |

**`foul`** adds:
| Field | Type | Meaning |
|---|---|---|
| `called` | string | refbox's internal infraction name — one of `Unknown`, `StickInfringement`, `IllegalAdvancement`, `IllegalSubstitution`, `IllegallyStoppingThePuck`, `OutOfBounds`, `GrabbingTheBarrier`, `Obstruction`, `DelayOfGame`, `UnsportsmanlikeConduct`, `FreeArm`, `FalseStart` (`uwh-common/src/game_snapshot.rs:336-350`) |

#### Worked example: one goal, one penalty, one foul

For game `"1"` of the schedule above (Black Sheep, `dark`, vs. White Knights, `light`), sent as
the body of `POST /api/admin/events/stats?eventId=events/1234-A&gameNumber=1`:

```json
[
  {
    "$type": "goal",
    "playerCapNumber": 7,
    "side": "dark",
    "gamePeriod": "FirstHalf",
    "periodTime": 507.0,
    "occurredOn": "2026-08-08T09:06:33.000000000Z"
  },
  {
    "$type": "penalty",
    "playerCapNumber": 12,
    "side": "light",
    "gamePeriod": "FirstHalf",
    "periodTime": 200.0,
    "occurredOn": "2026-08-08T09:11:40.000000000Z",
    "duration": 60,
    "isTotalDismissal": false
  },
  {
    "$type": "foul",
    "playerCapNumber": 4,
    "side": "dark",
    "gamePeriod": "SecondHalf",
    "periodTime": 812.0,
    "occurredOn": "2026-08-08T09:19:28.000000000Z",
    "called": "Obstruction"
  }
]
```

This is internally consistent with the schedule above: game 1's first half is 900 seconds
(`halfPlayDuration`) starting at `startsOn` (09:00:00), so a `periodTime` of `507.0` (seconds
*remaining*) during `FirstHalf` corresponds to a goal 6 minutes 33 seconds after kickoff — matching
the `occurredOn` timestamp. The same arithmetic carries through the penalty and, after the 180-second
half-time, the foul in the second half.

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
